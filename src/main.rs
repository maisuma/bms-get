mod app;
mod cli;
mod client;
mod download;
mod downloader;
mod event;
mod extract;
mod organize;
mod parser;
mod provider;
mod table;

use std::num::NonZeroU32;

use clap::Parser;
use cli::Cli;
use governor::Quota;

use crate::client::RateLimitedClient;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("bms_get=info"));
    let cli = Cli::parse();

    let quota = Quota::per_second(NonZeroU32::new(2).unwrap());
    let client = reqwest::Client::builder()
        .user_agent("bms-get")
        .build()
        .unwrap();
    let client = RateLimitedClient::new(client, quota);

    if let Err(e) = app::run(cli, client).await {
        log::error!("{:#}", e);
        std::process::exit(1);
    }
}
