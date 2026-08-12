//! VCP browser plugin compatibility layer.
//!
//! VCP `BrowserNavigator` + `WebReadFile` 1:1 mapping. Per v2 plan §9.6,
//! we keep VCP compat as a separate adapter (not the primary interface) —
//! Apeireth users should prefer `Browser` trait directly.
//!
//! VCP `BrowserNavigator` (40KB) features we keep:
//! - navigate(url)
//! - snapshot() / accessibility tree
//! - click(ref) / type(ref, text)
//! - extract main content
//!
//! VCP `WebReadFile` (3KB) features we keep:
//! - WebReadFile(url) — HTTP GET + return body

use crate::browser::BrowserMode;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BrowserCommand {
    BrowserNavigator,
    WebReadFile,
    Unknown,
}

pub const BROWSER_COMMAND_COUNT: usize = 2;

impl BrowserCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "BrowserNavigator" => Self::BrowserNavigator,
            "WebReadFile" => Self::WebReadFile,
            _ => Self::Unknown,
        }
    }

    pub fn mode(&self) -> BrowserMode {
        match self {
            // BrowserNavigator is CDP-like; WebReadFile is HTTP fetch
            BrowserCommand::BrowserNavigator => BrowserMode::Auto,
            BrowserCommand::WebReadFile => BrowserMode::Fetch,
            BrowserCommand::Unknown => BrowserMode::Fetch,
        }
    }
}

pub struct BrowserCompatRouter;

impl BrowserCompatRouter {
    pub fn new() -> Self {
        Self
    }
    pub fn command_count() -> usize {
        BROWSER_COMMAND_COUNT
    }
}

impl Default for BrowserCompatRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_2_commands() {
        for s in ["BrowserNavigator", "WebReadFile"] {
            assert_ne!(BrowserCommand::from_str(s), BrowserCommand::Unknown);
        }
        assert_eq!(BROWSER_COMMAND_COUNT, 2);
    }

    #[test]
    fn unknown_maps_correctly() {
        assert_eq!(BrowserCommand::from_str("Nonexistent"), BrowserCommand::Unknown);
    }

    #[test]
    fn mode_mapping() {
        assert_eq!(BrowserCommand::BrowserNavigator.mode(), BrowserMode::Auto);
        assert_eq!(BrowserCommand::WebReadFile.mode(), BrowserMode::Fetch);
    }

    #[test]
    fn router_count() {
        assert_eq!(BrowserCompatRouter::command_count(), 2);
    }
}