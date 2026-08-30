//! WHATWG HTML §9.2 Server-Sent Events frame parser.
//!
//! Donor: `legacy/donor/apeireth-mcp/src/transport/sse.rs` (`parse_sse_frame`,
//! `absolutize_endpoint`, frame-separator scan).
//!
//! Recovered **without reqwest**. The donor `SseTransport` is deferred: it
//! is a live HTTP client and would pull a new dependency plus a parallel
//! host. A later transport can feed bytes into [`SseBuffer`].

use std::time::Duration;

/// One parsed SSE frame.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SseFrame {
    pub event: Option<String>,
    pub data_lines: Vec<String>,
    /// `id:` field (Last-Event-ID). Donor ignored this; we keep it so
    /// reconnect can resume.
    pub id: Option<String>,
    /// `retry:` field, milliseconds. Donor ignored this.
    pub retry: Option<Duration>,
}

impl SseFrame {
    /// Join `data:` lines with `\n` (WHATWG: data fields concatenated).
    pub fn data(&self) -> String {
        self.data_lines.join("\n")
    }

    pub fn is_message(&self) -> bool {
        self.event.as_deref() == Some("message")
    }

    pub fn is_endpoint(&self) -> bool {
        self.event.as_deref() == Some("endpoint")
    }
}

/// Parse one frame's text (no trailing blank-line separator required).
///
/// - `event:value` → event
/// - `data:value` → data line; one leading space after `data:` is stripped
/// - `id:value` → last-event-id
/// - `retry:value` → reconnect hint
/// - `:comment` and unknown fields → ignored
pub fn parse_sse_frame(text: &str) -> SseFrame {
    let mut frame = SseFrame::default();
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if let Some(_rest) = line.strip_prefix(':') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("event:") {
            // WHATWG: a single leading U+0020 after the colon is optional.
            let v = rest.strip_prefix(' ').unwrap_or(rest);
            frame.event = Some(v.to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("data:") {
            let v = rest.strip_prefix(' ').unwrap_or(rest);
            frame.data_lines.push(v.to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("id:") {
            let v = rest.strip_prefix(' ').unwrap_or(rest);
            if !v.is_empty() {
                frame.id = Some(v.to_string());
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("retry:") {
            let v = rest.trim();
            if let Ok(ms) = v.parse::<u64>() {
                frame.retry = Some(Duration::from_millis(ms));
            }
        }
    }
    frame
}

/// Incremental byte accumulator. Push chunks, pull complete frames.
#[derive(Debug, Default)]
pub struct SseBuffer {
    pending: String,
}

impl SseBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a UTF-8 chunk. Invalid UTF-8 is lossy-replaced so a single
    /// bad byte cannot stall the stream.
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        self.pending.push_str(&String::from_utf8_lossy(bytes));
    }

    pub fn push_str(&mut self, s: &str) {
        self.pending.push_str(s);
    }

    /// Pop the next complete frame, if a `\n\n` or `\r\n\r\n` separator is
    /// already in the buffer. Returns `None` when more bytes are needed.
    pub fn next_frame(&mut self) -> Option<SseFrame> {
        let end = find_frame_sep(&self.pending)?;
        let frame_text: String = self.pending.drain(..end).collect();
        let skip = if self.pending.starts_with("\r\n\r\n") {
            4
        } else {
            2
        };
        let _ = self.pending.drain(..skip.min(self.pending.len()));
        Some(parse_sse_frame(&frame_text))
    }

    /// Drain every currently complete frame.
    pub fn drain_frames(&mut self) -> Vec<SseFrame> {
        let mut out = Vec::new();
        while let Some(f) = self.next_frame() {
            out.push(f);
        }
        out
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

fn find_frame_sep(s: &str) -> Option<usize> {
    let lf = s.find("\n\n");
    let crlf = s.find("\r\n\r\n");
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// Resolve a (possibly relative) endpoint against a base URL.
///
/// Donor used `reqwest::Url::join`. This version is std-only:
/// - already-absolute `http(s)://` endpoints are returned as-is
/// - paths starting with `/` replace the base path
/// - other relatives append after the last `/` of the base path
pub fn absolutize_endpoint(base: &str, endpoint: &str) -> String {
    let endpoint = endpoint.trim();
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        return endpoint.to_string();
    }
    let Some((scheme, rest)) = split_scheme(base) else {
        return endpoint.to_string();
    };
    let (authority, path_and_query) = split_authority(rest);
    let path = path_and_query.split('?').next().unwrap_or(path_and_query);
    if endpoint.starts_with('/') {
        return format!("{scheme}://{authority}{endpoint}");
    }
    let dir = match path.rfind('/') {
        Some(i) => &path[..=i],
        None => "/",
    };
    format!("{scheme}://{authority}{dir}{endpoint}")
}

fn split_scheme(url: &str) -> Option<(&str, &str)> {
    let rest = url.strip_prefix("https://").map(|r| ("https", r));
    rest.or_else(|| url.strip_prefix("http://").map(|r| ("http", r)))
}

fn split_authority(rest: &str) -> (&str, &str) {
    match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_frame_message_basic() {
        let raw = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":42}\n\n";
        let f = parse_sse_frame(raw);
        assert_eq!(f.event.as_deref(), Some("message"));
        assert!(f.is_message());
        assert_eq!(
            f.data_lines[0],
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":42}"
        );
    }

    #[test]
    fn parse_sse_frame_endpoint() {
        let raw = "event: endpoint\ndata: /messages?sessionId=abc123\n\n";
        let f = parse_sse_frame(raw);
        assert!(f.is_endpoint());
        assert_eq!(f.data_lines[0], "/messages?sessionId=abc123");
    }

    #[test]
    fn parse_sse_frame_comment_and_multiline_data() {
        let raw = ": keepalive\ndata: line1\ndata: line2\n\n";
        let f = parse_sse_frame(raw);
        assert!(f.event.is_none());
        assert_eq!(f.data_lines.len(), 2);
        assert_eq!(f.data(), "line1\nline2");
    }

    #[test]
    fn parse_sse_frame_data_leading_space_stripped() {
        let f = parse_sse_frame("data: {\"k\":1}\n\n");
        assert_eq!(f.data_lines[0], "{\"k\":1}");
    }

    #[test]
    fn parse_sse_frame_data_multiple_leading_spaces_kept() {
        let f = parse_sse_frame("data:   {\"k\":1}\n\n");
        assert_eq!(f.data_lines[0], "  {\"k\":1}");
    }

    #[test]
    fn parse_sse_frame_id_and_retry() {
        let f = parse_sse_frame("id: 42\nretry: 1500\ndata: x\n\n");
        assert_eq!(f.id.as_deref(), Some("42"));
        assert_eq!(f.retry, Some(Duration::from_millis(1500)));
    }

    #[test]
    fn sse_buffer_two_frames() {
        let mut buf = SseBuffer::new();
        buf.push_str("event: endpoint\ndata: /messages\n\nevent: message\ndata: {\"ok\":1}\n\n");
        let frames = buf.drain_frames();
        assert_eq!(frames.len(), 2);
        assert!(frames[0].is_endpoint());
        assert!(frames[1].is_message());
        assert_eq!(frames[1].data(), "{\"ok\":1}");
        assert_eq!(buf.pending_len(), 0);
    }

    #[test]
    fn sse_buffer_incomplete_waits() {
        let mut buf = SseBuffer::new();
        buf.push_str("event: message\ndata: hi");
        assert!(buf.next_frame().is_none());
        buf.push_str("\n\n");
        let f = buf.next_frame().unwrap();
        assert_eq!(f.data(), "hi");
    }

    #[test]
    fn sse_buffer_crlf_separator() {
        let mut buf = SseBuffer::new();
        buf.push_str("data: a\r\n\r\n");
        let f = buf.next_frame().unwrap();
        assert_eq!(f.data(), "a");
    }

    #[test]
    fn absolutize_endpoint_relative_path() {
        let base = "https://example.com/api/sse";
        assert_eq!(
            absolutize_endpoint(base, "/messages?x=1"),
            "https://example.com/messages?x=1"
        );
    }

    #[test]
    fn absolutize_endpoint_already_absolute() {
        let base = "https://example.com/api/sse";
        assert_eq!(
            absolutize_endpoint(base, "https://other.com/messages"),
            "https://other.com/messages"
        );
    }

    #[test]
    fn absolutize_endpoint_relative_not_rooted() {
        let base = "https://example.com/api/sse";
        assert_eq!(
            absolutize_endpoint(base, "messages"),
            "https://example.com/api/messages"
        );
    }

    #[test]
    fn find_frame_sep_prefers_earliest() {
        assert_eq!(find_frame_sep("a\nb\n\nrest"), Some(3));
        assert_eq!(find_frame_sep("a\r\nb\r\n\r\nrest"), Some(4));
        assert_eq!(find_frame_sep("no sep yet"), None);
    }
}
