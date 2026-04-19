use clap::Parser;

use pubky_hs_inspect::cli::Cli;
use pubky_hs_inspect::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    pubky_hs_inspect::commands::run(&cli).await
}
