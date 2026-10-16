//! Pure, deterministic reconciliation for extracted memory candidates.
//!
//! This module intentionally has no store or SQL dependency. A lifecycle can
//! apply the returned outcomes to whichever governed stores are available.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ExtractedMemory, ExtractionClass, MemoryScope};

/// A durable projection needed by reconciliation; stores can adapt their rows
/// to this shape without exposing their backend to the reconciliation logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryReconciliationRecord {
    pub id: String,
    pub class: ExtractionClass,
    pub content: String,
    pub scope: MemoryScope,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub reinforcement: u64,
}

/// Deterministic action a lifecycle should apply for one candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryReconciliationOutcome {
    /// No matching durable record exists; insert the candidate.
    New,
    /// Same typed assertion already exists; do not create another row.
    Duplicate,
    /// Existing assertion was observed again; increment reinforcement.
    Reinforced,
    /// Same typed assertion key changed; update content with optimistic revision.
    Revised,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryReconciliationDecision {
    pub candidate: ExtractedMemory,
    pub outcome: MemoryReconciliationOutcome,
    pub existing_id: Option<String>,
    pub next_revision: Option<u64>,
    pub next_reinforcement: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MemoryReconciliationReport {
    pub decisions: Vec<MemoryReconciliationDecision>,
}

impl MemoryReconciliationReport {
    #[must_use]
    pub fn count(&self, outcome: MemoryReconciliationOutcome) -> usize {
        self.decisions
            .iter()
            .filter(|d| d.outcome == outcome)
            .count()
    }
}

/// Stateless reconciliation service. It is safe to use from a lifecycle or
/// transaction coordinator and makes no persistence assumptions.
#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryReconciler;

impl MemoryReconciler {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Reconcile candidates against a snapshot of durable records.
    ///
    /// Candidates are sorted by type, scope, and normalized content, so the
    /// report is identical regardless of input iteration order. Exact typed
    /// matches reinforce; preference/fact/relation assertions with the same
    /// stable key produce revisions. Events and experiences are append-only
    /// unless they are exact duplicates.
    #[must_use]
    pub fn reconcile(
        &self,
        existing: &[MemoryReconciliationRecord],
        candidates: &[ExtractedMemory],
    ) -> MemoryReconciliationReport {
        let mut ordered = candidates.to_vec();
        ordered.sort_by_key(candidate_sort_key);
        let mut working: BTreeMap<String, MemoryReconciliationRecord> = existing
            .iter()
            .cloned()
            .map(|record| (record.id.clone(), record))
            .collect();
        let mut decisions = Vec::with_capacity(ordered.len());

        for candidate in ordered {
            let exact_match = working
                .values()
                .find(|record| exact_key(record) == exact_key_candidate(&candidate))
                .cloned();
            if let Some(record) = exact_match {
                let next = record.reinforcement.saturating_add(1);
                let id = record.id.clone();
                if let Some(updated) = working.get_mut(&id) {
                    updated.reinforcement = next;
                }
                decisions.push(MemoryReconciliationDecision {
                    candidate,
                    outcome: if record.reinforcement == 0 {
                        MemoryReconciliationOutcome::Duplicate
                    } else {
                        MemoryReconciliationOutcome::Reinforced
                    },
                    existing_id: Some(id),
                    next_revision: None,
                    next_reinforcement: Some(next),
                });
                continue;
            }
            let revisionable = matches!(
                candidate.class,
                ExtractionClass::Preference | ExtractionClass::Fact | ExtractionClass::Relation
            );
            let revised = if revisionable {
                working
                    .values()
                    .find(|record| revision_key(record) == revision_key_candidate(&candidate))
                    .cloned()
            } else {
                None
            };
            if let Some(record) = revised {
                let id = record.id.clone();
                let next_revision = record.revision.saturating_add(1);
                let replacement = MemoryReconciliationRecord {
                    id: id.clone(),
                    class: candidate.class.clone(),
                    content: candidate.content.clone(),
                    scope: candidate.scope.clone(),
                    confidence: candidate.confidence,
                    revision: next_revision,
                    reinforcement: 0,
                };
                working.insert(id.clone(), replacement);
                decisions.push(MemoryReconciliationDecision {
                    candidate,
                    outcome: MemoryReconciliationOutcome::Revised,
                    existing_id: Some(id),
                    next_revision: Some(next_revision),
                    next_reinforcement: None,
                });
                continue;
            }
            let id = format!("candidate-{}", decisions.len());
            working.insert(
                id.clone(),
                MemoryReconciliationRecord {
                    id: id.clone(),
                    class: candidate.class.clone(),
                    content: candidate.content.clone(),
                    scope: candidate.scope.clone(),
                    confidence: candidate.confidence,
                    revision: 0,
                    reinforcement: 0,
                },
            );
            decisions.push(MemoryReconciliationDecision {
                candidate,
                outcome: MemoryReconciliationOutcome::New,
                existing_id: None,
                next_revision: Some(0),
                next_reinforcement: Some(0),
            });
        }
        MemoryReconciliationReport { decisions }
    }
}

fn normalized(content: &str) -> String {
    content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn class_name(class: &ExtractionClass) -> &'static str {
    match class {
        ExtractionClass::Preference => "preference",
        ExtractionClass::Fact => "fact",
        ExtractionClass::Event => "event",
        ExtractionClass::Experience => "experience",
        ExtractionClass::Relation => "relation",
        ExtractionClass::PersonaDelta => "persona_delta",
    }
}
fn scope_key(scope: &MemoryScope) -> String {
    scope.to_string()
}
fn exact_key(record: &MemoryReconciliationRecord) -> String {
    format!(
        "{}|{}|{}",
        class_name(&record.class),
        scope_key(&record.scope),
        normalized(&record.content)
    )
}
fn exact_key_candidate(candidate: &ExtractedMemory) -> String {
    format!(
        "{}|{}|{}",
        class_name(&candidate.class),
        scope_key(&candidate.scope),
        normalized(&candidate.content)
    )
}
fn assertion_key(content: &str) -> String {
    let text = normalized(content);
    let key = text
        .split_once('=')
        .or_else(|| text.split_once(':'))
        .map(|(key, _)| key);
    key.map_or(text.clone(), |key| key.trim().to_string())
}
fn revision_key(record: &MemoryReconciliationRecord) -> String {
    format!(
        "{}|{}|{}",
        class_name(&record.class),
        scope_key(&record.scope),
        assertion_key(&record.content)
    )
}
fn revision_key_candidate(candidate: &ExtractedMemory) -> String {
    format!(
        "{}|{}|{}",
        class_name(&candidate.class),
        scope_key(&candidate.scope),
        assertion_key(&candidate.content)
    )
}
fn candidate_sort_key(candidate: &ExtractedMemory) -> (u8, String, String, String) {
    let rank = match candidate.class {
        ExtractionClass::Preference => 0,
        ExtractionClass::Fact => 1,
        ExtractionClass::Relation => 2,
        ExtractionClass::Event => 3,
        ExtractionClass::Experience => 4,
        ExtractionClass::PersonaDelta => 5,
    };
    (
        rank,
        scope_key(&candidate.scope),
        normalized(&candidate.content),
        candidate.provenance.source.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryProvenance;

    fn c(class: ExtractionClass, content: &str) -> ExtractedMemory {
        ExtractedMemory {
            class,
            content: content.into(),
            confidence: 0.8,
            scope: MemoryScope::Global,
            provenance: MemoryProvenance::default(),
            source_trace: None,
        }
    }
    fn r(class: ExtractionClass, content: &str, reinforcement: u64) -> MemoryReconciliationRecord {
        MemoryReconciliationRecord {
            id: "r1".into(),
            class,
            content: content.into(),
            scope: MemoryScope::Global,
            confidence: 0.8,
            revision: 2,
            reinforcement,
        }
    }

    #[test]
    fn exact_typed_match_reinforces_and_is_case_whitespace_insensitive() {
        let report = MemoryReconciler::new().reconcile(
            &[r(ExtractionClass::Fact, "Color = Blue", 2)],
            &[c(ExtractionClass::Fact, " color   = blue ")],
        );
        assert_eq!(
            report.decisions[0].outcome,
            MemoryReconciliationOutcome::Reinforced
        );
        assert_eq!(report.decisions[0].next_reinforcement, Some(3));
    }
    #[test]
    fn same_key_revision_is_type_and_scope_aware() {
        let report = MemoryReconciler::new().reconcile(
            &[r(ExtractionClass::Preference, "theme=dark", 0)],
            &[c(ExtractionClass::Preference, "theme=light")],
        );
        assert_eq!(
            report.decisions[0].outcome,
            MemoryReconciliationOutcome::Revised
        );
        assert_eq!(report.decisions[0].next_revision, Some(3));
    }
    #[test]
    fn events_do_not_revision_merge() {
        let report = MemoryReconciler::new().reconcile(
            &[r(ExtractionClass::Event, "deadline=friday", 0)],
            &[c(ExtractionClass::Event, "deadline=monday")],
        );
        assert_eq!(
            report.decisions[0].outcome,
            MemoryReconciliationOutcome::New
        );
    }
    #[test]
    fn output_order_is_deterministic() {
        let a = [
            c(ExtractionClass::Fact, "z=1"),
            c(ExtractionClass::Preference, "a=2"),
        ];
        let b = [a[1].clone(), a[0].clone()];
        assert_eq!(
            MemoryReconciler::new().reconcile(&[], &a),
            MemoryReconciler::new().reconcile(&[], &b)
        );
    }
}
