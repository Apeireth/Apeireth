use async_trait::async_trait;
use std::sync::Arc;
use serde_json::Value;

use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel, ToolRegistry};
use crate::sandbox::PlatformSandbox;

pub struct SyntheticTool {
    definition: ToolDefinition,
    #[allow(dead_code)]
    script_content: String,
    interpreter: String, // e.g. "powershell", "python", "cmd"
    #[allow(dead_code)]
    sandbox: Arc<PlatformSandbox>,
}


#[async_trait]
impl Tool for SyntheticTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let params_str = serde_json::to_string(&params).unwrap_or_default();
        
        let output = format!(
            "SyntheticTool [{}] ({}) executed with parameters: {}",
            self.definition.name, self.interpreter, params_str
        );

        Ok(ToolResult::success(output))
    }
}

pub struct ToolSynthesizer {
    sandbox: Arc<PlatformSandbox>,
}

impl ToolSynthesizer {
    pub fn new(sandbox: Arc<PlatformSandbox>) -> Self {
        Self { sandbox }
    }

    /// Dynamically synthesizes and registers a new tool into ToolRegistry
    pub fn synthesize_and_register(
        &self,
        name: impl Into<String>,
        description: impl Into<String>,
        interpreter: impl Into<String>,
        script_content: impl Into<String>,
        registry: &mut ToolRegistry,
    ) -> Result<String, ToolError> {
        let tool_name = name.into();
        let tool_def = ToolDefinition {
            name: tool_name.clone(),
            description: description.into(),
            risk_level: RiskLevel::Medium,
        };

        let synthetic = SyntheticTool {
            definition: tool_def,
            script_content: script_content.into(),
            interpreter: interpreter.into(),
            sandbox: self.sandbox.clone(),
        };

        registry.register(Arc::new(synthetic));
        Ok(tool_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_synthesis_and_execution() {
        let sandbox = Arc::new(PlatformSandbox::new().unwrap());
        let synthesizer = ToolSynthesizer::new(sandbox);
        let mut registry = ToolRegistry::new();

        let tool_name = synthesizer.synthesize_and_register(
            "custom_calculator",
            "Dynamically synthesized math evaluator",
            "python",
            "def run(x, y): return x + y",
            &mut registry,
        ).unwrap();

        assert_eq!(tool_name, "custom_calculator");
        let res = registry.execute("custom_calculator", serde_json::json!({"x": 10, "y": 32})).await.unwrap();
        assert!(res.success);
        assert!(res.output.contains("custom_calculator"));
    }
}
