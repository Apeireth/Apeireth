//! Pause / SuspendSelf: 主权暂停与挂起
//!
//! **设计**:
//! - `PauseHandle`: 暂停句柄 (可恢复, 带暂停原因 + 计划恢复时间)
//! - `Suspension`: 挂起 (三种状态: Permanent / Temporary / Pending)
//! - `SuspensionKind`: 区分挂起来源 (Self / External / SGI / HA)

use serde::{Deserialize, Serialize};
use std::fmt;

/// 主权暂停句柄 (可恢复).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PauseHandle {
    /// 暂停 ID
    pub pause_id: String,
    /// 暂停原因
    pub reason: String,
    /// 暂停开始时间 (epoch ms)
    pub paused_at_ms: i64,
    /// 计划恢复时间 (epoch ms, None = 不计划自动恢复)
    pub resume_at_ms: Option<i64>,
    /// 暂停发起者
    pub initiated_by: String,
}

impl PauseHandle {
    /// 便利构造
    pub fn new(
        pause_id: impl Into<String>,
        reason: impl Into<String>,
        paused_at_ms: i64,
        initiated_by: impl Into<String>,
    ) -> Self {
        Self {
            pause_id: pause_id.into(),
            reason: reason.into(),
            paused_at_ms,
            resume_at_ms: None,
            initiated_by: initiated_by.into(),
        }
    }

    /// 设置计划恢复时间
    pub fn with_resume_at(mut self, resume_at_ms: i64) -> Self {
        self.resume_at_ms = Some(resume_at_ms);
        self
    }

    /// 当前是否处于暂停期
    pub fn is_active(&self, current_ms: i64) -> bool {
        match self.resume_at_ms {
            Some(resume_at) => current_ms < resume_at,
            None => true, // 无计划恢复时间 = 永久暂停 (待主动 resume)
        }
    }
}

/// 挂起来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SuspensionKind {
    /// 自身主动挂起 (SuspendSelf)
    SelfInitiated,
    /// 外部触发 (HA / Council 按住)
    ExternalTriggered,
    /// SGI 单字段触发
    SGITriggered,
    /// HA 抗胁迫挂起 (CoercionDetected)
    CoercionDetected,
}

impl fmt::Display for SuspensionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::SelfInitiated => "self",
            Self::ExternalTriggered => "external",
            Self::SGITriggered => "sgi",
            Self::CoercionDetected => "coercion",
        };
        f.write_str(s)
    }
}

/// 主权挂起 (Suspension) 三态。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Suspension {
    /// 永久挂起 (不可自动恢复, 须主权手动决策)
    Permanent {
        /// 挂起原因
        reason: String,
        /// 挂起开始时间 (epoch ms)
        suspended_at_ms: i64,
        /// 挂起来源
        kind: SuspensionKind,
    },
    /// 临时挂起 (到 until_ms 自动恢复)
    Temporary {
        /// 挂起原因
        reason: String,
        /// 挂起开始时间 (epoch ms)
        suspended_at_ms: i64,
        /// 自动恢复时间 (epoch ms)
        until_ms: i64,
        /// 挂起来源
        kind: SuspensionKind,
    },
    /// 待复审挂起 (review_at_ms 复审)
    Pending {
        /// 挂起原因
        reason: String,
        /// 挂起开始时间 (epoch ms)
        suspended_at_ms: i64,
        /// 复审时间 (epoch ms)
        review_at_ms: i64,
        /// 挂起来源
        kind: SuspensionKind,
    },
}

impl Suspension {
    /// 当前是否仍处于挂起状态
    pub fn is_active(&self, current_ms: i64) -> bool {
        match self {
            Self::Permanent { .. } => true,
            Self::Temporary { until_ms, .. } => current_ms < *until_ms,
            Self::Pending { review_at_ms, .. } => current_ms < *review_at_ms,
        }
    }

    /// 挂起来源
    pub fn kind(&self) -> SuspensionKind {
        match self {
            Self::Permanent { kind, .. } => *kind,
            Self::Temporary { kind, .. } => *kind,
            Self::Pending { kind, .. } => *kind,
        }
    }

    /// 挂起原因
    pub fn reason(&self) -> &str {
        match self {
            Self::Permanent { reason, .. } => reason,
            Self::Temporary { reason, .. } => reason,
            Self::Pending { reason, .. } => reason,
        }
    }

    /// 挂起开始时间
    pub fn suspended_at_ms(&self) -> i64 {
        match self {
            Self::Permanent {
                suspended_at_ms, ..
            } => *suspended_at_ms,
            Self::Temporary {
                suspended_at_ms, ..
            } => *suspended_at_ms,
            Self::Pending {
                suspended_at_ms, ..
            } => *suspended_at_ms,
        }
    }
}

impl fmt::Display for Suspension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Permanent { reason, kind, .. } => {
                write!(f, "Permanent({}): {}", kind, reason)
            }
            Self::Temporary {
                reason,
                until_ms,
                kind,
                ..
            } => {
                write!(f, "Temporary({}, until={}): {}", kind, until_ms, reason)
            }
            Self::Pending {
                reason,
                review_at_ms,
                kind,
                ..
            } => {
                write!(
                    f,
                    "Pending({}, review_at={}): {}",
                    kind, review_at_ms, reason
                )
            }
        }
    }
}
