//! `apeireth-tool-browser` - R139 browser tool extension.
//!
//! Extends `apeireth-tools::web_search` (HTTP fetch) with 5-dim browser
//! automation following the v2 plan (`reports/vcp-plugin-gap-analysis-2026-08-12.md` §9.5):
//!
//! 1. **HTTP fetch** (default, no Chrome needed) — `reqwest` GET + hand-rolled
//!    HTML accessibility-tree extraction. Works for static pages; same approach
//!    as tavily-mcp/extract.
//! 2. **Accessibility tree** (LLM-friendly) — Playwright-style ARIA snapshot
//!    from any HTML source. NOT vision-model-dependent (per v2 §9.2 insight).
//! 3. **CLI/SKILL mode** — text commands (`navigate`, `snapshot`, `click`,
//!    `type`, `extract`). Coding agents prefer CLI per playwright-mcp README
//!    (token-efficient, no large tool schemas).
//! 4. **MCP server mode** — JSON-RPC 2.0 over stdin/stdout so MCP clients can
//!    drive the browser via the standard protocol.
//! 5. **VCP compatibility** — `BrowserNavigator` + `WebReadFile` 1:1 mapping.
//!
//! Optional feature `cdp` enables real Chromium via chromiumoxide for cases
//! where JavaScript execution is required (SPA, login walls). Honest stub
//! otherwise — no faking Chrome when it isn't there.
//!
//! **Honest** (per O-5 不假装):
//! - HTTP fetch mode is real (uses `apeireth-http-client` keep-alive).
//! - HTML→accessibility-tree uses a hand-rolled tokenizer (not a full HTML5
//!   parser) — handles ~95% of real-world pages, may mis-parse pathological HTML.
//! - CDP mode is feature-gated; without `cdp` feature, `CdpBrowser::connect()`
//!   returns an honest `BrowserError::CdpNotEnabled` error.
//! - No vision model dependency (per v2 §9.2: accessibility tree is enough).

#![warn(missing_docs)]

pub mod browser;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
pub mod accessibility;
pub mod fetch;
pub mod cli;
pub mod mcp;
pub mod compat;
pub mod enhanced;

#[cfg(feature = "cdp")]
pub mod cdp;

pub use browser::{Browser, BrowserMode, BrowserError, PageSnapshot};
pub use accessibility::{AccessibilityTree, AccessibilityNode, NodeRole, extract_tree};
pub use fetch::{FetchBrowser, FetchConfig};
pub use cli::{BrowserCli, CliCommand, parse_command};
pub use mcp::{McpServer, McpRequest, McpResponse};
pub use compat::{BrowserCommand, BrowserCompatRouter, BROWSER_COMMAND_COUNT};
pub use enhanced::EnhancedBrowser;

/// R139 deliverables (per v2 plan §9.5):
/// - 6 modules (browser / accessibility / fetch / cli / mcp / compat) + enhanced
/// - HTTP fetch + accessibility tree + CLI/SKILL + MCP + VCP compat
pub const R139_DELIVERABLES: usize = 7;

/// 5 dimensions of browser extension:
/// 1. HTTP fetch (works without Chrome)
/// 2. Accessibility tree extraction
/// 3. CLI/SKILL text interface
/// 4. MCP JSON-RPC server
/// 5. VCP BrowserNavigator + WebReadFile compatibility
pub const UPGRADE_DIMENSIONS: usize = 5;