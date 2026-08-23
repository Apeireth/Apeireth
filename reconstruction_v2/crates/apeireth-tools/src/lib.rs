pub mod builtin;
pub mod sandbox;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
}

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    #[error("Sandbox error: {0}")]
    SandboxError(String),
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.definition().name, tool);
    }

    pub async fn execute(&self, name: &str, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let tool = self.tools.get(name).ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        tool.execute(params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct DummyTool;
    #[async_trait]
    impl Tool for DummyTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "dummy".to_string(),
                description: "Dummy".to_string(),
                risk_level: RiskLevel::Low,
            }
        }
        async fn execute(&self, _params: serde_json::Value) -> Result<ToolResult, ToolError> {
            Ok(ToolResult { success: true, output: "ok".to_string() })
        }
    }

    #[tokio::test]
    async fn test_registry() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool));
        let res = reg.execute("dummy", serde_json::json!({})).await.unwrap();
        assert!(res.success);
    }
}
