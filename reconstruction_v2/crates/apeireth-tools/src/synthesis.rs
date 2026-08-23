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
        
        let ext = match self.interpreter.as_str() {
            "powershell" => "ps1",
            "python" => "py",
            "cmd" => "bat",
            _ => "txt",
        };
        
        let uuid = uuid::Uuid::new_v4();
        let tmp_path = std::env::temp_dir().join(format!("{}.{}", uuid, ext));
        
        std::fs::write(&tmp_path, &self.script_content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write temp script: {}", e)))?;

        struct TempFileGuard(std::path::PathBuf);
        impl Drop for TempFileGuard {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _guard = TempFileGuard(tmp_path.clone());

        let mut cmd = match self.interpreter.as_str() {
            "powershell" => {
                let mut c = tokio::process::Command::new("powershell");
                c.arg("-ExecutionPolicy").arg("Bypass").arg("-File").arg(&tmp_path);
                c
            }
            "python" => {
                let mut c = tokio::process::Command::new("python");
                c.arg(&tmp_path);
                c
            }
            "cmd" => {
                let mut c = tokio::process::Command::new("cmd");
                c.arg("/C").arg(&tmp_path);
                c
            }
            other => return Err(ToolError::ExecutionFailed(format!("Unsupported interpreter: {}", other))),
        };

        cmd.env("APEIRETH_PARAMS", &params_str);

        let timeout_duration = std::time::Duration::from_secs(30);
        let output_result = tokio::time::timeout(timeout_duration, cmd.output()).await;

        let output = match output_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(ToolError::ExecutionFailed(format!("Subprocess error: {}", e))),
            Err(_) => return Err(ToolError::ExecutionFailed("Execution timed out".to_string())),
        };

        let mut stdout_text = String::from_utf8_lossy(&output.stdout).into_owned();
        let mut stderr_text = String::from_utf8_lossy(&output.stderr).into_owned();

        if stdout_text.len() > 65536 {
            stdout_text.truncate(65536);
            stdout_text.push_str("\n[Output truncated at 64KB]");
        }
        if stderr_text.len() > 65536 {
            stderr_text.truncate(65536);
            stderr_text.push_str("\n[Output truncated at 64KB]");
        }

        if output.status.success() {
            Ok(ToolResult::success(stdout_text))
        } else {
            Ok(ToolResult::failure(stderr_text))
        }
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
            "powershell",
            "Write-Output 'hello_from_synth'",
            &mut registry,
        ).unwrap();

        assert_eq!(tool_name, "custom_calculator");
        let res = registry.execute("custom_calculator", serde_json::json!({"x": 10, "y": 32})).await.unwrap();
        assert!(res.success);
        assert!(res.output.contains("hello_from_synth"));
    }
}
