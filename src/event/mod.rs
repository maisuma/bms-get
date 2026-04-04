use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

pub struct EventEntry {
    pub urls: Vec<String>,
}

#[async_trait]
pub trait EventScraper: Send + Sync {
    fn can_handle(&self, url: &str) -> bool;
    async fn scrape(&self, client: &Client, url: &str) -> Result<Vec<EventEntry>>;
}

pub fn get_scraper(url: &str) -> Option<Box<dyn EventScraper>> {
    let scrapers: Vec<Box<dyn EventScraper>> = vec![
        // 新しいイベントパーサーはここに追加
    ];

    scrapers.into_iter().find(|s| s.can_handle(url))
}
