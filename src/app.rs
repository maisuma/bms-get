use std::path::Path;

use log::{error, info};

use crate::cli::{Cli, Commands};
use crate::client::RateLimitedClient;
use crate::download;
use crate::event;
use crate::table;

pub async fn run(cli: Cli, client: RateLimitedClient) {
    match &cli.command {
        Commands::Md5 { md5 } => handle_md5(&client, md5, &cli.output_dir).await,
        Commands::Table { url } => handle_table(&client, url, &cli.output_dir).await,
        Commands::Event { url } => handle_event(&client, url, &cli.output_dir).await,
    }
}

async fn handle_md5(client: &RateLimitedClient, md5: &str, output_dir: &Path) {
    info!("md5: {}", md5);
    let _ = download::download_md5(client, md5, output_dir).await;
}

async fn handle_table(client: &RateLimitedClient, url: &str, output_dir: &Path) {
    let table = match table::parse_table(client, url).await {
        Ok(table) => table,
        Err(error) => {
            error!("Failed to fetch table: {}", error);
            return;
        }
    };

    info!("Table fetched: {}", table.name);

    for bms in &table.bms_data {
        let _ = download::download_table_entry(client, bms, output_dir).await;
    }
}

async fn handle_event(client: &RateLimitedClient, url: &str, output_dir: &Path) {
    let scraper = match event::get_scraper(url) {
        Some(s) => s,
        None => {
            error!("This event site URL is currently not supported");
            return;
        }
    };

    let entries = match scraper.scrape(client, url).await {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to parse event page: {}", e);
            return;
        }
    };

    info!("Event fetched ({} songs in total)", entries.len());

    for entry in &entries {
        let _ = download::download_event_entry(client, entry, output_dir).await;
    }
}
