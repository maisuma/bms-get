use anyhow::{Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info};
use reqwest::Client;
use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

use crate::{downloader, extract};
use crate::parser::{self, ParseResult};
use crate::song::{
    BmsFileType, BmsProvider, BmsUrl, bms_search::BmsSearchProvider, lr2ir::Lr2IrProvider,
};
use crate::table::BmsData;

pub async fn download_md5(client: &Client, md5: &str, output_dir: &Path) -> Result<()> {
    try_download(client, md5, output_dir, BmsUrl::default()).await
}

pub async fn download_event_entry(client: &Client, entry: &crate::event::EventEntry, output_dir: &Path) -> Result<()> {
    let mut attempted = HashSet::new();
    match download_and_extract(client, &entry.urls, &mut attempted, output_dir).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(anyhow!("No downloadable URLs found")),
        Err(e) => Err(e),
    }
}

pub async fn download_table_entry(client: &Client, bms: &BmsData, output_dir: &Path) -> Result<()> {
    let seed = BmsUrl {
        main_urls: bms.main_url.clone().map_or(vec![], |u| vec![u]),
        diff_urls: bms.diff_url.clone().map_or(vec![], |u| vec![u]),
        unknown_urls: vec![],
        target_type: if bms.diff_url.is_some() {
            BmsFileType::Diff
        } else {
            BmsFileType::Unknown
        },
    };
    try_download(client, &bms.md5, output_dir, seed).await
}

async fn try_download(client: &Client, md5: &str, output_dir: &Path, seed: BmsUrl) -> Result<()> {
    let mut main_done = false;
    let mut diff_done = false;
    let mut target_type = seed.target_type;
    let mut unknown_urls = seed.unknown_urls;
    let mut attempted_urls = HashSet::new();
    let mut last_error = None;

    // 初期URLの試行
    if !seed.main_urls.is_empty() {
        match download_and_extract(client, &seed.main_urls, &mut attempted_urls, output_dir).await {
            Ok(true) => main_done = true,
            Ok(false) => {}
            Err(e) => last_error = Some(e),
        }
    }
    if !seed.diff_urls.is_empty() {
        match download_and_extract(client, &seed.diff_urls, &mut attempted_urls, output_dir).await {
            Ok(true) => diff_done = true,
            Ok(false) => {}
            Err(e) => last_error = Some(e),
        }
    }
    if is_satisfied(main_done, diff_done, target_type) {
        return Ok(());
    }

    // プロバイダ一覧
    let providers: Vec<Box<dyn BmsProvider>> =
        vec![Box::new(BmsSearchProvider), Box::new(Lr2IrProvider)];

    for provider in providers {
        info!("Searching on {}.... (md5: {})", provider.name(), md5);

        match provider.find_urls(client, md5).await {
            Ok(found) => {
                target_type = merge_target_type(target_type, found.target_type);

                unknown_urls.extend(found.unknown_urls);

                // 未取得のものをダウンロード
                if !main_done && !found.main_urls.is_empty() {
                    match download_and_extract(client, &found.main_urls, &mut attempted_urls, output_dir).await {
                        Ok(true) => main_done = true,
                        Ok(false) => {}
                        Err(e) => last_error = Some(e),
                    }
                }

                if !diff_done && !found.diff_urls.is_empty() {
                    match download_and_extract(client, &found.diff_urls, &mut attempted_urls, output_dir).await {
                        Ok(true) => diff_done = true,
                        Ok(false) => {}
                        Err(e) => last_error = Some(e),
                    }
                }
            }
            Err(e) => {
                last_error = Some(e);
            }
        }

        if is_satisfied(main_done, diff_done, target_type) {
            return Ok(());
        }
    }

    if !is_satisfied(main_done, diff_done, target_type) && !unknown_urls.is_empty() {
        info!("Trying unknown URLs... (md5: {})", md5);

        if let Err(e) = download_and_extract(client, &unknown_urls, &mut attempted_urls, output_dir).await {
            last_error = Some(e);
        }
    }

    let status = format!(
        "Download incomplete: main={}, diff={}, type={:?} (md5: {})",
        main_done, diff_done, target_type, md5
    );

    match last_error {
        Some(e) => Err(e.context(status)),
        None => Err(anyhow!(status)),
    }
}

fn is_satisfied(main: bool, diff: bool, target_type: BmsFileType) -> bool {
    match target_type {
        BmsFileType::Main => main,
        BmsFileType::Diff => main && diff,
        BmsFileType::Unknown => main && diff,
    }
}

fn merge_target_type(current: BmsFileType, found: BmsFileType) -> BmsFileType {
    match found {
        BmsFileType::Diff => BmsFileType::Diff,
        BmsFileType::Main if current == BmsFileType::Unknown => BmsFileType::Main,
        _ => current,
    }
}

async fn download_and_extract(
    client: &Client,
    urls: &[String],
    attempted_urls: &mut HashSet<String>,
    output_dir: &Path
) -> Result<bool> {
    let mut queue: VecDeque<String> = urls.iter().cloned().collect();
    let mut last_error = None;

    while let Some(url) = queue.pop_front() {
        if !attempted_urls.insert(url.clone()) {
            continue;
        }

        match parser::parse_url(client, &url).await {
            Ok(ParseResult::Links(new_urls)) => {
                queue.extend(new_urls);
            }
            Ok(ParseResult::File(dl_url)) => {
                sleep(Duration::from_secs(3)).await;

                let pb = ProgressBar::new(0);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{msg}\n[{bar:40.green/white}] {bytes}/{total_bytes} ({percent}%) {elapsed_precise}")?
                        .progress_chars("=> "),
                );
                pb.set_message(format!("Starting: {}", dl_url));

                let pb_clone = pb.clone();
                let result = downloader::download(
                    client,
                    &dl_url,
                    output_dir,
                    Box::new(move |inc, total| {
                        if pb.length().is_none() || pb.length() == Some(0) {
                            if total > 0 {
                                pb.set_length(total);
                            }
                        }
                        pb.inc(inc);
                    }),
                )
                .await;

                match result {
                    Ok(path) => {
                        pb_clone.finish_with_message(format!("Finished: {}", dl_url));
                        extract::extract(&path)?;
                        return Ok(true);
                    }
                    Err(e) => {
                        error!("Failed: {} - {}", dl_url, e);
                        last_error = Some(e);
                    }
                }
            }
            Err(e) => {
                error!("Parsing failed: {} - {}", url, e);
                last_error = Some(e);
            }
        }
    }

    if let Some(e) = last_error {
        Err(e.context("No valid URL found"))
    } else {
        Ok(false)
    }
}
