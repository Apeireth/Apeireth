//! Skill → MCP ToolServer adapter.
pub fn skill_to_mcp_tool(_skill_id: &str) -> String { "mcp_tool".to_string() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn converts() { assert_eq!(skill_to_mcp_tool("x"), "mcp_tool"); }
}
