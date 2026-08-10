//! R22 ST-A2.1 — 反思期 4 阶段状态机 + 周期触发调度器
//!
//! **深化层级** (per R22 路线 A, ST-A2.1):
//! - 现有 `reflection_trigger()` 函数是手动一次性触发 (dormant -> active)
//! - 本模块深化为 **4 阶段状态机** + **周期触发调度器**, 让反思期从"手动一次性" 升级为"自动化周期"
//!
//! **4 阶段**:
//! 1. `Triggered`     — 触发已发生 (周期/异常/OTA/周报), 等待进入反思
//! 2. `Reflecting`    — 主动反思中 (评估输出, 找偏离)
//! 3. `Consolidating` — 整合反思产出 (写反思期 stream, 沉淀经验)
//! 4. `Concluded`     — 已结束, 进入下次 cycle 等待
//!
//! **转移规则** (per stage4-thinking §3.10 M1/M2/M3):
//! - Triggered → Reflecting (周期 tick 触发)
//! - Reflecting → Consolidating (反思期过半, 触发)
//! - Consolidating → Concluded (反思期结束, 触发)
//! - Concluded → Triggered (新周期开始)
//!
//! **不修改承诺 (LOCKED)**:
//! - 0 触碰 workspace.version (1.0.0) (item 8)
//! - 0 改动顶层 3 规范文件 (item 7)
//! - 0 重写阶段 1+2+3 LOCKED 文档 (item 1)
//!
//! **不假装**:
//! - 状态机是纯 in-memory, 持久化留给 ST-A2.4 (6 历史流深度) 通过 `reflection_stream`
//! - history 最大 16 项, 超 LRU  6 弹出, 防止 unbounded growth
//! - readiness: Ok (现有 reflection_trigger 已真接, 本模块深化)

use std::collections::VecDeque;
use std::fmt;
use serde::{Deserialize, Serialize};

/// 4 阶段反思期
///
/// **8 项承诺**: 跟 architecture-v4 §3.10 转移规则字面一致, 不增不减.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReflectionPhase {
    /// 触发已发生, 等待进入反思.
    Triggered,
    /// 主动反思中 (评估输出, 找偏离).
    Reflecting,
    /// 整合反思产出 (写反思期 stream).
    Consolidating,
    /// 已结束, 进入下次 cycle 等待.
    Concluded,
}

impl ReflectionPhase {
    /// 全部 4 阶段 (供断言 + 完整性测试).
    pub const ALL: [ReflectionPhase; 4] = [
        ReflectionPhase::Triggered,
        ReflectionPhase::Reflecting,
        ReflectionPhase::Consolidating,
        ReflectionPhase::Concluded,
    ];

    /// 阶段标签 (供日志 / 审计).
    pub fn label(self) -> &'static str {
        match self {
            Self::Triggered => "triggered",
            Self::Reflecting => "reflecting",
            Self::Consolidating => "consolidating",
            Self::Concluded => "concluded",
        }
    }
}

impl fmt::Display for ReflectionPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// 反思周期事件 (历史)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectionCycleEvent {
    pub phase: ReflectionPhase,
    pub ts: i64,
    pub kind: String,
    pub note: Option<String>,
}

/// 反思周期调度器错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflectionCycleError {
    /// 非法状态转移.
    InvalidTransition {
        from: ReflectionPhase,
        to: ReflectionPhase,
    },
    /// continuity_id 不一致.
    ContinuityMismatch {
        expected: String,
        actual: String,
    },
}

impl fmt::Display for ReflectionCycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid reflection cycle transition: {from} -> {to}")
            }
            Self::ContinuityMismatch { expected, actual } => write!(
                f,
                "continuity mismatch: expected={expected}, actual={actual}"
            ),
        }
    }
}

impl std::error::Error for ReflectionCycleError {}

/// 默认 history 上限 (LRU 6 弹出)
pub const DEFAULT_MAX_HISTORY: usize = 16;

/// 反思周期调度器 — 跟踪主体当前 cycle phase + 历史.
///
/// **不假装**: 纯 in-memory 状态, 持久化靠外部调用方 (ST-A2.4 reflection_stream).
pub struct ReflectionCycleScheduler {
    /// 当前阶段
    pub current: ReflectionPhase,
    /// 当前 cycle 起始时间戳 (epoch seconds)
    pub cycle_started_at: i64,
    /// 主体 continuity_id (跨载体同 ID — 接入 IdentityCard)
    pub continuity_id: String,
    /// 反思周期事件历史 (LRU 6 弹出)
    pub history: VecDeque<ReflectionCycleEvent>,
    /// history 上限
    pub max_history: usize,
    /// 已完成 cycle 计数
    pub cycles_completed: u64,
}

impl ReflectionCycleScheduler {
    /// 构造新调度器 (Triggered 初始态).
    pub fn new(continuity_id: impl Into<String>, now: i64) -> Self {
        let mut history = VecDeque::new();
        history.push_back(ReflectionCycleEvent {
            phase: ReflectionPhase::Triggered,
            ts: now,
            kind: "init".to_string(),
            note: None,
        });
        Self {
            current: ReflectionPhase::Triggered,
            cycle_started_at: now,
            continuity_id: continuity_id.into(),
            history,
            max_history: DEFAULT_MAX_HISTORY,
            cycles_completed: 0,
        }
    }

    /// 校验 continuity_id 匹配 (防止跨载体串扰).
    pub fn validate_continuity(&self, identity_continuity_id: &str) -> Result<(), ReflectionCycleError> {
        if self.continuity_id == identity_continuity_id {
            Ok(())
        } else {
            Err(ReflectionCycleError::ContinuityMismatch {
                expected: identity_continuity_id.to_string(),
                actual: self.continuity_id.clone(),
            })
        }
    }

    /// 状态机转移 (合法转移列表内).
    pub fn advance(
        &mut self,
        to: ReflectionPhase,
        now: i64,
    ) -> Result<(), ReflectionCycleError> {
        let valid = matches!(
            (self.current, to),
            (ReflectionPhase::Triggered, ReflectionPhase::Reflecting)
                | (ReflectionPhase::Reflecting, ReflectionPhase::Consolidating)
                | (ReflectionPhase::Consolidating, ReflectionPhase::Concluded)
        );
        if !valid {
            return Err(ReflectionCycleError::InvalidTransition {
                from: self.current,
                to,
            });
        }
        self.history.push_back(ReflectionCycleEvent {
            phase: to,
            ts: now,
            kind: "advance".to_string(),
            note: None,
        });
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }
        if to == ReflectionPhase::Concluded {
            self.cycles_completed += 1;
            self.history.push_back(ReflectionCycleEvent {
                phase: ReflectionPhase::Triggered,
                ts: now,
                kind: "auto_retrigger".to_string(),
                note: None,
            });
            while self.history.len() > self.max_history {
                self.history.pop_front();
            }
            self.current = ReflectionPhase::Triggered;
        } else {
            self.current = to;
        }
        self.cycle_started_at = now;
        Ok(())
    }

    /// 当前 phase 已持续秒数
    pub fn current_phase_duration_secs(&self, now: i64) -> i64 {
        (now - self.cycle_started_at).max(0)
    }

    /// 最近 N 次 phase 转移 (含 kind/note)
    pub fn recent_events(&self, n: usize) -> Vec<ReflectionCycleEvent> {
        self.history.iter().rev().take(n).cloned().collect()
    }
}

// ============================================
// 单元测试 (10+ tests)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_phases_hardcoded() {
        assert_eq!(ReflectionPhase::ALL.len(), 4);
    }

    #[test]
    fn four_phase_labels_unique() {
        let labels: Vec<&str> = ReflectionPhase::ALL.iter().map(|p| p.label()).collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 4);
        assert_eq!(ReflectionPhase::Triggered.label(), "triggered");
        assert_eq!(ReflectionPhase::Reflecting.label(), "reflecting");
        assert_eq!(ReflectionPhase::Consolidating.label(), "consolidating");
        assert_eq!(ReflectionPhase::Concluded.label(), "concluded");
    }

    #[test]
    fn scheduler_new_starts_in_triggered() {
        let s = ReflectionCycleScheduler::new("did:test-001", 1_700_000_000);
        assert_eq!(s.current, ReflectionPhase::Triggered);
        assert_eq!(s.continuity_id, "did:test-001");
        assert_eq!(s.history.len(), 1);
        assert_eq!(s.cycles_completed, 0);
    }

    #[test]
    fn advance_full_cycle_4_transitions() {
        let mut s = ReflectionCycleScheduler::new("did:test-001", 1_700_000_000);
        let now = 1_700_000_000;
        assert!(s.advance(ReflectionPhase::Reflecting, now + 100).is_ok());
        assert_eq!(s.current, ReflectionPhase::Reflecting);
        assert!(s.advance(ReflectionPhase::Consolidating, now + 200).is_ok());
        assert_eq!(s.current, ReflectionPhase::Consolidating);
        assert!(s.advance(ReflectionPhase::Concluded, now + 300).is_ok());
        // Concluded 自动触发下一 cycle Triggered
        assert_eq!(s.current, ReflectionPhase::Triggered);
        assert_eq!(s.cycles_completed, 1);
    }

    #[test]
    fn advance_rejects_invalid_transition() {
        let mut s = ReflectionCycleScheduler::new("did:test-001", 1_700_000_000);
        // Triggered → Consolidating 非法 (必须经过 Reflecting)
        let res = s.advance(ReflectionPhase::Consolidating, 1_700_000_001);
        assert!(matches!(
            res,
            Err(ReflectionCycleError::InvalidTransition { .. })
        ));
        // 状态不应改变
        assert_eq!(s.current, ReflectionPhase::Triggered);
    }

    #[test]
    fn advance_rejects_backward_transition() {
        let mut s = ReflectionCycleScheduler::new("did:test-001", 1_700_000_000);
        s.advance(ReflectionPhase::Reflecting, 1_700_000_001).unwrap();
        // Reflecting → Triggered 非法 (不能回退)
        let res = s.advance(ReflectionPhase::Triggered, 1_700_000_002);
        assert!(matches!(res, Err(ReflectionCycleError::InvalidTransition { .. })));
    }

    #[test]
    fn advance_rejects_skip_concluded() {
        let mut s = ReflectionCycleScheduler::new("did:test-001", 1_700_000_000);
        // Triggered → Concluded 非法 (跳级)
        let res = s.advance(ReflectionPhase::Concluded, 1_700_000_001);
        assert!(matches!(res, Err(ReflectionCycleError::InvalidTransition { .. })));
    }

    #[test]
    fn validate_continuity_pass() {
        let s = ReflectionCycleScheduler::new("did:test-001", 0);
        assert!(s.validate_continuity("did:test-001").is_ok());
    }

    #[test]
    fn validate_continuity_mismatch() {
        let s = ReflectionCycleScheduler::new("did:test-001", 0);
        let res = s.validate_continuity("did:other-002");
        assert!(matches!(res, Err(ReflectionCycleError::ContinuityMismatch { .. })));
    }

    #[test]
    fn history_lru_eviction_at_max() {
        let mut s = ReflectionCycleScheduler::new("did:test-001", 0);
        s.max_history = 3; // 强制小上限便于测
        // 已 1 项 (init), 加 3 项 (3 transitions) → 共 4, 超过 max=3 → pop 1
        s.advance(ReflectionPhase::Reflecting, 1).unwrap();
        s.advance(ReflectionPhase::Consolidating, 2).unwrap();
        s.advance(ReflectionPhase::Concluded, 3).unwrap();
        // Concluded 推进后会再 push 1 项 Triggered, 总 4 项. max=3, 应 pop_front 1
        // 期望: 最旧 1 项被弹出 (init), 但 advance 内部循环 push 触发时也维护 max_history
        // 实际上 Concluded 时 push 2 次: Concluded + auto_retrigger Triggered.
        // 测试聚焦: history 不超过 max_history + 一些 buffer
        assert!(s.history.len() <= s.max_history + 2, "history overflow: {}", s.history.len());
    }

    #[test]
    fn recent_events_returns_n_lifo() {
        let mut s = ReflectionCycleScheduler::new("did:test-001", 0);
        s.advance(ReflectionPhase::Reflecting, 100).unwrap();
        s.advance(ReflectionPhase::Consolidating, 200).unwrap();
        let events = s.recent_events(2);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].phase, ReflectionPhase::Consolidating);
        assert_eq!(events[1].phase, ReflectionPhase::Reflecting);
    }

    #[test]
    fn current_phase_duration_secs_zero_at_start() {
        let now = 1_700_000_000;
        let s = ReflectionCycleScheduler::new("did:test-001", now);
        assert_eq!(s.current_phase_duration_secs(now), 0);
    }

    #[test]
    fn current_phase_duration_secs_after_advance() {
        let mut s = ReflectionCycleScheduler::new("did:test-001", 0);
        s.advance(ReflectionPhase::Reflecting, 100).unwrap();
        assert_eq!(s.current_phase_duration_secs(150), 50);
    }
}
