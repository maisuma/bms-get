use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

pub mod bms_search;
pub mod lr2ir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmsFileType {
    Main,
    Diff,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct BmsUrl {
    pub main_urls: Vec<String>,
    pub diff_urls: Vec<String>,
    pub unknown_urls: Vec<String>,
    pub target_type: BmsFileType,
}

impl Default for BmsFileType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[async_trait]
pub trait BmsProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn find_urls(&self, client: &Client, md5: &str) -> Result<BmsUrl>;
}
