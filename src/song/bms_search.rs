use anyhow::{Context, Result};
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use tokio::time::{Duration, sleep};

use super::{BmsFileType, BmsProvider, BmsUrl};

pub struct BmsSearchProvider;

#[derive(serde::Deserialize)]
struct BmsPattern {
    bms: BmsId,
    #[serde(rename = "packType")]
    pack_type: Option<String>,
}

#[derive(serde::Deserialize)]
struct BmsId {
    id: Option<String>,
}

#[derive(serde::Deserialize)]
struct BmsData {
    downloads: Option<Vec<DownloadUrl>>,
}

#[derive(serde::Deserialize)]
struct DownloadUrl {
    url: Option<String>,
}

#[async_trait]
impl BmsProvider for BmsSearchProvider {
    fn name(&self) -> &'static str {
        "BMS SEARCH API"
    }

    async fn find_urls(&self, client: &Client, md5: &str) -> Result<BmsUrl> {
        let api_url = format!("https://api.bmssearch.net/v1/patterns/{}", md5);
        let response = client.get(&api_url).send().await?;

        let pattern: BmsPattern = response.json().await.context("Parsing failed")?;
        let id = pattern.bms.id.context("BMS ID not found")?;

        sleep(Duration::from_secs(5)).await;

        let api_url = format!("https://api.bmssearch.net/v1/bmses/{}", id);
        let response = client.get(&api_url).send().await?;

        let bms: BmsData = response.json().await.context("Parsing failed")?;
        let urls: Vec<String> = bms
            .downloads
            .context("URL not found")?
            .iter()
            .filter_map(|u| u.url.clone())
            .collect();

        // TODO: 差分の場合は本体もついでに探す
        let (main_urls, diff_urls, unknown_urls, target_type) = match pattern.pack_type.as_deref() {
            Some("INCLUDED") => (urls, vec![], vec![], BmsFileType::Main),
            Some("ADDITIONAL") => (urls, vec![], vec![], BmsFileType::Diff),
            _ => (vec![], vec![], urls, BmsFileType::Unknown),
        };

        Ok(BmsUrl {
            main_urls,
            diff_urls,
            unknown_urls,
            target_type,
        })
    }
}
