//! VCP code-search plugin compatibility layer.
//!
//! VCP `CodeSearch` + `RepoInspector` + `CodeAnalyzer` (combined ~50KB across
//! multiple plugins) 1:1 mapping. We provide a router that classifies
//! commands and dispatches to MCP tool equivalents.

use crate::mcp::McpTool;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VcpCodeSearchCommand {
    /// VCP `CodeSearch` (regex search across files)
    CodeSearch,
    /// VCP `RepoInspector` (project structure)
    RepoInspector,
    /// VCP `CodeAnalyzer` (symbol extraction + metrics)
    CodeAnalyzer,
    Unknown,
}

pub const VCP_CODESEARCH_COMMAND_COUNT: usize = 3;

impl VcpCodeSearchCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "CodeSearch" => Self::CodeSearch,
            "RepoInspector" => Self::RepoInspector,
            "CodeAnalyzer" => Self::CodeAnalyzer,
            _ => Self::Unknown,
        }
    }

    /// Map VCP command to our MCP tool (for compatibility shim).
    pub fn to_mcp_tool(&self) -> Option<McpTool> {
        match self {
            VcpCodeSearchCommand::CodeSearch => Some(McpTool::SearchText),
            VcpCodeSearchCommand::RepoInspector => Some(McpTool::ProjectOverview),
            VcpCodeSearchCommand::CodeAnalyzer => Some(McpTool::ExtractSymbols),
            VcpCodeSearchCommand::Unknown => None,
        }
    }
}

pub struct VcpCodeSearchRouter;

impl VcpCodeSearchRouter {
    pub fn new() -> Self {
        Self
    }
    pub fn command_count() -> usize {
        VCP_CODESEARCH_COMMAND_COUNT
    }
}

impl Default for VcpCodeSearchRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_3_commands() {
        for s in ["CodeSearch", "RepoInspector", "CodeAnalyzer"] {
            assert_ne!(VcpCodeSearchCommand::from_str(s), VcpCodeSearchCommand::Unknown);
        }
        assert_eq!(VCP_CODESEARCH_COMMAND_COUNT, 3);
    }

    #[test]
    fn unknown_maps_correctly() {
        assert_eq!(VcpCodeSearchCommand::from_str("xyz"), VcpCodeSearchCommand::Unknown);
    }

    #[test]
    fn all_have_mcp_mapping() {
        assert!(VcpCodeSearchCommand::CodeSearch.to_mcp_tool().is_some());
        assert!(VcpCodeSearchCommand::RepoInspector.to_mcp_tool().is_some());
        assert!(VcpCodeSearchCommand::CodeAnalyzer.to_mcp_tool().is_some());
        assert!(VcpCodeSearchCommand::Unknown.to_mcp_tool().is_none());
    }

    #[test]
    fn router_count() {
        assert_eq!(VcpCodeSearchRouter::command_count(), 3);
    }
}