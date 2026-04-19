use clap::CommandFactory;
use colored::Colorize;

use crate::cli::{Cli, Commands};
use crate::client::{parse_input, Client, HomeserverInfo, InputType};
use crate::error::Result;

pub async fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Commands::Inspect { url }) => cmd_inspect(url).await,
        Some(Commands::InspectUser { url }) => cmd_inspect_user(url).await,
        Some(Commands::Pkdns { url }) => cmd_pkdns(url).await,
        Some(Commands::Storage { url }) => cmd_storage(url).await,
        Some(Commands::Ls { url, path }) => cmd_ls(url, path).await,
        Some(Commands::Version) => cmd_version(),
        Some(Commands::Events { homeserver, limit }) => {
            // Use global URL as fallback if homeserver not provided
            let target = homeserver.as_deref().or(cli.url.as_deref()).or(Some(""));
            cmd_events(target, *limit).await
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

// ── inspect (homeserver) ─────────────────────────────────────────

/// Inspect a homeserver — resolve its PKRR, show metadata and user count.
async fn cmd_inspect(input: &str) -> Result<()> {
    let client = Client::new()?;
    println!("{}", "═══ Homeserver Inspection ═══".bold().cyan());
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

            // Identity
            println!("{}", "▸ Homeserver Identity".bold());
            println!("   Input:  {key_str}");
            println!("   Z32:    {z32}");
            println!();

            // PKRR resolution for the homeserver itself
            println!("{}", "▸ PKRR Record".bold());
            match client.resolve_pkrr_endpoint(&z32).await {
                Some(target) => {
                    let is_domain = target.contains('.');
                    println!("   Target:    {target}");
                    if is_domain {
                        println!("   Type:      ICANN domain");
                    } else {
                        println!("   Type:      pubkey-as-host");
                    }
                    println!("   Status:    {}", "resolved ✓".green());
                }
                None => {
                    println!("   {}", "no PKRR record found".yellow());
                    println!("   Status:  {}", "unresolvable ✗".red());
                }
            }
            println!();

            // Homeserver info
            println!("{}", "▸ Homeserver Info".bold());
            let hs_url = format!("https://_pubky.{}", z32);
            println!("   Base URL:   {hs_url}");
            println!("   Profile:    {hs_url}/pub/pubky.app/profile.json");
            println!("   Status:     {}", "info available ✓".green());
        }
        InputType::Url(url_str) => {
            println!("   Target: {url_str}");
            println!();
            println!("   Note: Input appears to be a URL. For homeserver inspection,");
            println!("   please provide a z32 public key directly.");
        }
    }

    Ok(())
}

// ── inspect-user ───────────────────────────────────────────────────

/// Inspect a Pubky user — resolve their homeserver, show storage and endpoints.
async fn cmd_inspect_user(input: &str) -> Result<()> {
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
                print_homeserver_info(&info);
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
                            match reqwest::get(format!("https://{domain}/pub/?limit=5")).await {
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
            let addr = strip_pubky_scheme(url_str);
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

// ── pkdns ──────────────────────────────────────────────────────────

/// Query PKRR records — show raw endpoint data for _pubky SVCB/HTTPS records.
async fn cmd_pkdns(input: &str) -> Result<()> {
    let client = Client::new()?;
    println!("{}", "═══ PKRR Record Query ═══".bold().cyan());
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
            let qname = format!("_pubky.{z32}");

            println!("Querying PKRR record: {qname}");
            println!();

            // Low-level endpoint resolution
            println!("{}", "▸ Endpoint Resolution".bold());
            match client.resolve_pkrr_endpoint(&z32).await {
                Some(target) => {
                    let domain_str = if target.contains('.') {
                        target.clone()
                    } else {
                        "(pubkey-as-host)".to_string()
                    };
                    println!("   Host:    {target}");
                    println!("   Domain:  {domain_str}");
                }
                None => {
                    println!("   {}", "no PKRR record found".yellow());
                }
            }
            println!();

            // SDK resolution
            println!("{}", "▸ SDK Resolution".bold());
            match client.get_homeserver_of(&pk).await {
                Some(hs) => {
                    println!("   Homeserver PK: {}", hs);
                    println!("   Homeserver Z32: {}", hs.z32());
                }
                None => {
                    println!("   {}", "no homeserver record".yellow());
                }
            }

            // Full transport URL
            if let Some(info) = client.get_homeserver_address(&pk).await {
                println!();
                println!("{}", "▸ Transport URL".bold());
                let base = if info.homeserver_domain.is_some() {
                    format!("https://{}/", info.homeserver_z32)
                } else {
                    format!("https://_pubky.{}/", info.homeserver_z32)
                };
                println!("   Base:    {base}");
                println!("   Profile: {base}pub/pubky.app/profile.json");
            }
        }
        InputType::Url(url_str) => {
            println!("Input: {url_str}");
            println!();

            println!("{}", "▸ Pubky Resolution".bold());
            match client.resolve_pubky(url_str) {
                Ok(url) => {
                    println!("   Transport URL: {url}");
                }
                Err(e) => {
                    println!("   Error: {e}");
                }
            }
        }
    }

    Ok(())
}

// ── storage ────────────────────────────────────────────────────────

/// Inspect public storage for a PKRR public key.
async fn cmd_storage(input: &str) -> Result<()> {
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
                print_homeserver_info(&info);
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
            let addr = strip_pubky_scheme(url_str);
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

// ── ls ────────────────────────────────────────────────────────────

/// List files under a path for a user's storage.
async fn cmd_ls(input: &str, path: &str) -> Result<()> {
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
                print_homeserver_info(&info);
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

// ── events ───────────────────────────────────────────────────────

/// Fetch and print recent file change events from a homeserver.
async fn cmd_events(homeserver: Option<&str>, limit: Option<u64>) -> Result<()> {
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

    let (events, next_cursor) = match client.get_events(&base_url, limit).await {
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

// ── helpers ────────────────────────────────────────────────────────

fn print_homeserver_info(info: &HomeserverInfo) {
    println!("   Query key:   {}", info.user);
    println!("   Homeserver:  {}", info.homeserver_z32);
    if let Some(ref domain) = info.homeserver_domain {
        println!("   Domain:      {domain}");
    }
    if let Some(port) = info.port {
        println!("   Port:        {port}");
    }
    println!("   Record PK:   {}", info.record_public_key);
    println!("   Status:      {}", "resolved ✓".green());

    let base = if info.homeserver_domain.is_some() {
        format!("https://{}/", info.homeserver_z32)
    } else {
        format!("https://_pubky.{}/", info.homeserver_z32)
    };
    println!("   Base URL:    {base}");
    println!("   Profile:     {base}pub/pubky.app/profile.json");
}

fn strip_pubky_scheme(input: &str) -> String {
    if input.starts_with("pubky://") {
        input.strip_prefix("pubky://").unwrap_or(input).to_string()
    } else if input.starts_with("pubky<") && input.ends_with('>') {
        input[6..input.len() - 1].to_string()
    } else {
        input.to_string()
    }
}

fn cmd_version() -> Result<()> {
    println!("pubky-hs-inspect {}", env!("CARGO_PKG_VERSION"));
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
