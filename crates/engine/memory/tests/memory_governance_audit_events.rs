//! Contract of the memory-governance audit-event feed.
//!
//! The governance surface emits normalized audit events after each operation
//! completes. Emission is observation only: it never changes the operation's
//! outcome. What is proved here is the emit contract the runtime invariant
//! stream relies on:
//!
//! - an applied forget emits one forget event plus one forget-attributable
//!   state-change record;
//! - a repeat forget emits a forget event and no new state-change record;
//! - protect / unprotect emit markers and state-change records;
//! - a conforming governance stream is silent under the first-batch invariants.

use std::sync::{Arc, Mutex};

use apeireth_memory::{
    MemoryCoordinator, MemoryGovernanceStore, MemoryWritebackEntry, SqliteMemoryStore,
};
use apeireth_orchestration::runtime_invariants::{
    auditor_sink, first_batch_auditor, AuditEvent, AuditEventKind, AuditEventSink, InvariantMode,
};
use tempfile::tempdir;

fn coordinator_with(path: &std::path::Path, sink: AuditEventSink) -> MemoryCoordinator {
    let store = Arc::new(SqliteMemoryStore::open(path).expect("open SQLite file"));
    let backend = store.clone();
    let governance: Arc<dyn MemoryGovernanceStore> = store;
    MemoryCoordinator::new(backend, governance).with_audit_events(sink)
}

fn capturing_sink() -> (AuditEventSink, Arc<Mutex<Vec<AuditEvent>>>) {
    let events: Arc<Mutex<Vec<AuditEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let events = Arc::clone(&events);
        Arc::new(move |event: AuditEvent| {
            events.lock().unwrap().push(event);
        })
    };
    (sink, events)
}

fn kinds(events: &[AuditEvent]) -> Vec<AuditEventKind> {
    events.iter().map(|event| event.kind).collect()
}

#[test]
fn a_successful_forget_emits_one_forget_and_one_state_change() {
    let dir = tempdir().expect("temporary directory");
    let (sink, events) = capturing_sink();
    let coordinator = coordinator_with(&dir.path().join("memory.sqlite"), sink);

    let episode = coordinator
        .writeback(&MemoryWritebackEntry::new(
            "session",
            "user",
            "to be forgotten",
        ))
        .expect("writeback");
    coordinator
        .forget_episode(&episode, Some("test"), 0)
        .expect("first forget applies");

    let recorded = events.lock().unwrap().clone();
    assert_eq!(
        kinds(&recorded),
        vec![AuditEventKind::Forget, AuditEventKind::StateChange],
        "an applied forget emits exactly one forget and one state change: {recorded:?}"
    );
    assert_eq!(recorded[0].subject, episode);
    assert_eq!(
        recorded[1].correlation,
        format!("forget:{episode}"),
        "the state-change record is attributable to the forget of the target"
    );
}

#[test]
fn a_repeat_forget_emits_a_forget_without_a_new_state_change() {
    let dir = tempdir().expect("temporary directory");
    let (sink, events) = capturing_sink();
    let coordinator = coordinator_with(&dir.path().join("memory.sqlite"), sink);

    let episode = coordinator
        .writeback(&MemoryWritebackEntry::new(
            "session",
            "user",
            "to be forgotten",
        ))
        .expect("writeback");
    coordinator
        .forget_episode(&episode, Some("test"), 0)
        .expect("first forget applies");
    coordinator
        .forget_episode(&episode, Some("test again"), 1)
        .expect_err("a repeat forget reports already-forgotten");

    let recorded = events.lock().unwrap().clone();
    assert_eq!(
        kinds(&recorded),
        vec![
            AuditEventKind::Forget,
            AuditEventKind::StateChange,
            AuditEventKind::Forget,
        ],
        "the repeat is observable as a forget event with no new state change: {recorded:?}"
    );
}

#[test]
fn protect_and_unprotect_emit_markers_and_state_changes() {
    let dir = tempdir().expect("temporary directory");
    let (sink, events) = capturing_sink();
    let coordinator = coordinator_with(&dir.path().join("memory.sqlite"), sink);

    let episode = coordinator
        .writeback(&MemoryWritebackEntry::new("session", "user", "kept"))
        .expect("writeback");
    coordinator.protect_episode(&episode, 0).expect("protect");
    coordinator
        .unprotect_episode(&episode, 1)
        .expect("unprotect");

    let recorded = events.lock().unwrap().clone();
    assert_eq!(
        kinds(&recorded),
        vec![
            AuditEventKind::Protect,
            AuditEventKind::StateChange,
            AuditEventKind::Unprotect,
            AuditEventKind::StateChange,
        ],
        "{recorded:?}"
    );
    assert_eq!(recorded[1].correlation, "protect");
    assert_eq!(recorded[3].correlation, "unprotect");
}

#[test]
fn a_conforming_governance_stream_is_silent_under_the_first_batch() {
    let dir = tempdir().expect("temporary directory");
    let auditor = Arc::new(first_batch_auditor(InvariantMode::LogOnly));
    let coordinator = coordinator_with(
        &dir.path().join("memory.sqlite"),
        auditor_sink(Arc::clone(&auditor)),
    );

    let forgotten = coordinator
        .writeback(&MemoryWritebackEntry::new("session", "user", "gone"))
        .expect("writeback");
    let kept = coordinator
        .writeback(&MemoryWritebackEntry::new("session", "user", "kept"))
        .expect("writeback");

    coordinator
        .forget_episode(&forgotten, Some("test"), 0)
        .expect("forget applies");
    let _ = coordinator.forget_episode(&forgotten, Some("test"), 1);
    coordinator.protect_episode(&kept, 0).expect("protect");
    coordinator.unprotect_episode(&kept, 1).expect("unprotect");

    assert!(
        auditor.recorded().is_empty(),
        "a conforming governance stream must produce zero violations: {:?}",
        auditor.recorded()
    );
}
