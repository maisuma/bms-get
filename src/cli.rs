use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    #[arg(short, long, default_value = ".")]
    pub output_dir: PathBuf,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download BMS form an MD5 hash
    Md5 { md5: String },
    /// Download BMS from a difficulty table
    Table { url: String },
    /// Download BMS from an event page
    Event { url: String },
}
