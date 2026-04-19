#![allow(dead_code)]

mod cli;
mod client;
mod commands;
mod error;

use clap::Parser;

use cli::Cli;
use error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    commands::run(&cli).await
}
