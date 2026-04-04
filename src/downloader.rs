use anyhow::{Context, Result};
use futures_util::StreamExt;
use log::debug;
use regex::Regex;
use reqwest::{header};
use std::path::{Path, PathBuf};
use std::str::from_utf8;
use std::sync::OnceLock;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::client::RateLimitedClient;

pub async fn download(
    client: &RateLimitedClient,
    url: &str,
    output_dir: &Path,
    on_progress: Box<dyn Fn(u64, u64) + Send + Sync>,
) -> Result<PathBuf> {
    debug!("[Downloader] ダウンロード開始: {}", url);

    let response = client
        .get(url)
        .await
        .send()
        .await?
        .error_for_status()?;

    let filename = if let Some(content) = response.headers().get(header::CONTENT_DISPOSITION) {
        let data = from_utf8(content.as_bytes())?;
        get_filename_from_header(data)
    } else {
        None
    };

    let filename = filename
        .or_else(|| get_filename_from_url(url))
        .context("Failed to get filename")?;

    debug!("[Downloader] ファイル名: {}", filename);

    let save_path = output_dir.join(filename);

    let total_size = response.content_length().unwrap_or(0);
    let mut file = File::create(&save_path).await?;
    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk).await?;
        on_progress(chunk.len() as u64, total_size);
    }

    Ok(save_path)
}

fn get_filename_from_url(url: &str) -> Option<String> {
    let url = url.split('?').next()?;
    let segments: Vec<&str> = url.rsplit('/').collect();
    let filename = segments.first()?;

    if filename.contains('.') && filename.len() < 100 {
        Some(sanitize(filename))
    } else {
        None
    }
}

fn get_filename_from_header(header: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r#"filename=\"?([^";]+)\"?"#).unwrap());

    re.captures(header)
        .and_then(|caps| caps.get(1))
        .map(|m| sanitize(m.as_str()))
}

fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_control() || ['/', '\\', ':', '*', '?', '"', '<', '>', '|'].contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();

    let s = s.trim().trim_end_matches('.');

    if s.is_empty() {
        return "downloaded_file".to_string();
    }

    s.to_string()
}
