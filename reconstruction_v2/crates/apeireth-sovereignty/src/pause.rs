//! Pause / SuspendSelf: 主权暂停与挂起

use serde::{Deserialize, Serialize};
use std::fmt;

/// 主权暂停句柄 (可恢复).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PauseHandle {
    pub pause_id: String,
    pub reason: String,
    pub paused_at_ms: i64,
    pub resume_at_ms: Option<i64>,
    pub initiated_by: String,
}

impl PauseHandle {
    pub fn new(
        pause_id: impl Into<String>,
        reason: impl Into<String>,
        paused_at_ms: i64,
        initiated_by: impl Into<String>,
    ) -> Self {
        Self { pause_id: pause_id.into(), reason: reason.into(), paused_at_ms, resume_at_ms: None, initiated_by: initiated_by.into() }
    }
    pub fn with_resume_at(mut self, resume_at_ms: i64) -> Self {
        self.resume_at_ms = Some(resume_at_ms);
        self
    }
    pub fn is_active(&self, current_ms: i64) -> bool {
        match self.resume_at_ms {
            Some(resume_at) => current_ms < resume_at,
            None => true,
        }
    }
}

/// 挂起来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SuspensionKind {
    SelfInitiated,
    ExternalTriggered,
    SGITriggered,
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
    Permanent { reason: String, suspended_at_ms: i64, kind: SuspensionKind },
    Temporary { reason: String, suspended_at_ms: i64, until_ms: i64, kind: SuspensionKind },
    Pending { reason: String, suspended_at_ms: i64, review_at_ms: i64, kind: SuspensionKind },
}

impl Suspension {
    pub fn is_active(&self, current_ms: i64) -> bool {
        match self {
            Self::Permanent { .. } => true,
            Self::Temporary { until_ms, .. } => current_ms < *until_ms,
            Self::Pending { review_at_ms, .. } => current_ms < *review_at_ms,
        }
    }
    pub fn kind(&self) -> SuspensionKind {
        match self {
            Self::Permanent { kind, .. } => *kind,
            Self::Temporary { kind, .. } => *kind,
            Self::Pending { kind, .. } => *kind,
        }
    }
    pub fn reason(&self) -> &str {
        match self {
            Self::Permanent { reason, .. } => reason,
            Self::Temporary { reason, .. } => reason,
            Self::Pending { reason, .. } => reason,
        }
    }
    pub fn suspended_at_ms(&self) -> i64 {
        match self {
            Self::Permanent { suspended_at_ms, .. } => *suspended_at_ms,
            Self::Temporary { suspended_at_ms, .. } => *suspended_at_ms,
            Self::Pending { suspended_at_ms, .. } => *suspended_at_ms,
        }
    }
}

impl fmt::Display for Suspension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Permanent { reason, kind, .. } => write!(f, "Permanent({}): {}", kind, reason),
            Self::Temporary { reason, until_ms, kind, .. } => write!(f, "Temporary({}, until={}): {}", kind, until_ms, reason),
            Self::Pending { reason, review_at_ms, kind, .. } => write!(f, "Pending({}, review_at={}): {}", kind, review_at_ms, reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn pause_handle_new() {
        let p = PauseHandle::new("p1", "reason", 1000, "alice");
        assert_eq!(p.pause_id, "p1");
        assert_eq!(p.reason, "reason");
        assert_eq!(p.paused_at_ms, 1000);
        assert!(p.resume_at_ms.is_none());
        assert!(p.is_active(2000));
        assert!(p.is_active(1_000_000_000));
    }
    #[test] fn pause_handle_with_resume_at() {
        let p = PauseHandle::new("p", "x", 1000, "b").with_resume_at(2000);
        assert!(p.is_active(1500));
        assert!(!p.is_active(2500));
    }
    #[test] fn suspension_states() {
        let perm = Suspension::Permanent { reason: "x".into(), suspended_at_ms: 0, kind: SuspensionKind::SelfInitiated };
        assert!(perm.is_active(1_000_000_000));
        let tmp = Suspension::Temporary { reason: "x".into(), suspended_at_ms: 0, until_ms: 100, kind: SuspensionKind::SelfInitiated };
        assert!(tmp.is_active(50));
        assert!(!tmp.is_active(150));
        let pend = Suspension::Pending { reason: "x".into(), suspended_at_ms: 0, review_at_ms: 100, kind: SuspensionKind::SelfInitiated };
        assert!(pend.is_active(50));
        assert!(!pend.is_active(150));
    }
    #[test] fn suspension_kind_display() {
        assert_eq!(format!("{}", SuspensionKind::SelfInitiated), "self");
        assert_eq!(format!("{}", SuspensionKind::CoercionDetected), "coercion");
    }
}
