//! `hook` 模块的单元测试。
//!
//! 独立成文件（而不是内联在 `hook.rs` 里）是为了让 `hook.rs` 保持在尺寸棘轮
//! 之内 —— 棘轮只减不增，测试代码长大不该把主文件顶破。

use super::*;
use super::*;
use apeireth_core::kernel::{CapabilityId, TraceId};
use apeireth_governance::{Action, Decision};

use crate::classifier::RiskPrediction;
use crate::decision::GuardStage;
use crate::intent::AlignmentClass;

/// 构造一次能力派发请求；参数引用由调用方持有。
fn dispatch<'a>(
    capability: &'a CapabilityId,
    arguments: &'a serde_json::Value,
    session: SessionId,
    trace: TraceId,
    round: u32,
    action_id: &'a str,
) -> GovernanceRequest<'a> {
    GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability,
            arguments,
        },
        session,
        trace,
        round,
    )
    .with_action_id(action_id)
}

/// 走真实语义描述符生成归一化观察（不手搓 40 个字段）。
fn observation_for(capability_id: &str, arguments: &serde_json::Value) -> SafetyObservation {
    let capability = CapabilityId::new(capability_id).expect("test capability id must be valid");
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    let registry = CapabilitySafetyRegistry::canonical();
    SafetyObservation::from_governance_request_with_provider(
        &request,
        0,
        false,
        Vec::new(),
        Some(&registry),
    )
}

fn credential_arguments() -> serde_json::Value {
    serde_json::json!({ "path": ".env" })
}

fn credential_request<'a>(
    capability: &'a CapabilityId,
    arguments: &'a serde_json::Value,
    session: SessionId,
    trace: TraceId,
    round: u32,
    action_id: &'a str,
) -> GovernanceRequest<'a> {
    dispatch(capability, arguments, session, trace, round, action_id)
}

fn mounted_hook() -> BehaviorChainGuardHook {
    BehaviorChainGuardHook::new().with_immune_memory(Arc::new(ImmuneMemory::with_capacity(32)))
}

fn aligned_assessment() -> AlignmentAssessment {
    AlignmentAssessment {
        class: AlignmentClass::Aligned,
        score: 0.0,
        reasons: Vec::new(),
    }
}

/// 特征快照表**有界**：连续评估超过上限后，最旧的被淘汰
/// （guard hook 是长生命周期单例 ⇒ 没有上限就按「动作数」无界增长）。
#[test]
fn feature_snapshot_store_is_bounded() {
    let hook = BehaviorChainGuardHook::new();
    let session = SessionId::new();
    let capability = CapabilityId::new("credential.read").expect("valid id");
    let arguments = credential_arguments();
    let trace = TraceId::new();

    // 超过上限 + 余量：上限是 1024，跑 1100 次足够触发淘汰。
    for index in 0..(crate::snapshot_store::MAX_LAST_SNAPSHOTS + 76) {
        let action_id = format!("action-{index}");
        let request = credential_request(&capability, &arguments, session, trace, 1, &action_id);
        let _ = hook.evaluate_detailed(&request);
    }

    let max = crate::snapshot_store::MAX_LAST_SNAPSHOTS;
    assert!(
        hook.last_snapshots.len() <= max,
        "快照表必须有界，实际 {} 条（上限 {max}）",
        hook.last_snapshots.len()
    );
    // 最早的应该已被淘汰，最新的还在。
    assert!(
        !hook.last_snapshots.contains(&session, "action-0"),
        "最旧的快照应被淘汰"
    );
    assert!(
        hook.last_snapshots
            .contains(&session, &format!("action-{}", max + 75)),
        "最新的快照应保留"
    );
}

/// `clear_session` 也要清特征快照 —— 否则会话结束后它的快照仍然驻留。
#[test]
fn clear_session_drops_feature_snapshots() {
    let hook = BehaviorChainGuardHook::new();
    let session = SessionId::new();
    let capability = CapabilityId::new("credential.read").expect("valid id");
    let arguments = credential_arguments();
    let trace = TraceId::new();
    let request = credential_request(&capability, &arguments, session, trace, 1, "a1");
    let _ = hook.evaluate_detailed(&request);
    assert!(
        hook.last_feature_snapshot(&session, "a1").is_some(),
        "评估后应留下快照"
    );

    hook.clear_session(&session);
    assert!(
        hook.last_feature_snapshot(&session, "a1").is_none(),
        "会话结束后快照必须清掉"
    );
}

#[test]
fn default_off_decision_matches_pinned_baseline() {
    let hook = BehaviorChainGuardHook::new();
    let session = SessionId::new();
    let trace = TraceId::new();
    let capability = CapabilityId::new("credential.read").expect("valid id");
    let arguments = credential_arguments();
    let request = credential_request(
        &capability,
        &arguments,
        session,
        trace,
        1,
        "baseline-action",
    );

    let decision = hook.evaluate_detailed(&request);
    let expected = GuardDecision {
        decision: Decision::deny("guard chain denied: dotfile_or_hidden_resource_access"),
        risk_score: 0.85,
        reasons: vec![
            "credential_store_access".to_string(),
            "dotfile_or_hidden_resource_access".to_string(),
            "turn_intent_unavailable".to_string(),
        ],
        evidence: vec!["alignment=ScopeExpansion score=0.75".to_string()],
        stage: GuardStage::ChainGuard,
        classifier_prediction: Some(RiskPrediction::unavailable()),
    };
    assert_eq!(
        serde_json::to_string(&decision).expect("decision serializes"),
        serde_json::to_string(&expected).expect("baseline serializes"),
        "默认关路径必须与接线前的 pinned 判定逐字节一致"
    );
    assert!(
        decision
            .evidence
            .iter()
            .all(|line| !line.starts_with("immune:")),
        "默认关不得追加 immune evidence"
    );
    assert!(hook
        .last_immune_signal(&session, &trace.to_string())
        .is_none());
    assert!(!hook.immune_memory_enabled());
}

#[test]
fn default_off_never_touches_immune_outputs() {
    let hook = BehaviorChainGuardHook::new();
    let session = SessionId::new();
    let trace = TraceId::new();
    let capability = CapabilityId::new("credential.read").expect("valid id");
    let arguments = credential_arguments();
    let request = credential_request(&capability, &arguments, session, trace, 1, "a1");

    let (decision, verdict) = hook.evaluate_internal(&request);
    assert!(matches!(decision.decision, Decision::Deny { .. }));
    assert!(decision
        .evidence
        .iter()
        .all(|line| !line.starts_with("immune:")));
    assert!(verdict
        .metadata
        .iter()
        .all(|(key, _)| !key.starts_with("immune_")));
    assert!(hook.shared_immune_memory().is_none());
    assert!(hook
        .last_immune_signal(&session, &trace.to_string())
        .is_none());
}

#[test]
fn mounted_memory_only_appends_immune_signal() {
    let plain = BehaviorChainGuardHook::new();
    let immune = mounted_hook();
    let session = SessionId::new();
    let trace = TraceId::new();
    let capability = CapabilityId::new("credential.read").expect("valid id");
    let arguments = credential_arguments();
    let request = credential_request(&capability, &arguments, session, trace, 1, "a1");

    let (baseline_decision, baseline_verdict) = plain.evaluate_internal(&request);
    let (observed_decision, observed_verdict) = immune.evaluate_internal(&request);

    assert_eq!(observed_decision.decision, baseline_decision.decision);
    assert_eq!(observed_decision.risk_score, baseline_decision.risk_score);
    assert_eq!(observed_decision.reasons, baseline_decision.reasons);
    assert_eq!(observed_decision.stage, baseline_decision.stage);
    assert_eq!(
        observed_decision.classifier_prediction,
        baseline_decision.classifier_prediction
    );

    let added: Vec<&String> = observed_decision
        .evidence
        .iter()
        .filter(|line| line.starts_with("immune:"))
        .collect();
    assert_eq!(added.len(), 1, "每次风险出口恰好追加一条 immune evidence");
    assert_eq!(
        observed_decision.evidence.len(),
        baseline_decision.evidence.len() + 1
    );

    let mut stripped_decision = observed_decision.clone();
    stripped_decision
        .evidence
        .retain(|line| !line.starts_with("immune:"));
    assert_eq!(
        serde_json::to_string(&stripped_decision).expect("stripped decision serializes"),
        serde_json::to_string(&baseline_decision).expect("baseline decision serializes"),
        "挂载后除 immune 旁路外主判定逐字节不变"
    );

    let mut stripped_verdict = observed_verdict.clone();
    for key in [
        "immune_kind",
        "immune_tier",
        "immune_escalated",
        "immune_record_key",
        "immune_reason",
    ] {
        stripped_verdict.metadata.remove(key);
    }
    // `guard_evidence` 是 evidence 的拼接结果，需把追加上去的 immune 段一并去掉。
    if let Some(joined) = stripped_verdict.metadata.remove("guard_evidence") {
        let cleaned = joined
            .split("; ")
            .filter(|part| !part.starts_with("immune:"))
            .collect::<Vec<_>>()
            .join("; ");
        if !cleaned.is_empty() {
            stripped_verdict.metadata.insert("guard_evidence", cleaned);
        }
    }
    assert_eq!(stripped_verdict, baseline_verdict);
    assert!(observed_verdict.metadata.contains_key("immune_tier"));
    assert!(immune
        .last_immune_signal(&session, &trace.to_string())
        .is_some());
    assert!(plain
        .last_immune_signal(&session, &trace.to_string())
        .is_none());
}

#[test]
fn first_high_severity_incident_escalates_to_lockdown() {
    let hook = mounted_hook();
    let session = SessionId::new();
    let trace = TraceId::new();
    let capability = CapabilityId::new("credential.read").expect("valid id");
    let arguments = credential_arguments();
    let request = credential_request(&capability, &arguments, session, trace, 1, "a1");

    let (decision, _) = hook.evaluate_internal(&request);
    assert!(matches!(decision.decision, Decision::Deny { .. }));
    let signal = hook
        .last_immune_signal(&session, &trace.to_string())
        .expect("mounted hook must record an immune signal");
    assert_eq!(signal.kind, IncidentKind::CredentialLeak);
    assert_eq!(signal.tier(), SeverityTier::Tier3Lockdown);
    assert!(signal.escalated());
    assert!(signal.evidence_line().contains("tier=tier3_lockdown"));

    let shared = hook
        .shared_immune_memory()
        .expect("mounted hook exposes its memory");
    let record = shared
        .lock()
        .record(&signal.signature)
        .expect("escalated incident must be recorded");
    assert_eq!(record.occurrences, 1);
    assert_eq!(record.tolerance_level, 0);
}

#[test]
fn second_identical_signature_downgrades_without_releasing_deny() {
    let hook = mounted_hook();
    let session = SessionId::new();
    let trace = TraceId::new();
    let capability = CapabilityId::new("credential.read").expect("valid id");
    let arguments = credential_arguments();

    let first_request = credential_request(&capability, &arguments, session, trace, 1, "a1");
    let (first_decision, _) = hook.evaluate_internal(&first_request);
    let first = hook
        .last_immune_signal(&session, &trace.to_string())
        .expect("first observation");

    let second_request = credential_request(&capability, &arguments, session, trace, 2, "a2");
    let (second_decision, _) = hook.evaluate_internal(&second_request);
    let second = hook
        .last_immune_signal(&session, &trace.to_string())
        .expect("second observation");

    assert_eq!(first.signature, second.signature, "同签名才能命中适应记忆");
    assert_eq!(first.tier(), SeverityTier::Tier3Lockdown);
    assert_eq!(second.tier(), SeverityTier::Tier1Note);
    assert!(second.tier() < first.tier(), "二次暴露必须降级");
    assert!(!second.escalated(), "已知模式不得再升级");
    assert!(matches!(first_decision.decision, Decision::Deny { .. }));
    assert!(
        matches!(second_decision.decision, Decision::Deny { .. }),
        "免疫降级不允许把 Deny 改成 Allow（轻 != 允许）"
    );

    let shared = hook
        .shared_immune_memory()
        .expect("mounted hook exposes its memory");
    let record = shared
        .lock()
        .record(&second.signature)
        .expect("known pattern stays recorded");
    assert_eq!(record.occurrences, 2);
    assert_eq!(record.tolerance_level, 1);
}

#[test]
fn self_origin_tolerance_is_quiet_and_records_nothing() {
    let observation = observation_for("credential.read", &credential_arguments());
    let decision = GuardDecision {
        decision: Decision::require_approval("human confirmation required"),
        risk_score: 0.5,
        reasons: Vec::new(),
        evidence: Vec::new(),
        stage: GuardStage::ChainGuard,
        classifier_prediction: None,
    };
    let mut memory = ImmuneMemory::with_capacity(8);
    let signal = observe_incident(
        &mut memory,
        &observation,
        &decision,
        &aligned_assessment(),
        false,
        true,
        1_000,
    );

    assert_eq!(signal.tier(), SeverityTier::Tier0Quiet);
    assert!(!signal.escalated());
    assert_eq!(memory.open_count(), 0);
    assert_eq!(memory.known_patterns(), 0);
}

#[test]
fn resolved_incident_is_quiet_inside_cooldown() {
    let hook = mounted_hook();
    let session = SessionId::new();
    let trace = TraceId::new();
    let capability = CapabilityId::new("credential.read").expect("valid id");
    let arguments = credential_arguments();

    let first_request = credential_request(&capability, &arguments, session, trace, 1, "a1");
    let (_, _) = hook.evaluate_internal(&first_request);
    let first = hook
        .last_immune_signal(&session, &trace.to_string())
        .expect("first observation");
    assert_eq!(first.tier(), SeverityTier::Tier3Lockdown);

    let shared = hook
        .shared_immune_memory()
        .expect("mounted hook exposes its memory");
    assert!(shared
        .lock()
        .resolve(&first.signature, chrono::Utc::now().timestamp_millis()));

    let second_request = credential_request(&capability, &arguments, session, trace, 2, "a2");
    let (second_decision, _) = hook.evaluate_internal(&second_request);
    let second = hook
        .last_immune_signal(&session, &trace.to_string())
        .expect("second observation");

    assert_eq!(second.signature, first.signature);
    assert_eq!(
        second.tier(),
        SeverityTier::Tier0Quiet,
        "冷却窗内只静默计数"
    );
    assert!(!second.escalated());
    assert!(matches!(second_decision.decision, Decision::Deny { .. }));
    assert_eq!(shared.lock().open_count(), 0);
}

#[test]
fn allowed_actions_are_never_observed() {
    let hook = mounted_hook();
    let session = SessionId::new();
    let trace = TraceId::new();
    let capability = CapabilityId::new("fs.read").expect("valid id");
    let arguments = serde_json::json!({ "path": "README.md" });
    let request = dispatch(&capability, &arguments, session, trace, 1, "a1");

    let (decision, verdict) = hook.evaluate_internal(&request);
    assert!(decision.is_allowed(), "纯读路径应保持 allow");
    assert!(
        hook.last_immune_signal(&session, &trace.to_string())
            .is_none(),
        "Allow 出口不得观察，避免正常流量预置记忆"
    );
    assert_eq!(
        hook.shared_immune_memory()
            .expect("mounted hook exposes its memory")
            .lock()
            .known_patterns(),
        0
    );
    assert!(verdict
        .metadata
        .iter()
        .all(|(key, _)| !key.starts_with("immune_")));
}

#[test]
fn incident_kind_mapping_matches_documented_table() {
    let aligned = aligned_assessment();
    let injection = AlignmentAssessment {
        class: AlignmentClass::HighRiskMismatch,
        score: 0.95,
        reasons: vec!["unrequested_credential_access".to_string()],
    };

    assert_eq!(
        classify_incident(
            &observation_for("credential.read", &credential_arguments()),
            &aligned,
            false
        ),
        IncidentKind::CredentialLeak
    );
    assert_eq!(
        classify_incident(
            &observation_for(
                "http.send",
                &serde_json::json!({ "url": "https://example.com" })
            ),
            &aligned,
            false
        ),
        IncidentKind::EgressAnomaly
    );
    assert_eq!(
        classify_incident(
            &observation_for(
                "tool.content_feed",
                &serde_json::json!({ "url": "https://example.com" })
            ),
            &aligned,
            false
        ),
        IncidentKind::UntrustedContentEscape
    );
    assert_eq!(
        classify_incident(
            &observation_for("tool.mystery", &serde_json::json!({})),
            &aligned,
            false
        ),
        IncidentKind::ToolPoison
    );
    assert_eq!(
        classify_incident(
            &observation_for("fs.read", &serde_json::json!({ "path": "README.md" })),
            &injection,
            true
        ),
        IncidentKind::PromptInjection
    );
    assert_eq!(
        classify_incident(
            &observation_for("fs.read", &serde_json::json!({ "path": "README.md" })),
            &injection,
            false
        ),
        IncidentKind::Unknown,
        "未绑定 intent 时不得把对齐缺失当成注入"
    );
}

#[test]
fn verdict_metadata_carries_immune_tier_without_changing_decision() {
    let hook = mounted_hook();
    let session = SessionId::new();
    let trace = TraceId::new();
    let capability = CapabilityId::new("credential.read").expect("valid id");
    let arguments = credential_arguments();
    let request = credential_request(&capability, &arguments, session, trace, 1, "a1");

    let (decision, verdict) = hook.evaluate_internal(&request);
    assert!(matches!(decision.decision, Decision::Deny { .. }));
    assert_eq!(verdict.decision, decision.decision, "主结论不得被免疫改写");
    assert_eq!(verdict.metadata.get("immune_kind"), Some("credential_leak"));
    assert_eq!(verdict.metadata.get("immune_tier"), Some("tier3_lockdown"));
    assert_eq!(verdict.metadata.get("immune_escalated"), Some("true"));
    assert!(verdict
        .metadata
        .get("immune_record_key")
        .is_some_and(|key| key.starts_with("credential_leak:")));
    assert!(verdict
        .metadata
        .get("immune_reason")
        .is_some_and(|reason| !reason.is_empty()));
    assert!(verdict
        .metadata
        .get("guard_evidence")
        .is_some_and(|evidence| evidence.contains("immune:")));
}
