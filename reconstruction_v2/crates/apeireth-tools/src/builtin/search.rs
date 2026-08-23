use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};
use async_trait::async_trait;

pub struct SearchTool;

#[async_trait]
impl Tool for SearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search".into(),
            description: "Local keyword search".into(),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult { success: true, output: "search ok".into() })
    }
}
