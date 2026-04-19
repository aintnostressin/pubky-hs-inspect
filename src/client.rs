use pkarr::dns::rdata::RData;
use pkarr::SignedPacket;
use pubky::Pubky;
use pubky::{Pkdns, PublicKey};

use crate::error::Result;

/// Client wrapper around the pubky SDK with PKRR resolution helpers.
pub struct Client {
    pubky: Pubky,
}

impl Client {
    /// Create a new client using the pubky SDK (mainnet).
    pub fn new() -> Result<Self> {
        let pubky = Pubky::new()?;
        Ok(Self { pubky })
    }

    /// Return a reference to the underlying pubky facade.
    pub fn pubky(&self) -> &Pubky {
        &self.pubky
    }

    // ── PKRR / PKDNS resolution ──────────────────────────────────

    /// High-level: resolve the `_pubky` SVCB record for a public key,
    /// returning the homeserver public key it points to (if any).
    pub async fn resolve_homeserver(&self, pk: &PublicKey) -> Option<PublicKey> {
        self.pubky.get_homeserver_of(pk).await
    }

    /// Low-level PKRR endpoint resolution via the pkarr crate.
    /// Resolves the public key packet directly and extracts _pubky SVCB/HTTPS records.
    /// Returns the homeserver host as a string (domain or z32 pubkey-as-host).
    pub async fn resolve_pkrr_endpoint(&self, z32: &str) -> Option<String> {
        let pkarr_client = self.pubky.client().pkarr().clone();

        // Parse the public key and resolve the packet directly
        if let Ok(pk) = pkarr::PublicKey::try_from(z32) {
            if let Some(packet) = pkarr_client.resolve(&pk).await {
                // Extract _pubky host from the packet
                return extract_host_from_packet(&packet);
            }
        }
        None
    }

    /// Create a read-only PKDNS actor for SDK-level queries.
    pub fn pkdns(&self) -> Pkdns {
        Pkdns::new().expect("failed to create Pkdns actor")
    }

    /// Get the homeserver address (domain or z32 pubkey-as-host) from
    /// the PKRR record for a user public key.
    pub async fn get_homeserver_address(&self, pk: &PublicKey) -> Option<HomeserverInfo> {
        let z32 = pk.z32();
        let pkarr_client = self.pubky.client().pkarr().clone();

        // Resolve the packet directly from the public key (not _pubky.<key>)
        if let Ok(pkarr_pk) = pkarr::PublicKey::try_from(&z32) {
            if let Some(packet) = pkarr_client.resolve(&pkarr_pk).await {
                // Extract _pubky host from the packet
                if let Some(target) = extract_host_from_packet(&packet) {
                    // Determine if target is a domain or a pubkey-as-host
                    let is_domain = target.contains('.');
                    let is_z32 = is_z32(&target);

                    let hs_z32 = if is_domain || is_z32 {
                        target.clone()
                    } else {
                        z32.clone()
                    };

                    let domain = if is_domain {
                        Some(target.clone())
                    } else {
                        None
                    };

                    // Parse the homeserver as a PublicKey if it looks like one
                    let record_pk = pubky::PublicKey::try_from_z32(&hs_z32).ok();

                    return Some(HomeserverInfo {
                        user: pk.clone(),
                        user_z32: z32,
                        homeserver_z32: hs_z32,
                        homeserver_domain: domain,
                        port: None,
                        record_public_key: record_pk.unwrap_or_else(|| pk.clone()),
                    });
                }
            }
        }

        // Fallback: high-level SDK resolution
        if let Some(hs_pk) = self.resolve_homeserver(pk).await {
            let hs_z32 = hs_pk.z32();
            Some(HomeserverInfo {
                user: pk.clone(),
                user_z32: z32,
                homeserver_z32: hs_z32,
                homeserver_domain: None,
                port: None,
                record_public_key: hs_pk,
            })
        } else {
            None
        }
    }

    /// Re-exported: resolve the `_pubky` SVCB/HTTPS record for a public key
    /// via the pubky SDK's high-level API.
    pub async fn get_homeserver_of(&self, pk: &PublicKey) -> Option<PublicKey> {
        self.pubky.get_homeserver_of(pk).await
    }

    /// Resolve a Pubky identifier (pubky://<user>/path or pubky<pk>/path)
    /// into a transport URL.
    pub fn resolve_pubky(&self, input: &str) -> Result<url::Url> {
        let url = pubky::resolve_pubky(input)?;
        Ok(url)
    }

    // ── Public storage operations ────────────────────────────────

    /// Fetch and parse JSON from a public storage address.
    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, addr: &str) -> Result<T> {
        let storage = self.pubky.public_storage();
        let data = storage.get_json(addr).await?;
        Ok(data)
    }

    /// GET a public storage resource and return its text body.
    pub async fn get_text(&self, addr: &str) -> Result<String> {
        let storage = self.pubky.public_storage();
        let resp = storage.get(addr).await?;
        let text = resp.text().await?;
        Ok(text)
    }

    /// HEAD check — does a resource exist?
    pub async fn exists(&self, addr: &str) -> Result<bool> {
        let storage = self.pubky.public_storage();
        storage.exists(addr).await
    }

    /// List directory entries under a public storage address.
    pub async fn list(&self, addr: &str) -> Result<Vec<String>> {
        let storage = self.pubky.public_storage();
        let builder = storage.list(addr)?;
        let entries = builder.limit(200).send().await?;
        let paths: Vec<String> = entries.into_iter().map(|e| e.to_pubky_url()).collect();
        Ok(paths)
    }

    /// Format a flat listing of pubky URLs as a table with file/directory indicators.
    pub fn format_list(&self, entries: &[String]) -> Vec<String> {
        let mut lines = Vec::new();
        for entry_url in entries {
            // Check if entry is a directory (ends with '/')
            let is_dir = entry_url.ends_with('/');
            let icon = if is_dir { "📁" } else { "📄" };
            // Strip pubky:// scheme and leading slash to get relative name
            let name = entry_url.strip_prefix("pubky://").unwrap_or(entry_url);
            let name = name.strip_prefix('/').unwrap_or(name);
            lines.push(format!("   {icon} {name}"));
        }
        lines
    }

    // ── Helpers ──────────────────────────────────────────────────

    /// Build a transport URL for a resource on a given user's space.
    /// `{user_z32}/pub/pubky.app/profile.json` → `https://_pubky.{user_z32}/pub/pubky.app/profile.json`
    pub fn transport_url(&self, user_z32: &str, path: &str) -> String {
        format!("https://_pubky.{user_z32}/{path}")
    }

    /// Build the homeserver base URL from resolved info.
    pub fn homeserver_url(&self, info: &HomeserverInfo) -> String {
        if let Some(ref domain) = info.homeserver_domain {
            format!("https://{domain}/")
        } else {
            format!("https://_pubky.{}/", info.homeserver_z32)
        }
    }

    /// Build a full resource URL on the homeserver.
    pub fn homeserver_resource(&self, info: &HomeserverInfo, path: &str) -> String {
        let base = self.homeserver_url(info);
        format!("{base}{path}")
    }

    /// Fetch the homeserver profile/metadata JSON from a known z32 address.
    pub async fn get_homeserver_profile(&self, z32: &str) -> Option<serde_json::Value> {
        let profile_url = format!("https://_pubky.{z32}/pub/pubky.app/profile.json");
        if let Ok(profile) = self.get_json::<serde_json::Value>(&profile_url).await {
            return Some(profile);
        }
        // Also try with pubky:// scheme
        let alt_url = format!("pubky://{z32}/pub/pubky.app/profile.json");
        self.get_json::<serde_json::Value>(&alt_url).await.ok()
    }
}

/// Information about a homeserver resolved from a PKRR record.
#[derive(Debug)]
pub struct HomeserverInfo {
    /// The user whose `_pubky` record was queried.
    pub user: PublicKey,
    /// The user's z32 public key.
    pub user_z32: String,
    /// The homeserver's z32 address (pubkey-as-host).
    pub homeserver_z32: String,
    /// Optional ICANN domain name for the homeserver.
    pub homeserver_domain: Option<String>,
    /// Optional port from the SVCB record.
    pub port: Option<u16>,
    /// The public key from the PKRR record.
    pub record_public_key: PublicKey,
}

impl HomeserverInfo {
    /// Full transport URL for a resource on this homeserver.
    pub fn resource_url(&self, path: &str) -> String {
        let base = if let Some(ref domain) = self.homeserver_domain {
            format!("https://{domain}/")
        } else {
            format!("https://_pubky.{}/", self.homeserver_z32)
        };
        format!("{base}{path}")
    }
}

/// Extract the homeserver host from a PKRR packet's _pubky records.
/// This mirrors the logic in `pubky::actors::pkdns::extract_host_from_packet`.
///
/// If no _pubky records are found, also checks for records at the zone apex
/// (the public key name itself) since some PKRR deployments store records there.
fn extract_host_from_packet(packet: &SignedPacket) -> Option<String> {
    // First, try _pubky records (standard PKRR location)
    for rr in packet.resource_records("_pubky") {
        match &rr.rdata {
            RData::SVCB(svcb) => {
                let target = svcb.target.to_string();
                // Skip root targets (empty string or ".")
                if !target.is_empty() && target != "." {
                    return Some(target);
                }
            }
            RData::HTTPS(https) => {
                let target = https.0.target.to_string();
                if !target.is_empty() && target != "." {
                    return Some(target);
                }
            }
            _ => {}
        }
    }

    // Fallback: check for records at the zone apex (root)
    // This handles non-standard PKRR deployments that store records at the key itself
    for rr in packet.resource_records("@") {
        match &rr.rdata {
            RData::SVCB(svcb) => {
                let target = svcb.target.to_string();
                if !target.is_empty() && target != "." {
                    return Some(target);
                }
            }
            RData::HTTPS(https) => {
                let target = https.0.target.to_string();
                if !target.is_empty() && target != "." {
                    return Some(target);
                }
            }
            _ => {}
        }
    }

    None
}

// ── Input parsing ──────────────────────────────────────────────────

/// Try to parse a user-friendly input as a PublicKey (with or without
/// pubky<…> prefix). Falls back to treating it as a URL string.
pub fn parse_input(input: &str) -> InputType {
    let stripped = input
        .strip_prefix("pubky<")
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(input);

    if stripped.is_empty() {
        return InputType::Url(input.to_string());
    }

    if is_potential_key(stripped) && PublicKey::try_from(stripped).is_ok() {
        return InputType::PublicKey(input.to_string());
    }

    InputType::Url(input.to_string())
}

fn is_valid_z32(s: &str) -> bool {
    let z32_chars: std::collections::HashSet<char> = "234567abcdefgjkmnpqrtvwxyz".chars().collect();
    !s.is_empty() && s.chars().all(|c| z32_chars.contains(&c))
}

/// Check if a string could be a public key (52 chars, no slashes).
/// This is a pre-check before calling `PublicKey::try_from()`.
fn is_potential_key(s: &str) -> bool {
    !s.contains('/') && s.len() == 52
}

fn is_z32(s: &str) -> bool {
    is_valid_z32(s) && s.len() >= 10
}

pub fn looks_like_pubkey(s: &str) -> bool {
    let stripped = s
        .strip_prefix("pubky<")
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(s);
    !stripped.is_empty() && !stripped.contains('/') && is_potential_key(stripped)
}

/// Whether the user provided a pubkey or a URL.
#[derive(Clone, Debug)]
pub enum InputType {
    PublicKey(String),
    Url(String),
}

impl InputType {
    pub fn is_pubkey(&self) -> bool {
        matches!(self, InputType::PublicKey(_))
    }

    pub fn is_url(&self) -> bool {
        matches!(self, InputType::Url(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_input_url() {
        let parsed = parse_input("https://example.pubky.app");
        assert!(parsed.is_url());
        match parsed {
            InputType::Url(u) => assert_eq!(u, "https://example.pubky.app"),
            _ => panic!("expected Url"),
        }
    }

    #[test]
    fn test_parse_input_path() {
        let parsed =
            parse_input("234567abcdefgjkmnpqrtvwxyz234567abcdefgjkmnpqrtvwxyz/pub/my-app/file.txt");
        assert!(parsed.is_url());
    }

    #[test]
    fn test_is_valid_z32() {
        assert!(is_valid_z32(
            "234567abcdefgjkmnpqrtvwxyz234567abcdefgjkmnpqrtvwxyz"
        ));
        assert!(!is_valid_z32("https://example.com"));
        assert!(!is_valid_z32("pubky<key>"));
        assert!(!is_valid_z32(""));
    }

    #[test]
    fn test_looks_like_pubkey_valid() {
        assert!(looks_like_pubkey(
            "234567abcdefgjkmnpqrtvwxyz234567abcdefgjkmnpqrtvwxyz"
        ));
        assert!(looks_like_pubkey(
            "pubky<234567abcdefgjkmnpqrtvwxyz234567abcdefgjkmnpqrtvwxyz>"
        ));
    }

    #[test]
    fn test_looks_like_pubkey_invalid() {
        assert!(!looks_like_pubkey("abc123"));
        assert!(!looks_like_pubkey(
            "234567abcdefgjkmnpqrtvwxyz234567abcdefgjkmnpqrtvwxyz/pub/file"
        ));
    }

    #[test]
    fn test_user_key_parsing() {
        let parsed = parse_input("8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty");
        assert!(
            parsed.is_pubkey(),
            "User's key should be recognized as pubkey"
        );
        match parsed {
            InputType::PublicKey(k) => {
                assert_eq!(k, "8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty")
            }
            _ => panic!("expected PublicKey"),
        }
    }
}
