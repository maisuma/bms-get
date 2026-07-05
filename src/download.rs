use anyhow::{Result, anyhow};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info};
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::bms::{analyze::analyze_dir, merge, validation};
use crate::client::RateLimitedClient;
use crate::parser::{self, ParseResult};
use crate::provider::{
    BmsProvider, BmsUrl, bms_search::BmsSearchProvider, lr2ir_archive::Lr2IrArchiveProvider,
    seed::SeedProvider,
};
use crate::table::BmsData;
use crate::{downloader, extract};

pub async fn download_md5(client: &RateLimitedClient, md5: &str, output_dir: &Path) -> Result<()> {
    download_by_md5(client, md5, output_dir, BmsUrl::default()).await
}

pub async fn download_table_entry(
    client: &RateLimitedClient,
    bms: &BmsData,
    output_dir: &Path,
) -> Result<()> {
    let seed = BmsUrl {
        main_urls: bms.main_url.clone().map_or(vec![], |u| vec![u]),
        diff_urls: bms.diff_url.clone().map_or(vec![], |u| vec![u]),
    };

    download_by_md5(client, &bms.md5, output_dir, seed).await
}

async fn download_by_md5(
    client: &RateLimitedClient,
    md5: &str,
    output_dir: &Path,
    seed: BmsUrl,
) -> Result<()> {
    let mut attempted_urls = HashSet::new();
    let providers: Vec<Box<dyn BmsProvider>> = vec![
        Box::new(SeedProvider::new(seed)),
        Box::new(BmsSearchProvider),
        Box::new(Lr2IrArchiveProvider),
    ];

    for need in [NeedBmsType::Diff, NeedBmsType::Main] {
        'providers: for provider in &providers {
            info!("Searching on {}.... (md5: {})", provider.name(), md5);

            let urls = match provider.find_urls(client, md5).await {
                Ok(urls) => urls,
                Err(e) => {
                    error!("Searching on {} failed: {}", provider.name(), e);
                    continue;
                }
            };

            let urls = match need {
                NeedBmsType::Diff => &urls.diff_urls,
                NeedBmsType::Main => &urls.main_urls,
            };
            let mut walker = UrlWalker::new(urls, &mut attempted_urls);

            while let Some(next) = walker.next(client).await {
                let url = match next {
                    Ok(url) => url,
                    Err(e) => {
                        error!("Parsing failed: {}", e);
                        continue;
                    }
                };

                let path = match download(client, &url, output_dir).await {
                    Ok(path) => path,
                    Err(e) => {
                        error!("Failed: {} - {}", url, e);
                        continue;
                    }
                };

                let extractor = extract::find_extractor(&path)?;
                let result = extractor.extract_to(&path)?;
                debug!(
                    "Extracted {} entries from {} to {}",
                    result.extracted_paths.len(),
                    result.archive_path.display(),
                    result.target_dir.display()
                );

                let dir = analyze_dir(output_dir)?;
                let source = analyze_dir(&result.target_dir)?;
                merge::merge_bms_dir(&dir, &source)?;

                if !validation::validate_md5(md5, output_dir)? {
                    continue;
                }

                if validation::validate_ref(md5, output_dir)? {
                    return Ok(());
                }

                break 'providers;
            }
        }
    }

    Err(anyhow!("Download incomplete: md5={}", md5))
}

#[derive(Clone, Copy)]
enum NeedBmsType {
    Diff,
    Main,
}

pub async fn download_event_entry(
    client: &RateLimitedClient,
    entry: &crate::event::EventEntry,
    output_dir: &Path,
) -> Result<()> {
    let mut attempted_urls = HashSet::new();
    let mut walker = UrlWalker::new(&entry.urls, &mut attempted_urls);
    let mut downloaded = false;

    while let Some(next) = walker.next(client).await {
        match next {
            Ok(url) => match download(client, &url, output_dir).await {
                Ok(path) => {
                    let extractor = extract::find_extractor(&path)?;
                    let result = extractor.extract_to(&path)?;
                    let dir = analyze_dir(output_dir)?;
                    let source = analyze_dir(&result.target_dir)?;
                    merge::merge_bms_dir(&dir, &source)?;

                    downloaded = true;
                }
                Err(e) => {
                    error!("Failed: {} - {}", url, e);
                }
            },
            Err(e) => {
                error!("Parsing failed: {}", e);
            }
        }
    }

    if downloaded {
        Ok(())
    } else {
        Err(anyhow!("No downloadable URLs found"))
    }
}

async fn download(client: &RateLimitedClient, url: &str, output_dir: &Path) -> Result<PathBuf> {
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n[{bar:40.green/white}] {bytes}/{total_bytes} ({percent}%) {elapsed_precise}")?
            .progress_chars("=> "),
    );
    pb.set_message(format!("Starting: {}", url));

    let pb_clone = pb.clone();
    let path = downloader::download(
        client,
        url,
        output_dir,
        Box::new(move |inc, total| {
            if (pb.length().is_none() || pb.length() == Some(0)) && total > 0 {
                pb.set_length(total);
            }
            pb.inc(inc);
        }),
    )
    .await?;

    pb_clone.finish_with_message(format!("Finished: {}", url));
    Ok(path)
}

struct UrlWalker<'a> {
    queue: VecDeque<String>,
    attempted_urls: &'a mut HashSet<String>,
}

impl<'a> UrlWalker<'a> {
    fn new(urls: &[String], attempted_urls: &'a mut HashSet<String>) -> Self {
        Self {
            queue: urls.iter().cloned().collect(),
            attempted_urls,
        }
    }

    async fn next(&mut self, client: &RateLimitedClient) -> Option<Result<String>> {
        while let Some(url) = self.queue.pop_front() {
            if !self.attempted_urls.insert(url.clone()) {
                continue;
            }

            match parser::parse_url(client, &url).await {
                Ok(ParseResult::Links(new_urls)) => self.queue.extend(new_urls),
                Ok(ParseResult::File(dl_url)) => return Some(Ok(dl_url)),
                Err(e) => return Some(Err(e.context(format!("Parsing failed: {}", url)))),
            }
        }

        None
    }
}
