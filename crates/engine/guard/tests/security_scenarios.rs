use apeireth_guard::{AgentChainFeatureV2, ChainRiskClassifier, JointRiskClassifier};

const SCENARIOS: &str = include_str!("../../../../scripts/guard_ml/scenarios.jsonl");
const ARTIFACT: &str = include_str!("../../../../artifacts/guard-joint-shadow-v0.json");

fn feature_from_row(values: &serde_json::Value) -> AgentChainFeatureV2 {
    let number = |name: &str| {
        values
            .get(name)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
    };
    let flag = |name: &str| number(name) > 0.5;
    let mut features = AgentChainFeatureV2::default();
    features.alignment_score = number("alignment_score");
    features.credential_to_external = flag("credential_to_external");
    features.unrequested_network_egress = flag("unrequested_network_egress");
    features.unrequested_credential_access = flag("unrequested_credential_access");
    features.unrequested_shell_execution = flag("unrequested_shell_execution");
    features.unrequested_delete = flag("unrequested_delete");
    features.unrequested_publish = flag("unrequested_publish");
    features.v1.sensitive_to_external_flow = flag("sensitive_to_external_flow");
    features.v1.retry_after_denial = flag("retry_after_denial");
    features.v1.alternate_tool_after_denial = flag("alternate_tool_after_denial");
    features.v1.denied_count = number("denied_count") as u32;
    features.v1.external_effect_count = number("external_effect_count") as u32;
    features.scope_expansion_count = number("scope_expansion_count") as u32;
    features.cross_turn.denied_action_count = number("cross_turn_denied_action_count") as u32;
    features.cross_turn.credential_probe_count = number("cross_turn_credential_probe_count") as u32;
    features.failed_action_ratio = number("failed_action_ratio");
    features
}

#[test]
fn scenario_fixture_is_bounded_and_covers_required_categories() {
    let mut count = 0;
    let mut categories = std::collections::BTreeSet::new();
    for line in SCENARIOS.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        count += 1;
        categories.insert(row["category"].as_str().unwrap().to_string());
        let serialized = row["features"].to_string();
        for forbidden in ["prompt", "command", "path", "url", "secret", "password"] {
            assert!(!serialized.to_ascii_lowercase().contains(forbidden));
        }
    }
    assert!(count >= 30);
    for required in [
        "benign",
        "hard_negative",
        "scope_creep",
        "exfiltration",
        "credential",
        "destructive",
        "retry_bypass",
        "tool_switching",
        "cross_turn_probing",
        "security_tamper",
    ] {
        assert!(categories.contains(required), "missing category {required}");
    }
}

#[test]
fn generated_shadow_classifier_matches_fixture_labels() {
    let classifier = JointRiskClassifier::from_json_str(ARTIFACT).unwrap();
    for line in SCENARIOS.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value = serde_json::from_str(line).unwrap();
        let features = feature_from_row(&row["features"]);
        let prediction = classifier.classify_v2(&features);
        assert_eq!(
            prediction.score >= 0.5,
            row["label"].as_i64().unwrap() == 1,
            "{}",
            row["id"]
        );
    }
}
