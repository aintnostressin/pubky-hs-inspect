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

/// Resolve a homeserver identifier to a full HTTP base URL.
/// Tries the input directly as a URL, or resolves it via PKRR if it's a z32 key.
pub async fn resolve_homeserver_url(client: &crate::client::Client, input: &str) -> crate::error::Result<String> {
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

/// Parse a batch of SSE lines into a vector of parsed events.
/// Format per event:
///   path: <event_type> <path>
///   cursor: <number>
///   content_hash: <base64>  (optional)
///   <blank line separates events>
pub fn parse_sse_batch(text: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_cursor: Option<u64> = None;
    let mut current_hash: Option<String> = None;

    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            // End of event block — emit if we have data
            if let (Some(path), Some(cursor)) = (current_path.take(), current_cursor.take()) {
                events.push(SseEvent {
                    path,
                    cursor,
                    content_hash: current_hash.take(),
                });
            }
        } else if let Some(rest) = line.strip_prefix("path: ") {
            current_path = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("cursor: ") {
            if let Ok(cursor) = rest.trim().parse::<u64>() {
                current_cursor = Some(cursor);
            }
        } else if let Some(rest) = line.strip_prefix("content_hash: ") {
            current_hash = Some(rest.trim().to_string());
        }
    }

    // Emit any remaining event (no trailing blank line)
    if let (Some(path), Some(cursor)) = (current_path, current_cursor) {
        events.push(SseEvent {
            path,
            cursor,
            content_hash: current_hash,
        });
    }

    events
}

/// A parsed SSE event from the /events-stream endpoint.
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// The event path (e.g. "PUT /pub/my-file.txt")
    pub path: String,
    /// The event cursor position
    pub cursor: u64,
    /// Optional base64-encoded content hash
    pub content_hash: Option<String>,
}

/// Print a single SSE event with color coding.
pub fn print_sse_event(event: &SseEvent) {
    if let Some(event_type) = event.path.split_whitespace().next() {
        let rest = &event.path[event_type.len()..].trim();
        let base = match event_type {
            "PUT" => event_type.green(),
            "DEL" => event_type.red(),
            _ => event_type.to_string().into(),
        };
        print!("  {base} {rest}  cursor={}", event.cursor);
        if let Some(ref hash) = event.content_hash {
            print!("  hash={hash}");
        }
        println!();
    } else {
        println!("  {}", event.path);
    }
}

// ── Unit tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_single_event_no_hash() {
        let input = "path: PUT /pub/my-file.txt\ncursor: 42\n";
        let events = parse_sse_batch(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].path, "PUT /pub/my-file.txt");
        assert_eq!(events[0].cursor, 42);
        assert!(events[0].content_hash.is_none());
    }

    #[test]
    fn test_parse_sse_single_event_with_hash() {
        let input = "path: PUT /pub/my-file.txt\ncursor: 42\ncontent_hash: YWJjZGVm\n";
        let events = parse_sse_batch(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].path, "PUT /pub/my-file.txt");
        assert_eq!(events[0].cursor, 42);
        assert_eq!(events[0].content_hash, Some("YWJjZGVm".to_string()));
    }

    #[test]
    fn test_parse_sse_multiple_events() {
        let input = "path: PUT /pub/file1.txt\ncursor: 42\n\npath: DEL /pub/file2.txt\ncursor: 43\ncontent_hash: c29tZWhhc2g=\n";
        let events = parse_sse_batch(input);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].path, "PUT /pub/file1.txt");
        assert_eq!(events[0].cursor, 42);
        assert!(events[0].content_hash.is_none());
        assert_eq!(events[1].path, "DEL /pub/file2.txt");
        assert_eq!(events[1].cursor, 43);
        assert_eq!(events[1].content_hash, Some("c29tZWhhc2g=".to_string()));
    }

    #[test]
    fn test_parse_sse_trailing_newline() {
        let input = "path: PUT /pub/file.txt\ncursor: 10\n\n";
        let events = parse_sse_batch(input);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].path, "PUT /pub/file.txt");
        assert_eq!(events[0].cursor, 10);
    }

    #[test]
    fn test_parse_sse_empty_input() {
        let input = "";
        let events = parse_sse_batch(input);
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_sse_only_whitespace() {
        let input = "   \n  \n";
        let events = parse_sse_batch(input);
        assert!(events.is_empty());
    }
}
