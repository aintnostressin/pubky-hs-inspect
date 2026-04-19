# Plan: Add `events` CLI command to fetch and print homeserver file change events

## 1. OBJECTIVE

Add a new `events` subcommand that fetches historical file change events (PUT/DEL operations) from a Pubky homeserver and prints them to stdout in a readable, colorized format.

## 2. CONTEXT SUMMARY

The project is a Rust CLI tool (`pubky-hs-inspect`) for inspecting Pubky homeserver instances. It uses `clap` (derive mode) for CLI parsing, `reqwest` for HTTP requests, `tokio` for async runtime, and the `pubky` SDK (v0.7) for network operations.

**Homeserver Events API** (from `pubky-core`):
- **Legacy feed endpoint** — `GET {base_url}/_matrix/client/v3/events/`
  - Query params: `cursor` (default `"0"`, string), `limit` (optional, unsigned int)
  - Response: plain text, one event per line in format `PUT pubky://user/pub/path` or `DEL pubky://user/pub/path`
  - Final line: `cursor: <next_cursor>`
  - Returns events for ALL users on the homeserver (no authentication required)
- **SSE endpoint** — `GET {base_url}/_matrix/client/v3/events-stream`
  - Requires `user` parameter (z32 pubkey), optional `live`, `limit`, `reverse`, `path` params
  - SSE format with multiline data fields (event type + path + cursor + content_hash)
  - Not suitable for v1 "all users" use case — requires per-user filtering

**Relevant file from pubky-core** (user-provided): `pubky-homeserver/src/client_server/routes/events.rs`
- Contains `feed()` function (legacy plain-text endpoint)
- Contains `feed_stream()` function (SSE endpoint)
- `ListQueryParams` extractor provides `cursor` and `limit` params

**Key files:**
- `src/cli.rs` — CLI structure with `clap` subcommands: `Inspect`, `InspectUser`, `Pkdns`, `Storage`, `Version`
- `src/commands.rs` — Command handler implementations
- `src/client.rs` — `Client` wrapper with PKRR resolution and storage operations
- `src/main.rs` — Entry point
- `src/error.rs` — Error type alias (`Result<T> = std::result::Result<T, pubky::Error>`)

## 3. APPROACH OVERVIEW

Use the **legacy plain-text feed endpoint** (`GET /_matrix/client/v3/events/`) for v1. It's the simplest path: no authentication, no user filtering, plain text response that's easy to parse and print.

**Why legacy over SSE:**
- SSE endpoint requires a `user` parameter (per-user filtering only)
- Legacy endpoint returns all events across all users — exactly what's needed
- Legacy response format is a simple, single-line-per-event structure that maps directly to CLI output

**Approach:**
1. Add `Events` subcommand to `cli.rs` with optional `--limit` flag and optional cursor argument
2. Add `get_events()` method to `Client` in `client.rs` — constructs the URL, makes GET request, parses response
3. Add `cmd_events()` handler in `commands.rs` — calls `get_events()`, prints each event with formatting
4. Wire it into `run()` in `commands.rs`

## 4. IMPLEMENTATION STEPS

### Step 1: Add `Events` subcommand to `cli.rs`

**File:** `src/cli.rs`

Add a new variant to the `Commands` enum:

```rust
/// Fetch and print recent file change events from a homeserver
Events {
    /// Maximum number of events to fetch (optional)
    #[arg(short, long, value_name = "N")]
    limit: Option<u64>,

    /// Homeserver key (z32), domain, or URL. Defaults to the global URL argument.
    #[arg(value_name = "HOMESERVER")]
    homeserver: Option<String>,
},
```

The `homeserver` field is optional — if not provided, the command will fall back to the global `Cli::url` field (consistent with how other commands use the global URL when available).

### Step 2: Add `get_events()` method to `Client` in `client.rs`

**File:** `src/client.rs`

Add a new method to the `Client` impl block (near the bottom, before the closing `}`):

```rust
/// Fetch historical events from a homeserver's events feed endpoint.
///
/// Returns a tuple of (event lines, next_cursor).
/// Each event line is in the format: "PUT pubky://user/path" or "DEL pubky://user/path"
pub async fn get_events(
    &self,
    base_url: &str,
    limit: Option<u64>,
) -> Result<(Vec<String>, Option<String>)> {
    let mut url = format!("{base_url}/_matrix/client/v3/events/");
    let mut query_parts = Vec::new();

    if let Some(l) = limit {
        query_parts.push(format!("limit={}", l));
    }

    if !query_parts.is_empty() {
        url.push('?');
        url.push_str(&query_parts.join("&"));
    }

    let resp = reqwest::get(&url).await?;
    let text = resp.text().await?;

    // Parse response: last line is "cursor: N", rest are events
    let mut events = Vec::new();
    let mut next_cursor: Option<String> = None;

    for line in text.lines() {
        if line.starts_with("cursor: ") {
            next_cursor = Some(line["cursor: ".len()..].trim().to_string());
        } else if !line.is_empty() {
            events.push(line.to_string());
        }
    }

    Ok((events, next_cursor))
}
```

**Note:** This method uses `reqwest::get` directly (available via the `reqwest` dependency already in `Cargo.toml`). Alternatively, it could use `self.pubky().client().get(...)` for consistency with the existing client pattern. Either approach works.

### Step 3: Add `cmd_events()` handler in `commands.rs`

**File:** `src/commands.rs`

Add the command handler function:

```rust
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
            // Fallback: use the global URL if available
            // We need to get the url from somewhere — either pass it in or use a default
            // For now, require the homeserver argument
            eprintln!("{}", "Error: homeserver address required. Provide as argument or via -u/--url.".yellow());
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
```

Add the helper functions (near the existing helpers section):

```rust
/// Resolve a homeserver identifier to a full HTTP base URL.
/// Tries the input directly as a URL, or resolves it via PKRR if it's a z32 key.
async fn resolve_homeserver_url(client: &Client, input: &str) -> Result<String> {
    // If it looks like a URL, use it directly
    if input.starts_with("http://") || input.starts_with("https://") {
        Ok(input.trim_end_matches('/').to_string())
    } else {
        // Try to resolve via PKRR
        if let Some(info) = client.get_homeserver_address(&pubky::PublicKey::try_from(input)?).await {
            if let Some(domain) = info.homeserver_domain {
                Ok(format!("https://{domain}"))
            } else {
                Ok(format!("https://_pubky.{}", info.homeserver_z32))
            }
        } else {
            // Fall back to treating it as a domain
            Ok(format!("https://{input}"))
        }
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
```

### Step 4: Wire `Events` into `run()` in `commands.rs`

**File:** `src/commands.rs`

Update the match in `run()`:

```rust
Some(Commands::Events { homeserver, limit }) => {
    cmd_events(homeserver.as_deref(), limit).await
}
```

The `run()` function needs to pass the global `Cli::url` as the `homeserver` fallback. Update the dispatch:

```rust
pub async fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Commands::Events { homeserver, limit }) => {
            // Use global URL as fallback if homeserver not provided
            let target = homeserver.as_deref().or(cli.url.as_deref()).or(Some(""));
            cmd_events(target, limit.clone()).await
        }
        // ... other arms unchanged
    }
}
```

### Step 5: Update `README.md`

**File:** `README.md`

- Add `events` to the command table
- Add a usage example showing fetching events from a homeserver

```markdown
| Command | Description |
|---------|-------------|
| `events` | Fetch and print file change events (PUT/DEL) from a homeserver |
```

Example:
```bash
$ pubky-hs-inspect events 9kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx --limit 10

═══ Homeserver Events ═══

Fetching events from https://myhomeserver.pubky.app

Total events: 10

  PUT pubky://o1gg96ewuojmopcjbz8895478wdtxtzzuxnfjjz8o8e77csa1ngo/pub/photo.jpg
  DEL pubky://3kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx/pub/old.txt
  PUT pubky://5kx3kz7y2jvm4h8qgdp1fwbncs5e6tuxragwoidz8h73bqy41vfx/pub/doc.pdf

  Next cursor: 12345
```

## 5. TESTING AND VALIDATION

1. **Build verification:** Run `cargo build` to ensure the project compiles without errors.

2. **CLI help output:** Run `pubky-hs-inspect events --help` to verify:
   - `--limit` / `-l` flag is documented
   - Optional `HOMESERVER` positional argument is documented
   - Description matches "Fetch and print recent file change events"

3. **Local homeserver test:** Test against one of the available local homeservers:
   ```bash
   pubky-hs-inspect events http://localhost:42363 --limit 5
   ```
   This verifies the direct URL path works.

4. **Key-based resolution test:** Test with a z32 homeserver key to verify PKRR resolution:
   ```bash
   pubky-hs-inspect events <homeserver_z32_key>
   ```
   This verifies the PKRR→domain resolution path.

5. **Edge case - empty events:** Test with a homeserver that has no events to verify the "no events found" message.

6. **Edge case - invalid URL:** Test with an unresolvable key to verify error handling.

7. **Unit tests:** No new unit tests required for v1, but the existing `cargo test` suite should still pass.
