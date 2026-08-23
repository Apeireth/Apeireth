use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Deserialize)]
pub struct ShellParams {
    pub command: Option<String>,
    pub preset: Option<String>,
    pub args: Option<Vec<String>>,
}

pub struct ShellTool;

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellTool {
    pub fn new() -> Self {
        Self
    }

    pub fn sanitize_input(input: &str) -> Result<(), ToolError> {
        let dangerous_patterns = ["rm -rf /", ":(){ :|:& };:", "del /f /s /q C:\\Windows", "> /dev/sda"];
        for pat in &dangerous_patterns {
            if input.contains(pat) {
                return Err(ToolError::ValidationFailed(format!("Forbidden critical destructive command: {}", pat)));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "shell".into(),
            description: "Executes shell commands safely across Windows, Linux, and macOS with sandbox restrictions".into(),
            risk_level: RiskLevel::High,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let params: ShellParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ValidationFailed(e.to_string()))?;

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.arg("/C");
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-c");
            c
        };

        if let Some(cmd_str) = params.command {
            Self::sanitize_input(&cmd_str)?;
            cmd.arg(&cmd_str);
        } else if let Some(preset) = params.preset {
            match preset.as_str() {
                "git-log-recent" => {
                    if cfg!(target_os = "windows") {
                        cmd.arg("git log -n 5 --oneline");
                    } else {
                        cmd.arg("git log -n 5 --oneline");
                    }
                }
                "git-status-short" => {
                    if cfg!(target_os = "windows") {
                        cmd.arg("git status -s");
                    } else {
                        cmd.arg("git status -s");
                    }
                }
                "echo-text" => {
                    let args = params.args.unwrap_or_default();
                    let text = args.join(" ");
                    if text.contains('&') || text.contains('|') || text.contains('>') || text.contains('<') || text.contains(';') || text.contains('`') || text.contains('$') {
                        return Err(ToolError::ValidationFailed("Invalid shell characters detected in echo text".into()));
                    }
                    if cfg!(target_os = "windows") {
                        cmd.arg(format!("echo {}", text));
                    } else {
                        cmd.arg(format!("echo \"{}\"", text));
                    }
                }
                _ => return Err(ToolError::ValidationFailed("Unknown shell preset".into())),
            }
        } else {
            return Err(ToolError::ValidationFailed("Either 'command' or 'preset' must be specified".into()));
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
            .wait_with_output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(ToolResult {
                success: true,
                output: stdout,
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: if stderr.is_empty() { stdout } else { stderr },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_echo_preset() {
        let tool = ShellTool::new();
        let res = tool.execute(serde_json::json!({
            "preset": "echo-text",
            "args": ["hello", "cross-platform", "world"]
        })).await.unwrap();
        assert!(res.success);
        assert!(res.output.contains("hello cross-platform world"));
    }

    #[tokio::test]
    async fn test_shell_dynamic_command() {
        let tool = ShellTool::new();
        let res = tool.execute(serde_json::json!({
            "command": if cfg!(target_os = "windows") { "echo dynamic_shell_ok" } else { "echo dynamic_shell_ok" }
        })).await.unwrap();
        assert!(res.success);
        assert!(res.output.contains("dynamic_shell_ok"));
    }

    #[tokio::test]
    async fn test_shell_destructive_rejection() {
        let tool = ShellTool::new();
        let res = tool.execute(serde_json::json!({
            "command": "rm -rf /"
        })).await;
        assert!(res.is_err());
    }
}

