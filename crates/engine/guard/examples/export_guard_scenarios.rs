//! Generate desensitized FeatureV2, action, trace, and pair rows from the
//! real Guard scenario pipeline.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use apeireth_guard::{
    canonical_descriptor_coverage, family_split, feature_schema_hash, run_scenario, ScenarioCatalog,
};

fn write_jsonl(path: &PathBuf, rows: &[serde_json::Value]) {
    let mut output = String::new();
    for row in rows {
        output.push_str(&serde_json::to_string(row).expect("serialize row"));
        output.push('\n');
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create export directory");
    }
    std::fs::write(path, output).expect("write JSONL export");
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let catalog = ScenarioCatalog::all();
    let mut feature_rows = Vec::new();
    let mut action_rows = Vec::new();
    let mut trace_rows = Vec::new();
    let mut pair_members: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    let mut families = BTreeSet::new();
    let mut languages = BTreeMap::new();
    let mut labels = BTreeMap::new();

    for scenario in &catalog {
        families.insert(scenario.family.clone());
        *languages.entry(scenario.language.clone()).or_insert(0usize) += 1;
        *labels.entry(scenario.label.clone()).or_insert(0usize) += 1;
        let outcome = run_scenario(scenario).await;
        let split = family_split(&outcome.family);

        // Keep the historical aggregate export for compatibility. The new
        // action/trace exports below are the fidelity-preserving sources.
        feature_rows.push(serde_json::json!({
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
            "family_split": split,
            "feature_snapshot_id": outcome.snapshot.snapshot_id,
            "features": outcome.snapshot.features,
        }));

        for action in &outcome.execution_trace.actions {
            let action_row = serde_json::json!({
                "scenario_id": outcome.id,
                "family": outcome.family,
                "category": outcome.category,
                "language": outcome.language,
                "family_split": split,
                "intent_template_id": outcome.intent_template_id,
                "action_template_id": outcome.action_template_id,
                "tool_origin": outcome.tool_origin,
                "pair_id": outcome.pair_id,
                "turn_index": action.turn_index,
                "trace_id": action.trace_id,
                "action_id": action.action_id,
                "capability": action.capability,
                "tool_name": action.tool_name,
                "intent_id": action.intent.intent_id,
                "intent_class": format!("{:?}", action.intent.intent_class),
                "intent_confidence": action.intent.confidence,
                "runtime_effects": action.runtime_effects,
                "expected_effects": action.expected_effects,
                "effects_reconciled": action.effects_reconciled,
                "missing_effects": action.missing_effects,
                "unexpected_effects": action.unexpected_effects,
                "oracle_label": action.oracle_label,
                "decision": action.decision,
                "risk_score": action.risk_score,
                "snapshot_id": action.snapshot.snapshot_id,
                "features": action.snapshot.features,
            });
            if let Some(pair_id) = &outcome.pair_id {
                pair_members
                    .entry(pair_id.clone())
                    .or_default()
                    .push(action_row.clone());
            }
            action_rows.push(action_row);
        }

        let trace = &outcome.execution_trace;
        let trace_row = serde_json::json!({
            "scenario_id": outcome.id,
            "session_id": trace.session_id,
            "family": outcome.family,
            "category": outcome.category,
            "language": outcome.language,
            "family_split": split,
            "intent_template_id": outcome.intent_template_id,
            "action_template_id": outcome.action_template_id,
            "tool_origin": outcome.tool_origin,
            "pair_id": outcome.pair_id,
            "effects_reconciled": trace.effects_reconciled,
            "turns": trace.turns.iter().map(|turn| serde_json::json!({
                "turn_index": turn.turn_index,
                "trace_id": turn.trace_id,
                "intent_id": turn.intent.intent_id,
                "intent_class": format!("{:?}", turn.intent.intent_class),
                "intent_confidence": turn.intent.confidence,
                "session_history": turn.session_history,
                "actions": turn.actions.iter().map(|action| serde_json::json!({
                    "action_id": action.action_id,
                    "capability": action.capability,
                    "tool_name": action.tool_name,
                    "snapshot_id": action.snapshot.snapshot_id,
                    "oracle_class": action.oracle_label.class,
                    "risk_score": action.risk_score,
                    "effects_reconciled": action.effects_reconciled,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        });
        trace_rows.push(trace_row);
    }

    let pair_rows: Vec<serde_json::Value> = pair_members
        .into_iter()
        .map(|(pair_id, members)| {
            let member_splits: BTreeSet<String> = members
                .iter()
                .filter_map(|member| member.get("family_split").and_then(|value| value.as_str()))
                .map(str::to_string)
                .collect();
            serde_json::json!({
                "pair_id": pair_id,
                "member_count": members.len(),
                "same_split": member_splits.len() <= 1,
                "splits": member_splits,
                "members": members,
            })
        })
        .collect();

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let generated_dir = root.join("../../../scripts/guard_ml");
    write_jsonl(
        &generated_dir.join("generated_features.jsonl"),
        &feature_rows,
    );
    write_jsonl(&generated_dir.join("action_samples.jsonl"), &action_rows);
    write_jsonl(&generated_dir.join("trace_samples.jsonl"), &trace_rows);
    write_jsonl(&generated_dir.join("pair_samples.jsonl"), &pair_rows);

    let coverage = canonical_descriptor_coverage();
    let manifest = serde_json::json!({
        "dataset_id": "guard-scenario-catalog-v3.3",
        "dataset_version": "v3.3",
        "scenario_count": catalog.len(),
        "action_count": action_rows.len(),
        "trace_count": trace_rows.len(),
        "pair_count": pair_rows.len(),
        "family_count": families.len(),
        "language_distribution": languages,
        "class_balance": labels,
        "feature_schema_hash": feature_schema_hash(),
        "descriptor_coverage": coverage,
        "exports": ["generated_features.jsonl", "action_samples.jsonl", "trace_samples.jsonl", "pair_samples.jsonl"],
        "privacy": {
            "raw_prompt": false,
            "raw_command": false,
            "raw_path": false,
            "raw_url": false,
            "tool_output": false,
            "memory_body": false,
            "secrets": false,
            "credentials": false,
            "tokens": false,
            "passwords": false,
            "chain_of_thought": false,
        },
        "note": "Generated by the Rust scenario exporter; provenance hashes and repository-relative training identifiers are completed by train_eval.py.",
    });
    let manifest_path = root.join("../../../artifacts/guard-dataset-manifest.json");
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent).expect("create artifact directory");
    }
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("manifest"),
    )
    .expect("write manifest");

    let pair_dir = root.join("../../../scripts/guard_ml");
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
    println!(
        "wrote {} scenarios, {} actions, {} traces, and {} pairs",
        catalog.len(),
        action_rows.len(),
        trace_rows.len(),
        pair_rows.len()
    );
}
