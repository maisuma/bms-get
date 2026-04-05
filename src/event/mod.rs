use anyhow::Result;
use async_trait::async_trait;

use crate::client::RateLimitedClient;

pub struct EventEntry {
    pub urls: Vec<String>,
}

#[async_trait]
pub trait EventScraper: Send + Sync {
    fn can_handle(&self, url: &str) -> bool;
    async fn scrape(&self, client: &RateLimitedClient, url: &str) -> Result<Vec<EventEntry>>;
}

const SCRAPERS: &[&dyn EventScraper] = &[];

pub fn get_scraper(url: &str) -> Option<&dyn EventScraper> {
    SCRAPERS.into_iter().find(|s| s.can_handle(url)).copied()
}
