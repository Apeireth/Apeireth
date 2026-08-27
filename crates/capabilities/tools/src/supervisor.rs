//! P-arch (2026-08-27): B5 process supervisor trait skeleton.
//!
//! 借鉴 v1 `apeireth-supervisor`（5 sub-supervisor + RestartStrategy + ChildSpec +
//! PidOneSupervisor），v2 形态：
//!
//! - `RestartStrategy` enum (OneForOne / RestForOne / Transient)
//! - `ChildSpec` 声明式子进程规格
//! - `SubSupervisor` trait (5 sub-supervisor: Core/Cognition/Council/Upgrade/Plugin 借鉴)
//! - `ExitReason` + `RestartDecision` (event 流)
//!
//! **0 装 PASS (v2.0 alpha)**:
//! - 仅 trait + 数据类; **不**实现真实 tokio::process::Command 调用
//! - 真实 supervisor 留 v2.0.0-rc (与 ROADMAP §4 P5 路线同步做运行时集成)
//! - v2.0 alpha 是单进程 gateway/cli, supervisor 不是阻塞路径, 但 trait 边界先建立
//!
//! **架构原则**:
//! - trait 在 `capabilities/tools` 与 `ProcessExecutor` 同位 (都是 process-level)
//! - 不依赖 tokio (trait 是 sync, impl 时再 async)
//! - 与 v1 supervisor 1:1 字段对齐 (RestartStrategy 3 态, ChildSpec 包含 cmd/args/env/restart/cwd)
//! - 0 触碰现有 tools 任何 public API
//!
//! 详见 `v2-unabsorbed-features.md` §B5.

use std::time::Duration;

use serde::{Deserialize, Serialize};

// ============================================
// 借鉴 v1: 3 种 RestartStrategy
// ============================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartStrategy {
    /// 一个子进程崩, 只重启那一个 (v1 OneForOne)
    OneForOne,
    /// 一个子进程崩, 重启它**和之后**所有子进程 (v1 RestForOne)
    RestForOne,
    /// 临时退出 (如正常退出码) 不重启; 仅异常退出重启 (v1 Transient)
    Transient,
}

// ============================================
// 借鉴 v1: ChildSpec
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildSpec {
    /// 唯一 id (在 supervisor 内)
    pub id: String,
    /// 启动命令
    pub cmd: String,
    /// 启动参数
    pub args: Vec<String>,
    /// 工作目录
    pub cwd: Option<String>,
    /// 环境变量
    pub env: Vec<(String, String)>,
    /// 重启策略
    pub restart: RestartStrategy,
    /// 启动超时 (0 = 不超时)
    pub start_timeout_ms: u64,
    /// 健康检查间隔 (0 = 不做健康检查)
    pub health_check_interval: Duration,
}

impl ChildSpec {
    pub fn new(id: impl Into<String>, cmd: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            cmd: cmd.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            restart: RestartStrategy::OneForOne,
            start_timeout_ms: 30_000,
            health_check_interval: Duration::from_secs(0),
        }
    }
}

// ============================================
// 借鉴 v1: ExitReason + RestartDecision
// ============================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitReason {
    /// 正常退出码
    Normal { code: i32 },
    /// 异常退出
    Abnormal { code: i32 },
    /// 信号杀掉 (SIGKILL 等)
    Signaled { signal: i32 },
    /// supervisor 自己 kill
    Killed,
    /// 健康检查失败
    HealthCheckFailed,
    /// 启动超时
    StartTimeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartDecision {
    /// 不重启 (按 RestartStrategy 决定)
    DoNotRestart,
    /// 立即重启
    RestartNow,
    /// 延迟重启 (在 0-5s 范围)
    DelayedRestart { delay_ms: u64 },
}

// ============================================
// 借鉴 v1: SubSupervisor trait (5 sub-supervisor 的抽象)
// ============================================

/// 5 个 sub-supervisor (v1: Core / Cognition / Council / Upgrade / Plugin)
/// v2.0 alpha 仅 trait 边界; 真实监督 5 个领域留 v2.0.0-rc
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubSupervisorKind {
    /// 核心 (Runtime)
    Core,
    /// 认知 (Orchestrator)
    Cognition,
    /// 评审 (Council)
    Council,
    /// 升级
    Upgrade,
    /// 插件
    Plugin,
}

/// Sub-supervisor trait (v2.0 alpha: 0 装, 不实际启进程)
pub trait SubSupervisor: Send + Sync {
    /// 启动 supervisor 树
    fn start(&mut self) -> Result<(), SupervisorError>;

    /// 停止
    fn stop(&mut self) -> Result<(), SupervisorError>;

    /// 列出受监督的 child
    fn children(&self) -> Vec<ChildSpec>;

    /// 报告 child 退出 (event 流, 由 runtime 调)
    fn on_child_exit(
        &mut self,
        child_id: &str,
        reason: ExitReason,
    ) -> Result<RestartDecision, SupervisorError>;

    /// 名字 (用于监控/日志)
    fn name(&self) -> &'static str;
}

// ============================================
// 错误 (P-arch 2026-08-27 + 子代理 A 建议: 显式 derive Send + Sync)
// ============================================

#[derive(Debug)]
pub enum SupervisorError {
    /// 0 装: 启动失败
    StartFailed(String),
    /// 0 装: child 不存在
    UnknownChild(String),
    /// 0 装: 重启次数超限 (v1: max_restarts per period)
    RestartLimitExceeded(String),
    /// IO
    Io(String),
}

impl std::fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StartFailed(msg) => write!(f, "supervisor start failed: {msg}"),
            Self::UnknownChild(id) => write!(f, "unknown child: {id}"),
            Self::RestartLimitExceeded(msg) => {
                write!(f, "restart limit exceeded: {msg}")
            }
            Self::Io(msg) => write!(f, "io: {msg}"),
        }
    }
}

impl std::error::Error for SupervisorError {}

// Send / Sync 由编译器自动派生: 字段全 String, 满足自动推导条件.
// crate 内 #![deny(unsafe_code)] 不允许 unsafe impl, 自动派生无需 unsafe impl.
// 上一版曾尝试 `#[derive(Debug, Send, Sync)]`, 但 Send / Sync 是 unsafe trait 不能 derive, 已修正.

// ============================================
// v2.0 alpha 唯一真实现: NoopSubSupervisor (0 装测试用)
// ============================================

/// 0 装: 不启任何进程, 全部返空
pub struct NoopSubSupervisor {
    kind: SubSupervisorKind,
    children: Vec<ChildSpec>,
}

impl NoopSubSupervisor {
    pub fn new(kind: SubSupervisorKind) -> Self {
        Self {
            kind,
            children: Vec::new(),
        }
    }
}

impl SubSupervisor for NoopSubSupervisor {
    fn start(&mut self) -> Result<(), SupervisorError> {
        // 0 装: 不假装启动
        Ok(())
    }

    fn stop(&mut self) -> Result<(), SupervisorError> {
        Ok(())
    }

    fn children(&self) -> Vec<ChildSpec> {
        self.children.clone()
    }

    fn on_child_exit(
        &mut self,
        child_id: &str,
        _reason: ExitReason,
    ) -> Result<RestartDecision, SupervisorError> {
        // 0 装: 永远不重启 (Noop 行为)
        let _ = child_id;
        Ok(RestartDecision::DoNotRestart)
    }

    fn name(&self) -> &'static str {
        match self.kind {
            SubSupervisorKind::Core => "noop_core",
            SubSupervisorKind::Cognition => "noop_cognition",
            SubSupervisorKind::Council => "noop_council",
            SubSupervisorKind::Upgrade => "noop_upgrade",
            SubSupervisorKind::Plugin => "noop_plugin",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0 装: NoopSubSupervisor 不启进程
    #[test]
    fn noop_sub_supervisor_does_not_spawn() {
        let mut s = NoopSubSupervisor::new(SubSupervisorKind::Core);
        assert!(s.start().is_ok());
        assert!(s.stop().is_ok());
        assert_eq!(s.children().len(), 0);
        assert_eq!(s.name(), "noop_core");
    }

    /// 0 装: Noop 永远不重启 (测试 RestartStrategy 的副作用)
    #[test]
    fn noop_sub_supervisor_never_restarts() {
        let mut s = NoopSubSupervisor::new(SubSupervisorKind::Cognition);
        for reason in [
            ExitReason::Normal { code: 0 },
            ExitReason::Abnormal { code: 1 },
            ExitReason::Signaled { signal: 9 },
        ] {
            let d = s.on_child_exit("any", reason).unwrap();
            assert_eq!(d, RestartDecision::DoNotRestart);
        }
    }

    /// RestartStrategy 3 态序列化往返
    #[test]
    fn restart_strategy_serde_roundtrip() {
        for s in [
            RestartStrategy::OneForOne,
            RestartStrategy::RestForOne,
            RestartStrategy::Transient,
        ] {
            let json = serde_json::to_string(&s).unwrap();
            let back: RestartStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(back, s);
        }
    }

    /// ChildSpec 构造 + 序列化 (no 实际 spawn)
    #[test]
    fn child_spec_construction_serde() {
        let mut spec = ChildSpec::new("worker-1", "/usr/bin/apeireth-worker");
        spec.args = vec!["--port=8080".into()];
        spec.cwd = Some("/var/lib/apeireth".into());
        spec.env = vec![("RUST_LOG".into(), "info".into())];
        spec.restart = RestartStrategy::RestForOne;
        let json = serde_json::to_string(&spec).unwrap();
        let back: ChildSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "worker-1");
        assert_eq!(back.restart, RestartStrategy::RestForOne);
    }
}
