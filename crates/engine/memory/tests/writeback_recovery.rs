use std::sync::Arc;

use apeireth_core::kernel::Timestamp;
use apeireth_memory::experience_store_sqlite::SQLiteExperienceStore;
use apeireth_memory::writeback_recovery::{
    payload_hash, suppress_source_jobs, PreparedWritebackPlan, RecoveryJob, SqliteWritebackJournal,
    WritebackOperation,
};
use apeireth_memory::{
    IngestGate, MemoryCoordinator, MemoryGovernanceStore, MemoryLayerKind, MemoryProvenance,
    MemoryRecallQuery, MemoryScope, PersonaProfileDelta, PersonaProfileStore,
    SqlitePersonaProfileStore, SqliteTemporalGraphStore, TemporalGraphFact, TemporalGraphQuery,
};
use apeireth_plugin::experience::AssociationStore;
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_storage::SqliteConnectionPool;

async fn pool(path: &std::path::Path) -> Arc<SqliteConnectionPool> {
    let pool = Arc::new(SqliteConnectionPool::open(path).await.unwrap());
    apeireth_memory::run_migrations_on_pool(&pool)
        .await
        .unwrap();
    pool
}

fn job(id: &str) -> RecoveryJob {
    let source = serde_json::json!({"response_id":id});
    RecoveryJob {
        id: id.into(),
        namespace: "owner".into(),
        source_session: "source-session".into(),
        source_request: id.into(),
        source_hash: payload_hash(&source).unwrap(),
        source,
        created_ms: 1_700_000_000_000,
        plan: None,
        next_operation: 0,
        state: "pending".into(),
        attempts: 0,
        last_error: None,
    }
}

fn plan() -> PreparedWritebackPlan {
    PreparedWritebackPlan {
        operations: vec![WritebackOperation::PolicySkipped {
            reason: "fixture policy".into(),
        }],
        source_episode_ids: vec!["source-episode".into()],
    }
}

fn sql(pool: &SqliteConnectionPool, sql: &'static str) {
    pool.write_sync(move |conn| {
        conn.execute_batch(sql)?;
        Ok(())
    })
    .unwrap();
}

fn count(pool: &SqliteConnectionPool, table: &str) -> u64 {
    pool.read(|conn| {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .map_err(Into::into)
    })
    .unwrap()
}

#[tokio::test]
async fn registration_failure_cannot_advance_the_source_cursor_and_stale_pages_are_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let pool = pool(&dir.path().join("memory.db")).await;
    let journal = SqliteWritebackJournal::from_pool(pool.clone());
    sql(&pool,"CREATE TRIGGER reject_job BEFORE INSERT ON memory_writeback_jobs BEGIN SELECT RAISE(ABORT,'injected registration failure'); END");
    assert!(journal
        .register_page("owner", None, &[job("first")], Some("opaque-page-1".into()))
        .is_err());
    assert_eq!(journal.cursor("owner").unwrap(), None);
    assert!(journal.pending("owner", 8).unwrap().is_empty());
    sql(&pool, "DROP TRIGGER reject_job");
    journal
        .register_page("owner", None, &[job("first")], Some("opaque-page-1".into()))
        .unwrap();
    assert!(journal
        .register_page(
            "owner",
            None,
            &[job("second")],
            Some("opaque-page-2".into())
        )
        .is_err());
    assert_eq!(journal.pending("owner", 8).unwrap().len(), 1);
    assert_eq!(
        journal.cursor("owner").unwrap().as_deref(),
        Some("opaque-page-1")
    );
    let mut conflicting = job("first");
    conflicting.source_hash = "different source".into();
    assert!(journal
        .register_page(
            "owner",
            Some("opaque-page-1".into()),
            &[conflicting],
            Some("opaque-page-2".into())
        )
        .is_err());
    assert_eq!(
        journal.cursor("owner").unwrap().as_deref(),
        Some("opaque-page-1")
    );
}

#[tokio::test]
async fn frozen_plans_and_terminal_tombstones_survive_reopen_and_duplicate_registration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.db");
    {
        let pool = pool(&path).await;
        let journal = SqliteWritebackJournal::from_pool(pool.clone());
        journal
            .register_page(
                "owner",
                None,
                &[job("first"), job("second")],
                Some("opaque-page".into()),
            )
            .unwrap();
        sql(&pool,"CREATE TRIGGER reject_plan BEFORE UPDATE OF plan_json ON memory_writeback_jobs BEGIN SELECT RAISE(ABORT,'injected plan failure'); END");
        assert!(journal.save_plan("first", &plan()).is_err());
        assert!(journal.pending("owner", 8).unwrap()[0].plan.is_none());
        assert_eq!(count(&pool, "memory_writeback_dependencies"), 0);
        sql(&pool, "DROP TRIGGER reject_plan");
        journal.save_plan("first", &plan()).unwrap();
        journal.save_plan("second", &plan()).unwrap();
        let mut different = plan();
        different.operations.clear();
        assert!(journal.save_plan("first", &different).is_err());
        journal.advance("first", 0, 1, 1_700_000_001_000).unwrap();
        pool.write_sync(|conn| {
            suppress_source_jobs(conn, "source-episode")
                .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))
        })
        .unwrap();
    }
    let pool = pool(&path).await;
    let journal = SqliteWritebackJournal::from_pool(pool);
    assert!(journal.pending("owner", 8).unwrap().is_empty());
    assert_eq!(journal.status("owner").unwrap().suppressed, 2);
    journal
        .register_page(
            "owner",
            Some("opaque-page".into()),
            &[job("first"), job("second")],
            Some("opaque-next".into()),
        )
        .unwrap();
    assert!(
        journal.pending("owner", 8).unwrap().is_empty(),
        "retained transcript scans must not resurrect suppressed jobs"
    );
    assert_eq!(journal.status("other-owner").unwrap().suppressed, 0);
}

#[tokio::test]
async fn association_failure_rolls_back_the_observation_and_replay_counts_once() {
    let dir = tempfile::tempdir().unwrap();
    let pool = pool(&dir.path().join("memory.db")).await;
    let store = SQLiteExperienceStore::from_arc(pool.clone());
    store.ensure_schema().await.unwrap();
    sql(&pool,"CREATE TRIGGER reject_node BEFORE INSERT ON association_nodes BEGIN SELECT RAISE(ABORT,'injected node failure'); END");
    assert!(store
        .record_cooccurrence("left", "right", "episode")
        .is_err());
    assert_eq!(count(&pool, "association_observations"), 0);
    assert_eq!(count(&pool, "association_edges"), 0);
    sql(&pool, "DROP TRIGGER reject_node");
    store
        .record_cooccurrence("left", "right", "episode")
        .unwrap();
    store
        .record_cooccurrence("left", "right", "episode")
        .unwrap();
    assert_eq!(count(&pool, "association_observations"), 1);
    assert_eq!(
        store.top_associations("left", 8).unwrap()[0].co_occurrence_count,
        1
    );
    assert_eq!(count(&pool, "association_nodes"), 2);
}

#[tokio::test]
async fn persona_effect_and_receipt_roll_back_together_and_replay_does_not_increase_revision() {
    let dir = tempfile::tempdir().unwrap();
    let pool = pool(&dir.path().join("memory.db")).await;
    let store = SqlitePersonaProfileStore::from_arc(pool.clone());
    let delta = PersonaProfileDelta {
        facts_add: vec!["偏好先核对原始证据".into()],
        ..Default::default()
    };
    let at = Timestamp::from_epoch_millis(1_700_000_000_000).unwrap();
    sql(&pool,"CREATE TRIGGER reject_receipt BEFORE INSERT ON memory_recovery_operation_receipts BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END");
    assert!(store
        .apply_delta_once("persona-op", "persona", "subject", 0, delta.clone(), at)
        .await
        .is_err());
    assert!(store
        .get_profile("persona", "subject")
        .await
        .unwrap()
        .is_none());
    assert_eq!(count(&pool, "persona_profile_history"), 0);
    sql(&pool, "DROP TRIGGER reject_receipt");
    let first = store
        .apply_delta_once("persona-op", "persona", "subject", 0, delta.clone(), at)
        .await
        .unwrap();
    let replay = store
        .apply_delta_once("persona-op", "persona", "subject", 0, delta.clone(), at)
        .await
        .unwrap();
    assert_eq!(first.revision, 1);
    assert_eq!(replay.revision, 1);
    assert_eq!(count(&pool, "persona_profile_history"), 1);
    let mut conflict = delta;
    conflict.facts_add.push("different payload".into());
    assert!(store
        .apply_delta_once("persona-op", "persona", "subject", 0, conflict, at)
        .await
        .is_err());
    assert_eq!(
        store
            .get_profile("persona", "subject")
            .await
            .unwrap()
            .unwrap()
            .revision,
        1
    );
}

#[tokio::test]
async fn temporal_same_id_same_payload_is_idempotent_and_a_conflicting_payload_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let pool = pool(&dir.path().join("memory.db")).await;
    let store = SqliteTemporalGraphStore::from_arc(pool.clone());
    let fact = TemporalGraphFact {
        id: "frozen-fact".into(),
        relation_key: "project/city".into(),
        subject_id: "project".into(),
        predicate: "city".into(),
        object_id: "上海".into(),
        valid_from_ms: 1000,
        valid_until_ms: None,
        believed_at_ms: 1000,
        source_episode_id: None,
        scope: serde_json::json!({"scope":"session","session_id":"owner"}),
        provenance: serde_json::json!({"source":"user"}),
        confidence: 1.0,
        revision: 0,
        supersedes_id: None,
        retracted_at_ms: None,
        created_at_ms: 1000,
    };
    let first = store.append(fact.clone()).await.unwrap();
    assert_eq!(
        store.append(fact.clone()).await.unwrap().revision,
        first.revision
    );
    let mut conflict = fact;
    conflict.object_id = "武汉".into();
    assert!(store.append(conflict).await.is_err());
    assert_eq!(count(&pool, "temporal_graph_facts"), 1);
}

#[tokio::test]
async fn ingest_gate_receipt_prevents_double_counting_and_rolls_back_on_ack_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.db");
    let store = apeireth_memory::SqliteMemoryStore::open(&path).unwrap();
    store
        .record_recovery("ingest-op", "source", "frozen material", 1000, false)
        .unwrap();
    store
        .record_recovery("ingest-op", "source", "frozen material", 1000, false)
        .unwrap();
    let conn = store.conn().unwrap();
    let ingested: i64 = conn
        .query_row(
            "SELECT ingest_count FROM source_hash WHERE source='source'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(ingested, 1);
    conn.execute_batch("CREATE TRIGGER reject_receipt BEFORE INSERT ON memory_recovery_operation_receipts BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END").unwrap();
    drop(conn);
    assert!(store
        .record_recovery("another-op", "source", "frozen material", 2000, false)
        .is_err());
    let conn = store.conn().unwrap();
    assert_eq!(
        conn.query_row::<i64, _, _>(
            "SELECT ingest_count FROM source_hash WHERE source='source'",
            [],
            |r| r.get(0)
        )
        .unwrap(),
        1
    );
}

#[test]
fn persona_consolidation_and_its_receipt_share_one_transaction() {
    use apeireth_memory::persona_layer::{PersonaExtract, PersonaMemoryStore, PersonaStatement};
    let dir = tempfile::tempdir().unwrap();
    let store = apeireth_memory::SqliteMemoryStore::open(dir.path().join("memory.db")).unwrap();
    let extract = PersonaExtract {
        preferences: vec![PersonaStatement {
            text: "我喜欢先核对证据".into(),
            source_episode_id: "source".into(),
        }],
        ..Default::default()
    };
    store.conn().unwrap().execute_batch("CREATE TRIGGER reject_receipt BEFORE INSERT ON memory_recovery_operation_receipts BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END").unwrap();
    assert!(store
        .consolidate_once("persona-consolidation", &extract, 1000)
        .is_err());
    assert!(store.load().unwrap().preferences.is_empty());
    store
        .conn()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_receipt")
        .unwrap();
    store
        .consolidate_once("persona-consolidation", &extract, 1000)
        .unwrap();
    store
        .consolidate_once("persona-consolidation", &extract, 1000)
        .unwrap();
    assert_eq!(store.load().unwrap().preferences.len(), 1);
}

fn seed_episode(
    coordinator: &MemoryCoordinator,
    id: &str,
    namespace: &str,
    metadata: serde_json::Value,
) {
    coordinator
        .writeback_scoped_episode_with_metadata(
            &apeireth_core::Episode {
                id: id.into(),
                timestamp: 1_700_000_000,
                role: if id == "source-episode" {
                    "user"
                } else {
                    "fact_history"
                }
                .into(),
                content: format!("{id} 可撤回历史事实"),
                session_id: namespace.into(),
            },
            &MemoryScope::session(namespace),
            &MemoryProvenance {
                source: "recovery-test".into(),
                source_request: Some(format!("request-{id}")),
                source_session: Some(namespace.into()),
                source_trace: None,
            },
            metadata,
        )
        .unwrap();
}

#[tokio::test]
async fn forgetting_source_and_recursive_history_and_pending_suppression_is_atomic() {
    let dir = tempfile::tempdir().unwrap();
    let pool = pool(&dir.path().join("memory.db")).await;
    let backend = Arc::new(apeireth_memory::backend::sqlite::SqliteBackend::from_arc(
        pool.clone(),
    ));
    let coordinator = MemoryCoordinator::new(backend.clone(), backend.clone());
    seed_episode(
        &coordinator,
        "source-episode",
        "owner",
        serde_json::json!({}),
    );
    seed_episode(
        &coordinator,
        "history",
        "owner",
        serde_json::json!({"derived_from_episode_id":"source-episode"}),
    );
    seed_episode(
        &coordinator,
        "grandchild",
        "owner",
        serde_json::json!({"derived_from_episode_id":"history"}),
    );
    seed_episode(
        &coordinator,
        "evidence-child",
        "owner",
        serde_json::json!({"correction_evidence_id":"source-episode"}),
    );
    seed_episode(
        &coordinator,
        "corrected-target",
        "owner",
        serde_json::json!({"correction":{"evidence":"source-episode"}}),
    );
    seed_episode(
        &coordinator,
        "other-owner",
        "other",
        serde_json::json!({"derived_from_episode_id":"source-episode"}),
    );
    let journal = SqliteWritebackJournal::from_pool(pool.clone());
    journal
        .register_page("owner", None, &[job("pending")], None)
        .unwrap();
    journal.save_plan("pending", &plan()).unwrap();
    sql(&pool,"CREATE TRIGGER reject_suppression BEFORE UPDATE OF state ON memory_writeback_jobs BEGIN SELECT RAISE(ABORT,'injected suppression failure'); END");
    assert!(backend
        .forget_episode("source-episode", Some("withdraw"), 0)
        .is_err());
    for id in [
        "source-episode",
        "history",
        "grandchild",
        "evidence-child",
        "corrected-target",
    ] {
        let governed = backend.get_governed(id).unwrap().unwrap();
        assert_eq!(governed.status.as_str(), "active");
        assert_eq!(governed.revision, 0);
    }
    assert_eq!(journal.status("owner").unwrap().pending, 1);
    sql(&pool, "DROP TRIGGER reject_suppression");
    backend.protect_episode("history", 0).unwrap();
    assert!(matches!(
        backend.forget_episode("source-episode", None, 0),
        Err(apeireth_memory::MemoryGovernanceError::Protected(_))
    ));
    assert_eq!(
        backend
            .get_governed("source-episode")
            .unwrap()
            .unwrap()
            .revision,
        0
    );
    backend.unprotect_episode("history", 1).unwrap();
    backend
        .forget_episode("source-episode", Some("withdraw"), 0)
        .unwrap();
    for id in [
        "source-episode",
        "history",
        "grandchild",
        "evidence-child",
        "corrected-target",
    ] {
        assert_eq!(
            backend.get_governed(id).unwrap().unwrap().status.as_str(),
            "forgotten"
        );
    }
    assert_eq!(
        backend
            .get_governed("other-owner")
            .unwrap()
            .unwrap()
            .status
            .as_str(),
        "active"
    );
    assert_eq!(journal.status("owner").unwrap().suppressed, 1);
    let recall = coordinator
        .recall(
            &MemoryRecallQuery::new("owner", "可撤回 历史 事实")
                .with_layers(vec![MemoryLayerKind::Working, MemoryLayerKind::Episodic]),
        )
        .unwrap();
    assert!(
        recall.items.is_empty(),
        "governed historical material must leave all default episode recall layers"
    );
    assert!(
        backend.get_episode("history").unwrap().is_none(),
        "the ordinary backend getter also enforces withdrawal"
    );
    let audit = pool
        .read(|conn| {
            Ok(conn.query_row::<(String, String), _, _>(
                "SELECT role,content FROM episodes WHERE id='history'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })
        .unwrap();
    assert_eq!(
        audit,
        ("fact_history".into(), "history 可撤回历史事实".into()),
        "the official immutable episode table retains exact audit evidence"
    );
    assert!(
        pool.read(
            |conn| apeireth_memory::memory_chain::verify_subject_chain(conn, "owner")
                .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))
        )
        .unwrap()
        .ok,
        "the retained audit chain remains verifiable after withdrawal"
    );
    journal
        .register_page("owner", None, &[job("pending")], None)
        .unwrap();
    assert_eq!(
        journal.status("owner").unwrap().suppressed,
        1,
        "source rescan cannot revive pending work"
    );
}

#[tokio::test]
async fn temporal_queries_and_history_hide_withdrawn_ancestry_without_falling_back() {
    let dir = tempfile::tempdir().unwrap();
    let pool = pool(&dir.path().join("memory.db")).await;
    let backend = Arc::new(apeireth_memory::backend::sqlite::SqliteBackend::from_arc(
        pool.clone(),
    ));
    let coordinator = MemoryCoordinator::new(backend.clone(), backend.clone());
    for id in ["source-episode", "new-evidence", "fresh-evidence"] {
        seed_episode(&coordinator, id, "owner", serde_json::json!({}));
    }
    let temporal = SqliteTemporalGraphStore::from_arc(pool.clone());
    let fact = TemporalGraphFact {
        id: "old-fact".into(),
        relation_key: "owner/city".into(),
        subject_id: "owner".into(),
        predicate: "city".into(),
        object_id: "武汉".into(),
        valid_from_ms: 1000,
        valid_until_ms: None,
        believed_at_ms: 1000,
        source_episode_id: Some("source-episode".into()),
        scope: serde_json::to_value(MemoryScope::session("owner")).unwrap(),
        provenance: serde_json::json!({"source":"test"}),
        confidence: 1.0,
        revision: 0,
        supersedes_id: None,
        retracted_at_ms: None,
        created_at_ms: 1000,
    };
    temporal.append(fact.clone()).await.unwrap();
    let mut corrected = fact.clone();
    corrected.id = "new-fact".into();
    corrected.object_id = "上海".into();
    corrected.valid_from_ms = 2000;
    corrected.believed_at_ms = 2000;
    corrected.created_at_ms = 2000;
    corrected.source_episode_id = Some("new-evidence".into());
    corrected.supersedes_id = Some(fact.id.clone());
    temporal.append(corrected.clone()).await.unwrap();
    let query = TemporalGraphQuery::new(3000).relation("owner/city");
    assert_eq!(temporal.query(&query).unwrap()[0].object_id, "上海");
    backend.forget_episode("source-episode", None, 0).unwrap();
    assert!(temporal.query(&query).unwrap().is_empty());
    assert!(temporal.history("owner/city").unwrap().is_empty());
    assert!(temporal
        .traverse("owner", &query, Default::default())
        .unwrap()
        .facts
        .is_empty());
    assert_eq!(
        count(&pool, "temporal_graph_facts"),
        2,
        "immutable audit rows remain intact"
    );
    let mut fresh = corrected;
    fresh.id = "fresh-fact".into();
    fresh.source_episode_id = Some("fresh-evidence".into());
    fresh.supersedes_id = None;
    fresh.valid_from_ms = 3000;
    fresh.believed_at_ms = 3000;
    fresh.created_at_ms = 3000;
    temporal.append(fresh).await.unwrap();
    assert_eq!(
        temporal.query(&query).unwrap().len(),
        1,
        "an independent newly authorized assertion remains usable"
    );
    assert_eq!(temporal.history("owner/city").unwrap().len(), 1);
}

#[tokio::test]
async fn experience_reads_remove_withdrawn_sources_and_recount_only_active_observations() {
    use apeireth_plugin::experience::{
        GraphFact, GraphLink, KnowledgeGraphStore, WikiEntry, WikiEntryStore,
    };
    let dir = tempfile::tempdir().unwrap();
    let pool = pool(&dir.path().join("memory.db")).await;
    let backend = Arc::new(apeireth_memory::backend::sqlite::SqliteBackend::from_arc(
        pool.clone(),
    ));
    let experience = Arc::new(SQLiteExperienceStore::from_arc(pool.clone()));
    experience.ensure_schema().await.unwrap();
    let coordinator = MemoryCoordinator::new(backend.clone(), backend.clone())
        .with_experience(experience.clone(), experience.clone());
    for id in ["source-episode", "active-source"] {
        seed_episode(&coordinator, id, "owner", serde_json::json!({}));
    }
    for (id, source, confidence) in [
        ("visible", "active-source", 0.5),
        ("withdrawn", "source-episode", 1.0),
    ] {
        experience
            .put_wiki(&WikiEntry {
                id: id.into(),
                session_id: "owner".into(),
                source_episode_id: source.into(),
                topic: "shared-topic".into(),
                summary: id.into(),
                body: id.into(),
                confidence,
                tags: vec![],
                extracted_at: if id == "withdrawn" { 2000 } else { 1000 },
            })
            .unwrap();
        experience
            .put_fact(&GraphFact {
                id: id.into(),
                subject_id: "owner".into(),
                subject_kind: "user".into(),
                predicate: "city".into(),
                object_id: id.into(),
                object_kind: "city".into(),
                valid_from: 1000,
                valid_until: None,
                source_episode_id: source.into(),
                confidence,
            })
            .unwrap();
        experience
            .put_link(&GraphLink {
                from_id: "owner".into(),
                to_id: id.into(),
                kind: "city".into(),
                weight: confidence,
                source_episode_id: source.into(),
                created_at: 1000,
            })
            .unwrap();
        experience
            .record_cooccurrence("owner", "shared-edge", source)
            .unwrap();
    }
    experience
        .record_cooccurrence("owner", "withdrawn-edge", "source-episode")
        .unwrap();
    experience
        .put_fact(&GraphFact {
            id: "host-external".into(),
            subject_id: "host".into(),
            subject_kind: "host".into(),
            predicate: "external".into(),
            object_id: "external".into(),
            object_kind: "external".into(),
            valid_from: 1000,
            valid_until: None,
            source_episode_id: "host-source-without-episode".into(),
            confidence: 0.5,
        })
        .unwrap();
    assert_eq!(
        experience
            .top_associations("owner", 8)
            .unwrap()
            .iter()
            .find(|e| e.to_entity == "shared-edge")
            .unwrap()
            .co_occurrence_count,
        2
    );
    backend.forget_episode("source-episode", None, 0).unwrap();
    assert!(experience
        .wiki_for_episode("source-episode")
        .unwrap()
        .is_empty());
    assert_eq!(
        experience.list_wiki("owner", "shared-topic", 1).unwrap()[0].id,
        "visible",
        "withdrawn top rows cannot consume the visible limit"
    );
    assert_eq!(experience.facts_from("owner", 1).unwrap()[0].id, "visible");
    assert_eq!(
        experience.facts_from_as_of("owner", 1, 2000).unwrap()[0].id,
        "visible"
    );
    assert_eq!(
        experience.all_facts(8).unwrap().len(),
        2,
        "Host external sources stay compatible until explicitly withdrawn"
    );
    assert_eq!(
        experience.links_from("owner", 1).unwrap()[0].to_id,
        "visible"
    );
    let associations = experience.top_associations("owner", 8).unwrap();
    assert_eq!(associations.len(), 1);
    assert_eq!(associations[0].co_occurrence_count, 1);
    assert_eq!(associations[0].last_seen_episode_id, "active-source");
    let recall = coordinator
        .recall(
            &MemoryRecallQuery::new("owner", "owner")
                .with_layers(vec![MemoryLayerKind::Relational]),
        )
        .unwrap();
    let mut ids: Vec<_> = recall.items.iter().map(|item| item.id.as_str()).collect();
    ids.sort();
    assert_eq!(
        ids,
        vec!["assoc:active-source", "fact:owner:city:visible"],
        "both the active fact and independently active association remain eligible"
    );
    assert!(
        recall
            .items
            .iter()
            .all(|item| !item.content.contains("withdrawn") && !item.id.contains("source-episode")),
        "no withdrawn fact or observation may enter any relational channel"
    );
    assert!(recall
        .items
        .iter()
        .find(|item| item.id == "fact:owner:city:visible")
        .unwrap()
        .content
        .contains("visible"));
    assert_eq!(
        recall
            .items
            .iter()
            .find(|item| item.id == "assoc:active-source")
            .unwrap()
            .content,
        backend
            .get_episode(&associations[0].last_seen_episode_id)
            .unwrap()
            .unwrap()
            .content,
        "the associative candidate is the exact governed active observation source"
    );
    assert_eq!(count(&pool, "wiki_entries"), 2);
    assert_eq!(count(&pool, "kg_facts"), 3);
    assert_eq!(count(&pool, "kg_links"), 2);
    assert_eq!(count(&pool, "association_observations"), 3);
    backend
        .update_episode_content("active-source", "已修订的有效原始证据", Some("panel"), 0)
        .unwrap();
    assert_eq!(
        backend
            .get_governed("active-source")
            .unwrap()
            .unwrap()
            .status
            .as_str(),
        "active",
        "source revision invalidates its old projections without retracting the source"
    );
    assert!(experience.list_wiki("owner", "", 8).unwrap().is_empty());
    assert!(experience.facts_from("owner", 8).unwrap().is_empty());
    assert!(experience.links_from("owner", 8).unwrap().is_empty());
    assert!(experience.top_associations("owner", 8).unwrap().is_empty());
    assert!(
        coordinator
            .recall(
                &MemoryRecallQuery::new("owner", "owner")
                    .with_layers(vec![MemoryLayerKind::Relational])
            )
            .unwrap()
            .items
            .is_empty(),
        "removing the final active source leaves neither graph nor associative recall candidates"
    );
    backend.forget_episode("active-source", None, 1).unwrap();
}

#[tokio::test]
async fn revision_after_admission_is_rechecked_inside_each_typed_effect_transaction() {
    let dir = tempfile::tempdir().unwrap();
    let pool = pool(&dir.path().join("memory.db")).await;
    let backend = Arc::new(apeireth_memory::backend::sqlite::SqliteBackend::from_arc(
        pool.clone(),
    ));
    let coordinator = MemoryCoordinator::new(backend.clone(), backend.clone());
    seed_episode(
        &coordinator,
        "source-episode",
        "owner",
        serde_json::json!({}),
    );
    let journal = SqliteWritebackJournal::from_pool(pool.clone());
    journal
        .register_page("owner", None, &[job("phase")], None)
        .unwrap();
    journal.save_plan("phase", &plan()).unwrap();
    assert!(
        journal.is_pending("phase").unwrap(),
        "admission happened before the concurrent source revision"
    );
    backend
        .update_episode_content("source-episode", "新的明确声明", Some("panel"), 0)
        .unwrap();
    assert!(!journal.is_pending("phase").unwrap());
    let op = "phase/operation/0";
    let fact = TemporalGraphFact {
        id: "stale-typed".into(),
        relation_key: "owner/city".into(),
        subject_id: "owner".into(),
        predicate: "city".into(),
        object_id: "旧值".into(),
        valid_from_ms: 1000,
        valid_until_ms: None,
        believed_at_ms: 1000,
        source_episode_id: Some("source-episode".into()),
        scope: serde_json::to_value(MemoryScope::session("owner")).unwrap(),
        provenance: serde_json::json!({"source":"typed"}),
        confidence: 1.0,
        revision: 0,
        supersedes_id: None,
        retracted_at_ms: None,
        created_at_ms: 1000,
    };
    let temporal = SqliteTemporalGraphStore::from_arc(pool.clone());
    assert!(temporal.append_recovery(op, fact.clone()).await.is_err());
    let persona = SqlitePersonaProfileStore::from_arc(pool.clone());
    assert!(persona
        .apply_delta_once(
            op,
            "persona",
            "owner",
            0,
            PersonaProfileDelta {
                facts_add: vec!["old value".into()],
                ..Default::default()
            },
            Timestamp::from_epoch_millis(1000).unwrap()
        )
        .await
        .is_err());
    let commitments = apeireth_memory::SqliteCommitmentStore::from_arc(pool.clone());
    let commitment = apeireth_memory::Commitment {
        id: "stale-commitment".into(),
        subject_id: "owner".into(),
        kind: apeireth_memory::CommitmentKind::Shared,
        status: apeireth_memory::CommitmentStatus::Active,
        text: "old promise".into(),
        scope: None,
        provenance: None,
        source_episode: Some("source-episode".into()),
        deadline: None,
        confidence: 1.0,
        supersedes_id: None,
        retracted_at_ms: None,
        completed_at_ms: None,
        created_at: 1000,
        updated_at: 1000,
        revision: 0,
    };
    assert!(commitments.create_recovery(op, commitment).await.is_err());
    assert_eq!(count(&pool, "temporal_graph_facts"), 0);
    assert_eq!(count(&pool, "persona_profiles"), 0);
    assert_eq!(count(&pool, "commitments"), 0);
    assert_eq!(count(&pool, "memory_recovery_operation_receipts"), 0);
    temporal.append(fact).await.unwrap();
    assert!(temporal.query(&TemporalGraphQuery::new(2000)).unwrap().is_empty(),"an old typed projection cannot become current merely because its source remains active after revision");
    assert_eq!(
        count(&pool, "temporal_graph_facts"),
        1,
        "the immutable audit row is still present"
    );
}
