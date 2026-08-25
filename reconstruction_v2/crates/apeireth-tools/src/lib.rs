#![allow(unexpected_cfgs)]

#[cfg(feature = "cdp")]
pub mod v1_browser;
pub mod builtin;
pub mod codesearch;
pub mod mcp;
pub mod sandbox;
pub mod skills;
pub mod synthesis;
pub mod v1_filesystem;
pub mod v1_image_gen;
pub mod v1_image_process;
pub mod v1_repo_tools;
pub mod vision;
pub mod worktree;




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

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
        }
    }

    pub fn failure(output: impl Into<String>) -> Self {
        Self {
            success: false,
            output: output.into(),
        }
    }
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
    tools: std::sync::RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: std::sync::RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, tool: Arc<dyn Tool>) {
        let mut map = self.tools.write().unwrap();
        map.insert(tool.definition().name, tool);
    }

    pub fn unregister(&self, name: &str) -> bool {
        let mut map = self.tools.write().unwrap();
        map.remove(name).is_some()
    }

    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        let map = self.tools.read().unwrap();
        map.values().map(|t| t.definition()).collect()
    }

    pub fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let map = self.tools.read().unwrap();
        if let Some(t) = map.get(name) {
            return Some(t.clone());
        }
        let alias = match name {
            "fs" => "filesystem",
            "filesystem" => "fs",
            "sh" => "shell",
            "bash" => "shell",
            "cmd" => "shell",
            "powershell" => "shell",
            "gui" => "desktop_action",
            "vision" => "screen_observe",
            _ => name,
        };
        map.get(alias).cloned()
    }


    pub async fn execute(&self, name: &str, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let tool = {
            let map = self.tools.read().unwrap();
            map.get(name).cloned()
        };
        let tool = tool.ok_or_else(|| ToolError::NotFound(name.to_string()))?;
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
        let reg = ToolRegistry::new();
        reg.register(Arc::new(DummyTool));

        let res = reg.execute("dummy", serde_json::json!({})).await.unwrap();
        assert!(res.success);
    }
}
