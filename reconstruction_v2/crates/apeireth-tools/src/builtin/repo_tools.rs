use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};
use async_trait::async_trait;

pub struct RepoTools;

#[async_trait]
impl Tool for RepoTools {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repo".into(),
            description: "Codebase inspection".into(),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult { success: true, output: "repo ok".into() })
    }
}
