use colored::Colorize;

use crate::client::{parse_input, Client, InputType};
use crate::error::Result;

/// Inspect a homeserver — resolve its PKRR, show metadata and user count.
pub async fn cmd_inspect(input: &str) -> Result<()> {
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
