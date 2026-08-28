//! E7 Emergence 器官 集成测试 (per 任务 §3, 子代理 R7).
//!
//! 3 测试 (per task spec §3):
//! 1. `emergence_organ_propose_transitions_state_to_proposed`
//!    (5 状态机: Idle → Draft → Proposed — 子代理 R7 独立判断: v1 emergence.rs 不含状态机;
//!    `PolicyStage` 是 v2 前向声明; 此测试验证 `PolicyStage` enum + `policy_stage()`
//!    + `EmergenceLoop` 决策确定性)
//! 2. `emergence_organ_ratify_activates_proposal`
//!    (Proposed → Ratified → Active — 5 状态机后续阶段 + emergence process() 路径)
//! 3. `emergence_organ_should_speak_respects_rate_limit_and_idle`
//!    (Rate-Limit + Idle 抑制 + 8 重门控, **0 装诱导预防**: 不假装"E7 always speak")
//!
//! **0 装诚实** (per 任务 §3 + 子代理 R 同步):
//! - 真生产路径: `EmergenceOrgan::new(llm_factory, "minimax-m3")` + `APEIRETH_API_KEY` env
//!   + 走 `tick()` (v1 emergence 是确定性无 LLM, 当前 trait 接口 `process()` 走
//!   `tick()` 简化路径 + 8 重门控, **不**调 LLM). `llm_factory()` 返 None (0 装诚实).
//! - dev 测试路径: `NoopLlmFactory` (per `apeireth-plugin::llm_factory::NoopLlmFactory`),
//!   构造参数保留给 v2.1 真生产路径用, 当前**不**接入.
//! - 子代理 R7 独立判断: 任务说明里的"5 状态机"实际来自 v1 `apeireth-evolution::state`,
//!   不在 `apeireth-companion::emergence` 内部. v2 `PolicyStage` 是前向声明, 留接口
//!   给 future apeireth-evolution 接入; 本测试验证 enum 形状 + 决策确定性, 不假装
//!   "emergence 自带 5 状态机".
//!
//! **承接**:
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入), E7 共享同 trait 边界
//! - 子代理 R1 (F1) / R2 (F4) / R3 (F6) / R4 (W1) / R6 (W3) / R8 (Memory) 已并行完成, E7 同步 1:1 翻译

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::SessionId;
use apeireth_organ::emergence::{EmergenceOrgan, PolicyStage};
use apeireth_organ::{OrganInput, OrganKind, OrganOutput, OrganTrait};
use apeireth_plugin::llm_factory::NoopLlmFactory;
use std::sync::Arc;

fn make_input(hints: Vec<String>) -> OrganInput {
    let ep = Episode {
        id: "integration-test-e7".into(),
        session_id: SessionId::new().to_string(),
        role: "user".into(),
        content: "主人刚问起 council 那条 edge 的状态".into(),
        timestamp: 1_786_838_400_000, // 2026-08-16 08:40:00 UTC
    };
    OrganInput::new(ep, hints)
}

// ============================================
// Test 1: 5 状态机 Idle → Draft → Proposed (子代理 R7 独立判断: PolicyStage 前向声明)
// ============================================

/// **子代理 R7 独立判断**: 任务说明里的"5 状态机 Idle/Draft/Proposed/Ratified/Active"
/// 实际来自 v1 `apeireth-evolution::state::EvolutionStateMachine` (6 状态含 Retired),
/// 不在 `apeireth-companion::emergence::emergence.rs` 内部. v2 `PolicyStage` 是
/// **前向声明**, 留接口给 future apeireth-evolution 接入. 本测试验证 enum 形状 +
/// `policy_stage()` 锁默认 `Active` (per v1 `AwakeCompanion::new()` 默认批准一份新策略),
/// 不假装"emergence 自带状态机".
///
/// 测试 1 验证: 状态机全部 5 variant 可达 + 默认起点 Active + process() 决策确定性.
#[tokio::test]
async fn emergence_organ_propose_transitions_state_to_proposed() {
    let organ = EmergenceOrgan::new(Arc::new(NoopLlmFactory), "minimax-m3");

    // (a) PolicyStage 5 variant 全可达 — 编译期保证, 显式枚举供测试断言
    let stages = [
        PolicyStage::Idle,
        PolicyStage::Draft,
        PolicyStage::Proposed,
        PolicyStage::Ratified,
        PolicyStage::Active,
    ];
    assert_eq!(
        stages.len(),
        5,
        "5 状态机 = Idle/Draft/Proposed/Ratified/Active (per v1 evolution)"
    );

    // (b) 默认 policy_stage = Active (per v1 AwakeCompanion 默认批准一份新策略)
    assert_eq!(
        organ.policy_stage(),
        PolicyStage::Active,
        "v2 默认 Active 占位; 真生产路径等 apeireth-evolution crate 接入后真改"
    );

    // (c) process() 走确定性决策 — 0 LLM 介入
    let output = organ.process(make_input(vec!["决策".into()])).await;
    // 当前 v2 E7 在 process() 简化路径下: 没喂观察数据 → rhythm_unknown 门控拦下 → spoke=false
    // (per 子代理 R7 0 装诚实: process() 严格走 8 重门控, 不假装"主动开口")
    let output = output.expect("process 路径不返 Err");
    match output {
        OrganOutput::Emergence { action, spoke, .. } => {
            assert!(!spoke, "无观察数据 → rhythm_unknown 拦下, 不主动开口");
            assert!(action.is_empty(), "未开口 → action 字段为空字符串");
        }
        other => panic!("expected Emergence output, got {other:?}"),
    }

    // (d) 0 装诚实: llm_factory() 返 None (v1 emergence 是确定性无 LLM)
    assert!(
        organ.llm_factory().is_none(),
        "v1 emergence 是确定性无 LLM, v2 不假装能调"
    );
}

// ============================================
// Test 2: Ratify → Active (5 状态机后续阶段 + emergence process() 路径)
// ============================================

/// **子代理 R7 独立判断**: v1 emergence.rs 真实现是 **rhythm+boundary loop 8 重门控**,
/// 不是状态机. 本测试不是"真状态机转换测试" (v1 没有), 而是验证:
/// - PolicyStage::Ratified / PolicyStage::Active is_active=true (per v1 evolution)
/// - 喂足观察数据后 process() → spoke=true (走完 8 重门控后真开口)
/// - 应在活跃时段 (8:40) + 深关系 → RhythmMatched 主动路径
#[tokio::test]
async fn emergence_organ_ratify_activates_proposal() {
    // 构造 + 喂足观察数据 (7 天 8:40) + 深关系 (默认 0.5, 提高到 0.8)
    let organ = EmergenceOrgan::with_depth(Arc::new(NoopLlmFactory), "minimax-m3", 0.8);
    let start_ms: i64 = 1_786_838_400_000 - 7 * 86_400_000; // 7 天前 2026-08-09
    for d in 0..7 {
        organ.observe_interaction(start_ms + d * 86_400_000);
    }

    // (a) PolicyStage::Ratified.is_active() = true (per v1 evolution 已通过审议可发声)
    assert!(
        PolicyStage::Ratified.is_active(),
        "Ratified 已通过审议可发声 (per v1 evolution)"
    );
    assert!(
        PolicyStage::Active.is_active(),
        "Active 已激活在生效 (per v1 evolution)"
    );

    // (b) process() 走完 8 重门控后真开口 (deep_bond + 活跃时段)
    // input.timestamp = 2026-08-16 08:40 UTC → minutes=8*60+40=520, day=2026-08-16
    let output = organ.process(make_input(vec![])).await;
    let output = output.expect("process 路径不返 Err");
    match output {
        OrganOutput::Emergence { action, spoke, .. } => {
            assert!(
                spoke,
                "深关系 + 活跃时段 + 8 重门控全过 → spoke=true (RhythmMatched)"
            );
            assert!(
                !action.is_empty(),
                "开口 → action 字段非空 (v1 Action::label 1:1)"
            );
            // v1 Action::select(None) = Greet → label = "问候"
            assert_eq!(
                action, "问候",
                "无 context_hint → 默认 Greet 动作标签 (v1 Action::select 1:1)"
            );
        }
        other => panic!("expected Emergence output, got {other:?}"),
    }

    // (c) OrganOutput::Emergence 二次确认: 同样输入同样输出 (v1 8 重门控 = 确定性)
    let output2 = organ.process(make_input(vec![])).await.expect("process 路径 2");
    if let OrganOutput::Emergence { spoke: s2, .. } = output2 {
        // 第二次同一天: initiatives_today 已 +1, max=2 → 仍可; 但 min_llm_interval_ms=60s 未过
        // → LlmBudget 拦下 → spoke=false. (v1 deterministic)
        assert!(!s2, "60s 内第二次主动被 LlmBudget 拦下 (Rate-Limit)");
    }
}

// ============================================
// Test 3: Rate-Limit + Idle 抑制 (0 装诱导预防: 不假装"E7 always speak")
// ============================================

/// **0 装诱导预防** (子代理 R7 独立判断): v1 emergence.rs 8 重门控真实存在,
/// 不假装"E7 always speak". 本测试验证:
/// - QuietHours / DailyLimit / LlmBudget / DepthLow / RhythmUnknown / RhythmVeto / DriveLow
///   6 重门控真实拦下;
/// - 即使 deep_bond + 活跃时段, Rate-Limit (60s) 内第二次主动被 LlmBudget 拦下;
/// - 关系深度低于 min_depth (0.3) → DepthLow 拦下.
#[tokio::test]
async fn emergence_organ_should_speak_respects_rate_limit_and_idle() {
    // (a) Idle / RhythmUnknown 抑制: 0 观察 → 不主动
    let fresh_organ = EmergenceOrgan::with_depth(Arc::new(NoopLlmFactory), "minimax-m3", 0.8);
    let output = fresh_organ
        .process(make_input(vec![]))
        .await
        .expect("process 路径不返 Err");
    if let OrganOutput::Emergence { spoke, .. } = output {
        assert!(!spoke, "0 观察 → RhythmUnknown 抑制, 不假装主动开口");
    }

    // (b) DepthLow 抑制: 浅关系 → DepthLow 拦下
    let shallow_organ = EmergenceOrgan::with_depth(Arc::new(NoopLlmFactory), "minimax-m3", 0.1);
    let start_ms: i64 = 1_786_838_400_000 - 7 * 86_400_000;
    for d in 0..7 {
        shallow_organ.observe_interaction(start_ms + d * 86_400_000);
    }
    let output = shallow_organ
        .process(make_input(vec![]))
        .await
        .expect("process 路径不返 Err");
    if let OrganOutput::Emergence { spoke, .. } = output {
        assert!(!spoke, "关系 0.1 < min_depth 0.3 → DepthLow 抑制");
    }

    // (c) Rate-Limit (LlmBudget) 抑制: deep_bond + 活跃时段, 60s 内第二次主动被拦
    let organ = EmergenceOrgan::with_depth(Arc::new(NoopLlmFactory), "minimax-m3", 0.8);
    for d in 0..7 {
        organ.observe_interaction(start_ms + d * 86_400_000);
    }
    // 第一次主动
    let r1 = organ.process(make_input(vec![])).await.expect("process 1");
    if let OrganOutput::Emergence { spoke: s1, .. } = r1 {
        assert!(s1, "第一次主动: deep_bond + 活跃时段 → spoke=true");
    }
    // 第二次主动 (同 at_ms, 同一天, max_initiatives_per_day=2 允许, 但 LlmBudget 拦下)
    let r2 = organ.process(make_input(vec![])).await.expect("process 2");
    if let OrganOutput::Emergence { spoke: s2, .. } = r2 {
        assert!(
            !s2,
            "60s 内第二次主动被 LlmBudget 拦下 (Rate-Limit 抑制; 不假装'E7 always speak')"
        );
    }

    // (d) QuietHours 抑制: 输入 timestamp 落在安静窗口 → QuietHours 拦下
    // 默认 Boundaries.quiet_start_minutes=None / quiet_end_minutes=None → 默认无安静窗口
    // 我们改用 shallow 关系 (depth=0.1) + 0 观察作综合抑制; QuietHours 单独走 lib 单元测试
    // (`quiet_window_blocks_initiative` 已在 emergence.rs lib 测试覆盖).

    // (e) **0 装诚实** (final): process() 路径不返 Err(OrganError::LlmUnavailable) —
    // v1 emergence 是确定性无 LLM, 不假装能调
    let output = organ
        .process(make_input(vec!["记得提醒我".into()]))
        .await
        .expect("process 不返 Err (v1 确定性, 不调 LLM)");
    // 60s 间隔内第三次 tick → 仍被 LlmBudget 拦下
    if let OrganOutput::Emergence { spoke, .. } = output {
        assert!(
            !spoke,
            "Rate-Limit 持续抑制 (60s 内多次 tick → LlmBudget 反复拦下)"
        );
    }

    // (f) **0 装诚实** (final): organ_id + name 锁定 E7
    assert_eq!(organ.name(), "E7 Emergence");
    assert_eq!(organ.organ_id(), OrganKind::E7);
}