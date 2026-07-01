use anyhow::Result;
use async_trait::async_trait;
use reqwest::Url;

use crate::{client::RateLimitedClient, parser::UrlParser};

pub struct DropboxParser;

#[async_trait]
impl UrlParser for DropboxParser {
    fn can_parse(&self, url: &str) -> bool {
        Url::parse(url)
            .ok()
            .and_then(|url| {
                let host = url.host_str()?;
                let path = url.path();

                Some(
                    host == "www.dropbox.com"
                        && (path.starts_with("/s/") || path.starts_with("/scl/fi/")),
                )
            })
            .unwrap_or(false)
    }

    async fn parse(&self, _client: &RateLimitedClient, url: &str) -> Result<Vec<String>> {
        let mut url = Url::parse(url)?;
        url.set_scheme("https").ok();
        url.set_host(Some("dl.dropboxusercontent.com"))?;
        url.set_fragment(None);

        Ok(vec![url.to_string()])
    }
}
