//! Narrow orchestration boundary for turning bounded conversation input into memory episodes.
//!
//! This module deliberately does not wire itself into runtime assembly. Callers may use
//! [`MemoryCoordinator`] to persist the returned episodes, or adapt the typed report to
//! their own sinks through [`MemoryMaterializationSink`]. Extraction and reconciliation
//! remain delegated to their existing, independently testable services.

use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::memory::Episode;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    CommitmentCandidate, CommitmentSignal, ExtractedMemory, ExtractionClass, MemoryError,
    MemoryExtractionInput, MemoryExtractionMessage, MemoryExtractionResult, MemoryExtractor,
    MemoryProvenance, MemoryReconciler, MemoryReconciliationRecord, MemoryReconciliationReport,
    MemoryScope, RuleMemoryExtractor,
};

pub const DEFAULT_MAX_MESSAGES: usize = 64;
pub const DEFAULT_MAX_MESSAGE_CHARS: usize = 4_096;
pub const DEFAULT_MAX_EPISODES: usize = 64;

/// Input boundary that prevents untrusted conversations from growing without bound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedMemoryInput {
    pub scope: MemoryScope,
    pub source_session: Option<String>,
    pub source_trace: Option<String>,
    pub source_request: Option<String>,
    pub messages: Vec<MemoryExtractionMessage>,
    pub max_messages: usize,
    pub max_message_chars: usize,
}

impl BoundedMemoryInput {
    pub fn new(scope: MemoryScope, messages: Vec<MemoryExtractionMessage>) -> Self {
        Self {
            scope,
            source_session: None,
            source_trace: None,
            source_request: None,
            messages,
            max_messages: DEFAULT_MAX_MESSAGES,
            max_message_chars: DEFAULT_MAX_MESSAGE_CHARS,
        }
    }

    /// Validate and truncate at the boundary. Characters, rather than bytes, are counted.
    pub fn bounded(self) -> Result<MemoryExtractionInput, MemoryError> {
        if self.max_messages == 0 || self.max_message_chars == 0 {
            return Err(MemoryError::Invalid(
                "memory input bounds must be non-zero".into(),
            ));
        }
        let messages = self
            .messages
            .into_iter()
            .take(self.max_messages)
            .map(|mut message| {
                message.content = message
                    .content
                    .chars()
                    .take(self.max_message_chars)
                    .collect();
                message.role = message.role.chars().take(64).collect();
                message
            })
            .collect();
        Ok(MemoryExtractionInput {
            scope: self.scope,
            source_session: self.source_session,
            source_trace: self.source_trace,
            source_request: self.source_request,
            messages,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryMaterializationStatus {
    Materialized,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationCandidate {
    pub subject_id: String,
    pub predicate: String,
    pub object_id: String,
    pub content: String,
    pub confidence: f64,
    pub provenance: MemoryProvenance,
    pub scope: MemoryScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryTypedCandidate {
    Commitment {
        candidate: CommitmentCandidate,
        signal: CommitmentSignal,
    },
    Persona {
        delta: crate::PersonaProfileDelta,
    },
    Relation {
        candidate: RelationCandidate,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryMaterializationOutcome {
    Applied,
    Skipped { reason: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySinkReport {
    pub applied: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedMemoryEpisode {
    pub episode: Episode,
    pub candidate: ExtractedMemory,
    #[serde(default)]
    pub typed_candidate: Option<MemoryTypedCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMaterializationReport {
    pub status: MemoryMaterializationStatus,
    pub extraction: MemoryExtractionResult,
    pub reconciliation: MemoryReconciliationReport,
    pub episodes: Vec<MaterializedMemoryEpisode>,
    #[serde(default)]
    pub typed_candidates: Vec<MemoryTypedCandidate>,
    pub warning: Option<String>,
}

impl Default for MemoryMaterializationReport {
    fn default() -> Self {
        Self {
            status: MemoryMaterializationStatus::Skipped,
            extraction: MemoryExtractionResult::default(),
            reconciliation: MemoryReconciliationReport::default(),
            episodes: Vec::new(),
            typed_candidates: Vec::new(),
            warning: None,
        }
    }
}

/// Typed sink port. The complete projection is passed so sinks can persist both
/// the immutable episode and its extraction/provenance metadata atomically.
#[async_trait]
pub trait MemoryMaterializationSink: Send + Sync {
    async fn materialize_episode(
        &self,
        materialized: &MaterializedMemoryEpisode,
    ) -> Result<(), MemoryError>;
}

/// Storage-independent port for the typed projections produced by extraction.
/// Implementors can bridge these calls to any durable store; the default outcome
/// is deliberately `Skipped`, so an unconfigured sink never claims persistence.
#[async_trait]
pub trait MemoryTypedMaterializationSink: Send + Sync {
    async fn materialize_commitment(
        &self,
        candidate: &CommitmentCandidate,
        signal: &CommitmentSignal,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        let _ = (candidate, signal);
        Ok(MemoryMaterializationOutcome::Skipped {
            reason: "commitment sink is not configured".into(),
        })
    }

    async fn materialize_persona(
        &self,
        delta: &crate::PersonaProfileDelta,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        let _ = delta;
        Ok(MemoryMaterializationOutcome::Skipped {
            reason: "persona sink is not configured".into(),
        })
    }

    async fn materialize_relation(
        &self,
        candidate: &RelationCandidate,
    ) -> Result<MemoryMaterializationOutcome, MemoryError> {
        let _ = candidate;
        Ok(MemoryMaterializationOutcome::Skipped {
            reason: "relation sink is not configured".into(),
        })
    }
}

/// Object-safe runtime port for one bounded turn materialization.
#[async_trait]
pub trait MemoryMaterializerPort: Send + Sync {
    async fn materialize_episodes(
        &self,
        input: BoundedMemoryInput,
        timestamp_secs: i64,
    ) -> Result<Vec<MaterializedMemoryEpisode>, MemoryError>;

    /// Materialize one turn including typed projections when a sink is supplied.
    /// The default keeps compatibility for lightweight embedding implementations.
    async fn materialize_typed(
        &self,
        input: BoundedMemoryInput,
        timestamp_secs: i64,
        sink: &dyn MemoryTypedMaterializationSink,
    ) -> Result<MemorySinkReport, MemoryError> {
        let _ = (input, timestamp_secs, sink);
        Ok(MemorySinkReport::default())
    }
}

pub struct MemoryMaterializer<E = RuleMemoryExtractor> {
    extractor: E,
    reconciler: MemoryReconciler,
    max_episodes: usize,
}

impl Default for MemoryMaterializer<RuleMemoryExtractor> {
    fn default() -> Self {
        Self::new(RuleMemoryExtractor)
    }
}

#[async_trait]
impl<E> MemoryMaterializerPort for MemoryMaterializer<E>
where
    E: MemoryExtractor,
{
    async fn materialize_episodes(
        &self,
        input: BoundedMemoryInput,
        timestamp_secs: i64,
    ) -> Result<Vec<MaterializedMemoryEpisode>, MemoryError> {
        MemoryMaterializer::materialize_episodes(self, input, timestamp_secs).await
    }

    async fn materialize_typed(
        &self,
        input: BoundedMemoryInput,
        timestamp_secs: i64,
        sink: &dyn MemoryTypedMaterializationSink,
    ) -> Result<MemorySinkReport, MemoryError> {
        let existing: Vec<MemoryReconciliationRecord> = Vec::new();
        let (_report, sink_report) = self
            .materialize_to_typed_sink(input, &existing, timestamp_secs, sink)
            .await?;
        Ok(sink_report)
    }
}

impl<E> MemoryMaterializer<E>
where
    E: MemoryExtractor,
{
    pub fn new(extractor: E) -> Self {
        Self {
            extractor,
            reconciler: MemoryReconciler::new(),
            max_episodes: DEFAULT_MAX_EPISODES,
        }
    }

    #[must_use]
    pub fn with_max_episodes(mut self, max_episodes: usize) -> Self {
        self.max_episodes = max_episodes;
        self
    }

    pub async fn extract(
        &self,
        input: BoundedMemoryInput,
    ) -> Result<MemoryExtractionResult, MemoryError> {
        self.extractor.extract(input.bounded()?).await
    }

    /// Build generic episodic projections without selecting a concrete typed store.
    pub async fn materialize_episodes(
        &self,
        input: BoundedMemoryInput,
        timestamp_secs: i64,
    ) -> Result<Vec<MaterializedMemoryEpisode>, MemoryError> {
        let extraction = self.extract(input).await?;
        Ok(self.episodes_from_extraction(extraction, timestamp_secs))
    }

    pub async fn materialize(
        &self,
        input: BoundedMemoryInput,
        existing: &[MemoryReconciliationRecord],
        timestamp_secs: i64,
    ) -> Result<MemoryMaterializationReport, MemoryError> {
        let extraction = self.extract(input).await?;
        let candidates = candidates(&extraction);
        let reconciliation = self.reconciler.reconcile(existing, &candidates);
        let episodes = self.episodes_from_decisions(&reconciliation, timestamp_secs);
        let typed_candidates = typed_candidates(&extraction);

        Ok(MemoryMaterializationReport {
            status: MemoryMaterializationStatus::Materialized,
            extraction,
            reconciliation,
            episodes,
            typed_candidates,
            warning: None,
        })
    }

    /// Materialize and deliver each new/revised projection to the supplied sink.
    /// The sink receives the full typed projection, not only the lossy core episode.
    pub async fn materialize_to_sink<S: MemoryMaterializationSink>(
        &self,
        input: BoundedMemoryInput,
        existing: &[MemoryReconciliationRecord],
        timestamp_secs: i64,
        sink: &S,
    ) -> Result<MemoryMaterializationReport, MemoryError> {
        let report = self.materialize(input, existing, timestamp_secs).await?;
        for episode in &report.episodes {
            sink.materialize_episode(episode).await?;
        }
        Ok(report)
    }
    /// Deliver every typed candidate in the report to a storage-independent sink.
    /// A skipped outcome is retained in the returned counts and is not presented as
    /// automatic persistence.
    pub async fn materialize_typed_to_sink<S: MemoryTypedMaterializationSink + ?Sized>(
        &self,
        report: &MemoryMaterializationReport,
        sink: &S,
    ) -> Result<MemorySinkReport, MemoryError> {
        let mut outcome = MemorySinkReport::default();
        for candidate in &report.typed_candidates {
            let result = match candidate {
                MemoryTypedCandidate::Commitment { candidate, signal } => {
                    sink.materialize_commitment(candidate, signal).await?
                }
                MemoryTypedCandidate::Persona { delta } => sink.materialize_persona(delta).await?,
                MemoryTypedCandidate::Relation { candidate } => {
                    sink.materialize_relation(candidate).await?
                }
            };
            match result {
                MemoryMaterializationOutcome::Applied => outcome.applied += 1,
                MemoryMaterializationOutcome::Skipped { .. } => outcome.skipped += 1,
            }
        }
        Ok(outcome)
    }

    /// Materialize episodes and typed projections, then deliver both sinks.
    pub async fn materialize_to_typed_sink<S: MemoryTypedMaterializationSink + ?Sized>(
        &self,
        input: BoundedMemoryInput,
        existing: &[MemoryReconciliationRecord],
        timestamp_secs: i64,
        sink: &S,
    ) -> Result<(MemoryMaterializationReport, MemorySinkReport), MemoryError> {
        let report = self.materialize(input, existing, timestamp_secs).await?;
        let sink_report = self.materialize_typed_to_sink(&report, sink).await?;
        Ok((report, sink_report))
    }

    /// Fail-open convenience API: extraction failures become a skipped report.
    pub async fn materialize_fail_open(
        &self,
        input: BoundedMemoryInput,
        existing: &[MemoryReconciliationRecord],
        timestamp_secs: i64,
    ) -> MemoryMaterializationReport {
        match self.materialize(input, existing, timestamp_secs).await {
            Ok(report) => report,
            Err(error) => MemoryMaterializationReport {
                warning: Some(error.to_string()),
                ..Default::default()
            },
        }
    }

    fn episodes_from_extraction(
        &self,
        extraction: MemoryExtractionResult,
        timestamp_secs: i64,
    ) -> Vec<MaterializedMemoryEpisode> {
        let decisions = self.reconciler.reconcile(&[], &candidates(&extraction));
        self.episodes_from_decisions(&decisions, timestamp_secs)
    }

    fn episodes_from_decisions(
        &self,
        report: &MemoryReconciliationReport,
        timestamp_secs: i64,
    ) -> Vec<MaterializedMemoryEpisode> {
        report
            .decisions
            .iter()
            .filter(|d| {
                matches!(
                    d.outcome,
                    crate::MemoryReconciliationOutcome::New
                        | crate::MemoryReconciliationOutcome::Revised
                )
            })
            .take(self.max_episodes)
            .map(|decision| {
                let session_id = decision
                    .candidate
                    .provenance
                    .source_session
                    .clone()
                    .unwrap_or_else(|| "memory-materializer".into());
                let candidate = decision.candidate.clone();
                let typed_candidate = typed_candidate(&candidate);
                let id = stable_episode_id(timestamp_secs, &candidate);
                MaterializedMemoryEpisode {
                    episode: Episode {
                        id,
                        timestamp: timestamp_secs,
                        role: "memory".into(),
                        content: candidate.content.clone(),
                        session_id,
                    },
                    candidate,
                    typed_candidate,
                }
            })
            .collect()
    }
}

fn typed_candidate(candidate: &ExtractedMemory) -> Option<MemoryTypedCandidate> {
    if candidate.class == ExtractionClass::Relation {
        let mut parts = candidate.content.splitn(3, ' ');
        let (Some(subject_id), Some(predicate), Some(object_id)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return None;
        };
        return Some(MemoryTypedCandidate::Relation {
            candidate: RelationCandidate {
                subject_id: subject_id.to_string(),
                predicate: predicate.to_string(),
                object_id: object_id.to_string(),
                content: candidate.content.clone(),
                confidence: candidate.confidence,
                provenance: candidate.provenance.clone(),
                scope: candidate.scope.clone(),
            },
        });
    }
    if !matches!(candidate.class, ExtractionClass::Event) {
        return None;
    }
    let lower = candidate.content.to_lowercase();
    let signal = if ["submitted", "completed", "finished", "已提交", "完成"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        CommitmentSignal::Complete
    } else if ["no longer need", "cancel", "取消", "不再需要"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        CommitmentSignal::Cancel
    } else {
        CommitmentSignal::Create
    };
    Some(MemoryTypedCandidate::Commitment {
        candidate: CommitmentCandidate {
            content: candidate.content.clone(),
            confidence: candidate.confidence,
            deadline_hint: candidate
                .content
                .split_once("by ")
                .map(|(_, deadline)| deadline.trim().to_string()),
            provenance: candidate.provenance.clone(),
            scope: candidate.scope.clone(),
        },
        signal,
    })
}

fn typed_candidates(result: &MemoryExtractionResult) -> Vec<MemoryTypedCandidate> {
    let mut out: Vec<MemoryTypedCandidate> = candidates(result)
        .iter()
        .filter_map(typed_candidate)
        .collect();
    if let Some(delta) = &result.profile_delta {
        out.push(MemoryTypedCandidate::Persona {
            delta: delta.clone(),
        });
    }
    out
}

fn stable_episode_id(timestamp_secs: i64, candidate: &ExtractedMemory) -> String {
    let mut hasher = Sha256::new();
    hasher.update(
        format!(
            "{:?}|{}|{}",
            candidate.class, candidate.scope, candidate.content
        )
        .as_bytes(),
    );
    let digest = hasher.finalize();
    let mut suffix = String::new();
    for byte in &digest[..12] {
        use std::fmt::Write;
        let _ = write!(&mut suffix, "{byte:02x}");
    }
    format!("memory-materialized-{timestamp_secs}-{suffix}")
}

fn candidates(result: &MemoryExtractionResult) -> Vec<ExtractedMemory> {
    result
        .preferences
        .iter()
        .chain(&result.facts)
        .chain(&result.events)
        .chain(&result.experiences)
        .chain(&result.relations)
        .cloned()
        .collect()
}

#[allow(dead_code)]
fn _now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(content: &str) -> MemoryExtractionMessage {
        MemoryExtractionMessage {
            role: "user".into(),
            content: content.into(),
        }
    }

    #[tokio::test]
    async fn bounds_input_before_extraction() {
        let input = BoundedMemoryInput {
            max_messages: 1,
            max_message_chars: 12,
            ..BoundedMemoryInput::new(
                MemoryScope::Global,
                vec![msg("I prefer concise answers"), msg("I prefer later")],
            )
        };
        let extracted = MemoryMaterializer::default().extract(input).await.unwrap();
        assert_eq!(extracted.preferences.len(), 1);
        assert!(extracted.preferences[0].content.chars().count() <= 12);
    }

    #[tokio::test]
    async fn materializes_only_new_typed_candidates() {
        let report = MemoryMaterializer::default()
            .materialize(
                BoundedMemoryInput::new(MemoryScope::Global, vec![msg("I prefer concise answers")]),
                &[],
                10,
            )
            .await
            .unwrap();
        assert_eq!(report.status, MemoryMaterializationStatus::Materialized);
        assert_eq!(report.episodes.len(), 1);
        assert_eq!(report.episodes[0].episode.role, "memory");
    }

    #[tokio::test]
    async fn typed_sink_reports_skipped_without_claiming_persistence() {
        let report = MemoryMaterializationReport {
            typed_candidates: vec![MemoryTypedCandidate::Persona {
                delta: crate::PersonaProfileDelta::default(),
            }],
            ..Default::default()
        };
        let outcome = MemoryMaterializer::default()
            .materialize_typed_to_sink(&report, &MemoryMaterializationDefaultSink)
            .await
            .unwrap();
        assert_eq!(outcome.applied, 0);
        assert_eq!(outcome.skipped, 1);
    }

    struct MemoryMaterializationDefaultSink;

    #[async_trait]
    impl MemoryTypedMaterializationSink for MemoryMaterializationDefaultSink {}

    struct MemoryMaterializationSinkStub;

    #[async_trait]
    impl MemoryTypedMaterializationSink for MemoryMaterializationSinkStub {
        async fn materialize_persona(
            &self,
            _delta: &crate::PersonaProfileDelta,
        ) -> Result<MemoryMaterializationOutcome, MemoryError> {
            Ok(MemoryMaterializationOutcome::Applied)
        }
    }

    #[tokio::test]
    async fn typed_sink_consumes_commitment_persona_and_relation_candidates() {
        let report = MemoryMaterializationReport {
            typed_candidates: vec![
                MemoryTypedCandidate::Commitment {
                    candidate: CommitmentCandidate {
                        content: "finish task".into(),
                        confidence: 0.9,
                        deadline_hint: None,
                        provenance: MemoryProvenance::default(),
                        scope: MemoryScope::Global,
                    },
                    signal: CommitmentSignal::Create,
                },
                MemoryTypedCandidate::Persona {
                    delta: crate::PersonaProfileDelta::default(),
                },
                MemoryTypedCandidate::Relation {
                    candidate: RelationCandidate {
                        subject_id: "Ada".into(),
                        predicate: "works_with".into(),
                        object_id: "team".into(),
                        content: "works with team".into(),
                        confidence: 0.8,
                        provenance: MemoryProvenance::default(),
                        scope: MemoryScope::Global,
                    },
                },
            ],
            ..Default::default()
        };
        let outcome = MemoryMaterializer::default()
            .materialize_typed_to_sink(&report, &MemoryMaterializationSinkStub)
            .await
            .unwrap();
        assert_eq!(outcome.applied, 1);
        assert_eq!(outcome.skipped, 2);
    }
    #[tokio::test]
    async fn invalid_bounds_fail_open() {
        let input = BoundedMemoryInput {
            max_messages: 0,
            ..BoundedMemoryInput::new(MemoryScope::Global, vec![])
        };
        let report = MemoryMaterializer::default()
            .materialize_fail_open(input, &[], 10)
            .await;
        assert_eq!(report.status, MemoryMaterializationStatus::Skipped);
        assert!(report.warning.is_some());
    }
}
