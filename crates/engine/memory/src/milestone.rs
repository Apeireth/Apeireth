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
//! 2. 系统: 放置在 `apeireth-memory`, 与 `RelationStream` 和 `SessionNoteStore` 协同
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
        let mut guard = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
        let guard = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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

// ============================================================================
// 关系里程碑的**生产记录面**（2026-10-24 接电）
// ============================================================================
//
// 此前 `MilestoneStore` 只有内存实现、**没有任何生产写入方** ——
// 契约齐全但一份里程碑都不会被记下来。
//
// 缺的其实不是存储，而是「**什么时候算里程碑**」这个判定。这里只做
// **确定性**的那两个（不猜、不用 LLM、不看内容）：
//
// 1. **初见**：某个主体第一次建立伙伴记录时（`FirstMeeting`）；
// 2. **阶段跃迁**：羁绊阶段从 A 变成 B 时（`StageTransition`）——
//    `Bond::evolve` 的阶段推进是纯确定性阈值（0.15/0.40/0.65/0.85），
//    所以「变了」这件事本身没有歧义。
//
// 其余类型（首次分享 / 情绪共鸣 / 决策 / 冲突 / 修复）需要**语义判定**，
// 本模块不猜 —— 由上层在有明确信号时自行 `record`。

/// 里程碑记录面: 幂等 + 只记确定性事件。
///
/// 幂等是硬要求：装配处每回合都会调，初见只能记一次。
pub struct MilestoneRecorder {
    store: std::sync::Arc<dyn MilestoneStore>,
}

impl std::fmt::Debug for MilestoneRecorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MilestoneRecorder").finish_non_exhaustive()
    }
}

impl MilestoneRecorder {
    /// 绑定一个里程碑存储。
    pub fn new(store: std::sync::Arc<dyn MilestoneStore>) -> Self {
        Self { store }
    }

    /// 记「初见」——**已记过就不再记**（返回 `false`）。
    ///
    /// 之所以要幂等：伙伴记录在每回合都会被 `get_or_create`，
    /// 没有这道闸会写出一堆重复的「初次相遇」。
    pub fn record_first_meeting(
        &self,
        session_id: &SessionId,
        at_epoch_ms: i64,
    ) -> Result<bool, MemoryError> {
        if self
            .store
            .has_milestone(session_id, MilestoneKind::FirstMeeting)?
        {
            return Ok(false);
        }
        let milestone = Milestone::new(
            *session_id,
            MilestoneKind::FirstMeeting,
            MilestonePayload::Text("第一次建立伙伴关系".to_string()),
            at_epoch_ms,
        );
        self.store.record(&milestone)?;
        Ok(true)
    }

    /// 记一次**阶段跃迁**；`from == to` 时什么都不做（返回 `false`）。
    ///
    /// 只在阶段**真的变了**的时候落一条 —— 否则每回合都会写一条噪声。
    pub fn record_stage_transition(
        &self,
        session_id: &SessionId,
        from: &str,
        to: &str,
        at_epoch_ms: i64,
    ) -> Result<bool, MemoryError> {
        if from == to {
            return Ok(false);
        }
        let milestone = Milestone::new(
            *session_id,
            MilestoneKind::StageTransition,
            MilestonePayload::Stage {
                from: from.to_string(),
                to: to.to_string(),
            },
            at_epoch_ms,
        );
        self.store.record(&milestone)?;
        Ok(true)
    }

    /// 查询某会话的关系里程碑（按时间正序）。
    pub fn query(
        &self,
        session_id: &SessionId,
        kind: Option<MilestoneKind>,
    ) -> Result<Vec<Milestone>, MemoryError> {
        self.store.query(session_id, kind)
    }
}

#[cfg(test)]
mod recorder_tests {
    use super::*;

    fn session() -> SessionId {
        SessionId::new()
    }

    #[test]
    fn first_meeting_is_recorded_exactly_once() {
        let recorder = MilestoneRecorder::new(std::sync::Arc::new(InMemoryMilestoneStore::new()));
        let session = session();
        assert!(recorder.record_first_meeting(&session, 1_000).unwrap());
        // 第二、三次都不得重复记（装配处每回合都会调）
        assert!(!recorder.record_first_meeting(&session, 2_000).unwrap());
        assert!(!recorder.record_first_meeting(&session, 3_000).unwrap());
        let list = recorder
            .query(&session, Some(MilestoneKind::FirstMeeting))
            .unwrap();
        assert_eq!(list.len(), 1, "初见只能有一条: {list:?}");
        assert_eq!(list[0].at_epoch_ms, 1_000, "记的是第一次的时间");
    }

    #[test]
    fn stage_transition_is_only_recorded_when_the_stage_changes() {
        let recorder = MilestoneRecorder::new(std::sync::Arc::new(InMemoryMilestoneStore::new()));
        let session = session();
        // 阶段没变 ⇒ 不记（否则每回合一条噪声）
        assert!(!recorder
            .record_stage_transition(&session, "familiar", "familiar", 1_000)
            .unwrap());
        assert!(recorder
            .record_stage_transition(&session, "familiar", "trusted", 2_000)
            .unwrap());
        assert!(recorder
            .record_stage_transition(&session, "trusted", "intimate", 3_000)
            .unwrap());

        let list = recorder
            .query(&session, Some(MilestoneKind::StageTransition))
            .unwrap();
        assert_eq!(list.len(), 2, "{list:?}");
        match &list[0].payload {
            MilestonePayload::Stage { from, to } => {
                assert_eq!((from.as_str(), to.as_str()), ("familiar", "trusted"));
            }
            other => panic!("阶段跃迁应带 Stage 载荷: {other:?}"),
        }
    }

    #[test]
    fn different_sessions_keep_separate_first_meetings() {
        let recorder = MilestoneRecorder::new(std::sync::Arc::new(InMemoryMilestoneStore::new()));
        let one = session();
        let two = session();
        assert!(recorder.record_first_meeting(&one, 1_000).unwrap());
        assert!(recorder.record_first_meeting(&two, 2_000).unwrap());
        assert_eq!(recorder.query(&one, None).unwrap().len(), 1);
        assert_eq!(recorder.query(&two, None).unwrap().len(), 1);
    }
}
