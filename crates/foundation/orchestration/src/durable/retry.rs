//! `durable::retry` — 重试策略元数据 + 活动守门状态机.
//!
//! **语义来源**:
//! - frozen/apeireth-task `RetryPolicy` 硬编码常量 (max_retries=3, backoff 基数 1s,
//!   指数 1s→2s→4s) 与 `TaskStateMachine` 7 状态守门 (Pending/Queued/Running/...,
//!   终态锁定, Failed/Timeout 可重入, Cancelled 不可重试)。
//! - canonical/apeireth-pipeline-g5 `DefaultReliability::backoff_ms` (attempt→退避查询,
//!   不在原语内 sleep, 异步 sleep 由上层拥有)。
//! - v2 已有 [`crate::worktree_sandbox::RateLimitBackoff`] 是限流场景的实例化退避;
//!   本模块的 [`RetryPolicy`] 补齐"尝试预算 + 元数据落账"维度, 两者互补不重复。
//!
//! **确定性**: `backoff_ms` 是纯函数 (无随机抖动); 尝试号作为事件元数据写入
//! [`super::ActivityEvent::attempt`], 构成可重放的 failure/retry 账目。

use serde::{Deserialize, Serialize};

use super::history::RUN_EVENT_ACTOR;

/// 重试策略: 尝试预算 + 确定性指数退避 (canonical frozen-task 常量为默认值)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// 最大尝试次数 (含首次; canonical `MAX_RETRIES_DEFAULT = 3`)。
    pub max_attempts: u32,
    /// 退避基数毫秒 (第 1 次重试前; canonical `RETRY_BACKOFF_MS = 1000`)。
    pub base_backoff_ms: u64,
    /// 退避上限毫秒 (指数封顶)。
    pub max_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_backoff_ms: 1_000,
            max_backoff_ms: 30_000,
        }
    }
}

impl RetryPolicy {
    /// 构造策略; `max_attempts` 下限为 1 (0 视为 1, 防御性钳制)。
    pub fn new(max_attempts: u32, base_backoff_ms: u64, max_backoff_ms: u64) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            base_backoff_ms,
            max_backoff_ms,
        }
    }

    /// 第 `attempt` 次尝试失败后是否还应重试 (attempt 1-based)。
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }

    /// 第 `retry_index` 次重试前的退避毫秒数 (1-based: 1 = 第一次重试)。
    ///
    /// 纯确定性指数退避: `base * 2^(retry_index - 1)`, 以 `max_backoff_ms` 封顶;
    /// `retry_index == 0` 返回 0。无随机抖动 — 持久重放要求退避可推导。
    pub fn backoff_ms(&self, retry_index: u32) -> u64 {
        if retry_index == 0 {
            return 0;
        }
        let shift = (retry_index - 1).min(62);
        let factor = 1u64 << shift;
        self.base_backoff_ms
            .saturating_mul(factor)
            .min(self.max_backoff_ms)
    }
}

/// 持久活动调用的状态 (frozen task 7 状态适配到活动粒度)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityState {
    /// 已创建, 未调度 (canonical `Pending`)
    Pending,
    /// 已写入调度事件 (canonical `Queued`)
    Scheduled,
    /// 执行器在途 (canonical `Running`)
    Running,
    /// 成功 (终态; canonical `Completed`)
    Succeeded,
    /// 失败 (可重试; canonical `Failed`)
    Failed,
    /// 取消 (终态, 永不可重试; canonical `Cancelled`)
    Cancelled,
    /// 超时 (可重试; canonical `Timeout`)
    TimedOut,
}

/// 状态数编译期守门 (canonical `TASK_STATE_COUNT` 模式)。
pub const ACTIVITY_STATE_COUNT: usize = 7;

/// 全部 7 个状态 (canonical `SUPPORTED_STATES` 模式)。
pub const SUPPORTED_ACTIVITY_STATES: &[ActivityState] = &[
    ActivityState::Pending,
    ActivityState::Scheduled,
    ActivityState::Running,
    ActivityState::Succeeded,
    ActivityState::Failed,
    ActivityState::Cancelled,
    ActivityState::TimedOut,
];

const _: () = assert!(SUPPORTED_ACTIVITY_STATES.len() == ACTIVITY_STATE_COUNT);

/// 活动状态机守门错误 (canonical `TaskError::InvalidTransition` 模式)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityStateError {
    /// 非法状态转换。
    InvalidTransition {
        /// 当前状态
        from: ActivityState,
        /// 目标状态
        to: ActivityState,
    },
    /// 重试超出预算 (canonical `RetryPolicy.max_retries` 守门)。
    RetryExhausted {
        /// 活动 ID
        activity_id: String,
        /// 已用尝试数
        attempt: u32,
        /// 预算上限
        max_attempts: u32,
    },
}

impl std::fmt::Display for ActivityStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid activity state transition: {from:?} -> {to:?}")
            }
            Self::RetryExhausted {
                activity_id,
                attempt,
                max_attempts,
            } => write!(
                f,
                "activity {activity_id:?} retry budget exhausted: attempt {attempt} >= max {max_attempts}"
            ),
        }
    }
}

impl std::error::Error for ActivityStateError {}

/// 活动调用守门状态机 (canonical `TaskStateMachine` 适配)。
///
/// 转换表 (canonical 状态图 1:1, Queued→Scheduled 改名):
///
/// ```text
/// Pending  ──schedule──▶  Scheduled ──dispatch──▶ Running
///    │                       │                      │
///    │ cancel                │ cancel        ┌──────┼──────────┐
///    ▼                       ▼               ▼      ▼          ▼
/// Cancelled              Cancelled      Succeeded Failed  TimedOut
///                                           │  └──retry(政策内)──▶ Scheduled
///                                           └─────retry──────────▶ Scheduled
/// ```
///
/// **不变量** (不可妥协, canonical O-1 教训):
/// - 终态 (`Succeeded` / `Failed` / `Cancelled` / `TimedOut`) 除重试通道外不可再转换;
/// - `Cancelled` 永不可重试 (主动取消不是失败);
/// - `Pending` 不得跳过 `Scheduled` 直达 `Running`;
/// - 离开 `Running` 时按注入时间累计运行时长。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityStateMachine {
    /// 活动 ID
    pub activity_id: String,
    /// 当前状态
    pub state: ActivityState,
    /// 已开始的尝试数 (Pending 为 0; 首次 `Scheduled` 置 1; 每次 `retry` 再 +1)
    pub attempt: u32,
    /// 进入当前状态的时间 (注入毫秒)
    pub state_changed_at_ms: i64,
    /// `Running` 期间累计运行毫秒
    pub accumulated_runtime_ms: u64,
}

impl ActivityStateMachine {
    /// 新建: 初始 `Pending`, attempt 0。
    pub fn new(activity_id: impl Into<String>, now_ms: i64) -> Self {
        Self {
            activity_id: activity_id.into(),
            state: ActivityState::Pending,
            attempt: 0,
            state_changed_at_ms: now_ms,
            accumulated_runtime_ms: 0,
        }
    }

    /// 状态转换守门 (不含重试通道; 重试走 [`Self::retry`] 以接入预算检查)。
    pub fn transition(
        &mut self,
        next: ActivityState,
        now_ms: i64,
    ) -> Result<(), ActivityStateError> {
        if !self.can_transition(next) {
            return Err(ActivityStateError::InvalidTransition {
                from: self.state,
                to: next,
            });
        }
        self.apply(next, now_ms);
        Ok(())
    }

    /// 重试通道: 仅 `Failed` / `TimedOut` 可重入 `Scheduled`, 且受预算守门;
    /// `Cancelled` 永不可重试。
    pub fn retry(&mut self, policy: &RetryPolicy, now_ms: i64) -> Result<(), ActivityStateError> {
        match self.state {
            ActivityState::Failed | ActivityState::TimedOut => {}
            other => {
                return Err(ActivityStateError::InvalidTransition {
                    from: other,
                    to: ActivityState::Scheduled,
                })
            }
        }
        if !policy.should_retry(self.attempt) {
            return Err(ActivityStateError::RetryExhausted {
                activity_id: self.activity_id.clone(),
                attempt: self.attempt,
                max_attempts: policy.max_attempts,
            });
        }
        self.attempt += 1;
        self.apply(ActivityState::Scheduled, now_ms);
        Ok(())
    }

    /// 转换是否合法 (不修改状态; canonical `can_transition`)。
    pub fn can_transition(&self, next: ActivityState) -> bool {
        match (self.state, next) {
            (ActivityState::Pending, ActivityState::Scheduled)
            | (ActivityState::Pending, ActivityState::Cancelled) => true,
            (ActivityState::Scheduled, ActivityState::Running)
            | (ActivityState::Scheduled, ActivityState::Cancelled) => true,
            (ActivityState::Running, ActivityState::Succeeded)
            | (ActivityState::Running, ActivityState::Failed)
            | (ActivityState::Running, ActivityState::TimedOut)
            | (ActivityState::Running, ActivityState::Cancelled) => true,
            // 重试例外由 retry() 守门; can_transition 对终态一律拒绝
            (
                ActivityState::Pending
                | ActivityState::Scheduled
                | ActivityState::Running
                | ActivityState::Succeeded
                | ActivityState::Failed
                | ActivityState::Cancelled
                | ActivityState::TimedOut,
                _,
            ) => false,
        }
    }

    /// 是否终态 (canonical `is_terminal`; Failed/TimedOut 属终态, 仅保留重试通道)。
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            ActivityState::Succeeded
                | ActivityState::Failed
                | ActivityState::Cancelled
                | ActivityState::TimedOut
        )
    }

    fn apply(&mut self, next: ActivityState, now_ms: i64) {
        if self.state == ActivityState::Running {
            let elapsed = (now_ms - self.state_changed_at_ms).max(0) as u64;
            self.accumulated_runtime_ms += elapsed;
        }
        // First schedule is attempt 1; subsequent re-entry goes through `retry()`
        // which increments before applying Scheduled. Events and RetryPolicy are
        // 1-based (canonical max_attempts=3 means 3 executions, not 3 retries on top
        // of a 0-based first try).
        if self.state == ActivityState::Pending
            && next == ActivityState::Scheduled
            && self.attempt == 0
        {
            self.attempt = 1;
        }
        self.state = next;
        self.state_changed_at_ms = now_ms;
    }
}

// RUN_EVENT_ACTOR 引用守门: run 级事件不参与活动状态机 (编译期触达, 防误删常量)。
const _: &str = RUN_EVENT_ACTOR;

#[cfg(test)]
mod tests {
    use super::*;

    // ====== RetryPolicy ======

    /// canonical frozen-task 常量: max_retries=3, backoff 基数 1000ms。
    #[test]
    fn defaults_match_donor_constants() {
        let p = RetryPolicy::default();
        assert_eq!(p.max_attempts, 3);
        assert_eq!(p.base_backoff_ms, 1_000);
    }

    /// canonical g5 语义: attempt 计数守门, 超预算拒绝。
    #[test]
    fn should_retry_boundaries() {
        let p = RetryPolicy::new(3, 100, 1_000);
        assert!(p.should_retry(1));
        assert!(p.should_retry(2));
        assert!(!p.should_retry(3));
        assert!(!p.should_retry(4));
        // max_attempts=1 → 永不重试
        let p1 = RetryPolicy::new(1, 100, 1_000);
        assert!(!p1.should_retry(1));
        // 0 被钳制为 1
        assert_eq!(RetryPolicy::new(0, 1, 1).max_attempts, 1);
    }

    /// canonical frozen-task 指数退避 1s→2s→4s, 封顶生效, 零重试为 0。
    #[test]
    fn backoff_is_exponential_and_capped() {
        let p = RetryPolicy::new(8, 1_000, 30_000);
        assert_eq!(p.backoff_ms(0), 0);
        assert_eq!(p.backoff_ms(1), 1_000);
        assert_eq!(p.backoff_ms(2), 2_000);
        assert_eq!(p.backoff_ms(3), 4_000);
        assert_eq!(p.backoff_ms(4), 8_000);
        assert_eq!(p.backoff_ms(5), 16_000);
        assert_eq!(p.backoff_ms(6), 30_000, "封顶");
        assert_eq!(p.backoff_ms(20), 30_000, "高位饱和不溢出");
        // g5 风格小步表: base 100
        let small = RetryPolicy::new(10, 100, 1_000);
        assert_eq!(small.backoff_ms(1), 100);
        assert_eq!(small.backoff_ms(4), 800);
        assert_eq!(small.backoff_ms(5), 1_000);
    }

    // ====== ActivityStateMachine (canonical frozen-task 移植测试) ======

    /// canonical `test_seven_states_hardcoded`。
    #[test]
    fn seven_states_hardcoded() {
        assert_eq!(ACTIVITY_STATE_COUNT, 7);
        assert_eq!(SUPPORTED_ACTIVITY_STATES.len(), 7);
    }

    /// canonical `test_state_machine_happy_path`。
    #[test]
    fn happy_path_reaches_terminal() {
        let mut sm = ActivityStateMachine::new("t1", 1000);
        sm.transition(ActivityState::Scheduled, 1001).unwrap();
        sm.transition(ActivityState::Running, 1002).unwrap();
        sm.transition(ActivityState::Succeeded, 1003).unwrap();
        assert!(sm.is_terminal());
        assert_eq!(
            sm.accumulated_runtime_ms, 1,
            "离开 Running 时累计 1003-1002"
        );
    }

    /// canonical `test_terminal_blocks_further_transitions`。
    #[test]
    fn terminal_blocks_further_transitions() {
        let mut sm = ActivityStateMachine::new("t2", 1000);
        sm.transition(ActivityState::Scheduled, 1001).unwrap();
        sm.transition(ActivityState::Running, 1002).unwrap();
        sm.transition(ActivityState::Failed, 1003).unwrap();
        assert!(sm.is_terminal());
        // Failed 除重试通道外不可再转 Running
        assert!(sm.transition(ActivityState::Running, 1004).is_err());
    }

    /// canonical `test_pending_to_cancelled_direct`。
    #[test]
    fn pending_to_cancelled_direct() {
        let mut sm = ActivityStateMachine::new("t3", 1000);
        sm.transition(ActivityState::Cancelled, 1001).unwrap();
        assert!(sm.is_terminal());
    }

    /// canonical `test_invalid_skip_queue_to_running`: Pending 不得跳过 Scheduled。
    #[test]
    fn invalid_skip_scheduled_to_running() {
        let mut sm = ActivityStateMachine::new("t4", 1000);
        assert!(matches!(
            sm.transition(ActivityState::Running, 1001),
            Err(ActivityStateError::InvalidTransition { .. })
        ));
    }

    /// 重试通道: Failed/TimedOut 政策内可重入 Scheduled, Cancelled 永不可。
    #[test]
    fn retry_channel_semantics() {
        let policy = RetryPolicy::new(3, 100, 1_000);
        let mut sm = ActivityStateMachine::new("t5", 1000);
        sm.transition(ActivityState::Scheduled, 1001).unwrap();
        assert_eq!(sm.attempt, 1, "first schedule is attempt 1");
        sm.transition(ActivityState::Running, 1002).unwrap();
        sm.transition(ActivityState::Failed, 1003).unwrap();
        sm.retry(&policy, 1004).unwrap();
        assert_eq!(sm.state, ActivityState::Scheduled);
        assert_eq!(sm.attempt, 2);
        sm.transition(ActivityState::Running, 1005).unwrap();
        sm.transition(ActivityState::TimedOut, 1006).unwrap();
        sm.retry(&policy, 1007).unwrap();
        assert_eq!(sm.attempt, 3);

        // 预算 3 次尝试: 第 3 次失败后 should_retry(3) 为 false, 耗尽
        sm.transition(ActivityState::Running, 1008).unwrap();
        sm.transition(ActivityState::Failed, 1009).unwrap();
        assert!(matches!(
            sm.retry(&policy, 1010),
            Err(ActivityStateError::RetryExhausted {
                attempt: 3,
                max_attempts: 3,
                ..
            })
        ));

        // Cancelled 不可重试
        let mut cancelled = ActivityStateMachine::new("t6", 1000);
        cancelled.transition(ActivityState::Cancelled, 1).unwrap();
        assert!(matches!(
            cancelled.retry(&policy, 2),
            Err(ActivityStateError::InvalidTransition {
                from: ActivityState::Cancelled,
                ..
            })
        ));
    }
}
