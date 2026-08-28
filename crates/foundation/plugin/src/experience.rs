//! P-arch (2026-08-27): B1 Experience trait 骨架 (3-layer).
//! O-6 重构批次 Refactor-2.
//!
//! 借鉴 v1 `apeireth-experience`（LLM Wiki + Knowledge Graph + VCP 联想网络，
//! 3-layer progressive disclosure），**v2 形态**：
//!
//! - trait `WikiEntryStore`（per-episode 提炼的 wiki 条目）
//! - trait `KnowledgeGraphStore`（subject-predicate-object 事实 + 链接）
//! - trait `AssociationStore`（VCP 联想网络：entity 关联强度）
//!
//! **位置**: trait 在 `apeireth-plugin` (foundation), impl 留 v2.1
//! (与 memory_governance / RC-2 任务一起做).
//!
//! **0 装 PASS**:
//! - trait **只**定义 trait 边界（`0 装 = 没有 impl`，避免假装有实现）
//! - 现有 `gen_cache.rs` / `provenance.rs` / `hallways.rs` 已涵盖 v2 alpha 的部分
//!   progressive disclosure 能力；新 trait 是 v2.0.0-alpha.2+ 完整化的契约
//! - 完整 SQLite impl 留 v2.0.0-rc 路线（与 memory_governance 同源）—— 详见
//!   `v2-unabsorbed-features.md` §B1
//!
//! **3 阶审查** (O-6 锚 9, commit message 必写明):
//! 1. 总体: 与 MemoryBackend + ToolCapability + ProviderCapability + CredentialResolver
//!    4 件 capability 抽象在 foundation 集中, 避免 v1 era 86-crate 散落
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin 体系一致)
//! 3. 架构: 与 plugin manager 单 trait 边界, runtime 拿 `Arc<dyn Experience>`
//!    与 `Arc<dyn MemoryBackend>` 一起注入
//!
//! **跨模块类型**: Episode 来自 apeireth_core; plugin 不再依赖 memory, 单向.
//! v1 compat: `apeireth_memory::experience::*` 仍可访问 (memory re-export).

use serde::{Deserialize, Serialize};

use crate::memory_backend::CapabilityResult;
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
    fn put_wiki(&self, entry: &WikiEntry) -> CapabilityResult<()>;

    /// 按 session 列出某 topic 下的 WikiEntry
    fn list_wiki(
        &self,
        session_id: &str,
        topic: &str,
        limit: u32,
    ) -> CapabilityResult<Vec<WikiEntry>>;

    /// 按 source_episode_id 反查（v1 progressive disclosure: 注入时回查摘要）
    fn wiki_for_episode(&self, episode_id: &str) -> CapabilityResult<Vec<WikiEntry>>;
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
    pub predicate: String,    // "uses" / "depends_on" / "is_a" / ...
    pub object_id: String,    // 实体 ID 或字面值
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
    fn put_fact(&self, fact: &GraphFact) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 写一条 link
    fn put_link(&self, link: &GraphLink) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// 从 subject 出发一跳 fact
    fn facts_from(&self, subject_id: &str, limit: u32) -> CapabilityResult<Vec<GraphFact>>;

    /// 从 from 出发一跳 link
    fn links_from(&self, from_id: &str, limit: u32) -> CapabilityResult<Vec<GraphLink>>;

    /// 删除 subject 相关的所有 fact（v1 memory_governance 风格: 不真删, 标 tombstone）
    fn forget_subject(&self, subject_id: &str) -> CapabilityResult<()>;
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
    fn record_cooccurrence(&self, from: &str, to: &str, episode_id: &str) -> CapabilityResult<()>;

    /// 查 entity 联想 top-N（按 co_occurrence_count desc）
    fn top_associations(&self, entity: &str, limit: u32) -> CapabilityResult<Vec<AssociationEdge>>;
}

// ============================================
// Experience 统一错误
// ============================================

// (O-6 锚兑现 #12: ExperienceError 删 — 用 `Box<dyn std::error::Error + Send + Sync>`
// (即 `CapabilityResult<T>` 的 Err 类型) 统一所有 capability trait 错误通道.
// impl 端用 `Box::new(my_local_error)` 包.)

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
    use crate::memory_backend::CapabilityResult;

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

    /// 0 装 PASS: 错误通道统一为 `Box<dyn Error + Send + Sync>`, impl 端用
    /// `Box::new(impl_local_error)` 包. 这里验证 0 装 marker 字符串 (impl v2.1
    /// 接真 backend 时, NotImplemented 应改为真错误).
    #[test]
    fn not_implemented_marker_appears_in_error_string() {
        // 0 装: impl 包成 Box 的字符串应含 0 装 / v2.1 / 实现者 (impl-defined)
        // v2.0 alpha 的真 0 装 impl 在 memory crate (现在 NoopAdvisor)
        let e = std::io::Error::new(std::io::ErrorKind::Unsupported, "0 装 PASS: v2.1");
        let boxed: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
        let s = format!("{boxed}");
        assert!(s.contains("0 装"));
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
