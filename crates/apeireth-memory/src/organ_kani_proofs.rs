//! R177 memory organ Kani proofs (W2)

#![allow(missing_docs)]

use apeireth_core::Episode;
use crate::{
    append_only::kind_from_str, EpisodeQuery, HistoryEntry, SHORT_TERM_WINDOW_SECS, StreamKind,
    Tombstone, WORKING_CAPACITY,
};

fn make_episode(id: &str, session_id: &str, role: &str, content: &str) -> Episode {
    Episode {
        id: id.to_string(),
        timestamp: 1_700_000_000,
        role: role.to_string(),
        content: content.to_string(),
        session_id: session_id.to_string(),
    }
}

#[test]
fn r177_mem_01_constants() {
    assert_eq!(WORKING_CAPACITY, 50);
    assert_eq!(SHORT_TERM_WINDOW_SECS, 24 * 3600);
}

#[test]
fn r177_mem_02_query_default() {
    let q = EpisodeQuery::new();
    assert!(q.session_id.is_none());
    assert!(q.continuity_id.is_none());
    assert!(q.since.is_none());
    assert!(q.until.is_none());
    assert!(q.role.is_none());
    assert!(q.limit.is_none());
}

#[test]
fn r177_mem_03_query_chain() {
    let q = EpisodeQuery::new()
        .for_session("sess-1")
        .for_continuity("cid-1")
        .in_range(Some(100), Some(200))
        .with_role("user")
        .limit(10);
    assert_eq!(q.session_id, Some("sess-1".to_string()));
    assert_eq!(q.continuity_id, Some("cid-1".to_string()));
    assert_eq!(q.since, Some(100));
    assert_eq!(q.until, Some(200));
    assert_eq!(q.role, Some("user".to_string()));
    assert_eq!(q.limit, Some(10));
}

#[test]
fn r177_mem_04_episode_basic() {
    let ep = make_episode("id-1", "sess-1", "user", "hello");
    assert_eq!(ep.id, "id-1");
    assert_eq!(ep.session_id, "sess-1");
    assert_eq!(ep.role, "user");
    assert_eq!(ep.content, "hello");
    assert!(ep.timestamp > 0);
}

#[test]
fn r177_mem_05_stream_kind_parse() {
    assert!(kind_from_str("thought").is_ok());
    assert!(kind_from_str("proposal").is_ok());
    assert!(kind_from_str("action").is_ok());
    assert!(kind_from_str("relation").is_ok());
    assert!(kind_from_str("evolution").is_ok());
    assert!(kind_from_str("reflection").is_ok());
}

#[test]
fn r177_mem_06_tombstone_structure() {
    let t = Tombstone {
        id: "t-1".to_string(),
        tombstoned_at: 1_700_000_000,
        reason: "test".to_string(),
    };
    assert_eq!(t.id, "t-1");
    assert!(t.tombstoned_at > 0);
}

#[test]
fn r177_mem_07_history_entry() {
    let entry = HistoryEntry {
        id: "e-1".to_string(),
        subject_id: "sub-1".to_string(),
        subject_rev: 1,
        session_id: Some("s-1".to_string()),
        created_at: 1_700_000_000,
        payload: serde_json::json!({"k": "v"}),
        source: "ai_generated".to_string(),
        tags: vec!["tag1".to_string()],
        tombstoned_at: None,
    };
    assert_eq!(entry.id, "e-1");
    assert!(entry.tombstoned_at.is_none());
    assert_eq!(entry.subject_rev, 1);
}

#[test]
fn r177_mem_08_short_term_window_24h() {
    assert_eq!(SHORT_TERM_WINDOW_SECS, 86400);
}

#[test]
fn r177_mem_09_working_capacity_positive() {
    assert!(WORKING_CAPACITY > 0);
    assert!(WORKING_CAPACITY < 10_000);
}

#[test]
fn r177_mem_10_query_chain_idempotent() {
    let q1 = EpisodeQuery::new().for_session("s").limit(5);
    let q2 = q1.clone().for_continuity("c");
    assert_eq!(q1.session_id, Some("s".to_string()));
    assert_eq!(q1.limit, Some(5));
    assert_eq!(q2.continuity_id, Some("c".to_string()));
}

#[test]
fn r177_mem_11_stream_kind_table_names() {
    assert_eq!(StreamKind::Thought.table_name(), "thought_stream");
    assert_eq!(StreamKind::Proposal.table_name(), "proposal_stream");
    assert_eq!(StreamKind::Action.table_name(), "action_stream");
    assert_eq!(StreamKind::Relation.table_name(), "relation_stream");
    assert_eq!(StreamKind::Evolution.table_name(), "evolution_stream");
    assert_eq!(StreamKind::Reflection.table_name(), "reflection_stream");
}

#[cfg(kani)]
#[kani::proof]
fn r177_mem_kani_01_constants_invariants() {
    assert!(WORKING_CAPACITY > 0);
    assert!(SHORT_TERM_WINDOW_SECS > 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_mem_kani_02_query_default_all_none() {
    let q = EpisodeQuery::new();
    assert!(q.session_id.is_none());
    assert!(q.continuity_id.is_none());
    assert!(q.since.is_none());
    assert!(q.until.is_none());
    assert!(q.role.is_none());
    assert!(q.limit.is_none());
}
