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
    /// **W1 §2.4 沙箱开关** (2026-10-10, 默认**开** —— 设计拍板"产品定位=桌面
    /// 伴侣"): 开 = 文件限定工作区 + 断网 (AppContainer, 不可实施时**拒绝执行**,
    /// 绝不裸跑); 关 (`APEIRETH_SHELL_SANDBOX=0`) = 本机全权, 显式裸跑自担风险。
    pub sandbox: bool,
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
            sandbox: true,
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

    /// W1 §2.4: 显式关闭沙箱 (裸跑, 本机全权) —— 进阶用户主动选择。
    #[must_use]
    pub fn with_sandbox(mut self, sandbox: bool) -> Self {
        self.sandbox = sandbox;
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
    /// L 组 (2026-09-24 审计): 冻结时固化的规范化 workspace root。执行前
    /// 用它复核 frozen cwd 的包含性 —— 审批等待期 (分钟级) cwd 可能被换成
    /// 指向工作区外的 symlink, 冻结时的校验届时已失效 (TOCTOU)。
    workspace_root: String,
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
        params.extend(parameters.as_object().cloned().unwrap_or_default());
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
            // 展示用: 审批卡上呈现 "/D /S /C <script>" 的形态。真正执行时
            // Windows 走 raw_arg 尾巴 (`/D /S /C "<script>"`), 保证内层双引号
            // 存活 (见 execute_frozen, 2026-10-06 真机引号被吃修复)。
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
                // W1 §2.1 (2026-10-10 真机矩阵定案): AppContainer 进程初始化要
                // 档案基础变量 —— 缺失报 os error 203 "找不到环境选项"。均为标准
                // 用户/档案元数据 (零密钥), "最小化不泄密"原则保留。
                "USERPROFILE",
                "APPDATA",
                "LOCALAPPDATA",
                "HOMEDRIVE",
                "HOMEPATH",
                "USERNAME",
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

    /// W1 §2.4 (2026-10-10): 沙箱开关决定隔离要求集 —— 开 = 在历史集之上**要求**
    /// 文件+网络隔离 Enforced (AppContainer 实施; 平台无能力即拒绝执行, 绝不裸跑),
    /// 关 = 维持历史要求集。
    fn isolation_requirements_for(sandbox: bool) -> IsolationRequirement {
        let base = Self::base_isolation_requirements();
        if sandbox {
            base.require(
                IsolationCapability::FilesystemIsolation,
                crate::process::EnforcementLevel::Enforced,
            )
            .require(
                IsolationCapability::NetworkIsolation,
                crate::process::EnforcementLevel::Enforced,
            )
        } else {
            base
        }
    }

    fn base_isolation_requirements() -> IsolationRequirement {
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

        // Windows: the script travels as a verbatim command-line tail
        // (`/D /S /C "<script>"`) so embedded double quotes survive cmd's /S
        // rule; Rust's normal arg quoting mangles them (2026-10-06 真机:
        // powershell -Command "..." 只回显不执行). Non-Windows keeps argv.
        #[cfg(windows)]
        let request = {
            let script = frozen
                .shell_args
                .last()
                .map(|arg| arg.as_str())
                .unwrap_or_default();
            ProcessRequest::new(Self::os_string(&frozen.shell_executable))
                .with_raw_arg(format!("/D /S /C \"{script}\""))
        };
        #[cfg(not(windows))]
        let request = ProcessRequest::new(Self::os_string(&frozen.shell_executable))
            .with_args(frozen.shell_args.iter().map(|arg| Self::os_string(arg)));

        Ok(request
            .with_working_directory(PathBuf::from(&frozen.cwd))
            .with_environment(environment)
            .with_limits(limits)
            .with_isolation(frozen.isolation.clone()))
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

        // M13 (2026-09-24 审计): guardrail 前置守门接线 —— 冻结前 fail-closed
        // 拦截高危破坏性命令 (`rm -rf /` / `netsh advfirewall set ...` /
        // `reg add` ...)。守门不是沙箱的替代 (AppContainer 仍是主墙), 但
        // `APEIRETH_SHELL_SANDBOX=0` 显式裸跑与审批卡之间必须有这一层内容
        // 过滤; 放在 cwd 解析前, 危险命令不碰文件系统。
        crate::guardrail::ToolGuardrail::verify_shell_command(&params.command).map_err(|e| {
            ToolResult::permanent_error(
                &call.id,
                format!("shell command rejected by pre-call guard: {e}"),
            )
            .with_name("shell")
        })?;

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
            // root 单独 canonicalize 一次 (resolve_cwd_for 内部已解析 cwd,
            // 这里取其 root 一并固化, 供执行前 TOCTOU 复核)。
            workspace_root: self
                .resolve_cwd()
                .map_err(|e| ToolResult::permanent_error(&call.id, e).with_name("shell"))?
                .to_string_lossy()
                .to_string(),
            timeout_ms,
            max_stdout_bytes: self.config.max_stdout_bytes,
            max_stderr_bytes: self.config.max_stderr_bytes,
            environment,
            isolation: Self::isolation_requirements_for(self.config.sandbox),
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
            "sandbox": if frozen.isolation.requires(IsolationCapability::FilesystemIsolation).is_some() {
                "工作区限定 + 断网 (AppContainer)"
            } else {
                "未沙箱 (本机全权)"
            },
            "filesystem_isolation": format!("{:?}", capabilities.filesystem_isolation),
            "network_isolation": format!("{:?}", capabilities.network_isolation),
            "process_tree_containment": format!("{:?}", capabilities.process_tree_containment),
        })
    }

    async fn execute_frozen(&self, call: &ToolCall, frozen: &ShellFrozenInvocation) -> ToolResult {
        // L 组 (2026-09-24 审计): freeze→execute TOCTOU 复核。审批等待期可
        // 达分钟级, 期间 frozen cwd 可能被替换成指向工作区外的 symlink ——
        // 冻结时 (build_frozen) 的包含校验届时已失效。执行前对 frozen cwd
        // 再 canonicalize, 以冻结时固化的 workspace_root 复核包含性; root
        // 自身若被换成 symlink (canonicalize 结果与固化值不一致) 同样拒绝。
        let frozen_root = Path::new(&frozen.workspace_root);
        let cwd_canonical = match std::fs::canonicalize(&frozen.cwd) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "frozen cwd {} is no longer accessible: {e}",
                        frozen.cwd
                    ),
                )
                .with_name("shell")
            }
        };
        let root_now = match std::fs::canonicalize(frozen_root) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "frozen workspace root {} is no longer accessible: {e}",
                        frozen.workspace_root
                    ),
                )
                .with_name("shell")
            }
        };
        if root_now.as_path() != frozen_root || !cwd_canonical.starts_with(frozen_root) {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "frozen cwd {} no longer resolves inside the approved workspace root {}",
                    frozen.cwd, frozen.workspace_root
                ),
            )
            .with_name("shell");
        }

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

    /// Decode captured command output. UTF-8 first; when that fails, the bytes
    /// are almost certainly GBK (cp936) from a Chinese-Windows cmd console —
    /// `String::from_utf8_lossy` alone turned `date`'s output into 乱码
    /// (2026-10-06 真机). GBK fallback restores readable Chinese.
    fn decode_command_output(bytes: &[u8]) -> String {
        match std::str::from_utf8(bytes) {
            Ok(text) => text.to_string(),
            Err(_) => {
                let (decoded, _, _) = encoding_rs::GBK.decode(bytes);
                decoded.into_owned()
            }
        }
    }

    fn format_result(result: &ProcessResult) -> serde_json::Value {
        // W1 §2.2 后置出站凭据绊线接线 (2026-10-10): 此前 scan_and_sanitize_output
        // 零生产调用 (IMPLEMENTED ≠ PRODUCTION WIRED)。shell 输出是唯一"全权出口"
        // (2026-10-06 真机边界测试结论) —— stdout/stderr 双双过绊线, 命中即脱敏
        // 截断并打 credential_tripwire 标记 (审批卡/UI 可见墙的存在)。
        let stdout = crate::guardrail::ToolGuardrail::scan_and_sanitize_output(
            &Self::decode_command_output(&result.stdout),
        );
        let stderr = crate::guardrail::ToolGuardrail::scan_and_sanitize_output(
            &Self::decode_command_output(&result.stderr),
        );
        let mut leaked_kinds = stdout.leaked_kinds.clone();
        for kind in &stderr.leaked_kinds {
            if !leaked_kinds.contains(kind) {
                leaked_kinds.push(*kind);
            }
        }
        serde_json::json!({
            "exit_code": result.exit_code(),
            "timed_out": result.timed_out(),
            "stdout": stdout.sanitized_output,
            "stderr": stderr.sanitized_output,
            "stdout_truncated": result.stdout_truncated,
            "stderr_truncated": result.stderr_truncated,
            "credential_tripwire": {
                "triggered": !leaked_kinds.is_empty(),
                "leaked_kinds": leaked_kinds,
            },
        })
    }
}

#[async_trait]
impl ToolCapability for ShellTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        // W1 §2.4 声明诚实 (2026-10-10): 描述随沙箱状态走 —— 开 = 声明墙的存在
        // (平台无法实施时拒绝执行), 关 = 如实声明本机全权 (双态都不假称)。
        let description = if self.config.sandbox {
            "Executes a platform-native local shell command after explicit user approval. \
             Sandboxed (W1): workspace-directory-only filesystem access and no network \
             (Windows AppContainer); when the platform cannot enforce the sandbox, \
             execution is refused instead of running unsandboxed. \
             APEIRETH_SHELL_SANDBOX=0 explicitly opts out (full user account authority)."
        } else {
            "Executes a platform-native local shell command after explicit user approval. \
             Runs with the user's OS account authority; not a filesystem or network sandbox."
        };
        NormalizedTool::new("shell")
            .with_description(description)
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
    fn command_output_decodes_utf8_then_gbk() {
        // UTF-8 passes through untouched.
        let utf8 = "现在是 13:21".as_bytes();
        assert_eq!(ShellTool::decode_command_output(utf8), "现在是 13:21");
        // GBK bytes (Chinese-Windows cmd, cp936) decode instead of 乱码.
        let gbk = encoding_rs::GBK.encode("日期 2026/09/19").0;
        assert!(
            std::str::from_utf8(&gbk).is_err(),
            "fixture must not be valid UTF-8"
        );
        assert_eq!(ShellTool::decode_command_output(&gbk), "日期 2026/09/19");
    }

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
    fn declaration_honesty_covers_both_sandbox_states() {
        // W1 §2.4 双态诚实 (2026-10-10): 开 = 声明墙的存在与"绝不裸跑"语义;
        // 关 = 如实声明本机全权。两态都不假称。
        let on = ShellTool::new(TrustedShellConfig::new("."));
        let on_desc = on.declaration().description.unwrap_or_default();
        assert!(on_desc.contains("Sandboxed (W1)"), "{on_desc}");
        assert!(
            on_desc.contains("refused instead of running unsandboxed"),
            "{on_desc}"
        );

        let off = ShellTool::new(TrustedShellConfig::new(".").with_sandbox(false));
        let off_desc = off.declaration().description.unwrap_or_default();
        assert!(
            off_desc.contains("not a filesystem or network sandbox"),
            "{off_desc}"
        );
        assert!(off_desc.contains("user approval"), "{off_desc}");
    }

    #[test]
    fn sandbox_default_on_and_knob_turns_off() {
        // W1 §2.4 保护默认 (设计拍板"默认开"): 默认沙箱; with_sandbox(false) 显式裸跑。
        // 要求集随之变化: 开 = 文件+网络 Enforced 要求 (AppContainer 路径), 关 = 历史集。
        assert!(TrustedShellConfig::new(".").sandbox);
        assert!(!TrustedShellConfig::new(".").with_sandbox(false).sandbox);

        let on_req = ShellTool::isolation_requirements_for(true);
        assert!(on_req
            .requires(IsolationCapability::FilesystemIsolation)
            .is_some());
        assert!(on_req
            .requires(IsolationCapability::NetworkIsolation)
            .is_some());

        let off_req = ShellTool::isolation_requirements_for(false);
        assert!(off_req
            .requires(IsolationCapability::FilesystemIsolation)
            .is_none());
        assert!(off_req
            .requires(IsolationCapability::NetworkIsolation)
            .is_none());
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
    fn dangerous_command_is_rejected_by_precall_guard() {
        // M13 (2026-09-24 审计): guardrail 守门在冻结前 fail-closed 接线。
        // 沙箱 (AppContainer) 是主墙, 但 APEIRETH_SHELL_SANDBOX=0 裸跑与
        // 审批卡之间必须有命令内容过滤这一层。
        let tool = ShellTool::new(TrustedShellConfig::new("."));
        for command in [
            "rm -rf / --no-preserve-root",
            "netsh advfirewall set allprofiles off",
            "reg add HKLM\\Software\\x /v y",
        ] {
            let call = ToolCall {
                id: "call_1".into(),
                name: "shell".into(),
                arguments: json!({ "command": command }),
            };
            let frozen = tool.freeze_invocation(&call);
            assert!(frozen.is_err(), "{command} must be rejected before freezing");
            let rendered = frozen.unwrap_err().render();
            assert!(
                rendered.contains("pre-call guard"),
                "{command}: {}",
                rendered
            );
        }
    }

    #[test]
    fn benign_command_still_freezes_normally() {
        // 守门接线不得误伤正常命令 (echo/ping/type 类)。
        let tmp = tempfile::tempdir().unwrap();
        let tool = ShellTool::new(TrustedShellConfig::new(tmp.path().to_path_buf()));
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: json!({ "command": "echo hi" }),
        };
        assert!(tool.freeze_invocation(&call).unwrap().is_some());
    }

    #[test]
    fn frozen_cwd_symlink_swap_is_rejected_before_execution() {
        // L 组 (2026-09-24 审计): freeze→execute TOCTOU。审批等待期内 frozen
        // cwd 被换成指向工作区外的 symlink, 执行前复核必须拒绝。
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("root");
        let work = root.join("work");
        let outside = base.path().join("outside");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        let tool = ShellTool::new(TrustedShellConfig::new(root.clone()));
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: json!({ "command": "echo hi", "cwd": "work" }),
        };
        let frozen = tool.freeze_invocation(&call).unwrap().unwrap();

        // 模拟审批等待期攻击: work 目录被替换成指向外部的 symlink。
        std::fs::remove_dir_all(&work).unwrap();
        #[cfg(unix)]
        let swapped = std::os::unix::fs::symlink(&outside, &work).is_ok();
        #[cfg(windows)]
        let swapped = std::os::windows::fs::symlink_dir(&outside, &work).is_ok();
        if !swapped {
            // Symlink creation often needs extra privileges on Windows.
            return;
        }

        let result = tokio_test_invoke_frozen(&tool, call, Some(&frozen));
        assert!(!result.is_ok(), "swapped cwd must fail closed");
        assert!(
            result
                .render()
                .contains("no longer resolves inside the approved workspace root"),
            "{}",
            result.render()
        );
    }

    #[test]
    fn frozen_display_does_not_expose_environment_values() {
        // 反泄露通道测试 (2026-10-10 重写): 真值子串断言结构性脆 (HOMEDRIVE="C:"
        // 撞路径、USERPROFILE 撞 cwd 展示) 且 crate forbid(unsafe_code) 不能突变
        // 进程 env —— 手造含唯一标记值的冻结载荷, 纯函数直测 display 通道。
        const MARKER_KEY: &str = "PATHEXT";
        const MARKER_VALUE: &str = "APEIRETH_UNIQ_SECRET_MARKER_12345";
        let frozen = ShellFrozenInvocation {
            version: SHELL_FROZEN_VERSION,
            shell_executable: "cmd.exe".to_string(),
            shell_args: vec!["/c".to_string(), "echo hi".to_string()],
            cwd: "C:\\work".to_string(),
            workspace_root: "C:\\work".to_string(),
            timeout_ms: 1_000,
            max_stdout_bytes: 1_024,
            max_stderr_bytes: 1_024,
            environment: vec![(MARKER_KEY.to_string(), MARKER_VALUE.to_string())],
            isolation: ShellTool::base_isolation_requirements(),
        };
        let display_text = serde_json::to_string(&ShellTool::display_invocation(&frozen)).unwrap();

        assert!(
            !display_text.contains(MARKER_VALUE),
            "display payload must not expose environment values: {display_text}"
        );
        assert!(
            display_text.contains("environment_vars"),
            "display payload should still show environment variable names"
        );
        assert!(
            display_text.contains(MARKER_KEY),
            "键名应可见 (approval 可读性)"
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
