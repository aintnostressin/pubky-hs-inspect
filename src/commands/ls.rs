use colored::Colorize;

use crate::client::{parse_input, Client, InputType};
use crate::commands::shared;
use crate::error::Result;

/// List files under a path for a user's storage.
pub async fn cmd_ls(input: &str, path: &str) -> Result<()> {
    let client = Client::new()?;
    println!("{}", "═══ Storage File Listing ═══".bold().cyan());
    println!();

    let parsed = parse_input(input);

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
            let storage_addr = format!("pubky://{z32}{path}");

            // Resolve homeserver
            if let Some(info) = client.get_homeserver_address(&pk).await {
                println!("{}", "▸ Homeserver".bold());
                shared::print_homeserver_info(&info);
                println!();
            }

            // Listing
            println!("{}", "▸ Listing".bold());
            println!("   Target: {storage_addr}");
            println!();

            match client.list(&storage_addr).await {
                Ok(entries) if !entries.is_empty() => {
                    let lines = client.format_list(&entries);
                    println!("   Total entries: {}", entries.len());
                    println!();
                    for line in lines {
                        println!("{}", line);
                    }
                }
                Ok(_) => {
                    println!("   {}", "no entries found".yellow());
                }
                Err(e) => {
                    println!("   Error: {}", e);
                }
            }
        }
        InputType::Url(url_str) => {
            let parsed_url = match client.resolve_pubky(url_str) {
                Ok(u) => u.to_string(),
                Err(e) => {
                    println!("   Error resolving URL: {e}");
                    return Ok(());
                }
            };
            println!("   Target: {parsed_url}");
            println!();

            match client.list(&parsed_url).await {
                Ok(entries) if !entries.is_empty() => {
                    let lines = client.format_list(&entries);
                    println!("   Total entries: {}", entries.len());
                    println!();
                    for line in lines {
                        println!("{}", line);
                    }
                }
                Ok(_) => {
                    println!("   {}", "no entries found".yellow());
                }
                Err(e) => {
                    println!("   Error: {}", e);
                }
            }
        }
    }

    Ok(())
}
