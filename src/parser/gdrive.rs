use super::{UrlParser};
use anyhow::{Context, Result};
use async_trait::async_trait;
use log::debug;
use regex::Regex;
use reqwest::{header, Client, Url};
use scraper::{Html, Selector};
use std::sync::OnceLock;

pub struct GDriveParser;

#[async_trait]
impl UrlParser for GDriveParser {
    fn can_parse(&self, url: &str) -> bool {
        url.contains("drive.google.com")
    }

    async fn parse(&self, client: &Client, url: &str) -> Result<Vec<String>> {
        let file_id = get_drive_id(url).context("Failed to get Google Drive file ID")?;
        let download_url = format!(
            "https://drive.usercontent.google.com/download?id={}&export=download&authuser=0",
            file_id
        );

        debug!("[GDRIVE] ID: {}", file_id);
        debug!("[GDRIVE] URL: {}", download_url);

        let response = client.get(&download_url).send().await?.error_for_status()?;

        if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
            if content_type.to_str().unwrap_or("").contains("text/html") {
                debug!("[GDRIVE] Virus scan warning page detected");
                let text = response.text().await?;

                let document = Html::parse_document(&text);
                let real_download_url = get_download_url_from_form(&document)
                    .context("Gogole Drive download link not found")?
                    .to_string();

                debug!("[GDRIVE] Parsing succeeded: {}", real_download_url);
                return Ok(vec![real_download_url]);
            }
        }

        debug!("[GDRIVE] Parsing succeeded: {}", download_url);
        Ok(vec![download_url])
    }
}

fn get_drive_id(url: &str) -> Option<String> {
    static RE_PATH: OnceLock<Regex> = OnceLock::new();
    let re_path = RE_PATH.get_or_init(|| Regex::new(r"/file/d/([a-zA-Z0-9_-]+)").unwrap());

    if let Some(caps) = re_path.captures(url) {
        return Some(caps.get(1)?.as_str().to_string());
    }

    static RE_QUERY: OnceLock<Regex> = OnceLock::new();
    let re_query = RE_QUERY.get_or_init(|| Regex::new(r"[?&]id=([a-zA-Z0-9_-]+)").unwrap());

    if let Some(caps) = re_query.captures(url) {
        return Some(caps.get(1)?.as_str().to_string());
    }

    None
}

fn get_download_url_from_form(document: &Html) -> Option<Url> {
    let form_selector = Selector::parse("form#download-form").ok()?;
    let input_selector = Selector::parse("input").ok()?;

    let form = document.select(&form_selector).next()?;
    let action = form.value().attr("action")?;

    let mut url = Url::parse(action).ok()?;

    for input in form.select(&input_selector) {
        if let (Some(name), Some(value)) = (input.value().attr("name"), input.value().attr("value"))
        {
            url.query_pairs_mut().append_pair(name, value);
        }
    }

    Some(url)
}
