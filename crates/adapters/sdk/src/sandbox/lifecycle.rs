//! # Sandbox lifecycle — 生命周期状态机
//!
//! 六态状态机: `pending → creating → running → (stopping → stopped) | failed`。
//! 本模块是**迁移矩阵的唯一权威**: 客户端每一次状态读取/写入 (spawn 建档、
//! 巡检对账、kill/cleanup 落终态) 都必须经 [`SandboxStatus::can_transition_to`]
//! / [`apply_transition`] 校验; 非法迁移收口成
//! [`SandboxError::InvalidState`](crate::sandbox::SandboxError::InvalidState)。
//!
//! 终态 (`stopped` / `failed`) 吸收: 不接受任何后续迁移。

use serde::{Deserialize, Serialize};

use crate::sandbox::error::{SandboxError, SandboxResult};
use crate::sandbox::runtime::SandboxStatus;

/// 生命周期驱动事件 (客户端动作 / 服务端回报统一归一到这 6 类)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEvent {
    /// 开始创建 (资源预留、镜像准备)。
    BeginCreate,
    /// 创建完成, 进入运行。
    FinishCreate,
    /// 请求停止 (graceful)。
    RequestStop,
    /// 运行结束 (正常退出)。
    Finish,
    /// 失败 (创建失败 / 运行中错误 / 停止失败)。
    Fail,
    /// 已清理 (终态回收, 不改状态, 仅从登记表移除)。
    Reap,
}

impl LifecycleEvent {
    /// 稳定标签 (日志 / 报告)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::BeginCreate => "begin_create",
            Self::FinishCreate => "finish_create",
            Self::RequestStop => "request_stop",
            Self::Finish => "finish",
            Self::Fail => "fail",
            Self::Reap => "reap",
        }
    }
}

impl SandboxStatus {
    /// 是否终态 (吸收态: 不接受任何后续迁移)。
    pub fn is_terminal(self) -> bool {
        matches!(self, SandboxStatus::Stopped | SandboxStatus::Failed)
    }

    /// 迁移矩阵: `self → next` 是否合法。
    ///
    /// 合法边 (8 条):
    /// `pending→creating` `pending→failed`
    /// `creating→running` `creating→failed`
    /// `running→stopping` `running→stopped` `running→failed`
    /// `stopping→stopped` `stopping→failed`
    /// 自迁移恒非法 (状态机不接受 no-op 迁移, 防止"重放即合法")。
    pub fn can_transition_to(self, next: SandboxStatus) -> bool {
        use SandboxStatus as S;
        matches!(
            (self, next),
            (S::Pending, S::Creating)
                | (S::Pending, S::Failed)
                | (S::Creating, S::Running)
                | (S::Creating, S::Failed)
                | (S::Running, S::Stopping)
                | (S::Running, S::Stopped)
                | (S::Running, S::Failed)
                | (S::Stopping, S::Stopped)
                | (S::Stopping, S::Failed)
        )
    }

    /// 事件驱动的后继状态 (确定性: 每个 (状态, 事件) 至多一个后继)。
    pub fn after(self, event: LifecycleEvent) -> SandboxResult<SandboxStatus> {
        use LifecycleEvent as E;
        use SandboxStatus as S;
        match (self, event) {
            (S::Pending, E::BeginCreate) => Ok(S::Creating),
            (S::Pending, E::Fail) => Ok(S::Failed),
            (S::Creating, E::FinishCreate) => Ok(S::Running),
            (S::Creating, E::Fail) => Ok(S::Failed),
            (S::Running, E::RequestStop) => Ok(S::Stopping),
            (S::Running, E::Finish) => Ok(S::Stopped),
            (S::Running, E::Fail) => Ok(S::Failed),
            (S::Stopping, E::Finish) => Ok(S::Stopped),
            (S::Stopping, E::Fail) => Ok(S::Failed),
            // 终态只接受 Reap (由 apply_transition 单独处理)。
            (state, E::Reap) if state.is_terminal() => Ok(state),
            (state, event) => Err(SandboxError::InvalidState(format!(
                "event `{}` not applicable in state `{}`",
                event.label(),
                state
            ))),
        }
    }
}

/// 把事件应用到当前状态: 先算后继, 再走迁移矩阵校验, 双保险。
pub fn apply_transition(
    current: SandboxStatus,
    event: LifecycleEvent,
) -> SandboxResult<SandboxStatus> {
    let next = current.after(event)?;
    if next == current {
        return Ok(current);
    }
    if !current.can_transition_to(next) {
        return Err(SandboxError::InvalidState(format!(
            "illegal transition {current} -> {next}"
        )));
    }
    Ok(next)
}

/// 矩阵可达性 (传递闭包): `from` 能否经若干合法边到达 `to`。
pub fn is_reachable(from: SandboxStatus, to: SandboxStatus) -> bool {
    if from == to {
        return true;
    }
    let all = [
        SandboxStatus::Pending,
        SandboxStatus::Creating,
        SandboxStatus::Running,
        SandboxStatus::Stopping,
        SandboxStatus::Stopped,
        SandboxStatus::Failed,
    ];
    let mut frontier = vec![from];
    let mut seen = vec![from];
    while let Some(state) = frontier.pop() {
        for next in all {
            if next != state && state.can_transition_to(next) {
                if next == to {
                    return true;
                }
                if !seen.contains(&next) {
                    seen.push(next);
                    frontier.push(next);
                }
            }
        }
    }
    false
}

/// 把服务端回报的状态并入本地状态: 只接受矩阵可达的前向迁移; 同态幂等接受;
/// 不可达 (后退 / 矩阵外跳变) 一律 [`SandboxError::InvalidState`]。
pub fn reconcile(local: SandboxStatus, reported: SandboxStatus) -> SandboxResult<SandboxStatus> {
    if local == reported {
        return Ok(local);
    }
    if is_reachable(local, reported) {
        return Ok(reported);
    }
    Err(SandboxError::InvalidState(format!(
        "server reported `{reported}` but local state `{local}` cannot reach it"
    )))
}

/// 生命周期登记项的进度序 (用于巡检报告排序 / 单调性检查)。
pub fn status_rank(status: SandboxStatus) -> u8 {
    match status {
        SandboxStatus::Pending => 0,
        SandboxStatus::Creating => 1,
        SandboxStatus::Running => 2,
        SandboxStatus::Stopping => 3,
        SandboxStatus::Stopped => 4,
        SandboxStatus::Failed => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SandboxStatus as S;

    /// 生命周期全迁移: 6×6 全矩阵逐格断言, 非法格全部拒绝。
    #[test]
    fn full_transition_matrix_is_exactly_the_documented_edges() {
        let legal = [
            (S::Pending, S::Creating),
            (S::Pending, S::Failed),
            (S::Creating, S::Running),
            (S::Creating, S::Failed),
            (S::Running, S::Stopping),
            (S::Running, S::Stopped),
            (S::Running, S::Failed),
            (S::Stopping, S::Stopped),
            (S::Stopping, S::Failed),
        ];
        let all = [
            S::Pending,
            S::Creating,
            S::Running,
            S::Stopping,
            S::Stopped,
            S::Failed,
        ];
        for from in all {
            for to in all {
                let expected = legal.contains(&(from, to));
                assert_eq!(
                    from.can_transition_to(to),
                    expected,
                    "matrix cell {from} -> {to} must be {expected}"
                );
            }
        }
    }

    /// 终态吸收: stopped / failed 不接受任何迁移。
    #[test]
    fn terminal_states_absorb_all_events() {
        for terminal in [S::Stopped, S::Failed] {
            assert!(terminal.is_terminal());
            for event in [
                LifecycleEvent::BeginCreate,
                LifecycleEvent::FinishCreate,
                LifecycleEvent::RequestStop,
                LifecycleEvent::Finish,
                LifecycleEvent::Fail,
            ] {
                assert!(
                    terminal.after(event).is_err(),
                    "{terminal} must reject {event:?}"
                );
            }
            // Reap 是唯一允许作用在终态上的事件, 且不改状态。
            assert_eq!(terminal.after(LifecycleEvent::Reap).unwrap(), terminal);
        }
    }

    /// 事件驱动全链: pending → creating → running → stopping → stopped。
    #[test]
    fn event_driven_full_chain_walks_every_stage() {
        let mut state = S::Pending;
        for event in [
            LifecycleEvent::BeginCreate,
            LifecycleEvent::FinishCreate,
            LifecycleEvent::RequestStop,
            LifecycleEvent::Finish,
        ] {
            state = apply_transition(state, event).expect("legal chain");
        }
        assert_eq!(state, S::Stopped);
    }

    /// 非法事件 / 非法迁移都收口成 InvalidState。
    #[test]
    fn illegal_events_and_transitions_close_to_invalid_state() {
        let err = apply_transition(S::Pending, LifecycleEvent::RequestStop).unwrap_err();
        assert!(matches!(err, SandboxError::InvalidState(_)));
        let err = apply_transition(S::Running, LifecycleEvent::BeginCreate).unwrap_err();
        assert!(matches!(err, SandboxError::InvalidState(_)));
        assert!(!S::Stopped.can_transition_to(S::Creating));
    }

    /// reconcile: 幂等接受同态, 前向接受, 回退拒绝。
    #[test]
    fn reconcile_accepts_forward_and_rejects_backward_drift() {
        assert_eq!(reconcile(S::Running, S::Running).unwrap(), S::Running);
        assert_eq!(reconcile(S::Creating, S::Running).unwrap(), S::Running);
        assert_eq!(reconcile(S::Running, S::Failed).unwrap(), S::Failed);
        assert!(matches!(
            reconcile(S::Stopped, S::Running),
            Err(SandboxError::InvalidState(_))
        ));
        assert!(matches!(
            reconcile(S::Running, S::Pending),
            Err(SandboxError::InvalidState(_))
        ));
    }

    /// 状态序单调, 供巡检报告用。
    #[test]
    fn status_rank_orders_the_pipeline() {
        assert!(status_rank(S::Pending) < status_rank(S::Creating));
        assert!(status_rank(S::Creating) < status_rank(S::Running));
        assert!(status_rank(S::Running) < status_rank(S::Stopping));
        assert!(status_rank(S::Stopping) < status_rank(S::Stopped));
        assert!(status_rank(S::Stopped) < status_rank(S::Failed));
    }
}
