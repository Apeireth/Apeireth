//! Real SQLite transactions, temporal history and replay for validated plans.
//! Semantic/source authority is tested at the Runtime assembly boundary.

use std::sync::Arc;

use apeireth_core::Episode;
use apeireth_memory::automatic_correction::{
    correction_evidence_id, correction_history_id, correction_operation_id,
    correction_relation_key, AutomaticCorrectionKind, AutomaticCorrectionPlan,
    AutomaticCorrectionTarget,
};
use apeireth_memory::writeback_recovery::{
    assert_operation_allowed, operation_id, payload_hash, PreparedWritebackPlan, RecoveryJob,
    SqliteWritebackJournal, WritebackOperation,
};
use apeireth_memory::{
    EpisodeStore, MemoryCoordinator, MemoryGovernanceStore, MemoryProvenance, MemoryScope,
    SqliteMemoryStore, SqliteTemporalGraphStore, TemporalGraphQuery,
};
use apeireth_storage::SqliteConnectionPool;
use rusqlite::OptionalExtension;
use serde_json::json;
use tempfile::{tempdir, TempDir};

const OLD: &str = "我住在武汉。";
const NEW: &str = "我住在上海。";
const NAMESPACE: &str = "correction-owner";
const OLD_MS: i64 = 1_700_000_000_000;
const NEW_MS: i64 = OLD_MS + 172_800_000;

fn fixture() -> (TempDir, Arc<SqliteMemoryStore>, AutomaticCorrectionPlan) {
    let dir = tempdir().unwrap();
    let store = Arc::new(SqliteMemoryStore::open(dir.path().join("memory.sqlite3")).unwrap());
    let coordinator = MemoryCoordinator::new(store.clone(), store.clone());
    coordinator
        .writeback_scoped_episode(
            &Episode {
                id: "original-user-fact".into(),
                timestamp: OLD_MS / 1000,
                role: "user".into(),
                content: OLD.into(),
                session_id: NAMESPACE.into(),
            },
            &MemoryScope::session(NAMESPACE),
            &MemoryProvenance {
                source: "cognitive.memory_writeback".into(),
                source_session: Some("old-session".into()),
                source_request: Some("old-request".into()),
                source_trace: Some("old-trace".into()),
            },
        )
        .unwrap();
    let plan = AutomaticCorrectionPlan {
        operation_id: correction_operation_id(NAMESPACE, "new-request"),
        namespace: NAMESPACE.into(),
        evidence: Episode {
            id: correction_evidence_id(NAMESPACE, "new-request"),
            timestamp: NEW_MS / 1000,
            role: "user".into(),
            content: "我昨天搬到上海。".into(),
            session_id: NAMESPACE.into(),
        },
        provenance: MemoryProvenance {
            source: "direct_user_correction".into(),
            source_session: Some("new-session".into()),
            source_request: Some("new-request".into()),
            source_trace: Some("new-trace".into()),
        },
        authorization_epoch: Some("host-epoch".into()),
        kind: AutomaticCorrectionKind::StateChange,
        valid_from_ms: OLD_MS + 86_400_000,
        recorded_at_ms: NEW_MS,
        targets: vec![AutomaticCorrectionTarget {
            id: "original-user-fact".into(),
            expected_revision: 0,
            before_content: OLD.into(),
            replacement_content: NEW.into(),
            source_session: "old-session".into(),
            source_request: "old-request".into(),
        }],
    };
    (dir, store, plan)
}

#[tokio::test]
async fn atomic_correction_preserves_evidence_time_and_belief_history_after_restart() {
    let (dir, store, plan) = fixture();
    assert!(!store.apply_automatic_correction(&plan).unwrap().replayed);
    let governed = store.get_governed("original-user-fact").unwrap().unwrap();
    assert_eq!(governed.episode.content, NEW);
    assert_eq!(governed.revision, 1);
    assert_eq!(
        EpisodeStore::get_episode(store.as_ref(), "original-user-fact")
            .unwrap()
            .unwrap()
            .content,
        OLD
    );
    assert!(store.apply_automatic_correction(&plan).unwrap().replayed);
    assert_eq!(
        store
            .get_governed("original-user-fact")
            .unwrap()
            .unwrap()
            .revision,
        1
    );
    assert!(
        apeireth_memory::memory_chain::verify_subject_chain(&store.conn().unwrap(), NAMESPACE)
            .unwrap()
            .ok
    );
    drop(store);
    let store = SqliteMemoryStore::open(dir.path().join("memory.sqlite3")).unwrap();
    assert_eq!(
        store
            .get_governed("original-user-fact")
            .unwrap()
            .unwrap()
            .episode
            .content,
        NEW
    );
    let history_id = correction_history_id(NAMESPACE, "original-user-fact", &plan.operation_id);
    let history = store.get_governed(&history_id).unwrap().unwrap();
    assert_eq!(history.episode.role, "fact_history");
    assert_eq!(
        history.episode.content,
        format!(
            "历史状态（已结束，不代表当前），有效区间：[{}, {})：{OLD}",
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(OLD_MS)
                .unwrap()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            chrono::DateTime::<chrono::Utc>::from_timestamp_millis(plan.valid_from_ms)
                .unwrap()
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        )
    );
    let metadata: String = store
        .conn()
        .unwrap()
        .query_row(
            "SELECT metadata_json FROM episode_memory_metadata WHERE episode_id=?1",
            [&history_id],
            |r| r.get(0),
        )
        .unwrap();
    let metadata: serde_json::Value = serde_json::from_str(&metadata).unwrap();
    assert_eq!(metadata["derived_from_episode_id"], "original-user-fact");
    assert_eq!(metadata["correction_evidence_id"], plan.evidence.id);
    assert_eq!(
        metadata["scope"],
        serde_json::to_value(MemoryScope::session(NAMESPACE)).unwrap()
    );
    let temporal = SqliteTemporalGraphStore::open(dir.path().join("memory.sqlite3"))
        .await
        .unwrap();
    let relation = correction_relation_key(NAMESPACE, "original-user-fact");
    let before = temporal
        .query(
            &TemporalGraphQuery::new(OLD_MS + 1_000)
                .with_belief_as_of_ms(NEW_MS + 1_000)
                .relation(&relation),
        )
        .unwrap();
    assert_eq!(before[0].object_id, OLD);
    let after = temporal
        .query(
            &TemporalGraphQuery::new(NEW_MS)
                .with_belief_as_of_ms(NEW_MS + 1_000)
                .relation(&relation),
        )
        .unwrap();
    assert_eq!(after[0].object_id, NEW);
    let previously_believed = temporal
        .query(
            &TemporalGraphQuery::new(NEW_MS)
                .with_belief_as_of_ms(OLD_MS + 1_000)
                .relation(&relation),
        )
        .unwrap();
    assert_eq!(previously_believed[0].object_id, OLD);
    assert_eq!(temporal.history(&relation).unwrap().len(), 2);
}

#[test]
fn failed_correction_rolls_back_sidecar_history_source_chain_and_operation_receipt() {
    let (_dir, store, plan) = fixture();
    store.conn().unwrap().execute_batch("CREATE TRIGGER reject_correction BEFORE UPDATE OF content_override ON episode_governance BEGIN SELECT RAISE(ABORT,'controlled correction write fault'); END;").unwrap();
    let failure = store.apply_automatic_correction(&plan).unwrap_err();
    assert!(failure
        .to_string()
        .contains("controlled correction write fault"));
    assert_eq!(
        store
            .get_governed("original-user-fact")
            .unwrap()
            .unwrap()
            .episode
            .content,
        OLD
    );
    assert!(EpisodeStore::get_episode(store.as_ref(), &plan.evidence.id)
        .unwrap()
        .is_none());
    assert!(EpisodeStore::get_episode(
        store.as_ref(),
        &correction_history_id(NAMESPACE, "original-user-fact", &plan.operation_id)
    )
    .unwrap()
    .is_none());
    let conn = store.conn().unwrap();
    for table in ["temporal_graph_facts", "memory_recovery_operation_receipts"] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "partial correction state in {table}");
    }
    let head: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_chain_heads WHERE subject_id=?1",
            [NAMESPACE],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(head, 0);
    conn.execute_batch("DROP TRIGGER reject_correction")
        .unwrap();
    drop(conn);
    assert!(!store.apply_automatic_correction(&plan).unwrap().replayed);
}

#[test]
fn changed_scope_stale_revision_protection_and_forget_cannot_be_overridden() {
    for refusal in ["scope", "revision", "protected", "forgotten", "source"] {
        let (_dir, store, mut plan) = fixture();
        match refusal {
            "scope" => {
                store.conn().unwrap().execute("UPDATE episode_memory_metadata SET metadata_json=json_set(metadata_json,'$.scope.session_id','other-owner') WHERE episode_id='original-user-fact'",[]).unwrap();
            }
            "revision" => {
                plan.targets[0].expected_revision = 3;
            }
            "protected" => {
                store.protect_episode("original-user-fact", 0).unwrap();
                plan.targets[0].expected_revision = 1;
            }
            "forgotten" => {
                store
                    .forget_episode("original-user-fact", Some("owner"), 0)
                    .unwrap();
                plan.targets[0].expected_revision = 1;
            }
            "source" => {
                plan.targets[0].source_request = "forged-request".into();
            }
            _ => unreachable!(),
        }
        assert!(
            store.apply_automatic_correction(&plan).is_err(),
            "{refusal}"
        );
        assert!(
            EpisodeStore::get_episode(store.as_ref(), &plan.evidence.id)
                .unwrap()
                .is_none(),
            "{refusal}"
        );
        assert_eq!(
            EpisodeStore::get_episode(store.as_ref(), "original-user-fact")
                .unwrap()
                .unwrap()
                .content,
            OLD
        );
    }
}

#[test]
fn prior_error_is_distinct_from_a_later_state_change_and_replay_never_revives_forget() {
    let (_dir, store, mut plan) = fixture();
    plan.kind = AutomaticCorrectionKind::CorrectPrior;
    store.apply_automatic_correction(&plan).unwrap();
    let conn = store.conn().unwrap();
    let effective: i64 = conn
        .query_row(
            "SELECT valid_from_ms FROM temporal_graph_facts WHERE supersedes_id IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(effective, OLD_MS);
    let histories: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episodes WHERE role='fact_history'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        histories, 0,
        "a corrected mistake must never become a once-true historical fact"
    );
    drop(conn);
    store
        .forget_episode("original-user-fact", Some("owner"), 1)
        .unwrap();
    assert!(store.apply_automatic_correction(&plan).unwrap().replayed);
    assert_eq!(
        store
            .get_governed("original-user-fact")
            .unwrap()
            .unwrap()
            .status
            .as_str(),
        "forgotten"
    );
    plan.targets[0].replacement_content = "我住在广州。".into();
    assert!(
        store.apply_automatic_correction(&plan).is_err(),
        "same operation cannot change its frozen payload"
    );
}

struct AliasFixture {
    dir: TempDir,
    store: Arc<SqliteMemoryStore>,
    plan: AutomaticCorrectionPlan,
    journal: Arc<SqliteWritebackJournal>,
    old_job: String,
    current_job: String,
    pending_projection: Episode,
}

fn old_provenance() -> MemoryProvenance {
    MemoryProvenance {
        source: "cognitive.memory_writeback".into(),
        source_session: Some("old-session".into()),
        source_request: Some("old-request".into()),
        source_trace: Some("old-trace".into()),
    }
}

fn register_plan(
    journal: &SqliteWritebackJournal,
    id: &str,
    request: &str,
    source: serde_json::Value,
    plan: &PreparedWritebackPlan,
    at: i64,
) {
    let job = RecoveryJob {
        id: id.into(),
        namespace: NAMESPACE.into(),
        source_session: if request == "old-request" {
            "old-session"
        } else {
            "new-session"
        }
        .into(),
        source_request: request.into(),
        source_hash: payload_hash(&source).unwrap(),
        source,
        created_ms: at,
        plan: None,
        next_operation: 0,
        state: "pending".into(),
        attempts: 0,
        last_error: None,
    };
    journal
        .register_page(NAMESPACE, journal.cursor(NAMESPACE).unwrap(), &[job], None)
        .unwrap();
    journal.save_plan(id, plan).unwrap();
}

async fn alias_fixture(reverse: bool) -> AliasFixture {
    let (dir, store, mut plan) = fixture();
    let coordinator = MemoryCoordinator::new(store.clone(), store.clone());
    let alias = Episode {
        id: "original-memory-alias".into(),
        timestamp: OLD_MS / 1000,
        role: "memory".into(),
        content: OLD.into(),
        session_id: NAMESPACE.into(),
    };
    let extra = json!({"derived_from_episode_id":"original-user-fact","extraction_class":"Fact"});
    coordinator
        .writeback_scoped_episode_with_metadata(
            &alias,
            &MemoryScope::session(NAMESPACE),
            &old_provenance(),
            extra.clone(),
        )
        .unwrap();
    plan.targets.push(AutomaticCorrectionTarget {
        id: alias.id.clone(),
        expected_revision: 0,
        before_content: OLD.into(),
        replacement_content: NEW.into(),
        source_session: "old-session".into(),
        source_request: "old-request".into(),
    });
    if reverse {
        plan.targets.reverse();
    }
    let pool = Arc::new(
        SqliteConnectionPool::open(dir.path().join("memory.sqlite3"))
            .await
            .unwrap(),
    );
    let journal = Arc::new(SqliteWritebackJournal::from_pool(pool));
    let pending_projection = Episode {
        id: "not-yet-materialized-old-projection".into(),
        ..alias.clone()
    };
    // The original raw evidence and one projection have committed, while a
    // later projection remains pending. No Runtime/source authorization is
    // mocked here: this fixture isolates the real SQLite governance/journal.
    let old_plan = PreparedWritebackPlan {
        operations: vec![
            WritebackOperation::Episode {
                episode: EpisodeStore::get_episode(store.as_ref(), "original-user-fact")
                    .unwrap()
                    .unwrap(),
                scope: MemoryScope::session(NAMESPACE),
                provenance: old_provenance(),
            },
            WritebackOperation::DerivedEpisode {
                episode: alias,
                scope: MemoryScope::session(NAMESPACE),
                provenance: old_provenance(),
                fingerprint: "existing-alias".into(),
                metadata: extra.clone(),
            },
            WritebackOperation::DerivedEpisode {
                episode: pending_projection.clone(),
                scope: MemoryScope::session(NAMESPACE),
                provenance: old_provenance(),
                fingerprint: "pending-projection".into(),
                metadata: extra,
            },
        ],
        source_episode_ids: vec![
            "original-user-fact".into(),
            "original-memory-alias".into(),
            pending_projection.id.clone(),
        ],
    };
    let old_job = "writeback/old-request".to_owned();
    register_plan(
        &journal,
        &old_job,
        "old-request",
        json!({"fixture":"already-accepted-old-source"}),
        &old_plan,
        OLD_MS,
    );
    journal.advance(&old_job, 0, 3, OLD_MS).unwrap();
    journal.advance(&old_job, 1, 3, OLD_MS).unwrap();
    let current_job = format!("correction/{}", plan.operation_id);
    let mut dependencies = vec![plan.evidence.id.clone()];
    dependencies.extend(plan.targets.iter().map(|target| target.id.clone()));
    let current_plan = PreparedWritebackPlan {
        operations: vec![WritebackOperation::Correction(plan.clone())],
        source_episode_ids: dependencies,
    };
    register_plan(
        &journal,
        &current_job,
        "new-request",
        serde_json::to_value(&plan).unwrap(),
        &current_plan,
        NEW_MS,
    );
    AliasFixture {
        dir,
        store,
        plan,
        journal,
        old_job,
        current_job,
        pending_projection,
    }
}

fn job_state(store: &SqliteMemoryStore, id: &str) -> (String, Option<String>, i64) {
    store
        .conn()
        .unwrap()
        .query_row(
            "SELECT state,last_error,next_operation FROM memory_writeback_jobs WHERE id=?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap()
}

#[tokio::test]
async fn correcting_both_alias_orders_invalidates_old_pending_work_but_preserves_its_current_job_and_history(
) {
    for reverse in [false, true] {
        let f = alias_fixture(reverse).await;
        assert!(
            !f.store
                .apply_automatic_correction(&f.plan)
                .unwrap()
                .replayed
        );
        for target in &f.plan.targets {
            let current = f.store.get_governed(&target.id).unwrap().unwrap();
            assert_eq!(current.status.as_str(), "active");
            assert_eq!(current.revision, 1);
            assert_eq!(current.episode.content, NEW);
            assert_eq!(
                EpisodeStore::get_episode(f.store.as_ref(), &target.id)
                    .unwrap()
                    .unwrap()
                    .content,
                OLD
            );
            let history_id = correction_history_id(NAMESPACE, &target.id, &f.plan.operation_id);
            let history = f.store.get_governed(&history_id).unwrap().unwrap();
            assert_eq!(history.status.as_str(), "active");
            assert_eq!(history.episode.role, "fact_history");
            assert!(history.episode.content.ends_with(OLD));
        }
        assert_eq!(
            job_state(&f.store, &f.old_job),
            (
                "suppressed".into(),
                Some("source_revision_changed".into()),
                2
            )
        );
        assert_eq!(
            job_state(&f.store, &f.current_job),
            ("pending".into(), None, 0),
            "the operation applying the new revision must remain acknowledgeable"
        );
        assert!(
            assert_operation_allowed(&f.store.conn().unwrap(), &operation_id(&f.old_job, 2))
                .is_err()
        );
        let coordinator = MemoryCoordinator::new(f.store.clone(), f.store.clone());
        assert!(
            coordinator
                .writeback_scoped_episode_with_metadata(
                    &f.pending_projection,
                    &MemoryScope::session(NAMESPACE),
                    &old_provenance(),
                    json!({"derived_from_episode_id":"original-user-fact"})
                )
                .is_err(),
            "the frozen old source must not create a stale projection after correction"
        );
        assert!(
            EpisodeStore::get_episode(f.store.as_ref(), &f.pending_projection.id)
                .unwrap()
                .is_none()
        );
        // A second real connection queries persisted temporal state. Revision
        // suppression must not turn the old source into a global withdrawal.
        let temporal = SqliteTemporalGraphStore::open(f.dir.path().join("memory.sqlite3"))
            .await
            .unwrap();
        for target in &f.plan.targets {
            let relation = correction_relation_key(NAMESPACE, &target.id);
            let before = temporal
                .query(
                    &TemporalGraphQuery::new(OLD_MS + 1000)
                        .with_belief_as_of_ms(NEW_MS + 1000)
                        .relation(&relation),
                )
                .unwrap();
            let after = temporal
                .query(
                    &TemporalGraphQuery::new(NEW_MS)
                        .with_belief_as_of_ms(NEW_MS + 1000)
                        .relation(&relation),
                )
                .unwrap();
            assert_eq!(
                before.len(),
                1,
                "historical source hidden by alias order {reverse}"
            );
            assert_eq!(before[0].object_id, OLD);
            assert_eq!(after.len(), 1);
            assert_eq!(after[0].object_id, NEW);
        }
        f.journal.advance(&f.current_job, 0, 1, NEW_MS).unwrap();
        assert_eq!(
            job_state(&f.store, &f.current_job),
            ("done".into(), None, 1)
        );
        assert!(
            f.store
                .apply_automatic_correction(&f.plan)
                .unwrap()
                .replayed
        );
        for target in &f.plan.targets {
            assert_eq!(
                f.store.get_governed(&target.id).unwrap().unwrap().revision,
                1
            );
        }
    }
}

#[tokio::test]
async fn second_alias_failure_rolls_back_the_first_revision_history_and_old_job_invalidation() {
    for reverse in [false, true] {
        let f = alias_fixture(reverse).await;
        let second = &f.plan.targets[1].id;
        let before_head: Option<(String, i64)> = f
            .store
            .conn()
            .unwrap()
            .query_row(
                "SELECT last_hash,entry_count FROM memory_chain_heads WHERE subject_id=?1",
                [NAMESPACE],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .unwrap();
        f.store.conn().unwrap().execute_batch(&format!("CREATE TRIGGER fail_second_alias BEFORE UPDATE OF content_override ON episode_governance WHEN NEW.episode_id='{second}' BEGIN SELECT RAISE(ABORT,'controlled second alias write failure'); END;")).unwrap();
        let error = f.store.apply_automatic_correction(&f.plan).unwrap_err();
        assert!(error
            .to_string()
            .contains("controlled second alias write failure"));
        for target in &f.plan.targets {
            let current = f.store.get_governed(&target.id).unwrap().unwrap();
            assert_eq!(current.status.as_str(), "active");
            assert_eq!(current.revision, 0);
            assert_eq!(current.episode.content, OLD);
            assert!(EpisodeStore::get_episode(
                f.store.as_ref(),
                &correction_history_id(NAMESPACE, &target.id, &f.plan.operation_id)
            )
            .unwrap()
            .is_none());
        }
        assert!(
            EpisodeStore::get_episode(f.store.as_ref(), &f.plan.evidence.id)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            job_state(&f.store, &f.old_job),
            ("pending".into(), None, 2),
            "failure must restore the old work's prior state"
        );
        assert_eq!(
            job_state(&f.store, &f.current_job),
            ("pending".into(), None, 0)
        );
        let connection = f.store.conn().unwrap();
        let after_head: Option<(String, i64)> = connection
            .query_row(
                "SELECT last_hash,entry_count FROM memory_chain_heads WHERE subject_id=?1",
                [NAMESPACE],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .unwrap();
        assert_eq!(after_head, before_head);
        let temporal_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM temporal_graph_facts", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(temporal_count, 0);
        let receipt_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM memory_recovery_operation_receipts WHERE operation_id=?1",
                [&f.plan.operation_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(receipt_count, 0);
        connection
            .execute_batch("DROP TRIGGER fail_second_alias")
            .unwrap();
        drop(connection);
        assert!(
            !f.store
                .apply_automatic_correction(&f.plan)
                .unwrap()
                .replayed
        );
        assert_eq!(job_state(&f.store, &f.old_job).0, "suppressed");
        assert_eq!(job_state(&f.store, &f.current_job).0, "pending");
    }
}
