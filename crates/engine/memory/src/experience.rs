//! P-arch (2026-08-27): B1 Experience trait skeleton (3-layer).
//!
//! 借鉴 v1 `apeireth-experience`（LLM Wiki + Knowledge Graph + VCP 联想网络，
//! 3-layer progressive disclosure），**v2 形态**：
//!
//! - trait `WikiEntryStore`（per-episode 提炼的 wiki 条目）
//! - trait `KnowledgeGraphStore`（subject-predicate-object 事实 + 链接）
//! - trait `AssociationStore`（VCP 联想网络：entity 关联强度）
//!
//! **0 装 PASS**：
//! - trait **只**定义 trait 边界（`0 装 = 没有 impl`，避免假装有实现）
//! - 现有 `gen_cache.rs` / `provenance.rs` / `hallways.rs` 已涵盖 v2 alpha 的部分
//!   progressive disclosure 能力；新 trait 是 v2.0.0-alpha.2+ 完整化的契约
//! - 完整 SQLite impl 留 v2.0.0-rc 路线（与 memory_governance 同源）—— 详见
//!   `v2-unabsorbed-features.md` §B1
//!
//! **架构原则**：
//! - 与 `MemoryBackend` (P1 A4) **解耦**：trait 抽象在 `apeireth-memory` 域
//!   (依赖 core domain types: `Episode`/`Session`)，backend 适配在 storage 层
//! - 0 触碰现有 24 memory 子模块：trait 是**新增**，现有 `SqliteMemoryStore`
//   后续可作为这 3 个 trait 的 impl（v2.1）
//! - 0 装语义: 这文件只提供 trait + minimal 数据结构 (dataclass) — 不预写
//!   "虚假 impl" 假装工作

use serde::{Deserialize, Serialize};

use apeireth_core::Episode;

// ============================================
// Wiki (L1: LLM 提炼的条目)
// ============================================

/// Wiki 条目（v1 借鉴: claude-mem 3-layer progressive disclosure 第一层）
///
/// 每次 episode 写入后，runtime 可异步提炼 1+ WikiEntry（O5 注解 #1）。
/// 检索时按 `source_episode_id` 反查到 episode，再决定是否展开。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WikiEntry {
    /// 全局唯一 id（v1 实践: SHA-256(source_episode_id + topic)[:16] hex）
    pub id: String,
    /// 所属 session
    pub session_id: String,
    /// 来源 episode（v1 D2 §5.3 #2: subject_id 必填）
    pub source_episode_id: String,
    /// 提炼时间戳
    pub extracted_at: i64,
    /// 主体 topic（"这条目是关于什么的"）
    pub topic: String,
    /// 摘要文本（限长，注入用）
    pub summary: String,
    /// 完整内容（按需展开）
    pub body: String,
    /// 置信度 0.0-1.0（v1 同）
    pub confidence: f64,
    /// 标签（检索用）
    pub tags: Vec<String>,
}

/// Wiki 存储 trait（0 装：v2.0 alpha 无 SQLite impl）
///
/// 0 装 PASS: trait 边界清晰, impl 留 v2.1
pub trait WikiEntryStore: Send + Sync {
    /// 写一条 WikiEntry
    fn put_wiki(&self, entry: &WikiEntry) -> Result<(), ExperienceError>;

    /// 按 session 列出某 topic 下的 WikiEntry
    fn list_wiki(&self, session_id: &str, topic: &str, limit: u32) -> Result<Vec<WikiEntry>, ExperienceError>;

    /// 按 source_episode_id 反查（v1 progressive disclosure: 注入时回查摘要）
    fn wiki_for_episode(&self, episode_id: &str) -> Result<Vec<WikiEntry>, ExperienceError>;
}

// ============================================
// Knowledge Graph (L2: 事实 + 链接)
// ============================================

/// Graph 事实 (s/p/o 三元组，v1 借鉴: safishamsi/graphify + VCP 联想网络)
///
/// 命名空间: subject (实体 ID) → predicate (关系) → object (实体 ID 或字面值)
/// v1 实践: subject_id 复用 `PluginId` 或 domain entity; 0 装存 String
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphFact {
    pub id: String,
    pub subject_id: String,
    pub subject_kind: String, // "person" / "tool" / "concept" / ...
    pub predicate: String, // "uses" / "depends_on" / "is_a" / ...
    pub object_id: String,   // 实体 ID 或字面值
    pub object_kind: String,
    /// 时间有效性: 何时该 fact 成立 (v1 D2 §5.4: 暂存为单一 timestamp)
    pub valid_from: i64,
    pub valid_until: Option<i64>,
    /// 来源 episode（s/p/o 写入的 episode）
    pub source_episode_id: String,
    /// 置信度
    pub confidence: f64,
}

/// Graph 链接 (entity-to-entity)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphLink {
    pub from_id: String,
    pub to_id: String,
    pub kind: String, // "cites" / "extends" / "contradicts" / ...
    pub weight: f64,  // 链接强度 0.0-1.0
    pub source_episode_id: String,
    pub created_at: i64,
}

/// KG 存储 trait
pub trait KnowledgeGraphStore: Send + Sync {
    /// 写一条 fact
    fn put_fact(&self, fact: &GraphFact) -> Result<(), ExperienceError>;

    /// 写一条 link
    fn put_link(&self, link: &GraphLink) -> Result<(), ExperienceError>;

    /// 从 subject 出发一跳 fact
    fn facts_from(&self, subject_id: &str, limit: u32) -> Result<Vec<GraphFact>, ExperienceError>;

    /// 从 subject 出发一跳 link
    fn links_from(&self, from_id: &str, limit: u32) -> Result<Vec<GraphLink>, ExperienceError>;

    /// 删除 subject 相关的所有 fact（v1 memory_governance 风格: 不真删, 标 tombstone）
    fn forget_subject(&self, subject_id: &str) -> Result<(), ExperienceError>;
}

// ============================================
// Association (L3: 联想网络)
// ============================================

/// 联想节点 (VCP `compound_eye` 借鉴: entity-pair 共现强度)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssociationNode {
    pub entity_id: String,
    /// 共现 episode 数（边权）
    pub co_occurrence_count: u32,
    /// 最近一次共现时间
    pub last_seen_at: i64,
}

/// 联想边 (entity-pair 共现)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssociationEdge {
    pub from_entity: String,
    pub to_entity: String,
    /// 共现次数（v1 联想强度）
    pub co_occurrence_count: u32,
    /// 最近一次共现的 episode
    pub last_seen_episode_id: String,
    pub last_seen_at: i64,
}

/// 联想网络 trait
pub trait AssociationStore: Send + Sync {
    /// 记录一次 entity-pair 共现（每次 episode 写入时调）
    fn record_cooccurrence(
        &self,
        from: &str,
        to: &str,
        episode_id: &str,
    ) -> Result<(), ExperienceError>;

    /// 查 entity 联想 top-N（按 co_occurrence_count desc）
    fn top_associations(&self, entity: &str, limit: u32) -> Result<Vec<AssociationEdge>, ExperienceError>;
}

// ============================================
// Experience 统一错误
// ============================================

/// Experience trait 统一的错误类型（v2.0 alpha: 0 装, 仅占位）
#[derive(Debug)]
pub enum ExperienceError {
    /// 未实现 (0 装 PASS: trait 0 装, impl 留 v2.1)
    NotImplemented(&'static str),
    /// 底层 backend 错误 (Future: 接 MemoryBackend trait 的错误链)
    Backend(String),
    /// 数据冲突（如 WikiEntry id 已存在）
    Conflict(String),
}

impl std::fmt::Display for ExperienceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented(what) => write!(f, "experience backend not implemented: {what} (0 装 PASS — v2.1 路线)"),
            Self::Backend(msg) => write!(f, "experience backend error: {msg}"),
            Self::Conflict(msg) => write!(f, "experience data conflict: {msg}"),
        }
    }
}

impl std::error::Error for ExperienceError {}

impl From<ExperienceError> for crate::MemoryError {
    fn from(e: ExperienceError) -> Self {
        // experience trait 在 v2.0 是 0 装; 错误回退到 memory 的 Other 通道
        crate::MemoryError::Other(e.to_string())
    }
}

// ============================================
// 占位 helper: 从 episode 提炼到 3 layer 的入口
// ============================================

/// Episode 写入后调一次 (v1 practice: gen_cache + wiki + kg + association 链式触发).
///
/// **0 装 PASS (v2.0 alpha)**: 当前是 `NotImplemented` 占位. 真实 pipeline
/// 在 v2.1 (与 memory_governance.forget + scene-d 例 1 SelfAssessmentCache 一起做).
pub fn extract_experience_from_episode(
    _episode: &Episode,
) -> (Option<WikiEntry>, Vec<GraphFact>, Vec<GraphLink>) {
    // 0 装: 不假装有提炼, 直接返空
    (None, Vec::new(), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P-arch (2026-08-27): 0 装验证.
    /// extract_experience_from_episode 不假装有 pipeline, 直接返空
    /// (None / Vec::new / Vec::new).
    /// 真 pipeline 在 v2.1 与 memory_governance 一起做.
    #[test]
    fn extract_does_not_pretend_to_work_in_v2_alpha() {
        let ep = Episode {
            id: "ep-1".into(),
            timestamp: 1_700_000_000,
            role: "user".into(),
            content: "hello".into(),
            session_id: "s".into(),
        };
        let (w, f, l) = extract_experience_from_episode(&ep);
        assert!(w.is_none());
        assert!(f.is_empty());
        assert!(l.is_empty());
    }

    /// 0 装 PASS: ExperienceError::NotImplemented 描述清楚 trait 在 v2.0 alpha
    /// 0 装, v2.1 路线.
    #[test]
    fn not_implemented_error_documents_zero_implementation() {
        let e = ExperienceError::NotImplemented("WikiEntryStore");
        let s = format!("{e}");
        assert!(s.contains("0 装"));
        assert!(s.contains("WikiEntryStore"));
        assert!(s.contains("v2.1"));
    }

    /// 0 装 PASS: WikiEntry 可从 episode 构造 (smoke), 不依赖 backend.
    /// 真实持久化由 trait impl (v2.1) 提供.
    #[test]
    fn wiki_entry_construction_works() {
        let w = WikiEntry {
            id: "wiki-1".into(),
            session_id: "s".into(),
            source_episode_id: "ep-1".into(),
            extracted_at: 1_700_000_000,
            topic: "memory extraction".into(),
            summary: "extract_experience trait 骨架就位".into(),
            body: "完整 impl 留 v2.1".into(),
            confidence: 0.85,
            tags: vec!["trait".into(), "0装".into()],
        };
        assert_eq!(w.confidence, 0.85);
        assert_eq!(w.tags.len(), 2);
    }
}
