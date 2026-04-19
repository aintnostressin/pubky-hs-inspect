use colored::Colorize;

use crate::client::Client;
use crate::error::Result;

/// Fetch and print recent file change events from a homeserver.
pub async fn cmd_events(homeserver: Option<&str>, limit: Option<u64>, rev: bool) -> Result<()> {
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

    let (events, next_cursor) = match client.get_events(&base_url, None, limit, None, rev).await {
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

/// Resolve a homeserver identifier to a full HTTP base URL.
/// Tries the input directly as a URL, or resolves it via PKRR if it's a z32 key.
async fn resolve_homeserver_url(client: &Client, input: &str) -> Result<String> {
    // If it looks like a URL, use it directly
    if input.starts_with("http://") || input.starts_with("https://") {
        Ok(input.trim_end_matches('/').to_string())
    } else {
        // Try to resolve via PKRR
        if let Ok(pk) = pubky::PublicKey::try_from(input) {
            if let Some(info) = client.get_homeserver_address(&pk).await {
                if let Some(domain) = info.homeserver_domain {
                    return Ok(format!("https://{domain}"));
                } else {
                    return Ok(format!("https://_pubky.{}", info.homeserver_z32));
                }
            }
        }
        // Fall back to treating it as a domain
        Ok(format!("https://{input}"))
    }
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
