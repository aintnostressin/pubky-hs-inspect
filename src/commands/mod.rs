use clap::CommandFactory;
use colored::Colorize;

use crate::cli::{Cli, Commands};

pub mod events;
pub mod events_stream;
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
        Some(Commands::Events {
            homeserver,
            limit,
            rev,
        }) => {
            // Use global URL as fallback if homeserver not provided
            let target = homeserver.as_deref().or(cli.url.as_deref()).or(Some(""));
            events::cmd_events(target, *limit, *rev).await
        }
        Some(Commands::EventsStream {
            homeserver,
            user,
            limit,
            reverse,
            live,
        }) => {
            // Use global URL as fallback if homeserver not provided
            let target = homeserver.as_deref().or(cli.url.as_deref()).or(Some(""));
            let hs = if target.as_ref().is_none_or(|s| s.is_empty()) {
                None
            } else {
                target.map(|s| s.trim_end_matches('/'))
            };
            events_stream::cmd_events_stream(hs, user.as_deref(), *limit, *reverse, *live).await
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
