use colored::Colorize;

use crate::client::{parse_input, Client, InputType};
use crate::error::Result;

/// Query PKRR records — show raw endpoint data for _pubky SVCB/HTTPS records.
pub async fn cmd_pkdns(input: &str) -> Result<()> {
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
                let base = if let Some(ref domain) = info.homeserver_domain {
                    format!("https://{domain}/")
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
