//! Real lifecycle operations precede grading; their evidence is never a prompt.
use std::path::Path;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{Episode, SessionId, VirtualClock};
use apeireth_memory::writeback_recovery::{SqliteWritebackJournal, WritebackOperation};
use apeireth_memory::{EpisodeStore, MemoryGovernanceStore, SqliteMemoryStore};
use apeireth_runtime::canonical::{Runtime, TurnRequest, TurnResponse};
use serde::{Deserialize, Serialize};

use super::audit::SourceAudit;
use super::capture::{CallContext, CallRecord};
use super::fixtures::{Lifecycle, ScenarioInput, BASIC_SEED, SEED_SYSTEM, SYSTEM};
use super::io::{hash, write_new, EvalResult};
use super::runner::{validate_fiction, Environment, InvalidSetup, RuntimeState};

#[derive(Default, Deserialize, Serialize)]
pub struct LifecycleProof {
    pub issues: Vec<String>,
    pub observations: Vec<serde_json::Value>,
}

async fn journal(db: &Path) -> EvalResult<SqliteWritebackJournal> {
    let pool = Arc::new(apeireth_storage::SqliteConnectionPool::open(db).await?);
    apeireth_memory::run_migrations_on_pool(&pool).await?;
    Ok(SqliteWritebackJournal::from_pool(pool))
}

fn status(journal: &SqliteWritebackJournal, namespace: &str) -> EvalResult<serde_json::Value> {
    let status = journal.status(namespace)?;
    Ok(
        serde_json::json!({"pending":status.pending,"done":status.done,"suppressed":status.suppressed,"attempts":status.attempts,"last_error":status.last_error}),
    )
}

async fn turn(
    environment: &Environment,
    runtime: &Runtime,
    state: &RuntimeState,
    scenario: &str,
    repetition: usize,
    phase: &str,
    text: &str,
    system: &str,
) -> EvalResult<TurnResponse> {
    state.context.lock().unwrap().phase = phase.into();
    let response = runtime
        .execute(TurnRequest::new(state.session, text).with_system(system))
        .await?;
    {
        let mut entries = state.audit.entries.lock().unwrap();
        let mut matching = entries.iter_mut().filter(|entry| {
            entry.input.request == response.request && entry.input.session == state.session
        });
        let entry = matching
            .next()
            .ok_or("completed turn has no independent Runtime input marker")?;
        entry.assistant_sha256 = Some(hash(&response.text));
        if matching.next().is_some() {
            return Err("duplicate Runtime input markers".into());
        }
    }
    write_new(
        &environment.config.output.join(format!(
            "source-audit-{scenario}-{repetition:03}-{phase}.json"
        )),
        &*state.audit.entries.lock().unwrap(),
    )?;
    Ok(response)
}

fn forget_all(db: &Path, session: SessionId, proof: &mut LifecycleProof) -> EvalResult<()> {
    let store = SqliteMemoryStore::open(db)?;
    let rows = EpisodeStore::recent_episodes(&store, &session.to_string(), 1000)?;
    if rows.is_empty() || rows.len() >= 1000 {
        return Err(
            InvalidSetup("forget operation lacks a bounded nonempty source set".into()).into(),
        );
    }
    let mut forgotten = Vec::new();
    for row in rows {
        let current = store
            .get_governed(&row.id)?
            .ok_or("forget source missing")?;
        if current.status.as_str() == "active" {
            forgotten.push(store.forget_episode(
                &row.id,
                Some("evaluation owner explicitly withdrew this source"),
                current.revision,
            )?);
        }
    }
    proof
        .observations
        .push(serde_json::json!({"operation":"formal_forget","rows":forgotten}));
    Ok(())
}

fn sql(db: &Path, statement: &str) -> EvalResult<()> {
    rusqlite::Connection::open(db)?.execute_batch(statement)?;
    Ok(())
}

pub async fn execute_case(
    environment: &Environment,
    scenario: &str,
    repetition: usize,
    input: &ScenarioInput,
) -> EvalResult<CallRecord> {
    let db = environment
        .config
        .output
        .join(format!("{scenario}-{repetition:03}.sqlite3"));
    let mut state = RuntimeState {
        session: SessionId::new(),
        context: Arc::new(Mutex::new(CallContext {
            scenario: scenario.into(),
            repetition,
            phase: "setup".into(),
            kind: "unclassified".into(),
        })),
        last_record: Arc::new(Mutex::new(None)),
        audit: Arc::new(SourceAudit::default()),
        forbidden: vec![],
        clock: Arc::new(VirtualClock::new(
            chrono::DateTime::parse_from_rfc3339(&input.fixed_time_utc)?
                .with_timezone(&chrono::Utc),
        )),
        read_only: false,
        persistent_sessions: matches!(input.lifecycle, Lifecycle::Recovery { .. }),
        recovery_only: false,
    };
    let mut proof = LifecycleProof::default();
    proof.observations.push(serde_json::json!({"operation":"isolation_configuration","case_root":environment.config.output,"memory_db":db,"memory_data_root":db.with_extension("isolated-memory-data"),"official_session_db":db.with_extension("sessions.sqlite3")}));
    let runtime = environment.runtime(&db, input, &state).await?;
    let recovery = journal(&db).await?;
    if let Lifecycle::Recovery { forget_pending } = input.lifecycle {
        sql(
            &db,
            if forget_pending {
                "CREATE TRIGGER eval_reject_assistant BEFORE INSERT ON episodes WHEN NEW.id LIKE '%-assistant' BEGIN SELECT RAISE(ABORT,'evaluation partial write failure'); END"
            } else {
                "CREATE TRIGGER eval_reject_all BEFORE INSERT ON episodes BEGIN SELECT RAISE(ABORT,'evaluation write failure'); END"
            },
        )?;
    }
    let setup_system = if matches!(input.lifecycle, Lifecycle::SameTurnCorrection) {
        SYSTEM
    } else {
        SEED_SYSTEM
    };
    let mut non_assertion_requests = Vec::new();
    for (index, text) in input.setup_turns.iter().enumerate() {
        if index == 1 && matches!(input.lifecycle, Lifecycle::StateChangeHistory) {
            state.clock.set(
                chrono::DateTime::parse_from_rfc3339("2026-11-12T00:00:00Z")?
                    .with_timezone(&chrono::Utc),
            );
        }
        let response = turn(
            environment,
            &runtime,
            &state,
            scenario,
            repetition,
            &format!("setup-{index}"),
            text,
            setup_system,
        )
        .await?;
        if input.non_assertion_turns.contains(&index) {
            non_assertion_requests.push(response.request.to_string());
        }
        if let Lifecycle::FictionSetup {
            turn,
            subject,
            predicate,
            value,
        } = &input.lifecycle
        {
            if index == *turn {
                validate_fiction(&response.text, subject, predicate, value)?;
            }
        }
    }
    if matches!(input.lifecycle, Lifecycle::SameTurnCorrection) {
        turn(
            environment,
            &runtime,
            &state,
            scenario,
            repetition,
            "query",
            &input.question,
            SYSTEM,
        )
        .await?;
        drop(runtime);
    } else {
        drop(runtime);
        match &input.lifecycle {
            Lifecycle::ForeignSession => {
                state.session = SessionId::new();
                state.forbidden = vec!["SESSIONONLY6F2A".into()];
            }
            Lifecycle::ForgetAll => {
                forget_all(&db, state.session, &mut proof)?;
                state.forbidden = vec!["FORGETONLY8B4C".into()];
            }
            Lifecycle::Recovery { forget_pending } => {
                let pending = recovery.pending(&state.session.to_string(), 32)?;
                if pending.len() != 1 || pending[0].plan.is_none() {
                    return Err(InvalidSetup(
                        "injected write failure did not leave exactly one frozen durable job"
                            .into(),
                    )
                    .into());
                }
                let expected: Vec<Episode> = pending[0]
                    .plan
                    .as_ref()
                    .unwrap()
                    .operations
                    .iter()
                    .filter_map(|op| match op {
                        WritebackOperation::Episode { episode, .. } => Some(episode.clone()),
                        _ => None,
                    })
                    .collect();
                if expected.len() != 2 {
                    return Err(InvalidSetup(
                        "recovery plan must freeze user and assistant evidence".into(),
                    )
                    .into());
                }
                proof.observations.push(serde_json::json!({"operation":"failed_write","pending":pending,"status":status(&recovery,&state.session.to_string())?}));
                if *forget_pending {
                    let user = expected
                        .iter()
                        .find(|row| row.role == "user")
                        .ok_or("frozen user source missing")?;
                    let store = SqliteMemoryStore::open(&db)?;
                    let current = store
                        .get_governed(&user.id)?
                        .ok_or("partial user write was not committed")?;
                    let hidden = store.forget_episode(
                        &user.id,
                        Some("owner withdrew the partially committed source"),
                        current.revision,
                    )?;
                    proof.observations.push(serde_json::json!({"operation":"withdraw_partial_source","governance":hidden}));
                    state.forbidden = vec!["PENDINGFORGET7C9D".into()];
                    sql(&db, "DROP TRIGGER eval_reject_assistant")?;
                } else {
                    sql(&db, "DROP TRIGGER eval_reject_all")?;
                }
                state.clock.advance(chrono::Duration::days(1));
                for pass in 0..if *forget_pending { 2 } else { 1 } {
                    state.recovery_only = true;
                    let official = environment.runtime(&db, input, &state).await?;
                    state.context.lock().unwrap().phase = format!("recover-{pass}");
                    match official.execute(TurnRequest::new(state.session,"请处理已提交的后台记忆待办，不新增个人事实。").with_system(SEED_SYSTEM)).await {
                        Err(apeireth_runtime::canonical::RuntimeError::ModuleStopped{module_id,reason}) if module_id=="eval.recovery_stopper" && reason=="evaluation recovery control completed without a primary answer"=>{},
                        other=>return Err(format!("recovery control did not stop at its registered boundary: {other:?}").into()),
                    }
                    drop(official);
                    proof.observations.push(serde_json::json!({"operation":"official_restart_recovery","pass":pass,"status":status(&recovery,&state.session.to_string())?}));
                }
                if recovery.status(&state.session.to_string())?.pending != 0 {
                    proof.issues.push("recovery_left_pending_jobs".into());
                }
                let store = SqliteMemoryStore::open(&db)?;
                if *forget_pending {
                    if recovery.status(&state.session.to_string())?.suppressed == 0 {
                        proof.issues.push("withdrawn_job_was_not_suppressed".into());
                    }
                    for row in expected.iter().filter(|row| row.role == "assistant") {
                        if EpisodeStore::get_episode(&store, &row.id)?.is_some() {
                            proof
                                .issues
                                .push("withdrawn_pending_assistant_was_resurrected".into());
                        }
                    }
                } else {
                    for original in &expected {
                        let restored = EpisodeStore::get_episode(&store, &original.id)?;
                        if !restored.is_some_and(|row| {
                            row.content == original.content
                                && row.timestamp == original.timestamp
                                && row.session_id == original.session_id
                                && row.role == original.role
                        }) {
                            proof.issues.push(
                                "frozen_source_identity_or_timestamp_was_not_restored".into(),
                            );
                        }
                    }
                }
            }
            _ => {}
        }
        state.read_only = true;
        state.persistent_sessions = false;
        state.recovery_only = false;
        let cold = environment.runtime(&db, input, &state).await?;
        if cold.sessions().load(&state.session).await?.is_some() {
            return Err("cold query retained an old transcript".into());
        }
        turn(
            environment,
            &cold,
            &state,
            scenario,
            repetition,
            "query",
            &input.question,
            SYSTEM,
        )
        .await?;
        drop(cold);
    }
    let mut record = state
        .last_record
        .lock()
        .unwrap()
        .clone()
        .ok_or("query never reached provider")?;
    if record.context.phase != "query" || record.context.kind != "main" {
        return Err("last receipt is not the primary query".into());
    }
    if !matches!(input.lifecycle, Lifecycle::SameTurnCorrection) {
        for message in &record.original_request.messages {
            if message.role == apeireth_protocol::canonical::MessageRole::User
                && input.setup_turns.iter().any(|text| {
                    apeireth_protocol::canonical::ContentPart::join_text(&message.content) == *text
                })
            {
                proof
                    .issues
                    .push("setup_user_transcript_replayed_in_cold_query".into());
            }
        }
    }
    let store = SqliteMemoryStore::open(&db)?;
    let rows = EpisodeStore::recent_episodes(&store, &state.session.to_string(), 1000)?;
    let mut governed = Vec::new();
    let mut metadata = Vec::new();
    for row in &rows {
        if let Some(current) = store.get_governed(&row.id)? {
            governed.push(current);
        }
        if let Some(meta) =
            apeireth_plugin::memory_backend::MemoryBackend::get_episode_metadata(&store, &row.id)?
        {
            if matches!(meta["extraction_class"].as_str(), Some("Fact" | "fact")) {
                let quoted = meta["provenance"]["source_request"]
                    .as_str()
                    .is_some_and(|id| non_assertion_requests.iter().any(|request| request == id));
                let assistant = meta["derived_from_episode_id"]
                    .as_str()
                    .map(|id| EpisodeStore::get_episode(&store, id))
                    .transpose()?
                    .flatten()
                    .is_some_and(|source| source.role == "assistant");
                if quoted || assistant {
                    proof
                        .issues
                        .push("nonassertion_or_assistant_was_promoted_to_fact".into());
                }
            }
            metadata.push(serde_json::json!({"id":row.id,"metadata":meta}));
        }
    }
    if matches!(
        input.lifecycle,
        Lifecycle::SameTurnCorrection | Lifecycle::StateChangeHistory
    ) {
        let corrected = BASIC_SEED.replace("北岚市", "南汀市");
        if !rows.iter().any(|row| {
            row.content == BASIC_SEED
                && governed.iter().any(|g| {
                    g.episode.id == row.id
                        && g.revision > 0
                        && g.content_override.as_deref() == Some(corrected.as_str())
                })
        }) {
            proof
                .issues
                .push("automatic_correction_did_not_revise_the_original_fact".into());
        }
    }
    if matches!(input.lifecycle,Lifecycle::StateChangeHistory) && !rows.iter().any(|row|row.role=="fact_history" && row.content==format!("历史状态（已结束，不代表当前），有效区间：[2026-11-01T00:00:00.000Z, 2026-11-10T00:00:00.000Z)：{BASIC_SEED}")) {proof.issues.push("state_change_history_or_interval_missing".into());}
    proof.observations.push(
        serde_json::json!({"operation":"final_sql_audit","governed":governed,"metadata":metadata}),
    );
    record
        .request_evidence
        .issues
        .extend(proof.issues.iter().cloned());
    write_new(
        &environment
            .config
            .output
            .join(format!("lifecycle-{scenario}-{repetition:03}.json")),
        &proof,
    )?;
    Ok(record)
}
