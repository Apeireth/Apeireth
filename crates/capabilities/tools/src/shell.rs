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
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::process::{
    current_platform_capabilities, EnforcementLevel, EnvironmentSpec, IsolationCapability,
    IsolationRequirement, ProcessLimits, ProcessRequest, ProcessResult,
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

const SHELL_FROZEN_VERSION: u32 = 1;

/// The exact, versioned execution inputs frozen at approval time.
///
/// This is Shell's own payload schema. Runtime treats it as opaque
/// `serde_json::Value`; Shell owns deserialization and must execute these
/// fields — and only these fields — when resuming an approved operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShellFrozenInvocation {
    version: u32,
    shell_executable: String,
    shell_args: Vec<String>,
    cwd: String,
    timeout_ms: u64,
    max_stdout_bytes: usize,
    max_stderr_bytes: usize,
    /// Actual environment values. The approval view displays names only.
    environment: Vec<(String, String)>,
    isolation: IsolationRequirement,
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

    fn minimal_environment_strings() -> Result<Vec<(String, String)>, String> {
        let mut vars: Vec<(String, String)> = Vec::new();

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
                    let value = value
                        .into_string()
                        .map_err(|_| format!("environment variable {key} is not valid unicode"))?;
                    vars.push((key.to_string(), value));
                }
            }
        }

        #[cfg(not(windows))]
        {
            vars.push((
                "PATH".to_string(),
                "/usr/local/bin:/usr/bin:/bin".to_string(),
            ));
            vars.push(("TMPDIR".to_string(), "/tmp".to_string()));
            vars.push(("LANG".to_string(), "C.UTF-8".to_string()));
        }

        Ok(vars)
    }

    fn isolation_requirements() -> IsolationRequirement {
        IsolationRequirement::new()
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
            )
    }

    fn os_string(value: &str) -> OsString {
        OsString::from(value)
    }

    /// Builds a [`ProcessRequest`] from frozen fields only.
    ///
    /// This deliberately does not call `resolve_cwd_for`, `selected_shell`,
    /// `minimal_environment`, or `resolve_timeout_ms`. If a frozen value is
    /// unusable, it returns a structured failure instead of substituting
    /// current configuration.
    fn process_request_from_frozen(
        frozen: &ShellFrozenInvocation,
    ) -> Result<ProcessRequest, String> {
        if frozen.version != SHELL_FROZEN_VERSION {
            return Err(format!(
                "unsupported frozen shell invocation version {} (expected {})",
                frozen.version, SHELL_FROZEN_VERSION
            ));
        }

        let limits = ProcessLimits {
            max_runtime: Duration::from_millis(frozen.timeout_ms),
            max_stdout_bytes: frozen.max_stdout_bytes,
            max_stderr_bytes: frozen.max_stderr_bytes,
            ..ProcessLimits::default()
        };

        let environment = EnvironmentSpec::Explicit(
            frozen
                .environment
                .iter()
                .map(|(key, value)| (Self::os_string(key), Self::os_string(value)))
                .collect(),
        );

        Ok(
            ProcessRequest::new(Self::os_string(&frozen.shell_executable))
                .with_args(frozen.shell_args.iter().map(|arg| Self::os_string(arg)))
                .with_working_directory(PathBuf::from(&frozen.cwd))
                .with_environment(environment)
                .with_limits(limits)
                .with_isolation(frozen.isolation.clone()),
        )
    }

    fn build_frozen(&self, call: &ToolCall) -> Result<ShellFrozenInvocation, ToolResult> {
        let params: ShellParams = serde_json::from_value(call.arguments.clone()).map_err(|e| {
            ToolResult::permanent_error(&call.id, format!("invalid shell parameters: {e}"))
                .with_name("shell")
        })?;

        if params.command.trim().is_empty() {
            return Err(
                ToolResult::permanent_error(&call.id, "shell command must not be empty")
                    .with_name("shell"),
            );
        }
        if params.command.len() > self.config.max_script_bytes {
            return Err(ToolResult::permanent_error(
                &call.id,
                format!(
                    "shell command is {} bytes; the configured maximum is {} bytes",
                    params.command.len(),
                    self.config.max_script_bytes
                ),
            )
            .with_name("shell"));
        }

        let cwd = self
            .resolve_cwd_for(&params.cwd)
            .map_err(|e| ToolResult::permanent_error(&call.id, e).with_name("shell"))?;
        let timeout_ms = self
            .resolve_timeout_ms(params.timeout_ms)
            .map_err(|e| ToolResult::permanent_error(&call.id, e).with_name("shell"))?;

        let shell_executable = self
            .selected_shell()
            .into_os_string()
            .into_string()
            .map_err(|_| {
                ToolResult::permanent_error(
                    &call.id,
                    "selected shell executable is not valid unicode",
                )
                .with_name("shell")
            })?;
        let shell_args = self
            .shell_args(&params.command)
            .into_iter()
            .map(|arg| {
                arg.into_string().map_err(|_| {
                    ToolResult::permanent_error(&call.id, "shell argument is not valid unicode")
                        .with_name("shell")
                })
            })
            .collect::<Result<Vec<String>, _>>()?;

        let environment = Self::minimal_environment_strings()
            .map_err(|e| ToolResult::permanent_error(&call.id, e).with_name("shell"))?;

        Ok(ShellFrozenInvocation {
            version: SHELL_FROZEN_VERSION,
            shell_executable,
            shell_args,
            cwd: cwd.to_string_lossy().to_string(),
            timeout_ms,
            max_stdout_bytes: self.config.max_stdout_bytes,
            max_stderr_bytes: self.config.max_stderr_bytes,
            environment,
            isolation: Self::isolation_requirements(),
        })
    }

    fn display_invocation(frozen: &ShellFrozenInvocation) -> serde_json::Value {
        let capabilities = current_platform_capabilities();
        serde_json::json!({
            "version": frozen.version,
            "shell_executable": frozen.shell_executable,
            "shell_args": frozen.shell_args,
            "cwd": frozen.cwd,
            "timeout_ms": frozen.timeout_ms,
            "max_stdout_bytes": frozen.max_stdout_bytes,
            "max_stderr_bytes": frozen.max_stderr_bytes,
            "environment_mode": "explicit_minimal",
            "environment_vars": frozen
                .environment
                .iter()
                .map(|(key, _value)| key)
                .collect::<Vec<_>>(),
            "filesystem_isolation": format!("{:?}", capabilities.filesystem_isolation),
            "network_isolation": format!("{:?}", capabilities.network_isolation),
            "process_tree_containment": format!("{:?}", capabilities.process_tree_containment),
        })
    }

    async fn execute_frozen(&self, call: &ToolCall, frozen: &ShellFrozenInvocation) -> ToolResult {
        let request = match Self::process_request_from_frozen(frozen) {
            Ok(request) => request,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("frozen shell invocation unavailable: {e}"),
                )
                .with_name("shell")
            }
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

    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        let frozen = self.build_frozen(call)?;
        let payload = serde_json::to_value(&frozen).map_err(|e| {
            ToolResult::permanent_error(
                &call.id,
                format!("failed to serialize frozen shell invocation: {e}"),
            )
            .with_name("shell")
        })?;
        let display = Self::display_invocation(&frozen);
        Ok(Some(FrozenInvocation::new(payload, display)))
    }

    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        let Some(frozen) = frozen else {
            return self.invoke(call).await;
        };

        let shell_frozen: ShellFrozenInvocation =
            match serde_json::from_value(frozen.payload.clone()) {
                Ok(shell_frozen) => shell_frozen,
                Err(e) => {
                    return ToolResult::permanent_error(
                        &call.id,
                        format!("frozen shell invocation is invalid: {e}"),
                    )
                    .with_name("shell")
                }
            };

        if shell_frozen.version != SHELL_FROZEN_VERSION {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported frozen shell invocation version {} (expected {})",
                    shell_frozen.version, SHELL_FROZEN_VERSION
                ),
            )
            .with_name("shell");
        }

        self.execute_frozen(call, &shell_frozen).await
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let frozen = match self.build_frozen(call) {
            Ok(frozen) => frozen,
            Err(result) => return result,
        };
        self.execute_frozen(call, &frozen).await
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

    #[test]
    fn freeze_invocation_rejects_invalid_cwd_without_pending() {
        let tmp = tempfile::tempdir().unwrap();
        let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: json!({ "command": "echo hi", "cwd": "missing_dir" }),
        };

        let frozen = tool.freeze_invocation(&call);
        assert!(frozen.is_err(), "invalid cwd must fail closed");
        assert!(
            frozen.unwrap_err().render().contains("not accessible"),
            "freeze error must explain why preparation failed"
        );
    }

    #[test]
    fn frozen_display_does_not_expose_environment_values() {
        let tool = ShellTool::new(TrustedShellConfig::new("."));
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: json!({ "command": "echo hi" }),
        };

        let frozen = tool.freeze_invocation(&call).unwrap().unwrap();
        let shell_frozen: ShellFrozenInvocation =
            serde_json::from_value(frozen.payload).expect("payload deserializes");
        let display_text = serde_json::to_string(&frozen.display).unwrap();

        for (_key, value) in &shell_frozen.environment {
            assert!(
                !display_text.contains(value.as_str()),
                "display payload must not expose environment value {value:?}"
            );
        }
        assert!(
            display_text.contains("environment_vars"),
            "display payload should still show environment variable names"
        );
    }

    #[test]
    fn invoke_frozen_uses_frozen_cwd_not_new_config_workspace_root() {
        use apeireth_protocol::canonical::ToolOutcome;

        let tmp = tempfile::tempdir().unwrap();
        let dir_a = tmp.path().join("target_a");
        let dir_b = tmp.path().join("target_b");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();

        #[cfg(windows)]
        let command = "cd";
        #[cfg(not(windows))]
        let command = "pwd";

        let old_tool = ShellTool::new(TrustedShellConfig::new(dir_a.clone()));
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: json!({ "command": command }),
        };
        let frozen = old_tool.freeze_invocation(&call).unwrap().unwrap();

        // Simulate configuration drift after approval: a rebuilt shell tool
        // would now use a different workspace root.
        let new_tool = ShellTool::new(TrustedShellConfig::new(dir_b.clone()));
        let result = tokio_test_invoke_frozen(&new_tool, call.clone(), Some(&frozen));

        assert!(result.is_ok(), "{}", result.render());
        let ToolOutcome::Ok { value } = result.outcome else {
            panic!("expected ok outcome");
        };
        let stdout = value["stdout"].as_str().unwrap_or_default();
        let stdout_canonical = PathBuf::from(stdout.trim())
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(stdout.trim()));
        let expected = dir_a.canonicalize().unwrap();
        let not_expected = dir_b.canonicalize().unwrap();
        assert_eq!(
            stdout_canonical, expected,
            "approved execution must use frozen cwd {expected:?}; got {stdout:?}"
        );
        assert_ne!(
            stdout_canonical, not_expected,
            "approved execution must not re-resolve against new config {not_expected:?}; got {stdout:?}"
        );
    }

    fn tokio_test_invoke(tool: &ShellTool, call: ToolCall) -> ToolResult {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(tool.invoke(&call))
    }

    fn tokio_test_invoke_frozen(
        tool: &ShellTool,
        call: ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(ToolCapability::invoke_frozen(tool, &call, frozen))
    }
}
