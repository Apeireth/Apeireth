//! Delivery evidence.
//!
//! A delivered memory record is only usable as a fact when four premises hold.
//! Each premise is decided on its own, from a different source of truth, so a
//! failure of one premise cannot be masked by another:
//!
//! 1. `direct_user_evidence` — the record traces back to a committed direct
//!    user request (storage-side writeback proof **and** the Runtime-side
//!    observation of that committed request).
//! 2. `host_authorization` — the Host policy that produced the delivery
//!    allows the recall capability. A model proposal never grants it.
//! 3. `current_scope` — the record lives in the scope of the querying
//!    partition and has not been withdrawn.
//! 4. `revision_cas` — the delivered snapshot still matches the current
//!    governed revision of the record (compare-and-swap semantics).

use std::path::Path;
use std::sync::{Arc, Mutex};

use apeireth_governance::PermissionPolicy;
use apeireth_memory::context_compiler::{
    memory_source_evidence_on_connection, project_memory_content, DELIVERY_SPAN_CHARS,
};
use apeireth_memory::{MemoryGovernanceStore, MemoryScope, SqliteMemoryStore};
use apeireth_orchestration::untrusted_envelope::escape_boundary_forgery;
use apeireth_protocol::canonical::{ContentPart, MessageRole, NormalizedRequest};
use apeireth_runtime::canonical::{
    BehaviorModule, CommittedEvidenceAccess, CommittedInputOrigin, CommittedUserInput, HookPoint,
    InvocationOrigin, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};
use async_trait::async_trait;
use rusqlite::OptionalExtension;
use serde::Deserialize;

use super::artifacts::{hash, Result};
use super::schema::{
    EvidenceReceipt, OracleIsolation, Premise, PremiseKind, PremiseSummary, SourceEvidence,
    EVIDENCE_PROTOCOL,
};

/// Capability that produces a memory delivery.
pub const RECALL_CAPABILITY: &str = "tool.behavior.cognitive.memory_recall";

/// Runtime-side observation of committed inputs. It never enters a model
/// context; it exists so that a delivered record can be tied back to the exact
/// request the user committed.
#[derive(Default)]
pub struct Audit {
    entries: Mutex<Vec<CommittedUserInput>>,
}

impl Audit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&self, input: CommittedUserInput) {
        let mut entries = self.entries.lock().unwrap();
        if !entries
            .iter()
            .any(|entry| entry.request == input.request && entry.session == input.session)
        {
            entries.push(input);
        }
    }

    /// The user text of the committed direct-user request, if the Runtime
    /// committed one for this session and request.
    pub fn committed_user_text(&self, session: &str, request: &str) -> Option<String> {
        let entries = self.entries.lock().unwrap();
        entries
            .iter()
            .filter(|entry| {
                entry.session.to_string() == session
                    && entry.request.to_string() == request
                    && entry.origin == CommittedInputOrigin::DirectUser
            })
            .find_map(|entry| {
                entry
                    .messages
                    .iter()
                    .find(|message| message.role == MessageRole::User)
                    .map(|message| ContentPart::join_text(&message.content))
            })
    }

    /// The number of committed user inputs the runtime observed.
    pub fn observed_requests(&self) -> usize {
        self.entries.lock().unwrap().len()
    }
}

/// Module that observes the committed input of every user turn.
pub struct Marker {
    pub manifest: ModuleManifest,
    pub audit: Arc<Audit>,
}

#[async_trait]
impl BehaviorModule for Marker {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn committed_evidence_access(&self) -> CommittedEvidenceAccess {
        CommittedEvidenceAccess::CurrentInput
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        context: &ModuleContext<'_>,
    ) -> std::result::Result<ModuleOutcome, ModuleError> {
        if (hook == HookPoint::TurnStart || hook == HookPoint::AfterTurn)
            && context.invocation.origin == InvocationOrigin::UserTurn
        {
            let input = context
                .invoker()
                .committed_input()
                .await
                .map_err(|error| ModuleError::Message(error.to_string()))?
                .ok_or_else(|| ModuleError::Message("committed input is unavailable".into()))?;
            self.audit.observe(input);
        }
        Ok(ModuleOutcome::continue_())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectedBody {
    pub source_role: String,
    pub claim_kind: String,
    pub citable_as_fact: bool,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Delivered {
    pub id: String,
    pub text: String,
    pub body: Option<ProjectedBody>,
    pub malformed: bool,
}

fn delivery(id: String, body: String) -> Delivered {
    let structured = body.starts_with('{');
    let parsed = structured
        .then(|| serde_json::from_str::<ProjectedBody>(&body).ok())
        .flatten();
    Delivered {
        id,
        text: parsed
            .as_ref()
            .map(|body| body.text.clone())
            .unwrap_or(body),
        malformed: structured && parsed.is_none(),
        body: parsed,
    }
}

/// Memory context envelopes carry the delivered records.
fn memory_context_texts(text: &str) -> Vec<String> {
    let Ok(envelope) = serde_json::from_str::<serde_json::Value>(text) else {
        return if text.contains("<governed_memory") {
            vec![text.into()]
        } else {
            vec![]
        };
    };
    if envelope["version"] != 1
        || envelope["origin"] != "module"
        || !matches!(
            envelope["module_id"].as_str(),
            Some("compat.cognitive.memory_recall" | "cognitive.memory_recall")
        )
        || envelope["hook"] != "turn_start"
        || envelope["payload"]["status"] != "ok"
        || envelope["payload"]["value"]["untrusted_data"] != true
    {
        return vec![];
    }
    envelope["payload"]["value"]["context"]
        .as_array()
        .map(|contexts| {
            contexts
                .iter()
                .filter(|context| context["source"] == "memory_recall")
                .filter_map(|context| context["text"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The delivered records of one request, with the identifier as rendered.
pub fn delivered_entries(request: &NormalizedRequest) -> Vec<Delivered> {
    let mut entries = Vec::new();
    for message in request
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::Tool)
    {
        for text in memory_context_texts(&ContentPart::join_text(&message.content)) {
            let mut current: Option<(String, String)> = None;
            for line in text.lines() {
                if let Some((header, content)) = line
                    .strip_prefix("[mem:")
                    .and_then(|line| line.split_once("] "))
                {
                    if let Some(entry) = current.take() {
                        entries.push(delivery(entry.0, entry.1));
                    }
                    if let Some(id) = header.split_whitespace().next() {
                        current = Some((id.into(), content.into()));
                    }
                } else if line == "</governed_memory>" {
                    if let Some(entry) = current.take() {
                        entries.push(delivery(entry.0, entry.1));
                    }
                } else if let Some((_, content)) = &mut current {
                    content.push('\n');
                    content.push_str(line);
                }
            }
            if let Some(entry) = current {
                entries.push(delivery(entry.0, entry.1));
            }
        }
    }
    entries
}

/// Everything the evidence pass needs from the outside.
pub struct Inspect<'a> {
    pub request: &'a NormalizedRequest,
    pub db: &'a Path,
    /// The querying partition (session id).
    pub partition: &'a str,
    /// The Host policy in force for this turn.
    pub policy: &'a PermissionPolicy,
    /// Runtime-side observation of committed inputs.
    pub audit: &'a Audit,
    /// Oracle canary: it must not appear in the request or in the store.
    pub canary: &'a str,
    /// Case label of the artifact.
    pub case: &'a str,
}

fn premise(
    kind: PremiseKind,
    satisfied: bool,
    proof: impl Into<String>,
    locators: Vec<String>,
) -> Premise {
    Premise {
        kind,
        satisfied,
        proof: proof.into(),
        locators,
    }
}

fn label_of<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Read the original content of an episode, ignoring any governance override.
fn original_content(connection: &rusqlite::Connection, episode_id: &str) -> Option<String> {
    connection
        .query_row(
            "SELECT content FROM episodes WHERE id=?1",
            [episode_id],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten()
}

/// The user-sourced episode a delivered record descends from.
fn source_episode_id(episode_id: &str, role: &str, metadata: &serde_json::Value) -> Option<String> {
    match role {
        "user" => Some(episode_id.to_string()),
        "memory" => metadata
            .get("derived_from_episode_id")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        _ => metadata
            .get("correction")
            .and_then(|value| value.get("evidence"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
    }
}

/// Decide the four premises of one delivered record.
fn inspect_source(
    input: &Inspect<'_>,
    delivered: &Delivered,
    store: &SqliteMemoryStore,
    connection: &rusqlite::Connection,
) -> SourceEvidence {
    let mut issues: Vec<String> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    let episode_id = delivered
        .id
        .strip_prefix("typed:episode:")
        .unwrap_or(&delivered.id)
        .to_string();

    let governed = match store.get_governed(&episode_id) {
        Ok(Some(row)) => row,
        Ok(None) => {
            issues.push(format!("delivered_id_missing_from_store:{episode_id}"));
            return missing_source(
                &episode_id,
                delivered,
                "delivered id is absent from the store",
            );
        }
        Err(error) => {
            issues.push(format!("source_read_failed:{episode_id}:{error}"));
            return missing_source(&episode_id, delivered, "the governed row could not be read");
        }
    };
    let metadata = match apeireth_plugin::memory_backend::MemoryBackend::get_episode_metadata(
        store,
        &episode_id,
    ) {
        Ok(Some(metadata)) => metadata,
        Ok(None) => {
            issues.push(format!("source_metadata_missing:{episode_id}"));
            serde_json::Value::Null
        }
        Err(error) => {
            issues.push(format!("source_metadata_read_failed:{episode_id}:{error}"));
            serde_json::Value::Null
        }
    };
    let original = governed.episode.content.clone();
    let effective = governed
        .content_override
        .clone()
        .unwrap_or_else(|| original.clone());
    let role = governed.episode.role.clone();

    // Commit-time source proof from the storage layer.
    let qualification = memory_source_evidence_on_connection(connection, &episode_id, &effective)
        .unwrap_or_default();
    let stored_role = label_of(qualification.source_role);
    let claim_kind = label_of(qualification.claim_kind);
    let delivered_body = delivered.body.clone();
    let delivered_role = delivered_body
        .as_ref()
        .map(|body| body.source_role.clone())
        .unwrap_or_default();
    let delivered_claim_kind = delivered_body
        .as_ref()
        .map(|body| body.claim_kind.clone())
        .unwrap_or_default();
    let delivered_citable = delivered_body
        .as_ref()
        .map(|body| body.citable_as_fact)
        .unwrap_or(false);
    if delivered.malformed {
        issues.push(format!("malformed_memory_source_record:{episode_id}"));
        failures.push("malformed_record".into());
    }
    if !delivered_role.is_empty() && delivered_role != stored_role {
        issues.push(format!("delivered_source_role_mismatch:{episode_id}"));
        failures.push("delivered_source_role_mismatch".into());
    }
    if !delivered_claim_kind.is_empty() && delivered_claim_kind != claim_kind {
        issues.push(format!("delivered_claim_kind_mismatch:{episode_id}"));
        failures.push("delivered_claim_kind_mismatch".into());
    }
    if delivered_citable != qualification.citable_as_fact {
        issues.push(format!("delivered_citability_mismatch:{episode_id}"));
        failures.push("delivered_citability_mismatch".into());
    }

    // Premise 1: direct user evidence.
    let direct = match qualification
        .source_group
        .as_deref()
        .and_then(|g| g.split_once('/'))
    {
        Some((session, request)) => {
            let parent = source_episode_id(&episode_id, &role, &metadata);
            let parent_content = parent
                .as_ref()
                .and_then(|id| original_content(connection, id));
            match parent_content {
                Some(text) => {
                    let committed = input.audit.committed_user_text(session, request);
                    if committed.as_deref() == Some(text.as_str()) {
                        premise(
                            PremiseKind::DirectUserEvidence,
                            true,
                            format!("committed direct-user request {session}/{request} carries the recorded text"),
                            vec![
                                format!("episode:{episode_id}"),
                                format!("session:{session}"),
                                format!("request:{request}"),
                            ],
                        )
                    } else {
                        premise(
                            PremiseKind::DirectUserEvidence,
                            false,
                            format!("committed direct-user request {session}/{request} does not carry the recorded text"),
                            vec![format!("episode:{episode_id}")],
                        )
                    }
                }
                None => premise(
                    PremiseKind::DirectUserEvidence,
                    false,
                    "the source episode of the record is not readable".to_string(),
                    vec![format!("episode:{episode_id}")],
                ),
            }
        }
        None => premise(
            PremiseKind::DirectUserEvidence,
            false,
            "the record has no committed direct-user source".to_string(),
            vec![format!("episode:{episode_id}")],
        ),
    };

    // Premise 2: Host authorization.
    let decision = input.policy.decision_for_capability(RECALL_CAPABILITY);
    let authorization = premise(
        PremiseKind::HostAuthorization,
        decision.is_allowed(),
        format!(
            "host policy decision for {RECALL_CAPABILITY}: {}",
            decision.reason().unwrap_or("allowed")
        ),
        vec![format!("capability:{RECALL_CAPABILITY}")],
    );

    // Premise 3: current scope.
    let expected_scope = serde_json::to_value(MemoryScope::session(input.partition)).unwrap();
    let scope_ok = metadata.get("scope") == Some(&expected_scope)
        && governed.episode.session_id == input.partition
        && metadata["provenance"]["source_session"].as_str() == Some(input.partition);
    let status = governed.status.as_str().to_string();
    let scope = premise(
        PremiseKind::CurrentScope,
        scope_ok && status == "active",
        format!(
            "record session={} scope={} governance={}",
            governed.episode.session_id,
            metadata
                .get("scope")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown"),
            status
        ),
        vec![
            format!("partition:{}", input.partition),
            format!("episode_session:{}", governed.episode.session_id),
            format!("status:{status}"),
        ],
    );

    // Premise 4: revision compare-and-swap.
    let expected_projection =
        escape_boundary_forgery(&project_memory_content(&effective, DELIVERY_SPAN_CHARS));
    let projection_ok = expected_projection == delivered.text;
    // The governed row is re-read from SQL on purpose: a revision that changed
    // after the delivery must make the revision premise false, and an unreadable
    // row must fail closed instead of silently passing.
    let (sql_governance, governance_readable) = match connection
        .query_row(
            "SELECT revision, content_override, status FROM episode_governance WHERE episode_id=?1",
            [&episode_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
    {
        Ok(value) => (value, true),
        Err(error) => {
            failures.push(format!("governance row unreadable: {error}"));
            (None, false)
        }
    };
    let (sql_revision, sql_override, sql_status) =
        sql_governance.unwrap_or((0, None, "active".to_string()));
    let governance_agrees = governance_readable
        && governed.revision == sql_revision
        && governed.content_override == sql_override
        && status == sql_status;
    let revision = premise(
        PremiseKind::RevisionCas,
        projection_ok && governance_agrees,
        format!(
            "delivered snapshot vs current governed revision (revision={}, projection_match={}, governance_agrees={})",
            governed.revision, projection_ok, governance_agrees
        ),
        vec![
            format!("episode:{episode_id}"),
            format!("revision:{}", governed.revision),
        ],
    );

    let premises = vec![direct, authorization, scope, revision];
    let mut all_failures = failures;
    for item in &premises {
        if !item.satisfied {
            all_failures.push(item.kind.label().to_string());
        }
    }
    let eligible = all_failures.is_empty()
        && qualification.citable_as_fact
        && delivered_citable
        && status == "active";

    SourceEvidence {
        id: delivered.id.clone(),
        episode_id,
        revision: governed.revision,
        content_sha256: hash(&effective),
        delivered_sha256: hash(&delivered.text),
        delivered_text: delivered.text.clone(),
        source_role: if stored_role.is_empty() {
            delivered_role
        } else {
            stored_role
        },
        claim_kind: if claim_kind.is_empty() {
            delivered_claim_kind
        } else {
            claim_kind
        },
        citable_as_fact: qualification.citable_as_fact && delivered_citable,
        active: status == "active",
        projection_verified: projection_ok,
        premises,
        eligible,
        failures: all_failures,
    }
}

fn missing_source(episode_id: &str, delivered: &Delivered, reason: &str) -> SourceEvidence {
    let premises = PremiseKind::ALL
        .iter()
        .map(|kind| premise(*kind, false, reason, vec![format!("episode:{episode_id}")]))
        .collect();
    SourceEvidence {
        id: delivered.id.clone(),
        episode_id: episode_id.to_string(),
        revision: -1,
        content_sha256: hash(""),
        delivered_sha256: hash(&delivered.text),
        delivered_text: delivered.text.clone(),
        source_role: "unknown".to_string(),
        claim_kind: "unknown".to_string(),
        citable_as_fact: false,
        active: false,
        projection_verified: false,
        premises,
        eligible: false,
        failures: vec![reason.to_string()],
    }
}

/// Inspect one request and decide the premises of every delivered record.
pub fn inspect(input: &Inspect<'_>) -> Result<EvidenceReceipt> {
    let store = SqliteMemoryStore::open(input.db)?;
    let connection = rusqlite::Connection::open_with_flags(
        input.db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let mut issues = Vec::new();
    let mut sources = Vec::new();
    for delivered in delivered_entries(input.request) {
        let source = inspect_source(input, &delivered, &store, &connection);
        for issue in &source.failures {
            if issue.starts_with("delivered_") || issue.starts_with("malformed_") {
                issues.push(format!("{}:{issue}", source.episode_id));
            }
        }
        sources.push(source);
    }
    let granted_capabilities = input
        .policy
        .iter()
        .map(|permission| permission.label())
        .collect::<Vec<_>>();
    let serialized = serde_json::to_string(input.request)?;
    let canary_in_request = serialized.contains(input.canary);
    let canary_in_store: i64 = connection.query_row(
        "SELECT COUNT(*) FROM episodes WHERE content LIKE '%' || ?1 || '%'",
        [input.canary],
        |row| row.get(0),
    )?;
    let premise_summary = summarize(&sources);
    Ok(EvidenceReceipt {
        protocol: EVIDENCE_PROTOCOL.to_string(),
        case: input.case.to_string(),
        partition: input.partition.to_string(),
        granted_capabilities,
        observed_committed_inputs: input.audit.observed_requests(),
        sources,
        premise_summary,
        oracle_isolation: OracleIsolation {
            canary: input.canary.to_string(),
            absent_from_requests: !canary_in_request,
            absent_from_store: canary_in_store == 0,
            absent_from_artifacts: true,
        },
        issues,
    })
}

/// Per-premise counters over the delivered records of one request.
pub fn summarize(sources: &[SourceEvidence]) -> Vec<PremiseSummary> {
    PremiseKind::ALL
        .iter()
        .map(|kind| {
            let relevant: Vec<&SourceEvidence> = sources
                .iter()
                .filter(|source| source.premises.iter().any(|premise| premise.kind == *kind))
                .collect();
            let satisfied = relevant
                .iter()
                .filter(|source| {
                    source
                        .premises
                        .iter()
                        .any(|premise| premise.kind == *kind && premise.satisfied)
                })
                .count();
            PremiseSummary {
                kind: *kind,
                label: kind.label().to_string(),
                source_count: relevant.len(),
                satisfied_count: satisfied,
                decidable: relevant.iter().all(|source| {
                    source
                        .premises
                        .iter()
                        .any(|premise| premise.kind == *kind && !premise.proof.is_empty())
                }),
            }
        })
        .collect()
}

/// Count stored records that carry the oracle canary.
pub fn canary_in_store(db: &Path, canary: &str) -> Result<bool> {
    let connection =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM episodes WHERE content LIKE '%' || ?1 || '%'",
        [canary],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
