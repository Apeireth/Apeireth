//! Transactional application of a frozen, Host-validated user correction.
//! Semantic proposals have no storage authority. The assembly validates their
//! source and spans; this boundary rechecks the durable identity and CAS state.

use apeireth_core::Episode;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::memory_chain::{
    get_episode_metadata, put_chained_episode_in_transaction, put_episode_metadata, ChainEntry,
};
use crate::memory_governance::{ensure_governance_on_conn, get_governed_impl};
use crate::writeback_recovery::{operation_applied, payload_hash, record_operation_applied};
use crate::{
    MemoryGovernanceError, MemoryGovernanceStatus, MemoryProvenance, MemoryScope, TemporalGraphFact,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutomaticCorrectionKind {
    CorrectPrior,
    StateChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomaticCorrectionTarget {
    pub id: String,
    pub expected_revision: i64,
    pub before_content: String,
    pub replacement_content: String,
    pub source_session: String,
    pub source_request: String,
}

/// The complete operation is frozen before persistence and never regenerated
/// from a later model response during recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomaticCorrectionPlan {
    pub operation_id: String,
    pub namespace: String,
    pub evidence: Episode,
    pub provenance: MemoryProvenance,
    pub authorization_epoch: Option<String>,
    pub kind: AutomaticCorrectionKind,
    pub valid_from_ms: i64,
    pub recorded_at_ms: i64,
    pub targets: Vec<AutomaticCorrectionTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutomaticCorrectionOutcome {
    pub replayed: bool,
}

pub fn correction_operation_id(namespace: &str, request: &str) -> String {
    stable_id("memory.correction.v1", &[namespace, request])
}

pub fn correction_evidence_id(namespace: &str, request: &str) -> String {
    stable_id("memory.correction.evidence.v1", &[namespace, request])
}

/// The time-graph relation is private to one original evidence identity.
pub fn correction_relation_key(namespace: &str, episode: &str) -> String {
    stable_id("memory.correction.relation.v1", &[namespace, episode])
}

pub fn correction_history_id(namespace: &str, episode: &str, operation: &str) -> String {
    stable_id(
        "memory.correction.history.v1",
        &[namespace, episode, operation],
    )
}

fn stable_id(domain: &str, parts: &[&str]) -> String {
    let input = serde_json::to_vec(&(domain, parts)).expect("string tuples serialize");
    format!("{:x}", Sha256::digest(input))
}

fn invalid(message: impl ToString) -> MemoryGovernanceError {
    MemoryGovernanceError::Invalid(message.to_string())
}

fn validate(plan: &AutomaticCorrectionPlan) -> Result<(), MemoryGovernanceError> {
    let request = plan
        .provenance
        .source_request
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| invalid("correction has no committed request"))?;
    if plan.namespace.is_empty()
        || plan.namespace.chars().count() > 512
        || plan.operation_id != correction_operation_id(&plan.namespace, request)
        || plan.evidence.id != correction_evidence_id(&plan.namespace, request)
        || plan.evidence.session_id != plan.namespace
        || plan.evidence.role != "user"
        || plan.evidence.content.trim().is_empty()
        || plan.evidence.content.chars().count() > 4_096
        || plan
            .provenance
            .source_session
            .as_deref()
            .is_none_or(str::is_empty)
        || plan
            .provenance
            .source_trace
            .as_deref()
            .is_none_or(str::is_empty)
        || plan.recorded_at_ms < 0
        || plan.valid_from_ms < 0
        || plan.valid_from_ms > plan.recorded_at_ms
        || plan.targets.is_empty()
        || plan.targets.len() > 16
    {
        return Err(invalid(
            "invalid correction identity, source, bounds or effective time",
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for target in &plan.targets {
        if !ids.insert(&target.id)
            || target.id == plan.evidence.id
            || target.expected_revision < 0
            || target.before_content == target.replacement_content
            || target.replacement_content.trim().is_empty()
            || target.replacement_content.chars().count() > 4_096
            || target.source_session.is_empty()
            || target.source_request.is_empty()
        {
            return Err(invalid("invalid correction target"));
        }
    }
    Ok(())
}

pub(crate) fn apply_on_conn(
    conn: &Connection,
    plan: &AutomaticCorrectionPlan,
) -> Result<AutomaticCorrectionOutcome, MemoryGovernanceError> {
    validate(plan)?;
    let hash = payload_hash(plan).map_err(invalid)?;
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    if operation_applied(&tx, &plan.operation_id, &hash).map_err(invalid)? {
        return Ok(AutomaticCorrectionOutcome { replayed: true });
    }
    let scope = MemoryScope::session(&plan.namespace);
    if get_governed_impl(&tx, &plan.evidence.id)?
        .is_some_and(|g| g.status == MemoryGovernanceStatus::Forgotten)
    {
        return Err(invalid("correction source was forgotten"));
    }
    // Validate the whole alias set before applying any mutation. A conflicting
    // member rolls back every content revision and the operation receipt.
    let mut current = Vec::with_capacity(plan.targets.len());
    for target in &plan.targets {
        let governed = get_governed_impl(&tx, &target.id)?
            .ok_or_else(|| MemoryGovernanceError::NotFound(target.id.clone()))?;
        if governed.status != MemoryGovernanceStatus::Active {
            return Err(MemoryGovernanceError::AlreadyForgotten(target.id.clone()));
        }
        if governed.protected {
            return Err(MemoryGovernanceError::Protected(target.id.clone()));
        }
        if governed.revision != target.expected_revision {
            return Err(MemoryGovernanceError::Conflict {
                id: target.id.clone(),
                expected: target.expected_revision,
                actual: governed.revision,
            });
        }
        let metadata = get_episode_metadata(&tx, &target.id)
            .map_err(invalid)?
            .ok_or_else(|| invalid("correction target has no scope provenance"))?;
        if governed.episode.session_id != plan.namespace
            || !matches!(governed.episode.role.as_str(), "user" | "memory")
            || governed.episode.content != target.before_content
            || metadata["scope"] != serde_json::to_value(&scope).map_err(invalid)?
            || metadata["provenance"]["source_session"].as_str()
                != Some(target.source_session.as_str())
            || metadata["provenance"]["source_request"].as_str()
                != Some(target.source_request.as_str())
        {
            return Err(invalid(
                "correction target identity or private scope changed",
            ));
        }
        current.push((governed, metadata));
    }
    let evidence_metadata = serde_json::json!({
        "scope":scope,"provenance":plan.provenance,"layer":"episodic",
        "content_hash":crate::canonical::vector::content_hash(&plan.evidence.content),
        "correction_operation":plan.operation_id,
    });
    put_chained_episode_in_transaction(
        &tx,
        &plan.namespace,
        &ChainEntry::from_episode(&plan.evidence, None),
        Some(&evidence_metadata),
    )
    .map_err(invalid)?;
    ensure_governance_on_conn(&tx, &plan.evidence.id)?;
    for (target, (governed, mut metadata)) in plan.targets.iter().zip(current) {
        let relation_key = correction_relation_key(&plan.namespace, &target.id);
        let prior: Option<(String, String, i64)> = tx.query_row(
            "SELECT id,object_id,valid_from_ms FROM temporal_graph_facts WHERE relation_key=?1 ORDER BY revision DESC LIMIT 1",
            [&relation_key], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        ).optional()?;
        let prior_start = governed
            .updated_at
            .unwrap_or(governed.episode.timestamp.saturating_mul(1_000));
        let (prior_id, prior_start) = match prior {
            Some((id, content, at)) if content == target.before_content => (id, at),
            other => {
                let id = stable_id(
                    "memory.correction.observed.v1",
                    &[&relation_key, &target.expected_revision.to_string()],
                );
                let fact = history_fact(
                    plan,
                    &relation_key,
                    &id,
                    &target.before_content,
                    (prior_start, prior_start),
                    Some(target.id.clone()),
                    other.map(|p| p.0),
                );
                append_fact(&tx, &fact)?;
                (id, prior_start)
            }
        };
        let effective = match plan.kind {
            AutomaticCorrectionKind::CorrectPrior => prior_start,
            AutomaticCorrectionKind::StateChange if plan.valid_from_ms >= prior_start => {
                plan.valid_from_ms
            }
            AutomaticCorrectionKind::StateChange => {
                return Err(invalid("state change predates the known prior state"))
            }
        };
        let id = stable_id(
            "memory.correction.revision.v1",
            &[&plan.operation_id, &target.id],
        );
        let fact = history_fact(
            plan,
            &relation_key,
            &id,
            &target.replacement_content,
            (effective, plan.recorded_at_ms),
            Some(plan.evidence.id.clone()),
            Some(prior_id.clone()),
        );
        append_fact(&tx, &fact)?;
        if plan.kind == AutomaticCorrectionKind::StateChange && effective > prior_start {
            let history = Episode {
                id: correction_history_id(&plan.namespace, &target.id, &plan.operation_id),
                timestamp: plan.recorded_at_ms / 1_000,
                role: "fact_history".into(),
                content: format!(
                    "历史状态（已结束，不代表当前），有效区间：[{}, {})：{}",
                    utc_time(prior_start)?,
                    utc_time(effective)?,
                    target.before_content
                ),
                session_id: plan.namespace.clone(),
            };
            let metadata = serde_json::json!({
                "scope":scope,"provenance":plan.provenance,"layer":"episodic",
                "content_hash":crate::canonical::vector::content_hash(&history.content),
                "derived_from_episode_id":target.id,"correction_evidence_id":plan.evidence.id,
                "correction_operation":plan.operation_id,"valid_from_ms":prior_start,"valid_until_ms":effective,
                "source_temporal_fact_id":prior_id,
            });
            put_chained_episode_in_transaction(
                &tx,
                &plan.namespace,
                &ChainEntry::from_episode(&history, None),
                Some(&metadata),
            )
            .map_err(invalid)?;
            ensure_governance_on_conn(&tx, &history.id)?;
        }
        crate::memory_governance::update_episode_content_for_operation_impl(
            &tx,
            &target.id,
            &target.replacement_content,
            Some("direct_user_correction"),
            target.expected_revision,
            Some(&format!("correction/{}", plan.operation_id)),
            &plan
                .targets
                .iter()
                .map(|target| target.id.clone())
                .collect::<Vec<_>>(),
        )?;
        if let Some(object) = metadata.as_object_mut() {
            object.remove("vector");
            object.insert(
                "content_hash".into(),
                serde_json::json!(crate::canonical::vector::content_hash(
                    &target.replacement_content
                )),
            );
            object.insert("correction".into(), serde_json::json!({"operation":plan.operation_id,"evidence":plan.evidence.id,"effective_from_ms":effective,"recorded_at_ms":plan.recorded_at_ms,"authorization_epoch":plan.authorization_epoch}));
        }
        put_episode_metadata(&tx, &target.id, &metadata).map_err(invalid)?;
    }
    record_operation_applied(&tx, &plan.operation_id, &hash, plan.recorded_at_ms)
        .map_err(invalid)?;
    tx.commit()?;
    Ok(AutomaticCorrectionOutcome { replayed: false })
}

fn utc_time(at_ms: i64) -> Result<String, MemoryGovernanceError> {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(at_ms)
        .map(|time| time.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .ok_or_else(|| invalid("correction history timestamp is out of range"))
}

fn history_fact(
    plan: &AutomaticCorrectionPlan,
    relation: &str,
    id: &str,
    content: &str,
    times: (i64, i64),
    source_episode_id: Option<String>,
    supersedes_id: Option<String>,
) -> TemporalGraphFact {
    TemporalGraphFact {
        id: id.into(),
        relation_key: relation.into(),
        subject_id: plan.namespace.clone(),
        predicate: "memory_assertion".into(),
        object_id: content.into(),
        valid_from_ms: times.0,
        valid_until_ms: None,
        believed_at_ms: times.1,
        source_episode_id,
        scope: serde_json::json!(MemoryScope::session(&plan.namespace)),
        provenance: serde_json::json!({"source":plan.provenance,"operation":plan.operation_id,"kind":plan.kind}),
        confidence: 0.9,
        revision: 0,
        supersedes_id,
        retracted_at_ms: None,
        created_at_ms: plan.recorded_at_ms,
    }
}

fn append_fact(
    tx: &Transaction<'_>,
    fact: &TemporalGraphFact,
) -> Result<(), MemoryGovernanceError> {
    crate::temporal_graph_store::append_in_transaction(
        tx,
        fact,
        &serde_json::to_string(&fact.scope).map_err(invalid)?,
        &serde_json::to_string(&fact.provenance).map_err(invalid)?,
    )
    .map(|_| ())
    .map_err(invalid)
}
