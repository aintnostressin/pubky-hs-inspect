# Plan: Add `ls` command for listing user storage files

## 1. OBJECTIVE

Add a new `ls` (list) subcommand that lists all files and directories at a given path for a Pubky user's public storage, enabling inspection of user storage on a homeserver.

## 2. CONTEXT SUMMARY

The project is a Rust CLI tool (`pubky-hs-inspect`) for inspecting Pubky homeserver instances. It uses:
- The `pubky` SDK (v0.7) for storage and PKRR operations
- The `pkarr` crate (v5) for DNS record resolution
- `clap` for CLI argument parsing with derive macros
- `colored` for terminal output styling

**Key files:**
- `src/cli.rs` — `Cli` struct with `Commands` enum (`Inspect`, `InspectUser`, `Pkdns`, `Storage`, `Version`)
- `src/commands.rs` — Command handlers; the `cmd_storage` function currently lists only the root `/pub/` directory
- `src/client.rs` — `Client` wrapper with `list()` method that calls `pubky::public_storage().list(addr)`; also contains `HomeserverInfo`, input parsing, and PKRR resolution helpers
- `src/main.rs` — Entry point
- `src/error.rs` — Thin `Result<T>` type alias

**Current `storage` behavior limitation:** Only lists entries at the root `/pub/` path (depth=1). There is no way to browse subdirectories or list files recursively.

**Available API:** `Client::list()` already wraps `pubky::public_storage().list(addr)` and returns `Vec<String>` of pubky URLs at a single depth. This is exactly what we need — no additional wrapping required.

## 3. APPROACH OVERVIEW

Add a new `ls` subcommand that:
1. Takes a user public key (z32 or pubky:// URL) and an optional path argument
2. Lists only the immediate contents of the given path (non-recursive, flat listing)
3. Produces a formatted output showing file/directory names with type indicators
4. Reuses the existing `Client::list()` infrastructure directly (no recursion needed)

**Why this approach:**
- The existing `Client::list()` already does exactly what we need — a flat directory listing at a single depth
- Adding a focused `ls` command mirrors the existing `Storage` command's single-purpose design
- Non-recursive keeps the command simple and predictable; users can run `ls` repeatedly to drill down
- This avoids potential performance issues with deep/degenerate directory trees on the homeserver

## 4. IMPLEMENTATION STEPS

### Step 1: Add `Ls` subcommand to `cli.rs`

**File:** `src/cli.rs`

Add a new variant to the `Commands` enum:

```rust
/// List files under a path for a user's storage
Ls {
    /// PKRR public key (z32) or pubky:// URL of a user
    #[arg(value_name = "KEY_OR_URL")]
    url: String,

    /// Path within storage (default: /pub/)
    #[arg(short, long, default_value = "/pub/")]
    path: String,
},
```

### Step 2: Add list formatting method to `client.rs`

**File:** `src/client.rs`

Add a single helper method to format a flat listing as a readable output. No recursive methods needed — just reuse the existing `Client::list()`:

```rust
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
```

### Step 3: Implement `cmd_ls` in `commands.rs`

**File:** `src/commands.rs`

**3a.** Update the `run()` match to dispatch the new command:
```rust
Some(Commands::Ls { url, path }) => cmd_ls(url, path).await,
```

**3b.** Add the `cmd_ls` handler:

```rust
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
```

### Step 4: Update README.md

**File:** `README.md`

- Add `ls` to the command table
- Add a usage example showing the `ls` command with default and custom path:

```bash
$ pubky-hs-inspect ls 8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty

═══ Storage File Listing ═══

▸ Homeserver
   Query key:   8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty
   Homeserver:  9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx
   Status:      resolved ✓

▸ Listing
   Target: pubky://8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty/pub/

   Total entries: 2

   📄 pubky.app/profile.json
   📁 my-app/

$ pubky-hs-inspect ls 8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty --path /pub/my-app/

═══ Storage File Listing ═══

▸ Homeserver
   Query key:   8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty
   Homeserver:  9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx
   Status:      resolved ✓

▸ Listing
   Target: pubky://8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty/pub/my-app/

   Total entries: 3

   📄 config.json
   📁 assets/
   📄 index.html

# Navigate into a subdirectory
$ pubky-hs-inspect ls 8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty --path /pub/my-app/assets/

═══ Storage File Listing ═══

▸ Listing
   Target: pubky://8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty/pub/my-app/assets/

   Total entries: 4

   📄 logo.png
   📄 style.css
   📄 main.js
   📄 favicon.ico
```

## 5. TESTING AND VALIDATION

1. **Build verification:** Run `cargo build` to ensure the project compiles without errors.

2. **CLI help output:** Run `pubky-hs-inspect --help` to verify:
   - `ls` appears as a new subcommand with description "List files under a path for a user's storage"
   - `-p, --path <PATH>` flag is documented with default value `/pub/`
   - All existing commands remain functional

3. **`ls` with default path:** Run `pubky-hs-inspect ls <user_key>` against a known user (e.g., `8um71us3fyw6h8wbcxb5ar3rwusy1a6u49956ikzojg3gcwd1dty` on the available homeserver hosts) to verify it lists only the immediate entries under `/pub/` (not recursive).

4. **`ls` with custom path:** Run `pubky-hs-inspect ls <user_key> --path /pub/some/subdir/` to verify it lists entries under the specified subdirectory.

5. **`ls` with URL input:** Run `pubky-hs-inspect ls pubky://<key>/pub/` to verify URL-style input works.

6. **Drill-down workflow:** Verify the recursive navigation pattern — run `ls` on root, identify a directory, then run `ls --path /pub/that-dir/` to go deeper.

7. **Empty directory handling:** Test with a non-existent path to verify graceful "no entries found" output.

8. **Unit tests:** Existing tests in `client.rs` should still pass.
