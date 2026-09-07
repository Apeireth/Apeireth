use apeireth_guard::{
    canonical_artifact_sha256, canonical_descriptor_coverage, family_split, feature_schema_hash,
    DescriptorSource, JointRiskClassifier, ScenarioCatalog, SecurityScenarioOracle,
    BUILTIN_CANONICAL_CAPABILITY_IDS,
};

#[test]
fn family_splits_are_disjoint() {
    let catalog = ScenarioCatalog::all();
    let mut buckets: std::collections::BTreeMap<&str, std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for scenario in &catalog {
        if scenario.holdout_group.is_empty() {
            buckets
                .entry(family_split(&scenario.family))
                .or_default()
                .insert(scenario.family.clone());
        }
    }
    let train = buckets.remove("train").unwrap_or_default();
    let val = buckets.remove("validation").unwrap_or_default();
    let test = buckets.remove("test").unwrap_or_default();
    let cal = buckets.remove("calibration").unwrap_or_default();
    assert!(train.is_disjoint(&val));
    assert!(train.is_disjoint(&test));
    assert!(train.is_disjoint(&cal));
    assert!(val.is_disjoint(&test));
    assert!(val.is_disjoint(&cal));
    assert!(test.is_disjoint(&cal));
    assert!(!train.is_empty() && !test.is_empty());
}

#[test]
fn intent_template_holdout_is_isolated() {
    let catalog = ScenarioCatalog::all();
    let holdout: std::collections::BTreeSet<_> = catalog
        .iter()
        .filter(|item| item.holdout_group == "intent_template")
        .map(|item| item.intent_template_id.as_str())
        .collect();
    assert!(!holdout.is_empty());
    for scenario in &catalog {
        if scenario.holdout_group.is_empty() {
            assert!(
                !holdout.contains(scenario.intent_template_id.as_str()),
                "held-out template {} leaked into train family {}",
                scenario.intent_template_id,
                scenario.family
            );
        }
    }
}

#[test]
fn oracle_source_stays_independent() {
    let source = include_str!("../src/oracle.rs");
    for forbidden in [
        ["RuleIntent", "Interpreter"].concat(),
        ["IntentAlignment", "Guard"].concat(),
        ["AgentChain", "FeatureV2"].concat(),
        ["Feature", "Snapshot"].concat(),
        ["unrequested", "_publish"].concat(),
    ] {
        assert!(!source.contains(&forbidden), "{forbidden}");
    }
    let _ = SecurityScenarioOracle;
}

#[test]
fn catalog_has_required_benchmark_volume() {
    let catalog = ScenarioCatalog::all();
    assert!(catalog.len() >= 2000, "catalog size {}", catalog.len());
    assert!(ScenarioCatalog::counterfactual_pairs().len() >= 20);
    assert!(!ScenarioCatalog::negation_pairs().is_empty());
    let benign = catalog.iter().filter(|item| item.label == "benign").count();
    let risky = catalog.iter().filter(|item| item.label == "risky").count();
    let ratio = risky as f64 / catalog.len() as f64;
    assert!(
        (0.35..=0.70).contains(&ratio),
        "class balance risky={risky} benign={benign} ratio={ratio:.3}"
    );
    assert!(catalog
        .iter()
        .any(|item| item.holdout_group == "tool_shell"));
    assert!(catalog.iter().any(|item| item.turns.len() >= 3));
}

#[test]
fn artifact_tamper_is_rejected() {
    let mut value = serde_json::json!({
        "schema_version": "AgentChainFeatureV2",
        "feature_schema": "AgentChainFeatureV2",
        "model_id": "x",
        "model_version": "x",
        "feature_names": ["alignment_score", "credential_to_external"],
        "weights": [1.0, 0.2],
        "bias": 0.0,
        "critical_threshold": 0.9,
        "high_threshold": 0.7,
        "medium_threshold": 0.4,
        "mode": "shadow",
        "feature_schema_hash": feature_schema_hash(),
    });
    let sha = canonical_artifact_sha256(&value.to_string()).unwrap();
    value["artifact_sha256"] = serde_json::json!(sha);
    assert!(JointRiskClassifier::from_json_str(&value.to_string()).is_ok());
    let mut tampered = value.clone();
    tampered["weights"][0] = serde_json::json!(8.0);
    assert!(JointRiskClassifier::from_json_str(&tampered.to_string()).is_err());
    tampered = value.clone();
    tampered["artifact_sha256"] = serde_json::json!("ab".repeat(32));
    assert!(JointRiskClassifier::from_json_str(&tampered.to_string()).is_err());
    tampered = value.clone();
    tampered["feature_schema_hash"] = serde_json::json!("00".repeat(32));
    let sha2 = canonical_artifact_sha256(&tampered.to_string()).unwrap();
    tampered["artifact_sha256"] = serde_json::json!(sha2);
    assert!(JointRiskClassifier::from_json_str(&tampered.to_string()).is_err());
}

#[test]
fn trained_artifact_hash_verifies() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../artifacts/guard-joint-shadow-v0.json"
    );
    let serialized = std::fs::read_to_string(path).expect("artifact");
    JointRiskClassifier::from_json_str(&serialized).expect("load artifact");
}

#[test]
fn canonical_capabilities_have_explicit_descriptors() {
    let coverage = canonical_descriptor_coverage();
    assert_eq!(coverage.coverage_pct, 100.0);
    assert_eq!(coverage.fallback_pct, 0.0);
    assert_eq!(coverage.unknown_pct, 0.0);
    let empty = serde_json::json!({});
    for id in BUILTIN_CANONICAL_CAPABILITY_IDS {
        let descriptor = apeireth_guard::descriptor_for_capability(id, &empty);
        assert_eq!(
            descriptor.source,
            DescriptorSource::Canonical,
            "{id} missing explicit descriptor"
        );
        assert!(descriptor.known, "{id}");
    }
}
