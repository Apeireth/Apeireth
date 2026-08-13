//! CLI/SKILL text interface for browser.
//!
//! Per v2 plan §9.2 / playwright-mcp README: coding agents prefer CLI over
//! MCP because CLI avoids loading large tool schemas and verbose accessibility
//! trees into the model context.
//!
//! Commands:
//! - `apeireth browser navigate <url>`     → PageSnapshot
//! - `apeireth browser snapshot`           → PageSnapshot
//! - `apeireth browser snapshot text`      → accessibility tree text only
//! - `apeireth browser snapshot refs`      → interactive refs only
//! - `apeireth browser click <ref_id>`     → (CDP only — returns stub error in fetch mode)
//! - `apeireth browser type <ref_id> <text>` → (CDP only)
//! - `apeireth browser extract`            → text content for LLM context
//! - `apeireth browser help`               → usage

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Navigate(String),
    Snapshot(SnapshotKind),
    Click(String),
    Type { ref_id: String, text: String },
    Extract,
    Help,
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotKind {
    Full,
    Text,
    Refs,
}

/// Parse CLI args after the `browser` subcommand.
pub fn parse_command(args: &[String]) -> CliCommand {
    let mut iter = args.iter();
    let first = iter.next().map(|s| s.as_str()).unwrap_or("");
    match first {
        "navigate" | "goto" | "open" => {
            let url = iter.next().cloned().unwrap_or_default();
            if url.is_empty() {
                CliCommand::Unknown("navigate requires <url>".to_string())
            } else {
                CliCommand::Navigate(url)
            }
        }
        "snapshot" | "snap" => {
            let kind = match iter.next().map(|s| s.as_str()) {
                Some("text") => SnapshotKind::Text,
                Some("refs") => SnapshotKind::Refs,
                _ => SnapshotKind::Full,
            };
            CliCommand::Snapshot(kind)
        }
        "click" => match iter.next() {
            Some(r) => CliCommand::Click(r.clone()),
            None => CliCommand::Unknown("click requires <ref_id>".to_string()),
        },
        "type" => {
            let ref_id = iter.next().cloned().unwrap_or_default();
            let text: Vec<String> = iter.cloned().collect();
            let text = text.join(" ");
            if ref_id.is_empty() {
                CliCommand::Unknown("type requires <ref_id> <text>".to_string())
            } else {
                CliCommand::Type { ref_id, text }
            }
        }
        "extract" => CliCommand::Extract,
        "help" | "--help" | "-h" => CliCommand::Help,
        "" => CliCommand::Unknown("no command given".to_string()),
        other => CliCommand::Unknown(other.to_string()),
    }
}

impl FromStr for CliCommand {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args: Vec<String> = s.split_whitespace().map(String::from).collect();
        Ok(parse_command(&args))
    }
}

/// CLI wrapper that holds a Browser and dispatches commands.
pub struct BrowserCli;

impl BrowserCli {
    pub fn new() -> Self {
        Self
    }

    /// Help text shown by `apeireth browser help`.
    pub fn help() -> &'static str {
        "apeireth browser — Playwright-style browser automation\n\
         \n\
         USAGE:\n\
           apeireth browser <command> [args...]\n\
         \n\
         COMMANDS:\n\
           navigate <url>         Navigate to URL\n\
           snapshot [text|refs]   Get page snapshot (default: full)\n\
           click <ref_id>         Click element (CDP mode only)\n\
           type <ref_id> <text>   Type into element (CDP mode only)\n\
           extract                Extract text for LLM context\n\
           help                   Show this help\n\
         \n\
         MODES:\n\
           Default: HTTP fetch (no Chrome required)\n\
           With --features cdp: Chromium via CDP (full browser)\n"
    }
}

impl Default for BrowserCli {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_navigate() {
        let cmd = parse_command(&v(&["navigate", "https://example.com"]));
        assert_eq!(cmd, CliCommand::Navigate("https://example.com".to_string()));
    }

    #[test]
    fn parse_navigate_missing_url() {
        let cmd = parse_command(&v(&["navigate"]));
        assert!(matches!(cmd, CliCommand::Unknown(_)));
    }

    #[test]
    fn parse_snapshot_full() {
        let cmd = parse_command(&v(&["snapshot"]));
        assert_eq!(cmd, CliCommand::Snapshot(SnapshotKind::Full));
    }

    #[test]
    fn parse_snapshot_text() {
        let cmd = parse_command(&v(&["snapshot", "text"]));
        assert_eq!(cmd, CliCommand::Snapshot(SnapshotKind::Text));
    }

    #[test]
    fn parse_snapshot_refs() {
        let cmd = parse_command(&v(&["snap", "refs"]));
        assert_eq!(cmd, CliCommand::Snapshot(SnapshotKind::Refs));
    }

    #[test]
    fn parse_click() {
        let cmd = parse_command(&v(&["click", "e5"]));
        assert_eq!(cmd, CliCommand::Click("e5".to_string()));
    }

    #[test]
    fn parse_click_missing_ref() {
        let cmd = parse_command(&v(&["click"]));
        assert!(matches!(cmd, CliCommand::Unknown(_)));
    }

    #[test]
    fn parse_type_with_text() {
        let cmd = parse_command(&v(&["type", "e3", "hello", "world"]));
        match cmd {
            CliCommand::Type { ref_id, text } => {
                assert_eq!(ref_id, "e3");
                assert_eq!(text, "hello world");
            }
            _ => panic!("expected Type"),
        }
    }

    #[test]
    fn parse_help() {
        let cmd = parse_command(&v(&["help"]));
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn parse_unknown() {
        let cmd = parse_command(&v(&["wat"]));
        assert!(matches!(cmd, CliCommand::Unknown(_)));
    }

    #[test]
    fn parse_empty() {
        let cmd = parse_command(&v(&[]));
        assert!(matches!(cmd, CliCommand::Unknown(_)));
    }

    #[test]
    fn help_nonempty() {
        assert!(BrowserCli::help().contains("USAGE"));
    }
}