//! P-arch (2026-08-27): B1 Experience trait 0 装接口 (O-6 重构批次 Refactor-2).
//!
//! **O-6 重构**: trait 抽象层搬到 `apeireth-plugin` (foundation), impl 留本 crate (engine).
//! 单向依赖: memory → plugin. `ExperienceError` / `ExperienceResult` 仍在 plugin (与 `MemoryBackendError`
//! 一样的占位模式, rc 阶段统一).
//!
//! **v1 compat**: `apeireth_memory::experience::WikiEntry` 仍可访问 (re-export),
//! 现有 3 个内部测试 + 0 外部 user 0 破坏.
//!
//! The conservative extractor is defined in the foundation plugin so the
//! runtime can materialize evidence-bound artifacts without importing SQL.

// Trait 在 plugin (P-arch 2026-08-27 O-6 重构); 这里 re-export 保持 v1 兼容路径
pub use apeireth_plugin::experience::{
    extract_experience, extract_experience_from_episode, AssociationEdge, AssociationNode,
    AssociationObservation, AssociationStore, ExperienceArtifacts, ExperienceExtraction,
    ExtractedAssociation, ExtractedFact, ExtractedLink, ExtractedWikiEntry, GraphFact, GraphLink,
    KnowledgeGraphStore, WikiEntry, WikiEntryStore,
};
