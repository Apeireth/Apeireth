use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};
use async_trait::async_trait;

pub struct BrowserTool;

#[async_trait]
impl Tool for BrowserTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser".into(),
            description: "Web page content reader".into(),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, _params: serde_json::Value) -> Result<ToolResult, ToolError> {
        Ok(ToolResult { success: true, output: "browser ok".into() })
    }
}
