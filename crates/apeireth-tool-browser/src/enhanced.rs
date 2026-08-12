//! EnhancedBrowser — composed entry point for browser tool.
//!
//! Composes FetchBrowser (or CdpBrowser when `cdp` feature enabled) with
//! accessibility tree + CLI command dispatcher + MCP server facade. This
//! is what `apeireth-tools` registers as a single Tool entry.

use std::sync::Arc;
use thiserror::Error;

use crate::browser::{Browser, BrowserError, BrowserMode, PageSnapshot};
use crate::cli::{BrowserCli, CliCommand, SnapshotKind};
use crate::fetch::FetchBrowser;
use crate::mcp::{McpRequest, McpResponse, McpServer};

#[derive(Debug, Error)]
pub enum EnhancedBrowserError {
    #[error("browser: `{0}`")]
    Browser(#[from] BrowserError),
    #[error("invalid request: `{0}`")]
    Invalid(String),
}

/// Composed browser tool: holds inner Browser + dispatches CLI/MCP requests.
pub struct EnhancedBrowser {
    inner: Arc<dyn Browser>,
}

impl EnhancedBrowser {
    /// Create from a `FetchBrowser` (default).
    pub fn from_fetch() -> Result<Self, EnhancedBrowserError> {
        Ok(Self {
            inner: Arc::new(FetchBrowser::new()?),
        })
    }

    /// Create from any `Browser` implementation.
    pub fn from_browser(browser: Arc<dyn Browser>) -> Self {
        Self { inner: browser }
    }

    pub fn mode(&self) -> BrowserMode {
        self.inner.mode()
    }

    /// Dispatch a CLI command. Returns either a snapshot (for navigate/snapshot)
    /// or a text result (for extract), or an error.
    pub async fn dispatch_cli(&self, cmd: CliCommand) -> Result<DispatchResult, EnhancedBrowserError> {
        match cmd {
            CliCommand::Navigate(url) => {
                let snap = self.inner.navigate(&url).await?;
                Ok(DispatchResult::Snapshot(snap))
            }
            CliCommand::Snapshot(kind) => {
                let snap = self.inner.snapshot().await?;
                match kind {
                    SnapshotKind::Full => Ok(DispatchResult::Snapshot(snap)),
                    SnapshotKind::Text => Ok(DispatchResult::Text(snap.accessibility.to_snapshot())),
                    SnapshotKind::Refs => {
                        let refs = snap.accessibility.interactive_refs();
                        let formatted = refs
                            .iter()
                            .map(|(r, role, name)| format!("{} {} \"{}\"", r, role.as_str(), name))
                            .collect::<Vec<_>>()
                            .join("\n");
                        Ok(DispatchResult::Text(formatted))
                    }
                }
            }
            CliCommand::Extract => {
                let text = self.inner.extract_text().await?;
                Ok(DispatchResult::Text(text))
            }
            CliCommand::Click(_ref_id) => Err(EnhancedBrowserError::Invalid(
                "click requires CDP mode (build with --features cdp)".to_string(),
            )),
            CliCommand::Type { .. } => Err(EnhancedBrowserError::Invalid(
                "type requires CDP mode (build with --features cdp)".to_string(),
            )),
            CliCommand::Help => Ok(DispatchResult::Text(BrowserCli::help().to_string())),
            CliCommand::Unknown(msg) => Err(EnhancedBrowserError::Invalid(msg)),
        }
    }

    /// Dispatch an MCP request.
    pub fn dispatch_mcp(&self, req: McpRequest) -> McpResponse {
        // We don't have an async path for MCP — current MCP handlers are
        // metadata-only (initialize / tools/list). Real tools/call routes
        // through dispatch_cli via async runtime on the server side.
        McpServer::handle(req)
    }
}

/// Result of dispatching a CLI command.
#[derive(Debug, Clone)]
pub enum DispatchResult {
    /// Full page snapshot (navigate / snapshot full)
    Snapshot(PageSnapshot),
    /// Text content (extract / snapshot text / snapshot refs / help)
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parse_command;

    #[tokio::test]
    async fn from_fetch_uses_fetch_mode() {
        let b = EnhancedBrowser::from_fetch().unwrap();
        assert_eq!(b.mode(), BrowserMode::Fetch);
    }

    #[tokio::test]
    async fn dispatch_help_returns_text() {
        let b = EnhancedBrowser::from_fetch().unwrap();
        let cmd = parse_command(&["help".to_string()]);
        let r = b.dispatch_cli(cmd).await.unwrap();
        match r {
            DispatchResult::Text(s) => assert!(s.contains("USAGE")),
            _ => panic!("expected Text"),
        }
    }

    #[tokio::test]
    async fn dispatch_click_in_fetch_mode_errors() {
        let b = EnhancedBrowser::from_fetch().unwrap();
        let cmd = parse_command(&["click".to_string(), "e3".to_string()]);
        let r = b.dispatch_cli(cmd).await;
        assert!(matches!(r, Err(EnhancedBrowserError::Invalid(_))));
    }

    #[test]
    fn dispatch_mcp_initialize() {
        let b = EnhancedBrowser::from_fetch().unwrap();
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: serde_json::json!({}),
        };
        let resp = b.dispatch_mcp(req);
        assert!(resp.result.is_some());
    }
}