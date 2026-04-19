use colored::Colorize;

use crate::client::{parse_input, Client, InputType};
use crate::commands::shared;
use crate::error::Result;

/// Inspect a Pubky user — resolve their homeserver, show storage and endpoints.
pub async fn cmd_inspect_user(input: &str) -> Result<()> {
    let client = Client::new()?;
    println!("{}", "═══ User Inspection ═══".bold().cyan());
    println!();

    let parsed = parse_input(input);

    match &parsed {
        InputType::PublicKey(key_str) => {
            let pk = match pubky::PublicKey::try_from(key_str.as_str()) {
                Ok(pk) => pk,
                Err(e) => {
                    println!("   Error parsing public key: {e}");
                    println!(
                        "   Make sure the input is a valid z32 public key or pubky<pk> identifier."
                    );
                    return Ok(());
                }
            };

            let z32 = pk.z32();

            // Identity
            println!("{}", "▸ Identity".bold());
            println!("   Input:  {key_str}");
            println!("   Z32:    {z32}");
            println!("   PKRR Q: _pubky.{z32}");
            println!();

            // PKRR endpoint resolution
            println!("{}", "▸ PKRR Endpoint Resolution".bold());
            match client.resolve_pkrr_endpoint(&z32).await {
                Some(target) => {
                    let domain = if target.contains('.') {
                        Some(target.clone())
                    } else {
                        None
                    };

                    println!("   Host:      {target}");
                    if let Some(d) = domain {
                        println!("   Domain:    {d}");
                    } else if target.len() == 52 {
                        println!("   (pubkey-as-host)");
                    }
                    println!("   Status:    {}", "PKRR record resolved ✓".green());
                }
                None => {
                    println!("   {}", "no PKRR record found".yellow());
                    println!("   Status:  {}", "unresolvable ✗".red());
                }
            }
            println!();

            // Homeserver resolution
            println!("{}", "▸ Homeserver Resolution".bold());
            if let Some(info) = client.get_homeserver_address(&pk).await {
                shared::print_homeserver_info(&info);
            } else {
                println!("   No homeserver record found for this key.");
            }
            println!();

            // Public storage
            println!("{}", "▸ Public Storage".bold());
            // Use the homeserver domain if available, otherwise fall back to z32
            let pub_addr = if let Some(info) = client.get_homeserver_address(&pk).await {
                if let Some(domain) = &info.homeserver_domain {
                    // Use the resolved homeserver domain directly
                    println!("   Homeserver: {domain}");
                    println!("   URL:        https://{domain}/pub/");

                    // Try using the pubky:// format with z32 (will resolve to _pubky.z32)
                    let fallback = format!("pubky://{z32}/pub/");
                    match client.list(&fallback).await {
                        Ok(entries) if !entries.is_empty() => {
                            println!("   Found {} entry(ies) (via pubky://):", entries.len());
                            for entry in entries.iter().take(10) {
                                println!("     {}", entry);
                            }
                            if entries.len() > 10 {
                                println!("     ... and {} more", entries.len() - 10);
                            }
                        }
                        Ok(_) => {
                            println!("   {}", "no public entries".yellow());
                        }
                        Err(e) => {
                            println!(
                                "   Note: pubky:// resolution failed ({}) - trying direct HTTPS...",
                                e
                            );
                            // Try direct HTTPS request to the homeserver
                            let resp_result = async {
                                reqwest::get(format!("https://{domain}/pub/?limit=5")).await
                            }
                            .await;
                            match resp_result {
                                Ok(resp) => {
                                    if resp.status().is_success() {
                                        let body = resp.text().await.unwrap_or_default();
                                        if body.contains("items") || body.starts_with('[') {
                                            println!("   Status:  {}", "found ✓".green());
                                            println!("   Body:    {} bytes", body.len());
                                        } else {
                                            println!(
                                                "   Status:  {}",
                                                "unrecognized response".yellow()
                                            );
                                            println!("   Body:    {} bytes", body.len());
                                        }
                                    } else {
                                        println!(
                                            "   Status:  {} ({})",
                                            "error".red(),
                                            resp.status()
                                        );
                                    }
                                }
                                Err(e) => {
                                    println!("   Error:   {}", e);
                                }
                            }
                        }
                    }
                    println!();
                    return Ok(());
                } else {
                    format!("pubky://{z32}/pub/")
                }
            } else {
                format!("pubky://{z32}/pub/")
            };
            match client.list(&pub_addr).await {
                Ok(entries) if !entries.is_empty() => {
                    println!("   Found {} entry(ies):", entries.len());
                    for entry in entries.iter().take(10) {
                        println!("     {}", entry);
                    }
                    if entries.len() > 10 {
                        println!("     ... and {} more", entries.len() - 10);
                    }
                }
                Ok(_) => {
                    println!("   {}", "no public entries".yellow());
                }
                Err(e) => {
                    println!("   Error listing: {}", e);
                }
            }
        }
        InputType::Url(url_str) => {
            println!("{}", "▸ Identifier".bold());
            println!("   Input: {url_str}");
            println!();

            // Pubky URL resolution
            println!("{}", "▸ Pubky URL Resolution".bold());
            match client.resolve_pubky(url_str) {
                Ok(transport_url) => {
                    println!("   Transport URL: {transport_url}");
                    println!("   Status: {}", "resolved ✓".green());
                }
                Err(e) => {
                    println!("   Error: {}", e);
                    println!("   Status: {}", "unresolvable".red());
                }
            }
            println!();

            // Fetch resource
            println!("{}", "▸ Resource Fetch".bold());
            let addr = shared::strip_pubky_scheme(url_str);
            match client.get_text(&addr).await {
                Ok(text) => {
                    println!("   Status: {}", "success ✓".green());
                    println!("   Body length: {} bytes", text.len());
                    if text.trim_start().starts_with('{') {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                            let formatted = serde_json::to_string_pretty(&val).unwrap_or(text);
                            println!("   Content:");
                            for line in formatted.lines().take(20) {
                                println!("     {line}");
                            }
                        } else {
                            println!("   Content:\n     {text}");
                        }
                    } else {
                        println!("   Content:\n     {text}");
                    }
                }
                Err(e) => {
                    println!("   Status: {}", "failed ✗".red());
                    println!("   Error: {}", e);
                }
            }
        }
    }

    Ok(())
}
