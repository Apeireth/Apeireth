//! P-arch (2026-08-28): B1 Experience trait and conservative extraction
//! re-export (O-6 重构批次 Refactor-2).
//!
//! **O-6 重构**: trait 抽象层搬到 `apeireth-plugin` (foundation), impl 留本 crate (engine).
//! 单向依赖: memory → plugin. Storage implementations stay in this engine;
//! extraction types and the deterministic extractor stay in the foundation
//! plugin so runtime code does not depend on SQL.
//!
//! **v1 compat**: `apeireth_memory::experience::WikiEntry` 仍可访问 (re-export),
//! Existing v1 callers can continue using the re-exported paths.
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
