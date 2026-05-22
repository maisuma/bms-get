use std::path::Path;

use anyhow::{Context, Result, anyhow};
use log::{error, info};

use crate::cli::{Cli, Commands};
use crate::client::RateLimitedClient;
use crate::download;
use crate::event;
use crate::table;

pub async fn run(cli: Cli, client: RateLimitedClient) -> Result<()> {
    match &cli.command {
        Commands::Md5 { md5 } => handle_md5(&client, md5, &cli.output_dir).await,
        Commands::Table { url } => handle_table(&client, url, &cli.output_dir).await,
        Commands::Event { url } => handle_event(&client, url, &cli.output_dir).await,
    }
}

async fn handle_md5(client: &RateLimitedClient, md5: &str, output_dir: &Path) -> Result<()> {
    info!("md5: {}", md5);
    download::download_md5(client, md5, output_dir).await
}

async fn handle_table(client: &RateLimitedClient, url: &str, output_dir: &Path) -> Result<()> {
    let table = table::parse_table(client, url)
        .await
        .context("Failed to fetch table")?;

    info!("Table fetched: {}", table.name);

    let mut succeeded = 0;
    let mut failed = 0;

    for bms in &table.bms_data {
        match download::download_table_entry(client, bms, output_dir).await {
            Ok(()) => succeeded += 1,
            Err(e) => {
                failed += 1;
                error!(
                    "Failed to download table entry: md5={}, level={}, title={}, artist={}: {:#}",
                    bms.md5,
                    bms.level,
                    bms.title.as_deref().unwrap_or("(no title)"),
                    bms.artist.as_deref().unwrap_or("(no artist)"),
                    e
                );
            }
        }
    }

    info!(
        "Table download finished: {} succeeded, {} failed",
        succeeded, failed
    );

    if failed > 0 {
        Err(anyhow!("{} table entries failed", failed))
    } else {
        Ok(())
    }
}

async fn handle_event(client: &RateLimitedClient, url: &str, output_dir: &Path) -> Result<()> {
    let scraper = match event::get_scraper(url) {
        Some(s) => s,
        None => {
            return Err(anyhow!("This event site URL is currently not supported"));
        }
    };

    let entries = scraper
        .scrape(client, url)
        .await
        .context("Failed to parse event page")?;

    info!("Event fetched ({} songs in total)", entries.len());

    let mut succeeded = 0;
    let mut failed = 0;

    for (index, entry) in entries.iter().enumerate() {
        match download::download_event_entry(client, entry, output_dir).await {
            Ok(()) => succeeded += 1,
            Err(e) => {
                failed += 1;
                error!(
                    "Failed to download event entry #{}: urls={:?}: {:#}",
                    index + 1,
                    entry.urls,
                    e
                );
            }
        }
    }

    info!(
        "Event download finished: {} succeeded, {} failed",
        succeeded, failed
    );

    if failed > 0 {
        Err(anyhow!("{} event entries failed", failed))
    } else {
        Ok(())
    }
}
