//! Read-only repository inspection tool capability.
//!
//! The tool runs a fixed set of read-only `git` subprocess commands against an
//! explicit repository root. It accepts no arbitrary git arguments and performs
//! no mutation, no checkout, no commit, and no network operations.
//!
//! This is not a shell tool: `git` is invoked with structured, fixed argument
//! construction only.

use std::ffi::OsString;
use std::path::PathBuf;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;

use crate::process::{ProcessExecutor, ProcessLimits, ProcessRequest};

/// Maximum bytes of git output returned to the model before truncation.
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepoParams {
    operation: String,
}

#[derive(Debug)]
pub enum RepoError {
    InvalidInput(String),
    Io(String),
}

impl RepoError {
    fn message(&self) -> String {
        match self {
            Self::InvalidInput(m) => format!("invalid repo request: {m}"),
            Self::Io(m) => format!("repo IO error: {m}"),
        }
    }
}

pub struct RepoTool {
    id: CapabilityId,
    root: PathBuf,
}

impl RepoTool {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            id: CapabilityId::new("tool.repo").unwrap(),
            root: root.into(),
        }
    }

    fn command_request(&self, operation: &str) -> Result<ProcessRequest, RepoError> {
        let mut base_args: Vec<OsString> =
            vec![OsString::from("-C"), self.root.as_os_str().to_os_string()];
        match operation {
            "status" => {
                base_args.extend([OsString::from("status"), OsString::from("--short")]);
            }
            "diff" => {
                base_args.extend([OsString::from("diff"), OsString::from("--stat")]);
            }
            "log" => {
                base_args.extend([
                    OsString::from("log"),
                    OsString::from("-n"),
                    OsString::from("20"),
                    OsString::from("--oneline"),
                ]);
            }
            "branch" => {
                base_args.extend([OsString::from("branch"), OsString::from("-a")]);
            }
            "summary" => {
                base_args.extend([
                    OsString::from("log"),
                    OsString::from("-n"),
                    OsString::from("1"),
                    OsString::from("--stat"),
                ]);
            }
            other => {
                return Err(RepoError::InvalidInput(format!(
                    "unsupported operation {other:?}; allowed: status, diff, log, branch, summary"
                )))
            }
        }

        Ok(ProcessRequest::new("git")
            .with_args(base_args)
            .with_working_directory(self.root.clone())
            .with_limits(ProcessLimits::default()))
    }

    fn error_result(&self, call: &ToolCall, error: RepoError) -> ToolResult {
        ToolResult::permanent_error(&call.id, error.message())
    }
}

#[async_trait]
impl ToolCapability for RepoTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "Read-only repository operation",
                    "enum": ["status", "diff", "log", "branch", "summary"]
                }
            },
            "required": ["operation"],
            "additionalProperties": false
        });
        let mut params = apeireth_protocol::canonical::ToolParameters::new();
        params.extend(parameters.as_object().cloned().unwrap_or_default());

        NormalizedTool::new("repo")
            .with_description(
                "Inspect the git repository at the workspace root. Read-only: status, diff, log, branch, summary. No commits, checkout, reset, or other mutation.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let params: RepoParams = match serde_json::from_value(call.arguments.clone()) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("invalid repo parameters: {e}"),
                )
            }
        };

        let operation = params.operation.to_lowercase();
        let request = match self.command_request(&operation) {
            Ok(request) => request,
            Err(e) => return self.error_result(call, e),
        };
        let max_runtime = request.limits().max_runtime;

        let result = match tokio::task::spawn_blocking(move || {
            let executor = ProcessExecutor::new();
            executor.execute(&request)
        })
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => {
                return ToolResult::permanent_error(&call.id, format!("failed to execute git: {e}"))
            }
            Err(join_error) => {
                return ToolResult::retryable_error(
                    &call.id,
                    format!("repo executor task failed: {join_error}"),
                )
            }
        };

        if result.timed_out() {
            return ToolResult::retryable_error(
                &call.id,
                format!("git {operation} timed out after {max_runtime:?}"),
            );
        }

        if result.success() {
            let mut stdout = result.stdout;
            let truncated = result.stdout_truncated || stdout.len() > MAX_OUTPUT_BYTES;
            if stdout.len() > MAX_OUTPUT_BYTES {
                stdout.truncate(MAX_OUTPUT_BYTES);
            }
            let text = if stdout.is_empty() {
                format!("git {operation} completed with empty output")
            } else {
                String::from_utf8_lossy(&stdout).to_string()
            };
            let value = serde_json::json!({
                "operation": operation,
                "output": text,
                "truncated": truncated,
            });
            ToolResult::ok(&call.id, value)
        } else {
            let code = result
                .exit_code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".into());
            let mut stderr = result.stderr;
            if stderr.len() > MAX_OUTPUT_BYTES {
                stderr.truncate(MAX_OUTPUT_BYTES);
            }
            let message = if stderr.is_empty() {
                format!("git {operation} failed with status {code}")
            } else {
                String::from_utf8_lossy(&stderr).to_string()
            };
            ToolResult::permanent_error(&call.id, message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::process::Command;

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn init_repo(root: &std::path::Path) -> bool {
        let run = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };
        run(&["init", "-q"])
            && run(&["config", "user.email", "m2a@test.local"])
            && run(&["config", "user.name", "M2A Test"])
    }

    async fn invoke(tool: &RepoTool, operation: &str) -> ToolResult {
        let call = ToolCall {
            id: "call_repo".into(),
            name: "repo".into(),
            arguments: json!({ "operation": operation }),
        };
        tool.invoke(&call).await
    }

    #[tokio::test]
    async fn status_is_clean_for_a_fresh_repo() {
        if !git_available() {
            eprintln!("git is not installed; skipping repo status test");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        assert!(init_repo(dir.path()));

        let result = invoke(&RepoTool::new(dir.path()), "status").await;
        assert!(result.is_ok());
        let rendered = result.render();
        assert!(rendered.contains("git status"), "{rendered}");
    }

    #[tokio::test]
    async fn status_reports_modified_files() {
        if !git_available() {
            eprintln!("git is not installed; skipping repo status test");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        assert!(init_repo(dir.path()));
        fs::write(dir.path().join("a.txt"), "initial").unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["add", "a.txt"])
            .status();
        let _ = Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["commit", "-q", "-m", "initial"])
            .status();
        fs::write(dir.path().join("a.txt"), "changed").unwrap();

        let result = invoke(&RepoTool::new(dir.path()), "status").await;
        assert!(result.is_ok());
        let rendered = result.render();
        assert!(rendered.contains("M a.txt"), "{rendered}");
    }

    #[tokio::test]
    async fn log_and_diff_are_read_only() {
        if !git_available() {
            eprintln!("git is not installed; skipping repo log/diff test");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        assert!(init_repo(dir.path()));
        fs::write(dir.path().join("a.txt"), "initial").unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["add", "a.txt"])
            .status();
        let _ = Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["commit", "-q", "-m", "initial commit"])
            .status();

        let log = invoke(&RepoTool::new(dir.path()), "log").await;
        assert!(log.is_ok());
        assert!(log.render().contains("initial commit"), "{}", log.render());

        let diff = invoke(&RepoTool::new(dir.path()), "diff").await;
        assert!(diff.is_ok());
    }

    #[tokio::test]
    async fn branch_lists_local_branches() {
        if !git_available() {
            eprintln!("git is not installed; skipping repo branch test");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        assert!(init_repo(dir.path()));

        let result = invoke(&RepoTool::new(dir.path()), "branch").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn non_git_directory_is_an_error() {
        if !git_available() {
            eprintln!("git is not installed; skipping non-git repo test");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let result = invoke(&RepoTool::new(dir.path()), "status").await;
        assert!(!result.is_ok());
        assert!(
            result.render().contains("not a git repository")
                || result.render().contains("failed to execute git"),
            "{}",
            result.render()
        );
    }

    #[tokio::test]
    async fn unsupported_operation_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = invoke(&RepoTool::new(dir.path()), "commit").await;
        assert!(!result.is_ok());
        assert!(
            result.render().contains("unsupported operation"),
            "{}",
            result.render()
        );
    }

    #[tokio::test]
    async fn repo_path_with_spaces_works() {
        if !git_available() {
            eprintln!("git is not installed; skipping repo path-with-spaces test");
            return;
        }
        let dir = tempfile::Builder::new()
            .prefix("apeireth repo ")
            .tempdir()
            .unwrap();
        assert!(init_repo(dir.path()));

        let result = invoke(&RepoTool::new(dir.path()), "status").await;
        assert!(result.is_ok(), "{}", result.render());
    }
}
