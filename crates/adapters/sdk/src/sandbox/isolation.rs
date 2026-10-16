//! # Sandbox isolation — 隔离配置校验与落点计划
//!
//! 3 隔离级别对应编排服务侧不同强制执行机制 (进程级 / 容器级 / 虚拟机级)。
//! 本层的职责是**校验 + 计划**:
//!
//! - [`IsolationConfig::validate`] — 级别/运行时兼容矩阵 + capability 白名单校验;
//! - [`IsolationPlan::plan`] — 把通过校验的隔离配置投影成下发计划
//!   (随创建请求发给编排服务, 强制执行点在服务侧)。
//!
//! 本层不做"假装已隔离": 隔离的强制执行属于编排服务的职责, 客户端只保证
//! 下发的计划合法且能力面收敛在白名单内。

use serde::{Deserialize, Serialize};

use crate::sandbox::error::{SandboxError, SandboxResult};
use crate::sandbox::runtime::{IsolationLevel, RuntimeKind};

/// capability 白名单 (编译期 hardcode)。白名单外 / 通配符 / 全量声明一律拒绝:
/// 能力面只允许显式列出的最小集合, 不允许"顺手多给"。
pub const ALLOWED_CAPABILITIES: &[&str] = &[
    "CAP_NET_BIND_SERVICE",
    "CAP_CHOWN",
    "CAP_FOWNER",
    "CAP_KILL",
    "CAP_SETUID",
    "CAP_SETGID",
];

/// 隔离策略描述符 (随创建请求下发)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolationConfig {
    /// 隔离级别 (3 选 1)。
    pub level: IsolationLevel,
    /// 底层运行时 (3 选 1)。
    pub runtime: RuntimeKind,
    /// PID namespace 启用。
    #[serde(default = "default_true")]
    pub pid_namespace: bool,
    /// Network namespace 启用。
    #[serde(default = "default_true")]
    pub network_namespace: bool,
    /// Mount namespace 启用。
    #[serde(default = "default_true")]
    pub mount_namespace: bool,
    /// seccomp 过滤器名 (由编排服务侧解析)。
    #[serde(default)]
    pub seccomp_profile: Option<String>,
    /// cgroup v2 资源 slice (由编排服务侧挂载)。
    #[serde(default)]
    pub cgroup_slice: Option<String>,
    /// 申请的 Linux capabilities (白名单校验, 见 [`ALLOWED_CAPABILITIES`])。
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
    /// 校验隔离级别/运行时兼容矩阵 + capability 白名单。
    ///
    /// 兼容矩阵:
    /// - `Vm` 隔离只与 `Firecracker` 运行时兼容
    /// - `Process` / `Container` 隔离与 `Docker` / `Gvisor` 运行时兼容
    ///
    /// capability 校验: 每一项必须精确命中 [`ALLOWED_CAPABILITIES`];
    /// 白名单外、含通配语义 (`*` / `ALL`)、空串一律拒绝。
    pub fn validate(&self) -> SandboxResult<()> {
        match (self.level, self.runtime) {
            (IsolationLevel::Vm, RuntimeKind::Firecracker) => {}
            (IsolationLevel::Process, RuntimeKind::Docker | RuntimeKind::Gvisor) => {}
            (IsolationLevel::Container, RuntimeKind::Docker | RuntimeKind::Gvisor) => {}
            (level, runtime) => {
                return Err(SandboxError::Isolation { runtime, level });
            }
        }
        for capability in &self.capabilities {
            let trimmed = capability.trim();
            if trimmed.is_empty() || trimmed.contains('*') {
                return Err(SandboxError::InvalidConfig(format!(
                    "capability must be a concrete name: {capability:?}"
                )));
            }
            if !ALLOWED_CAPABILITIES.contains(&trimmed) {
                return Err(SandboxError::InvalidConfig(format!(
                    "capability not in whitelist: {trimmed}"
                )));
            }
        }
        Ok(())
    }
}

/// 隔离落点计划: 通过校验的隔离配置 + 收敛后的能力面。
///
/// 计划随创建请求下发; 编排服务按计划强制执行。客户端不假装自己能强制隔离。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolationPlan {
    /// 通过校验的隔离配置。
    pub config: IsolationConfig,
    /// 收敛后的能力面 (与白名单求交, 保持输入顺序, 去重)。
    pub granted_capabilities: Vec<String>,
}

impl IsolationPlan {
    /// 校验并投影成计划。非法配置直接收口成对应闭合错误。
    pub fn plan(config: &IsolationConfig) -> SandboxResult<Self> {
        config.validate()?;
        let mut granted: Vec<String> = Vec::new();
        for capability in &config.capabilities {
            let trimmed = capability.trim().to_string();
            if !granted.contains(&trimmed) {
                granted.push(trimmed);
            }
        }
        Ok(Self {
            config: config.clone(),
            granted_capabilities: granted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(level: IsolationLevel, runtime: RuntimeKind) -> IsolationConfig {
        IsolationConfig {
            level,
            runtime,
            ..Default::default()
        }
    }

    /// 兼容矩阵: Vm + Firecracker 兼容。
    #[test]
    fn isolation_validate_vm_firecracker_ok() {
        assert!(cfg(IsolationLevel::Vm, RuntimeKind::Firecracker)
            .validate()
            .is_ok());
    }

    /// 兼容矩阵: Vm + Docker 不兼容。
    #[test]
    fn isolation_validate_vm_docker_rejected() {
        assert!(matches!(
            cfg(IsolationLevel::Vm, RuntimeKind::Docker).validate(),
            Err(SandboxError::Isolation { .. })
        ));
    }

    /// 兼容矩阵: Container + Docker 兼容。
    #[test]
    fn isolation_validate_container_docker_ok() {
        assert!(cfg(IsolationLevel::Container, RuntimeKind::Docker)
            .validate()
            .is_ok());
    }

    /// 兼容矩阵: Process + Gvisor 兼容。
    #[test]
    fn isolation_validate_process_gvisor_ok() {
        assert!(cfg(IsolationLevel::Process, RuntimeKind::Gvisor)
            .validate()
            .is_ok());
    }

    /// capability 白名单: 白名单内放行, 白名单外 / 通配 / 空串拒绝。
    #[test]
    fn capability_whitelist_is_enforced() {
        let mut ok = cfg(IsolationLevel::Container, RuntimeKind::Docker);
        ok.capabilities = vec!["CAP_NET_BIND_SERVICE".into(), "CAP_KILL".into()];
        assert!(ok.validate().is_ok());

        let mut wild = cfg(IsolationLevel::Container, RuntimeKind::Docker);
        wild.capabilities = vec!["CAP_SYS_ADMIN".into()];
        assert!(matches!(
            wild.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));

        let mut star = cfg(IsolationLevel::Container, RuntimeKind::Docker);
        star.capabilities = vec!["CAP_*".into()];
        assert!(matches!(
            star.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));

        let mut empty = cfg(IsolationLevel::Container, RuntimeKind::Docker);
        empty.capabilities = vec!["".into()];
        assert!(matches!(
            empty.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));
    }

    /// 落点计划: 校验 + 能力面去重收敛。
    #[test]
    fn isolation_plan_projects_and_dedups_capabilities() {
        let mut config = cfg(IsolationLevel::Container, RuntimeKind::Gvisor);
        config.capabilities = vec!["CAP_KILL".into(), "CAP_KILL".into(), "CAP_CHOWN".into()];
        let plan = IsolationPlan::plan(&config).expect("plan");
        assert_eq!(
            plan.granted_capabilities,
            vec!["CAP_KILL".to_string(), "CAP_CHOWN".to_string()]
        );
        assert_eq!(plan.config, config);

        let mut bad = config;
        bad.capabilities = vec!["CAP_SYS_MODULE".into()];
        assert!(matches!(
            IsolationPlan::plan(&bad),
            Err(SandboxError::InvalidConfig(_))
        ));
    }
}
