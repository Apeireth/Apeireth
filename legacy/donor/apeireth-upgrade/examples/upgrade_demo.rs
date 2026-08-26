//! apeireth-upgrade demo — round10-01 演示完整 7 阶段 (Sandbox) 升级流程.
//!
//! 运行: `cargo run -p apeireth-upgrade --example upgrade_demo`

use apeireth_upgrade::{
    CouncilOpinion, CouncilReport, CouncilReviewer, CouncilSeat, CouncilStance, DefaultSandbox,
    FiveFoldGovernance, Governance, GovernanceDecision, IntentStateMachine, ManifestBuilder,
    MonitorDashboard, MonitorMetric, MultiSigCollector, MultiSigConfig, OtaPipeline, OtaStage,
    PhysicalSignature, SandboxValidator, SandboxVerdict, UpgradeIntent, UpgradeKind,
};
use uuid::Uuid;

fn main() {
    println!("=== apeireth-upgrade round10-01 7 阶段 (Sandbox) demo ===\n");

    // 场景 1: 7 阶段完整升级 (Patch)
    println!("[场景 1] 完整 7 阶段 Patch 升级 (v1.0.0 → v1.0.1)");
    let manifest = ManifestBuilder::new("v1.0.1", UpgradeKind::Patch)
        .with_description("demo patch upgrade")
        .with_content_hash("sha256:abc123def456")
        .build();
    match run_7_stage(manifest) {
        Ok((stage, report)) => {
            println!("  最终阶段 = {stage:?}");
            println!("  监控建议 = {:?}\n", report.recommendation);
        }
        Err(e) => println!("  ERR: {}\n", e),
    }

    // 场景 2: E 层修改 (默认保守拒绝)
    println!("[场景 2] E 层修改尝试 (默认保守拒绝)");
    let e_layer_manifest = ManifestBuilder::new("v2.0.0-e", UpgradeKind::ELayerMutation)
        .with_description("E-layer mutation attempt")
        .with_content_hash("sha256:e_layer_hash")
        .build();
    if e_layer_manifest.validate().is_err() {
        println!("  manifest 校验失败\n");
    } else {
        let sandbox = DefaultSandbox;
        let verdict = sandbox.validate(&e_layer_manifest);
        println!("  sandbox = {verdict:?}\n");
    }
    let gov = FiveFoldGovernance::default();
    let gov_decision = gov.evaluate(&e_layer_manifest);
    println!("  governance = {gov_decision:?}\n");

    // 场景 3: 7 席智囊团审议 + 按住机制
    println!("[场景 3] 7 席审议 (1 席强反对 -> 触发按住)");
    let intent = UpgradeIntent::new(
        Uuid::new_v4(),
        "v1.0.2",
        "v1.0.1",
        UpgradeKind::Minor,
        "carrier-a",
        "demo",
    );
    let mut opinions: Vec<CouncilOpinion> = CouncilSeat::ALL
        .iter()
        .map(|s| CouncilOpinion::new(*s, CouncilStance::Approve, 0.9, "ok"))
        .collect();
    opinions[2] = CouncilOpinion::new(
        CouncilSeat::Continuity,
        CouncilStance::StrongDisapprove,
        0.95,
        "violates 6-stream append-only invariant",
    );
    let report: CouncilReport = CouncilReviewer::new().review(&intent, opinions);
    println!("  审议通过 = {}", report.is_approved());
    println!("  反对比例 = {:.2}", report.disapprove_ratio());
    println!("  按住动作 = {:?}\n", report.hold);

    // 场景 4: 物理多签 (5-of-7)
    println!("[场景 4] 5-of-7 物理多签");
    let hash = apeireth_upgrade::intent_payload_hash(&intent);
    let cfg = MultiSigConfig::five_of_seven();
    let mut col = MultiSigCollector::new(cfg, hash.clone());
    for i in 0..5 {
        let sig = PhysicalSignature::new(
            format!("signer-{i}"),
            hash.clone(),
            100 + i,
            format!("sig-{i}"),
        );
        col.submit(sig).unwrap();
    }
    let outcome = col.evaluate(200);
    println!("  多签结果 = {outcome:?}");
    println!("  allows_proceed = {}\n", outcome.allows_proceed());

    // 场景 5: Monitor dashboard + Keep/Rollback 建议
    println!("[场景 5] Monitor dashboard (健康 -> Keep, 高错误率 -> Rollback)");
    let mut d = MonitorDashboard::new();
    d.register_smoke(Box::new(apeireth_upgrade::HealthSmoke));
    d.register_smoke(Box::new(apeireth_upgrade::ErrorRateSmoke {
        error_rate: 0.01,
    }));
    d.register_smoke(Box::new(apeireth_upgrade::LatencySmoke { p99_ms: 100.0 }));
    d.run_smokes();
    let report = d.report();
    println!("  健康监控建议 = {:?}", report.recommendation);
    println!(
        "  Failed 指标 = {}, Degraded = {}",
        report.failed_count, report.degraded_count
    );

    // 失败 case
    let mut d2 = MonitorDashboard::new();
    d2.register_smoke(Box::new(apeireth_upgrade::ErrorRateSmoke {
        error_rate: 0.30,
    }));
    d2.register_smoke(Box::new(apeireth_upgrade::LatencySmoke { p99_ms: 100.0 }));
    d2.run_smokes();
    let report2 = d2.report();
    println!("  高错误率建议 = {:?}\n", report2.recommendation);

    // 场景 6: Intent 状态机
    println!("[场景 6] UpgradeIntent 状态机");
    let intent2 = UpgradeIntent::new(
        Uuid::new_v4(),
        "v1.0.3",
        "v1.0.2",
        UpgradeKind::Patch,
        "carrier-b",
        "demo intent flow",
    );
    let mut sm = IntentStateMachine::wrap(intent2);
    println!("  initial = {:?}", sm.status());
    sm.submit().unwrap();
    println!("  after submit = {:?}", sm.status());
    sm.approve().unwrap();
    println!("  after approve = {:?}", sm.status());

    // 抑制 unused warning for imported types
    let _ = SandboxVerdict::Accept;
    let _ = GovernanceDecision::Accept;
}

/// 完整 7 阶段流 (helper).
fn run_7_stage(
    manifest: apeireth_upgrade::UpgradeManifest,
) -> Result<(OtaStage, apeireth_upgrade::MonitorReport), String> {
    let intent = UpgradeIntent::new(
        manifest.id,
        manifest.version.clone(),
        "v0.0.0".to_string(),
        manifest.kind,
        "demo",
        manifest.description.clone(),
    );
    let mut pipeline = OtaPipeline::new(OtaStage::Idle);
    pipeline
        .start_intent(intent.clone())
        .map_err(|e| e.to_string())?;

    let all_approve: Vec<CouncilOpinion> = CouncilSeat::ALL
        .iter()
        .map(|s| CouncilOpinion::new(*s, CouncilStance::Approve, 0.9, "demo-approve"))
        .collect();
    let report = CouncilReviewer::new().review(&intent, all_approve);
    pipeline
        .enter_council_review(report)
        .map_err(|e| e.to_string())?;

    let hash = apeireth_upgrade::intent_payload_hash(&intent);
    let cfg = MultiSigConfig::new(5, (0..7).map(|i| format!("signer-{i}")).collect());
    let mut col = MultiSigCollector::new(cfg, hash.clone());
    for i in 0..5 {
        col.submit(PhysicalSignature::new(
            format!("signer-{i}"),
            hash.clone(),
            100 + i,
            format!("sig-{i}"),
        ))
        .unwrap();
    }
    let outcome = col.evaluate(200);
    pipeline
        .enter_multisig(outcome)
        .map_err(|e| e.to_string())?;

    pipeline
        .enter_sandbox(
            manifest.id,
            "blue".into(),
            "green".into(),
            &manifest,
            &DefaultSandbox,
        )
        .map_err(|e| e.to_string())?;
    pipeline.enter_switchover().map_err(|e| e.to_string())?;

    let mut dashboard = MonitorDashboard::new();
    dashboard.record(MonitorMetric::new("a", 0.01, Some(0.05), None));
    dashboard.record(MonitorMetric::new("b", 100.0, Some(500.0), None));
    let monitor_report = dashboard.report();
    pipeline
        .enter_monitor(monitor_report.clone())
        .map_err(|e| e.to_string())?;
    let final_stage = pipeline
        .finalize(monitor_report.clone())
        .map_err(|e| e.to_string())?;
    Ok((final_stage, monitor_report))
}
