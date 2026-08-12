//! VCP code-search plugin compatibility layer.
//!
//! VCP `CodeSearch` + `RepoInspector` + `CodeAnalyzer` (combined ~50KB across
//! multiple plugins) 1:1 mapping. We provide a router that classifies
//! commands and dispatches to MCP tool equivalents.

use crate::mcp::McpTool;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CodeSearchCommand {
    /// VCP `CodeSearch` (regex search across files)
    CodeSearch,
    /// VCP `RepoInspector` (project structure)
    RepoInspector,
    /// VCP `CodeAnalyzer` (symbol extraction + metrics)
    CodeAnalyzer,
    Unknown,
}

pub const CODESEARCH_COMMAND_COUNT: usize = 3;

impl CodeSearchCommand {
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
            CodeSearchCommand::CodeSearch => Some(McpTool::SearchText),
            CodeSearchCommand::RepoInspector => Some(McpTool::ProjectOverview),
            CodeSearchCommand::CodeAnalyzer => Some(McpTool::ExtractSymbols),
            CodeSearchCommand::Unknown => None,
        }
    }
}

pub struct CodeSearchCompatRouter;

impl CodeSearchCompatRouter {
    pub fn new() -> Self {
        Self
    }
    pub fn command_count() -> usize {
        CODESEARCH_COMMAND_COUNT
    }
}

impl Default for CodeSearchCompatRouter {
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
            assert_ne!(CodeSearchCommand::from_str(s), CodeSearchCommand::Unknown);
        }
        assert_eq!(CODESEARCH_COMMAND_COUNT, 3);
    }

    #[test]
    fn unknown_maps_correctly() {
        assert_eq!(CodeSearchCommand::from_str("xyz"), CodeSearchCommand::Unknown);
    }

    #[test]
    fn all_have_mcp_mapping() {
        assert!(CodeSearchCommand::CodeSearch.to_mcp_tool().is_some());
        assert!(CodeSearchCommand::RepoInspector.to_mcp_tool().is_some());
        assert!(CodeSearchCommand::CodeAnalyzer.to_mcp_tool().is_some());
        assert!(CodeSearchCommand::Unknown.to_mcp_tool().is_none());
    }

    #[test]
    fn router_count() {
        assert_eq!(CodeSearchCompatRouter::command_count(), 3);
    }
}