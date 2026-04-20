use colored::Colorize;
use futures::StreamExt;

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
    println!("  Mode: {}", if live { "live" } else { "batch" });
    println!();

    // Stream events in real-time
    let mut stream = match client
        .stream_events_streamed(&base_url, user, limit, reverse)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error starting stream: {e}");
            println!();
            println!(
                "  {}",
                "events-stream endpoint may not be supported by this homeserver".yellow()
            );
            return Ok(());
        }
    };

    let mut count = 0u64;
    let mut has_events = false;

    // Indicate we're waiting for live events
    if live {
        print!("  Waiting for new events...");
    }

    loop {
        match stream.next().await {
            Some(Ok(event)) => {
                if !live {
                    print_sse_event(&event);
                } else {
                    // In live mode, only print if we haven't shown the waiting message yet
                    // or if we've already received some events
                    if !has_events {
                        // First event: replace "Waiting..." with the event
                        print!("\r"); // Move to beginning of line
                        has_events = true;
                    }
                    print_sse_event(&event);
                }
                count += 1;
                if !live && count >= limit.unwrap_or(u64::MAX) {
                    break;
                }
            }
            Some(Err(e)) => {
                eprintln!("Error receiving event: {e}");
                break;
            }
            None => {
                // Stream ended (EOF)
                if live {
                    if count == 0 {
                        println!();
                        eprintln!(
                            "  {}",
                            "Stream closed immediately — no events received.".yellow()
                        );
                        eprintln!(
                            "  The /events-stream endpoint may not support live streaming on this homeserver."
                        );
                    } else {
                        println!();
                        println!(
                            "  Stream closed after {} events. {}",
                            count.to_string().green().bold(),
                            "Press Ctrl+C to exit.".yellow()
                        );
                    }
                } else {
                    println!();
                    println!("  {} events received.", count.to_string().green().bold());
                }
                break;
            }
        }
    }

    Ok(())
}
