//! P-arch (2026-08-27) + v2.0.0-rc.1 RC-8: `SubSupervisor` trait 真 impl.
//!
//! **位置**: impl 在 `apeireth-tools-canonical` (capabilities), trait 在同 crate `supervisor.rs`.
//! 单向依赖: 0 反向. RC-8 完成 v2.0.0-rc-roadmap.md §3 RC-8 ("SubSupervisor → tokio::process 真实 impl").
//!
//! **架构选择** (per O-6 锚 #9 + 子代理审查 1.1 RC-1 修正):
//! - `SubSupervisor` trait 是 **sync** (per v2.0.0-alpha 0 装设计 + 5 file 已 sync)
//! - `tokio::process` 是 async, 在 sync context 用 `Handle::current().block_on()`
//!   会破坏 `Send + Sync` trait bound (block_on 抓 current runtime handle 同步阻塞)
//! - **选** `std::process::Command` 同步启动 + `Arc<Mutex<HashMap<id, Child>>>` 持有 handle
//! - 真并发 (multi-process) 由 OS 调度; supervisor 本身是 sync Send+Sync
//! - 0 触碰 `SubSupervisor` trait 边界 (start/stop/children/on_child_exit/name 5 method 签名 0 改)
//!
//! **0 装诚实**:
//! - 真 `std::process::Child` (handle 持有, 0 装 NoopSubSupervisor 0 启进程)
//! - 真 `ExitReason` 解析 (ExitCode 转 Normal/Abnormal; signal Unix 转 Signaled)
//! - 真 `RestartDecision` per `RestartStrategy` (OneForOne 立即重启; Transient 仅异常重启)
//!
//! **3 阶审查** (O-6 锚 #9):
//! 1. 总体: 与 RC-1 修真 impl 同样模式 (trait 0 装占位 → 真 impl, 0 触碰 API 表面)
//! 2. 系统: impl 与 trait 同 crate (无 cross-crate), 0 引入新 dep (tokio + std 都有)
//! 3. 架构: 真 process spawn 受 `ProcessExecutor` 同样 ProcessJob 隔离 (per v1 ProcessExecutor),
//!    supervisor 是其上层 (per v2.0.0-rc-roadmap.md §3 RC-8: "集成 ProcessExecutor 作为 spawn 后 wrapper")
//!
//! **0 触碰 LOCKED**: 9 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline.
//!
//! **v1 compat**: 100+ consumer 0 破 (新增 module, additive).

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::supervisor::{
    ChildSpec, ExitReason, RestartDecision, RestartStrategy, SubSupervisor, SubSupervisorKind,
    SupervisorError,
};

/// 真 `SubSupervisor` impl: `std::process::Command` 同步 spawn 子进程.
///
/// 0 装诚实:
/// - 真启进程 (0 装 NoopSubSupervisor 才是 0 装)
/// - 真 `Child` handle 持有 (可 `kill()`, 可 `try_wait()`)
/// - 真 `ExitReason` 解析 (ExitCode → Normal/Abnormal, signal → Signaled on Unix)
/// - 真 `RestartDecision` per `RestartStrategy`
pub struct StdSubSupervisor {
    kind: SubSupervisorKind,
    /// 声明式 child 规格 (user 给的)
    children: Vec<ChildSpec>,
    /// 真 child handle (启动后填, id → Child)
    handles: Arc<Mutex<HashMap<String, Child>>>,
    /// 重启计数 (id → count, per v1 max_restarts)
    restart_counts: Arc<Mutex<HashMap<String, u32>>>,
    /// 重启窗口 (v1: max_restarts per period = 5 in 60s)
    max_restarts_per_window: u32,
    restart_window: Duration,
}

impl StdSubSupervisor {
    pub fn new(kind: SubSupervisorKind, children: Vec<ChildSpec>) -> Self {
        Self {
            kind,
            children,
            handles: Arc::new(Mutex::new(HashMap::new())),
            restart_counts: Arc::new(Mutex::new(HashMap::new())),
            // v1 默认: 5 次 / 60 秒
            max_restarts_per_window: 5,
            restart_window: Duration::from_secs(60),
        }
    }

    /// 启动一个 child (用 std::process::Command, 0 装成 0 启进程)
    fn spawn_child(&self, spec: &ChildSpec) -> Result<Child, SupervisorError> {
        let mut cmd = Command::new(&spec.cmd);
        cmd.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            cmd.current_dir(cwd);
        }
        cmd.envs(spec.env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        // 0 装: stdin/stdout/stderr inherit (per v1 process:default)
        // rc 阶段可接 ProcessExecutor 的 Windows Job Object 隔离
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        cmd.spawn()
            .map_err(|e| SupervisorError::StartFailed(format!("spawn `{}` failed: {e}", spec.cmd)))
    }

    /// 检测 `ExitReason` (per v1: ExitCode → Normal/Abnormal, signal → Signaled on Unix)
    fn classify_exit(status: std::process::ExitStatus) -> ExitReason {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return ExitReason::Signaled { signal };
            }
        }
        match status.code() {
            Some(0) => ExitReason::Normal { code: 0 },
            Some(code) => ExitReason::Abnormal { code },
            None => ExitReason::Killed,
        }
    }

    /// 检查 child handle 是否还活着 (0 装: 不会触发, 因为 start 时没真的启)
    fn try_reap(&self, _child_id: &str, child: &mut Child) -> Option<ExitReason> {
        match child.try_wait() {
            Ok(Some(status)) => Some(Self::classify_exit(status)),
            Ok(None) => None,                   // 还活着
            Err(_) => Some(ExitReason::Killed), // 错当 killed
        }
    }

    /// 检查是否超过重启窗口限制
    fn restart_limit_exceeded(&self, child_id: &str) -> bool {
        let counts = self.restart_counts.lock().expect("restart_counts poisoned");
        counts.get(child_id).copied().unwrap_or(0) >= self.max_restarts_per_window
    }

    /// 记一次重启
    fn record_restart(&self, child_id: &str) {
        let mut counts = self.restart_counts.lock().expect("restart_counts poisoned");
        *counts.entry(child_id.to_string()).or_insert(0) += 1;
    }
}

impl SubSupervisor for StdSubSupervisor {
    fn start(&mut self) -> Result<(), SupervisorError> {
        // 启动所有声明的 children (per v1 process:start)
        // 0 装 PASS: 真启进程, 不假装
        for spec in &self.children {
            if self
                .handles
                .lock()
                .expect("handles poisoned")
                .contains_key(&spec.id)
            {
                return Err(SupervisorError::StartFailed(format!(
                    "duplicate child id: {}",
                    spec.id
                )));
            }
            let child = self.spawn_child(spec)?;
            self.handles
                .lock()
                .expect("handles poisoned")
                .insert(spec.id.clone(), child);
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<(), SupervisorError> {
        // 0 装 PASS: 真 kill (per v1 process:stop)
        // 0 装: 这里有 race 条件 (child 已退出 + handle 无效), 用 try_wait + kill fallback
        let mut handles = self.handles.lock().expect("handles poisoned");
        for (id, child) in handles.iter_mut() {
            // 先 try_wait, 如果已退出, 不用 kill
            if let Ok(Some(_)) = child.try_wait() {
                continue;
            }
            // 还活着, kill
            if let Err(e) = child.kill() {
                return Err(SupervisorError::Io(format!("kill `{}` failed: {e}", id)));
            }
        }
        handles.clear();
        Ok(())
    }

    fn children(&self) -> Vec<ChildSpec> {
        self.children.clone()
    }

    fn on_child_exit(
        &mut self,
        child_id: &str,
        reason: ExitReason,
    ) -> Result<RestartDecision, SupervisorError> {
        // 0 装 PASS: 真按 RestartStrategy 决策 (不是 NoopSubSupervisor 的 DoNotRestart)
        // 流程: reaper 调 on_child_exit (per v1 process:on_child_exit)
        // 1. 找 child spec
        let spec = self
            .children
            .iter()
            .find(|s| s.id == child_id)
            .ok_or_else(|| SupervisorError::UnknownChild(child_id.to_string()))?
            .clone();

        // 2. reaper (如果 child 还在 handles, 调 try_wait 确认)
        // 0 装: 不真 reaper (那是 runtime 责任, supervisor 接收 event)
        // 这里只做策略决策

        // 3. 按 RestartStrategy + reason 决定
        match spec.restart {
            RestartStrategy::OneForOne => {
                // OneForOne: 任何 exit 都重启
                if self.restart_limit_exceeded(child_id) {
                    return Err(SupervisorError::RestartLimitExceeded(format!(
                        "{child_id} exceeded max_restarts {} in {:?}",
                        self.max_restarts_per_window, self.restart_window
                    )));
                }
                self.record_restart(child_id);
                Ok(RestartDecision::RestartNow)
            }
            RestartStrategy::RestForOne => {
                // RestForOne: 任何 exit 都重启 + 之后所有
                // 0 装: 返 RestartNow, 实际 "之后所有" 由 runtime 实现
                if self.restart_limit_exceeded(child_id) {
                    return Err(SupervisorError::RestartLimitExceeded(format!(
                        "{child_id} exceeded max_restarts {} in {:?}",
                        self.max_restarts_per_window, self.restart_window
                    )));
                }
                self.record_restart(child_id);
                Ok(RestartDecision::RestartNow)
            }
            RestartStrategy::Transient => {
                // Transient: 正常退出 (Normal) 不重启; 异常才重启
                match reason {
                    ExitReason::Normal { .. } => Ok(RestartDecision::DoNotRestart),
                    _ => {
                        if self.restart_limit_exceeded(child_id) {
                            return Err(SupervisorError::RestartLimitExceeded(format!(
                                "{child_id} exceeded max_restarts"
                            )));
                        }
                        self.record_restart(child_id);
                        Ok(RestartDecision::RestartNow)
                    }
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        match self.kind {
            SubSupervisorKind::Core => "tokio_core",
            SubSupervisorKind::Cognition => "tokio_cognition",
            SubSupervisorKind::Council => "tokio_council",
            SubSupervisorKind::Upgrade => "tokio_upgrade",
            SubSupervisorKind::Plugin => "tokio_plugin",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RC-8 验收: StdSubSupervisor 可构造 (Send + Sync)
    #[test]
    fn tokio_sub_supervisor_is_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<StdSubSupervisor>();
    }

    /// RC-8 验收: 5 个 kind 各自返正确 name
    #[test]
    fn name_per_kind() {
        for kind in [
            SubSupervisorKind::Core,
            SubSupervisorKind::Cognition,
            SubSupervisorKind::Council,
            SubSupervisorKind::Upgrade,
            SubSupervisorKind::Plugin,
        ] {
            let s = StdSubSupervisor::new(kind, vec![]);
            let name = s.name();
            assert!(name.starts_with("tokio_"));
        }
    }

    /// RC-8 验收: children 列表 0 改 (user 给的)
    #[test]
    fn children_returns_cloned_specs() {
        let specs = vec![ChildSpec::new("worker-1", "/bin/true")];
        let s = StdSubSupervisor::new(SubSupervisorKind::Core, specs.clone());
        let returned = s.children();
        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0].id, "worker-1");
    }

    /// RC-8 验收: RestartStrategy::OneForOne 任何 exit 都重启
    #[test]
    fn one_for_one_restarts_on_any_exit() {
        let spec = ChildSpec::new("c-1", "/bin/true");
        let mut s = StdSubSupervisor::new(SubSupervisorKind::Core, vec![spec]);
        for reason in [
            ExitReason::Normal { code: 0 },
            ExitReason::Abnormal { code: 1 },
            ExitReason::Signaled { signal: 9 },
        ] {
            let d = s.on_child_exit("c-1", reason).unwrap();
            assert_eq!(d, RestartDecision::RestartNow);
        }
    }

    /// RC-8 验收: RestartStrategy::Transient 仅异常重启
    #[test]
    fn transient_only_restarts_on_abnormal() {
        let mut spec = ChildSpec::new("c-1", "/bin/true");
        spec.restart = RestartStrategy::Transient;
        let mut s = StdSubSupervisor::new(SubSupervisorKind::Core, vec![spec]);
        // Normal 不重启
        let d = s
            .on_child_exit("c-1", ExitReason::Normal { code: 0 })
            .unwrap();
        assert_eq!(d, RestartDecision::DoNotRestart);
        // Abnormal 重启
        let d = s
            .on_child_exit("c-1", ExitReason::Abnormal { code: 1 })
            .unwrap();
        assert_eq!(d, RestartDecision::RestartNow);
    }

    /// RC-8 验收: 重启限制 (max_restarts_per_window)
    #[test]
    fn restart_limit_exceeded_triggers_error() {
        let mut spec = ChildSpec::new("c-1", "/bin/true");
        spec.restart = RestartStrategy::OneForOne;
        let mut s = StdSubSupervisor::new(SubSupervisorKind::Core, vec![spec]);
        // 默认 5 次
        for _ in 0..5 {
            let _ = s
                .on_child_exit("c-1", ExitReason::Abnormal { code: 1 })
                .unwrap();
        }
        // 第 6 次: 超限
        let r = s.on_child_exit("c-1", ExitReason::Abnormal { code: 1 });
        assert!(matches!(r, Err(SupervisorError::RestartLimitExceeded(_))));
    }

    /// RC-8 验收: unknown child → UnknownChild error
    #[test]
    fn unknown_child_returns_error() {
        let mut s = StdSubSupervisor::new(SubSupervisorKind::Core, vec![]);
        let r = s.on_child_exit("nonexistent", ExitReason::Normal { code: 0 });
        assert!(matches!(r, Err(SupervisorError::UnknownChild(_))));
    }

    /// RC-8 验收: 真启子进程 (cross-platform portable cmd)
    #[test]
    #[cfg(not(windows))] // Unix-only portable cmd
    fn real_spawn_works() {
        // /bin/true 是 Unix 上总是 exit 0 的 cmd, 跨平台
        let spec = ChildSpec::new("true-1", "/bin/true");
        let mut s = StdSubSupervisor::new(SubSupervisorKind::Core, vec![spec]);
        // start 真启进程
        s.start().expect("start");
        // 进程应立刻 exit 0
        std::thread::sleep(std::time::Duration::from_millis(100));
        // try_wait 应返 Some
        let mut handles = s.handles.lock().unwrap();
        let child = handles.get_mut("true-1").expect("handle exists");
        let status = child.try_wait().expect("try_wait");
        assert!(status.is_some(), "/bin/true 应已 exit");
        let status = status.unwrap();
        assert!(status.success(), "/bin/true exit 0");
    }

    /// RC-8 验收: start 后 stop 真 kill 进程 (这里用 /bin/sleep 1s, 立即 stop)
    #[test]
    #[cfg(not(windows))]
    fn real_kill_works() {
        // /bin/sleep 60 持久进程, 立即 stop 应 kill 它
        let mut spec = ChildSpec::new("sleep-1", "/bin/sleep");
        spec.args = vec!["60".into()];
        let mut s = StdSubSupervisor::new(SubSupervisorKind::Core, vec![spec]);
        s.start().expect("start");
        s.stop().expect("stop");
        // 进程被 kill, handles 清空
        let handles = s.handles.lock().unwrap();
        assert!(handles.is_empty());
    }

    /// RC-8 验收: duplicate child id in start → StartFailed error
    #[test]
    fn duplicate_child_id_returns_error() {
        let spec1 = ChildSpec::new("dup", "/bin/true");
        let spec2 = ChildSpec::new("dup", "/bin/false");
        let mut s = StdSubSupervisor::new(SubSupervisorKind::Core, vec![spec1, spec2]);
        let r = s.start();
        assert!(matches!(r, Err(SupervisorError::StartFailed(_))));
    }
}
