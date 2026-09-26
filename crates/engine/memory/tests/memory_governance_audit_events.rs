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
//! - the cleanup execution point feeds one cleanup event per subject entering
//!   the cleanup stream, so `inv_sweep_keeps_protected` guards live cleanup
//!   traffic;
//! - a conforming governance stream is silent under the first-batch invariants.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::{
    EpisodeStore, MemoryCoordinator, MemoryGovernanceStore, MemoryWritebackEntry, RetentionCleanup,
    RetentionPolicy, SqliteMemoryStore,
};
use apeireth_orchestration::runtime_invariants::{
    auditor_sink, first_batch_auditor, AuditEvent, AuditEventKind, AuditEventSink, InvariantMode,
    INV_SWEEP_KEEPS_PROTECTED,
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

fn put_episode(store: &SqliteMemoryStore, id: &str, session: &str, ts: i64) {
    <SqliteMemoryStore as EpisodeStore>::put_episode(
        store,
        &Episode {
            id: id.into(),
            timestamp: ts,
            role: "user".into(),
            content: format!("body-{id}"),
            session_id: session.into(),
        },
    )
    .expect("put episode");
}

/// 清理事件入流: every subject the cleanup execution point commits to cleaning
/// feeds one `AuditEvent::cleanup` into the shared stream — module, subject,
/// and cause — while kept subjects emit nothing.
#[test]
fn cleanup_events_enter_the_stream_at_the_cleanup_execution_point() {
    let store = SqliteMemoryStore::open_in_memory().expect("in-memory store");
    put_episode(&store, "old", "s", 10);
    put_episode(&store, "fresh", "s", 900);
    let (sink, events) = capturing_sink();
    let cleanup = RetentionCleanup::new(Some(sink));

    let report = cleanup
        .run(
            &store,
            "s",
            &RetentionPolicy::default().with_max_age_secs(100),
            1_000,
        )
        .expect("cleanup pass");
    assert_eq!(report.forgotten, 1, "only the old subject is cleaned");

    let recorded = events.lock().unwrap().clone();
    assert_eq!(
        kinds(&recorded),
        vec![AuditEventKind::Cleanup],
        "one cleanup event per subject entering the cleanup stream: {recorded:?}"
    );
    assert_eq!(recorded[0].subject, "old");
    assert_eq!(recorded[0].module, "retention");
    assert_eq!(recorded[0].detail, "retention-sweep");
}

/// 保护互斥违例在线可检: with the cleanup feed and the governance feed sharing
/// one consumption point, a subject that carries a live protect marker in the
/// stream while entering the cleanup stream is flagged online by
/// `inv_sweep_keeps_protected`. The divergence exercised here is the one the
/// stream-level check exists for: the marker was cleared at the store level
/// through a surface that never fed the stream, so the stream still shows the
/// subject as protected when the sweep cleans it.
#[test]
fn a_protected_subject_entering_the_cleanup_stream_is_flagged_online() {
    let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("in-memory store"));
    let auditor = Arc::new(first_batch_auditor(InvariantMode::LogOnly));
    let sink = auditor_sink(Arc::clone(&auditor));
    put_episode(&store, "ep-1", "s", 10);

    let coordinator =
        MemoryCoordinator::new(store.clone(), store.clone()).with_audit_events(Arc::clone(&sink));
    coordinator.protect_episode("ep-1", 0).expect("protect");
    // Store-level clear with no stream marker: bookkeeping diverges.
    store
        .unprotect_episode("ep-1", 1)
        .expect("store-level clear");

    let cleanup = RetentionCleanup::new(Some(sink));
    let report = cleanup
        .run(
            store.as_ref(),
            "s",
            &RetentionPolicy::default().with_max_age_secs(100),
            1_000,
        )
        .expect("cleanup pass");
    assert_eq!(
        report.forgotten, 1,
        "the diverged store lets the sweep clean the subject"
    );

    let violations = auditor.recorded();
    assert!(
        violations
            .iter()
            .any(|v| v.invariant == INV_SWEEP_KEEPS_PROTECTED),
        "the protect/cleanup mutual exclusion must be flagged online: {violations:?}"
    );
}

/// 正常清理零噪音: a conforming cleanup pass — protected memory skipped before
/// the cleanup stream, unprotected memory cleaned normally — produces zero
/// violations while the cleanup events still enter the stream.
#[test]
fn a_normal_cleanup_pass_is_silent() {
    let store = Arc::new(SqliteMemoryStore::open_in_memory().expect("in-memory store"));
    let auditor = Arc::new(first_batch_auditor(InvariantMode::LogOnly));
    let sink = auditor_sink(Arc::clone(&auditor));
    put_episode(&store, "kept", "s", 10);
    put_episode(&store, "dropped", "s", 20);
    put_episode(&store, "fresh", "s", 900);

    let coordinator =
        MemoryCoordinator::new(store.clone(), store.clone()).with_audit_events(Arc::clone(&sink));
    coordinator.protect_episode("kept", 0).expect("protect");

    let cleanup = RetentionCleanup::new(Some(sink));
    let report = cleanup
        .run(
            store.as_ref(),
            "s",
            &RetentionPolicy::default().with_max_age_secs(100),
            1_000,
        )
        .expect("cleanup pass");

    assert_eq!(
        report.skipped_protected, 1,
        "protected memory never enters the cleanup stream"
    );
    assert_eq!(
        report.forgotten, 1,
        "the unprotected old subject is cleaned"
    );
    assert!(
        auditor.recorded().is_empty(),
        "a normal cleanup pass must produce zero violations: {:?}",
        auditor.recorded()
    );
}
