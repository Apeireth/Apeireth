//! `apeireth-graph-primitive` — relation subsystem + property graph.
//!
//! v1 API surface preserved: `RelationKind`, `Relation`, `RelationError`,
//! `RelationDecision`, `RelationRegistry`, `classify`, `classify_pair`,
//! `RelationGraph`, `NodeId`, `EdgeId`, `GraphNode`, `GraphEdge`,
//! `NodeQuery`, `EdgeQuery`, `CombinedQuery`, `PropertyMatch`, `count_by_kind`,
//! `BfsIter`, `DfsIter`, `TraversalDirection`, `PathResult`, `shortest_path`,
//! `pathfinding::find_path`, `pathfinding::topological_sort`,
//! `pathfinding::cycle_detect`, `pathfinding::connected_components`.

#![deny(unsafe_code)]

pub mod graph;
pub mod pathfinding;
pub mod query;
pub mod traversal;

pub use graph::{EdgeId, GraphEdge, GraphNode, NodeId, RelationGraph};
pub use pathfinding::{
    cycle_detect, find_path, topological_sort, connected_components,
};
pub use query::{count_by_kind, CombinedQuery, EdgeQuery, NodeQuery, PropertyMatch};
pub use traversal::{shortest_path, BfsIter, DfsIter, PathResult, TraversalDirection};

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

/// 4-class relation enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RelationKind {
    Symbiosis,
    Coordination,
    Embedding,
    SelfRelation,
}

impl RelationKind {
    pub const ALL: [RelationKind; 4] = [
        RelationKind::Symbiosis,
        RelationKind::Coordination,
        RelationKind::Embedding,
        RelationKind::SelfRelation,
    ];

    pub const fn semantic_name(self) -> &'static str {
        match self {
            RelationKind::Symbiosis => "symbiosis",
            RelationKind::Coordination => "coordination",
            RelationKind::Embedding => "embedding",
            RelationKind::SelfRelation => "self_relation",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            RelationKind::Symbiosis => "Symbiosis",
            RelationKind::Coordination => "Coordination",
            RelationKind::Embedding => "Embedding",
            RelationKind::SelfRelation => "SelfRelation",
        }
    }

    pub const fn is_binary(self) -> bool {
        !matches!(self, RelationKind::SelfRelation)
    }
}

/// Relation instance.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Relation {
    pub id: Uuid,
    pub kind: RelationKind,
    pub party_a: String,
    pub party_b: String,
    pub established_at: DateTime<Utc>,
    pub note: Option<String>,
}

/// Relation errors.
#[derive(Debug, Error)]
pub enum RelationError {
    #[error("party id missing: {0}")]
    MissingPartyId(String),
    #[error("self_relation requires party_a == party_b")]
    SelfRelationMismatch { a: String, b: String },
    #[error("embedding requires party_a != party_b")]
    EmbeddingSelfLoop,
}

impl Relation {
    pub fn new_symbiosis(party_a: impl Into<String>, party_b: impl Into<String>) -> Result<Self, RelationError> {
        let (a, b) = (party_a.into(), party_b.into());
        if a.is_empty() || b.is_empty() {
            return Err(RelationError::MissingPartyId(if a.is_empty() { "party_a".into() } else { "party_b".into() }));
        }
        Ok(Self::build(RelationKind::Symbiosis, a, b, None))
    }

    pub fn new_coordination(party_a: impl Into<String>, party_b: impl Into<String>) -> Result<Self, RelationError> {
        let (a, b) = (party_a.into(), party_b.into());
        if a.is_empty() || b.is_empty() {
            return Err(RelationError::MissingPartyId(if a.is_empty() { "party_a".into() } else { "party_b".into() }));
        }
        Ok(Self::build(RelationKind::Coordination, a, b, None))
    }

    pub fn new_embedding(host: impl Into<String>, inner: impl Into<String>) -> Result<Self, RelationError> {
        let (h, i) = (host.into(), inner.into());
        if h.is_empty() || i.is_empty() {
            return Err(RelationError::MissingPartyId(if h.is_empty() { "host".into() } else { "inner".into() }));
        }
        if h == i { return Err(RelationError::EmbeddingSelfLoop); }
        Ok(Self::build(RelationKind::Embedding, h, i, None))
    }

    pub fn new_self_relation(continuity_id: impl Into<String>) -> Result<Self, RelationError> {
        let cid = continuity_id.into();
        if cid.is_empty() { return Err(RelationError::MissingPartyId("continuity_id".into())); }
        Ok(Self::build(RelationKind::SelfRelation, cid.clone(), cid, None))
    }

    fn build(kind: RelationKind, a: String, b: String, note: Option<String>) -> Self {
        Self { id: Uuid::new_v4(), kind, party_a: a, party_b: b, established_at: Utc::now(), note }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into()); self
    }

    pub fn is_self_relation(&self) -> bool { self.kind == RelationKind::SelfRelation }
    pub fn is_embedding(&self) -> bool { self.kind == RelationKind::Embedding }

    pub fn involved_parties(&self) -> Vec<&str> {
        if self.is_self_relation() { vec![&self.party_a] }
        else if self.party_a == self.party_b { vec![&self.party_a] }
        else { vec![&self.party_a, &self.party_b] }
    }
}

/// Relation decision tree input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationDecision {
    ALosesBDies,
    AIsInnerOfB,
    AEqualsB,
    Default,
}

pub fn classify(decision: RelationDecision) -> RelationKind {
    match decision {
        RelationDecision::AEqualsB => RelationKind::SelfRelation,
        RelationDecision::ALosesBDies => RelationKind::Symbiosis,
        RelationDecision::AIsInnerOfB => RelationKind::Embedding,
        RelationDecision::Default => RelationKind::Coordination,
    }
}

pub fn classify_pair(party_a: &str, party_b: &str) -> RelationKind {
    if party_a.is_empty() || party_b.is_empty() { return RelationKind::Coordination; }
    if party_a == party_b { return RelationKind::SelfRelation; }
    RelationKind::Coordination
}

/// Relation registry.
#[derive(Debug, Default, Clone)]
pub struct RelationRegistry {
    relations: Vec<Relation>,
}

impl RelationRegistry {
    pub fn new() -> Self { Self::default() }
    pub fn register(&mut self, relation: Relation) { self.relations.push(relation); }
    pub fn len(&self) -> usize { self.relations.len() }
    pub fn is_empty(&self) -> bool { self.relations.is_empty() }
    pub fn find_by_party(&self, party_id: &str) -> Vec<&Relation> {
        self.relations.iter().filter(|r| r.party_a == party_id || r.party_b == party_id).collect()
    }
    pub fn count_by_kind(&self, kind: RelationKind) -> usize {
        self.relations.iter().filter(|r| r.kind == kind).count()
    }
    pub fn all(&self) -> &[Relation] { &self.relations }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kinds_metadata() {
        for k in RelationKind::ALL {
            assert!(!k.semantic_name().is_empty());
            assert!(!k.describe().is_empty());
        }
        assert!(!RelationKind::SelfRelation.is_binary());
        assert!(RelationKind::Symbiosis.is_binary());
    }

    #[test]
    fn new_symbiosis_works() {
        let r = Relation::new_symbiosis("a", "b").unwrap();
        assert_eq!(r.kind, RelationKind::Symbiosis);
        assert_eq!(r.involved_parties().len(), 2);
    }

    #[test]
    fn new_self_relation_works() {
        let r = Relation::new_self_relation("cid").unwrap();
        assert_eq!(r.party_a, "cid");
        assert_eq!(r.party_b, "cid");
        assert!(r.is_self_relation());
    }

    #[test]
    fn embedding_rejects_self_loop() {
        assert!(matches!(Relation::new_embedding("x", "x"), Err(RelationError::EmbeddingSelfLoop)));
    }

    #[test]
    fn empty_party_rejected() {
        assert!(Relation::new_symbiosis("", "x").is_err());
        assert!(Relation::new_coordination("x", "").is_err());
        assert!(Relation::new_embedding("", "x").is_err());
        assert!(Relation::new_self_relation("").is_err());
    }

    #[test]
    fn classify_priority() {
        assert_eq!(classify(RelationDecision::AEqualsB), RelationKind::SelfRelation);
        assert_eq!(classify(RelationDecision::ALosesBDies), RelationKind::Symbiosis);
        assert_eq!(classify(RelationDecision::AIsInnerOfB), RelationKind::Embedding);
        assert_eq!(classify(RelationDecision::Default), RelationKind::Coordination);
    }

    #[test]
    fn classify_pair_handles_same() {
        assert_eq!(classify_pair("a", "a"), RelationKind::SelfRelation);
        assert_eq!(classify_pair("a", "b"), RelationKind::Coordination);
    }

    #[test]
    fn registry_queries() {
        let mut reg = RelationRegistry::new();
        reg.register(Relation::new_symbiosis("a", "b").unwrap());
        reg.register(Relation::new_coordination("c", "d").unwrap());
        reg.register(Relation::new_embedding("e", "f").unwrap());
        reg.register(Relation::new_self_relation("g").unwrap());
        assert_eq!(reg.count_by_kind(RelationKind::Symbiosis), 1);
        assert_eq!(reg.find_by_party("a").len(), 1);
    }
}
