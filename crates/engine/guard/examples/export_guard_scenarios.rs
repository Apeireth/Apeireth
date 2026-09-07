//! Generate FeatureV2 rows from the real Guard scenario pipeline.

use std::path::PathBuf;

use apeireth_guard::{
    canonical_descriptor_coverage, family_split, feature_schema_hash, run_scenario, ScenarioCatalog,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let catalog = ScenarioCatalog::all();
    let mut out = String::new();
    let mut families = std::collections::BTreeSet::new();
    let mut languages = std::collections::BTreeMap::new();
    let mut labels = std::collections::BTreeMap::new();
    for scenario in &catalog {
        families.insert(scenario.family.clone());
        *languages.entry(scenario.language.clone()).or_insert(0usize) += 1;
        *labels.entry(scenario.label.clone()).or_insert(0usize) += 1;
        let outcome = run_scenario(scenario).await;
        let row = serde_json::json!({
            "id": outcome.id,
            "label": outcome.label,
            "family": outcome.family,
            "category": outcome.category,
            "language": outcome.language,
            "intent_class": format!("{:?}", outcome.intent_class),
            "intent_template_id": outcome.intent_template_id,
            "action_template_id": outcome.action_template_id,
            "tool_origin": outcome.tool_origin,
            "holdout_group": outcome.holdout_group,
            "benchmarks": outcome.benchmarks,
            "pair_id": outcome.pair_id,
            "family_split": family_split(&outcome.family),
            "feature_snapshot_id": outcome.snapshot.snapshot_id,
            "features": outcome.snapshot.features,
        });
        out.push_str(&serde_json::to_string(&row).expect("serialize row"));
        out.push('\n');
    }
    let dest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../scripts/guard_ml/generated_features.jsonl");
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&dest, out).expect("write generated features");
    let coverage = canonical_descriptor_coverage();
    let manifest = serde_json::json!({
        "dataset_id": "guard-scenario-catalog-v3.2",
        "dataset_version": "v3.2",
        "scenario_count": catalog.len(),
        "family_count": families.len(),
        "language_distribution": languages,
        "class_balance": labels,
        "feature_schema_hash": feature_schema_hash(),
        "descriptor_coverage": coverage,
        "note": "scenario_catalog_hash and generator_commit are filled by train_eval.py",
    });
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../artifacts/guard-dataset-manifest.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest"),
    )
    .expect("write manifest");
    let pair_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../scripts/guard_ml");
    std::fs::write(
        pair_dir.join("counterfactual_pairs.json"),
        serde_json::to_string_pretty(&ScenarioCatalog::counterfactual_pairs()).expect("pairs"),
    )
    .expect("write counterfactual pairs");
    std::fs::write(
        pair_dir.join("negation_pairs.json"),
        serde_json::to_string_pretty(&ScenarioCatalog::negation_pairs()).expect("pairs"),
    )
    .expect("write negation pairs");
    println!("wrote {} rows to {}", catalog.len(), dest.display());
}
