use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};
use async_trait::async_trait;
use serde::Deserialize;
use tokio::process::Command;

#[derive(Debug, Deserialize)]
pub struct RepoParams {
    pub command: String, // "status", "log", "diff", "branch", "summary"
    pub args: Option<Vec<String>>,
}

pub struct RepoTools;

impl Default for RepoTools {
    fn default() -> Self {
        Self::new()
    }
}

impl RepoTools {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for RepoTools {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repo".into(),
            description: "Inspects Git repository status, commits, diffs, and branch states safely. Parameters: {\"command\": \"status|log|diff|branch|summary\", \"args\": []}".into(),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let params: RepoParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ValidationFailed(format!("Invalid repo tool parameters: {}", e)))?;

        let mut cmd = Command::new("git");

        match params.command.to_lowercase().as_str() {
            "status" => {
                cmd.arg("status").arg("-s");
            }
            "log" => {
                cmd.arg("log").arg("-n").arg("10").arg("--oneline");
            }
            "diff" => {
                cmd.arg("diff").arg("--stat");
            }
            "branch" => {
                cmd.arg("branch").arg("-a");
            }
            "summary" => {
                cmd.arg("log").arg("-n").arg("1").arg("--stat");
            }
            other => {
                return Err(ToolError::ValidationFailed(format!(
                    "Unsupported repo command '{}'. Allowed: status, log, diff, branch, summary",
                    other
                )));
            }
        }

        if let Some(extra_args) = params.args {
            for arg in extra_args {
                if !arg.starts_with(';') && !arg.starts_with('&') && !arg.starts_with('|') {
                    cmd.arg(arg);
                }
            }
        }

        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute git: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            let res = if stdout.trim().is_empty() {
                "Git command completed with empty output (clean tree).".to_string()
            } else {
                stdout
            };
            Ok(ToolResult::success(res))
        } else {
            Ok(ToolResult::failure(format!("Git error: {}", stderr)))
        }
    }
}
