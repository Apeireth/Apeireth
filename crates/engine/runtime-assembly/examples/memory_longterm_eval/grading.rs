//! Grading.
//!
//! A grade is only green when every answer element is traceable to a concrete
//! evidence entry: the cited identifier must resolve to a delivered record,
//! that record must satisfy every premise, and the record's own delivered
//! text must carry the support the oracle asks for. A citation that cannot be
//! resolved is an error, never a pass.

use std::collections::BTreeSet;

use super::artifacts::hash;
use super::schema::{
    Answer, AnswerFact, CitationTrace, ExpectedFact, Grade, OracleCase, SourceEvidence, Step,
    SCORING_PROTOCOL,
};

/// Everything the grader needs about one answer.
pub struct GradeInput<'a> {
    pub case: &'a str,
    pub raw_answer: &'a str,
    pub oracle_case: &'a OracleCase,
    pub evidence: &'a [SourceEvidence],
    pub runtime_errors: &'a [String],
}

fn find_evidence<'a>(evidence: &'a [SourceEvidence], cited: &str) -> Option<&'a SourceEvidence> {
    evidence
        .iter()
        .find(|source| source.id == cited || source.episode_id == cited)
}

/// A citation identifier must be the bare identifier of a delivered record.
fn citation_shape(cited: &str) -> bool {
    !cited.is_empty()
        && !cited.starts_with("mem:")
        && !cited.starts_with('[')
        && !cited.ends_with(']')
        && !cited.contains('[')
        && !cited.contains(']')
        && !cited.contains(char::is_whitespace)
}

fn anchors_present(anchors: &[String], texts: &[&str]) -> (Vec<String>, Vec<String>) {
    let mut matched = Vec::new();
    let mut missing = Vec::new();
    for anchor in anchors {
        if texts.iter().any(|text| text.contains(anchor.as_str())) {
            matched.push(anchor.clone());
        } else {
            missing.push(anchor.clone());
        }
    }
    (matched, missing)
}

fn expected_matches(expected: &ExpectedFact, fact: &AnswerFact) -> bool {
    expected.fact.subject == fact.subject
        && expected.fact.predicate == fact.predicate
        && expected.fact.value == fact.value
        && expected.fact.valid_from == fact.valid_from
        && expected.fact.valid_to == fact.valid_to
}

/// Check one citation group (the sources of one fact or step).
///
/// ⭐ `record_trace` is false for the second visit of a step (the required
/// step is checked again for its support anchors) ⇒ one citation leaves
/// exactly one trace.
fn check_citations(
    cited: &[String],
    anchors: &[String],
    forbidden: &[String],
    evidence: &[SourceEvidence],
    errors: &mut Vec<String>,
    traces: &mut Vec<CitationTrace>,
    record_trace: bool,
) {
    let mut seen = BTreeSet::new();
    let mut texts: Vec<&str> = Vec::new();
    for source_id in cited {
        if !seen.insert(source_id.clone()) {
            errors.push(format!("duplicate citation: {source_id}"));
            continue;
        }
        if !citation_shape(source_id) {
            errors.push(format!("malformed citation identifier: {source_id}"));
            if record_trace {
                traces.push(CitationTrace {
                    cited_id: source_id.clone(),
                    evidence_id: String::new(),
                    episode_id: String::new(),
                    revision: -1,
                    content_sha256: String::new(),
                    eligible: false,
                    anchors_matched: Vec::new(),
                    anchors_missing: anchors.to_vec(),
                    premise_failures: vec!["malformed_citation_identifier".into()],
                });
            }
            continue;
        }
        let Some(source) = find_evidence(evidence, source_id) else {
            errors.push(format!("citation has no evidence entry: {source_id}"));
            if record_trace {
                traces.push(CitationTrace {
                    cited_id: source_id.clone(),
                    evidence_id: String::new(),
                    episode_id: String::new(),
                    revision: -1,
                    content_sha256: String::new(),
                    eligible: false,
                    anchors_matched: Vec::new(),
                    anchors_missing: anchors.to_vec(),
                    premise_failures: vec!["no_evidence_entry".into()],
                });
            }
            continue;
        };
        if !source.eligible {
            errors.push(format!(
                "citation is not usable: {source_id} ({})",
                source.failures.join(",")
            ));
        }
        if hash(&source.delivered_text) != source.delivered_sha256 {
            errors.push(format!(
                "citation digest mismatch: {source_id} (the delivered record changed after the evidence stage)"
            ));
        }
        for anchor in forbidden {
            if source.delivered_text.contains(anchor.as_str()) {
                errors.push(format!(
                    "citation carries a forbidden source: {source_id} (carries {anchor})"
                ));
            }
        }
        let (matched, missing) = anchors_present(anchors, &[source.delivered_text.as_str()]);
        texts.push(source.delivered_text.as_str());
        let premise_failures: Vec<String> = source
            .premises
            .iter()
            .filter(|premise| !premise.satisfied)
            .map(|premise| premise.kind.label().to_string())
            .collect();
        if record_trace {
            traces.push(CitationTrace {
                cited_id: source_id.clone(),
                evidence_id: source.id.clone(),
                episode_id: source.episode_id.clone(),
                revision: source.revision,
                content_sha256: source.content_sha256.clone(),
                eligible: source.eligible,
                anchors_matched: matched,
                anchors_missing: missing,
                premise_failures,
            });
        }
    }
    let (matched, missing) = anchors_present(anchors, &texts);
    if !missing.is_empty() {
        errors.push(format!(
            "cited sources do not carry the required support: {}",
            missing.join(",")
        ));
    }
    if !anchors.is_empty() && matched.is_empty() && !texts.is_empty() {
        errors.push("no cited source carries any required support".into());
    }
}

/// Grade one answer against one oracle case.
pub fn grade(input: &GradeInput<'_>) -> Grade {
    let mut errors: Vec<String> = input.runtime_errors.to_vec();
    let mut traces: Vec<CitationTrace> = Vec::new();

    for source in input.evidence {
        // ⭐ The evidence layer may not skip a premise: every delivered
        // source must carry a decision for all four premises.
        //
        // ⭐ A delivered source that is **not usable** is not a defect of the
        // answer: the memory plane deliberately delivers in-scope records that
        // must not be cited (a quoted statement, an assistant turn). The rule
        // that matters is therefore on the citations: a citation to an
        // unusable source is rejected in `check_citations`.
        if source.premises.len() != super::schema::PremiseKind::ALL.len() {
            errors.push(format!(
                "delivered source {} carries {} premise decisions, expected {}",
                source.episode_id,
                source.premises.len(),
                super::schema::PremiseKind::ALL.len()
            ));
        }
    }

    let answer: Answer = match serde_json::from_str(input.raw_answer) {
        Ok(answer) => answer,
        Err(error) => {
            errors.push(format!("answer is not valid JSON: {error}"));
            return Grade {
                protocol: SCORING_PROTOCOL.to_string(),
                case: input.case.to_string(),
                passed: false,
                errors,
                citation_traces: traces,
            };
        }
    };

    if answer.status != input.oracle_case.status {
        errors.push(format!(
            "status mismatch: answer={:?} expected={:?}",
            answer.status, input.oracle_case.status
        ));
    }
    // ⭐ Three statuses, three rules — the rule is precise, not loose:
    // * known: may assert facts and steps;
    // * ambiguous: must **not** assert a fact (the subject is not settled),
    //   but may name the source that makes it ambiguous;
    // * unknown / denied: must assert nothing at all.
    match answer.status {
        super::schema::Status::Known => {}
        super::schema::Status::Ambiguous => {
            if !answer.facts.is_empty() {
                errors.push("an ambiguous answer must not assert facts".into());
            }
        }
        _ => {
            if !answer.facts.is_empty() || !answer.steps.is_empty() {
                errors.push("an unknown or denied answer must not assert facts or steps".into());
            }
        }
    }

    // Facts: the answer must match the oracle exactly, and every fact must be
    // grounded in a citable delivered record.
    let mut used_expected: Vec<bool> = vec![false; input.oracle_case.facts.len()];
    for fact in &answer.facts {
        let Some(index) =
            input
                .oracle_case
                .facts
                .iter()
                .enumerate()
                .position(|(index, expected)| {
                    !used_expected[index] && expected_matches(expected, fact)
                })
        else {
            errors.push(format!(
                "fact is not in the oracle: {}/{}/{}",
                fact.subject, fact.predicate, fact.value
            ));
            continue;
        };
        used_expected[index] = true;
        let expected = &input.oracle_case.facts[index];
        if fact.sources.is_empty() {
            errors.push(format!(
                "fact {}/{}/{} has no citation",
                fact.subject, fact.predicate, fact.value
            ));
            continue;
        }
        let mut anchors = expected.support_anchors.clone();
        anchors.push(fact.value.clone());
        check_citations(
            &fact.sources,
            &anchors,
            &input.oracle_case.forbidden_anchors,
            input.evidence,
            &mut errors,
            &mut traces,
            true,
        );
    }
    for (index, expected) in input.oracle_case.facts.iter().enumerate() {
        if !used_expected[index] {
            errors.push(format!(
                "oracle fact is missing from the answer: {}/{}/{}",
                expected.fact.subject, expected.fact.predicate, expected.fact.value
            ));
        }
    }

    // Steps: unique identifiers, no forbidden step, existing and acyclic
    // dependencies, and required steps present with their support.
    let mut step_ids: Vec<&str> = Vec::new();
    for step in &answer.steps {
        if step_ids.contains(&step.id.as_str()) {
            errors.push(format!("duplicate step: {}", step.id));
        }
        step_ids.push(step.id.as_str());
    }
    for step in &answer.steps {
        if input.oracle_case.forbidden_steps.contains(&step.id) {
            errors.push(format!("forbidden step is present: {}", step.id));
        }
        if let Some(after) = &step.after {
            if !step_ids.contains(&after.as_str()) {
                errors.push(format!(
                    "step {} depends on an unknown step {}",
                    step.id, after
                ));
            }
        }
        if step.sources.is_empty() {
            errors.push(format!("step {} has no citation", step.id));
        } else {
            check_citations(
                &step.sources,
                &[],
                &input.oracle_case.forbidden_anchors,
                input.evidence,
                &mut errors,
                &mut traces,
                true,
            );
        }
    }
    if has_cycle(&answer.steps) {
        errors.push("step dependency graph has a cycle".into());
    }
    for required in &input.oracle_case.required_steps {
        let Some(step) = answer.steps.iter().find(|step| step.id == required.id) else {
            errors.push(format!("required step is missing: {}", required.id));
            continue;
        };
        if step.after != required.after {
            errors.push(format!(
                "required step {} has the wrong dependency: {:?} expected {:?}",
                required.id, step.after, required.after
            ));
        }
        check_citations(
            &step.sources,
            &required.support_anchors,
            &input.oracle_case.forbidden_anchors,
            input.evidence,
            &mut errors,
            &mut traces,
            false,
        );
    }

    Grade {
        protocol: SCORING_PROTOCOL.to_string(),
        case: input.case.to_string(),
        passed: errors.is_empty(),
        errors,
        citation_traces: traces,
    }
}

/// Whether the dependency graph of the steps has a cycle.
pub fn has_cycle(steps: &[Step]) -> bool {
    let mut visiting: Vec<&str> = Vec::new();
    let mut done: Vec<&str> = Vec::new();
    for step in steps {
        if visit(step, steps, &mut visiting, &mut done) {
            return true;
        }
    }
    false
}

fn visit<'a>(
    step: &'a Step,
    steps: &'a [Step],
    visiting: &mut Vec<&'a str>,
    done: &mut Vec<&'a str>,
) -> bool {
    if done.contains(&step.id.as_str()) {
        return false;
    }
    if visiting.contains(&step.id.as_str()) {
        return true;
    }
    visiting.push(step.id.as_str());
    if let Some(after) = &step.after {
        if let Some(parent) = steps.iter().find(|candidate| &candidate.id == after) {
            if visit(parent, steps, visiting, done) {
                return true;
            }
        }
    }
    visiting.pop();
    done.push(step.id.as_str());
    false
}

/// One deliberately broken answer variant.
#[derive(Clone, Debug)]
pub struct Control {
    pub name: String,
    pub answer: String,
}

/// Negative controls: every variant must be rejected by the grader. They are
/// the in-run counterpart of the reverse-falsification discipline: a grader
/// that accepts any of them is broken.
pub fn negative_controls(valid_answer: &str) -> Vec<Control> {
    let mut controls = Vec::new();
    let parsed: Answer = match serde_json::from_str(valid_answer) {
        Ok(answer) => answer,
        Err(_) => return controls,
    };

    // 1. A citation identifier that no evidence entry carries.
    let mut forged = parsed.clone();
    if let Some(fact) = forged.facts.first_mut() {
        fact.sources = vec!["forged-source-id".to_string()];
        controls.push(Control {
            name: "forged_citation_id".into(),
            answer: serde_json::to_string(&forged).unwrap_or_default(),
        });
    }

    // 2. A citation identifier that carries the rendered prefix.
    let mut prefixed = parsed.clone();
    if let Some(fact) = prefixed.facts.first_mut() {
        if let Some(source) = fact.sources.first() {
            fact.sources = vec![format!("mem:{source}")];
            controls.push(Control {
                name: "prefixed_citation_id".into(),
                answer: serde_json::to_string(&prefixed).unwrap_or_default(),
            });
        }
    }

    // 3. A wrong fact value.
    let mut wrong_value = parsed.clone();
    if let Some(fact) = wrong_value.facts.first_mut() {
        fact.value = "a value that the record does not carry".into();
        controls.push(Control {
            name: "wrong_fact_value".into(),
            answer: serde_json::to_string(&wrong_value).unwrap_or_default(),
        });
    }

    // 4. A wrong subject.
    let mut wrong_subject = parsed.clone();
    if let Some(fact) = wrong_subject.facts.first_mut() {
        fact.subject = "a subject that was never recorded".into();
        controls.push(Control {
            name: "wrong_fact_subject".into(),
            answer: serde_json::to_string(&wrong_subject).unwrap_or_default(),
        });
    }

    // 5. A dropped citation.
    let mut dropped = parsed.clone();
    if let Some(fact) = dropped.facts.first_mut() {
        fact.sources.clear();
        controls.push(Control {
            name: "dropped_citation".into(),
            answer: serde_json::to_string(&dropped).unwrap_or_default(),
        });
    }

    // 6. A status that does not match the oracle.
    //
    // ⭐ The forged status must differ from the status of the answer it was
    // derived from, otherwise the control would be a copy of a valid answer
    // (the ambiguous case would "mismatch" into itself).
    let mut wrong_status = parsed.clone();
    wrong_status.status = if wrong_status.status == super::schema::Status::Known {
        super::schema::Status::Ambiguous
    } else {
        super::schema::Status::Known
    };
    controls.push(Control {
        name: "status_mismatch".into(),
        answer: serde_json::to_string(&wrong_status).unwrap_or_default(),
    });

    // 7. A duplicated fact.
    let mut duplicated = parsed.clone();
    if let Some(fact) = duplicated.facts.first().cloned() {
        duplicated.facts.push(fact);
        controls.push(Control {
            name: "duplicated_fact".into(),
            answer: serde_json::to_string(&duplicated).unwrap_or_default(),
        });
    }

    // 8. A dependency cycle.
    let mut cyclic = parsed.clone();
    cyclic.steps = vec![
        Step {
            id: "s1".into(),
            after: Some("s2".into()),
            sources: Vec::new(),
        },
        Step {
            id: "s2".into(),
            after: Some("s1".into()),
            sources: Vec::new(),
        },
    ];
    controls.push(Control {
        name: "cyclic_steps".into(),
        answer: serde_json::to_string(&cyclic).unwrap_or_default(),
    });

    controls
}
