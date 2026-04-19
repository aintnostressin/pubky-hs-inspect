use colored::Colorize;

use crate::client::Client;
use crate::commands::shared::resolve_homeserver_url;
use crate::error::Result;

/// Fetch and print recent file change events from a homeserver.
pub async fn cmd_events(homeserver: Option<&str>, limit: Option<u64>) -> Result<()> {
    let client = Client::new()?;
    println!("{}", "═══ Homeserver Events ═══".bold().cyan());
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

    println!("Fetching events from: {base_url}");
    println!();

    let (events, next_cursor) = match client.get_events(&base_url, None, limit, None).await {
        Ok((events, cursor)) => (events, cursor),
        Err(e) => {
            eprintln!("Error fetching events: {e}");
            return Ok(());
        }
    };

    if events.is_empty() {
        println!("  {}", "no events found".yellow());
    } else {
        println!("  Total events: {}", events.len());
        println!();
        for event_line in &events {
            print_event_line(event_line);
        }
    }

    if let Some(cursor) = &next_cursor {
        println!();
        println!("  Next cursor: {cursor}");
    }

    Ok(())
}

/// Print a single event line with color coding.
fn print_event_line(line: &str) {
    if let Some(event_type) = line.split_whitespace().next() {
        let rest = &line[event_type.len()..].trim();
        match event_type {
            "PUT" => {
                println!("  {} {}", event_type.green(), rest);
            }
            "DEL" => {
                println!("  {} {}", event_type.red(), rest);
            }
            _ => {
                println!("  {line}");
            }
        }
    } else {
        println!("  {line}");
    }
}
