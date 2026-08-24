//! 关系里程碑 —— 关系里的重要事件

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 里程碑类型 —— 关系中重要事件的分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MilestoneKind {
    /// 第一次相遇
    FirstMeeting,
    /// 用户第一次主动分享
    FirstShare,
    /// 用户第一次表达情绪
    FirstEmotion,
    /// 关系阶段跃迁
    StageTransition,
    /// 重要决定
    Decision,
    /// 危机 (冲突)
    Conflict,
    /// 修复 (冲突后)
    Repair,
    /// 用户自定义
    Custom,
}

impl MilestoneKind {
    pub const ALL: [MilestoneKind; 8] = [
        Self::FirstMeeting,
        Self::FirstShare,
        Self::FirstEmotion,
        Self::StageTransition,
        Self::Decision,
        Self::Conflict,
        Self::Repair,
        Self::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::FirstMeeting => "first_meeting",
            Self::FirstShare => "first_share",
            Self::FirstEmotion => "first_emotion",
            Self::StageTransition => "stage_transition",
            Self::Decision => "decision",
            Self::Conflict => "conflict",
            Self::Repair => "repair",
            Self::Custom => "custom",
        }
    }
}

/// 里程碑载荷 —— 里程碑里的实际内容
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MilestonePayload {
    /// 文本
    Text(String),
    /// 数字 (持续时间, 强度, etc.)
    Number(f64),
    /// 阶段跃迁
    Stage(crate::bond::BondStage),
    /// 决策标识
    Decision(String),
    /// 自定义 JSON
    Custom(serde_json::Value),
}

/// 关系里程碑
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    id: Uuid,
    kind: MilestoneKind,
    payload: MilestonePayload,
    at: DateTime<Utc>,
    note: Option<String>,
}

impl Milestone {
    pub fn new(kind: MilestoneKind, payload: MilestonePayload) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            payload,
            at: Utc::now(),
            note: None,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn kind(&self) -> MilestoneKind {
        self.kind
    }
    pub fn payload(&self) -> &MilestonePayload {
        &self.payload
    }
    pub fn at(&self) -> DateTime<Utc> {
        self.at
    }
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn with_note(mut self, note: String) -> Self {
        self.note = Some(note);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestone_creation() {
        let m = Milestone::new(
            MilestoneKind::FirstMeeting,
            MilestonePayload::Text("hi".into()),
        );
        assert_eq!(m.kind(), MilestoneKind::FirstMeeting);
        assert!(m.note().is_none());
    }

    #[test]
    fn milestone_with_note() {
        let m = Milestone::new(MilestoneKind::Decision, MilestonePayload::Text("x".into()))
            .with_note("why".into());
        assert_eq!(m.note(), Some("why"));
    }
}
