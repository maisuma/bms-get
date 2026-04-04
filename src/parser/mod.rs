use anyhow::Result;
use async_trait::async_trait;

use crate::client::RateLimitedClient;

pub mod gdrive;

#[derive(Debug)]
pub enum ParseResult {
    Links(Vec<String>),
    File(String),
}

#[async_trait]
pub trait UrlParser: Send + Sync {
    fn can_parse(&self, url: &str) -> bool;
    async fn parse(&self, client: &RateLimitedClient, url: &str) -> Result<Vec<String>>;
}

pub async fn parse_url(client: &RateLimitedClient, url: &str) -> Result<ParseResult> {
    let parsers: Vec<Box<dyn UrlParser>> = vec![
        Box::new(gdrive::GDriveParser),
        // 新規パーサーはここに追加
    ];

    for parser in parsers {
        if parser.can_parse(url) {
            let res = parser.parse(client, url).await?;
            return Ok(ParseResult::Links(res));
        }
    }

    Ok(ParseResult::File(url.to_string()))
}