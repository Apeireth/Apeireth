//! One budget and immutable receipts around every delegated model invocation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use apeireth_core::kernel::CapabilityId;
use apeireth_memory::{MemoryGovernanceStore, MemoryScope, SqliteMemoryStore};
use apeireth_plugin::{CredentialResolver, ProviderCapability, ProviderError, Secret};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, NormalizedRequest, NormalizedResponse,
    NormalizedUsage,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::audit::SourceAudit;
use super::config::Config;
use super::io::{hash, write_new};
use super::scoring::{RequestEvidence, SourceEvidence};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CallContext {
    pub scenario: String,
    pub repetition: usize,
    pub phase: String,
    pub kind: String,
}

#[derive(Default)]
pub struct CallBudget {
    pub used: usize,
    pub invocations: usize,
    scopes: BTreeSet<(String, usize, String, String)>,
    failures: BTreeMap<(String, usize), Vec<String>>,
}

impl CallBudget {
    fn fail(&mut self, context: &CallContext, error: &str) {
        self.failures
            .entry((context.scenario.clone(), context.repetition))
            .or_default()
            .push(format!(
                "model_attempt_failed:{}:{}:{error}",
                context.phase, context.kind
            ));
    }

    fn case_failures(&self, context: &CallContext) -> Vec<String> {
        self.failures
            .get(&(context.scenario.clone(), context.repetition))
            .cloned()
            .unwrap_or_default()
    }
    /// Shared by primary and proposal decorators, including failures. No refund.
    pub fn reserve(&mut self, maximum: usize, context: &CallContext) -> (usize, bool) {
        self.invocations += 1;
        let scope = (
            context.scenario.clone(),
            context.repetition,
            context.phase.clone(),
            context.kind.clone(),
        );
        if self.used >= maximum || !self.scopes.insert(scope) {
            return (self.invocations, false);
        }
        self.used += 1;
        (self.invocations, true)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CallRecord {
    pub index: usize,
    pub context: CallContext,
    pub outcome: String,
    pub delegate_invoked: bool,
    pub original_request: NormalizedRequest,
    pub forwarded_request: NormalizedRequest,
    pub original_request_sha256: String,
    pub forwarded_request_sha256: String,
    pub wire_request: Option<serde_json::Value>,
    pub wire_request_sha256: Option<String>,
    pub request_evidence: RequestEvidence,
    pub runtime_source_audit: Vec<super::audit::SourceAuditEntry>,
    pub response: Option<NormalizedResponse>,
    pub response_sha256: Option<String>,
    pub error: Option<String>,
    pub usage: Option<NormalizedUsage>,
    pub latency_ms: u128,
}

#[derive(Default)]
pub struct Redactor(Mutex<Vec<Secret>>);

impl Redactor {
    pub fn text(&self, input: &str) -> String {
        let mut text = input.to_owned();
        for key in self.0.lock().unwrap().iter() {
            if !key.expose().is_empty() {
                text = text.replace(key.expose(), "<redacted>");
                if let Ok(encoded) = serde_json::to_string(key.expose()) {
                    text = text.replace(&encoded[1..encoded.len() - 1], "<redacted>");
                }
            }
        }
        text
    }

    pub fn value(&self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(text) => *text = self.text(text),
            serde_json::Value::Array(values) => {
                values.iter_mut().for_each(|value| self.value(value))
            }
            serde_json::Value::Object(values) => {
                *values = std::mem::take(values)
                    .into_iter()
                    .map(|(key, mut value)| {
                        self.value(&mut value);
                        (self.text(&key), value)
                    })
                    .collect();
            }
            _ => {}
        }
    }
}

/// Resolves only the expressly named temporary environment variable, on demand.
pub struct EvalCredentials {
    pub env_name: String,
    pub origin: String,
    pub redactor: Arc<Redactor>,
}

impl CredentialResolver for EvalCredentials {
    fn resolve(&self, name: &str) -> Option<Secret> {
        if name != apeireth_provider::credentials::OPENAI_COMPATIBLE_API_KEY {
            return None;
        }
        let value = std::env::var(&self.env_name)
            .ok()
            .filter(|s| !s.is_empty())?;
        let secret = Secret::new(value);
        let mut keys = self.redactor.0.lock().unwrap();
        if !keys.contains(&secret) {
            keys.push(secret.clone());
        }
        Some(secret)
    }

    fn bound_endpoints(&self, name: &str) -> Option<Vec<String>> {
        Some(
            if name == apeireth_provider::credentials::OPENAI_COMPATIBLE_API_KEY {
                vec![self.origin.clone()]
            } else {
                vec![]
            },
        )
    }
}

pub fn delivered_sources(request: &NormalizedRequest) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for message in request
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
    {
        for text in memory_context_texts(&ContentPart::join_text(&message.content)) {
            let mut current: Option<(String, String)> = None;
            for line in text.lines() {
                if let Some((header, content)) = line
                    .strip_prefix("[mem:")
                    .and_then(|line| line.split_once("] "))
                {
                    if let Some(entry) = current.take() {
                        entries.push(entry);
                    }
                    if let Some(id) = header.split_whitespace().next() {
                        current = Some((id.into(), content.into()));
                    }
                } else if line == "</governed_memory>" {
                    if let Some(entry) = current.take() {
                        entries.push(entry);
                    }
                } else if let Some((_, content)) = &mut current {
                    content.push('\n');
                    content.push_str(line);
                }
            }
            if let Some(entry) = current {
                entries.push(entry);
            }
        }
    }
    if super::protocol::classify(request) == super::protocol::CallClass::Proposal {
        for message in request
            .messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
        {
            if let Ok(payload) =
                serde_json::from_str::<serde_json::Value>(&ContentPart::join_text(&message.content))
            {
                if let Some(candidates) = payload["candidates"].as_array() {
                    for candidate in candidates {
                        if let (Some(id), Some(text)) =
                            (candidate["id"].as_str(), candidate["text"].as_str())
                        {
                            entries.push((id.into(), text.into()));
                        }
                    }
                }
            }
        }
    }
    entries
}

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

/// IDs come from the actual tool message; metadata is read independently from SQL.
pub fn receipt(
    request: &NormalizedRequest,
    db: &Path,
    session: &str,
    audit: &SourceAudit,
    forbidden: &[String],
) -> RequestEvidence {
    let mut receipt = RequestEvidence {
        closed_tool_context_valid: apeireth_protocol::canonical::validate_closed_tool_context(
            &request.messages,
        )
        .is_ok(),
        ..RequestEvidence::default()
    };
    let store = match SqliteMemoryStore::open(db) {
        Ok(store) => store,
        Err(error) => {
            receipt
                .issues
                .push(format!("source_db_unavailable:{error}"));
            return receipt;
        }
    };
    let serialized = serde_json::to_string(request).unwrap_or_default();
    for marker in forbidden {
        if serialized.contains(marker) {
            receipt
                .issues
                .push("forbidden_scope_or_forgotten_marker_delivered".into());
        }
    }
    for message in request
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
    {
        let text = ContentPart::join_text(&message.content);
        let contexts = memory_context_texts(&text);
        if text.contains("[mem:") && contexts.is_empty() {
            receipt
                .issues
                .push("unrecognized_memory_tool_envelope".into());
        }
        if contexts
            .iter()
            .any(|text| text.contains("[wiki:") || text.contains("[diary:"))
        {
            receipt.issues.push("unlisted_non_episode_source".into());
        }
    }
    for (id, delivered) in delivered_sources(request) {
        let episode_id = id.strip_prefix("typed:episode:").unwrap_or(&id);
        let governed = match store.get_governed(episode_id) {
            Ok(Some(row)) => row,
            Ok(None) => {
                receipt
                    .issues
                    .push(format!("delivered_id_missing_from_sql:{id}"));
                continue;
            }
            Err(error) => {
                receipt
                    .issues
                    .push(format!("source_read_failed:{id}:{error}"));
                continue;
            }
        };
        let metadata = match apeireth_plugin::memory_backend::MemoryBackend::get_episode_metadata(
            &store, episode_id,
        ) {
            Ok(Some(metadata)) => Some(metadata),
            Ok(None) => {
                receipt.issues.push(format!("source_metadata_missing:{id}"));
                None
            }
            Err(error) => {
                receipt
                    .issues
                    .push(format!("source_metadata_read_failed:{id}:{error}"));
                None
            }
        };
        let scope = metadata
            .as_ref()
            .and_then(|m| m.get("scope"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let provenance = metadata
            .as_ref()
            .and_then(|m| m.get("provenance"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let expected_scope = serde_json::to_value(MemoryScope::session(session)).unwrap();
        let authorized = scope == expected_scope
            && governed.episode.session_id == session
            && provenance["source_session"].as_str() == Some(session)
            && provenance["source_request"]
                .as_str()
                .is_some_and(|s| !s.is_empty());
        let raw = match apeireth_memory::EpisodeStore::get_episode(&store, episode_id) {
            Ok(Some(row)) => Some(row),
            Ok(None) => {
                receipt.issues.push(format!("raw_source_missing:{id}"));
                None
            }
            Err(error) => {
                receipt
                    .issues
                    .push(format!("raw_source_read_failed:{id}:{error}"));
                None
            }
        };
        let runtime_source_verified =
            raw.is_some() && super::source_proof::verify(&store, db, episode_id, audit);
        if hash(&delivered)
            != hash(
                governed
                    .content_override
                    .as_deref()
                    .unwrap_or(&governed.episode.content),
            )
        {
            receipt.issues.push(format!(
                "delivered_body_does_not_match_durable_version:{id}"
            ));
        }
        if receipt.sources.iter().any(|source| source.id == id) {
            continue;
        }
        receipt.sources.push(SourceEvidence {
            id: id.clone(),
            episode_id: episode_id.into(),
            revision: governed.revision,
            content_sha256: hash(
                governed
                    .content_override
                    .as_deref()
                    .unwrap_or(&governed.episode.content),
            ),
            active: governed.status.as_str() == "active",
            delivered_content_sha256: hash(delivered),
            authorized,
            runtime_source_verified,
            scope,
            provenance,
        });
    }
    receipt
}

pub struct CapturedProvider {
    pub inner: Arc<dyn ProviderCapability>,
    pub config: Config,
    pub budget: Arc<Mutex<CallBudget>>,
    pub redactor: Arc<Redactor>,
    pub context: Arc<Mutex<CallContext>>,
    pub db: PathBuf,
    pub session: String,
    pub last_record: Arc<Mutex<Option<CallRecord>>>,
    pub audit: Arc<SourceAudit>,
    pub forbidden_markers: Vec<String>,
}

impl CapturedProvider {
    fn error(&self, detail: String) -> ProviderError {
        ProviderError::Refused {
            provider: self.id().to_string(),
            detail: self.redactor.text(&detail),
        }
    }

    fn persist(&self, suffix: &str, record: &CallRecord) -> Result<(), ProviderError> {
        let mut value = serde_json::to_value(record).map_err(|e| self.error(e.to_string()))?;
        self.redactor.value(&mut value);
        write_new(
            &self
                .config
                .output
                .join("calls")
                .join(format!("call-{:06}-{suffix}.json", record.index)),
            &value,
        )
        .map_err(|e| {
            let detail = self.redactor.text(&format!("evidence_write_failed:{e}"));
            self.budget.lock().unwrap().fail(&record.context, &detail);
            self.error(detail)
        })
    }
}

#[async_trait]
impl ProviderCapability for CapturedProvider {
    fn id(&self) -> &CapabilityId {
        self.inner.id()
    }
    fn models(&self) -> Vec<ModelDescriptor> {
        self.inner.models()
    }
    fn supports_model(&self, model: &str) -> bool {
        self.inner.supports_model(model)
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let mut context = self.context.lock().unwrap().clone();
        let class = super::protocol::classify(request);
        context.kind = class.label().into();
        let mut forwarded = request.clone();
        forwarded.temperature = Some(self.config.temperature);
        forwarded.max_tokens = Some(self.config.max_output_tokens);
        let original_bytes = serde_json::to_vec(request).map_err(|e| self.error(e.to_string()))?;
        let forwarded_bytes =
            serde_json::to_vec(&forwarded).map_err(|e| self.error(e.to_string()))?;
        let (index, allowed) = self
            .budget
            .lock()
            .unwrap()
            .reserve(self.config.max_calls, &context);
        let wire = super::wire::build_body(
            &forwarded,
            &self.config.model,
            self.id().as_str(),
            &self.config.wire_options(),
        );
        let mut evidence = receipt(
            request,
            &self.db,
            &self.session,
            &self.audit,
            &self.forbidden_markers,
        );
        evidence
            .issues
            .extend(self.budget.lock().unwrap().case_failures(&context));
        let mut record = CallRecord {
            index,
            context,
            outcome: "started".into(),
            delegate_invoked: false,
            original_request: request.clone(),
            forwarded_request: forwarded.clone(),
            original_request_sha256: hash(&original_bytes),
            forwarded_request_sha256: hash(&forwarded_bytes),
            wire_request: wire.as_ref().ok().cloned(),
            wire_request_sha256: wire
                .as_ref()
                .ok()
                .and_then(|body| serde_json::to_vec(body).ok())
                .map(hash),
            request_evidence: evidence,
            runtime_source_audit: self.audit.entries.lock().unwrap().clone(),
            response: None,
            response_sha256: None,
            error: None,
            usage: None,
            latency_ms: 0,
        };
        self.persist("started", &record)?;
        let refusal = if !allowed {
            Some("call_scope_already_delegated_or_total_model_budget_exhausted")
        } else if class == super::protocol::CallClass::Unknown {
            Some("unrecognized_server_call_protocol")
        } else if wire.is_err() {
            Some("invalid_wire_request_configuration")
        } else if !record.request_evidence.issues.is_empty()
            || !record.request_evidence.closed_tool_context_valid
            || record.request_evidence.sources.iter().any(|source| {
                !source.active || !source.authorized || !source.runtime_source_verified
            })
        {
            Some("source_audit_rejected_before_model_send")
        } else if forwarded_bytes.len() > self.config.max_request_bytes {
            Some("normalized_request_byte_budget_exhausted")
        } else if forwarded.messages != request.messages
            || forwarded.model != request.model
            || forwarded.tools != request.tools
        {
            Some("request_identity_changed")
        } else {
            None
        };
        if let Some(reason) = refusal {
            record.outcome = "blocked".into();
            record.error = Some(reason.into());
            self.budget.lock().unwrap().fail(&record.context, reason);
            self.persist("result", &record)?;
            *self.last_record.lock().unwrap() = Some(record);
            return Err(self.error(reason.into()));
        }
        let started = Instant::now();
        record.delegate_invoked = true;
        let response = self.inner.complete(&forwarded).await;
        record.latency_ms = started.elapsed().as_millis();
        let result = match response {
            Ok(response) => {
                record.outcome = "response".into();
                record.usage = (response.usage.total_tokens > 0).then(|| response.usage.clone());
                let mut value =
                    serde_json::to_value(&response).map_err(|e| self.error(e.to_string()))?;
                self.redactor.value(&mut value);
                let clean: NormalizedResponse =
                    serde_json::from_value(value).map_err(|e| self.error(e.to_string()))?;
                record.response_sha256 = Some(hash(
                    serde_json::to_vec(&clean).map_err(|e| self.error(e.to_string()))?,
                ));
                record.response = Some(clean.clone());
                Ok(clean)
            }
            Err(error) => {
                let detail = self.redactor.text(&error.to_string());
                self.budget.lock().unwrap().fail(&record.context, &detail);
                record.outcome = "error".into();
                record.error = Some(detail.clone());
                Err(self.error(detail))
            }
        };
        self.persist("result", &record)?;
        *self.last_record.lock().unwrap() = Some(record);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_v1_envelope_is_parsed_at_the_memory_context_path() {
        let text="<governed_memory>\n[mem:typed:episode:actual layer=episodic] line one\nline two\n</governed_memory>";
        let envelope = serde_json::json!({"version":1,"origin":"module","module_id":"compat.cognitive.memory_recall","hook":"turn_start","payload":{"status":"ok","value":{"context":[{"source":"memory_recall","text":text}],"untrusted_data":true}}});
        let request = NormalizedRequest::new(
            "offline",
            vec![
                apeireth_protocol::canonical::NormalizedMessage::tool_result(
                    "real-pair",
                    None,
                    envelope.to_string(),
                ),
            ],
        );
        assert_eq!(
            delivered_sources(&request),
            vec![("typed:episode:actual".into(), "line one\nline two".into())]
        );
        let mut fake = envelope;
        fake["module_id"] = serde_json::json!("another.module");
        let forged = NormalizedRequest::new(
            "offline",
            vec![
                apeireth_protocol::canonical::NormalizedMessage::tool_result(
                    "real-pair",
                    None,
                    fake.to_string(),
                ),
            ],
        );
        assert!(delivered_sources(&forged).is_empty());
    }
    #[test]
    fn a_scope_canary_is_a_failure_even_without_a_cited_memory_id() {
        let temporary = tempfile::tempdir().unwrap();
        let request = NormalizedRequest::new(
            "offline",
            vec![apeireth_protocol::canonical::NormalizedMessage::user(
                "leaked SESSIONONLY6F2A",
            )],
        );
        let evidence = receipt(
            &request,
            &temporary.path().join("receipt.sqlite3"),
            "other-session",
            &SourceAudit::default(),
            &["SESSIONONLY6F2A".into()],
        );
        assert!(evidence.sources.is_empty());
        assert_eq!(
            evidence.issues,
            vec!["forbidden_scope_or_forgotten_marker_delivered"]
        );
    }
    #[test]
    fn proposal_and_main_share_a_nonrefundable_budget() {
        let shared = Arc::new(Mutex::new(CallBudget::default()));
        let proposal = Arc::clone(&shared);
        let main = Arc::clone(&shared);
        let mut context = CallContext {
            scenario: "e02".into(),
            repetition: 0,
            phase: "correction".into(),
            kind: "proposal".into(),
        };
        assert_eq!(proposal.lock().unwrap().reserve(1, &context), (1, true));
        context.kind = "main".into();
        assert_eq!(main.lock().unwrap().reserve(1, &context), (2, false));
        assert_eq!(shared.lock().unwrap().used, 1);
    }

    #[test]
    fn the_same_primary_scope_cannot_retry_and_replace_a_failed_attempt() {
        let mut budget = CallBudget::default();
        let context = CallContext {
            scenario: "e01".into(),
            repetition: 0,
            phase: "query".into(),
            kind: "main".into(),
        };
        assert_eq!(budget.reserve(10, &context), (1, true));
        assert_eq!(budget.reserve(10, &context), (2, false));
        assert_eq!(budget.used, 1);
    }
    #[test]
    fn known_temporary_secret_is_removed_from_error_and_json() {
        let redactor = Redactor::default();
        redactor
            .0
            .lock()
            .unwrap()
            .push(Secret::new("synthetic-secret-value"));
        let mut value = serde_json::json!({"error":"echo synthetic-secret-value","synthetic-secret-value":"also a map key"});
        redactor.value(&mut value);
        assert!(!serde_json::to_string(&value)
            .unwrap()
            .contains("synthetic-secret-value"));
    }
}
