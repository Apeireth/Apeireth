//! # Sandbox isolation (per @anthropic-ai/sandbox 商业版 v0.9.21, 1:1 翻译)
//!
//! **STUB MODE**: 进程隔离 trait 表面 1:1 翻译, 实际由 docker/firecracker/gvisor 实现.
//! 现阶段 STUB 模式不引任何底层 SDK, 所有 `isolate()` 调用返 `SandboxError::NotImplemented`.
//!
//! 3 隔离级别对应不同底层机制:
//! - `Process`: Linux namespace (PID/Network/Mount) + seccomp + cgroup v2
//! - `Container`: Docker / gVisor runsc
//! - `Vm`: Firecracker microVM (KVM)
//!
//! R21+ 真接时, 每个级别换对应底层 client (bollard / runsc / firecracker-rs).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{SandboxError, SandboxResult};
use crate::runtime::{IsolationLevel, RuntimeKind};

/// 隔离策略描述符 (1:1 翻译 @anthropic-ai/sandbox 商业版 `isolationConfig` 字段).
///
/// STUB 模式: 字段保留 1:1 翻译, 但所有实现返 NotImplemented.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolationConfig {
    /// 隔离级别 (3 选 1, K-1 强校验 #3).
    pub level: IsolationLevel,
    /// 底层运行时 (3 选 1, K-1 强校验 #2).
    pub runtime: RuntimeKind,
    /// PID namespace 启用 (per v0.9.21 商业版 `pidNamespace` 字段).
    #[serde(default = "default_true")]
    pub pid_namespace: bool,
    /// Network namespace 启用 (per v0.9.21 商业版 `networkNamespace` 字段).
    #[serde(default = "default_true")]
    pub network_namespace: bool,
    /// Mount namespace 启用 (per v0.9.21 商业版 `mountNamespace` 字段).
    #[serde(default = "default_true")]
    pub mount_namespace: bool,
    /// seccomp 过滤器 (per v0.9.21 商业版 `seccompProfile`, R21+ 真接时下发).
    #[serde(default)]
    pub seccomp_profile: Option<String>,
    /// cgroup v2 资源 slice (per v0.9.21 商业版 `cgroupSlice`, R21+ 真接时下发).
    #[serde(default)]
    pub cgroup_slice: Option<String>,
    /// 启用的 Linux capabilities (per v0.9.21 商业版 `capabilities`, 白名单).
    #[serde(default)]
    pub capabilities: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            level: IsolationLevel::default(),
            runtime: RuntimeKind::default(),
            pid_namespace: true,
            network_namespace: true,
            mount_namespace: true,
            seccomp_profile: None,
            cgroup_slice: None,
            capabilities: Vec::new(),
        }
    }
}

impl IsolationConfig {
    /// 校验隔离级别和运行时是否兼容 (K-1 强校验 #6).
    ///
    /// 1:1 翻译 v0.9.21 商业版约束:
    /// - `Vm` 隔离只跟 `Firecracker` 兼容
    /// - `Process` 隔离不跟 `Firecracker` 兼容
    /// - `Container` 隔离不跟 `Firecracker` 兼容
    pub fn validate(&self) -> SandboxResult<()> {
        match (self.level, self.runtime) {
            (IsolationLevel::Vm, RuntimeKind::Firecracker) => Ok(()),
            (IsolationLevel::Process, RuntimeKind::Docker | RuntimeKind::Gvisor) => Ok(()),
            (IsolationLevel::Container, RuntimeKind::Docker | RuntimeKind::Gvisor) => Ok(()),
            (level, runtime) => Err(SandboxError::Isolation { runtime, level }),
        }
    }
}

/// Sandbox runtime async trait (R21+ 真接 docker/firecracker/gvisor 时实现).
///
/// STUB 模式: 默认实现返 `SandboxError::NotImplemented`, 防止整合时漏防.
#[async_trait]
pub trait SandboxRuntime: Send + Sync {
    /// 沙箱运行时类型 (Docker / Firecracker / Gvisor).
    fn kind(&self) -> RuntimeKind;
    /// 隔离级别 (Process / Container / Vm).
    fn isolation_level(&self) -> IsolationLevel;
    /// 应用隔离策略 (STUB 返 NotImplemented).
    async fn apply_isolation(&self, _config: &IsolationConfig) -> SandboxResult<()> {
        Err(SandboxError::NotImplemented("apply_isolation"))
    }
    /// 撤销隔离策略 (R21+ 真接时清理 namespace / cgroup / microVM).
    async fn teardown_isolation(&self) -> SandboxResult<()> {
        Err(SandboxError::NotImplemented("teardown_isolation"))
    }
}

/// STUB 默认 runtime (R20 阶段 4 skeleton 阶段, 编译期守 STUB_MODE).
#[derive(Debug, Default)]
pub struct StubSandboxRuntime {
    kind: RuntimeKind,
    level: IsolationLevel,
}

impl StubSandboxRuntime {
    /// 创建 STUB runtime (编译期守门 STUB_MODE 必为 true, R21+ 真接时改返真 client).
    pub fn new(kind: RuntimeKind, level: IsolationLevel) -> Self {
        Self { kind, level }
    }
}

#[async_trait]
impl SandboxRuntime for StubSandboxRuntime {
    fn kind(&self) -> RuntimeKind {
        self.kind
    }
    fn isolation_level(&self) -> IsolationLevel {
        self.level
    }
    // apply_isolation / teardown_isolation 走 trait 默认实现, 返 NotImplemented.
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IsolationConfig validate: Vm + Firecracker 兼容.
    #[test]
    fn isolation_validate_vm_firecracker_ok() {
        let cfg = IsolationConfig {
            level: IsolationLevel::Vm,
            runtime: RuntimeKind::Firecracker,
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
    }

    /// IsolationConfig validate: Vm + Docker 不兼容.
    #[test]
    fn isolation_validate_vm_docker_rejected() {
        let cfg = IsolationConfig {
            level: IsolationLevel::Vm,
            runtime: RuntimeKind::Docker,
            ..Default::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(SandboxError::Isolation { .. })
        ));
    }

    /// IsolationConfig validate: Container + Docker 兼容.
    #[test]
    fn isolation_validate_container_docker_ok() {
        let cfg = IsolationConfig {
            level: IsolationLevel::Container,
            runtime: RuntimeKind::Docker,
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
    }

    /// IsolationConfig validate: Process + Gvisor 兼容.
    #[test]
    fn isolation_validate_process_gvisor_ok() {
        let cfg = IsolationConfig {
            level: IsolationLevel::Process,
            runtime: RuntimeKind::Gvisor,
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
    }

    /// StubSandboxRuntime 返 kind / level 字段 (1:1 翻译 trait 表面).
    #[test]
    fn stub_runtime_returns_kind_and_level() {
        let rt = StubSandboxRuntime::new(RuntimeKind::Gvisor, IsolationLevel::Container);
        assert_eq!(rt.kind(), RuntimeKind::Gvisor);
        assert_eq!(rt.isolation_level(), IsolationLevel::Container);
    }
}
