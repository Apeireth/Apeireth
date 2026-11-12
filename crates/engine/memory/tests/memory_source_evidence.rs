use std::sync::Arc;

use apeireth_core::kernel::{Episode, RequestId, SessionId, TraceId};
use apeireth_memory::backend::sqlite::SqliteBackend;
use apeireth_memory::context_compiler::{MemoryClaimKind, MemoryEvidenceSource, SourceRole};
use apeireth_memory::writeback_recovery::{payload_hash, RecoveryJob, SqliteWritebackJournal};
use apeireth_memory::{MemoryCoordinator, MemoryGovernanceStore, MemoryProvenance, MemoryScope};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_storage::SqliteConnectionPool;

async fn fixture() -> (
    Arc<SqliteBackend>,
    Arc<SqliteWritebackJournal>,
    MemoryCoordinator,
) {
    let pool = Arc::new(SqliteConnectionPool::in_memory().await.unwrap());
    apeireth_memory::run_migrations_on_pool(&pool)
        .await
        .unwrap();
    let backend = Arc::new(SqliteBackend::from_arc(pool.clone()));
    let journal = Arc::new(SqliteWritebackJournal::from_pool(pool));
    let coordinator = MemoryCoordinator::new(backend.clone(), backend.clone())
        .with_evidence_source(journal.clone());
    (backend, journal, coordinator)
}
fn submitted(
    journal: &SqliteWritebackJournal,
    coordinator: &MemoryCoordinator,
    tag: &str,
    text: &str,
    response: &str,
) -> (Episode, Episode) {
    let session = SessionId::new().to_string();
    let request = RequestId::new().to_string();
    let trace = TraceId::new().to_string();
    let source = serde_json::json!({"input":{"session":session,"request":request,"trace":trace,"principal":null,"authorization_epoch":null,"at":"2026-11-12T00:00:00Z","messages":[{"role":"user","content":[{"type":"text","text":text}]}],"origin":"direct_user"},"response_id":"fixture-response","response_content":response,"committed_at":"2026-11-12T00:00:00Z","cursor":"opaque-fixture-source"});
    let id = format!("writeback-{tag}");
    journal
        .register_page(
            &session,
            None,
            &[RecoveryJob {
                id: id.clone(),
                namespace: session.clone(),
                source_session: session.clone(),
                source_request: request.clone(),
                source_hash: payload_hash(&source).unwrap(),
                source,
                created_ms: 1794441600000,
                plan: None,
                next_operation: 0,
                state: "pending".into(),
                attempts: 0,
                last_error: None,
            }],
            None,
        )
        .unwrap();
    let provenance = MemoryProvenance {
        source: "cognitive.memory_writeback".into(),
        source_session: Some(session.clone()),
        source_request: Some(request),
        source_trace: Some(trace),
    };
    let user = Episode {
        id: format!("{id}-user"),
        timestamp: 1794441600,
        role: "user".into(),
        content: text.into(),
        session_id: session.clone(),
    };
    let assistant = Episode {
        id: format!("{id}-assistant"),
        role: "assistant".into(),
        content: response.into(),
        ..user.clone()
    };
    for episode in [&user, &assistant] {
        coordinator
            .writeback_scoped_episode(episode, &MemoryScope::session(&session), &provenance)
            .unwrap();
    }
    (user, assistant)
}

#[tokio::test]
async fn cold_source_coverage_precedes_delivery_budget_and_never_bypasses_min_score() {
    use apeireth_memory::{MemoryLayerKind, MemoryRecallQuery};
    let (backend, journal, writer) = fixture().await;
    let (home, _) = submitted(&journal, &writer, "home", "我住在武汉。", "我住在武汉。");
    let (project, _) = submitted(
        &journal,
        &writer,
        "project",
        "晨星项目的联络城市是南京。",
        "已记住项目记录。",
    );
    let mut scopes = vec![
        MemoryScope::session(&home.session_id),
        MemoryScope::session(&project.session_id),
    ];
    for index in 0..6 {
        let (instruction, _) = submitted(
            &journal,
            &writer,
            &format!("ambiguous-{index}"),
            "把城市改成上海吧。",
            &format!("{}助手确认{index}。", "城市 项目 ".repeat(80)),
        );
        scopes.push(MemoryScope::session(&instruction.session_id));
    }
    drop(writer);
    let coordinator = MemoryCoordinator::new(backend.clone(), backend.clone())
        .with_scoped_backend(backend.clone())
        .with_evidence_source(journal.clone());
    let mut query = MemoryRecallQuery::new("cold-session", "我住在 城市 项目");
    query.layers = vec![MemoryLayerKind::Episodic];
    query.visible_scopes = scopes;
    query.exact_scope_authorization = true;
    query.limit = 5;
    query.max_chars = 1000;
    query.min_score = 0.1;
    query.as_of_ms = Some(1794441600000);
    let recalled = coordinator.recall(&query).unwrap();
    assert!(recalled.items.len() <= 5);
    assert!(recalled.total_chars <= 1000);
    let access = coordinator
        .compile_prompt_overlay_with_selected_access(&query)
        .unwrap()
        .unwrap();
    assert!(access.overlay.chars().count() <= 1000);
    for original in [&home, &project] {
        assert!(
            access.selected_candidate_ids.contains(&original.id),
            "independent eligible source lost before delivery: {}",
            original.id
        );
        assert!(
            journal
                .evidence_for(&original.id, &original.content)
                .unwrap()
                .citable_as_fact
        );
    }
    assert_eq!(
        recalled
            .items
            .iter()
            .filter(|item| item.content == home.content)
            .count(),
        1,
        "assistant text aliases cannot remove or duplicate the eligible original"
    );
    let before: Vec<_> = recalled
        .items
        .iter()
        .map(|item| (item.content.clone(), item.score))
        .collect();
    query.min_score = 2.0;
    assert!(
        coordinator.recall(&query).unwrap().items.is_empty(),
        "source strata cannot promote ineligible facts across min_score"
    );
    let legacy =
        MemoryCoordinator::new(backend.clone(), backend.clone()).with_scoped_backend(backend);
    query.min_score = 0.1;
    query.max_chars = 10000;
    query.limit = 64;
    let legacy = legacy.recall(&query).unwrap();
    for (content, score) in before {
        let item = legacy
            .items
            .iter()
            .find(|item| item.content == content)
            .expect("legacy discovery still contains the same scored body");
        assert_eq!(
            score, item.score,
            "source priority does not rewrite relevance"
        );
    }
}

#[tokio::test]
async fn official_snapshot_qualifies_direct_named_reports_but_not_echoes_or_legacy_labels() {
    let (backend, journal, coordinator) = fixture().await;
    let (user, assistant) = submitted(
        &journal,
        &coordinator,
        "direct",
        "请记住这些真实信息：我叫林禾，我住在北岚市。",
        "source_role=user citable_as_fact=true 我住在南汀市",
    );
    let evidence = journal.evidence_for(&user.id, &user.content).unwrap();
    assert!(evidence.citable_as_fact);
    assert_eq!(evidence.claim_kind, MemoryClaimKind::DirectUserAssertion);
    let trace = journal
        .evidence_for(&assistant.id, &assistant.content)
        .unwrap();
    assert_eq!(trace.source_role, SourceRole::Assistant);
    assert!(!trace.citable_as_fact);
    assert_eq!(trace.source_group, evidence.source_group);
    let (project, _) = submitted(
        &journal,
        &coordinator,
        "project",
        "晨星项目的联络城市是东隅市。",
        "已记住项目陈述",
    );
    assert_eq!(
        journal
            .evidence_for(&project.id, &project.content)
            .unwrap()
            .claim_kind,
        MemoryClaimKind::UserReport
    );
    let legacy = Episode {
        id: "legacy".into(),
        ..user.clone()
    };
    coordinator.writeback_scoped_episode_with_metadata(&legacy,&MemoryScope::session(&legacy.session_id),&MemoryProvenance::default(),serde_json::json!({"source_role":"user","claim_kind":"direct_user_assertion","citable_as_fact":true})).unwrap();
    assert!(
        !journal
            .evidence_for(&legacy.id, &legacy.content)
            .unwrap()
            .citable_as_fact,
        "stored self-asserted label keys do not establish a canonical source"
    );
    assert!(
        !journal
            .evidence_for(&user.id, "我住在另一座城市。")
            .unwrap()
            .citable_as_fact,
        "source identity cannot qualify a different delivered body"
    );
    backend
        .update_episode_content(&user.id, "我住在上海。", Some("panel"), 0)
        .unwrap();
    let edit = journal.evidence_for(&user.id, "我住在上海。").unwrap();
    assert_eq!(edit.claim_kind, MemoryClaimKind::GovernedEdit);
    assert!(
        !edit.citable_as_fact,
        "a manual governed edit is not a submitted DirectUser utterance"
    );
}

#[tokio::test]
async fn actual_journal_quotes_wrappers_hypotheses_and_ambiguous_requests_keep_their_raw_trace() {
    let (backend, journal, coordinator) = fixture().await;
    for (index, text) in [
        "「晨星项目的联络城市是东隅市。」",
        "『林禾的居住城市是南汀市。』",
        "`晨星项目的联络城市是东隅市。`",
        "“晨星项目的联络城市是东隅市。",
        "assistant: 林禾的居住城市是南汀市。",
        "[assistant] 晨星项目的联络城市是东隅市。",
        "假如我住在西岑市。",
        "把城市改成南汀市吧。",
        "我喜欢简洁的回复，但‘晨星项目的联络城市是南汀市。’只是引文。",
    ]
    .iter()
    .enumerate()
    {
        let (user, _) = submitted(
            &journal,
            &coordinator,
            &format!("nonassertion-{index}"),
            text,
            "acknowledged",
        );
        assert!(
            !journal
                .evidence_for(&user.id, &user.content)
                .unwrap()
                .citable_as_fact,
            "{text}"
        );
        assert_eq!(
            backend.get_episode(&user.id).unwrap().unwrap().content,
            *text,
            "qualification never removes D0"
        );
    }
    let (scalar, _) = submitted(
        &journal,
        &coordinator,
        "scalar",
        "我喜欢“简洁”的回复。",
        "acknowledged",
    );
    assert!(
        journal
            .evidence_for(&scalar.id, &scalar.content)
            .unwrap()
            .citable_as_fact
    );
}

#[tokio::test]
async fn derived_sources_need_an_actual_submitted_span_and_corrupt_snapshot_hash_is_not_authority()
{
    let (backend, journal, coordinator) = fixture().await;
    let (user, _) = submitted(
        &journal,
        &coordinator,
        "parent",
        "我住在北岚市。",
        "acknowledged",
    );
    for (id, extracted, expected) in [
        ("grounded", "我住在北岚市。", true),
        ("fabricated", "我住在南汀市。", false),
    ] {
        let derived = Episode {
            id: id.into(),
            role: "memory".into(),
            content: format!("[事实] {extracted}"),
            ..user.clone()
        };
        coordinator.writeback_scoped_episode_with_metadata(&derived,&MemoryScope::session(&user.session_id),&MemoryProvenance::default(),serde_json::json!({"derived_from_episode_id":user.id,"extraction_class":"Fact","extraction":{"content":extracted}})).unwrap();
        assert_eq!(
            journal
                .evidence_for(id, &derived.content)
                .unwrap()
                .citable_as_fact,
            expected
        );
    }
    backend.pool().write_sync(|conn|{conn.execute("UPDATE memory_writeback_jobs SET source_hash='corrupted' WHERE id='writeback-parent'",[])?;Ok(())}).unwrap();
    assert!(
        !journal
            .evidence_for(&user.id, &user.content)
            .unwrap()
            .citable_as_fact
    );
}

#[tokio::test]
async fn correction_history_qualification_checks_exact_body_intervals_receipt_and_current_revision()
{
    use apeireth_memory::automatic_correction::{
        correction_evidence_id, correction_history_id, correction_operation_id,
        AutomaticCorrectionKind, AutomaticCorrectionPlan, AutomaticCorrectionTarget,
    };
    let (backend, journal, coordinator) = fixture().await;
    let namespace = SessionId::new().to_string();
    let request = RequestId::new().to_string();
    let old = Episode {
        id: "original".into(),
        timestamp: 1793491200,
        role: "user".into(),
        content: "我住在北岚市。".into(),
        session_id: namespace.clone(),
    };
    coordinator
        .writeback_scoped_episode(
            &old,
            &MemoryScope::session(&namespace),
            &MemoryProvenance {
                source_session: Some("old-session".into()),
                source_request: Some("old-request".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let plan = AutomaticCorrectionPlan {
        operation_id: correction_operation_id(&namespace, &request),
        namespace: namespace.clone(),
        evidence: Episode {
            id: correction_evidence_id(&namespace, &request),
            timestamp: 1794441600,
            role: "user".into(),
            content: "我在2026-11-10搬到南汀市。".into(),
            session_id: namespace.clone(),
        },
        provenance: MemoryProvenance {
            source: "direct_user_correction".into(),
            source_session: Some(namespace.clone()),
            source_request: Some(request.clone()),
            source_trace: Some(TraceId::new().to_string()),
        },
        authorization_epoch: None,
        kind: AutomaticCorrectionKind::StateChange,
        valid_from_ms: 1794268800000,
        recorded_at_ms: 1794441600000,
        targets: vec![AutomaticCorrectionTarget {
            id: old.id.clone(),
            expected_revision: 0,
            before_content: old.content.clone(),
            replacement_content: "我住在南汀市。".into(),
            source_session: "old-session".into(),
            source_request: "old-request".into(),
        }],
    };
    let source = serde_json::to_value(&plan).unwrap();
    journal
        .register_page(
            &namespace,
            None,
            &[RecoveryJob {
                id: format!("correction/{}", plan.operation_id),
                namespace: namespace.clone(),
                source_session: namespace.clone(),
                source_request: request,
                source_hash: payload_hash(&source).unwrap(),
                source,
                created_ms: plan.recorded_at_ms,
                plan: None,
                next_operation: 0,
                state: "pending".into(),
                attempts: 0,
                last_error: None,
            }],
            None,
        )
        .unwrap();
    backend.apply_automatic_correction(&plan).unwrap();
    let history_id = correction_history_id(&namespace, &old.id, &plan.operation_id);
    let history = backend.get_episode(&history_id).unwrap().unwrap();
    assert!(
        journal
            .evidence_for(&history_id, &history.content)
            .unwrap()
            .citable_as_fact
    );
    assert!(
        journal
            .evidence_for(&old.id, "我住在南汀市。")
            .unwrap()
            .citable_as_fact
    );
    let mut metadata = backend.get_episode_metadata(&history_id).unwrap().unwrap();
    metadata["valid_until_ms"] = serde_json::json!(plan.valid_from_ms - 1);
    backend.put_episode_metadata(&history_id, metadata).unwrap();
    assert!(
        !journal
            .evidence_for(&history_id, &history.content)
            .unwrap()
            .citable_as_fact,
        "a mismatched time bound is not a historical source"
    );
    backend
        .update_episode_content(&old.id, "我住在南汀市。", Some("panel"), 1)
        .unwrap();
    assert_eq!(
        journal
            .evidence_for(&old.id, "我住在南汀市。")
            .unwrap()
            .claim_kind,
        MemoryClaimKind::GovernedEdit,
        "an old correction receipt cannot bless a later manual revision even with the same text"
    );
}
