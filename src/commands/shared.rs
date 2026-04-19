use colored::Colorize;

use crate::client::HomeserverInfo;

pub fn print_homeserver_info(info: &HomeserverInfo) {
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

pub fn strip_pubky_scheme(input: &str) -> String {
    if input.starts_with("pubky://") {
        input.strip_prefix("pubky://").unwrap_or(input).to_string()
    } else if input.starts_with("pubky<") && input.ends_with('>') {
        input[6..input.len() - 1].to_string()
    } else {
        input.to_string()
    }
}

pub fn cmd_version() -> crate::error::Result<()> {
    println!("pubky-hs-inspect {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
