use anyhow::Result;
use async_trait::async_trait;
use encoding_rs::SHIFT_JIS;
use log::debug;
use reqwest::Client;
use scraper::{Html, Selector};

use super::{BmsFileType, BmsProvider, BmsUrl};

pub struct Lr2IrProvider;

#[async_trait]
impl BmsProvider for Lr2IrProvider {
    fn name(&self) -> &'static str {
        "LR2IR"
    }

    async fn find_urls(&self, client: &Client, md5: &str) -> Result<BmsUrl> {
        let search_url = format!(
            "http://www.dream-pro.info/~lavalse/LR2IR/search.cgi?mode=ranking&bmsmd5={}",
            md5
        );
        debug!("[LR2IR] URL: {}", search_url);
        let bytes = client.get(&search_url).send().await?.bytes().await?;
        let (decoded, _, _) = SHIFT_JIS.decode(&bytes);
        let response = decoded.into_owned();

        let document = Html::parse_document(&response);

        let tr_selector = Selector::parse("tr").unwrap();
        let th_selector = Selector::parse("th").unwrap();
        let a_selector = Selector::parse("a").unwrap();

        let mut main_url: Option<String> = None;
        let mut diff_url: Option<String> = None;

        for tr in document.select(&tr_selector) {
            if let Some(th) = tr.select(&th_selector).next() {
                let th_text = th.text().collect::<String>().trim().to_string();
                if th_text == "本体URL" || th_text == "差分URL" {
                    if let Some(a) = tr.select(&a_selector).next() {
                        if let Some(href) = a.value().attr("href") {
                            if th_text == "本体URL" {
                                main_url = Some(href.to_string());
                            } else if th_text == "差分URL" {
                                diff_url = Some(href.to_string());
                            }
                        }
                    }
                }
            }
        }

        debug!("[LR2IR] main: {}", &main_url.as_deref().unwrap_or("None"));
        debug!("[LR2IR] diff: {}", &diff_url.as_deref().unwrap_or("None"));

        let target_type = if diff_url.is_some() {
            BmsFileType::Diff
        } else {
            BmsFileType::Unknown
        };

        Ok(BmsUrl {
            main_urls: main_url.map_or(vec![], |url| vec![url]),
            diff_urls: diff_url.map_or(vec![], |url| vec![url]),
            unknown_urls: vec![],
            target_type,
        })
    }
}
