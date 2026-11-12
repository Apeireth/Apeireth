//! Deterministic answer grading. Oracle values never participate in retrieval.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerStatus {
    Known,
    Unknown,
    Ambiguous,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FactValue {
    pub subject: String,
    pub predicate: String,
    pub value: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerFact {
    pub subject: String,
    pub predicate: String,
    pub value: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub sources: Vec<String>,
}

impl AnswerFact {
    fn value(&self) -> FactValue {
        FactValue {
            subject: self.subject.clone(),
            predicate: self.predicate.clone(),
            value: self.value.clone(),
            valid_from: self.valid_from.clone(),
            valid_to: self.valid_to.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub status: AnswerStatus,
    pub facts: Vec<AnswerFact>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExpectedFact {
    pub fact: FactValue,
    pub supporting_content_hashes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Oracle {
    pub status: AnswerStatus,
    pub facts: Vec<ExpectedFact>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceEvidence {
    pub id: String,
    pub episode_id: String,
    pub revision: i64,
    pub content_sha256: String,
    pub delivered_content_sha256: String,
    pub expected_projection_sha256: String,
    pub delivered_text: String,
    pub projection_verified: bool,
    pub source_role: String,
    pub claim_kind: String,
    pub citable_as_fact: bool,
    pub active: bool,
    pub authorized: bool,
    pub runtime_source_verified: bool,
    pub scope: serde_json::Value,
    pub provenance: serde_json::Value,
}

impl SourceEvidence {
    /// The oracle pins the complete durable source; delivery additionally has to
    /// match the production projection and retain the actual expected value.
    /// Visible substring presence alone never establishes a correct fact.
    pub fn supports(&self, expected: &ExpectedFact) -> bool {
        self.active
            && self.authorized
            && self.runtime_source_verified
            && self.projection_verified
            && self.citable_as_fact
            && self.source_role != "assistant"
            && self.source_role != "tool"
            && self.expected_projection_sha256 == self.delivered_content_sha256
            && super::io::hash(&self.delivered_text) == self.delivered_content_sha256
            && expected
                .supporting_content_hashes
                .contains(&self.content_sha256)
            && !expected.fact.value.is_empty()
            && self.delivered_text.contains(&expected.fact.value)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RequestEvidence {
    pub closed_tool_context_valid: bool,
    pub sources: Vec<SourceEvidence>,
    pub issues: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Grade {
    pub passed: bool,
    pub parse_valid: bool,
    pub facts_correct: bool,
    pub evidence_correct: bool,
    pub hard_failures: Vec<String>,
    /// Additional diagnostics never turn a failing grade into a passing grade.
    pub field_diagnostics: FactFieldDiagnostics,
    pub citation_diagnostics: Vec<CitationDiagnostic>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FactFieldDiagnostics {
    pub predicate_only_mismatch: bool,
    pub differing_fields: Vec<String>,
    pub missing_expected: Vec<FactValue>,
    pub unexpected_observed: Vec<FactValue>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CitationDiagnostic {
    pub fact_index: usize,
    pub source_id: String,
    pub issues: Vec<String>,
}

fn field_diagnostics(
    expected: &BTreeSet<FactValue>,
    observed: &BTreeSet<FactValue>,
) -> FactFieldDiagnostics {
    let mut diagnostics = FactFieldDiagnostics {
        missing_expected: expected.difference(observed).cloned().collect(),
        unexpected_observed: observed.difference(expected).cloned().collect(),
        ..FactFieldDiagnostics::default()
    };
    if expected == observed {
        return diagnostics;
    }
    // Compare unordered multisets after removing exactly one field. This avoids
    // assigning one answer fact to an unrelated oracle fact by array position.
    for field in ["subject", "predicate", "value", "valid_from", "valid_to"] {
        let project = |facts: &BTreeSet<FactValue>| {
            let mut rows: Vec<_> = facts
                .iter()
                .cloned()
                .map(|mut fact| {
                    match field {
                        "subject" => fact.subject.clear(),
                        "predicate" => fact.predicate.clear(),
                        "value" => fact.value.clear(),
                        "valid_from" => fact.valid_from = None,
                        "valid_to" => fact.valid_to = None,
                        _ => unreachable!(),
                    }
                    fact
                })
                .collect();
            rows.sort();
            rows
        };
        if project(expected) == project(observed) {
            diagnostics.differing_fields.push(field.into());
        }
    }
    diagnostics.predicate_only_mismatch = diagnostics.differing_fields == ["predicate"];
    if diagnostics.differing_fields.is_empty() {
        diagnostics
            .differing_fields
            .push("multiple_fields_or_fact_count".into());
    }
    diagnostics
}

/// Strict structured facts and independent durable/request receipts; no model judge.
pub fn score(raw: &str, oracle: &Oracle, request: &RequestEvidence) -> Grade {
    let mut grade = Grade::default();
    grade.hard_failures.extend(request.issues.iter().cloned());
    if !request.closed_tool_context_valid {
        grade.hard_failures.push("unclosed_tool_context".into());
    }
    let sources: BTreeMap<_, _> = request.sources.iter().map(|s| (s.id.as_str(), s)).collect();
    if sources.len() != request.sources.len() {
        grade.hard_failures.push("duplicate_source_receipt".into());
    }
    for source in &request.sources {
        if !source.active || !source.authorized || !source.runtime_source_verified {
            grade
                .hard_failures
                .push(format!("unauthorized_or_inactive_source:{}", source.id));
        }
        if !source.projection_verified
            || source.expected_projection_sha256 != source.delivered_content_sha256
            || super::io::hash(&source.delivered_text) != source.delivered_content_sha256
        {
            grade
                .hard_failures
                .push(format!("invalid_source_projection:{}", source.id));
        }
    }
    let answer: Answer = match serde_json::from_str(raw) {
        Ok(answer) => answer,
        Err(error) => {
            grade
                .hard_failures
                .push(format!("invalid_answer_json:{error}"));
            return grade;
        }
    };
    grade.parse_valid = true;
    if answer.status != oracle.status {
        grade.hard_failures.push("wrong_answer_status".into());
    }
    if answer.status != AnswerStatus::Known && !answer.facts.is_empty() {
        grade.hard_failures.push("abstention_contains_facts".into());
    }
    if answer.status == AnswerStatus::Known && answer.facts.is_empty() {
        grade
            .hard_failures
            .push("known_answer_without_facts".into());
    }
    let expected: BTreeSet<_> = oracle.facts.iter().map(|f| f.fact.clone()).collect();
    let observed: BTreeSet<_> = answer.facts.iter().map(AnswerFact::value).collect();
    grade.field_diagnostics = field_diagnostics(&expected, &observed);
    grade.facts_correct = expected == observed && observed.len() == answer.facts.len();
    if !grade.facts_correct {
        grade
            .hard_failures
            .push("wrong_subject_value_time_or_duplicate_fact".into());
    }
    let mut evidence_correct = true;
    for (fact_index, fact) in answer.facts.iter().enumerate() {
        let unique: BTreeSet<_> = fact.sources.iter().collect();
        if unique.is_empty() || unique.len() != fact.sources.len() {
            evidence_correct = false;
        }
        let expected_fact = oracle
            .facts
            .iter()
            .find(|expected| expected.fact == fact.value());
        for id in &fact.sources {
            let supported = sources.get(id.as_str()).is_some_and(|source| {
                expected_fact.is_some_and(|expected| source.supports(expected))
            });
            if !supported {
                evidence_correct = false;
                let mut issues = Vec::new();
                if expected_fact.is_none() {
                    issues.push("fact_tuple_not_in_oracle".into());
                }
                match sources.get(id.as_str()) {
                    None => {
                        issues.push("id_not_delivered".into());
                        if id
                            .strip_prefix("mem:")
                            .is_some_and(|bare| sources.contains_key(bare))
                        {
                            issues.push("display_prefix_is_not_part_of_source_id".into());
                        }
                    }
                    Some(source) => {
                        if !source.citable_as_fact
                            || matches!(source.source_role.as_str(), "assistant" | "tool")
                        {
                            issues.push("source_not_eligible_as_fact".into());
                        }
                        if !source.active || !source.authorized || !source.runtime_source_verified {
                            issues.push("source_not_active_authorized_and_runtime_verified".into());
                        }
                        if !source.projection_verified
                            || source.expected_projection_sha256 != source.delivered_content_sha256
                            || super::io::hash(&source.delivered_text)
                                != source.delivered_content_sha256
                        {
                            issues.push("delivery_does_not_match_exact_projection".into());
                        }
                        if let Some(expected) = expected_fact {
                            if !expected
                                .supporting_content_hashes
                                .contains(&source.content_sha256)
                            {
                                issues.push("durable_content_not_an_oracle_support".into());
                            }
                            if !source.delivered_text.contains(&expected.fact.value) {
                                issues.push("expected_value_not_visible_in_delivery".into());
                            }
                        }
                    }
                }
                grade.citation_diagnostics.push(CitationDiagnostic {
                    fact_index,
                    source_id: id.clone(),
                    issues,
                });
            }
        }
    }
    grade.evidence_correct = evidence_correct;
    if !evidence_correct {
        grade
            .hard_failures
            .push("missing_forged_stale_or_unsupported_citation".into());
    }
    grade.passed = grade.hard_failures.is_empty();
    grade
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn fixture() -> (Oracle, RequestEvidence, String) {
        let fact = FactValue {
            subject: "林禾".into(),
            predicate: "居住城市".into(),
            value: "上海".into(),
            valid_from: None,
            valid_to: None,
        };
        let oracle = Oracle {
            status: AnswerStatus::Known,
            facts: vec![ExpectedFact {
                fact,
                supporting_content_hashes: vec![super::super::io::hash("我住在上海。")],
            }],
        };
        let evidence = RequestEvidence {
            closed_tool_context_valid: true,
            sources: vec![SourceEvidence {
                id: "typed:episode:actual".into(),
                episode_id: "actual".into(),
                revision: 2,
                content_sha256: super::super::io::hash("我住在上海。"),
                delivered_content_sha256: super::super::io::hash("我住在上海。"),
                expected_projection_sha256: super::super::io::hash("我住在上海。"),
                delivered_text: "我住在上海。".into(),
                projection_verified: true,
                source_role: "user".into(),
                claim_kind: "direct_user_assertion".into(),
                citable_as_fact: true,
                active: true,
                authorized: true,
                runtime_source_verified: true,
                scope: serde_json::json!({"session_id":"a"}),
                provenance: serde_json::json!({"source_request":"r"}),
            }],
            issues: vec![],
        };
        let answer = r#"{"status":"known","facts":[{"subject":"林禾","predicate":"居住城市","value":"上海","valid_from":null,"valid_to":null,"sources":["typed:episode:actual"]}]}"#.into();
        (oracle, evidence, answer)
    }

    #[test]
    fn exact_fact_with_a_real_source_passes() {
        let (oracle, evidence, answer) = fixture();
        assert!(score(&answer, &oracle, &evidence).passed);
    }

    #[test]
    fn wrong_subject_stale_value_fake_citation_and_negated_substring_fail() {
        let (oracle, evidence, answer) = fixture();
        for (old, new) in [
            ("林禾", "周青"),
            ("上海", "武汉"),
            ("上海", "并非上海"),
            ("typed:episode:actual", "typed:episode:invented"),
        ] {
            assert!(!score(&answer.replace(old, new), &oracle, &evidence).passed);
        }
    }

    #[test]
    fn duplicate_fields_extra_fields_and_trailing_text_fail() {
        let (oracle, evidence, answer) = fixture();
        for altered in [
            answer.replacen("\"status\":", "\"status\":\"unknown\",\"status\":", 1),
            answer.replacen("\"value\":", "\"value\":\"武汉\",\"value\":", 1),
            answer.replacen("\"subject\":", "\"confidence\":1,\"subject\":", 1),
            format!("{answer}上海"),
        ] {
            assert!(!score(&altered, &oracle, &evidence).parse_valid);
        }
    }

    #[test]
    fn removed_recall_wrong_scope_hidden_record_and_stale_hash_fail() {
        let (oracle, evidence, answer) = fixture();
        let mut removed = evidence.clone();
        removed.sources.clear();
        let mut foreign = evidence.clone();
        foreign.sources[0].authorized = false;
        let mut hidden = evidence.clone();
        hidden.sources[0].active = false;
        let mut unverified = evidence.clone();
        unverified.sources[0].runtime_source_verified = false;
        let mut stale = evidence.clone();
        stale.sources[0].content_sha256 = "old-content".into();
        let mut forged_body = evidence.clone();
        forged_body.sources[0].delivered_content_sha256 = "forged-body-under-real-id".into();
        let mut unclosed = evidence;
        unclosed.closed_tool_context_valid = false;
        for altered in [
            removed,
            foreign,
            hidden,
            unverified,
            stale,
            forged_body,
            unclosed,
        ] {
            assert!(!score(&answer, &oracle, &altered).passed);
        }
    }

    #[test]
    fn unknown_and_ambiguous_require_empty_facts_and_the_correct_status() {
        let (_, evidence, answer) = fixture();
        for (status, word) in [
            (AnswerStatus::Unknown, "unknown"),
            (AnswerStatus::Ambiguous, "ambiguous"),
        ] {
            let oracle = Oracle {
                status,
                facts: vec![],
            };
            assert!(
                score(
                    &format!(r#"{{"status":"{word}","facts":[]}}"#),
                    &oracle,
                    &evidence
                )
                .passed
            );
            assert!(!score(&answer.replacen("known", word, 1), &oracle, &evidence).passed);
        }
    }

    #[test]
    fn correct_answer_cannot_hide_an_extra_fact_or_wrong_effective_date() {
        let (oracle, evidence, answer) = fixture();
        let mut parsed: Answer = serde_json::from_str(&answer).unwrap();
        parsed.facts.push(parsed.facts[0].clone());
        assert!(!score(&serde_json::to_string(&parsed).unwrap(), &oracle, &evidence).passed);
        parsed.facts.pop();
        parsed.facts[0].valid_from = Some("2026-12-01".into());
        assert!(!score(&serde_json::to_string(&parsed).unwrap(), &oracle, &evidence).passed);
    }

    #[test]
    fn diagnostics_separate_predicate_only_value_and_id_format_without_accepting_them() {
        let (oracle, evidence, answer) = fixture();
        let predicate = score(&answer.replace("居住城市", "居住地"), &oracle, &evidence);
        assert!(!predicate.passed);
        assert!(predicate.field_diagnostics.predicate_only_mismatch);
        assert!(!predicate.evidence_correct);
        let wrong_value = score(&answer.replace("上海", "武汉"), &oracle, &evidence);
        assert!(!wrong_value.passed);
        assert!(!wrong_value.field_diagnostics.predicate_only_mismatch);
        assert_eq!(
            wrong_value.field_diagnostics.differing_fields,
            vec!["value"]
        );
        let prefix = score(
            &answer.replace("typed:episode:actual", "mem:typed:episode:actual"),
            &oracle,
            &evidence,
        );
        assert!(!prefix.passed);
        assert!(prefix.citation_diagnostics[0]
            .issues
            .iter()
            .any(|s| s == "display_prefix_is_not_part_of_source_id"));
    }

    #[test]
    fn exact_projection_requires_visible_support_and_fact_authority() {
        let (mut oracle, mut evidence, answer) = fixture();
        let full = format!("我住在上海。{}", "附加上下文".repeat(100));
        oracle.facts[0].supporting_content_hashes = vec![super::super::io::hash(&full)];
        let source = &mut evidence.sources[0];
        source.content_sha256 = super::super::io::hash(&full);
        source.delivered_text = apeireth_memory::context_compiler::project_memory_content(
            &full,
            apeireth_memory::context_compiler::DELIVERY_SPAN_CHARS,
        );
        source.delivered_content_sha256 = super::super::io::hash(&source.delivered_text);
        source.expected_projection_sha256 = source.delivered_content_sha256.clone();
        assert!(score(&answer, &oracle, &evidence).passed);

        let mut changed = evidence.clone();
        changed.sources[0].delivered_text.push('伪');
        changed.sources[0].delivered_content_sha256 =
            super::super::io::hash(&changed.sources[0].delivered_text);
        assert!(!score(&answer, &oracle, &changed).passed);

        let hidden = format!("{}我住在上海。", "无关".repeat(110));
        oracle.facts[0].supporting_content_hashes = vec![super::super::io::hash(&hidden)];
        evidence.sources[0].content_sha256 = super::super::io::hash(&hidden);
        evidence.sources[0].delivered_text =
            apeireth_memory::context_compiler::project_memory_content(
                &hidden,
                apeireth_memory::context_compiler::DELIVERY_SPAN_CHARS,
            );
        evidence.sources[0].delivered_content_sha256 =
            super::super::io::hash(&evidence.sources[0].delivered_text);
        evidence.sources[0].expected_projection_sha256 =
            evidence.sources[0].delivered_content_sha256.clone();
        let hidden_grade = score(&answer, &oracle, &evidence);
        assert!(!hidden_grade.passed);
        assert!(hidden_grade.citation_diagnostics[0]
            .issues
            .iter()
            .any(|s| s == "expected_value_not_visible_in_delivery"));

        let (oracle, mut evidence, answer) = fixture();
        evidence.sources[0].source_role = "assistant".into();
        // Even a forged eligibility bit cannot turn an assistant echo into a
        // factual authority. The request audit also checks this against SQL.
        evidence.sources[0].citable_as_fact = true;
        assert!(!score(&answer, &oracle, &evidence).passed);
    }
}
