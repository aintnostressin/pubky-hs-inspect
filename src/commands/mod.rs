use clap::CommandFactory;
use colored::Colorize;

use crate::cli::{Cli, Commands};

pub mod events;
pub mod inspect;
pub mod inspect_user;
pub mod ls;
pub mod pkdns;
pub mod shared;
pub mod storage;

pub async fn run(cli: &Cli) -> crate::error::Result<()> {
    match &cli.command {
        Some(Commands::Inspect { url }) => inspect::cmd_inspect(url).await,
        Some(Commands::InspectUser { url }) => inspect_user::cmd_inspect_user(url).await,
        Some(Commands::Pkdns { url }) => pkdns::cmd_pkdns(url).await,
        Some(Commands::Storage { url }) => storage::cmd_storage(url).await,
        Some(Commands::Ls { url, path }) => ls::cmd_ls(url, path).await,
        Some(Commands::Version) => shared::cmd_version(),
        Some(Commands::Events { homeserver, limit, rev }) => {
            // Use global URL as fallback if homeserver not provided
            let target = homeserver.as_deref().or(cli.url.as_deref()).or(Some(""));
            events::cmd_events(target, *limit, *rev).await
        }
        None => {
            println!(
                "{}",
                "No subcommand specified. Use --help for usage information."
                    .bold()
                    .yellow()
            );
            println!();
            if let Err(e) = Cli::command().print_help() {
                eprintln!("Failed to print help: {e}");
            }
            println!();
            Ok(())
        }
    }
}
