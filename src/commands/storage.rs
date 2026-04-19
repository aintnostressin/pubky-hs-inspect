use colored::Colorize;

use crate::client::{parse_input, Client, InputType};
use crate::error::Result;
use crate::commands::shared;

/// Inspect public storage for a PKRR public key.
pub async fn cmd_storage(input: &str) -> Result<()> {
    let client = Client::new()?;
    println!("{}", "═══ Public Storage Inspector ═══".bold().cyan());
    println!();

    let parsed = parse_input(input);

    match &parsed {
        InputType::PublicKey(key_str) => {
            let pk = match pubky::PublicKey::try_from(key_str.as_str()) {
                Ok(pk) => pk,
                Err(e) => {
                    println!("   Error: {e}");
                    return Ok(());
                }
            };

            let z32 = pk.z32();

            // Homeserver info
            if let Some(info) = client.get_homeserver_address(&pk).await {
                println!("{}", "▸ Homeserver".bold());
                shared::print_homeserver_info(&info);
                println!();
            }

            // List storage
            let pub_addr = format!("pubky://{z32}/pub/");
            println!("{}", "▸ Storage Listing".bold());
            match client.list(&pub_addr).await {
                Ok(entries) if !entries.is_empty() => {
                    println!("   Total entries: {}", entries.len());
                    println!();
                    for entry in entries.iter() {
                        println!("   {}", entry);
                    }
                }
                Ok(_) => {
                    println!("   {}", "no public entries".yellow());
                }
                Err(e) => {
                    println!("   Error: {}", e);
                }
            }
        }
        InputType::Url(url_str) => {
            let addr = shared::strip_pubky_scheme(url_str);
            println!("Fetching: {addr}");
            match client.get_text(&addr).await {
                Ok(text) => {
                    println!("Status: {}", "success ✓".green());
                    println!("Length: {} bytes", text.len());
                    if text.trim_start().starts_with('{') {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                            let formatted = serde_json::to_string_pretty(&val).unwrap_or(text);
                            println!();
                            println!("Content:");
                            for line in formatted.lines().take(30) {
                                println!("  {line}");
                            }
                        }
                    } else {
                        println!("\n{text}");
                    }
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
    }

    Ok(())
}
