//! `apeireth-memory::milestone` — 关系里程碑机制 (R12-CoordinationContext-2 闭环).
//!
//! 关系里程碑记录人机长程交互与伙伴羁绊中的重大事件（如初见、首次主动分享、情绪共鸣、关系跃迁、重要决策等）。
//!
//! **设计哲学**:
//! - **① 关系流沉淀**: 作为长程伙伴演化与记忆图谱的关键锚点
//! - **② 0 假装 (O-5)**: 纯确定性数据模型与持久化契约，时间戳显式注入 (`at_epoch_ms`)
//! - **③ 核心保护 (O-1)**: 关系里程碑记录防篡改，按 session 隔离索引
//!
//! **O-6 三阶审查**:
//! 1. 总体: 为伙伴自主演化与长期关系沉淀提供不可变里程碑标记
//! 2. 系统: 放置在 `apeireth-memory`, 与 `RelationStream` 和 `SessionStore` 协同
//! 3. 架构: 强类型 Payload 与 Trait 契约, 支持内存与 SQLite 存储

use std::collections::BTreeMap;
use std::sync::Mutex;

use apeireth_core::kernel::SessionId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::MemoryError;

/// 里程碑分类 (关系发展中的标志性节点).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneKind {
    /// 第一次相遇与初次对话
    FirstMeeting,
    /// 用户第一次主动分享个人生活或偏好
    FirstShare,
    /// 用户或伙伴第一次表达真实情绪共鸣
    FirstEmotion,
    /// 关系阶段发生跃迁 (如从陌生到熟络、从协作到信任)
    StageTransition,
    /// 共同做出的重要决定
    Decision,
    /// 交互中出现的危机或观点冲突
    Conflict,
    /// 冲突后的理解与修复
    Repair,
    /// 用户自定义里程碑
    Custom,
}

impl MilestoneKind {
    /// 全部枚举分类.
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

    /// 对应的标识字符串.
    pub const fn label(self) -> &'static str {
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

/// 里程碑承载的具体内容 (类型化 Payload).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum MilestonePayload {
    /// 描述性文本
    Text(String),
    /// 数值指标 (如亲密度分值、持续天数)
    Number(f64),
    /// 阶段跃迁 (记录跃迁前后状态)
    Stage { from: String, to: String },
    /// 决策标识/决策内容
    Decision(String),
    /// 结构化自定义 JSON 数据
    Custom(serde_json::Value),
}

/// 关系里程碑实体.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    /// 唯一标识 ID
    pub id: String,
    /// 关联的会话 ID
    pub session_id: SessionId,
    /// 里程碑类型
    pub kind: MilestoneKind,
    /// 承载内容
    pub payload: MilestonePayload,
    /// 发生时间戳 (Unix epoch 毫秒)
    pub at_epoch_ms: i64,
    /// 补充备注/上下文说明
    pub note: Option<String>,
}

impl Milestone {
    /// 构造新的里程碑实例.
    pub fn new(
        session_id: SessionId,
        kind: MilestoneKind,
        payload: MilestonePayload,
        at_epoch_ms: i64,
    ) -> Self {
        Self {
            id: format!("ms-{}", Uuid::new_v4()),
            session_id,
            kind,
            payload,
            at_epoch_ms,
            note: None,
        }
    }

    /// 附加备注说明.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// 里程碑持久化与检索 Trait.
pub trait MilestoneStore: Send + Sync {
    /// 记录一条新的里程碑.
    fn record(&self, milestone: &Milestone) -> Result<(), MemoryError>;

    /// 查询某会话下的里程碑列表 (可选按类型过滤, 按时间正序排列).
    fn query(
        &self,
        session_id: &SessionId,
        kind: Option<MilestoneKind>,
    ) -> Result<Vec<Milestone>, MemoryError>;

    /// 快速检查某会话是否已达成特定里程碑.
    fn has_milestone(
        &self,
        session_id: &SessionId,
        kind: MilestoneKind,
    ) -> Result<bool, MemoryError> {
        let list = self.query(session_id, Some(kind))?;
        Ok(!list.is_empty())
    }
}

/// 内存版里程碑存储 (供测试与轻量嵌入场景使用).
#[derive(Debug, Default)]
pub struct InMemoryMilestoneStore {
    items: Mutex<BTreeMap<SessionId, Vec<Milestone>>>,
}

impl InMemoryMilestoneStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MilestoneStore for InMemoryMilestoneStore {
    fn record(&self, milestone: &Milestone) -> Result<(), MemoryError> {
        let mut guard = self.items.lock().expect("in-memory milestone store mutex");
        let entries = guard.entry(milestone.session_id).or_default();
        entries.push(milestone.clone());
        entries.sort_by_key(|m| m.at_epoch_ms);
        Ok(())
    }

    fn query(
        &self,
        session_id: &SessionId,
        kind: Option<MilestoneKind>,
    ) -> Result<Vec<Milestone>, MemoryError> {
        let guard = self.items.lock().expect("in-memory milestone store mutex");
        let Some(entries) = guard.get(session_id) else {
            return Ok(Vec::new());
        };
        let filtered = entries
            .iter()
            .filter(|m| kind.map_or(true, |k| m.kind == k))
            .cloned()
            .collect();
        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestone_creation_and_labels() {
        let session = SessionId::new();
        let ms = Milestone::new(
            session,
            MilestoneKind::FirstMeeting,
            MilestonePayload::Text("初次启动与相遇".into()),
            1756400000000,
        )
        .with_note("建立连接");

        assert_eq!(ms.kind.label(), "first_meeting");
        assert_eq!(ms.session_id, session);
        assert_eq!(ms.note.as_deref(), Some("建立连接"));
    }

    #[test]
    fn in_memory_store_record_query_and_has() {
        let store = InMemoryMilestoneStore::new();
        let session = SessionId::new();

        assert!(!store
            .has_milestone(&session, MilestoneKind::FirstMeeting)
            .unwrap());

        let ms1 = Milestone::new(
            session,
            MilestoneKind::FirstMeeting,
            MilestonePayload::Text("初次相识".into()),
            1000,
        );
        let ms2 = Milestone::new(
            session,
            MilestoneKind::StageTransition,
            MilestonePayload::Stage {
                from: "acquaintance".into(),
                to: "trusted_friend".into(),
            },
            2000,
        );

        store.record(&ms1).unwrap();
        store.record(&ms2).unwrap();

        assert!(store
            .has_milestone(&session, MilestoneKind::FirstMeeting)
            .unwrap());
        assert!(store
            .has_milestone(&session, MilestoneKind::StageTransition)
            .unwrap());
        assert!(!store
            .has_milestone(&session, MilestoneKind::Conflict)
            .unwrap());

        let all = store.query(&session, None).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].kind, MilestoneKind::FirstMeeting);
        assert_eq!(all[1].kind, MilestoneKind::StageTransition);

        let filtered = store
            .query(&session, Some(MilestoneKind::StageTransition))
            .unwrap();
        assert_eq!(filtered.len(), 1);
        if let MilestonePayload::Stage { from, to } = &filtered[0].payload {
            assert_eq!(from, "acquaintance");
            assert_eq!(to, "trusted_friend");
        } else {
            panic!("预期 Stage Payload");
        }
    }

    #[test]
    fn milestone_serde_roundtrip() {
        let session = SessionId::new();
        let ms = Milestone::new(
            session,
            MilestoneKind::Decision,
            MilestonePayload::Decision("选定 Rust 为唯一底层主干语言".into()),
            1756400000000,
        );
        let json = serde_json::to_string(&ms).unwrap();
        let decoded: Milestone = serde_json::from_str(&json).unwrap();
        assert_eq!(ms, decoded);
    }
}
