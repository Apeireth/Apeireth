//! HTML to plain-text extraction (hand-rolled tokenizer, pure std).
//!
//! Ported from legacy `apeireth-tool-fetch::html_extract` (R149 baseline).
//! Scope is honestly narrow: a hand-rolled tokenizer that handles the common
//! case — script/style content is skipped, block-level tags become newlines,
//! named and numeric HTML entities are decoded, and text is whitespace-
//! collapsed. It is not a full HTML5 parser and will mis-parse pathological
//! markup.
//!
//! The value: an LLM consuming a fetched page gets readable text instead of
//! markup noise, without pulling an HTML parser dependency.

/// Errors produced by text extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlExtractError {
    Empty,
    NoText,
}

impl std::fmt::Display for HtmlExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "html extract: empty input"),
            Self::NoText => write!(f, "html extract: no text found"),
        }
    }
}

/// Extract readable plain text from an HTML document.
pub fn extract_text(html: &str) -> Result<String, HtmlExtractError> {
    if html.trim().is_empty() {
        return Err(HtmlExtractError::Empty);
    }
    let mut out = String::with_capacity(html.len() / 2);
    // 0 = normal, 1 = inside script, 2 = inside style, 3 = inside pre.
    let mut in_skip: u8 = 0;
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == '>' {
                    chars.next();
                    break;
                }
                tag.push(nc);
                chars.next();
            }
            let tag_lc = tag.trim().to_lowercase();
            if tag_lc.starts_with("script") {
                in_skip = 1;
                continue;
            }
            if tag_lc.starts_with("style") {
                in_skip = 2;
                continue;
            }
            if tag_lc.starts_with("pre") {
                in_skip = 3;
                continue;
            }
            if tag_lc.starts_with("/script")
                || tag_lc.starts_with("/style")
                || tag_lc.starts_with("/pre")
            {
                in_skip = 0;
                continue;
            }
            if matches!(
                tag_lc.as_str(),
                "br" | "br/"
                    | "/p"
                    | "/div"
                    | "/li"
                    | "/h1"
                    | "/h2"
                    | "/h3"
                    | "/h4"
                    | "/h5"
                    | "/h6"
                    | "/tr"
            ) {
                out.push('\n');
            }
            continue;
        }
        if in_skip > 0 {
            continue;
        }
        match c {
            '&' => {
                let mut ent = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc == ';' {
                        chars.next();
                        break;
                    }
                    if nc == '&' || nc == '<' {
                        break;
                    }
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
                        if let Some(digits) = other.strip_prefix('#') {
                            digits
                                .parse::<u32>()
                                .ok()
                                .and_then(char::from_u32)
                                .unwrap_or('?')
                        } else {
                            '?'
                        }
                    }
                };
                out.push(decoded);
            }
            _ => out.push(c),
        }
    }
    let trimmed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.is_empty() {
        Err(HtmlExtractError::NoText)
    } else {
        Ok(trimmed)
    }
}

/// Extract `(url, link_text)` pairs from `href="..."` anchors.
pub fn extract_links(html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0usize;
    while i + 6 < bytes.len() {
        if &bytes[i..i + 6] == b"href=\"" {
            let start = i + 6;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            let url = String::from_utf8_lossy(&bytes[start..j]).to_string();
            let mut text_start = j + 1;
            let mut text_end = text_start;
            while text_end + 4 < bytes.len() {
                if &bytes[text_end..text_end + 4] == b"</a>" {
                    break;
                }
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

/// Extract the `<title>` element content, when present.
pub fn extract_title(html: &str) -> Option<String> {
    let bytes = html.as_bytes();
    let open = b"<title>";
    let close = b"</title>";
    let mut i = 0;
    while i + open.len() < bytes.len() {
        if &bytes[i..i + open.len()] == open {
            let start = i + open.len();
            let mut j = start;
            while j + close.len() < bytes.len() && &bytes[j..j + close.len()] != close {
                j += 1;
            }
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
        assert!(t.contains("A & B"), "{t}");
        assert!(t.contains("< C"), "{t}");
    }

    #[test]
    fn extract_numeric_entities() {
        // Donor decoder handles decimal numeric entities (`&#65;` → A). Hex
        // (`&#x42;`) is not in the ported algorithm.
        let h = "<p>&#65;&#66;</p>";
        let t = extract_text(h).unwrap();
        assert!(t.contains("AB"), "{t}");
    }

    #[test]
    fn extract_block_tags_keep_text() {
        let h = "<p>One</p><p>Two</p>";
        let t = extract_text(h).unwrap();
        assert!(t.contains("One"), "{t}");
        assert!(t.contains("Two"), "{t}");
    }

    #[test]
    fn extract_empty_errors() {
        assert_eq!(extract_text(""), Err(HtmlExtractError::Empty));
    }

    #[test]
    fn extract_no_text_errors() {
        assert_eq!(extract_text("<div></div>"), Err(HtmlExtractError::NoText));
    }

    #[test]
    fn extract_links_basic() {
        let h = "<a href=\"https://a.com\">A</a> <a href=\"https://b.com\">B</a>";
        let links = extract_links(h);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].0, "https://a.com");
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

    #[test]
    fn unicode_text_survives() {
        let h = "<p>你好，世界</p>";
        let t = extract_text(h).unwrap();
        assert_eq!(t, "你好，世界");
    }
}
