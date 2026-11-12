#[path = "support/convergence.rs"]
mod support;

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{Clock, SessionId, VirtualClock};
use apeireth_memory::backend::sqlite::SqliteBackend;
use apeireth_memory::writeback_recovery::{SqliteWritebackJournal, WritebackOperation};
use apeireth_memory::{MemoryCoordinator, MemoryGovernanceStore, MemoryTypedMaterializationSink};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_runtime::canonical::module::{
    BehaviorModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};
use apeireth_runtime::{Runtime, SessionEventKind, TurnRequest};
use apeireth_runtime_assembly::{MemoryWritebackModule, SqliteSessionStore};
use apeireth_storage::SqliteConnectionPool;
use async_trait::async_trait;

struct CaptureStartCursor {
    manifest: ModuleManifest,
    journal: Arc<SqliteWritebackJournal>,
    observed: Arc<Mutex<Option<String>>>,
}

#[async_trait]
impl BehaviorModule for CaptureStartCursor {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::TurnStart {
            *self.observed.lock().unwrap() =
                self.journal.cursor(&ctx.session_id.to_string()).unwrap();
        }
        Ok(ModuleOutcome::continue_())
    }
}

fn writer_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        chrono::DateTime::from_timestamp(2_000_000_000, 0).unwrap(),
    ))
}

async fn assemble(
    root: &std::path::Path,
) -> (
    Runtime,
    Arc<MemoryWritebackModule>,
    Arc<SqliteBackend>,
    Arc<SqliteWritebackJournal>,
) {
    assemble_with_capture(root, None).await
}

async fn assemble_with_capture(
    root: &std::path::Path,
    capture: Option<Arc<Mutex<Option<String>>>>,
) -> (
    Runtime,
    Arc<MemoryWritebackModule>,
    Arc<SqliteBackend>,
    Arc<SqliteWritebackJournal>,
) {
    assemble_with_options(root, capture, None).await
}

async fn assemble_with_options(
    root: &std::path::Path,
    capture: Option<Arc<Mutex<Option<String>>>>,
    typed: Option<Arc<dyn MemoryTypedMaterializationSink>>,
) -> (
    Runtime,
    Arc<MemoryWritebackModule>,
    Arc<SqliteBackend>,
    Arc<SqliteWritebackJournal>,
) {
    let pool = Arc::new(
        SqliteConnectionPool::open(root.join("memory.sqlite3"))
            .await
            .unwrap(),
    );
    apeireth_memory::run_migrations_on_pool(&pool)
        .await
        .unwrap();
    let backend = Arc::new(SqliteBackend::from_arc(pool.clone()));
    let journal = Arc::new(SqliteWritebackJournal::from_pool(pool));
    let coordinator = Arc::new(MemoryCoordinator::new(backend.clone(), backend.clone()));
    let mut writer = MemoryWritebackModule::new(backend.clone(), writer_clock())
        .with_coordinator(coordinator)
        .with_recovery_journal(journal.clone());
    if let Some(sink) = typed {
        writer = writer.with_typed_sink(sink)
    }
    let writer = Arc::new(writer);
    let mut builder = support::builder(support::Provider::new(vec![vec![], vec![]]))
        .with_session_store(Arc::new(
            SqliteSessionStore::open(root.join("sessions.sqlite3"))
                .await
                .unwrap(),
        ))
        .with_registered_behavior(Arc::new(
            apeireth_runtime_assembly::canonical::memory_recovery::MemoryRecoveryModule::new(
                writer.clone(),
            ),
        ));
    if let Some(observed) = capture {
        builder = builder.with_registered_behavior(Arc::new(CaptureStartCursor {
            manifest: ModuleManifest::new("test.capture-cursor", "Capture start cursor"),
            journal: journal.clone(),
            observed,
        }));
    }
    let runtime = builder
        .with_registered_behavior(writer.clone())
        .build()
        .await
        .unwrap();
    (runtime, writer, backend, journal)
}

#[tokio::test]
async fn failed_after_turn_write_restarts_from_the_frozen_committed_plan_and_retries_without_new_time(
) {
    let dir = tempfile::tempdir().unwrap();
    let session = SessionId::new();
    let (runtime, writer, backend, journal) = assemble(dir.path()).await;
    backend.pool().write_sync(|conn|{conn.execute_batch("CREATE TRIGGER reject_memory BEFORE INSERT ON episodes BEGIN SELECT RAISE(ABORT,'injected memory failure'); END")?;Ok(())}).unwrap();
    runtime
        .execute(TurnRequest::new(
            session,
            "我偏好先核对原始证据再作答。RECOVERY-FACT-A",
        ))
        .await
        .unwrap();
    let pending = journal.pending(&session.to_string(), 8).unwrap();
    assert_eq!(pending.len(), 1);
    let frozen = pending[0].plan.clone().unwrap();
    let expected: Vec<_> = frozen
        .operations
        .iter()
        .filter_map(|op| match op {
            WritebackOperation::Episode { episode, .. } => Some(episode.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(expected.len(), 2);
    assert!(
        expected.iter().all(|e| e.timestamp != 2_000_000_000),
        "module retry clock must not replace committed source time"
    );
    assert!(backend
        .recent_episodes(&session.to_string(), 32)
        .unwrap()
        .is_empty());
    assert_eq!(
        writer.recovery_status(&session).unwrap().unwrap().pending,
        1
    );
    backend
        .pool()
        .write_sync(|conn| {
            conn.execute_batch("DROP TRIGGER reject_memory")?;
            Ok(())
        })
        .unwrap();
    drop(runtime);
    drop(writer);
    drop(backend);
    drop(journal);

    let (runtime, writer, backend, journal) = assemble(dir.path()).await;
    runtime
        .execute(TurnRequest::new(session, "核对恢复后的记录。"))
        .await
        .unwrap();
    for original in &expected {
        let stored = backend.get_episode(&original.id).unwrap().unwrap();
        assert_eq!(stored.timestamp, original.timestamp);
        assert_eq!(stored.content, original.content);
        let metadata = backend.get_episode_metadata(&original.id).unwrap().unwrap();
        assert_eq!(metadata["scope"]["session_id"], session.to_string());
        assert_eq!(
            metadata["provenance"]["source_session"],
            session.to_string()
        );
    }
    assert_eq!(
        writer.recovery_status(&session).unwrap().unwrap().pending,
        0
    );
    assert!(journal.status(&session.to_string()).unwrap().done >= 2);
}

#[tokio::test]
async fn failed_journal_registration_is_rebuilt_from_the_official_session_commit_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let session = SessionId::new();
    let start_cursor = Arc::new(Mutex::new(None));
    let (runtime, writer, backend, journal) =
        assemble_with_capture(dir.path(), Some(start_cursor.clone())).await;
    backend.pool().write_sync(|conn|{conn.execute_batch("CREATE TRIGGER reject_registration BEFORE INSERT ON memory_writeback_jobs BEGIN SELECT RAISE(ABORT,'injected registration failure'); END")?;Ok(())}).unwrap();
    runtime
        .execute(TurnRequest::new(
            session,
            "REGISTER-RECOVERY-FACT 我选择先验证证据。",
        ))
        .await
        .unwrap();
    assert!(
        writer.recovery_status(&session).is_err(),
        "unavailable journal must not be reported as an empty successful queue"
    );
    assert!(
        start_cursor.lock().unwrap().is_some(),
        "TurnStart may checkpoint its completed empty scan"
    );
    assert_eq!(
        journal.cursor(&session.to_string()).unwrap(),
        *start_cursor.lock().unwrap(),
        "failed AfterTurn registration cannot advance past its response source"
    );
    assert!(journal.pending(&session.to_string(), 8).unwrap().is_empty());
    backend
        .pool()
        .write_sync(|conn| {
            conn.execute_batch("DROP TRIGGER reject_registration")?;
            Ok(())
        })
        .unwrap();
    drop(runtime);
    drop(writer);
    drop(backend);
    drop(journal);

    let (runtime, writer, backend, _journal) = assemble(dir.path()).await;
    runtime
        .execute(TurnRequest::new(session, "请检查上个完成回合。"))
        .await
        .unwrap();
    assert!(backend
        .recent_episodes(&session.to_string(), 32)
        .unwrap()
        .iter()
        .any(|e| e.content.contains("REGISTER-RECOVERY-FACT")));
    assert_eq!(
        writer.recovery_status(&session).unwrap().unwrap().pending,
        0
    );
}

#[tokio::test]
async fn forgetting_a_partial_source_suppresses_its_remaining_plan_and_survives_another_restart() {
    let dir = tempfile::tempdir().unwrap();
    let session = SessionId::new();
    let (runtime, writer, backend, journal) = assemble(dir.path()).await;
    backend.pool().write_sync(|conn|{conn.execute_batch("CREATE TRIGGER reject_assistant BEFORE INSERT ON episodes WHEN NEW.id LIKE '%-assistant' BEGIN SELECT RAISE(ABORT,'injected partial write'); END")?;Ok(())}).unwrap();
    runtime
        .execute(TurnRequest::new(
            session,
            "FORGET-PENDING-FACT 我选择可撤回的记录。",
        ))
        .await
        .unwrap();
    let pending = journal.pending(&session.to_string(), 8).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].next_operation, 1);
    let plan = pending[0].plan.clone().unwrap();
    let raw: Vec<_> = plan
        .operations
        .iter()
        .filter_map(|op| match op {
            WritebackOperation::Episode { episode, .. } => Some(episode),
            _ => None,
        })
        .collect();
    let user = raw.iter().find(|e| e.role == "user").unwrap();
    let assistant_id = raw
        .iter()
        .find(|e| e.role == "assistant")
        .unwrap()
        .id
        .clone();
    backend
        .forget_episode(&user.id, Some("user withdrew this source"), 0)
        .unwrap();
    backend
        .pool()
        .write_sync(|conn| {
            conn.execute_batch("DROP TRIGGER reject_assistant")?;
            Ok(())
        })
        .unwrap();
    drop(runtime);
    drop(writer);
    drop(backend);
    drop(journal);

    let (runtime, writer, backend, journal) = assemble(dir.path()).await;
    runtime
        .execute(TurnRequest::new(session, "继续处理其他内容。"))
        .await
        .unwrap();
    assert!(backend.get_episode(&assistant_id).unwrap().is_none());
    assert_eq!(journal.status(&session.to_string()).unwrap().suppressed, 1);
    assert_eq!(
        writer.recovery_status(&session).unwrap().unwrap().pending,
        0
    );
    drop(runtime);
    drop(writer);
    drop(backend);
    drop(journal);
    let (_runtime, _writer, backend, journal) = assemble(dir.path()).await;
    assert!(backend.get_episode(&assistant_id).unwrap().is_none());
    assert_eq!(journal.status(&session.to_string()).unwrap().suppressed, 1);
}

#[tokio::test]
async fn masking_a_committed_output_suppresses_its_pending_projection_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let session = SessionId::new();
    let (runtime, writer, backend, journal) = assemble(dir.path()).await;
    backend.pool().write_sync(|conn|{conn.execute_batch("CREATE TRIGGER reject_memory BEFORE INSERT ON episodes BEGIN SELECT RAISE(ABORT,'injected memory failure'); END")?;Ok(())}).unwrap();
    runtime
        .execute(TurnRequest::new(
            session,
            "MASKED-PENDING-SOURCE 我选择保留可撤回的证据。 ",
        ))
        .await
        .unwrap();
    let pending = journal.pending(&session.to_string(), 8).unwrap();
    assert_eq!(pending.len(), 1);
    let ids: Vec<_> = pending[0]
        .plan
        .as_ref()
        .unwrap()
        .operations
        .iter()
        .filter_map(|op| match op {
            WritebackOperation::Episode { episode, .. } => Some(episode.id.clone()),
            _ => None,
        })
        .collect();
    let mut stored = runtime.sessions().load(&session).await.unwrap().unwrap();
    let assistant = stored
        .log
        .iter()
        .find_map(|entry| match entry.event {
            SessionEventKind::TurnResponseCommitted { assistant_seq, .. } => Some(assistant_seq),
            _ => None,
        })
        .unwrap();
    stored
        .mask_message(
            assistant,
            "withdraw output before projection",
            writer_clock().as_ref(),
        )
        .unwrap();
    runtime.sessions().save(&stored).await.unwrap();
    backend
        .pool()
        .write_sync(|conn| {
            conn.execute_batch("DROP TRIGGER reject_memory")?;
            Ok(())
        })
        .unwrap();
    drop(runtime);
    drop(writer);
    drop(backend);
    drop(journal);

    let (runtime, _writer, backend, journal) = assemble(dir.path()).await;
    runtime
        .execute(TurnRequest::new(session, "处理其他事实。 "))
        .await
        .unwrap();
    assert_eq!(journal.status(&session.to_string()).unwrap().suppressed, 1);
    assert!(ids
        .iter()
        .all(|id| backend.get_episode(id).unwrap().is_none()));
}

#[tokio::test]
async fn typed_relation_retries_keep_the_frozen_id_and_new_operations_can_assert_again() {
    let dir = tempfile::tempdir().unwrap();
    let pool = Arc::new(
        SqliteConnectionPool::open(dir.path().join("memory.sqlite3"))
            .await
            .unwrap(),
    );
    apeireth_memory::run_migrations_on_pool(&pool)
        .await
        .unwrap();
    let relations = Arc::new(apeireth_memory::SqliteTemporalGraphStore::from_arc(
        pool.clone(),
    ));
    let sink = apeireth_runtime_assembly::CanonicalMemoryTypedSink::new()
        .with_relations(relations.clone());
    let candidate = apeireth_memory::MemoryTypedCandidate::Relation {
        candidate: apeireth_memory::RelationCandidate {
            subject_id: "owner".into(),
            predicate: "city".into(),
            object_id: "上海".into(),
            content: "我住在上海".into(),
            confidence: 1.0,
            provenance: apeireth_memory::MemoryProvenance::default(),
            scope: apeireth_memory::MemoryScope::session("owner"),
        },
    };
    let context = serde_json::json!({"source_episode_id":"committed-source"});
    sink.materialize_recovery(&candidate, "relation-operation-a", 1000, &context)
        .await
        .unwrap();
    sink.materialize_recovery(&candidate, "relation-operation-a", 1000, &context)
        .await
        .unwrap();
    assert!(
        sink.materialize_recovery(&candidate, "relation-operation-a", 2000, &context)
            .await
            .is_err(),
        "same operation cannot rewrite frozen time"
    );
    sink.materialize_recovery(&candidate, "relation-operation-b", 2000, &context)
        .await
        .unwrap();
    let rows = relations
        .query(&apeireth_memory::TemporalGraphQuery::new(3000))
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(relations.history(&rows[0].relation_key).unwrap().len(), 2);
    assert_eq!(
        pool.read(|conn| Ok(conn.query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM temporal_graph_facts",
            [],
            |r| r.get(0)
        )?))
        .unwrap(),
        2
    );
}

#[tokio::test]
async fn pending_accepted_correction_blocks_a_new_answer_until_the_frozen_plan_is_settled() {
    use apeireth_memory::automatic_correction::{
        correction_evidence_id, correction_operation_id, AutomaticCorrectionKind,
        AutomaticCorrectionPlan, AutomaticCorrectionTarget,
    };
    let dir = tempfile::tempdir().unwrap();
    let session = SessionId::new();
    let namespace = session.to_string();
    let (runtime, writer, backend, journal) = assemble(dir.path()).await;
    let coordinator = MemoryCoordinator::new(backend.clone(), backend.clone());
    coordinator
        .writeback_scoped_episode(
            &apeireth_core::Episode {
                id: "old-fact".into(),
                timestamp: 1_600_000_000,
                role: "user".into(),
                content: "我住在武汉。".into(),
                session_id: namespace.clone(),
            },
            &apeireth_memory::MemoryScope::session(&namespace),
            &apeireth_memory::MemoryProvenance {
                source_session: Some("old-session".into()),
                source_request: Some("old-request".into()),
                source_trace: Some("old-trace".into()),
                ..Default::default()
            },
        )
        .unwrap();
    runtime
        .execute(TurnRequest::new(session, "更正：我住在上海。 "))
        .await
        .unwrap();
    let source_json=backend.pool().read(|conn|Ok(conn.query_row::<String,_,_>("SELECT source_json FROM memory_writeback_jobs WHERE namespace=?1 AND id LIKE 'writeback-%' LIMIT 1",[&namespace],|r|r.get(0))?)).unwrap();
    let snapshot: apeireth_runtime::CommittedTurnSnapshot =
        serde_json::from_str(&source_json).unwrap();
    let request = snapshot.input.request.to_string();
    let recorded = snapshot.input.at.epoch_millis();
    let plan = AutomaticCorrectionPlan {
        operation_id: correction_operation_id(&namespace, &request),
        namespace: namespace.clone(),
        evidence: apeireth_core::Episode {
            id: correction_evidence_id(&namespace, &request),
            timestamp: recorded / 1000,
            role: "user".into(),
            content: "更正：我住在上海。 ".into(),
            session_id: namespace.clone(),
        },
        provenance: apeireth_memory::MemoryProvenance {
            source: "direct_user_correction".into(),
            source_session: Some(namespace.clone()),
            source_request: Some(request),
            source_trace: Some(snapshot.input.trace.to_string()),
        },
        authorization_epoch: None,
        kind: AutomaticCorrectionKind::CorrectPrior,
        valid_from_ms: recorded,
        recorded_at_ms: recorded,
        targets: vec![AutomaticCorrectionTarget {
            id: "old-fact".into(),
            expected_revision: 0,
            before_content: "我住在武汉。".into(),
            replacement_content: "我住在上海。".into(),
            source_session: "old-session".into(),
            source_request: "old-request".into(),
        }],
    };
    let payload = serde_json::to_value(&plan).unwrap();
    let intent = apeireth_runtime::CommittedOperationIntent {
        input: snapshot.input.clone(),
        operation_id: plan.operation_id.clone(),
        module_id: "cognitive.memory_correction".into(),
        payload_hash: apeireth_runtime::canonical::committed_evidence::operation_payload_hash(
            &payload,
        )
        .unwrap(),
        payload,
        prepared_at: snapshot.input.at,
        sequence: 0,
    };
    runtime
        .sessions()
        .store()
        .put_operation_evidence(&intent)
        .await
        .unwrap();
    backend.pool().write_sync(|conn|{conn.execute_batch("CREATE TRIGGER reject_correction BEFORE UPDATE OF content_override ON episode_governance BEGIN SELECT RAISE(ABORT,'injected correction failure'); END")?;Ok(())}).unwrap();
    drop(runtime);
    drop(writer);
    drop(backend);
    drop(journal);
    let (runtime, writer, backend, journal) = assemble(dir.path()).await;
    assert!(
        runtime
            .execute(TurnRequest::new(session, "我现在住在哪里？"))
            .await
            .is_err(),
        "unsettled accepted correction cannot silently answer from the older value"
    );
    assert!(journal.has_pending_correction(&namespace).unwrap());
    assert!(writer.ensure_corrections_settled(&session).is_err());
    assert_eq!(
        backend.get_episode("old-fact").unwrap().unwrap().content,
        "我住在武汉。"
    );
    backend
        .pool()
        .write_sync(|conn| {
            conn.execute_batch("DROP TRIGGER reject_correction")?;
            Ok(())
        })
        .unwrap();
    runtime
        .execute(TurnRequest::new(session, "再次检查现在的地址。 "))
        .await
        .unwrap();
    assert!(!journal.has_pending_correction(&namespace).unwrap());
    writer.ensure_corrections_settled(&session).unwrap();
    assert_eq!(
        backend.get_episode("old-fact").unwrap().unwrap().content,
        "我住在上海。"
    );
}

#[tokio::test]
async fn agent_task_body_stays_in_the_canonical_session_and_is_not_copied_into_the_journal() {
    use apeireth_runtime::{ModuleInvocationRequest, ModuleInvoker};
    let dir = tempfile::tempdir().unwrap();
    let session = SessionId::new();
    let (runtime, _writer, backend, journal) = assemble(dir.path()).await;
    let runtime = Arc::new(runtime);
    let completed = runtime
        .execute(TurnRequest::new(session, "ordinary direct input"))
        .await
        .unwrap();
    let invoker = apeireth_runtime::canonical::module::RuntimeModuleInvoker::new(
        runtime.clone(),
        apeireth_plugin::agents::AgentTaskOrigin {
            agent: apeireth_core::kernel::AgentId::new(),
            session,
            request: completed.request,
            trace: completed.trace.trace,
        },
        None,
        "fixture",
        "cognitive.memory_writeback",
        Arc::new(apeireth_runtime::canonical::module::ModuleTurnState::new(2)),
        0,
    );
    invoker
        .invoke(ModuleInvocationRequest::isolated(
            "isolated task",
            "TASK-ONLY-CANARY 我住在北京。 ",
        ))
        .await
        .unwrap();
    let sessions = runtime.sessions().store().list().await.unwrap();
    assert!(
        sessions.iter().any(|s| s.log.iter().any(|entry| matches!(
            entry.event,
            SessionEventKind::InputCommitted {
                origin: apeireth_runtime::CommittedInputOrigin::AgentTask,
                ..
            }
        )) && s.messages.iter().any(|m| {
            apeireth_protocol::canonical::ContentPart::join_text(&m.content)
                .contains("TASK-ONLY-CANARY")
        })),
        "formal Task evidence must remain auditable in its owning SessionStore"
    );
    let task = sessions
        .iter()
        .find(|s| {
            s.messages.iter().any(|m| {
                apeireth_protocol::canonical::ContentPart::join_text(&m.content)
                    .contains("TASK-ONLY-CANARY")
            })
        })
        .unwrap();
    let (response_index, response) = task
        .log
        .iter()
        .enumerate()
        .rev()
        .find(|(_, entry)| matches!(entry.event, SessionEventKind::TurnResponseCommitted { .. }))
        .unwrap();
    let reader = apeireth_runtime::canonical::module::RuntimeModuleInvoker::new(
        runtime.clone(),
        apeireth_plugin::agents::AgentTaskOrigin {
            agent: apeireth_core::kernel::AgentId::new(),
            session: task.id,
            request: response.request.unwrap(),
            trace: response.trace.unwrap(),
        },
        None,
        "fixture",
        "cognitive.memory_writeback",
        Arc::new(apeireth_runtime::canonical::module::ModuleTurnState::new(2)),
        0,
    );
    let page = reader.committed_turns(None, 64).await.unwrap();
    assert!(page.turns.iter().any(|turn|turn.input.origin==apeireth_runtime::CommittedInputOrigin::AgentTask&&turn.input.messages.iter().any(|m|apeireth_protocol::canonical::ContentPart::join_text(&m.content).contains("TASK-ONLY-CANARY"))),"the real committed source port must expose the completed Task to its authorized recovery owner");
    let watermark: serde_json::Value = serde_json::from_str(
        &journal
            .cursor(&task.id.to_string())
            .unwrap()
            .expect("actual Task recovery hook registered its scan watermark"),
    )
    .unwrap();
    assert!(watermark["streams"][format!("turn:{}",task.id)].as_u64().unwrap()>=response_index as u64,"the actual recovery hook scanned past the completed Task marker before the no-copy assertion");
    let copied=backend.pool().read(|conn|Ok(conn.query_row::<i64,_,_>("SELECT COUNT(*) FROM memory_writeback_jobs WHERE source_json LIKE '%TASK-ONLY-CANARY%'",[],|r|r.get(0))?)).unwrap();
    assert_eq!(
        copied, 0,
        "a skipped personal projection cannot introduce a plaintext task transcript copy"
    );
    assert_eq!(
        backend
            .pool()
            .read(|conn| Ok(conn.query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM episodes WHERE content LIKE '%TASK-ONLY-CANARY%'",
                [],
                |r| r.get(0)
            )?))
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn a_typed_sink_in_a_different_database_stays_pending_without_a_false_recovery_ack() {
    let dir = tempfile::tempdir().unwrap();
    let external = Arc::new(
        SqliteConnectionPool::open(dir.path().join("external.sqlite3"))
            .await
            .unwrap(),
    );
    apeireth_memory::run_migrations_on_pool(&external)
        .await
        .unwrap();
    let sink = Arc::new(
        apeireth_runtime_assembly::CanonicalMemoryTypedSink::new().with_relations(Arc::new(
            apeireth_memory::SqliteTemporalGraphStore::from_arc(external.clone()),
        )),
    );
    let session = SessionId::new();
    let (runtime, writer, backend, journal) =
        assemble_with_options(dir.path(), None, Some(sink.clone())).await;
    assert!(!sink.supports_recovery_journal(&journal));
    runtime
        .execute(TurnRequest::new(
            session,
            "relation: owner | city | Shanghai",
        ))
        .await
        .unwrap();
    let pending = journal.pending(&session.to_string(), 8).unwrap();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].plan.is_none());
    assert_eq!(pending[0].last_error.as_deref(), Some("prepare"));
    assert_eq!(writer.recovery_status(&session).unwrap().unwrap().done, 0);
    assert!(backend
        .recent_episodes(&session.to_string(), 8)
        .unwrap()
        .is_empty());
    assert_eq!(
        external
            .read(|conn| Ok(conn.query_row::<i64, _, _>(
                "SELECT COUNT(*) FROM temporal_graph_facts",
                [],
                |r| r.get(0)
            )?))
            .unwrap(),
        0
    );
    let common = Arc::new(SqliteConnectionPool::in_memory().await.unwrap());
    let common_journal = SqliteWritebackJournal::from_pool(common.clone());
    let shared = apeireth_runtime_assembly::CanonicalMemoryTypedSink::new().with_relations(
        Arc::new(apeireth_memory::SqliteTemporalGraphStore::from_arc(common)),
    );
    assert!(shared.supports_recovery_journal(&common_journal));
}

#[tokio::test]
async fn a_manual_cas_revision_atomically_invalidates_a_partial_plan_and_restart_cannot_refill_it()
{
    let dir = tempfile::tempdir().unwrap();
    let session = SessionId::new();
    let (runtime, writer, backend, journal) = assemble(dir.path()).await;
    backend.pool().write_sync(|conn|{conn.execute_batch("CREATE TRIGGER reject_derived BEFORE INSERT ON episodes WHEN NEW.role='memory' BEGIN SELECT RAISE(ABORT,'injected derived failure'); END")?;Ok(())}).unwrap();
    runtime
        .execute(TurnRequest::new(
            session,
            "我住在武汉。REVISION-PENDING-SOURCE",
        ))
        .await
        .unwrap();
    let pending = journal.pending(&session.to_string(), 8).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].next_operation, 2);
    let plan = pending[0].plan.as_ref().unwrap();
    let user = plan
        .operations
        .iter()
        .find_map(|op| match op {
            WritebackOperation::Episode { episode, .. } if episode.role == "user" => {
                Some(episode.clone())
            }
            _ => None,
        })
        .unwrap();
    let derived: Vec<_> = plan
        .operations
        .iter()
        .filter_map(|op| match op {
            WritebackOperation::DerivedEpisode { episode, .. } => Some(episode.id.clone()),
            _ => None,
        })
        .collect();
    assert!(!derived.is_empty());
    backend
        .pool()
        .write_sync(|conn| {
            conn.execute_batch("DROP TRIGGER reject_derived")?;
            Ok(())
        })
        .unwrap();
    let coordinator = MemoryCoordinator::new(backend.clone(), backend.clone());
    coordinator
        .writeback_scoped_episode_with_metadata(
            &apeireth_core::Episode {
                id: "previous-derived".into(),
                timestamp: user.timestamp,
                role: "memory".into(),
                content: "我住在武汉。".into(),
                session_id: user.session_id.clone(),
            },
            &apeireth_memory::MemoryScope::session(&user.session_id),
            &apeireth_memory::MemoryProvenance::default(),
            serde_json::json!({"derived_from_episode_id":user.id}),
        )
        .unwrap();
    backend.pool().write_sync(|conn|{conn.execute_batch("CREATE TRIGGER reject_revision_invalidation BEFORE UPDATE OF state ON memory_writeback_jobs WHEN NEW.last_error='source_revision_changed' BEGIN SELECT RAISE(ABORT,'injected invalidation failure'); END")?;Ok(())}).unwrap();
    assert!(backend
        .update_episode_content(&user.id, "我住在上海。", Some("panel"), 0)
        .is_err());
    assert_eq!(backend.get_governed(&user.id).unwrap().unwrap().revision, 0);
    assert_eq!(journal.status(&session.to_string()).unwrap().pending, 1);
    assert_eq!(
        backend
            .get_governed("previous-derived")
            .unwrap()
            .unwrap()
            .status
            .as_str(),
        "active"
    );
    backend
        .pool()
        .write_sync(|conn| {
            conn.execute_batch("DROP TRIGGER reject_revision_invalidation")?;
            Ok(())
        })
        .unwrap();
    backend
        .update_episode_content(&user.id, "我住在上海。", Some("panel"), 0)
        .unwrap();
    assert_eq!(journal.status(&session.to_string()).unwrap().suppressed, 1);
    assert_eq!(
        journal
            .status(&session.to_string())
            .unwrap()
            .last_error
            .as_deref(),
        Some("source_revision_changed")
    );
    assert_eq!(
        backend
            .get_governed("previous-derived")
            .unwrap()
            .unwrap()
            .status
            .as_str(),
        "forgotten"
    );
    drop(runtime);
    drop(writer);
    drop(backend);
    drop(journal);
    drop(coordinator);
    let (runtime, _writer, backend, journal) = assemble(dir.path()).await;
    runtime
        .execute(TurnRequest::new(session, "我偏好独立的正常新记录。 "))
        .await
        .unwrap();
    assert_eq!(
        backend.get_episode(&user.id).unwrap().unwrap().content,
        "我住在上海。"
    );
    assert!(derived
        .iter()
        .all(|id| backend.get_episode(id).unwrap().is_none()));
    assert!(backend.get_episode("previous-derived").unwrap().is_none());
    assert_eq!(journal.status(&session.to_string()).unwrap().pending, 0);
    assert!(
        journal.status(&session.to_string()).unwrap().done >= 1,
        "another independent completed source still progresses"
    );
}
