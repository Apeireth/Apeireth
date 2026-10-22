//! 生物机制族 W3.2 装配面: 群体感应审议门 (quorum gate) 接线测试.
//!
//! 覆盖装配级契约 (不是 W3.1 内核单测的重复):
//! 1. env 开关解析: 仅 `"1"` 开, 默认关 fail-closed (`APEIRETH_ENABLE_QUORUM_GATE`);
//! 2. 默认关: 走原 `decide_with_invoker` 路径, 输出逐字节一致 + 门账为空 + 0 信号;
//! 3. 升议 (`Escalate`) → 召开, 原 result 继续, `escalation.reasons` 上门账且不泄露用户原文;
//! 4. `Hold` (未达阈值) → 不召开: advisor 调用计数 0, tick 以 `Held(GateBlock)` 留痕;
//! 5. `Cooldown` (不应期) → 不召开: 新信号也不得再调 advisor;
//! 6. 同一波信号第二次被拦: 无冷却时第二次 = `Hold` (消费后清零, 不重复开会);
//! 7. 确定性: `council_deliberate_at` / tick 的 `now_ms` 全注入, 同输入 = 同门账;
//! 8. 器官链故障 → `Anomaly` 信号 (装配面自带生产信号源 ①);
//! 9. 已召开的否决仍是 `CouncilVeto` (与"未召开"的 `GateBlock` 区分, 不混淆语义);
//! 10. `Retry` → `Uncertainty` 证据回流 (装配面自带生产信号源 ②, 不改既有获准语义).
//!
//! **0 装诚实**: 门默认关; 通过 `with_quorum_gate` 显式注入, 不依赖进程 env, 因此测试
//! 之间无环境变量竞争. 外部事件源 (工具失败 / 守护告警 / 用户不信任) 尚未接线, 走
//! `push_quorum_signal` 显式注入口 —— 生产事件源待 W3.3.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::{Clock, Episode, SessionId};
use apeireth_orchestration::council::QuorumGate;
use apeireth_orchestration::quorum::{
    QuorumAccumulator, QuorumConfig, QuorumDecision, QuorumSignal, QuorumSignalKind,
};
use apeireth_orchestration::{
    Advisor, AdvisorDecision, AdvisorVerdict, Council, CouncilCallError, CouncilInvoker, Proposal,
};
use apeireth_plugin::organ::{OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait};
use apeireth_runtime_assembly::canonical::orchestrator::{
    quorum_gate_enabled_from_env_value, quorum_gate_from_env_value, quorum_now_ms_from_i64,
    LocalOrchestratorRelationship, LocalSovereignty, OrchestratorBoundaries, OrchestratorDecision,
    OrchestratorLoopConfig, OrganOrchestrator, OrganOrchestratorGate, OrganTickInput, QuorumTrace,
    SovereigntyGate, QUORUM_GATE_ENABLE_ENV,
};
use chrono::TimeZone;

type TestOrchestrator = OrganOrchestrator<LocalOrchestratorRelationship>;

// ============================================
// 测试替身: 9 organ mock / 计数 invoker / 固定时钟
// ============================================

struct MockOrgan {
    kind: OrganKind,
    failing: bool,
}

#[async_trait::async_trait]
impl OrganTrait for MockOrgan {
    fn name(&self) -> &'static str {
        "MockOrgan(bio_quorum_wiring)"
    }

    fn organ_id(&self) -> OrganKind {
        self.kind
    }

    async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
        if self.failing {
            return Err(OrganError::Internal(
                "mock organ failure (bio_quorum_wiring)".to_string(),
            ));
        }
        Ok(OrganOutput::NotImplemented {
            organ: self.kind,
            note: "mock organ (bio_quorum_wiring)".to_string(),
        })
    }
}

/// 计数 invoker: 每次 `invoke` = 一次 advisor side-call.
struct CountingInvoker {
    calls: Arc<AtomicUsize>,
    verdict: AdvisorDecision,
}

#[async_trait::async_trait]
impl CouncilInvoker for CountingInvoker {
    async fn invoke(
        &self,
        _advisor: Arc<dyn Advisor>,
        _proposal: &Proposal,
    ) -> Result<AdvisorVerdict, CouncilCallError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let score = if self.verdict == AdvisorDecision::Allow {
            1.0
        } else {
            0.0
        };
        AdvisorVerdict::new(score, self.verdict, "counting invoker", None)
            .map_err(CouncilCallError::Malformed)
    }
}

fn counting_invoker(verdict: AdvisorDecision) -> (Arc<dyn CouncilInvoker>, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    (
        Arc::new(CountingInvoker {
            calls: Arc::clone(&calls),
            verdict,
        }),
        calls,
    )
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
            .single()
            .expect("fixed virtual clock"),
    ))
}

fn make_episode() -> Episode {
    Episode {
        id: "ep-bio-quorum".into(),
        session_id: "sess-bio-quorum".into(),
        role: "user".into(),
        content: "user text that must never leak into quorum reasons".into(),
        timestamp: 0,
    }
}

fn make_tick_input(at_ms: i64) -> OrganTickInput {
    OrganTickInput {
        at_ms,
        minutes_of_day: ((at_ms.rem_euclid(86_400_000)) / 60_000) as u32,
        day_key: format!("tick-{}", at_ms.div_euclid(86_400_000)),
        context_hint: Some("你好".into()),
        episode: make_episode(),
        session_id: "sess-bio-quorum".into(),
    }
}

fn proposal() -> Proposal {
    Proposal {
        id: "p-bio-quorum".into(),
        proposer: "bio-quorum-test".into(),
        payload: serde_json::json!({
            "action": "proactive_contact",
            "user_text": "SECRET-USER-TEXT-MUST-NOT-LEAK",
        }),
        submitted_at: 1_767_268_800,
        session_id: SessionId::default(),
    }
}

fn build_with_failing(
    gate: QuorumGate,
    invoker: Arc<dyn CouncilInvoker>,
    failing: &[OrganKind],
) -> TestOrchestrator {
    let make = |kind: OrganKind| -> Arc<dyn OrganTrait> {
        Arc::new(MockOrgan {
            kind,
            failing: failing.contains(&kind),
        })
    };
    let sovereignty: Arc<parking_lot::Mutex<dyn SovereigntyGate>> =
        Arc::new(parking_lot::Mutex::new(LocalSovereignty::default()));
    OrganOrchestrator::new(
        make(OrganKind::E4),
        make(OrganKind::F1),
        make(OrganKind::F4),
        make(OrganKind::F6),
        make(OrganKind::W1),
        make(OrganKind::W2),
        make(OrganKind::W3),
        make(OrganKind::E7),
        make(OrganKind::Memory),
        Arc::new(Council::default_allow()),
        invoker,
        sovereignty,
        LocalOrchestratorRelationship::new(0.5),
        OrchestratorBoundaries::default(),
        OrchestratorLoopConfig::default(),
        clock(),
    )
    .with_quorum_gate(gate)
}

fn build(gate: QuorumGate, invoker: Arc<dyn CouncilInvoker>) -> TestOrchestrator {
    build_with_failing(gate, invoker, &[])
}

fn config(threshold: f32, decay_per_second: f32, cooldown_ms: u64) -> QuorumConfig {
    QuorumConfig {
        threshold,
        decay_per_second,
        cooldown_ms,
        max_accumulation: 10.0,
    }
}

fn enabled_gate(config: QuorumConfig) -> QuorumGate {
    QuorumGate::enabled(QuorumAccumulator::new(config))
}

fn risk_signal(weight: f32, at_ms: u64) -> QuorumSignal {
    QuorumSignal::weighted(
        QuorumSignalKind::RiskEscalation,
        weight,
        at_ms,
        "test.risk_escalation",
    )
}

// ============================================
// 1. env 开关解析 (默认关 fail-closed)
// ============================================

#[test]
fn quorum_env_switch_parses_only_explicit_one() {
    assert_eq!(QUORUM_GATE_ENABLE_ENV, "APEIRETH_ENABLE_QUORUM_GATE");

    // 默认关: 未设置 / 空 / 任何非 "1" 取值都不开
    assert!(!quorum_gate_enabled_from_env_value(None), "未设置 = 关");
    for raw in ["", "0", "true", "yes", "on", "01", "1.0", " 0 ", "2"] {
        assert!(
            !quorum_gate_enabled_from_env_value(Some(raw)),
            "{raw:?} 不得开启 (只认 \"1\", fail-closed)"
        );
    }
    // 仅 "1" 开 (允许两端空白)
    assert!(quorum_gate_enabled_from_env_value(Some("1")));
    assert!(quorum_gate_enabled_from_env_value(Some(" 1 ")));

    // 构造面同样 fail-closed
    assert!(!quorum_gate_from_env_value(None).is_enabled());
    assert!(!quorum_gate_from_env_value(Some("true")).is_enabled());
    assert!(quorum_gate_from_env_value(Some("1")).is_enabled());

    // 时钟注入消毒: 负 epoch 归零, 不 panic / 不倒流
    assert_eq!(quorum_now_ms_from_i64(-1), 0);
    assert_eq!(quorum_now_ms_from_i64(0), 0);
    assert_eq!(quorum_now_ms_from_i64(1_700_000_000_123), 1_700_000_000_123);
}

// ============================================
// 2. 默认关: 原路径逐字节一致 + 门账为空
// ============================================

#[tokio::test]
async fn default_off_is_byte_identical_to_the_legacy_path() {
    let (invoker_a, calls_a) = counting_invoker(AdvisorDecision::Allow);
    let (invoker_b, calls_b) = counting_invoker(AdvisorDecision::Allow);
    let mut a = build(QuorumGate::disabled(), invoker_a);
    let mut b = build(QuorumGate::disabled(), invoker_b);

    // 显式注入的 disabled 门覆盖 new() 时的 env 读取 (env 只在构造时读一次)
    assert!(!a.quorum_gate_enabled());
    assert!(!b.quorum_gate_enabled());

    // 审议: 两个独立实例同输入 → 同输出, 且 advisor 全走 (7 个, 原路径)
    let da = a.council_deliberate_at(&proposal(), 1_000).await;
    let db = b.council_deliberate_at(&proposal(), 1_000).await;
    assert_eq!(format!("{da:?}"), format!("{db:?}"), "默认关输出逐字节一致");
    assert!(da.expect("legacy path never errors"));
    assert_eq!(calls_a.load(Ordering::SeqCst), 7, "7 advisor 走原路径");
    assert_eq!(calls_b.load(Ordering::SeqCst), 7);

    // 门在默认关时一格也不记: 不读门账 / 不攒信号
    assert!(a.last_quorum_trace().is_none(), "默认关门账必须为空");
    assert!(a.quorum_signals().await.is_empty(), "默认关不记任何信号");

    // tick 全程: 输出逐字节一致 + 既有留痕 (Spoke) 不变
    let ta = a.tick(make_tick_input(1_000_000)).await;
    let tb = b.tick(make_tick_input(1_000_000)).await;
    assert_eq!(format!("{ta:?}"), format!("{tb:?}"), "tick 输出逐字节一致");
    assert_eq!(
        format!("{:?}", a.last_decision()),
        format!("{:?}", b.last_decision())
    );
    assert!(ta.is_some(), "默认关 = 原路径, 应照常开口");
    assert!(
        matches!(a.last_decision(), Some(OrchestratorDecision::Spoke { .. })),
        "默认关留痕仍是 Spoke, got {:?}",
        a.last_decision()
    );
    assert!(a.last_quorum_trace().is_none(), "tick 后门账仍必须为空");
    assert!(a.quorum_signals().await.is_empty());
}

// ============================================
// 3. Escalate: 召开 + reasons 上门账 (不泄露用户原文)
// ============================================

#[tokio::test]
async fn escalate_convenes_and_records_bounded_reasons() {
    let (invoker, calls) = counting_invoker(AdvisorDecision::Allow);
    let orch = build(enabled_gate(config(0.6, 0.0, 600_000)), invoker);
    assert!(orch.quorum_gate_enabled());

    orch.push_quorum_signal(risk_signal(0.6, 1_000), 1_000)
        .await;
    assert!(orch.quorum_score(1_000).await >= 0.6, "证据应已入账");

    let approved = orch
        .council_deliberate_at(&proposal(), 1_000)
        .await
        .expect("gated path never errors");
    assert!(approved, "升议 + allow invoker → 获准");
    assert_eq!(calls.load(Ordering::SeqCst), 7, "召开才调 advisor");

    let trace: QuorumTrace = orch.last_quorum_trace().expect("升议必须留门账");
    assert!(trace.convened, "升议 = 召开");
    assert_eq!(trace.at_ms, 1_000, "now_ms 注入原样落账");
    assert!(trace.denial.is_none());
    let reasons = trace.reasons();
    assert_eq!(reasons.len(), 1, "一条信号 = 一条有界理由");
    assert!(
        reasons[0].contains("risk_escalation"),
        "理由带稳定机器可读类别: {}",
        reasons[0]
    );
    assert!(
        reasons[0].contains("weight="),
        "理由带衰减后权重: {}",
        reasons[0]
    );
    // 不泄露用户原文 / 提案 payload
    let line = trace.summary_line();
    for needle in ["SECRET", "USER-TEXT", "user text that must never leak"] {
        assert!(!line.contains(needle), "门账不得含用户原文: {line}");
        for reason in reasons {
            assert!(!reason.contains(needle), "理由不得含用户原文: {reason}");
        }
    }
}

// ============================================
// 4. Hold: 不召开, advisor 调用计数 0, tick = Held(GateBlock)
// ============================================

#[tokio::test]
async fn hold_never_convenes_or_calls_an_advisor() {
    let (invoker, calls) = counting_invoker(AdvisorDecision::Allow);
    let orch = build(enabled_gate(config(1.0, 0.0, 600_000)), invoker);

    let approved = orch
        .council_deliberate_at(&proposal(), 5_000)
        .await
        .expect("hold is not an error");
    assert!(!approved, "未达升议阈值 = 未获准");
    assert_eq!(calls.load(Ordering::SeqCst), 0, "不召开: 0 次 advisor 调用");

    let trace = orch.last_quorum_trace().expect("拒绝也要留门账");
    assert!(!trace.convened);
    assert!(trace.escalation.is_none(), "未召开不得伪造升议快照");
    assert!(
        matches!(trace.denial, Some(QuorumDecision::Hold { .. })),
        "拒绝原因应是 Hold: {:?}",
        trace.denial
    );

    // 整 tick 装配路径: 未召开 → Held(GateBlock) (不是 CouncilVeto), advisor 仍 0
    let (invoker2, calls2) = counting_invoker(AdvisorDecision::Allow);
    let mut orch2 = build(enabled_gate(config(1.0, 0.0, 600_000)), invoker2);
    let outcome = orch2.tick(make_tick_input(1_000_000)).await;
    assert!(outcome.is_none(), "未召开不得开口");
    assert_eq!(calls2.load(Ordering::SeqCst), 0, "tick 路径同样 0 次调用");
    assert_eq!(
        orch2.last_decision(),
        Some(&OrchestratorDecision::Held(
            OrganOrchestratorGate::GateBlock
        )),
        "未召开 = 被上层 gate 拦下, 不写成 CouncilVeto"
    );
}

// ============================================
// 5. Cooldown: 不应期内不再召开
// ============================================

#[tokio::test]
async fn cooldown_blocks_the_second_wave() {
    let (invoker, calls) = counting_invoker(AdvisorDecision::Allow);
    let orch = build(enabled_gate(config(0.6, 0.0, 600_000)), invoker);

    orch.push_quorum_signal(risk_signal(0.6, 1_000), 1_000)
        .await;
    assert!(orch
        .council_deliberate_at(&proposal(), 1_000)
        .await
        .unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 7);
    assert_eq!(
        orch.quorum_cooldown_until_ms().await,
        Some(601_000),
        "升议后进入 600s 不应期"
    );

    // 冷却期内即使新证据过阈值也只报 Cooldown (优先级: 冷却 > 浓度)
    orch.push_quorum_signal(risk_signal(0.6, 2_000), 2_000)
        .await;
    assert!(!orch
        .council_deliberate_at(&proposal(), 2_000)
        .await
        .unwrap());
    assert_eq!(
        calls.load(Ordering::SeqCst),
        7,
        "冷却期不得再调 advisor (计数不涨)"
    );
    let trace = orch.last_quorum_trace().expect("冷却留门账");
    assert!(!trace.convened);
    assert!(
        matches!(trace.denial, Some(QuorumDecision::Cooldown { .. })),
        "拒绝原因应是 Cooldown: {:?}",
        trace.denial
    );
}

// ============================================
// 6. 同一波信号第二次被拦 (消费后清零, 无冷却时是 Hold)
// ============================================

#[tokio::test]
async fn consumed_wave_never_convenes_twice() {
    let (invoker, calls) = counting_invoker(AdvisorDecision::Allow);
    let orch = build(enabled_gate(config(0.6, 0.0, 0)), invoker);

    orch.push_quorum_signal(risk_signal(0.6, 1_000), 1_000)
        .await;
    assert!(orch
        .council_deliberate_at(&proposal(), 1_000)
        .await
        .unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 7);
    assert!(
        orch.quorum_signals().await.is_empty(),
        "升议兑现 = 信号被 consume 清零"
    );

    // 同一时刻第二次: 无新证据 → Hold (若配置了冷却则是 Cooldown, 见上一个测试)
    assert!(!orch
        .council_deliberate_at(&proposal(), 1_000)
        .await
        .unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 7, "同一波信号不得重复召集");
    assert!(matches!(
        orch.last_quorum_trace().unwrap().denial,
        Some(QuorumDecision::Hold { .. })
    ));
}

// ============================================
// 7. 确定性: now_ms 全注入, 同输入 = 同门账; 时间推进 = 衰减
// ============================================

#[tokio::test]
async fn injected_now_is_deterministic_and_decays() {
    let gate_a = enabled_gate(config(0.6, 0.0, 600_000));
    let gate_b = enabled_gate(config(0.6, 0.0, 600_000));
    let (invoker_a, _) = counting_invoker(AdvisorDecision::Allow);
    let (invoker_b, _) = counting_invoker(AdvisorDecision::Allow);
    let a = build(gate_a, invoker_a);
    let b = build(gate_b, invoker_b);

    for orch in [&a, &b] {
        orch.push_quorum_signal(risk_signal(0.6, 10_000), 10_000)
            .await;
        assert!(orch
            .council_deliberate_at(&proposal(), 10_000)
            .await
            .unwrap());
    }
    assert_eq!(
        format!("{:?}", a.last_quorum_trace()),
        format!("{:?}", b.last_quorum_trace()),
        "同信号 + 同 now_ms → 门账逐字节同一"
    );

    // 同一份证据在 10s 后 (衰减 0.05/s) 只剩 0.1 < 阈值 0.6 → Hold
    let gate_c = enabled_gate(config(0.6, 0.05, 0));
    let (invoker_c, calls_c) = counting_invoker(AdvisorDecision::Allow);
    let c = build(gate_c, invoker_c);
    c.push_quorum_signal(risk_signal(0.6, 0), 0).await;
    assert!((c.quorum_score(10_000).await - 0.1).abs() < 1e-6);
    assert!(!c.council_deliberate_at(&proposal(), 10_000).await.unwrap());
    assert_eq!(calls_c.load(Ordering::SeqCst), 0);
    assert!(matches!(
        c.last_quorum_trace().unwrap().denial,
        Some(QuorumDecision::Hold { .. })
    ));
}

// ============================================
// 8. 生产信号源 ①: 器官链故障 → Anomaly → 当 tick 升议
// ============================================

#[tokio::test]
async fn organ_chain_failures_feed_anomaly_signals() {
    // 4 个 organ 失败 × Anomaly 0.3 = 1.2 ≥ 阈值 1.0
    let (invoker, calls) = counting_invoker(AdvisorDecision::Allow);
    let mut orch = build_with_failing(
        enabled_gate(config(1.0, 0.0, 600_000)),
        invoker,
        &[OrganKind::E4, OrganKind::F1, OrganKind::F4, OrganKind::F6],
    );
    assert_eq!(orch.quorum_score(1_000_000).await, 0.0, "tick 前无信号");

    let outcome = orch.tick(make_tick_input(1_000_000)).await;
    assert!(
        outcome.is_some(),
        "器官链异常证据够阈值 → 本次 tick 升议并召开 → 开口"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 7, "召开后 advisor 全走");

    let trace = orch.last_quorum_trace().expect("升议留门账");
    assert!(trace.convened);
    assert!(
        trace.reasons().iter().any(|r| r.contains("anomaly")),
        "升议理由应含 Anomaly 类别: {:?}",
        trace.reasons()
    );
    assert!(
        trace
            .reasons()
            .iter()
            .any(|r| r.contains("orchestrator.organ_chain")),
        "来源应标器官链: {:?}",
        trace.reasons()
    );
    assert_eq!(
        orch.quorum_score(1_000_000).await,
        0.0,
        "升议兑现后信号清零"
    );
}

// ============================================
// 9. 已召开的否决仍是 CouncilVeto (与"未召开"语义分开)
// ============================================

#[tokio::test]
async fn convened_veto_stays_council_veto() {
    let (invoker, calls) = counting_invoker(AdvisorDecision::Stop);
    let mut orch = build(enabled_gate(config(0.6, 0.0, 600_000)), invoker);
    orch.push_quorum_signal(risk_signal(0.6, 1_000), 1_000)
        .await;

    let outcome = orch.tick(make_tick_input(1_000)).await;
    assert!(outcome.is_none(), "智囊团否决 → 不开口");
    assert_eq!(calls.load(Ordering::SeqCst), 7, "已召开: advisor 真被调用");
    assert_eq!(
        orch.last_decision(),
        Some(&OrchestratorDecision::Held(
            OrganOrchestratorGate::CouncilVeto
        )),
        "已开会的否决仍是 CouncilVeto (不降级成 GateBlock)"
    );
    let trace = orch.last_quorum_trace().expect("已召开留门账");
    assert!(trace.convened, "否决 ≠ 未召开");
    assert!(trace.escalation.is_some(), "本次确实兑现了升议");
    assert!(trace.denial.is_none());
}

// ============================================
// 10. 生产信号源 ②: Retry → Uncertainty 证据回流 (不改既有"获准"语义)
// ============================================

#[tokio::test]
async fn retry_feeds_uncertainty_evidence() {
    let (invoker, calls) = counting_invoker(AdvisorDecision::Retry);
    let orch = build(enabled_gate(config(0.6, 0.0, 600_000)), invoker);
    orch.push_quorum_signal(risk_signal(0.6, 1_000), 1_000)
        .await;

    assert!(
        orch.council_deliberate_at(&proposal(), 1_000)
            .await
            .unwrap(),
        "Retry 仍按既有语义算获准 (仅留 retry feedback)"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 7);

    // 升议信号已消费, 只剩审议后回流的一条 uncertainty
    let signals = orch.quorum_signals().await;
    assert_eq!(signals.len(), 1, "本次升议被 consume 清零");
    assert_eq!(signals[0].kind, QuorumSignalKind::Uncertainty);
    assert_eq!(signals[0].source, "orchestrator.council.retry");
    assert_eq!(signals[0].at_ms, 1_000);
}
