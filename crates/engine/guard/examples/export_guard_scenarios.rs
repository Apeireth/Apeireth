//! Generate FeatureV2 rows from the real Guard scenario pipeline.

use std::path::PathBuf;

use apeireth_guard::{run_scenario, ScenarioCatalog};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let catalog = ScenarioCatalog::all();
    let mut out = String::new();
    for scenario in &catalog {
        let outcome = run_scenario(scenario).await;
        let row = serde_json::json!({
            "id": outcome.id,
            "label": outcome.label,
            "family": outcome.family,
            "category": outcome.category,
            "language": outcome.language,
            "intent_class": format!("{:?}", outcome.intent_class),
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
    println!("wrote {} rows to {}", catalog.len(), dest.display());
}
