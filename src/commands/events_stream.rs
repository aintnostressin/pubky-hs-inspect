use colored::Colorize;

use crate::client::Client;
use crate::commands::shared::{print_sse_event, resolve_homeserver_url};
use crate::error::Result;

/// Stream events from a homeserver in real-time via the /events-stream/ endpoint.
pub async fn cmd_events_stream(
    homeserver: Option<&str>,
    user: Option<&str>,
    limit: Option<u64>,
    reverse: bool,
    live: bool,
) -> Result<()> {
    let client = Client::new()?;
    if live {
        println!(
            "{} {}",
            "═══ Events Stream ═══".bold().cyan(),
            "(LIVE)".bold().magenta()
        );
    } else {
        println!("{}", "═══ Events Stream ═══".bold().cyan());
    }
    println!();

    // Determine homeserver target
    let target = match homeserver {
        Some(hs) => hs.to_string(),
        None => {
            eprintln!(
                "{}",
                "Error: homeserver address required. Provide as argument or via -u/--url.".yellow()
            );
            return Ok(());
        }
    };

    // Resolve to a base URL
    let base_url = resolve_homeserver_url(&client, &target).await?;

    println!("  URL: {base_url}");
    if let Some(u) = user {
        println!("  User: {u}");
    }
    if let Some(l) = limit {
        println!("  Limit: {l}");
    }
    println!("  Reverse: {reverse}");
    println!();

    // Stream events
    let events = match client.stream_events(&base_url, user, limit, reverse).await {
        Ok(events) => events,
        Err(e) => {
            eprintln!("Error streaming events: {e}");
            return Ok(());
        }
    };

    if events.is_empty() {
        println!("  {}", "no events found".yellow());
    } else {
        println!("  Total events: {}", events.len());
        println!();
        for event in &events {
            print_sse_event(event);
        }
        if let Some(last) = events.last() {
            println!();
            println!("  Last cursor: {}", last.cursor);
        }
    }

    Ok(())
}
