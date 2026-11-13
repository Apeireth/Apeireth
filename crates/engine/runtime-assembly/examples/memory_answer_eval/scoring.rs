//! Deterministic answer grading. Oracle values never participate in retrieval.

use std::collections::{BTreeMap, BTreeSet};

use apeireth_memory::automatic_correction::AutomaticCorrectionPlan;
use apeireth_memory::context_compiler::VerifiedCorrectionSupport;
use serde::{Deserialize, Serialize};

pub const SCORING_PROTOCOL: &str = "apeireth.memory-answer-scoring.v2";

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
    pub scoring_protocol: String,
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
    pub correction_support: Vec<CorrectionSupportProof>,
}

/// Independent SQL evidence, persisted outside every provider request.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionSupportProof {
    pub scoring_protocol: String,
    pub edge: VerifiedCorrectionSupport,
    pub frozen_plan: AutomaticCorrectionPlan,
    pub applied_payload_hash: String,
}

impl SourceEvidence {
    /// The oracle pins the complete durable source; delivery additionally has to
    /// match the production projection and retain the actual expected value.
    /// Visible substring presence alone never establishes a correct fact.
    fn eligible_for(&self, expected: &ExpectedFact) -> bool {
        self.scoring_protocol == SCORING_PROTOCOL
            && self.active
            && self.authorized
            && self.runtime_source_verified
            && self.projection_verified
            && self.citable_as_fact
            && self.source_role != "assistant"
            && self.source_role != "tool"
            && self.expected_projection_sha256 == self.delivered_content_sha256
            && super::io::hash(&self.delivered_text) == self.delivered_content_sha256
            && !expected.fact.value.is_empty()
            && self.delivered_text.contains(&expected.fact.value)
    }

    pub fn supports(&self, expected: &ExpectedFact) -> bool {
        self.eligible_for(expected)
            && expected
                .supporting_content_hashes
                .contains(&self.content_sha256)
    }

    fn supports_through(&self, anchor: &SourceEvidence, expected: &ExpectedFact) -> bool {
        // V2 admits one direct correction-evidence edge for a current fact.
        // Historical/derived edges and arbitrary graph reachability are absent.
        expected.fact.valid_from.is_none()
            && expected.fact.valid_to.is_none()
            && self.eligible_for(expected)
            && self.source_role == "user"
            && self.claim_kind == "governed_correction"
            && matches!(anchor.source_role.as_str(), "user" | "memory")
            && anchor.claim_kind == "governed_correction"
            && anchor.supports(expected)
            && self.episode_id != anchor.episode_id
            && self.correction_support.iter().any(|proof| {
                proof.binds(self, anchor) && proof.supports_self_residence(self, anchor, expected)
            })
    }
}

struct LiteralSelfValue<'a> {
    value: &'a str,
    clause: &'a str,
    range: std::ops::Range<usize>,
}

#[derive(Default)]
struct LiteralSelfStatements<'a> {
    names: Vec<LiteralSelfValue<'a>>,
    residences: Vec<LiteralSelfValue<'a>>,
}

fn dated_home_relocation(clause: &str) -> bool {
    let Some(rest) = clause
        .strip_prefix("我于")
        .or_else(|| clause.strip_prefix("本人于"))
    else {
        return false;
    };
    ["搬到了新家", "搬到新家"].iter().any(|suffix| {
        rest.strip_suffix(suffix).is_some_and(|date| {
            let date = date.trim();
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
                || chrono::NaiveDate::parse_from_str(date, "%Y年%m月%d日").is_ok()
        })
    })
}

/// A deliberately finite audit grammar, separate from model instructions.
/// Unknown forms cannot extend an episode lineage into fact-level support.
fn literal_self_statements(text: &str) -> Option<LiteralSelfStatements<'_>> {
    let lower = text.to_lowercase();
    if text.contains([
        '"', '\'', '“', '”', '‘', '’', '「', '」', '『', '』', '`', '>',
    ]) || [
        "他说",
        "她说",
        "据说",
        "听说",
        "转述",
        "引用",
        "表示",
        "声称",
        "提到",
        "写道",
        "假设",
        "假如",
        "如果",
        "比如",
        "例如",
        "设想",
        "也许",
        "可能",
        "不确定",
        "小说",
        "虚构",
        "并非",
        "不是",
        "不再",
        "不住",
        "assistant:",
        "assistant：",
        "user:",
        "user：",
        "<|",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return None;
    }
    let names = [
        "本人的姓名是",
        "本人的名字是",
        "我的姓名是",
        "我的名字是",
        "本人名叫",
        "我名叫",
        "本人叫",
        "我叫",
    ];
    let residences = [
        "本人的居住城市是",
        "本人的常住城市是",
        "我的居住城市是",
        "我的常住城市是",
        "本人的常住地是",
        "本人的居住地是",
        "我的常住地是",
        "我的居住地是",
        "本人居住在",
        "本人常住在",
        "我居住在",
        "我常住在",
        "本人住在",
        "我住在",
        "本人现居于",
        "我现居于",
    ];
    let boundaries = ['，', ',', '。', '！', '!', '；', ';', '\n'];
    let mut statements = LiteralSelfStatements::default();
    let mut offset = 0;
    for part in text.split_inclusive(boundaries) {
        let start = offset;
        offset += part.len();
        let untrimmed = part.trim().trim_end_matches(boundaries).trim_end();
        if untrimmed.is_empty() {
            continue;
        }
        let mut clause = untrimmed;
        for prefix in [
            "请记住这些真实信息：",
            "请记住这些真实信息:",
            "请记住这条真实信息：",
            "请记住这条真实信息:",
            "请记住：",
            "请记住:",
            "记住：",
            "记住:",
            "更正：",
            "更正:",
            "纠正：",
            "纠正:",
        ] {
            if let Some(rest) = clause.strip_prefix(prefix) {
                clause = rest.trim_start();
                break;
            }
        }
        let matched = names
            .iter()
            .find_map(|prefix| clause.strip_prefix(prefix).map(|value| (true, value)))
            .or_else(|| {
                residences
                    .iter()
                    .find_map(|prefix| clause.strip_prefix(prefix).map(|value| (false, value)))
            });
        let Some((name, tail)) = matched else {
            // Ignoring an unsupported self clause could hide an alternate
            // name or contradictory residence behind an adverb. A dated move
            // to a new home is a supported event alongside the separately
            // asserted city, never itself evidence of a city value.
            if (clause.starts_with('我') || clause.starts_with("本人"))
                && !dated_home_relocation(clause)
            {
                return None;
            }
            continue;
        };
        let value = tail.trim();
        if value.is_empty() || value.contains(['?', '？']) {
            return None;
        }
        // All transformations above remove only a prefix and trailing space.
        // The value range therefore names the complete original literal,
        // including its internal spaces, rather than a matching substring.
        let value_start =
            start + part.len() - part.trim_start().len() + untrimmed.len() - value.len();
        let literal = LiteralSelfValue {
            value,
            clause,
            range: value_start..value_start + value.len(),
        };
        if name {
            statements.names.push(literal);
        } else {
            statements.residences.push(literal);
        }
    }
    Some(statements)
}

fn sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl CorrectionSupportProof {
    fn supports_self_residence(
        &self,
        source: &SourceEvidence,
        target: &SourceEvidence,
        expected: &ExpectedFact,
    ) -> bool {
        // The edge proves an episode operation. Extending it to a fact also
        // requires a complete, unchanged self identity and the exact property
        // actually changed. Same-value facts elsewhere in the episode do not
        // acquire support from a residence correction.
        if expected.fact.predicate != "居住城市" {
            return false;
        }
        let Some(item) = self
            .frozen_plan
            .targets
            .iter()
            .find(|item| item.id == target.episode_id)
        else {
            return false;
        };
        let (Some(before), Some(after), Some(raw)) = (
            literal_self_statements(&item.before_content),
            literal_self_statements(&item.replacement_content),
            literal_self_statements(&self.frozen_plan.evidence.content),
        ) else {
            return false;
        };
        let ([old_name], [new_name], [old_city], [new_city], [raw_city]) = (
            before.names.as_slice(),
            after.names.as_slice(),
            before.residences.as_slice(),
            after.residences.as_slice(),
            raw.residences.as_slice(),
        ) else {
            return false;
        };
        old_name.value == expected.fact.subject
            && new_name.value == old_name.value
            && raw.names.len() <= 1
            && raw.names.iter().all(|name| name.value == old_name.value)
            && old_city.value != new_city.value
            && new_city.value == expected.fact.value
            && raw_city.value == new_city.value
            && item.before_content[..old_city.range.start]
                == item.replacement_content[..new_city.range.start]
            && item.before_content[old_city.range.end..]
                == item.replacement_content[new_city.range.end..]
            && source.delivered_text.contains(raw_city.clause)
            && target.delivered_text.contains(new_name.clause)
            && target.delivered_text.contains(new_city.clause)
    }

    pub(super) fn binds(&self, source: &SourceEvidence, target: &SourceEvidence) -> bool {
        let edge = &self.edge;
        let plan = &self.frozen_plan;
        let scope = serde_json::json!({"scope":"session","session_id":edge.namespace});
        self.scoring_protocol == SCORING_PROTOCOL
            && !edge.namespace.is_empty()
            && !edge.operation_id.is_empty()
            && sha256_hex(&edge.source_content_sha256)
            && sha256_hex(&edge.target_content_sha256)
            && sha256_hex(&edge.payload_hash)
            && edge.source_episode_id == source.episode_id
            && edge.source_content_sha256 == source.content_sha256
            && edge.target_episode_id == target.episode_id
            && edge.target_revision == target.revision
            && edge.target_content_sha256 == target.content_sha256
            && source.scope == scope
            && target.scope == scope
            && plan.namespace == edge.namespace
            && plan.operation_id == edge.operation_id
            && !plan.targets.is_empty()
            && plan.targets.len() <= 16
            && self.applied_payload_hash == edge.payload_hash
            && apeireth_memory::writeback_recovery::payload_hash(plan)
                .ok()
                .as_deref()
                == Some(edge.payload_hash.as_str())
            && plan.evidence.id == source.episode_id
            && plan.evidence.role == "user"
            && plan.evidence.session_id == edge.namespace
            && plan.evidence.timestamp == plan.recorded_at_ms / 1000
            && super::io::hash(&plan.evidence.content) == source.content_sha256
            && plan.valid_from_ms >= 0
            && plan.valid_from_ms <= plan.recorded_at_ms
            && plan.provenance.source_session.as_deref()
                == source.provenance["source_session"].as_str()
            && plan.provenance.source_request.as_deref()
                == source.provenance["source_request"].as_str()
            && plan
                .provenance
                .source_session
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && plan
                .provenance
                .source_request
                .as_deref()
                .is_some_and(|value| !value.is_empty())
            && plan.operation_id
                == apeireth_memory::automatic_correction::correction_operation_id(
                    &plan.namespace,
                    plan.provenance.source_request.as_deref().unwrap_or(""),
                )
            && plan.evidence.id
                == apeireth_memory::automatic_correction::correction_evidence_id(
                    &plan.namespace,
                    plan.provenance.source_request.as_deref().unwrap_or(""),
                )
            && plan.targets.iter().any(|item| {
                item.id == target.episode_id
                    && item.expected_revision >= 0
                    && item.expected_revision.checked_add(1) == Some(target.revision)
                    && super::io::hash(&item.replacement_content) == target.content_sha256
                    && target.provenance["source_session"].as_str()
                        == Some(item.source_session.as_str())
                    && target.provenance["source_request"].as_str()
                        == Some(item.source_request.as_str())
            })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RequestEvidence {
    pub scoring_protocol: String,
    pub closed_tool_context_valid: bool,
    pub sources: Vec<SourceEvidence>,
    pub issues: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Grade {
    pub scoring_protocol: String,
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
    let mut grade = Grade {
        scoring_protocol: SCORING_PROTOCOL.into(),
        ..Grade::default()
    };
    if request.scoring_protocol != SCORING_PROTOCOL {
        grade.hard_failures.push("scoring_protocol_mismatch".into());
    }
    grade.hard_failures.extend(request.issues.iter().cloned());
    if !request.closed_tool_context_valid {
        grade.hard_failures.push("unclosed_tool_context".into());
    }
    let sources: BTreeMap<_, _> = request.sources.iter().map(|s| (s.id.as_str(), s)).collect();
    if sources.len() != request.sources.len() {
        grade.hard_failures.push("duplicate_source_receipt".into());
    }
    for source in &request.sources {
        if source.scoring_protocol != SCORING_PROTOCOL {
            grade
                .hard_failures
                .push(format!("source_scoring_protocol_mismatch:{}", source.id));
        }
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
        // Each fact needs its own original gold citation, actually cited here.
        // An edge alone, another fact's anchor or an uncited delivered row
        // cannot replace this requirement.
        let anchors: Vec<_> = fact
            .sources
            .iter()
            .filter_map(|id| sources.get(id.as_str()).copied())
            .filter(|source| expected_fact.is_some_and(|expected| source.supports(expected)))
            .collect();
        if anchors.is_empty() {
            evidence_correct = false;
            grade
                .hard_failures
                .push(format!("missing_direct_gold_anchor:{fact_index}"));
        }
        for id in &fact.sources {
            let supported = sources.get(id.as_str()).is_some_and(|source| {
                expected_fact.is_some_and(|expected| {
                    source.supports(expected)
                        || anchors
                            .iter()
                            .any(|anchor| source.supports_through(anchor, expected))
                })
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
                                issues.push("no_direct_gold_or_verified_single_hop_support".into());
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
            scoring_protocol: SCORING_PROTOCOL.into(),
            closed_tool_context_valid: true,
            sources: vec![SourceEvidence {
                scoring_protocol: SCORING_PROTOCOL.into(),
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
                scope: serde_json::json!({"scope":"session","session_id":"a"}),
                provenance: serde_json::json!({"source_session":"a","source_request":"r"}),
                correction_support: vec![],
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

    fn correction_fixture() -> (Oracle, RequestEvidence, String) {
        correction_fixture_with_contents(
            "我叫林禾，我住在武汉。",
            "我叫林禾，我住在上海。",
            "更正：我住在上海。",
        )
    }

    fn correction_fixture_with_contents(
        before: &str,
        after: &str,
        raw: &str,
    ) -> (Oracle, RequestEvidence, String) {
        use apeireth_memory::automatic_correction::{
            correction_evidence_id, correction_operation_id, AutomaticCorrectionKind,
            AutomaticCorrectionTarget,
        };
        let (mut oracle, mut evidence, answer) = fixture();
        let canonical_hash = super::super::io::hash(after);
        oracle.facts[0].supporting_content_hashes = vec![canonical_hash.clone()];
        evidence.sources[0].content_sha256 = canonical_hash.clone();
        evidence.sources[0].delivered_content_sha256 = canonical_hash.clone();
        evidence.sources[0].expected_projection_sha256 = canonical_hash;
        evidence.sources[0].delivered_text = after.into();
        evidence.sources[0].claim_kind = "governed_correction".into();
        let request = "correction-r";
        let plan = AutomaticCorrectionPlan {
            operation_id: correction_operation_id("a", request),
            namespace: "a".into(),
            evidence: apeireth_core::Episode {
                id: correction_evidence_id("a", request),
                timestamp: 2,
                role: "user".into(),
                content: raw.into(),
                session_id: "a".into(),
            },
            provenance: apeireth_memory::MemoryProvenance {
                source: "direct_user_correction".into(),
                source_session: Some("a".into()),
                source_trace: Some("trace".into()),
                source_request: Some(request.into()),
            },
            authorization_epoch: None,
            kind: AutomaticCorrectionKind::CorrectPrior,
            valid_from_ms: 2000,
            recorded_at_ms: 2000,
            targets: vec![AutomaticCorrectionTarget {
                id: "actual".into(),
                expected_revision: 1,
                before_content: before.into(),
                replacement_content: after.into(),
                source_session: "a".into(),
                source_request: "r".into(),
            }],
        };
        let hash = super::super::io::hash(raw);
        let payload_hash = apeireth_memory::writeback_recovery::payload_hash(&plan).unwrap();
        let edge = VerifiedCorrectionSupport {
            source_episode_id: plan.evidence.id.clone(),
            source_content_sha256: hash.clone(),
            target_episode_id: "actual".into(),
            target_revision: 2,
            target_content_sha256: evidence.sources[0].content_sha256.clone(),
            namespace: "a".into(),
            operation_id: plan.operation_id.clone(),
            payload_hash: payload_hash.clone(),
        };
        let extra = SourceEvidence {
            scoring_protocol: SCORING_PROTOCOL.into(),
            id: plan.evidence.id.clone(),
            episode_id: plan.evidence.id.clone(),
            revision: 0,
            content_sha256: hash.clone(),
            delivered_content_sha256: hash.clone(),
            expected_projection_sha256: hash,
            delivered_text: raw.into(),
            projection_verified: true,
            source_role: "user".into(),
            claim_kind: "governed_correction".into(),
            citable_as_fact: true,
            active: true,
            authorized: true,
            runtime_source_verified: true,
            scope: evidence.sources[0].scope.clone(),
            provenance: serde_json::to_value(&plan.provenance).unwrap(),
            correction_support: vec![CorrectionSupportProof {
                scoring_protocol: SCORING_PROTOCOL.into(),
                edge,
                frozen_plan: plan,
                applied_payload_hash: payload_hash,
            }],
        };
        let mut parsed: Answer = serde_json::from_str(&answer).unwrap();
        parsed.facts[0].sources.push(extra.id.clone());
        evidence.sources.push(extra);
        (oracle, evidence, serde_json::to_string(&parsed).unwrap())
    }

    #[test]
    fn v2_extra_correction_binds_the_changed_self_fact_not_same_value_episode_neighbors() {
        for (before, after, subject, predicate, value) in [
            (
                "我叫林禾，我住在武汉。晨星项目的联络城市是上海。",
                "我叫林禾，我住在上海。晨星项目的联络城市是上海。",
                "晨星项目",
                "联络城市",
                "上海",
            ),
            (
                "我叫林禾，我住在武汉。周青的居住城市是上海。",
                "我叫林禾，我住在上海。周青的居住城市是上海。",
                "周青",
                "居住城市",
                "上海",
            ),
            (
                "我叫上海，我住在武汉。",
                "我叫上海，我住在上海。",
                "上海",
                "姓名",
                "上海",
            ),
            (
                "我叫林禾，我住在武汉。我的昵称是上海。",
                "我叫林禾，我住在上海。我的昵称是上海。",
                "林禾",
                "昵称",
                "上海",
            ),
        ] {
            let (mut oracle, evidence, answer) =
                correction_fixture_with_contents(before, after, "更正：我住在上海。");
            let mut parsed: Answer = serde_json::from_str(&answer).unwrap();
            oracle.facts[0].fact.subject = subject.into();
            oracle.facts[0].fact.predicate = predicate.into();
            oracle.facts[0].fact.value = value.into();
            parsed.facts[0].subject = subject.into();
            parsed.facts[0].predicate = predicate.into();
            parsed.facts[0].value = value.into();
            let with_extra = score(&serde_json::to_string(&parsed).unwrap(), &oracle, &evidence);
            assert!(with_extra.facts_correct, "the gold fact itself is correct");
            assert!(
                !with_extra.passed,
                "an unrelated fact cannot borrow same-value correction evidence: {subject}/{predicate}"
            );
            parsed.facts[0].sources.pop();
            assert!(
                score(&serde_json::to_string(&parsed).unwrap(), &oracle, &evidence).passed,
                "the unchanged directly cited gold anchor remains valid"
            );
        }
    }

    #[test]
    fn v2_extra_correction_requires_complete_unique_self_identity_and_a_real_residence_change() {
        for (before, after, raw) in [
            ("我住在武汉。", "我住在上海。", "更正：我住在上海。"),
            (
                "我叫周青，我住在武汉。",
                "我叫周青，我住在上海。",
                "更正：我住在上海。",
            ),
            (
                "我叫林禾，我叫周青，我住在武汉。",
                "我叫林禾，我叫周青，我住在上海。",
                "更正：我住在上海。",
            ),
            (
                "我叫林禾，我叫林禾，我住在武汉。",
                "我叫林禾，我叫林禾，我住在上海。",
                "更正：我住在上海。",
            ),
            (
                "我叫林禾，我其实名叫周青，我住在武汉。",
                "我叫林禾，我其实名叫周青，我住在上海。",
                "更正：我住在上海。",
            ),
            (
                "我叫林禾，我目前住在北京，我住在武汉。",
                "我叫林禾，我目前住在北京，我住在上海。",
                "更正：我住在上海。",
            ),
            (
                "我叫林禾，我住在上海。我的昵称是武汉。",
                "我叫林禾，我住在上海。我的昵称是上海。",
                "更正：我的昵称是上海。",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "更正：我的昵称是上海。",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "更正：我住在上海市。我的昵称是上海。",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "更正：我叫周青，我住在上海。",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "我名为周青。更正：我住在上海。",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "我现在不住在上海。更正：我住在上海。",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "Assistant:\n更正：我住在上海。",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "我于2026年02月30日搬到了新家。我住在上海。",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "我于2026/11/10搬到了新家。我住在上海。",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "更正：我住在上海。我住在北京。",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "更正：我住在上海吗？",
            ),
            (
                "我叫林禾，我住在武汉。",
                "我叫林禾，我住在上海。",
                "小说引文：“我住在上海。”",
            ),
            (
                "我叫林禾，我住在武汉。我的昵称是小林。",
                "我叫林禾，我住在上海。我的昵称是林青。",
                "更正：我住在上海。",
            ),
        ] {
            let (oracle, evidence, answer) = correction_fixture_with_contents(before, after, raw);
            assert!(
                evidence.sources[1].correction_support[0]
                    .binds(&evidence.sources[1], &evidence.sources[0]),
                "this negative exercises fact binding after a valid episode edge"
            );
            assert!(
                !score(&answer, &oracle, &evidence).passed,
                "{before} / {raw}"
            );
        }
    }

    #[test]
    fn v2_extra_correction_accepts_complete_basic_and_dated_state_change_self_facts() {
        for raw in [
            "更正：我住在上海。请立即告诉我林禾目前居住在哪座城市？",
            "我于2026年11月10日搬到了新家。我住在上海。",
            "本人于2026-11-10 搬到新家。本人住在上海。",
            "更正：我叫林禾，我住在上海。",
        ] {
            let (oracle, evidence, answer) = correction_fixture_with_contents(
                "请记住这些真实信息：我叫林禾，我住在武汉。",
                "请记住这些真实信息：我叫林禾，我住在上海。",
                raw,
            );
            assert!(score(&answer, &oracle, &evidence).passed, "{raw}");
        }
        let (mut oracle, evidence, answer) = correction_fixture_with_contents(
            " \t我叫 Mary Jane ， 我住在 武汉 。",
            " \t我叫 Mary Jane ， 我住在 上海 。",
            " 更正：我住在 上海 。请核对现行资料。",
        );
        oracle.facts[0].fact.subject = "Mary Jane".into();
        let mut parsed: Answer = serde_json::from_str(&answer).unwrap();
        parsed.facts[0].subject = "Mary Jane".into();
        assert!(
            score(&serde_json::to_string(&parsed).unwrap(), &oracle, &evidence).passed,
            "complete values retain internal spaces and surrounding formatting"
        );
        let (oracle, mut evidence, answer) = correction_fixture_with_contents(
            "我叫林禾，我住在武汉。",
            "我叫林禾，我住在上海。",
            "更正：我住在上海。",
        );
        evidence.sources[1].delivered_text = "上海".into();
        let delivered_hash = super::super::io::hash("上海");
        evidence.sources[1].delivered_content_sha256 = delivered_hash.clone();
        evidence.sources[1].expected_projection_sha256 = delivered_hash;
        assert!(
            !score(&answer, &oracle, &evidence).passed,
            "visible value without its complete self-property clause is insufficient"
        );
    }

    #[test]
    fn v2_extra_correction_requires_a_direct_gold_anchor_and_exact_single_hop() {
        let (oracle, evidence, answer) = correction_fixture();
        let passed = score(&answer, &oracle, &evidence);
        assert!(passed.passed, "{passed:?}");
        assert_eq!(passed.scoring_protocol, SCORING_PROTOCOL);
        let mut no_edge = evidence.clone();
        no_edge.sources[1].correction_support.clear();
        assert!(
            !score(&answer, &oracle, &no_edge).passed,
            "removing the actual proof reverses the result"
        );
        let mut extra_only: Answer = serde_json::from_str(&answer).unwrap();
        extra_only.facts[0].sources.remove(0);
        let rejected = score(
            &serde_json::to_string(&extra_only).unwrap(),
            &oracle,
            &evidence,
        );
        assert!(!rejected.passed);
        assert!(rejected
            .hard_failures
            .iter()
            .any(|failure| failure == "missing_direct_gold_anchor:0"));
        let mut uncited = extra_only;
        uncited.facts[0].sources = vec!["not-delivered".into(), evidence.sources[1].id.clone()];
        assert!(
            !score(
                &serde_json::to_string(&uncited).unwrap(),
                &oracle,
                &evidence
            )
            .passed
        );
    }

    #[test]
    fn v2_forged_edges_receipts_old_versions_and_scope_changes_fail() {
        let (oracle, evidence, answer) = correction_fixture();
        let mut variants = Vec::new();
        let mut fake = evidence.clone();
        fake.sources[1].correction_support[0].edge.target_episode_id =
            "same-content-other-subject".into();
        variants.push(fake);
        let mut fake = evidence.clone();
        fake.sources[1].correction_support[0]
            .edge
            .source_content_sha256 = "0".repeat(64);
        variants.push(fake);
        let mut fake = evidence.clone();
        fake.sources[1].correction_support[0].edge.payload_hash = "0".repeat(64);
        variants.push(fake);
        let mut fake = evidence.clone();
        fake.sources[1].correction_support[0]
            .applied_payload_hash
            .clear();
        variants.push(fake);
        let mut fake = evidence.clone();
        fake.sources[1].correction_support[0]
            .frozen_plan
            .evidence
            .content = "他人的上海。".into();
        variants.push(fake);
        let mut fake = evidence.clone();
        fake.sources[0].revision += 1;
        variants.push(fake);
        let mut fake = evidence.clone();
        fake.sources[1].scope = serde_json::json!({"scope":"session","session_id":"foreign"});
        variants.push(fake);
        let mut fake = evidence.clone();
        fake.sources[1].correction_support[0].edge.namespace = "foreign".into();
        variants.push(fake);
        let mut fake = evidence.clone();
        fake.sources[1].correction_support[0].scoring_protocol =
            "apeireth.memory-answer-scoring.v1".into();
        variants.push(fake);
        let mut fake = evidence.clone();
        fake.scoring_protocol = "apeireth.memory-answer-scoring.v1".into();
        variants.push(fake);
        for altered in variants {
            assert!(!score(&answer, &oracle, &altered).passed);
        }
    }

    #[test]
    fn v2_extra_authority_projection_and_visible_value_remain_mandatory() {
        let (oracle, evidence, answer) = correction_fixture();
        for role in ["assistant", "tool", "memory", "fact_history", "unknown"] {
            let mut altered = evidence.clone();
            altered.sources[1].source_role = role.into();
            assert!(!score(&answer, &oracle, &altered).passed, "{role}");
        }
        let mut hidden_value = evidence.clone();
        hidden_value.sources[1].delivered_text = "更正：".into();
        hidden_value.sources[1].delivered_content_sha256 = super::super::io::hash("更正：");
        hidden_value.sources[1].expected_projection_sha256 = super::super::io::hash("更正：");
        assert!(!score(&answer, &oracle, &hidden_value).passed);
        for flag in ["active", "authorized", "runtime", "projection", "citable"] {
            for index in [0, 1] {
                let mut altered = evidence.clone();
                match flag {
                    "active" => altered.sources[index].active = false,
                    "authorized" => altered.sources[index].authorized = false,
                    "runtime" => altered.sources[index].runtime_source_verified = false,
                    "projection" => altered.sources[index].projection_verified = false,
                    "citable" => altered.sources[index].citable_as_fact = false,
                    _ => unreachable!(),
                }
                assert!(
                    !score(&answer, &oracle, &altered).passed,
                    "{flag} source {index}"
                );
            }
        }
        let mut historical_anchor = evidence;
        historical_anchor.sources[0].source_role = "fact_history".into();
        assert!(!score(&answer, &oracle, &historical_anchor).passed);
    }

    #[test]
    fn v2_other_fact_anchor_same_text_and_transitive_links_do_not_authorize_extra_citations() {
        let (mut oracle, mut evidence, answer) = correction_fixture();
        let mut other = oracle.facts[0].clone();
        other.fact.subject = "周青".into();
        oracle.facts.push(other);
        let mut other_source = evidence.sources[0].clone();
        other_source.id = "other-subject".into();
        other_source.episode_id = "other-subject".into();
        evidence.sources.push(other_source);
        let mut parsed: Answer = serde_json::from_str(&answer).unwrap();
        parsed.facts[0].sources = vec![evidence.sources[0].id.clone()];
        let mut second = parsed.facts[0].clone();
        second.subject = "周青".into();
        second.sources = vec!["other-subject".into(), evidence.sources[1].id.clone()];
        parsed.facts.push(second);
        assert!(
            !score(&serde_json::to_string(&parsed).unwrap(), &oracle, &evidence).passed,
            "same body/hash is not the same target identity or fact anchor"
        );
        parsed.facts[1].sources.remove(0);
        assert!(
            !score(&serde_json::to_string(&parsed).unwrap(), &oracle, &evidence).passed,
            "the first fact's anchor cannot serve the second fact"
        );

        let (oracle, mut evidence, answer) = correction_fixture();
        let mut bridge = evidence.sources[1].clone();
        bridge.revision = 1;
        let mut extra = bridge.clone();
        let request = "another-correction";
        let raw = "再次更正：我住在上海。";
        let hash = super::super::io::hash(raw);
        extra.id = apeireth_memory::automatic_correction::correction_evidence_id("a", request);
        extra.episode_id = extra.id.clone();
        extra.content_sha256 = hash.clone();
        extra.delivered_content_sha256 = hash.clone();
        extra.expected_projection_sha256 = hash.clone();
        extra.delivered_text = raw.into();
        let proof = &mut extra.correction_support[0];
        proof.frozen_plan.operation_id =
            apeireth_memory::automatic_correction::correction_operation_id("a", request);
        proof.frozen_plan.evidence.id = extra.id.clone();
        proof.frozen_plan.evidence.content = raw.into();
        proof.frozen_plan.provenance.source_request = Some(request.into());
        extra.provenance = serde_json::to_value(&proof.frozen_plan.provenance).unwrap();
        proof.frozen_plan.targets[0].id = bridge.episode_id.clone();
        proof.frozen_plan.targets[0].expected_revision = 0;
        proof.frozen_plan.targets[0].source_request = "correction-r".into();
        proof.frozen_plan.targets[0].replacement_content = bridge.delivered_text.clone();
        proof.edge.source_episode_id = extra.id.clone();
        proof.edge.source_content_sha256 = hash;
        proof.edge.target_episode_id = bridge.episode_id.clone();
        proof.edge.target_revision = bridge.revision;
        proof.edge.target_content_sha256 = bridge.content_sha256.clone();
        proof.edge.operation_id = proof.frozen_plan.operation_id.clone();
        proof.edge.payload_hash =
            apeireth_memory::writeback_recovery::payload_hash(&proof.frozen_plan).unwrap();
        proof.applied_payload_hash = proof.edge.payload_hash.clone();
        assert!(extra.correction_support[0].binds(&extra, &bridge));
        evidence.sources[1] = bridge;
        assert!(
            score(&answer, &oracle, &evidence).passed,
            "the bridge's direct canonical proof is valid"
        );
        let mut parsed: Answer = serde_json::from_str(&answer).unwrap();
        parsed.facts[0].sources.push(extra.id.clone());
        evidence.sources.push(extra);
        assert!(
            !score(&serde_json::to_string(&parsed).unwrap(), &oracle, &evidence).passed,
            "even cited, independently bound edges never supply an implicit second hop"
        );
    }
}
