use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{
    Action, DestructivePolicy, GovernanceHook, GovernanceRequest, IntentClass, MutationPolicy,
    NetworkPolicy, ShellPolicy, TurnSecurityContext,
};
use apeireth_guard::run_scenario;
use apeireth_guard::{
    AgentChainFeatureV2, BehaviorChainGuardHook, ChainRiskClassifier, ClassifierEnforcementMode,
    CommandEffectAnalyzer, DatasetRecorder, DecisionFusion, DescriptorSource, FeatureSnapshot,
    IntentInput, IntentInterpreter, JointRiskClassifier, NegationAwareOperationExtractor,
    NoClassifier, RiskClass, RiskPrediction, RuleIntentInterpreter, ScenarioCatalog,
    SessionBehaviorHistory, TurnBehaviorSummary, GUARD_DATASET_V3, MAX_TURN_HISTORY,
};

fn interpret(text: &str) -> apeireth_governance::TaskIntentEnvelopeV1 {
    RuleIntentInterpreter.interpret(IntentInput {
        session_id: "s".into(),
        trace_id: "t".into(),
        user_request: text.into(),
        created_at_ms: 1,
    })
}

#[test]
fn unknown_intent_fails_narrow() {
    let intent = interpret("hello world maybe later");
    assert_eq!(intent.intent_class, IntentClass::Unknown);
    assert_eq!(intent.network_policy, NetworkPolicy::Deny);
    assert_eq!(
        intent.credential_policy,
        apeireth_governance::CredentialPolicy::Deny
    );
    assert_eq!(intent.shell_policy, ShellPolicy::Deny);
    assert_eq!(intent.mutation_policy, MutationPolicy::Deny);
    assert_eq!(intent.destructive_policy, DestructivePolicy::Deny);
    assert_eq!(
        intent.persistence_policy,
        apeireth_governance::PersistencePolicy::Deny
    );
}

#[test]
fn negation_cases_cover_required_phrases() {
    let inspect = interpret("只检查，不要修改，也不要联网");
    assert_eq!(inspect.intent_class, IntentClass::ReadOnlyInspection);
    assert_eq!(inspect.mutation_policy, MutationPolicy::Deny);
    assert_eq!(inspect.network_policy, NetworkPolicy::Deny);

    let no_push = interpret("不要 push，只提交本地修改");
    assert!(!no_push.allows_publish());
    assert!(no_push.allows_mutation());

    let no_delete = interpret("不要删除任何文件");
    assert_eq!(no_delete.destructive_policy, DestructivePolicy::Deny);

    let token = interpret("检查 token 配置，但不要显示 token 内容");
    assert_eq!(token.intent_class, IntentClass::CredentialOperation);
    assert!(!token.allows_network());

    let tests = interpret("运行测试，但不要安装任何东西");
    assert!(tests.allows_shell());
    assert_eq!(
        tests.persistence_policy,
        apeireth_governance::PersistencePolicy::Deny
    );

    let no_shell = interpret("不要运行命令，只分析源码");
    assert_eq!(no_shell.shell_policy, ShellPolicy::Deny);
    assert_eq!(no_shell.mutation_policy, MutationPolicy::Deny);

    for phrase in [
        "不要 push",
        "不要删除",
        "不要联网",
        "不要执行",
        "不要修改",
        "不要读取 token",
        "don't push",
        "do not delete",
        "never run",
    ] {
        let extracted = NegationAwareOperationExtractor::extract(phrase);
        assert!(
            extracted.publish == Some(apeireth_guard::OperationPolarity::Denied)
                || extracted.delete == Some(apeireth_guard::OperationPolarity::Denied)
                || extracted.network == Some(apeireth_guard::OperationPolarity::Denied)
                || extracted.shell == Some(apeireth_guard::OperationPolarity::Denied)
                || extracted.write == Some(apeireth_guard::OperationPolarity::Denied)
                || extracted.credential == Some(apeireth_guard::OperationPolarity::Denied)
                || extracted.credential_disclosure_denied,
            "missing negation for {phrase}"
        );
    }
}

#[test]
fn shadow_advisory_enforce_are_distinct() {
    let base = apeireth_guard::GuardDecision::allow_fast();
    let fast = apeireth_guard::FastGuardResult::allow();
    let prediction = RiskPrediction {
        class: RiskClass::Critical,
        score: 0.99,
        confidence: 0.99,
        confidence_kind: apeireth_guard::MARGIN_CONFIDENCE_KIND.to_string(),
        model_version: "t".into(),
        available: true,
    };
    let mut features = AgentChainFeatureV2::default();
    features.v1.sensitive_to_external_flow = true;
    let shadow = DecisionFusion::apply(
        ClassifierEnforcementMode::Shadow,
        &base,
        &fast,
        &prediction,
        &features,
    );
    assert!(matches!(
        shadow.decision,
        apeireth_governance::Decision::Allow
    ));
    let advisory = DecisionFusion::apply(
        ClassifierEnforcementMode::Advisory,
        &base,
        &fast,
        &prediction,
        &features,
    );
    assert!(matches!(
        advisory.decision,
        apeireth_governance::Decision::RequireApproval { .. }
    ));
    let enforce = DecisionFusion::apply(
        ClassifierEnforcementMode::Enforce,
        &base,
        &fast,
        &prediction,
        &features,
    );
    assert!(matches!(
        enforce.decision,
        apeireth_governance::Decision::Deny { .. }
    ));
}

#[tokio::test]
async fn same_turn_three_actions_count_as_one_turn() {
    let hook = BehaviorChainGuardHook::new();
    let session = SessionId::new();
    let trace = TraceId::new();
    for (round, name) in [
        (1, "credential.read"),
        (2, "env.read"),
        (3, "credential.read"),
    ] {
        let cap = CapabilityId::new(name).unwrap();
        let args = serde_json::json!({});
        let req = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &cap,
                arguments: &args,
            },
            session,
            trace,
            round,
        );
        let _ = hook.evaluate(&req).await;
    }
    let summary = hook.session_behavior_summary(&session);
    assert_eq!(summary.recent_turns, 1);
    let verdicts_cross = hook.status();
    assert!(verdicts_cross.enabled);
    let last = hook.chain_for_trace(&session, &trace.to_string()).unwrap();
    let features = AgentChainFeatureV2::from_chain(&last);
    assert_eq!(features.cross_turn.sensitive_probe_turn_count, 0);
}

#[tokio::test]
async fn three_traces_trigger_cross_turn_probing() {
    let hook = BehaviorChainGuardHook::new();
    let session = SessionId::new();
    let mut last = None;
    for name in ["credential.read", "env.read", "secret.read"] {
        let cap = CapabilityId::new(name).unwrap();
        let args = serde_json::json!({});
        let req = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &cap,
                arguments: &args,
            },
            session,
            TraceId::new(),
            1,
        );
        last = Some(hook.evaluate_verbose(&req).await);
    }
    let verdict = last.unwrap();
    assert!(
        verdict
            .metadata
            .get("guard_reasons")
            .is_some_and(|value| value.contains("cross_turn_sensitive_probing")),
        "{verdict:?}"
    );
}

#[test]
fn history_is_bounded_and_decays() {
    let mut history = SessionBehaviorHistory::default();
    for index in 0..20 {
        history.upsert(TurnBehaviorSummary {
            trace_id: format!("t{index}"),
            max_risk_score: if index == 0 { 1.0 } else { 0.01 },
            denied: index == 0,
            approval_required: false,
            credential_probe_count: u32::from(index == 0),
            sensitive_read_count: 0,
            network_egress_count: 0,
            scope_expansion_count: 0,
            alternate_tool_count: 0,
            retry_after_denial_count: 0,
            destructive_count: 0,
            publish_count: 0,
            intent_class: IntentClass::Unknown,
            completed_at_ms: i64::from(index),
        });
    }
    assert_eq!(history.turns.len(), MAX_TURN_HISTORY);
    assert!(history.turns.front().unwrap().trace_id != "t0");
    let cross = history.cross_turn_features("current");
    assert!(
        cross.risk_trend < 0.2,
        "stale high risk should decay: {}",
        cross.risk_trend
    );
}

struct RecordingClassifier {
    inner: NoClassifier,
    last: std::sync::Mutex<Option<AgentChainFeatureV2>>,
}

impl ChainRiskClassifier for RecordingClassifier {
    fn classify(&self, features: &apeireth_guard::AgentChainFeatureV1) -> RiskPrediction {
        self.inner.classify(features)
    }

    fn classify_v2(&self, features: &AgentChainFeatureV2) -> RiskPrediction {
        *self.last.lock().expect("classifier lock") = Some(features.clone());
        self.inner.classify_v2(features)
    }
}

#[tokio::test]
async fn classifier_and_dataset_share_exact_feature_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("guard.jsonl");
    let recorder = Arc::new(DatasetRecorder::new(&path));
    recorder.set_enabled(true);
    let classifier = Arc::new(RecordingClassifier {
        inner: NoClassifier,
        last: std::sync::Mutex::new(None),
    });
    let hook = BehaviorChainGuardHook::new()
        .with_dataset_recorder(recorder)
        .with_classifier(classifier.clone());
    let cap = CapabilityId::new("fs.read").unwrap();
    let args = serde_json::json!({"path_class": "workspace"});
    let req = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &cap,
            arguments: &args,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    let _ = hook.evaluate(&req).await;
    let classified = classifier.last.lock().unwrap().clone().unwrap();
    let raw = std::fs::read_to_string(&path).unwrap();
    let row: serde_json::Value = serde_json::from_str(raw.lines().next().unwrap()).unwrap();
    assert_eq!(row["format"], GUARD_DATASET_V3);
    let stored: AgentChainFeatureV2 =
        serde_json::from_value(row["chain_features"].clone()).unwrap();
    assert_eq!(classified, stored);
    assert_eq!(classified.cross_turn, stored.cross_turn);
    assert!(row["feature_snapshot_id"].as_str().is_some());
}

#[test]
fn dataset_new_events_are_v3() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("guard.jsonl");
    let recorder = DatasetRecorder::new(&path);
    recorder.set_enabled(true);
    recorder.record_outcome(
        "t",
        Some("a"),
        None,
        None,
        Some("approved"),
        Some("success"),
    );
    recorder.record_approval("t", "a", "c", "p", "approved");
    recorder.record_execution("t", "a", "c", "success");
    recorder.record_compensation("t", "a", "success");
    let raw = std::fs::read_to_string(path).unwrap();
    for line in raw.lines() {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(row["format"], GUARD_DATASET_V3);
    }
}

#[test]
fn shell_effects_match_required_semantics() {
    let cargo = CommandEffectAnalyzer::analyze("cargo test");
    assert_eq!(
        cargo.operation_classes,
        vec![apeireth_governance::OperationClass::Execute]
    );
    let status = CommandEffectAnalyzer::analyze("git status");
    assert!(status
        .operation_classes
        .contains(&apeireth_governance::OperationClass::Read));
    let push = CommandEffectAnalyzer::analyze("git push");
    assert!(push.repository_publish && push.network_send);
    let rm = CommandEffectAnalyzer::analyze("rm file");
    assert!(rm.destructive);
    let get = CommandEffectAnalyzer::analyze("curl https://example.invalid");
    assert!(get.network_read && !get.network_send);
    let post = CommandEffectAnalyzer::analyze("curl -X POST https://example.invalid -d a=1");
    assert!(post.network_send);
}

#[tokio::test]
async fn same_effect_bypass_across_tool_families() {
    let hook = BehaviorChainGuardHook::new();
    let session = SessionId::new();
    let trace = TraceId::new();
    let intent = interpret("review the code, do not delete any files");
    let context =
        TurnSecurityContext::new(intent.intent_id.clone(), trace.to_string()).with_intent(intent);
    let del = CapabilityId::new("fs.delete").unwrap();
    let del_args = serde_json::json!({"path_class": "workspace"});
    let first = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &del,
            arguments: &del_args,
        },
        session,
        trace,
        1,
    )
    .with_security_context(&context);
    let first_verdict = hook.evaluate_verbose(&first).await;
    assert!(!first_verdict.is_allowed());
    let shell = CapabilityId::new("shell.exec").unwrap();
    let shell_args = serde_json::json!({"command": "rm file.txt"});
    let second = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &shell,
            arguments: &shell_args,
        },
        session,
        trace,
        2,
    )
    .with_security_context(&context);
    let _ = hook.evaluate_verbose(&second).await;
    let chain = hook.chain_for_trace(&session, &trace.to_string()).unwrap();
    assert!(
        chain.has_same_effect_after_denial(),
        "expected same-effect bypass edges: {:?}",
        chain.edges
    );
}

#[test]
fn unknown_model_feature_and_bad_thresholds_are_rejected() {
    let mut ok = serde_json::json!({
        "schema_version": "AgentChainFeatureV2",
        "feature_schema": "AgentChainFeatureV2",
        "model_id": "x",
        "model_version": "x",
        "feature_names": ["alignment_score"],
        "weights": [1.0],
        "bias": 0.0,
        "critical_threshold": 0.9,
        "high_threshold": 0.7,
        "medium_threshold": 0.4
    });
    ok["feature_schema_hash"] = serde_json::json!(apeireth_guard::feature_schema_hash());
    let sha = apeireth_guard::canonical_artifact_sha256(&ok.to_string()).unwrap();
    ok["artifact_sha256"] = serde_json::json!(sha);
    assert!(JointRiskClassifier::from_json_str(&ok.to_string()).is_ok());
    let typo = ok
        .to_string()
        .replace("alignment_score", "unrequested_network_egres");
    assert!(JointRiskClassifier::from_json_str(&typo).is_err());
    let mut bad = ok.clone();
    bad["medium_threshold"] = serde_json::json!(0.95);
    assert!(JointRiskClassifier::from_json_str(&bad.to_string()).is_err());
}

#[tokio::test]
async fn scenario_dsl_runs_real_extractor_and_catalog_is_large() {
    let catalog = ScenarioCatalog::all();
    assert!(catalog.len() >= 300, "catalog size {}", catalog.len());
    let languages: std::collections::BTreeSet<_> =
        catalog.iter().map(|item| item.language.as_str()).collect();
    assert!(languages.contains("zh") && languages.contains("en") && languages.contains("mixed"));
    let categories: std::collections::BTreeSet<_> =
        catalog.iter().map(|item| item.category.as_str()).collect();
    for required in [
        "benign",
        "hard_negative",
        "scope_creep",
        "credential",
        "exfiltration",
        "destructive",
        "tool_switching",
        "cross_turn_probing",
        "security_tamper",
        "unknown_plugin",
        "shell_semantics",
        "publish_alignment",
    ] {
        assert!(categories.contains(required), "missing {required}");
    }
    let outcome = run_scenario(&catalog[0]).await;
    assert_eq!(outcome.snapshot.schema_version, "AgentChainFeatureV2");
    assert!(catalog.iter().any(|item| item.label == "benign"));
    assert!(
        catalog
            .iter()
            .filter(|item| item.category == "hard_negative")
            .count()
            >= 8
    );
    assert!(categories.contains("ambiguous"));
    assert!(categories.contains("retry_bypass"));
    assert_ne!(outcome.snapshot.features.alignment_score, 0.95);
}

#[test]
fn intent_parser_alignment_and_fusion_are_fast_enough_for_local_path() {
    use std::time::Instant;
    let start = Instant::now();
    for _ in 0..200 {
        let _ = interpret("只检查这个仓库，不要修改，也不要联网");
    }
    let intent_us = start.elapsed().as_secs_f64() * 1_000_000.0 / 200.0;
    let start = Instant::now();
    for _ in 0..200 {
        let _ = CommandEffectAnalyzer::analyze("git push origin main");
    }
    let shell_us = start.elapsed().as_secs_f64() * 1_000_000.0 / 200.0;
    let start = Instant::now();
    for _ in 0..200 {
        let _ = apeireth_guard::descriptor_for_capability(
            "fs.read",
            &serde_json::json!({ "path_class": "workspace" }),
        );
    }
    let descriptor_us = start.elapsed().as_secs_f64() * 1_000_000.0 / 200.0;
    assert!(
        intent_us < 2_000.0 && shell_us < 1_000.0 && descriptor_us < 1_000.0,
        "local path too slow: intent={intent_us:.1}us shell={shell_us:.1}us descriptor={descriptor_us:.1}us"
    );
}

#[test]
fn descriptor_source_is_explicit() {
    let descriptor = apeireth_guard::descriptor_for_capability("fs.read", &serde_json::json!({}));
    assert_eq!(descriptor.source, DescriptorSource::Canonical);
    let unknown =
        apeireth_guard::descriptor_for_capability("plugin.mystery", &serde_json::json!({}));
    assert_ne!(unknown.source, DescriptorSource::Canonical);
}
