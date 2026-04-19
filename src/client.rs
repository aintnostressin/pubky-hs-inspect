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
    /// Queries the `_pubky.<z32>` SVCB/HTTPS record directly and
    /// returns the resolved endpoint data.
    pub async fn resolve_pkrr_endpoint(
        &self,
        z32: &str,
    ) -> Option<pkarr::extra::endpoints::Endpoint> {
        let pkarr_client = self.pubky.client().pkarr().clone();
        let qname = format!("_pubky.{z32}");
        pkarr_client.resolve_svcb_endpoint(&qname).await.ok()
    }

    /// Create a read-only PKDNS actor for SDK-level queries.
    pub fn pkdns(&self) -> Pkdns {
        Pkdns::new().expect("failed to create Pkdns actor")
    }

    /// Get the homeserver address (domain or z32 pubkey-as-host) from
    /// the PKRR record for a user public key.
    pub async fn get_homeserver_address(&self, pk: &PublicKey) -> Option<HomeserverInfo> {
        let z32 = pk.z32();
        let qname = format!("_pubky.{z32}");

        // Try the pkarr client for detailed endpoint info
        let pkarr_client = self.pubky.client().pkarr().clone();
        match pkarr_client.resolve_svcb_endpoint(&qname).await {
            Ok(endpoint) => {
                let target = endpoint.target().to_string();
                let domain = endpoint.domain().map(|s| s.to_string());
                let port = endpoint.port();
                let record_pk = endpoint.public_key();

                // The homeserver address is:
                // - domain() if set (ICANN domain)
                // - target if it looks like a z32 pubkey-as-host
                // - the record's own public key z32 as fallback
                let hs_z32 = domain
                    .clone()
                    .or_else(|| {
                        if target == "." || target.contains('.') {
                            domain.clone()
                        } else if is_z32(&target) {
                            Some(target.clone())
                        } else {
                            Some(z32.clone())
                        }
                    })
                    .unwrap_or_else(|| z32.clone());

                Some(HomeserverInfo {
                    user: pk.clone(),
                    user_z32: z32,
                    homeserver_z32: hs_z32,
                    homeserver_domain: domain,
                    port,
                    record_public_key: pubky::PublicKey::from(record_pk),
                })
            }
            Err(_) => {
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
    let z32_chars: std::collections::HashSet<char> =
        "234567abcdefgjkmnpqrtvwxyz".chars().collect();
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
            InputType::PublicKey(k) => assert_eq!(
                k,
                "8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty"
            ),
            _ => panic!("expected PublicKey"),
        }
    }
}
