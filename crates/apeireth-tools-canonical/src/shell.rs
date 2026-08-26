//! Cross-platform Trusted Shell capability.
//!
//! `tool.shell` executes a platform-native local shell command after explicit
//! human approval. It is **not** a filesystem sandbox and **not** a network
//! sandbox. The command runs with the user's effective OS account authority;
//! `ProcessExecutor` supplies bounded lifetime/output and a minimal explicit
//! environment.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;

use crate::process::{
    EnvironmentSpec, IsolationCapability, IsolationRequirement, ProcessLimits, ProcessRequest,
    ProcessResult,
};

/// Configuration for the M2C-T Trusted Shell capability.
#[derive(Debug, Clone)]
pub struct TrustedShellConfig {
    /// The workspace root. Every shell invocation runs with this directory as
    /// its explicit base. A relative `cwd` is resolved under this root and is
    /// rejected if it escapes. This is execution context, **not** a filesystem
    /// sandbox.
    pub workspace_root: PathBuf,
    /// Explicit shell executable path. When `None`, the platform default is
    /// used: `cmd.exe` on Windows and `/bin/sh` on Unix.
    pub shell_executable: Option<PathBuf>,
    /// Maximum accepted command/script size in UTF-8 bytes.
    pub max_script_bytes: usize,
    /// Default timeout in milliseconds when the model does not supply one.
    pub default_timeout_ms: u64,
    /// Hard maximum configurable timeout in milliseconds.
    pub max_timeout_ms: u64,
    /// stdout bound in bytes.
    pub max_stdout_bytes: usize,
    /// stderr bound in bytes.
    pub max_stderr_bytes: usize,
}

impl Default for TrustedShellConfig {
    fn default() -> Self {
        Self {
            workspace_root: PathBuf::from("."),
            shell_executable: None,
            max_script_bytes: 64 * 1024,
            default_timeout_ms: 30_000,
            max_timeout_ms: 300_000,
            max_stdout_bytes: 64 * 1024,
            max_stderr_bytes: 64 * 1024,
        }
    }
}

impl TrustedShellConfig {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_shell_executable(mut self, executable: impl Into<PathBuf>) -> Self {
        self.shell_executable = Some(executable.into());
        self
    }

    #[must_use]
    pub fn with_max_script_bytes(mut self, bytes: usize) -> Self {
        self.max_script_bytes = bytes;
        self
    }

    #[must_use]
    pub fn with_timeouts(mut self, default_ms: u64, max_ms: u64) -> Self {
        self.default_timeout_ms = default_ms;
        self.max_timeout_ms = max_ms;
        self
    }

    #[must_use]
    pub fn with_output_bounds(mut self, stdout: usize, stderr: usize) -> Self {
        self.max_stdout_bytes = stdout;
        self.max_stderr_bytes = stderr;
        self
    }
}

#[derive(Debug, Deserialize)]
struct ShellParams {
    command: String,
    cwd: Option<String>,
    timeout_ms: Option<u64>,
}

/// The M2C-T Trusted Shell tool.
pub struct ShellTool {
    id: CapabilityId,
    config: TrustedShellConfig,
}

impl ShellTool {
    pub fn new(config: TrustedShellConfig) -> Self {
        Self {
            id: CapabilityId::new("tool.shell").unwrap(),
            config,
        }
    }

    pub fn config(&self) -> &TrustedShellConfig {
        &self.config
    }

    fn declaration_parameters() -> ToolParameters {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Exact shell command or script to run. It is shown to the user for approval and runs unchanged."
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory relative to the trusted workspace root. Defaults to the root."
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional timeout in milliseconds. The effective timeout is shown in the approval request."
                }
            },
            "required": ["command"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(
            parameters
                .as_object()
                .cloned()
                .unwrap_or_default()
                .into_iter(),
        );
        params
    }

    fn resolve_cwd(&self) -> Result<PathBuf, String> {
        let root = self
            .config
            .workspace_root
            .canonicalize()
            .map_err(|e| format!("workspace root is not accessible: {e}"))?;
        Ok(root)
    }

    fn resolve_cwd_for(&self, cwd: &Option<String>) -> Result<PathBuf, String> {
        let root = self.resolve_cwd()?;
        let Some(relative) = cwd.as_deref() else {
            return Ok(root);
        };
        if relative.trim().is_empty() {
            return Ok(root);
        }
        let candidate = root.join(relative);
        let candidate = candidate
            .canonicalize()
            .map_err(|e| format!("cwd {relative:?} is not accessible: {e}"))?;
        if !candidate.starts_with(&root) {
            return Err(format!("cwd {relative:?} escapes the workspace root"));
        }
        if !candidate.is_dir() {
            return Err(format!("cwd {relative:?} is not a directory"));
        }
        Ok(candidate)
    }

    fn resolve_timeout_ms(&self, requested: Option<u64>) -> Result<u64, String> {
        let timeout_ms = requested.unwrap_or(self.config.default_timeout_ms);
        if timeout_ms == 0 {
            return Err("timeout_ms must be non-zero".into());
        }
        if timeout_ms > self.config.max_timeout_ms {
            return Err(format!(
                "timeout_ms {timeout_ms} exceeds the configured maximum {}",
                self.config.max_timeout_ms
            ));
        }
        Ok(timeout_ms)
    }

    fn selected_shell(&self) -> PathBuf {
        if let Some(explicit) = &self.config.shell_executable {
            return explicit.clone();
        }

        #[cfg(windows)]
        {
            if let Some(root) = std::env::var_os("SystemRoot") {
                let candidate = PathBuf::from(root).join("System32").join("cmd.exe");
                if candidate.is_file() {
                    return candidate;
                }
            }
            PathBuf::from("cmd.exe")
        }

        #[cfg(not(windows))]
        {
            PathBuf::from("/bin/sh")
        }
    }

    fn shell_args(&self, script: &str) -> Vec<OsString> {
        #[cfg(windows)]
        {
            vec![
                OsString::from("/D"),
                OsString::from("/S"),
                OsString::from("/C"),
                OsString::from(script),
            ]
        }

        #[cfg(not(windows))]
        {
            vec![OsString::from("-c"), OsString::from(script)]
        }
    }

    fn minimal_environment() -> EnvironmentSpec {
        let mut vars: Vec<(OsString, OsString)> = Vec::new();

        #[cfg(windows)]
        {
            for key in [
                "SystemRoot",
                "WINDIR",
                "TEMP",
                "TMP",
                "PATH",
                "PATHEXT",
                "COMSPEC",
            ] {
                if let Some(value) = std::env::var_os(key) {
                    vars.push((OsString::from(key), value));
                }
            }
        }

        #[cfg(not(windows))]
        {
            vars.push((
                OsString::from("PATH"),
                OsString::from("/usr/local/bin:/usr/bin:/bin"),
            ));
            vars.push((OsString::from("TMPDIR"), OsString::from("/tmp")));
            vars.push((OsString::from("LANG"), OsString::from("C.UTF-8")));
        }

        EnvironmentSpec::Explicit(vars)
    }

    fn process_request(
        &self,
        cwd: PathBuf,
        timeout_ms: u64,
        script: &str,
    ) -> Result<ProcessRequest, String> {
        let limits = ProcessLimits {
            max_runtime: Duration::from_millis(timeout_ms),
            max_stdout_bytes: self.config.max_stdout_bytes,
            max_stderr_bytes: self.config.max_stderr_bytes,
            ..ProcessLimits::default()
        };

        let isolation = IsolationRequirement::new()
            .require(
                IsolationCapability::StructuredSpawn,
                crate::process::EnforcementLevel::Enforced,
            )
            .require(
                IsolationCapability::ExplicitCwd,
                crate::process::EnforcementLevel::Enforced,
            )
            .require(
                IsolationCapability::Timeout,
                crate::process::EnforcementLevel::Enforced,
            )
            .require(
                IsolationCapability::StdoutLimit,
                crate::process::EnforcementLevel::Enforced,
            )
            .require(
                IsolationCapability::StderrLimit,
                crate::process::EnforcementLevel::Enforced,
            )
            .require(
                IsolationCapability::EnvironmentIsolation,
                crate::process::EnforcementLevel::Enforced,
            )
            .require(
                IsolationCapability::ProcessTreeContainment,
                crate::process::EnforcementLevel::Partial,
            )
            .require(
                IsolationCapability::FailClosedPreExecutionContainment,
                crate::process::EnforcementLevel::Enforced,
            );

        Ok(ProcessRequest::new(self.selected_shell())
            .with_args(self.shell_args(script))
            .with_working_directory(cwd)
            .with_environment(Self::minimal_environment())
            .with_limits(limits)
            .with_isolation(isolation))
    }

    fn environment_var_names() -> Vec<&'static str> {
        #[cfg(windows)]
        {
            vec![
                "SystemRoot",
                "WINDIR",
                "TEMP",
                "TMP",
                "PATH",
                "PATHEXT",
                "COMSPEC",
            ]
        }

        #[cfg(not(windows))]
        {
            vec!["PATH", "TMPDIR", "LANG"]
        }
    }

    fn format_result(result: &ProcessResult) -> serde_json::Value {
        serde_json::json!({
            "exit_code": result.exit_code(),
            "timed_out": result.timed_out(),
            "stdout": String::from_utf8_lossy(&result.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&result.stderr).to_string(),
            "stdout_truncated": result.stdout_truncated,
            "stderr_truncated": result.stderr_truncated,
        })
    }
}

#[async_trait]
impl ToolCapability for ShellTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("shell")
            .with_description(
                "Executes a platform-native local shell command after explicit user approval. \
                 Runs with the user's OS account authority; not a filesystem or network sandbox.",
            )
            .with_parameters(Self::declaration_parameters())
    }

    fn freeze_invocation(&self, call: &ToolCall) -> Option<serde_json::Value> {
        let params: Result<ShellParams, _> = serde_json::from_value(call.arguments.clone());
        let params = match params {
            Ok(params) => params,
            Err(e) => {
                return Some(serde_json::json!({
                    "normalization_error": format!("invalid shell parameters: {e}"),
                }))
            }
        };

        if params.command.trim().is_empty() {
            return Some(serde_json::json!({
                "normalization_error": "shell command must not be empty",
            }));
        }
        if params.command.len() > self.config.max_script_bytes {
            return Some(serde_json::json!({
                "normalization_error": format!(
                    "shell command is {} bytes; the configured maximum is {} bytes",
                    params.command.len(),
                    self.config.max_script_bytes
                ),
            }));
        }

        let cwd = match self.resolve_cwd_for(&params.cwd) {
            Ok(cwd) => cwd.to_string_lossy().to_string(),
            Err(e) => e,
        };
        let timeout_ms = match self.resolve_timeout_ms(params.timeout_ms) {
            Ok(timeout_ms) => timeout_ms.to_string(),
            Err(e) => e,
        };
        let capabilities = crate::process::current_platform_capabilities();

        Some(serde_json::json!({
            "shell_executable": self.selected_shell().to_string_lossy().to_string(),
            "cwd": cwd,
            "timeout_ms": timeout_ms,
            "environment_mode": "explicit_minimal",
            "environment_vars": Self::environment_var_names(),
            "filesystem_isolation": format!("{:?}", capabilities.filesystem_isolation),
            "network_isolation": format!("{:?}", capabilities.network_isolation),
            "process_tree_containment": format!("{:?}", capabilities.process_tree_containment),
        }))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let params: ShellParams = match serde_json::from_value(call.arguments.clone()) {
            Ok(params) => params,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("invalid shell parameters: {e}"),
                )
                .with_name("shell")
            }
        };

        if params.command.trim().is_empty() {
            return ToolResult::permanent_error(&call.id, "shell command must not be empty")
                .with_name("shell");
        }
        if params.command.len() > self.config.max_script_bytes {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "shell command is {} bytes; the configured maximum is {} bytes",
                    params.command.len(),
                    self.config.max_script_bytes
                ),
            )
            .with_name("shell");
        }

        let cwd = match self.resolve_cwd_for(&params.cwd) {
            Ok(cwd) => cwd,
            Err(e) => return ToolResult::permanent_error(&call.id, e).with_name("shell"),
        };
        let timeout_ms = match self.resolve_timeout_ms(params.timeout_ms) {
            Ok(timeout_ms) => timeout_ms,
            Err(e) => return ToolResult::permanent_error(&call.id, e).with_name("shell"),
        };
        let request = match self.process_request(cwd, timeout_ms, &params.command) {
            Ok(request) => request,
            Err(e) => return ToolResult::permanent_error(&call.id, e).with_name("shell"),
        };

        let result = tokio::task::spawn_blocking(move || {
            crate::process::ProcessExecutor::new().execute(&request)
        })
        .await;

        match result {
            Ok(Ok(process_result)) => {
                let value = Self::format_result(&process_result);
                ToolResult::ok(&call.id, value).with_name("shell")
            }
            Ok(Err(process_error)) => ToolResult::permanent_error(
                &call.id,
                format!("shell process execution failed: {process_error}"),
            )
            .with_name("shell"),
            Err(join_error) => ToolResult::retryable_error(
                &call.id,
                format!("shell execution task failed to join: {join_error}"),
            )
            .with_name("shell"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn declaration_is_honest_and_not_sandbox_named() {
        let tool = ShellTool::new(TrustedShellConfig::new("."));
        let declaration = tool.declaration();
        assert_eq!(declaration.name, "shell");
        let description = declaration.description.unwrap_or_default();
        assert!(!description.contains("sandboxed shell"), "{description}");
        assert!(!description.contains("safe shell"), "{description}");
        assert!(!description.contains("secure shell"), "{description}");
        assert!(description.contains("user approval"), "{description}");
    }

    #[test]
    fn empty_command_is_rejected() {
        let tool = ShellTool::new(TrustedShellConfig::new("."));
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: json!({ "command": "   " }),
        };
        let result = tokio_test_invoke(&tool, call);
        assert!(!result.is_ok());
        assert!(
            result.render().contains("must not be empty"),
            "{}",
            result.render()
        );
    }

    #[test]
    fn oversized_command_is_rejected() {
        let tool = ShellTool::new(TrustedShellConfig::new(".").with_max_script_bytes(4));
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: json!({ "command": "12345" }),
        };
        let result = tokio_test_invoke(&tool, call);
        assert!(!result.is_ok());
        assert!(result.render().contains("maximum"), "{}", result.render());
    }

    #[test]
    fn zero_timeout_is_rejected() {
        let tool = ShellTool::new(TrustedShellConfig::new("."));
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: json!({ "command": "echo hi", "timeout_ms": 0 }),
        };
        let result = tokio_test_invoke(&tool, call);
        assert!(!result.is_ok());
        assert!(result.render().contains("non-zero"), "{}", result.render());
    }

    #[test]
    fn timeout_above_max_is_rejected() {
        let tool = ShellTool::new(TrustedShellConfig::new(".").with_timeouts(30_000, 60_000));
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: json!({ "command": "echo hi", "timeout_ms": 61_000 }),
        };
        let result = tokio_test_invoke(&tool, call);
        assert!(!result.is_ok());
        assert!(result.render().contains("maximum"), "{}", result.render());
    }

    #[test]
    fn cwd_escape_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: json!({ "command": "echo hi", "cwd": "../" }),
        };
        let result = tokio_test_invoke(&tool, call);
        assert!(!result.is_ok());
        assert!(result.render().contains("escapes"), "{}", result.render());
    }

    fn tokio_test_invoke(tool: &ShellTool, call: ToolCall) -> ToolResult {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(tool.invoke(&call))
    }
}
