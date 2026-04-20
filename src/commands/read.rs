use colored::Colorize;

use crate::client::{parse_input, Client, InputType};
use crate::commands::shared;
use crate::error::Result;

/// Read and display a file from a user's storage.
pub async fn cmd_read(url: String, path: String) -> Result<()> {
    let client = Client::new()?;
    println!("{}", "═══ File Reader ═══".bold().cyan());
    println!();

    let parsed = parse_input(&url);

    match &parsed {
        InputType::PublicKey(key_str) => {
            let pk = match pubky::PublicKey::try_from(key_str.as_str()) {
                Ok(pk) => pk,
                Err(e) => {
                    println!("   Error parsing public key: {e}");
                    return Ok(());
                }
            };

            let z32 = pk.z32();
            let file_url = format!("pubky://{z32}{path}");

            // Resolve homeserver
            if let Some(info) = client.get_homeserver_address(&pk).await {
                println!("{}", "▸ Homeserver".bold());
                shared::print_homeserver_info(&info);
                println!();
            }

            // Read file
            println!("{}", "▸ File Content".bold());
            println!("   Target: {file_url}");
            println!();

            match client.get_text(&file_url).await {
                Ok(text) => {
                    println!("Status: {}", "success ✓".green());
                    println!("Length: {} bytes", text.len());

                    if text.trim_start().starts_with('{') {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                            let formatted = serde_json::to_string_pretty(&val).unwrap_or(text);
                            println!("\nContent:");
                            for line in formatted.lines() {
                                println!("  {line}");
                            }
                        } else {
                            println!("\n{text}");
                        }
                    } else {
                        println!("\n{text}");
                    }
                }
                Err(e) => {
                    println!("Error: {e}");
                }
            }
        }
        InputType::Url(url_str) => {
            let resolved = match client.resolve_pubky(url_str) {
                Ok(u) => u.to_string(),
                Err(e) => {
                    println!("   Error resolving URL: {e}");
                    return Ok(());
                }
            };
            println!("   Resolved: {resolved}");
            println!();

            match client.get_text(&resolved).await {
                Ok(text) => {
                    println!("Status: {}", "success ✓".green());
                    println!("Length: {} bytes", text.len());

                    if text.trim_start().starts_with('{') {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                            let formatted = serde_json::to_string_pretty(&val).unwrap_or(text);
                            println!("\nContent:");
                            for line in formatted.lines() {
                                println!("  {line}");
                            }
                        } else {
                            println!("\n{text}");
                        }
                    } else {
                        println!("\n{text}");
                    }
                }
                Err(e) => {
                    println!("Error: {e}");
                }
            }
        }
    }

    Ok(())
}
