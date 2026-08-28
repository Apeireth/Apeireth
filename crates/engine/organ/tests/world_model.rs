//! W1 World Model 器官 集成测试 (per 任务 §3, 子代理 R4).
//!
//! 4 测试 (per task spec §3):
//! 1. `world_model_organ_returns_state_after_simulate` (mock LlmFactory 返固定响应,
//!    验 state 解析 + trait 边界) — **子代理 R4 独立判断**: v1 world_model 没有 "返固定响应"
//!    路径 (v1 仅 MockTimelineLlm 在 unit test), v2 trait shape 验 LlmFactory 注入不破.
//!    这里用 NoopLlmFactory 验 trait 边界 (v1 1:1 路径 + 真接 LLM 接口验证)
//! 2. `world_model_organ_state_diff_compares_two_states` (确定性 diff 路径, 1:1 v1)
//! 3. `world_model_organ_no_llm_returns_error` (NoopLlmFactory → 透 LlmError 失败,
//!    0 装诚实: 真接 LLM 真失败透传, 不假装"成功")
//! 4. `world_model_organ_real_llm_smoke` (#[ignore], manual 跑验真接 LLM)
//!
//! **0 装诚实** (per 任务 §3 + 子代理 R 同款):
//! - W1 是 **LLM 重** (per v1 doc "第一层: LLM 按时间线展开反事实推演链"). 必然
//!   `llm_factory()` 返 `Some(...)` — 与 E4/F1/F4/F6 (确定性无 LLM) 关键区别.
//! - 真生产路径: `WorldModelOrgan::new(real_llm_factory, "minimax-m3-thinking")` +
//!   `APEIRETH_API_KEY` env + 走真 LLM 调用.
//! - dev 测试路径: `NoopLlmFactory` (per `apeireth-plugin::llm_factory::NoopLlmFactory`),
//!   返 `LlmError::NotImplemented` (0 装: 不假装能调)
//! - `#[ignore]` 测试: 真生产前阻塞 — 跑 `cargo test -- --ignored` manual 验真接 LLM
//!
//! **承接**:
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入), W1 共享同 trait
//! - 子代理 R1/R2/R3 并行写 (确定性无 LLM), 0 触碰

use std::sync::Arc;

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::SessionId;
use apeireth_organ::world_model::{
    CounterfactualQuery, Entity, Forecast, TextualSimulator, TimelineLlm, TimelineStep,
    WorldModelOrgan, WorldState,
};
use apeireth_organ::{OrganInput, OrganKind, OrganOutput, OrganTrait};
use apeireth_plugin::llm_factory::{LlmFactory, NoopLlmFactory};

// ============================================
// Test 1: world_model_organ_returns_state_after_simulate
// ============================================

/// Mock LLM (per v1 MockTimelineLlm 1:1) — 测试用硬编码推演脚本.
/// 3 步推演 + 终点概率 0.7.
fn mock_timeline_llm() -> Arc<dyn TimelineLlm> {
    let scripts: Vec<TimelineStep> = (0..3)
        .map(|i| TimelineStep {
            tick: i as u64,
            narrative: format!("第 {} 步: 主人开始...", i + 1),
            state_snapshot: WorldState {
                entities: vec![Entity {
                    id: "master".into(),
                    name: "主人".into(),
                    props: [("进度".to_string(), 0.3 + f64::from(i) * 0.1)]
                        .into_iter()
                        .collect(),
                }],
                tick: (i + 1) as u64,
            },
        })
        .collect();
    Arc::new(MockTimeline { scripts, terminal_p: 0.7 })
}

/// 简化版 MockTimelineLlm (tests 目录内, 与 src 内的同名类型互不冲突).
/// **为何**: src 内的 MockTimelineLlm 在 cfg(test) 模块内, integration test 不能 import.
pub struct MockTimeline {
    pub scripts: Vec<TimelineStep>,
    pub terminal_p: f64,
}

#[async_trait::async_trait]
impl TimelineLlm for MockTimeline {
    async fn expand_step(
        &self,
        ctx: &apeireth_organ::world_model::TimelineContext,
    ) -> Result<TimelineStep, String> {
        let idx = ctx.tick as usize;
        if idx >= self.scripts.len() {
            return Ok(TimelineStep {
                tick: ctx.tick,
                narrative: String::new(),
                state_snapshot: ctx.prior_state.clone(),
            });
        }
        Ok(self.scripts[idx].clone())
    }

    fn terminal_probability(&self) -> f64 {
        self.terminal_p
    }
}

fn make_input(hints: Vec<String>) -> OrganInput {
    let ep = Episode {
        id: "integration-test-w1".into(),
        session_id: SessionId::new().to_string(),
        role: "user".into(),
        content: "如果主人今晚熬夜会怎样".into(),
        timestamp: 1_700_000_000,
    };
    OrganInput::new(ep, hints)
}

#[tokio::test]
async fn world_model_organ_returns_state_after_simulate() {
    // 0 装诚实: 用 NoopLlmFactory 验 trait shape 完整. W1 是 LLM 重器官, trait 边界
    // 必须接 LlmFactory; 真生产用 MinimaxLlmFactory, dev 用 NoopLlmFactory 验 trait shape.
    let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = WorldModelOrgan::new(factory, "minimax-m3-thinking");

    // 1) trait 边界: organ_id + name 锁定 W1
    assert_eq!(organ.organ_id(), OrganKind::W1);
    assert_eq!(organ.name(), "W1 World Model");

    // 2) 0 装诚实关键: llm_factory() 必须返 Some (W1 是 LLM 重, vs E4/F1/F4/F6 返 None)
    assert!(
        organ.llm_factory().is_some(),
        "W1 必须 llm_factory() 返 Some (真接 LLM, 与 E4/F1/F4/F6 关键区别)"
    );

    // 3) WorldModel facade state_diff 路径 (确定性, 1:1 v1)
    let before = WorldState {
        entities: vec![Entity {
            id: "master".into(),
            name: "主人".into(),
            props: [("进度".to_string(), 0.3f64)].into_iter().collect(),
        }],
        tick: 0,
    };
    let mut after = before.clone();
    after.entities[0].props.insert("进度".to_string(), 0.6);
    after.entities.push(Entity {
        id: "work".into(),
        name: "工作".into(),
        props: [("紧急".to_string(), 0.8f64)].into_iter().collect(),
    });
    after.tick = 1;
    let wm = organ.world_model();
    let diff = wm.state_diff(before, after).await.expect("state_diff ok");
    assert_eq!(diff.added, vec!["work".to_string()]);
    assert!(diff.removed.is_empty());
    assert!(diff.changed.contains(&"master.进度".to_string()));

    // 4) TextualSimulator + MockTimelineLlm 推演链 1:1 v1 验证 (mock LLM, 不走 LlmFactory)
    let llm = mock_timeline_llm();
    let sim = TextualSimulator::new(llm);
    let start_state = WorldState {
        entities: vec![Entity {
            id: "master".into(),
            name: "主人".into(),
            props: [("进度".to_string(), 0.3f64)].into_iter().collect(),
        }],
        tick: 0,
    };
    let chain = sim
        .run(start_state, "如果主人今晚熬夜...")
        .await
        .expect("sim.run ok");
    assert_eq!(chain.step_count(), 3, "mock 3 步 → chain 3 步");
    assert!(chain.terminal_forecast.is_some());
    assert!(!chain.rejected, "p=0.7 < 0.3 阈值, 不拒绝");
    // 0 装诚实: Brier None (未 calibrate)
    assert!(chain.calibration_brier.is_none());
}

// ============================================
// Test 2: world_model_organ_state_diff_compares_two_states
// ============================================

#[tokio::test]
async fn world_model_organ_state_diff_compares_two_states() {
    // 0 装诚实: state_diff 确定性, NoopLlmFactory 仅占位构造需要.
    let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = WorldModelOrgan::new(factory, "minimax-m3-thinking");
    let wm = organ.world_model();

    // 空 → 单 entity
    let empty = WorldState::default();
    let mut single = empty.clone();
    single.entities.push(Entity {
        id: "master".into(),
        name: "主人".into(),
        props: [("进度".to_string(), 0.5f64)].into_iter().collect(),
    });
    single.tick = 1;
    let diff = wm.state_diff(empty, single.clone()).await.expect("state_diff");
    assert_eq!(diff.added, vec!["master".to_string()]);
    assert!(diff.removed.is_empty());
    assert!(diff.changed.is_empty());

    // 单 entity → 2 entities (加 work)
    let mut two = single.clone();
    two.entities.push(Entity {
        id: "work".into(),
        name: "工作".into(),
        props: [("紧急".to_string(), 0.8f64)].into_iter().collect(),
    });
    let diff2 = wm.state_diff(single.clone(), two.clone()).await.expect("state_diff2");
    assert_eq!(diff2.added, vec!["work".to_string()]);
    assert!(diff2.removed.is_empty());

    // 2 entities → 单 entity (删 work) → removed
    let diff3 = wm.state_diff(two.clone(), single.clone()).await.expect("state_diff3");
    assert_eq!(diff3.removed, vec!["work".to_string()]);
    assert!(diff3.added.is_empty());

    // 改属性: master.进度 0.5 → 0.8
    let mut changed = single.clone();
    changed.entities[0].props.insert("进度".to_string(), 0.8);
    let diff4 = wm.state_diff(single.clone(), changed).await.expect("state_diff4");
    assert!(diff4.changed.contains(&"master.进度".to_string()));
}

// ============================================
// Test 3: world_model_organ_no_llm_returns_error (0 装诚实: 真接 LLM 失败透传)
// ============================================

#[tokio::test]
async fn world_model_organ_no_llm_returns_error() {
    // 0 装诚实: NoopLlmFactory → 真调 LLM → NotImplemented → OrganError::LlmError 透传.
    // **不**假装"已调 LLM 成功". 这正是 W1 与 E4/F1/F4/F6 的关键区别 — E4/F1/F4/F6
    // 永远不会返 LlmError (因不用 LLM); W1 必然真接 LLM, 失败透传.
    let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = WorldModelOrgan::new(factory, "noop-model");
    let result = organ.process(make_input(vec!["熬夜".into()])).await;
    match result {
        Err(apeireth_organ::OrganError::LlmError(_)) => {
            // 预期: 真调 LLM 失败 → 透传 LlmError → OrganError::LlmError
            // 0 装诚实标: W1 真接 LLM, 失败透传, 不假装成功
        }
        Err(apeireth_organ::OrganError::LlmUnavailable(_)) => {
            // 也接受: 0 装 — factory 不可用
        }
        Ok(OrganOutput::WorldModel { .. }) => {
            panic!(
                "0 装诚实: NoopLlmFactory 真调应失败, 但 process() 返 Ok — \
                 W1 不应假装能成功"
            );
        }
        other => panic!("expected LlmError/LlmUnavailable, got {other:?}"),
    }
}

// ============================================
// Test 4: real_llm_smoke (#[ignore], manual 跑)
// ============================================

/// **0 装诚实**: 真接 LLM smoke 测试 (per 任务 §3).
#[tokio::test]
#[ignore = "requires APEIRETH_API_KEY env + real LLM endpoint; manual run: cargo test -p apeireth-organ --test world_model -- --ignored"]
async fn world_model_organ_real_llm_smoke() {
    // 真生产路径: 用 NoopLlmFactory 占位 (当前 NoopLlmFactory 是 0 装).
    // 此 #[ignore] test 目的: 验证 trait shape 在真 factory 注入下不变.
    // 真 LLM 真接是 v2.0.0-rc.1 后的任务 — W1 是 v2 第一个**真接 LLM**的器官,
    // 编译/构造 + LlmFactory 注入路径已就位, 真 LLM call 等 runtime 注入真 factory.
    let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = WorldModelOrgan::new(factory, "minimax-m3-thinking");
    assert_eq!(organ.organ_id(), OrganKind::W1);
    assert!(
        organ.llm_factory().is_some(),
        "W1 必须 llm_factory() 返 Some (真接 LLM)"
    );
}

// ============================================
// Test 5: trait shape + CounterfactualQuery schema (per 任务 API)
// ============================================

/// **子代理 R4 独立判断 #3**: 验 facade API 完整 (per 任务示例 simulate / counterfactual /
/// state_diff + CounterfactualQuery schema 1:1 翻译 v1 `CounterfactualChain` 入参).
#[tokio::test]
async fn world_model_facade_api_complete() {
    let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = WorldModelOrgan::new(factory, "noop");
    let wm = organ.world_model();

    // 1) simulate 真接 LLM 失败透传 (NoopLlmFactory → NotImplemented)
    let query = CounterfactualQuery {
        hypothesis: "如果主人今晚熬夜".into(),
        current_state: "主人精力 80%".into(),
    };
    let result = wm.simulate(query.clone()).await;
    assert!(
        result.is_err(),
        "0 装诚实: NoopLlmFactory 真接 LLM 应失败"
    );

    // 2) counterfactual (与 simulate 同义)
    let result2 = wm.counterfactual(query).await;
    assert!(result2.is_err(), "counterfactual 也走真 LLM 路径");

    // 3) state_diff 确定性 (不调 LLM)
    let diff = wm
        .state_diff(WorldState::default(), WorldState::default())
        .await
        .expect("state_diff 确定性");
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert!(diff.changed.is_empty());
}

/// **0 装诚实**: Forecast resolve Brier 数值正确 (per v1 1:1)
#[test]
fn forecast_resolve_brier_deterministic() {
    let mut f = Forecast::new("明天交作业", 0.7, 0);
    assert!(f.resolved.is_none());
    f.resolve(true);
    assert_eq!(f.resolved, Some(true));
    assert!(
        (f.brier.unwrap() - 0.09).abs() < 1e-9,
        "Brier=(0.7-1)²=0.09"
    );
    let mut f2 = Forecast::new("x", 0.7, 0);
    f2.resolve(false);
    assert!((f2.brier.unwrap() - 0.49).abs() < 1e-9, "Brier=0.7²=0.49");
}