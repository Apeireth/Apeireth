//! # Sandbox runtime / isolation / status (per @anthropic-ai/sandbox 商业版 v0.9.21, 1:1 翻译)
//!
//! **STUB MODE**: 3 RuntimeKind + 3 IsolationLevel + 5 SandboxStatus 编译期 hardcode.
//! 真接 docker/firecracker/gvisor 时, 字段保持不变, 仅实现由 stub → 真接替换.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{SandboxError, SandboxResult};

// ============================================================================
// §1 RuntimeKind (3 variant, K-1 强校验 #2)
// ============================================================================

/// 沙箱运行时 (3 variant, 1:1 翻译 @anthropic-ai/sandbox 商业版 v0.9.21).
///
/// K-1 强校验 #2: 编译期 hardcode, 不允许运行时增删 variant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    /// **默认**: Docker daemon (per @anthropic-ai/sandbox v0.9.21 `runtime: "docker"`).
    #[default]
    Docker,
    /// Firecracker microVM (per @anthropic-ai/sandbox v0.9.21 `runtime: "firecracker"`).
    Firecracker,
    /// gVisor (runsc) 用户态内核 (per @anthropic-ai/sandbox v0.9.21 `runtime: "gvisor"`).
    Gvisor,
}

impl RuntimeKind {
    /// 运行时字符串 (1:1 翻译 v0.9.21 商业版 `runtime` 字段).
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeKind::Docker => "docker",
            RuntimeKind::Firecracker => "firecracker",
            RuntimeKind::Gvisor => "gvisor",
        }
    }
}

impl std::fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RuntimeKind {
    type Err = SandboxError;

    fn from_str(s: &str) -> SandboxResult<Self> {
        match s {
            "docker" => Ok(RuntimeKind::Docker),
            "firecracker" => Ok(RuntimeKind::Firecracker),
            "gvisor" => Ok(RuntimeKind::Gvisor),
            other => Err(SandboxError::InvalidConfig(format!(
                "unknown runtime kind: {other}"
            ))),
        }
    }
}

/// 编译期守门: 3 RuntimeKind 守门 (K-1 强校验 #2).
pub const SUPPORTED_RUNTIME_KINDS: &[RuntimeKind] = &[
    RuntimeKind::Docker,
    RuntimeKind::Firecracker,
    RuntimeKind::Gvisor,
];
const _: () = assert!(SUPPORTED_RUNTIME_KINDS.len() == 3);

// ============================================================================
// §2 IsolationLevel (3 variant, K-1 强校验 #3)
// ============================================================================

/// 沙箱隔离级别 (3 variant, 1:1 翻译 @anthropic-ai/sandbox 商业版 v0.9.21).
///
/// K-1 强校验 #3: 编译期 hardcode, 不允许运行时增删 variant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    /// **默认**: 进程级隔离 (per v0.9.21 `isolation: "process"` — Linux namespace + seccomp).
    #[default]
    Process,
    /// 容器级隔离 (per v0.9.21 `isolation: "container"` — Docker / gVisor runsc).
    Container,
    /// 虚拟机级隔离 (per v0.9.21 `isolation: "vm"` — Firecracker microVM).
    Vm,
}

impl IsolationLevel {
    /// 隔离级别字符串 (1:1 翻译 v0.9.21 商业版 `isolation` 字段).
    pub fn as_str(&self) -> &'static str {
        match self {
            IsolationLevel::Process => "process",
            IsolationLevel::Container => "container",
            IsolationLevel::Vm => "vm",
        }
    }
}

impl std::fmt::Display for IsolationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for IsolationLevel {
    type Err = SandboxError;

    fn from_str(s: &str) -> SandboxResult<Self> {
        match s {
            "process" => Ok(IsolationLevel::Process),
            "container" => Ok(IsolationLevel::Container),
            "vm" => Ok(IsolationLevel::Vm),
            other => Err(SandboxError::InvalidConfig(format!(
                "unknown isolation level: {other}"
            ))),
        }
    }
}

/// 编译期守门: 3 IsolationLevel 守门 (K-1 强校验 #3).
pub const SUPPORTED_ISOLATION_LEVELS: &[IsolationLevel] = &[
    IsolationLevel::Process,
    IsolationLevel::Container,
    IsolationLevel::Vm,
];
const _: () = assert!(SUPPORTED_ISOLATION_LEVELS.len() == 3);

// ============================================================================
// §3 SandboxStatus (5 状态机, R21+ 真接 runtime 时用)
// ============================================================================

/// 沙箱状态机 (5 variant, 1:1 翻译 @anthropic-ai/sandbox 商业版 v0.9.21 `status` 字段).
///
/// 状态流转: `Pending → Creating → Running → (Stopping → Stopped) | Failed`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStatus {
    /// 已请求, 等待运行时调度 (per v0.9.21 商业版 `status: "pending"`).
    #[default]
    Pending,
    /// 正在创建 (拉镜像 / 启动 microVM / 启动 runsc).
    Creating,
    /// 正在运行 (容器/进程已启动, 可 stream logs / wait / kill).
    Running,
    /// 正在停止 (graceful shutdown, 等待进程退出).
    Stopping,
    /// 已停止 (正常退出, exit code 可查).
    Stopped,
    /// 失败 (拉镜像失败 / OOM / runtime daemon 错误).
    Failed,
}

impl SandboxStatus {
    /// 状态字符串 (1:1 翻译 v0.9.21 商业版 `status` 字段).
    pub fn as_str(&self) -> &'static str {
        match self {
            SandboxStatus::Pending => "pending",
            SandboxStatus::Creating => "creating",
            SandboxStatus::Running => "running",
            SandboxStatus::Stopping => "stopping",
            SandboxStatus::Stopped => "stopped",
            SandboxStatus::Failed => "failed",
        }
    }
}

impl std::fmt::Display for SandboxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 编译期守门: 5 SandboxStatus 守门 (1:1 翻译 v0.9.21 商业版状态机).
pub const SANDBOX_STATUS_COUNT: usize = 6;
