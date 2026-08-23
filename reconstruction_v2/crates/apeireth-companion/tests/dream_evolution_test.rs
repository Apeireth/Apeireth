use apeireth_companion::dream::DreamEngine;
use apeireth_companion::epistemic::EpistemicHealer;

#[test]
fn test_p9_dream_and_evolution_cycle() {
    let mut dream_engine = DreamEngine::new(0.80, 1800);

    let raw_conversations = vec![
        "User: My name is Jimmy and I develop distributed systems in Rust. | Apeireth: Hello Jimmy! Rust provides memory safety.".into(),
        "User: I like high-concurrency async Tokio. | Apeireth: Tokio is excellent for fast async execution.".into(),
        "User: Apeireth is a sovereign companion. | Apeireth: I protect your boundaries.".into(),
    ];

    let unresolved_episodes = vec![
        ("ep_timeout_42".into(), "network_fetch_delayed".into()),
    ];

    let brier_evaluations = vec![
        (0.95, true),
        (0.80, true),
        (0.10, false),
        (0.30, false),
    ];

    let report = dream_engine.run_nightly_evolution(&raw_conversations, &unresolved_episodes, &brier_evaluations);

    println!("=== Phase P9 Nightly Dream & Evolution Report ===");
    println!("Extracted (S, P, O) Triplet Count: {}", report.extracted_triplets.len());
    for t in &report.extracted_triplets {
        println!("  - [{}] --({})--> [{}] (conf: {:.2})", t.subject, t.predicate, t.object, t.confidence);
    }
    println!("Memories Compressed: {}", report.memories_compressed_count);
    println!("Tombstones Pruned: {}", report.tombstones_pruned_count);
    println!("W3 Counterfactual Rehearsals: {}", report.rehearsals.len());
    for r in &report.rehearsals {
        println!("  - Episode {}: Original [{}] -> Alternative [{}] (Reward Gain: +{:.2})",
            r.episode_id, r.original_action, r.counterfactual_alternative, r.expected_reward_gain);
    }
    println!("Brier Score (30-round): {:.4}", report.brier_score_30);
    println!("Intent Calibration Status: {}", report.intent_calibrated);
    println!("Post-Sleep Restored Drive Pressure: {:.2}", report.sleep_pressure_after);

    assert!(!report.extracted_triplets.is_empty());
    assert_eq!(report.rehearsals.len(), 1);
    assert!(report.brier_score_30 < 0.20, "Well-calibrated predictions must achieve Brier score < 0.20");
    assert_eq!(report.sleep_pressure_after, 0.15);
}

#[test]
fn test_epistemic_self_repair_distillation() {
    let mut healer = EpistemicHealer::new();

    let inc = healer.distill_failure("fs_write", "Sandbox error: Path traversal (..) is strictly prohibited");
    assert_eq!(inc.action_name, "fs_write");
    assert!(inc.preventative_anchor.contains("sandbox jail"));

    let anchors = healer.get_preventative_anchors();
    assert_eq!(anchors.len(), 1);
}
