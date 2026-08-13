//! HTML to text extraction (hand-rolled tokenizer).

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HtmlExtractError {
    #[error("empty input")]
    Empty,
    #[error("no text found")]
    NoText,
}

pub fn extract_text(html: &str) -> Result<String, HtmlExtractError> {
    if html.trim().is_empty() { return Err(HtmlExtractError::Empty); }
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_skip: u8 = 0; // 0=normal, 1=script, 2=style, 3=pre
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == '>' { chars.next(); break; }
                tag.push(nc);
                chars.next();
            }
            let tag_lc = tag.trim().to_lowercase();
            if tag_lc.starts_with("script") { in_skip = 1; continue; }
            if tag_lc.starts_with("style") { in_skip = 2; continue; }
            if tag_lc.starts_with("pre") { in_skip = 3; continue; }
            if tag_lc.starts_with("/script") { in_skip = 0; continue; }
            if tag_lc.starts_with("/style") { in_skip = 0; continue; }
            if tag_lc.starts_with("/pre") { in_skip = 0; continue; }
            if matches!(tag_lc.as_str(),
                "br" | "br/" | "/p" | "/div" | "/li" | "/h1" | "/h2" | "/h3" | "/h4" | "/h5" | "/h6" | "/tr"
            ) {
                out.push('\n');
            }
            continue;
        }
        if in_skip > 0 { continue; }
        match c {
            '&' => {
                let mut ent = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc == ';' { chars.next(); break; }
                    if nc == '&' || nc == '<' { break; }
                    ent.push(nc);
                    chars.next();
                }
                let decoded = match ent.as_str() {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    "nbsp" => '\u{00A0}',
                    other => {
                        if other.starts_with('#') && other.len() > 1 {
                            if let Some(d) = other[1..].parse::<u32>().ok() {
                                char::from_u32(d).unwrap_or('?')
                            } else { '?' }
                        } else { '?' }
                    }
                };
                out.push(decoded);
            }
            _ => out.push(c),
        }
    }
    let trimmed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() { Err(HtmlExtractError::NoText) } else { Ok(trimmed) }
}

pub fn extract_links(html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0usize;
    while i + 6 < bytes.len() {
        if &bytes[i..i+6] == b"href=\"" {
            let start = i + 6;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'"' { j += 1; }
            let url = String::from_utf8_lossy(&bytes[start..j]).to_string();
            let mut text_start = j + 1;
            let mut text_end = text_start;
            while text_end + 4 < bytes.len() {
                if &bytes[text_end..text_end+4] == b"</a>" { break; }
                text_end += 1;
            }
            let text = String::from_utf8_lossy(&bytes[text_start..text_end]).to_string();
            out.push((url, text.split_whitespace().collect::<Vec<_>>().join(" ")));
            i = text_end;
        } else {
            i += 1;
        }
    }
    out
}

pub fn extract_title(html: &str) -> Option<String> {
    let bytes = html.as_bytes();
    let open = b"<title>";
    let close = b"</title>";
    let mut i = 0;
    while i + open.len() < bytes.len() {
        if &bytes[i..i+open.len()] == open {
            let start = i + open.len();
            let mut j = start;
            while j + close.len() < bytes.len() && &bytes[j..j+close.len()] != close { j += 1; }
            return Some(String::from_utf8_lossy(&bytes[start..j]).to_string());
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_simple_text() {
        let h = "<html><body><p>Hello <b>World</b>!</p></body></html>";
        let t = extract_text(h).unwrap();
        assert_eq!(t, "Hello World!");
    }

    #[test]
    fn extract_skips_script() {
        let h = "<html><head><script>alert('x')</script></head><body>OK</body></html>";
        let t = extract_text(h).unwrap();
        assert_eq!(t, "OK");
    }

    #[test]
    fn extract_entities() {
        let h = "<p>A &amp; B &lt; C</p>";
        let t = extract_text(h).unwrap();
        assert!(t.contains("A & B"));
        assert!(t.contains("< C"));
    }

    #[test]
    fn extract_block_tags_add_newline() {
        let h = "<p>One</p><p>Two</p>";
        let t = extract_text(h).unwrap();
        assert!(t.contains("One"));
        assert!(t.contains("Two"));
    }

    #[test]
    fn extract_empty_errors() {
        assert!(extract_text("").is_err());
    }

    #[test]
    fn extract_links_basic() {
        let h = "<a href=\"https://a.com\">A</a> <a href=\"https://b.com\">B</a>";
        let links = extract_links(h);
        assert_eq!(links.len(), 2);
    }

    #[test]
    fn extract_title_basic() {
        let h = "<html><head><title>My Page</title></head></html>";
        assert_eq!(extract_title(h), Some("My Page".to_string()));
    }

    #[test]
    fn extract_title_missing() {
        assert_eq!(extract_title("<html></html>"), None);
    }
}
