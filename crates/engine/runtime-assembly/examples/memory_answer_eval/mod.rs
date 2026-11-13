mod audit;
mod capture;
mod config;
mod fixtures;
mod io;
mod lifecycle;
mod offline;
mod protocol;
mod runner;
mod scoring;
mod source_proof;
mod wire;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use capture::{CallBudget, CallRecord, Redactor};
use config::Config;
use fixtures::{scenarios, Scenario};
use io::{hash, read, write_new, EvalResult};
use runner::Environment;
use scoring::{score, Grade, SCORING_PROTOCOL};

#[derive(Clone, Deserialize, Serialize)]
pub struct Plan {
    pub schema_version: u32,
    #[serde(default)]
    pub scoring_protocol: String,
    pub mode: String,
    pub case_root: std::path::PathBuf,
    pub config: Config,
    pub scenarios: Vec<Scenario>,
    pub fixture_sha256: String,
    pub system_sha256: String,
    pub seed_system_sha256: String,
    pub proposal_system_sha256: String,
    #[serde(default)]
    pub proposal_protocol: String,
    pub source_hashes: BTreeMap<String, String>,
    pub git_head: Option<String>,
    pub git_tree: Option<String>,
    pub dirty_diff_sha256: Option<String>,
    pub planned_slots: usize,
    pub wired_slots: usize,
    pub unwired_slots: usize,
    pub wired_primary_turns: usize,
    pub total_delegate_call_cap: usize,
    pub output_token_cap: u64,
    pub selected_slots: usize,
    pub selected_primary_turns: usize,
    pub selected_model_call_upper_bound: usize,
    pub notes: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct CaseResult {
    pub scenario: String,
    pub repetition: usize,
    pub status: String,
    pub reason: Option<String>,
    pub query_call_index: Option<usize>,
    pub grade: Option<Grade>,
}

#[derive(Serialize)]
pub struct RunReport {
    pub scoring_protocol: String,
    pub mode: String,
    pub complete: bool,
    pub live_quality_passed: bool,
    pub selected_quality_passed: bool,
    pub offline_contract_passed: bool,
    pub counts: BTreeMap<String, usize>,
    pub cases: Vec<CaseResult>,
}

fn source_hashes() -> BTreeMap<String, String> {
    [
        (
            "entry",
            include_bytes!("../memory_answer_eval.rs").as_slice(),
        ),
        ("runner", include_bytes!("runner.rs").as_slice()),
        ("capture", include_bytes!("capture.rs").as_slice()),
        ("scoring", include_bytes!("scoring.rs").as_slice()),
        ("fixtures", include_bytes!("fixtures.rs").as_slice()),
        ("config", include_bytes!("config.rs").as_slice()),
        ("io", include_bytes!("io.rs").as_slice()),
        ("manifest", include_bytes!("mod.rs").as_slice()),
        ("source_audit", include_bytes!("audit.rs").as_slice()),
        ("launcher", include_bytes!("run-live.ps1").as_slice()),
        ("call_protocol", include_bytes!("protocol.rs").as_slice()),
        ("offline_protocol", include_bytes!("offline.rs").as_slice()),
        ("lifecycle", include_bytes!("lifecycle.rs").as_slice()),
        ("source_proof", include_bytes!("source_proof.rs").as_slice()),
        ("http_wire", include_bytes!("wire.rs").as_slice()),
    ]
    .into_iter()
    .map(|(name, bytes)| (name.into(), hash(bytes)))
    .collect()
}

fn plan(config: Config, mode: &str) -> EvalResult<Plan> {
    let scenarios = scenarios();
    let wired = scenarios.iter().filter(|s| s.live_wired).count();
    Ok(Plan {
        schema_version: 2,
        scoring_protocol: SCORING_PROTOCOL.into(),
        mode: mode.into(),
        case_root:config.output.clone(),
        fixture_sha256: hash(serde_json::to_vec(&scenarios)?),
        system_sha256: hash(fixtures::SYSTEM),
        seed_system_sha256: hash(fixtures::SEED_SYSTEM),
        proposal_system_sha256: hash(apeireth_runtime_assembly::canonical::memory_correction::CORRECTION_PROPOSAL_SYSTEM),
        proposal_protocol: apeireth_runtime_assembly::canonical::memory_correction::CORRECTION_PROPOSAL_PROTOCOL.into(),
        source_hashes: source_hashes(),
        git_head: io::git_output("HEAD"),
        git_tree: io::git_output("HEAD^{tree}"),
        dirty_diff_sha256: io::dirty_diff_hash(),
        planned_slots: scenarios.len()*config.repetitions,
        wired_slots: wired*config.repetitions,
        unwired_slots: (scenarios.len()-wired)*config.repetitions,
        wired_primary_turns: scenarios.iter().filter(|s|s.live_wired).map(|s|s.input.primary_turns()).sum::<usize>()*config.repetitions,
        total_delegate_call_cap: config.max_calls,
        output_token_cap: (config.max_calls as u64).saturating_mul(u64::from(config.max_output_tokens)),
        selected_slots:scenarios.iter().filter(|s|config.selected(&s.id)).count()*config.repetitions,
        selected_primary_turns:scenarios.iter().filter(|s|s.live_wired && config.selected(&s.id)).map(|s|s.input.primary_turns()).sum::<usize>()*config.repetitions,
        selected_model_call_upper_bound:scenarios.iter().filter(|s|s.live_wired && config.selected(&s.id)).map(|s|s.input.primary_turns()).sum::<usize>()*config.repetitions*2,
        config, scenarios,
        notes: vec![
            format!("{wired} lifecycle routes are connected in source; real-model quality is unmeasured until run-live completes."),
            "One delegate cap includes setup, main, Q5 isolated proposals, unknown and failed invocations. Protocol classification uses server-owned system contracts.".into(),
            "Proposal protocol v3 retains the Host-owned committed input UTC time and policy UTC offset. It adds finite calendar equivalence for one supported absolute date inside the validated original evidence; relative expressions still require exact source text. Prior proposal protocols require their original executable; scores across these mechanisms are not a same-condition comparison.".into(),
            "Every repetition is retained. No best-of selection; blocked/missing/error/invalid_setup/unwired slots remain in the denominator.".into(),
            "The request byte bound is on normalized JSON, not a provider tokenizer or currency guarantee. Missing usage is unknown.".into(),
            "Prices and model versions are caller supplied. No paid endpoint/model is selected implicitly; top_p and seed are absent because the canonical request has no such fields.".into(),
            "Offline contract mode never measures real-model answer quality.".into(),
            "Scoring v2 keeps each fact's original gold anchor and admits only independently SQL-verified direct correction-evidence support for extra citations. Old runs require their original executable; this protocol never silently regrades them.".into(),
        ],
    })
}

fn summarize(mode: &str, cases: Vec<CaseResult>) -> RunReport {
    let mut counts = BTreeMap::new();
    for row in &cases {
        *counts.entry(row.status.clone()).or_insert(0) += 1;
    }
    let complete = cases
        .iter()
        .all(|r| r.status == "passed" || r.status == "hard_failure");
    RunReport {
        scoring_protocol: SCORING_PROTOCOL.into(),
        mode: mode.into(),
        complete,
        live_quality_passed: mode == "live"
            && complete
            && cases.iter().all(|r| r.status == "passed"),
        selected_quality_passed: mode == "live"
            && cases.iter().any(|r| r.status == "passed")
            && cases
                .iter()
                .filter(|r| r.status != "not_selected")
                .all(|r| r.status == "passed"),
        offline_contract_passed: mode == "offline_contract"
            && cases.iter().any(|r| r.status == "delivery_contract_passed")
            && cases.iter().all(|r| {
                r.status == "delivery_contract_passed"
                    || r.status == "unwired"
                    || r.status == "not_selected"
            }),
        counts,
        cases,
    }
}

fn offline_receipt_valid(record: &CallRecord, scenario: &Scenario) -> bool {
    record.request_evidence.closed_tool_context_valid
        && record.request_evidence.issues.is_empty()
        && (if matches!(
            scenario.input.lifecycle,
            fixtures::Lifecycle::ForeignSession | fixtures::Lifecycle::ForgetAll
        ) {
            record.request_evidence.sources.is_empty()
        } else if scenario.oracle.status == scoring::AnswerStatus::Unknown {
            true
        } else {
            !record.request_evidence.sources.is_empty()
        })
        && record
            .request_evidence
            .sources
            .iter()
            .all(|s| s.active && s.authorized && s.runtime_source_verified && s.projection_verified)
        && scenario.oracle.facts.iter().all(|fact| {
            record
                .request_evidence
                .sources
                .iter()
                .any(|source| source.supports(fact))
        })
}

async fn execute(config: Config, live: bool) -> EvalResult<bool> {
    io::create_run(&config.output)?;
    let mode = if live { "live" } else { "offline_contract" };
    let manifest = plan(config.clone(), mode)?;
    if config.max_calls < manifest.selected_model_call_upper_bound {
        return Err(format!(
            "call cap {} is below frozen selected bound {} including isolated proposals",
            config.max_calls, manifest.selected_model_call_upper_bound
        )
        .into());
    }
    write_new(&config.output.join("manifest.json"), &manifest)?;
    let environment = Environment {
        config: config.clone(),
        live,
        budget: Arc::new(Mutex::new(CallBudget::default())),
        redactor: Arc::new(Redactor::default()),
    };
    let mut cases = Vec::new();
    for repetition in 0..config.repetitions {
        for scenario in &manifest.scenarios {
            let mut row = CaseResult {
                scenario: scenario.id.clone(),
                repetition,
                status: "unwired".into(),
                reason: scenario.unavailable_reason.clone(),
                query_call_index: None,
                grade: None,
            };
            if !config.selected(&scenario.id) {
                row.status = "not_selected".into();
                row.reason = Some("explicit caller subset; not measured or passed".into());
            } else if scenario.live_wired {
                match environment
                    .execute_case(&scenario.id, repetition, &scenario.input)
                    .await
                {
                    Ok(record) => {
                        row.query_call_index = Some(record.index);
                        if live {
                            let raw = record
                                .response
                                .as_ref()
                                .map(|r| r.content.as_str())
                                .unwrap_or("");
                            let grade = score(raw, &scenario.oracle, &record.request_evidence);
                            row.status = if grade.passed {
                                "passed"
                            } else {
                                "hard_failure"
                            }
                            .into();
                            row.grade = Some(grade);
                        } else {
                            row.status = if offline_receipt_valid(&record, scenario) {
                                "delivery_contract_passed"
                            } else {
                                "delivery_contract_failed"
                            }
                            .into();
                        }
                        row.reason = None;
                    }
                    Err(error) => {
                        row.status = if error.downcast_ref::<runner::InvalidSetup>().is_some() {
                            "invalid_setup"
                        } else {
                            "runtime_error"
                        }
                        .into();
                        row.reason = Some(environment.redactor.text(&error.to_string()));
                    }
                }
            }
            write_new(
                &config
                    .output
                    .join(format!("case-{}-{repetition:03}.json", scenario.id)),
                &row,
            )?;
            cases.push(row);
        }
    }
    let report = summarize(mode, cases);
    write_new(&config.output.join("report.json"), &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(if live {
        if config.cases.is_empty() {
            report.live_quality_passed
        } else {
            report.selected_quality_passed
        }
    } else {
        report.offline_contract_passed
    })
}

fn rescore(path: &Path) -> EvalResult<bool> {
    let manifest: Plan = read(&path.join("manifest.json"))?;
    if manifest.schema_version != 2
        || manifest.scoring_protocol != SCORING_PROTOCOL
        || manifest.proposal_protocol
            != apeireth_runtime_assembly::canonical::memory_correction::CORRECTION_PROPOSAL_PROTOCOL
        || manifest.proposal_system_sha256
            != hash(
                apeireth_runtime_assembly::canonical::memory_correction::CORRECTION_PROPOSAL_SYSTEM,
            )
        || manifest.fixture_sha256 != hash(serde_json::to_vec(&scenarios())?)
        || manifest.source_hashes != source_hashes()
    {
        return Err(
            "scoring/proposal protocol, system, fixture or evaluator source differs from the run manifest; use the run's original executable; refusing silent regrading"
                .into(),
        );
    }
    let mut cases = Vec::new();
    for repetition in 0..manifest.config.repetitions {
        for scenario in &manifest.scenarios {
            let case_path = path.join(format!("case-{}-{repetition:03}.json", scenario.id));
            let mut row = match read::<CaseResult>(&case_path) {
                Ok(row) => row,
                Err(error) => {
                    cases.push(CaseResult {
                        scenario: scenario.id.clone(),
                        repetition,
                        status: "missing_or_invalid_evidence".into(),
                        reason: Some(error.to_string()),
                        query_call_index: None,
                        grade: None,
                    });
                    continue;
                }
            };
            if row.scenario != scenario.id || row.repetition != repetition {
                return Err("case identity mismatch".into());
            }
            if row
                .grade
                .as_ref()
                .is_some_and(|grade| grade.scoring_protocol != SCORING_PROTOCOL)
            {
                return Err(
                    "saved grade uses another scoring protocol; use the run's original executable"
                        .into(),
                );
            }
            if !manifest.config.selected(&scenario.id) {
                row.status = "not_selected".into();
                row.grade = None;
                row.query_call_index = None;
                row.reason = Some("explicit caller subset; not measured or passed".into());
            } else if !scenario.live_wired {
                row.status = "unwired".into();
                row.reason = scenario.unavailable_reason.clone();
                row.grade = None;
                row.query_call_index = None;
            } else if let Some(index) = row.query_call_index {
                let mut record: CallRecord = read(
                    &path
                        .join("calls")
                        .join(format!("call-{index:06}-result.json")),
                )?;
                if record.context.scenario != scenario.id
                    || record.context.repetition != repetition
                    || record.context.phase != "query"
                    || record.context.kind != "main"
                    || hash(serde_json::to_vec(&record.original_request)?)
                        != record.original_request_sha256
                    || hash(serde_json::to_vec(&record.forwarded_request)?)
                        != record.forwarded_request_sha256
                {
                    return Err("request identity/hash mismatch".into());
                }
                if let Some(response) = &record.response {
                    if record.response_sha256.as_deref()
                        != Some(hash(serde_json::to_vec(response)?).as_str())
                    {
                        return Err("raw answer hash mismatch".into());
                    }
                }
                let proof: lifecycle::LifecycleProof =
                    read(&path.join(format!("lifecycle-{}-{repetition:03}.json", scenario.id)))?;
                record.request_evidence.issues.extend(proof.issues);
                if manifest.mode == "live" {
                    let grade = score(
                        record
                            .response
                            .as_ref()
                            .map(|r| r.content.as_str())
                            .unwrap_or(""),
                        &scenario.oracle,
                        &record.request_evidence,
                    );
                    row.status = if grade.passed {
                        "passed"
                    } else {
                        "hard_failure"
                    }
                    .into();
                    row.grade = Some(grade);
                } else {
                    row.status = if offline_receipt_valid(&record, scenario) {
                        "delivery_contract_passed"
                    } else {
                        "delivery_contract_failed"
                    }
                    .into();
                    row.grade = None;
                }
            }
            cases.push(row);
        }
    }
    let report = summarize(&manifest.mode, cases);
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(if manifest.mode == "live" {
        if manifest.config.cases.is_empty() {
            report.live_quality_passed
        } else {
            report.selected_quality_passed
        }
    } else {
        report.offline_contract_passed
    })
}

fn scorer_contract_check() -> EvalResult<()> {
    let mut scenario = scenarios().remove(0);
    scenario.oracle.facts.truncate(1);
    let source = scoring::SourceEvidence {
        scoring_protocol: SCORING_PROTOCOL.into(),
        id: "typed:episode:contract".into(),
        episode_id: "contract".into(),
        revision: 0,
        content_sha256: hash(fixtures::BASIC_SEED),
        delivered_content_sha256: hash(fixtures::BASIC_SEED),
        expected_projection_sha256: hash(fixtures::BASIC_SEED),
        delivered_text: fixtures::BASIC_SEED.into(),
        projection_verified: true,
        source_role: "user".into(),
        claim_kind: "direct_user_assertion".into(),
        citable_as_fact: true,
        active: true,
        authorized: true,
        runtime_source_verified: true,
        scope: serde_json::json!({}),
        provenance: serde_json::json!({}),
        correction_support: vec![],
    };
    let evidence = scoring::RequestEvidence {
        scoring_protocol: SCORING_PROTOCOL.into(),
        closed_tool_context_valid: true,
        sources: vec![source],
        issues: vec![],
    };
    let answer = r#"{"status":"known","facts":[{"subject":"林禾","predicate":"居住城市","value":"北岚市","valid_from":null,"valid_to":null,"sources":["typed:episode:contract"]}]}"#;
    if !score(answer, &scenario.oracle, &evidence).passed {
        return Err("scorer valid contract rejected".into());
    }
    let mut variants = vec![
        answer.replace("北岚市", "不是北岚市"),
        answer.replace("林禾", "周青"),
        answer.replace("居住城市", "居住地"),
        answer.replace("typed:episode:contract", "forged"),
        answer.replace("typed:episode:contract", "mem:typed:episode:contract"),
        format!("{answer} extra"),
    ];
    variants.push(answer.replacen("\"status\":", "\"status\":\"unknown\",\"status\":", 1));
    if variants
        .iter()
        .any(|raw| score(raw, &scenario.oracle, &evidence).passed)
    {
        return Err("scorer falsification accepted invalid answer".into());
    }
    let mut assistant = evidence.clone();
    assistant.sources[0].citable_as_fact = false;
    assistant.sources[0].source_role = "assistant".into();
    let mut bad_projection = evidence.clone();
    bad_projection.sources[0].expected_projection_sha256 = "unmatched-projection".into();
    if [assistant, bad_projection]
        .iter()
        .any(|altered| score(answer, &scenario.oracle, altered).passed)
    {
        return Err("source qualification or projection falsification accepted".into());
    }
    let mut removed = evidence;
    removed.sources.clear();
    if score(answer, &scenario.oracle, &removed).passed {
        return Err("capture falsification accepted removed recall".into());
    }
    Ok(())
}

pub async fn run(args: Vec<String>) -> EvalResult<bool> {
    let Some((command, options)) = args.split_first() else {
        return Err("commands: plan [options]; offline-check --output NEW_DIR; run-live --provider compatible --endpoint URL --model ID --key-env ENV_NAME --temperature N --max-output-tokens N --max-calls N --repetitions N --output NEW_DIR; score RUN_DIR".into());
    };
    match command.as_str() {
        "plan" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&plan(Config::parse(options, false)?, "plan")?)?
            );
            Ok(true)
        }
        "offline-check" => {
            scorer_contract_check()?;
            let config = Config::parse(options, false)?;
            if config.output.as_os_str().is_empty() {
                return Err("offline-check requires --output NEW_DIR".into());
            }
            execute(config, false).await
        }
        "run-live" => execute(Config::parse(options, true)?, true).await,
        "score" if options.len() == 1 => rescore(Path::new(&options[0])),
        _ => Err("unknown command or invalid score arguments".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prior_scoring_manifest_requires_its_original_executable() {
        let temporary = tempfile::tempdir().unwrap();
        let mut old = serde_json::to_value(
            plan(Config::offline(temporary.path().into()), "offline_contract").unwrap(),
        )
        .unwrap();
        old["schema_version"] = serde_json::json!(1);
        old.as_object_mut().unwrap().remove("scoring_protocol");
        write_new(&temporary.path().join("manifest.json"), &old).unwrap();
        let error = rescore(temporary.path()).unwrap_err().to_string();
        assert!(error.contains("original executable"));
        assert!(
            !temporary.path().join("report.json").exists(),
            "no old report is overwritten"
        );
    }

    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "read-only byte comparison of isolated temporary evaluation reports"
    )]
    fn prior_proposal_protocol_cannot_be_regraded_with_current_hashes() {
        for previous in [
            Some("apeireth.memory-correction.v1"),
            Some("apeireth.memory-correction.v2"),
            Some(""),
            None,
        ] {
            let temporary = tempfile::tempdir().unwrap();
            let mut manifest = serde_json::to_value(
                plan(Config::offline(temporary.path().into()), "offline_contract").unwrap(),
            )
            .unwrap();
            if let Some(protocol) = previous {
                manifest["proposal_protocol"] = serde_json::json!(protocol);
            } else {
                manifest
                    .as_object_mut()
                    .unwrap()
                    .remove("proposal_protocol");
            }
            // Every digest and the scoring version remain current. The
            // proposal version must reject this before reading any case.
            let retained = serde_json::json!({"retained_prior_report":true});
            write_new(&temporary.path().join("manifest.json"), &manifest).unwrap();
            write_new(&temporary.path().join("report.json"), &retained).unwrap();
            let before = std::fs::read(temporary.path().join("report.json")).unwrap();
            let error = rescore(temporary.path()).unwrap_err().to_string();
            assert!(
                error.contains("original executable"),
                "{previous:?}: {error}"
            );
            assert_eq!(
                std::fs::read(temporary.path().join("report.json")).unwrap(),
                before,
                "an old report remains byte-identical"
            );
            assert!(!temporary.path().join("case-e01-000.json").exists());
        }
    }

    #[test]
    fn mismatched_proposal_system_or_evaluator_digest_cannot_regrade_a_run() {
        for field in ["proposal_system_sha256", "source_hashes"] {
            let temporary = tempfile::tempdir().unwrap();
            let mut manifest = serde_json::to_value(
                plan(Config::offline(temporary.path().into()), "offline_contract").unwrap(),
            )
            .unwrap();
            if field == "proposal_system_sha256" {
                manifest[field] = serde_json::json!(hash("retained previous proposal system"));
            } else {
                manifest[field]["call_protocol"] =
                    serde_json::json!(hash("retained previous evaluator source"));
            }
            write_new(&temporary.path().join("manifest.json"), &manifest).unwrap();
            let error = rescore(temporary.path()).unwrap_err().to_string();
            assert!(error.contains("original executable"), "{field}: {error}");
            assert!(!temporary.path().join("report.json").exists());
        }
    }
    #[test]
    fn offline_falsification_rejects_answer_and_capture_mutations() {
        scorer_contract_check().unwrap();
    }

    #[test]
    fn plan_keeps_unwired_slots_and_total_call_cost_bound() {
        let p = plan(Config::offline("not-created".into()), "plan").unwrap();
        assert_eq!(p.schema_version, 2);
        assert_eq!(p.scoring_protocol, "apeireth.memory-answer-scoring.v2");
        assert_eq!(p.proposal_protocol, "apeireth.memory-correction.v3");
        assert_eq!(p.planned_slots, 12);
        assert_eq!(p.wired_slots, 12);
        assert_eq!(p.unwired_slots, 0);
        assert_eq!(p.wired_primary_turns, 39);
        assert_eq!(p.output_token_cap, 131072);
    }

    #[test]
    fn an_unwired_slot_cannot_become_a_quality_pass() {
        let report = summarize(
            "live",
            vec![CaseResult {
                scenario: "e11".into(),
                repetition: 0,
                status: "unwired".into(),
                reason: Some("pending".into()),
                query_call_index: None,
                grade: None,
            }],
        );
        assert!(!report.live_quality_passed);
        assert!(!report.complete);
    }

    #[test]
    fn oracle_mutations_do_not_change_the_input_serialization() {
        let mut scenario = scenarios().remove(0);
        let before = serde_json::to_vec(&scenario.input).unwrap();
        scenario.oracle.facts[0].fact.value = "ORACLE_ONLY_CANARY".into();
        assert_eq!(before, serde_json::to_vec(&scenario.input).unwrap());
        assert!(!String::from_utf8(before)
            .unwrap()
            .contains("ORACLE_ONLY_CANARY"));
    }

    #[tokio::test]
    async fn offline_contract_runs_sqlite_restart_recall_and_can_be_rescored() {
        let temporary = tempfile::tempdir().unwrap();
        let output = temporary.path().join("contract-run");
        let attempted = execute(Config::offline(output.clone()), false).await;
        if !matches!(&attempted, Ok(true)) {
            let retained = temporary.keep();
            panic!(
                "offline contract failed: {attempted:?}; retained artifacts: {}",
                retained.display()
            );
        }
        assert!(rescore(&output).unwrap());
        let manifest: Plan = read(&output.join("manifest.json")).unwrap();
        assert_eq!(manifest.mode, "offline_contract");
        let row: CaseResult = read(&output.join("case-e01-000.json")).unwrap();
        assert_eq!(row.status, "delivery_contract_passed");
        assert!(row.grade.is_none());
        let call: CallRecord = read(&output.join("calls").join(format!(
            "call-{:06}-result.json",
            row.query_call_index.unwrap()
        )))
        .unwrap();
        assert!(!call.request_evidence.sources.is_empty());
        assert!(call
            .request_evidence
            .sources
            .iter()
            .any(|s| s.content_sha256 == hash(fixtures::BASIC_SEED)
                && s.delivered_content_sha256 == s.content_sha256));
    }
}
