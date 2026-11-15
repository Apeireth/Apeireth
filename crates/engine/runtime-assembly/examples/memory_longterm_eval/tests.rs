//! The reverse-falsification vehicle (§5.3): every assertion here must be
//! able to go red when the wiring it checks is removed.
//!
//! ⭐ 口径：这些测试只跑在**离线夹具**上（本轮未调用真实模型）；题集是**回归集**，
//! 不是独立盲测。

use super::config::Config;
use super::corpus;
use super::evidence::{self, Inspect};
use super::grading::{self, GradeInput};
use super::harness::Harness;
use super::live;
use super::schema;

use tempfile::TempDir;

use apeireth_memory::MemoryGovernanceStore;

/// Build a store with the corpus written through the production writeback
/// path, and hand back everything a test needs to decide a premise.
struct Fixture {
    _dir: TempDir,
    harness: Harness,
    input: schema::InputBundle,
    oracle: schema::OracleBundle,
}

async fn fixture() -> Fixture {
    let dir = TempDir::new().expect("temp dir");
    let input = corpus::input_bundle();
    schema::validate_input(&input).expect("the input bundle is consistent");
    let oracle = corpus::oracle_bundle(&input);
    schema::validate_oracle(&oracle, &input).expect("the oracle bundle is consistent");
    let db = dir.path().join("memory.sqlite3");
    let harness = Harness::new(&db, Config::offline(), &input).expect("the harness builds");
    harness
        .write_corpus(&input)
        .await
        .expect("the corpus is written");
    Fixture {
        _dir: dir,
        harness,
        input,
        oracle,
    }
}

impl Fixture {
    fn oracle_case(&self, id: &str) -> schema::OracleCase {
        self.oracle
            .cases
            .iter()
            .find(|case| case.id == id)
            .cloned()
            .unwrap_or_else(|| panic!("the oracle case {id} is missing"))
    }

    async fn ask(&self, id: &str) -> (String, String) {
        let query = self
            .input
            .queries
            .iter()
            .find(|query| query.id == id)
            .unwrap_or_else(|| panic!("the query {id} is missing"));
        let response = self
            .harness
            .ask(&query.session, &query.text)
            .await
            .expect("the question is answered");
        let request = self.harness.calls.last().expect("the provider was called");
        (
            response.text,
            super::artifacts::hash(&serde_json::to_string(&request).unwrap()),
        )
    }

    async fn receipt(&self, id: &str) -> schema::EvidenceReceipt {
        let query = self
            .input
            .queries
            .iter()
            .find(|query| query.id == id)
            .unwrap_or_else(|| panic!("the query {id} is missing"));
        let _response = self
            .harness
            .ask(&query.session, &query.text)
            .await
            .expect("the question is answered");
        let request = self.harness.calls.last().expect("the provider was called");
        let partition = self
            .harness
            .partition(&query.session)
            .expect("the partition");
        evidence::inspect(&Inspect {
            request: &request,
            db: &self.harness.db,
            partition: &partition,
            policy: &self.harness.policy,
            audit: &self.harness.audit,
            canary: corpus::ORACLE_CANARY,
            case: &query.id,
        })
        .expect("the evidence pass runs")
    }

    fn satisfied(&self, receipt: &schema::EvidenceReceipt, label: &str) -> usize {
        receipt
            .premise_summary
            .iter()
            .find(|summary| summary.label == label)
            .map(|summary| summary.satisfied_count)
            .unwrap_or(0)
    }
}

/// The premise layer decides four independent questions, and every premise is
/// decided for the delivered sources.
#[tokio::test]
async fn the_four_premises_are_decided_for_the_delivered_sources() {
    let fixture = fixture().await;
    let receipt = fixture.receipt("c01_long_range_direct").await;
    assert!(
        !receipt.sources.is_empty(),
        "the cold query must deliver at least one source"
    );
    for label in [
        "direct_user_evidence",
        "host_authorization",
        "current_scope",
        "revision_cas",
    ] {
        let summary = receipt
            .premise_summary
            .iter()
            .find(|summary| summary.label == label)
            .unwrap_or_else(|| panic!("the premise {label} is missing"));
        assert!(
            summary.decidable,
            "the premise {label} must be decidable for every delivered source"
        );
    }
    assert!(
        fixture.satisfied(&receipt, "direct_user_evidence") > 0,
        "the delivered record must trace to a committed direct user input"
    );
    assert!(
        fixture.satisfied(&receipt, "current_scope") > 0,
        "the delivered record must sit inside the querying scope"
    );
    assert!(
        fixture.satisfied(&receipt, "revision_cas") > 0,
        "the delivered snapshot must match the current governed revision"
    );
}

/// The Host authorization premise follows the Host policy.
#[tokio::test]
async fn the_authorization_premise_follows_the_host_policy() {
    let fixture = fixture().await;
    let query = fixture
        .input
        .queries
        .iter()
        .find(|query| query.id == "c01_long_range_direct")
        .expect("the c01 query exists")
        .clone();
    fixture
        .harness
        .ask(&query.session, &query.text)
        .await
        .expect("the question is answered");
    let request = fixture
        .harness
        .calls
        .last()
        .expect("the provider was called");
    let partition = fixture
        .harness
        .partition(&query.session)
        .expect("the partition");

    let granted = evidence::inspect(&Inspect {
        request: &request,
        db: &fixture.harness.db,
        partition: &partition,
        policy: &fixture.harness.policy,
        audit: &fixture.harness.audit,
        canary: corpus::ORACLE_CANARY,
        case: "probe",
    })
    .expect("the evidence pass runs");
    assert!(
        fixture.satisfied(&granted, "host_authorization") > 0,
        "the granted capability must satisfy the authorization premise"
    );

    let mut denying = fixture.harness.policy.clone();
    denying.revoke(&apeireth_governance::Permission::ExecuteTool(
        evidence::RECALL_CAPABILITY.to_string(),
    ));
    let denied = evidence::inspect(&Inspect {
        request: &request,
        db: &fixture.harness.db,
        partition: &partition,
        policy: &denying,
        audit: &fixture.harness.audit,
        canary: corpus::ORACLE_CANARY,
        case: "probe",
    })
    .expect("the evidence pass runs");
    assert_eq!(
        fixture.satisfied(&denied, "host_authorization"),
        0,
        "revoking the capability must falsify the authorization premise"
    );
}

/// The current-scope premise follows the querying partition.
#[tokio::test]
async fn the_scope_premise_follows_the_partition() {
    let fixture = fixture().await;
    let query = fixture
        .input
        .queries
        .iter()
        .find(|query| query.id == "c01_long_range_direct")
        .expect("the c01 query exists")
        .clone();
    fixture
        .harness
        .ask(&query.session, &query.text)
        .await
        .expect("the question is answered");
    let request = fixture
        .harness
        .calls
        .last()
        .expect("the provider was called");
    let partition = fixture
        .harness
        .partition(&query.session)
        .expect("the partition");
    let foreign = "00000000-0000-4000-8000-000000000000".to_string();

    let own = evidence::inspect(&Inspect {
        request: &request,
        db: &fixture.harness.db,
        partition: &partition,
        policy: &fixture.harness.policy,
        audit: &fixture.harness.audit,
        canary: corpus::ORACLE_CANARY,
        case: "probe",
    })
    .expect("the evidence pass runs");
    assert!(
        fixture.satisfied(&own, "current_scope") > 0,
        "the owning partition must satisfy the scope premise"
    );

    let outside = evidence::inspect(&Inspect {
        request: &request,
        db: &fixture.harness.db,
        partition: &foreign,
        policy: &fixture.harness.policy,
        audit: &fixture.harness.audit,
        canary: corpus::ORACLE_CANARY,
        case: "probe",
    })
    .expect("the evidence pass runs");
    assert_eq!(
        fixture.satisfied(&outside, "current_scope"),
        0,
        "a foreign partition must falsify the scope premise"
    );
}

/// Every citation in an answer must trace to a concrete evidence entry.
#[tokio::test]
async fn every_citation_traces_to_an_evidence_entry() {
    let fixture = fixture().await;
    for query in &fixture.input.queries {
        let (answer, _) = fixture.ask(&query.id).await;
        let receipt = fixture.receipt(&query.id).await;
        let grade = grading::grade(&GradeInput {
            case: &query.id,
            raw_answer: &answer,
            oracle_case: &fixture.oracle_case(&query.id),
            evidence: &receipt.sources,
            runtime_errors: &[],
        });
        assert!(
            grade.passed,
            "the case {} must pass: {:?}",
            query.id, grade.errors
        );
        let parsed: schema::Answer = serde_json::from_str(&answer)
            .unwrap_or_else(|error| panic!("the answer parses: {error}"));
        for fact in &parsed.facts {
            for source in &fact.sources {
                assert!(
                    receipt
                        .sources
                        .iter()
                        .any(|entry| entry.id == *source && entry.eligible),
                    "the citation {source} must resolve to an eligible evidence entry"
                );
            }
        }
        assert_eq!(
            grade.citation_traces.len(),
            parsed
                .facts
                .iter()
                .map(|fact| fact.sources.len())
                .sum::<usize>()
                + parsed
                    .steps
                    .iter()
                    .map(|step| step.sources.len())
                    .sum::<usize>(),
            "every citation (fact and step) must leave a trace"
        );
    }
}

/// The grader must reject every forgery.
#[tokio::test]
async fn the_grader_rejects_every_negative_control() {
    let fixture = fixture().await;
    for query in &fixture.input.queries {
        let (answer, _) = fixture.ask(&query.id).await;
        let receipt = fixture.receipt(&query.id).await;
        let oracle_case = fixture.oracle_case(&query.id);
        for control in grading::negative_controls(&answer) {
            let grade = grading::grade(&GradeInput {
                case: &query.id,
                raw_answer: &control.answer,
                oracle_case: &oracle_case,
                evidence: &receipt.sources,
                runtime_errors: &[],
            });
            assert!(
                !grade.passed,
                "the control {} of the case {} must be rejected",
                control.name, query.id
            );
        }
    }
}

/// The oracle canary never reaches a model-facing request or the store.
#[tokio::test]
async fn the_oracle_canary_never_reaches_a_request_or_the_store() {
    let fixture = fixture().await;
    for query in &fixture.input.queries {
        fixture
            .harness
            .ask(&query.session, &query.text)
            .await
            .expect("the question is answered");
        let request = fixture
            .harness
            .calls
            .last()
            .expect("the provider was called");
        let rendered = serde_json::to_string(&request).expect("the request renders");
        assert!(
            !rendered.contains(corpus::ORACLE_CANARY),
            "the oracle canary must not appear in the request of {}",
            query.id
        );
    }
    assert!(
        !evidence::canary_in_store(&fixture.harness.db, corpus::ORACLE_CANARY)
            .expect("the store is readable"),
        "the oracle canary must not be stored as a memory record"
    );
}

/// A governed revision bump falsifies the revision premise, and a stale
/// revision write is rejected by the compare-and-set.
///
/// ⭐ The **same** request is inspected before and after the bump: a request
/// carries the delivered snapshot it received, so asking again would deliver
/// the new content and prove nothing.
#[tokio::test]
async fn the_revision_premise_follows_the_governed_revision() {
    let fixture = fixture().await;
    let query = fixture
        .input
        .queries
        .iter()
        .find(|query| query.id == "c01_long_range_direct")
        .expect("the c01 query exists")
        .clone();
    fixture
        .harness
        .ask(&query.session, &query.text)
        .await
        .expect("the question is answered");
    let request = fixture
        .harness
        .calls
        .last()
        .expect("the provider was called");
    let partition = fixture
        .harness
        .partition(&query.session)
        .expect("the partition");
    let before = evidence::inspect(&Inspect {
        request: &request,
        db: &fixture.harness.db,
        partition: &partition,
        policy: &fixture.harness.policy,
        audit: &fixture.harness.audit,
        canary: corpus::ORACLE_CANARY,
        case: "c01_long_range_direct",
    })
    .expect("the evidence pass runs");
    let source = before
        .sources
        .iter()
        .find(|source| source.eligible)
        .expect("an eligible source is delivered")
        .clone();
    let store =
        apeireth_memory::SqliteMemoryStore::open(&fixture.harness.db).expect("the store opens");
    let governed = store
        .get_governed(&source.episode_id)
        .expect("the governed row reads")
        .expect("the governed row exists");
    let updated = store
        .update_episode_content(
            &source.episode_id,
            "我住在北岚市。（受管修订）",
            Some("evaluation"),
            governed.revision,
        )
        .expect("the revision is applied");
    assert!(
        updated.revision > governed.revision,
        "the governed revision must advance"
    );
    let stale = store.update_episode_content(
        &source.episode_id,
        "我住在北岚市。（过期修订）",
        Some("evaluation"),
        governed.revision,
    );
    assert!(
        stale.is_err(),
        "a write with a stale revision must be rejected"
    );
    let after = evidence::inspect(&Inspect {
        request: &request,
        db: &fixture.harness.db,
        partition: &partition,
        policy: &fixture.harness.policy,
        audit: &fixture.harness.audit,
        canary: corpus::ORACLE_CANARY,
        case: "c01_long_range_direct",
    })
    .expect("the evidence pass runs");
    let entry = after
        .sources
        .iter()
        .find(|entry| entry.episode_id == source.episode_id)
        .expect("the delivered source is still inspected");
    let revision = entry
        .premises
        .iter()
        .find(|premise| premise.kind == schema::PremiseKind::RevisionCas)
        .expect("the revision premise is decided");
    assert!(
        !revision.satisfied,
        "the delivered snapshot must no longer match the governed revision"
    );
}

/// The seal chain verifies, and the shard can be revealed only once.
#[tokio::test]
async fn the_seal_chain_verifies_and_the_reveal_is_one_time() {
    let dir = TempDir::new().expect("temp dir");
    let custody = dir.path().join("custody");
    let input = corpus::input_bundle();
    let oracle = corpus::oracle_bundle(&input);
    let seal = super::seal::create_seal(&input, &oracle, &custody).expect("the seal is written");
    let executable = std::fs::read(std::env::current_exe().expect("the executable path"))
        .expect("the executable reads");
    let receipt = super::seal::create_build_receipt("cargo build", "desktop", &executable)
        .expect("the build receipt is written");
    let config_sha256 = super::artifacts::hash(
        &serde_json::to_string(&Config::offline()).expect("the config renders"),
    );
    let freeze = super::seal::create_freeze(
        &seal,
        &receipt,
        &config_sha256,
        1,
        &super::artifacts::hash(schema::READER_SYSTEM),
    );
    super::seal::verify_chain(
        &input,
        &seal,
        &freeze,
        &receipt,
        &config_sha256,
        &executable,
    )
    .expect("the chain verifies");
    super::seal::claim_reveal(&custody, &seal.seal_sha256, "run-dir", &oracle.input_sha256)
        .expect("the first reveal is accepted");
    assert!(
        super::seal::claim_reveal(
            &custody,
            &seal.seal_sha256,
            "run-dir-2",
            &oracle.input_sha256
        )
        .is_err(),
        "a second reveal of the same shard must be rejected"
    );
}

/// A written artifact must never carry a credential.
///
/// ⭐ Two things are checked: a credential-shaped token is replaced by the
/// redaction marker, and the value of the credential environment variable (if
/// it is set at all) never reaches the file.
#[test]
fn the_written_artifacts_carry_no_credential() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("artifact.json");
    let shaped = "sk-should-never-be-written-0123456789";
    let payload = serde_json::json!({ "note": format!("a credential looks like {shaped} here") });
    super::artifacts::write_new(&path, &payload).expect("the artifact is written");
    let text = std::fs::read_to_string(&path).expect("the artifact reads");
    assert!(
        !text.contains(shaped),
        "the written artifact must not contain a credential-shaped string"
    );
    assert!(
        text.contains("[REDACTED]"),
        "the credential-shaped string must be replaced by the redaction marker"
    );
    if let Ok(value) = std::env::var("OPENAI_API_KEY") {
        assert!(
            !text.contains(&value),
            "the written artifact must not contain the credential of the environment"
        );
    }
}

/// The reveal is verifiable from the files alone: a verifier that only reads
/// the custody directory can recompute the binding and confirm the ledger.
#[tokio::test]
async fn the_reveal_is_verifiable_from_the_files_alone() {
    let dir = TempDir::new().expect("temp dir");
    let custody = dir.path().join("custody");
    let input = corpus::input_bundle();
    let oracle = corpus::oracle_bundle(&input);
    let seal = super::seal::create_seal(&input, &oracle, &custody).expect("the seal is written");
    let entry =
        super::seal::claim_reveal(&custody, &seal.seal_sha256, "run-dir", &oracle.input_sha256)
            .expect("the first reveal is accepted");
    assert_eq!(
        entry.protocol,
        super::seal::LEDGER_PROTOCOL,
        "the ledger entry must declare the ledger protocol"
    );
    let verified =
        super::seal::verify_reveal(&custody, Some("run-dir")).expect("the reveal verifies");
    assert_eq!(
        verified.seal_sha256, seal.seal_sha256,
        "the verified reveal must point at the sealed shard"
    );
    assert!(
        super::seal::verify_reveal(&custody, Some("another-run-dir")).is_err(),
        "a reveal that points at another run directory must not verify"
    );
}

/// The real run path binds the seal chain: the chain is verified before the
/// first request, the shard is revealed once, and the reveal is verifiable.
#[tokio::test]
async fn the_run_path_binds_the_seal_chain() {
    let dir = TempDir::new().expect("temp dir");
    let custody = dir.path().join("custody");
    let run_dir = dir.path().join("run");
    std::fs::create_dir_all(&run_dir).expect("the run directory is created");
    let input = corpus::input_bundle();
    let oracle = corpus::oracle_bundle(&input);
    let (chain, reveal, probes) =
        super::bind_seal_chain(&custody, &input, &oracle, &Config::offline(), &run_dir)
            .expect("the run path binds the seal chain");
    assert_eq!(
        chain.get("status").and_then(|value| value.as_str()),
        Some("verified"),
        "the seal chain must verify"
    );
    assert_eq!(
        reveal.get("protocol").and_then(|value| value.as_str()),
        Some(super::seal::LEDGER_PROTOCOL),
        "the reveal must declare the ledger protocol"
    );
    for name in ["reveal_is_one_time", "reveal_is_independently_verifiable"] {
        let probe = probes
            .iter()
            .find(|probe| probe.get("name").and_then(|value| value.as_str()) == Some(name))
            .unwrap_or_else(|| panic!("the probe {name} is missing"));
        assert_eq!(
            probe.get("passed").and_then(|value| value.as_bool()),
            Some(true),
            "the probe {name} must pass"
        );
    }
}

/// The live configuration carries the endpoint, the model and the *name* of
/// the credential environment variable — never the credential itself.
#[test]
fn the_live_configuration_carries_no_credential() {
    let config = Config::live(
        "https://api.deepseek.com/v1",
        "deepseek-v4-flash",
        "OPENAI_API_KEY",
    );
    config
        .validate(true)
        .expect("the live configuration is valid");
    let rendered = serde_json::to_string(&config).expect("the configuration renders");
    assert!(
        !rendered.contains("sk-"),
        "the rendered configuration must not contain a credential-shaped string"
    );
    assert_eq!(
        config.key_env.as_deref(),
        Some("OPENAI_API_KEY"),
        "only the name of the credential environment variable is carried"
    );
    assert!(
        config.is_live(),
        "a configuration with an endpoint is a live configuration"
    );
    assert!(
        !Config::offline().is_live(),
        "the default configuration never reaches a real model"
    );
}

/// The vendor-private request fields are **opt-in**: without an explicit
/// switch the evaluation sends no vendor-private field at all, so an endpoint
/// that does not know the field is left alone.
#[test]
fn vendor_private_request_fields_are_opt_in() {
    let config = Config::offline();
    let body = live::vendor_body_from(&config, None).expect("no vendor fields");
    assert!(
        body.as_object().expect("object").is_empty(),
        "without an explicit switch no vendor-private field may be sent: {body}"
    );

    let mut switched = Config::offline();
    switched.thinking = Some(false);
    let body = live::vendor_body_from(&switched, None).expect("the switch is accepted");
    assert_eq!(body["thinking"]["type"], "disabled");

    let mut with_effort = Config::offline();
    with_effort.reasoning_effort = Some("low".to_string());
    let body = live::vendor_body_from(&with_effort, None).expect("the effort is accepted");
    assert_eq!(body["reasoning_effort"], "low");
}

/// A reserved protocol key is refused **before the first request**, so a
/// misconfiguration reads as a configuration error instead of a vendor 400.
#[test]
fn vendor_private_request_fields_refuse_reserved_protocol_keys() {
    let config = Config::offline();
    let error = live::vendor_body_from(&config, Some(r#"{"messages": []}"#))
        .expect_err("a reserved key must be refused");
    assert!(
        error.to_string().contains("messages"),
        "the error must name the offending key: {error}"
    );
    let error = live::vendor_body_from(&config, Some("not json"))
        .expect_err("a malformed extra body must be refused");
    assert!(
        error.to_string().contains("JSON"),
        "the error must say the extra body is not valid JSON: {error}"
    );
    let error = live::vendor_body_from(&config, Some("[1, 2]"))
        .expect_err("a non-object extra body must be refused");
    assert!(
        error.to_string().contains("对象"),
        "the error must say the extra body must be an object: {error}"
    );
}
