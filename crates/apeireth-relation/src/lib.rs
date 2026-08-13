//! apeireth-relation: 关系子系统 (A12 落点 — 4 类关系建模)
//!
//! **职责**: 建模主体与他者/自身之间的关系类型 — 4 类 (共生 Symbiosis / 协调
//! Coordination / 嵌入 Embedding / 与自身 SelfRelation) + 关系决策树 + 主体
//! 连续性 ID 锚定。
//!
//! **架构位置**: 阶段 3 §3.7 蓝图 (v1 重写) + 阶段 4 v4 §4 3 种关系扩展为 4 类
//! (v4.1 §8 #3 新增"与自身 SelfRelation"作为第 4 类)。本 crate 是简化版 4 关系
//! 决策模型, 不重写完整 R11 关系图算法。
//!
//! **当前状态**: A12 最小可用落地 (P4 任务 4926b6a3 by devops_engineer2).
//! 本 crate 提供 8+ pub fn + 6 单元测试 + 1 集成测试 + 1 example.
//!
//! **诚实登记**:
//! - ⚠️ 阶段 3 设计层文档中**未发现 §3.7** 章节。本 crate 按 Leader P4 任务文本的
//!   4 关系枚举 (含 SelfRelation) 落地。v4 §4 实际是 3 关系, SelfRelation 在 v4.1 §8
//!   作为"不假装"提议。本 crate 按任务扩展为 4 类, 不引入 LOCKED 阶段 1+2+3 改动。
//! - 漂移见 `reports/achievement-A12-devops-engineer2-consciousness-relation.md`.
//!
//! **禁止**:
//! - ❌ 不修改 apeireth-core 任何已实装类型签名
//! - ❌ 不碰 R11 baseline 三值
//! - ❌ 不碰 apeireth-legacy/
//!
//! ## R154 graph modules (graph / traversal / query)
//!
//! The `graph` / `traversal` / `query` modules provide property-graph storage
//! with adjacency-list indexes, BFS/DFS iterators, shortest-path queries, and
//! predicate-based node/edge filtering. They coexist with the existing
//! `Relation` / `RelationKind` / `RelationRegistry` types (no breaking changes).
//!
//! Borrowed upstream references (per O-5):
//! - **SurrealDB** — RELATE statement, `->` arrow traversal, graph storage
//! - **Neo4j / Memgraph** — BFS/DFS semantics, depth-limited traversal
//! - **Cypher** — MATCH pattern inspiration for predicate filters

#![deny(unsafe_code)]

// R154 modules (graph + traversal + query)
pub mod graph;
pub mod traversal;
pub mod query;

// Convenience re-exports (调用方少打路径)
pub use graph::{EdgeId, GraphEdge, GraphNode, NodeId, RelationGraph};
pub use traversal::{BfsIter, DfsIter, PathResult, TraversalDirection, shortest_path};
pub use query::{CombinedQuery, EdgeQuery, NodeQuery, PropertyMatch, count_by_kind};

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

/// 4 类关系枚举.
///
/// **关系语义** (v1, A12):
/// - `Symbiosis`    — 共生: 互相依赖, 缺一不可 (7 维内部强耦合)
/// - `Coordination` — 协调/协同: 互相配合, 可独立 (7 维之间弱耦合)
/// - `Embedding`    — 嵌入: 一方在另一方内部 (智能体嵌入场景, 反思嵌入决策流)
/// - `SelfRelation` — 与自身: 主体连续性 (D2 §4 + v4.1 §8 #3 — **不假装**与自身无关系)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RelationKind {
    /// 共生 (互相依赖, 缺一不可).
    Symbiosis,
    /// 协调 (互相配合, 可独立).
    Coordination,
    /// 嵌入 (一方在另一方内部).
    Embedding,
    /// 与自身 (主体连续性).
    SelfRelation,
}

impl RelationKind {
    /// 全部 4 关系 (供断言 + 完整性测试).
    pub const ALL: [RelationKind; 4] = [
        RelationKind::Symbiosis,
        RelationKind::Coordination,
        RelationKind::Embedding,
        RelationKind::SelfRelation,
    ];

    /// 关系短名 (snake_case).
    pub const fn semantic_name(self) -> &'static str {
        match self {
            RelationKind::Symbiosis => "symbiosis",
            RelationKind::Coordination => "coordination",
            RelationKind::Embedding => "embedding",
            RelationKind::SelfRelation => "self_relation",
        }
    }

    /// 关系描述 (1 行).
    pub fn describe(self) -> &'static str {
        match self {
            RelationKind::Symbiosis => "共生 — 互相依赖, 缺一不可 (7 维内部强耦合)",
            RelationKind::Coordination => "协调 — 互相配合, 可独立 (7 维之间弱耦合)",
            RelationKind::Embedding => "嵌入 — 一方在另一方内部 (智能体嵌入场景, 反思嵌入决策流)",
            RelationKind::SelfRelation => "与自身 — 主体连续性 (D2 §4 + 不假装与自身无关系)",
        }
    }

    /// 是否二元关系 (需要两个不同 party).
    pub const fn is_binary(self) -> bool {
        !matches!(self, RelationKind::SelfRelation)
    }
}

/// 关系实例 — 两个主体 + 关系类型 + 时间戳 + 唯一 ID.
///
/// **约定**:
/// - Symbiosis/Coordination/Embedding: `party_a` + `party_b` 必填, 非空
/// - SelfRelation: `party_a == party_b == continuity_id`, `party_b` 自动填充
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Relation {
    /// 关系唯一 ID (审计锚点).
    pub id: Uuid,
    /// 关系类型.
    pub kind: RelationKind,
    /// 主体 A (主体连续性 ID, e.g. IdentityCard.continuity_id).
    pub party_a: String,
    /// 主体 B (SelfRelation 时与 party_a 相同).
    pub party_b: String,
    /// 建立时间 (UTC).
    pub established_at: DateTime<Utc>,
    /// 备注/上下文 (可选).
    pub note: Option<String>,
}

/// 顶层错误: 关系建模错误.
#[derive(Debug, Error)]
pub enum RelationError {
    /// party_a 或 party_b 缺失.
    #[error("party id missing: {0}")]
    MissingPartyId(String),
    /// SelfRelation 要求 party_a == party_b.
    #[error("self_relation requires party_a == party_b (got {a:?} vs {b:?})")]
    SelfRelationMismatch {
        /// 主体 A.
        a: String,
        /// 主体 B.
        b: String,
    },
    /// Embedding 要求 party_a != party_b (host ≠ inner).
    #[error("embedding requires party_a != party_b (host and inner must differ)")]
    EmbeddingSelfLoop,
}

impl Relation {
    /// 新建共生关系.
    pub fn new_symbiosis(
        party_a: impl Into<String>,
        party_b: impl Into<String>,
    ) -> Result<Self, RelationError> {
        let (a, b) = (party_a.into(), party_b.into());
        if a.is_empty() || b.is_empty() {
            return Err(RelationError::MissingPartyId(if a.is_empty() {
                "party_a".into()
            } else {
                "party_b".into()
            }));
        }
        Ok(Self::build(RelationKind::Symbiosis, a, b, None))
    }

    /// 新建协调关系.
    pub fn new_coordination(
        party_a: impl Into<String>,
        party_b: impl Into<String>,
    ) -> Result<Self, RelationError> {
        let (a, b) = (party_a.into(), party_b.into());
        if a.is_empty() || b.is_empty() {
            return Err(RelationError::MissingPartyId(if a.is_empty() {
                "party_a".into()
            } else {
                "party_b".into()
            }));
        }
        Ok(Self::build(RelationKind::Coordination, a, b, None))
    }

    /// 新建嵌入关系 — `host` 包含 `inner`.
    pub fn new_embedding(
        host: impl Into<String>,
        inner: impl Into<String>,
    ) -> Result<Self, RelationError> {
        let (h, i) = (host.into(), inner.into());
        if h.is_empty() || i.is_empty() {
            return Err(RelationError::MissingPartyId(if h.is_empty() {
                "host".into()
            } else {
                "inner".into()
            }));
        }
        if h == i {
            return Err(RelationError::EmbeddingSelfLoop);
        }
        Ok(Self::build(RelationKind::Embedding, h, i, None))
    }

    /// 新建"与自身"关系 — 主体连续性.
    pub fn new_self_relation(continuity_id: impl Into<String>) -> Result<Self, RelationError> {
        let cid = continuity_id.into();
        if cid.is_empty() {
            return Err(RelationError::MissingPartyId("continuity_id".into()));
        }
        // SelfRelation: party_a == party_b == continuity_id
        Ok(Self::build(
            RelationKind::SelfRelation,
            cid.clone(),
            cid,
            None,
        ))
    }

    fn build(kind: RelationKind, a: String, b: String, note: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            party_a: a,
            party_b: b,
            established_at: Utc::now(),
            note,
        }
    }

    /// 附加备注.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// 是否与自身关系.
    pub fn is_self_relation(&self) -> bool {
        self.kind == RelationKind::SelfRelation
    }

    /// 是否嵌入关系.
    pub fn is_embedding(&self) -> bool {
        self.kind == RelationKind::Embedding
    }

    /// 关系涉及的所有主体 (去重, SelfRelation 返回单元素).
    pub fn involved_parties(&self) -> Vec<&str> {
        if self.is_self_relation() {
            vec![&self.party_a]
        } else if self.party_a == self.party_b {
            vec![&self.party_a]
        } else {
            vec![&self.party_a, &self.party_b]
        }
    }
}

// ---------------------------------------------------------------------------
// 关系决策树 (v4 §4.3 扩展为 4 类)
// ---------------------------------------------------------------------------

/// 关系决策树输入 — 判断两个主体 A, B 的关系类型.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationDecision {
    /// A 没了 B 也不能活 → Symbiosis.
    ALosesBDies,
    /// A 是 B 的内部子机制 → Embedding.
    AIsInnerOfB,
    /// A == B (同一主体) → SelfRelation.
    AEqualsB,
    /// 默认 → Coordination.
    Default,
}

/// 关系决策树 (v4 §4.3 扩展).
///
/// 判断规则 (按优先级):
/// 1. A == B → SelfRelation (主体连续性)
/// 2. A 没了 B 也不能活 → Symbiosis
/// 3. A 是 B 的内部子机制 → Embedding
/// 4. 默认 → Coordination
pub fn classify(decision: RelationDecision) -> RelationKind {
    match decision {
        RelationDecision::AEqualsB => RelationKind::SelfRelation,
        RelationDecision::ALosesBDies => RelationKind::Symbiosis,
        RelationDecision::AIsInnerOfB => RelationKind::Embedding,
        RelationDecision::Default => RelationKind::Coordination,
    }
}

/// 便捷分类: 给定 party_a, party_b, 自动判断.
pub fn classify_pair(party_a: &str, party_b: &str) -> RelationKind {
    if party_a.is_empty() || party_b.is_empty() {
        // 空 id 是 Coordination (无法判断更具体 — 安全降级)
        return RelationKind::Coordination;
    }
    if party_a == party_b {
        return RelationKind::SelfRelation;
    }
    // 默认: Coordination (无法在零信息下判断共生/嵌入)
    RelationKind::Coordination
}

/// 关系注册表 — 持有多个关系, 支持按主体查询.
#[derive(Debug, Default, Clone)]
pub struct RelationRegistry {
    /// 关系列表.
    relations: Vec<Relation>,
}

impl RelationRegistry {
    /// 新建空注册表.
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册关系.
    pub fn register(&mut self, relation: Relation) {
        self.relations.push(relation);
    }

    /// 关系总数.
    pub fn len(&self) -> usize {
        self.relations.len()
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.relations.is_empty()
    }

    /// 按主体查询 (任一 party 匹配).
    pub fn find_by_party(&self, party_id: &str) -> Vec<&Relation> {
        self.relations
            .iter()
            .filter(|r| r.party_a == party_id || r.party_b == party_id)
            .collect()
    }

    /// 按关系类型统计.
    pub fn count_by_kind(&self, kind: RelationKind) -> usize {
        self.relations.iter().filter(|r| r.kind == kind).count()
    }

    /// 全部关系.
    pub fn all(&self) -> &[Relation] {
        &self.relations
    }
}

// ---------------------------------------------------------------------------
// 单元测试 (≥ 6)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kinds_have_semantic_name_and_describe() {
        for k in RelationKind::ALL {
            assert!(!k.semantic_name().is_empty());
            assert!(!k.describe().is_empty());
        }
        assert_eq!(RelationKind::ALL.len(), 4);
        assert!(RelationKind::SelfRelation.is_binary() == false);
        assert!(RelationKind::Symbiosis.is_binary());
        assert!(RelationKind::Coordination.is_binary());
        assert!(RelationKind::Embedding.is_binary());
    }

    #[test]
    fn new_symbiosis_requires_two_parties() {
        let r = Relation::new_symbiosis("perception", "cognition").unwrap();
        assert_eq!(r.kind, RelationKind::Symbiosis);
        assert_eq!(r.party_a, "perception");
        assert_eq!(r.party_b, "cognition");
        assert_eq!(r.involved_parties().len(), 2);
    }

    #[test]
    fn new_self_relation_requires_same_party() {
        let r = Relation::new_self_relation("cid-self-1").unwrap();
        assert_eq!(r.kind, RelationKind::SelfRelation);
        assert_eq!(r.party_a, "cid-self-1");
        assert_eq!(r.party_b, "cid-self-1");
        assert!(r.is_self_relation());
        assert_eq!(r.involved_parties(), vec!["cid-self-1"]);
    }

    #[test]
    fn embedding_rejects_self_loop() {
        let err = Relation::new_embedding("agent", "agent").unwrap_err();
        assert!(matches!(err, RelationError::EmbeddingSelfLoop));
    }

    #[test]
    fn empty_party_id_rejected() {
        assert!(matches!(
            Relation::new_symbiosis("", "x").unwrap_err(),
            RelationError::MissingPartyId(_)
        ));
        assert!(matches!(
            Relation::new_coordination("x", "").unwrap_err(),
            RelationError::MissingPartyId(_)
        ));
        assert!(matches!(
            Relation::new_embedding("", "x").unwrap_err(),
            RelationError::MissingPartyId(_)
        ));
        assert!(matches!(
            Relation::new_self_relation("").unwrap_err(),
            RelationError::MissingPartyId(_)
        ));
    }

    #[test]
    fn classify_decision_tree_priority() {
        // 优先级 1: AEqualsB → SelfRelation (即使其它条件也满足)
        assert_eq!(
            classify(RelationDecision::AEqualsB),
            RelationKind::SelfRelation
        );
        assert_eq!(
            classify(RelationDecision::ALosesBDies),
            RelationKind::Symbiosis
        );
        assert_eq!(
            classify(RelationDecision::AIsInnerOfB),
            RelationKind::Embedding
        );
        assert_eq!(
            classify(RelationDecision::Default),
            RelationKind::Coordination
        );
    }

    #[test]
    fn classify_pair_handles_same_party() {
        assert_eq!(classify_pair("cid-x", "cid-x"), RelationKind::SelfRelation);
        assert_eq!(classify_pair("a", "b"), RelationKind::Coordination);
        assert_eq!(classify_pair("", "b"), RelationKind::Coordination);
        assert_eq!(classify_pair("a", ""), RelationKind::Coordination);
    }

    #[test]
    fn registry_supports_query() {
        let mut reg = RelationRegistry::new();
        assert!(reg.is_empty());
        reg.register(Relation::new_symbiosis("perception", "cognition").unwrap());
        reg.register(Relation::new_coordination("constraint", "evolution").unwrap());
        reg.register(Relation::new_embedding("user", "agent").unwrap());
        reg.register(Relation::new_self_relation("cid-main").unwrap());
        assert_eq!(reg.len(), 4);
        assert_eq!(reg.count_by_kind(RelationKind::Symbiosis), 1);
        assert_eq!(reg.count_by_kind(RelationKind::Coordination), 1);
        assert_eq!(reg.count_by_kind(RelationKind::Embedding), 1);
        assert_eq!(reg.count_by_kind(RelationKind::SelfRelation), 1);
        let perc = reg.find_by_party("perception");
        assert_eq!(perc.len(), 1);
        assert_eq!(perc[0].kind, RelationKind::Symbiosis);
    }
}

// R214: 高级图算法 (pathfinding / cycle detect / topological sort / connected components)
pub mod pathfinding;
