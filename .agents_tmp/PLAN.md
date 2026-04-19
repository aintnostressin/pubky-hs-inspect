# Plan: Refactor `inspect` to target homeservers, move user inspection to `inspect-user`

## 1. OBJECTIVE

Refactor the `inspect` subcommand so it inspects a **homeserver instance** (prints how many users are on a given homeserver) instead of inspecting a user account. The current user-focused inspection logic (PKRR resolution, storage listing, transport URL display) should be moved to a new `inspect-user` subcommand.

## 2. CONTEXT SUMMARY

The project is a Rust CLI tool (`pubky-hs-inspect`) that interacts with the Pubky federated network. It uses the `pubky` SDK (v0.7) and `pkarr` (v5) for PKRR record resolution.

**Key files:**
- `src/cli.rs` — Defines the CLI structure using `clap`: currently has `Inspect`, `Pkdns`, `Storage`, `Version` subcommands
- `src/commands.rs` — Implements command handlers: `cmd_inspect`, `cmd_pkdns`, `cmd_storage`, `cmd_version`
- `src/client.rs` — `Client` wrapper around `pubky::Pubky` with PKRR resolution helpers, `HomeserverInfo` struct, input parsing utilities
- `src/main.rs` — Entry point, minimal
- `src/error.rs` — Thin error type alias

**Current `inspect` behavior:** Accepts a user PKRR key, resolves its homeserver via PKRR, shows identity/endpoints/storage listing. This is a **user-centric** operation.

**Target `inspect` behavior:** Accepts a homeserver key (z32 public key) or domain, resolves its own PKRR record, and displays metadata including the user count of that homeserver.

## 3. APPROACH OVERVIEW

1. **Add `InspectUser` subcommand** in `cli.rs` to capture the existing `inspect` functionality.
2. **Rename `cmd_inspect` → `cmd_inspect_user`** in `commands.rs` and wire it to the new subcommand.
3. **Create new `cmd_inspect`** in `commands.rs` that:
   - Accepts a homeserver public key (z32) or domain
   - Resolves the homeserver's own PKRR record
   - Displays homeserver metadata
   - Queries the homeserver for user count (via `/pub/pubky.app/profile.json` or metadata endpoint)
4. **Add `get_homeserver_profile` method** to `Client` in `client.rs` for fetching homeserver metadata.
5. **Update README.md** with the new command structure.

**Why this approach:**
- It cleanly separates user-centric and homeserver-centric concerns
- The existing `Storage` command already serves as a focused, single-purpose command — `InspectUser` follows that same pattern
- The homeserver inspection reuses existing PKRR resolution infrastructure, just targeting the homeserver's own key instead of a user's key

## 4. IMPLEMENTATION STEPS

### Step 1: Update `cli.rs` — Add `InspectUser` subcommand, change `Inspect` description

**File:** `src/cli.rs`

- Change the `Inspect` variant's `about` description to reflect homeserver inspection
- Add a new `InspectUser` variant with the current `inspect` subcommand's behavior description
- The `Inspect` subcommand takes a homeserver key/domain (z32 public key or domain)
- The `InspectUser` subcommand takes a user key (z32 public key or pubky:// URL)

Changes:
```rust
#[command(about = "Inspect and gather information about Pubky homeserver instances")]

// Update existing Inspect
/// Inspect a homeserver — resolve its PKRR, show metadata and user count
Inspect {
    /// Homeserver public key (z32), domain, or pubky:// URL
    #[arg(value_name = "HOMESERVER")]
    url: String,
},

// Add new InspectUser
/// Inspect a Pubky user — resolve their homeserver, show storage and endpoints
InspectUser {
    /// PKRR public key (z32) or pubky:// URL of a user
    #[arg(value_name = "KEY_OR_URL")]
    url: String,
},
```

### Step 2: Update `commands.rs` — Wire new subcommand, create new homeserver inspect

**File:** `src/commands.rs`

**2a.** Update `run()` to dispatch to the new subcommands:
```rust
Some(Commands::Inspect { url }) => cmd_inspect(url).await,
Some(Commands::InspectUser { url }) => cmd_inspect_user(url).await,
```

**2b.** Rename `cmd_inspect` to `cmd_inspect_user` and update its doc comment to reflect it's now a user-focused command.

**2c.** Create new `cmd_inspect` function for homeserver inspection:
- Accept a homeserver key (z32) or domain
- Resolve the homeserver's own PKRR record (similar to existing PKRR resolution but for the homeserver key directly)
- Display homeserver metadata:
  - Homeserver z32 public key
  - Homeserver domain (if available)
  - PKRR record status
  - Base URL
- Query user count:
  - Try to fetch the homeserver profile from `https://{domain}/pub/pubky.app/profile.json` (or `https://_pubky.{z32}/pub/pubky.app/profile.json` for pubkey-as-host)
  - Parse the profile JSON for a user count field
  - If no profile or user count field is found, report "user count unavailable"

```rust
async fn cmd_inspect(input: &str) -> Result<()> {
    let client = Client::new()?;
    println!("{}", "═══ Homeserver Inspection ═══".bold().cyan());
    println!();

    // Parse input as a key or domain
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

            // Try to get profile / user count
            println!("{}", "▸ Metadata".bold());
            let base_url = format!("https://_pubky.{z32}/pub/pubky.app/profile.json");
            match client.get_homeserver_profile(&z32).await {
                Some(profile) => {
                    println!("   Profile URL: {base_url}");
                    if let Some(count) = profile.get("users").and_then(|v| v.as_u64()) {
                        println!("   Users:       {count}");
                    } else if let Some(count) = profile.get("user_count").and_then(|v| v.as_u64()) {
                        println!("   Users:       {count}");
                    } else if let Some(reg) = profile.get("registrations") {
                        println!("   Registrations: {reg}");
                    } else {
                        println!("   Profile:     {}", "found (no user count field)".yellow());
                        // Show abbreviated profile
                        if let Some(name) = profile.get("name").and_then(|v| v.as_str()) {
                            println!("   Name:        {name}");
                        }
                        if let Some(desc) = profile.get("description").and_then(|v| v.as_str()) {
                            let truncated: String = desc.chars().take(80).collect();
                            println!("   Description: {truncated}");
                        }
                    }
                    println!("   Status:      {}", "profile fetched ✓".green());
                }
                None => {
                    println!("   Profile URL: {base_url}");
                    println!("   {}", "profile not available".yellow());
                    println!("   Status:      {}", "no profile found".red());
                }
            }
        }
        InputType::Url(url_str) => {
            // Handle domain URLs like "myhomeserver.pubky.app"
            println!("   Target: {url_str}");
            println!();
            println!("   Note: Input appears to be a URL. For homeserver inspection,");
            println!("   please provide a z32 public key directly.");
        }
    }

    Ok(())
}
```

### Step 3: Update `client.rs` — Add homeserver profile method

**File:** `src/client.rs`

Add a new method to `Client` for fetching the homeserver profile/metadata:

```rust
/// Fetch the homeserver profile/metadata JSON from a known z32 address.
pub async fn get_homeserver_profile(&self, z32: &str) -> Result<Option<serde_json::Value>> {
    let profile_url = format!("https://_pubky.{z32}/pub/pubky.app/profile.json");
    match self.get_json::<serde_json::Value>(&profile_url).await {
        Ok(profile) => Ok(Some(profile)),
        Err(_) => {
            // Also try with pubky:// scheme
            let alt_url = format!("pubky://{z32}/pub/pubky.app/profile.json");
            match self.get_json::<serde_json::Value>(&alt_url).await {
                Ok(profile) => Ok(Some(profile)),
                Err(_) => Ok(None),
            }
        }
    }
}
```

### Step 4: Update `README.md` — Document new commands

**File:** `README.md`

- Update the command table to include `inspect-user`
- Update the `inspect` description to reflect homeserver inspection
- Add a new usage example for `inspect` showing homeserver inspection with user count

## 5. TESTING AND VALIDATION

1. **Build verification:** Run `cargo build` to ensure the project compiles without errors after all changes.

2. **CLI help output:** Run `pubky-hs-inspect --help` to verify:
   - `inspect` now shows homeserver-focused description
   - `inspect-user` appears as a new subcommand with user-focused description
   - `pkdns`, `storage`, `version` remain unchanged

3. **`inspect-user` functionality:** Run `pubky-hs-inspect inspect-user <user_key>` with a known user key (e.g., `8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty`) to verify it produces the same output as the old `inspect` command did.

4. **`inspect` functionality:** Run `pubky-hs-inspect inspect <homeserver_key>` with a known homeserver key to verify it:
   - Resolves the PKRR record for the homeserver
   - Displays homeserver metadata (z32, domain, status)
   - Attempts to fetch the profile and reports user count (or "unavailable" if not present)

5. **Unit tests:** Verify existing tests in `client.rs` (input parsing tests) still pass with `cargo test`. No new tests are strictly required but the existing `test_user_key_parsing` test validates that the key parsing logic works correctly.
