//! 按住机制 — 30% 强反对 / 一致反对 / 60s 裁决超时
//!
//! **3 触发条件** (任一满足即按住):
//! 1. 强反对占比 ≥ 30% (`HOLD_STRONG_DISAPPROVE_PERCENT`)
//! 2. 所有非弃权意见一致反对 (unanimous disapprove)
//! 3. 裁决耗时 ≥ 60s (`HOLD_DELIBERATION_TIMEOUT_MS`)
//!
//! 按住后果: 阻塞动作, 进入反思期 (Cognitive-Dream), 等待主权 (`SovereigntyHook`) 仲裁。

use crate::advisor::AdvisorOpinion;
use crate::advisor::StanceKind;
use serde::{Deserialize, Serialize};

/// 按住阈值 (编译时 hardcode)。
pub const HOLD_STRONG_DISAPPROVE_PERCENT: u8 = 30;

/// 按住裁决超时 (60s = 60_000 ms)。
pub const HOLD_DELIBERATION_TIMEOUT_MS: u64 = 60_000;

/// 按住触发原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HoldThreshold {
    /// 强反对占比 ≥ 30% 触发
    StrongDisapprovePercent {
        /// 实际百分比 (0-100)
        actual_percent: u8,
        /// 阈值
        threshold: u8,
    },
    /// 所有非弃权意见一致反对
    UnanimousDisapprove {
        /// 反对人数
        opposing_count: usize,
    },
    /// 裁决超时 (≥ 60s)
    DeliberationTimeout {
        /// 实际耗时 (ms)
        actual_ms: u64,
        /// 阈值 (ms)
        threshold_ms: u64,
    },
}

impl HoldThreshold {
    /// 是否真触发 (各 threshold 类型本身即真触发)。
    pub fn triggered(&self) -> bool {
        match self {
            Self::StrongDisapprovePercent {
                actual_percent,
                threshold,
            } => actual_percent >= threshold,
            Self::UnanimousDisapprove { .. } => true,
            Self::DeliberationTimeout {
                actual_ms,
                threshold_ms,
            } => actual_ms >= threshold_ms,
        }
    }
}

/// 按住触发器 — 评估意见集合是否触发按住。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldTrigger {
    /// 触发原因
    pub threshold: HoldThreshold,
    /// 触发时强反对的意见 IDs
    pub dissenting_opinions: Vec<String>,
}

impl HoldTrigger {
    /// 评估意见集合, 返回 `Some(HoldTrigger)` 当触发时。
    ///
    /// **判定流程**:
    /// 1. 统计强反对人数
    /// 2. 强反对占比 ≥ 30% → 触发
    /// 3. 所有非弃权均反对 → 一致反对触发
    pub fn evaluate(opinions: &[AdvisorOpinion]) -> Option<Self> {
        let total = opinions.len();
        if total == 0 {
            return None;
        }

        let non_abstain: Vec<&AdvisorOpinion> = opinions
            .iter()
            .filter(|o| !o.stance.kind.is_abstain())
            .collect();
        if non_abstain.is_empty() {
            return None;
        }

        let strong_disapprove: Vec<&AdvisorOpinion> = non_abstain
            .iter()
            .filter(|o| o.stance.kind.is_strong_disapprove())
            .copied()
            .collect();
        let disapprove: Vec<&AdvisorOpinion> = non_abstain
            .iter()
            .filter(|o| {
                matches!(
                    o.stance.kind,
                    StanceKind::Disapprove | StanceKind::StrongDisapprove
                )
            })
            .copied()
            .collect();

        // 1. 30% 强反对
        let strong_pct = ((strong_disapprove.len() * 100) / total) as u8;
        if strong_pct >= HOLD_STRONG_DISAPPROVE_PERCENT {
            return Some(Self {
                threshold: HoldThreshold::StrongDisapprovePercent {
                    actual_percent: strong_pct,
                    threshold: HOLD_STRONG_DISAPPROVE_PERCENT,
                },
                dissenting_opinions: strong_disapprove
                    .iter()
                    .map(|o| o.advisor_id.0.clone())
                    .collect(),
            });
        }

        // 2. 一致反对 (所有非弃权均 Disapprove/StrongDisapprove)
        if disapprove.len() == non_abstain.len() {
            return Some(Self {
                threshold: HoldThreshold::UnanimousDisapprove {
                    opposing_count: disapprove.len(),
                },
                dissenting_opinions: disapprove.iter().map(|o| o.advisor_id.0.clone()).collect(),
            });
        }

        None
    }

    /// 评估裁决耗时是否超时。
    pub fn evaluate_timeout(actual_ms: u64) -> Option<Self> {
        if actual_ms >= HOLD_DELIBERATION_TIMEOUT_MS {
            Some(Self {
                threshold: HoldThreshold::DeliberationTimeout {
                    actual_ms,
                    threshold_ms: HOLD_DELIBERATION_TIMEOUT_MS,
                },
                dissenting_opinions: Vec::new(),
            })
        } else {
            None
        }
    }
}

/// 按住决策 — synthesis 产出后调用 [`HoldTrigger::evaluate`] 得到。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldDecision {
    /// 是否按住
    pub held: bool,
    /// 触发原因 (按住时)
    pub trigger: Option<HoldTrigger>,
    /// 阻塞的 advisor IDs (按住时)
    pub blocking_advisors: Vec<String>,
}

impl HoldDecision {
    /// 自由放行 (无按住)。
    pub fn released() -> Self {
        Self {
            held: false,
            trigger: None,
            blocking_advisors: Vec::new(),
        }
    }

    /// 触发按住。
    pub fn held(trigger: HoldTrigger) -> Self {
        let blocking = trigger.dissenting_opinions.clone();
        Self {
            held: true,
            trigger: Some(trigger),
            blocking_advisors: blocking,
        }
    }

    /// 是否被按住。
    pub fn is_held(&self) -> bool {
        self.held
    }
}

/// 按住后果 — 阻塞后进入反思期, 等待主权仲裁。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HoldOutcome {
    /// 反思期开始 (Cognitive-Dream 接管)
    ReflectionStarted {
        /// 按住原因
        reason: String,
        /// 阻塞开始时间 (epoch ms)
        started_at_ms: i64,
    },
    /// 主权仲裁接管 (SovereigntyHook::on_council_verdict 触发)
    SovereigntyAdjudicated {
        /// 主权是否放行
        released: bool,
        /// 主权决定理由
        rationale: String,
    },
    /// 按住已解决 (无后续动作)
    Resolved,
}
