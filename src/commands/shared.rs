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
pub async fn resolve_homeserver_url(
    client: &crate::client::Client,
    input: &str,
) -> crate::error::Result<String> {
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

/// State for accumulating a single SSE event during parsing.
#[derive(Default)]
pub struct SseEventAccumulator {
    path: Option<String>,
    cursor: Option<u64>,
    hash: Option<String>,
}

impl SseEventAccumulator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SseEventAccumulator {
    /// Process a single SSE line. Returns true if the line was blank (event boundary).
    pub fn process_line(&mut self, line: &str) -> bool {
        if line.is_empty() {
            return true;
        }
        if let Some(rest) = line.strip_prefix("path: ") {
            self.path = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("cursor: ") {
            if let Ok(cursor) = rest.trim().parse::<u64>() {
                self.cursor = Some(cursor);
            }
        } else if let Some(rest) = line.strip_prefix("content_hash: ") {
            self.hash = Some(rest.to_string());
        }
        false
    }

    /// Try to build a complete event from accumulated state. Returns `Some` when ready.
    pub fn try_emit(&mut self) -> Option<SseEvent> {
        self.path
            .take()
            .zip(self.cursor.take())
            .map(|(path, cursor)| SseEvent {
                path,
                cursor,
                content_hash: self.hash.take(),
            })
    }
}

/// Parse a batch of SSE lines into a vector of parsed events.
/// Format per event:
///   path: <event_type> <path>
///   cursor: <number>
///   content_hash: <base64>  (optional)
///   <blank line separates events>
pub fn parse_sse_batch(text: &str) -> Vec<SseEvent> {
    let mut acc = SseEventAccumulator::new();
    let mut events = Vec::new();

    for line in text.lines() {
        if acc.process_line(line.trim_end()) {
            // Blank line = event boundary
            if let Some(event) = acc.try_emit() {
                events.push(event);
            }
        }
    }

    // Emit any remaining event (no trailing blank line)
    if let Some(event) = acc.try_emit() {
        events.push(event);
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

/// Async stream that parses SSE events from a bytes stream.
/// Uses `SseEventAccumulator` for incremental line parsing.
pub struct SseEventStream {
    body: futures::stream::BoxStream<'static, std::result::Result<bytes::Bytes, reqwest::Error>>,
    eof: bool,
    line_buf: String,
    acc: SseEventAccumulator,
}

impl SseEventStream {
    pub fn new<B>(body: B) -> Self
    where
        B: futures::Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>>
            + Send
            + 'static,
    {
        Self {
            body: Box::pin(body),
            eof: false,
            line_buf: String::new(),
            acc: SseEventAccumulator::new(),
        }
    }
}

impl futures::Stream for SseEventStream {
    type Item = std::result::Result<SseEvent, pubky::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        loop {
            // If EOF, emit any pending event and end
            if self.eof {
                return std::task::Poll::Ready(self.acc.try_emit().map(Ok));
            }

            // Poll the body stream
            match std::pin::Pin::new(&mut self.body).poll_next(cx) {
                std::task::Poll::Ready(Some(Ok(bytes))) => {
                    self.line_buf.push_str(&String::from_utf8_lossy(&bytes));

                    // Collect complete lines (avoiding borrow checker issues)
                    let lines: Vec<String> = self
                        .line_buf
                        .split('\n')
                        .map(|s| s.trim_end_matches('\r').to_string())
                        .collect();
                    self.line_buf.clear();

                    // Process all but the last line (which may be partial)
                    for line in lines.iter().take(lines.len().saturating_sub(1)) {
                        if self.acc.process_line(line) {
                            // Blank line = event boundary
                            if let Some(event) = self.acc.try_emit() {
                                return std::task::Poll::Ready(Some(Ok(event)));
                            }
                        }
                    }

                    // Keep the last (partial) line if buffer didn't end with newline
                    if lines.last() == Some(&String::new()) && !self.line_buf.ends_with('\n') {
                        // Empty line means buffer ended with \n, nothing to keep
                    } else {
                        self.line_buf = lines.last().cloned().unwrap_or_default();
                    }
                }
                std::task::Poll::Ready(Some(Err(e))) => {
                    self.eof = true;
                    return std::task::Poll::Ready(Some(Err(pubky::Error::Request(
                        pubky::errors::RequestError::Validation {
                            message: format!("Stream error: {e}"),
                        },
                    ))));
                }
                std::task::Poll::Ready(None) => {
                    // EOF — emit any pending event
                    self.eof = true;
                }
                std::task::Poll::Pending => return std::task::Poll::Pending,
            }
        }
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
