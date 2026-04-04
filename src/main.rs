mod app;
mod cli;
mod download;
mod downloader;
mod event;
mod parser;
mod song;
mod table;
mod extract;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("bms_get=info"));
    let cli = Cli::parse();

    let client = reqwest::Client::builder()
        .user_agent("bms-get")
        .build()
        .unwrap();
    app::run(cli, client).await;
}
