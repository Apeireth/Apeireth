//! `apeireth-evolution` demo — 6 状态机 + trait fail-6 + 与 apeireth-council 集成
//!
//! 运行: `cargo run -p apeireth-evolution --example evolution_demo`
//!
//! 演示路径:
//! 1. 启动提案 (Idle → Draft)
//! 2. 提交审议 (Draft → Proposed)
//! 3. 智囊团模拟审议 (all_approve → Ratified)
//! 4. 激活 (Ratified → Active)
//! 5. 4 trait 协同 (Learning / Abstraction / SelfModification / Extension)

use apeireth_council::CouncilEvent;
use apeireth_evolution::council_bridge::{
    all_approve_report, build_verdict_from_report, CouncilAdapter, CouncilIntegrationConfig,
    EvolutionOutcome, EvolutionProposal,
};
use apeireth_evolution::engine::EvolutionStep;
use apeireth_evolution::fail::StrictFailPolicy;
use apeireth_evolution::state::EvolutionState;
use apeireth_evolution::{
    Abstraction, BasicEvolution, Episode, EvolutionEngine, Extension, Learning, MockPlugin, Plugin,
    PluginKind, SelfModification, SystemState,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let now = apeireth_evolution::current_time_ms();

    println!("=== Apeireth 演化器官 6 状态机 + fail-6 demo (v1.0.0) ===\n");

    // ============================================================
    // 1. 启动提案 (Idle → Draft)
    // ============================================================
    let mut engine: EvolutionEngine = EvolutionEngine::new("demo-proposal-001", StrictFailPolicy);
    println!(
        "[1/6] 创建提案 — 初始状态: {}, proposal_id: {}",
        engine.current_state(),
        engine.log().proposal_id
    );

    engine.start(now)?;
    println!(
        "      启动 (Idle → Draft) — 当前: {}, log steps: {}",
        engine.current_state(),
        engine.log().steps.len()
    );

    // ============================================================
    // 2. 提交审议 (Draft → Proposed)
    // ============================================================
    engine.submit(now)?;
    println!(
        "[2/6] 提交审议 (Draft → Proposed) — 当前: {}",
        engine.current_state()
    );

    // ============================================================
    // 3. 智囊团审议 (合成 verdict → Ratified)
    // ============================================================
    let report = all_approve_report(now);
    let verdict = build_verdict_from_report(
        &CouncilEvent::DeliberationCompleted {
            session_id: "demo-session".into(),
            report: report.clone(),
            elapsed_ms: 120,
        },
        &report,
    );

    let outcome: EvolutionOutcome;
    {
        let mut adapter = CouncilAdapter::new(&mut engine, CouncilIntegrationConfig::default());
        outcome = adapter.handle_council_verdict(&verdict, now)?;
    }
    println!(
        "[3/6] 智囊团裁决 — outcome: {:?}, 当前: {}",
        outcome,
        engine.current_state()
    );

    // ============================================================
    // 4. 激活 (Ratified → Active)
    // ============================================================
    engine.activate(now)?;
    println!(
        "[4/6] 激活 (Ratified → Active) — 当前: {}, succeeded: {}",
        engine.current_state(),
        engine.log().succeeded
    );

    // ============================================================
    // 5. L0 防护演示 — 任何 L0 proposal 立即 Retired
    // ============================================================
    let mut l0_engine: EvolutionEngine = EvolutionEngine::new("demo-l0-block", StrictFailPolicy);
    l0_engine.start(now)?;
    {
        let mut adapter = CouncilAdapter::new(&mut l0_engine, CouncilIntegrationConfig::default());
        let l0_proposal = EvolutionProposal::new("l0", "try touching L0", "L0", "nuclear");
        let ok = adapter.guard_proposal(&l0_proposal, now)?;
        assert!(!ok, "L0 must be blocked");
    }
    println!(
        "[5/6] L0 防护演示 — 当前: {} (terminated), log has L0GuardTriggered: {}",
        l0_engine.current_state(),
        l0_engine
            .log()
            .steps
            .iter()
            .any(|s| matches!(s, EvolutionStep::L0GuardTriggered { .. }))
    );

    // ============================================================
    // 6. 4 trait 协同演示
    // ============================================================
    let mut evo = BasicEvolution::new();
    let episodes = vec![
        Episode::new("1", "auth.login", "verify", "ok", now),
        Episode::new("2", "auth.refresh", "verify", "ok", now),
        Episode::new("3", "auth.refresh", "verify", "ok", now),
    ];
    for ep in &episodes {
        evo.learn(ep)?;
    }
    let concept = evo.abstract_concept(&episodes).unwrap();
    let patch = evo.propose_patch(&SystemState::new("L3", 88));
    let p1: Box<dyn Plugin> = Box::new(MockPlugin::new("p1", PluginKind::Tool, "x"));
    let p2: Box<dyn Plugin> = Box::new(MockPlugin::new("s1", PluginKind::Sensor, "y"));
    evo.extend_capability(p1)?;
    evo.extend_capability(p2)?;

    println!(
        "[6/6] 4 trait 协同 — knowledge={:.3}, concept='{}' ({} examples), patch.risk={}, plugins={}",
        evo.knowledge_score(),
        concept.name,
        concept.example_count,
        patch.risk,
        evo.plugin_count()
    );

    println!(
        "\n=== 完成 — final state {}, succeeded: {} ===",
        engine.current_state(),
        engine.log().succeeded
    );
    println!(
        "L0 demo final state: {}, succeeded: {}",
        l0_engine.current_state(),
        l0_engine.log().succeeded
    );
    println!(
        "6 状态枚举 (compile-time hardcode): {:?}",
        EvolutionState::ALL
    );

    Ok(())
}
