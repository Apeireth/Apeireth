//! Live E2E for the organs' genuine LLM paths with the OpenAI-compatible
//! factory (DeepSeek). 0 装纪律与 provider live tests 一致:
//! - `#[ignore]` 默认不跑; 手动跑需 `OPENAI_API_KEY` + `APEIRETH_OPENAI_URL` +
//!   `APEIRETH_OPENAI_MODELS` env;
//! - 0 commit key / 0 print key / 0 mock 真 LLM;
//! - 覆盖范围 (诚实): 9 organ 中**真接 LLM**的只有 W1 (反事实推演) 与
//!   W2 (因果图推演/边提议); 其余 7 organ 为确定性实现 (E4/F4/F6/F1/E7/Memory)
//!   或仅 trait shape (W3 边挖掘走确定性统计 + LLM 提议经 W2 路径).

use std::sync::Arc;

use apeireth_plugin::llm_factory::LlmFactory;
use apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory;

use apeireth_organ::causal_world_model::{
    CausalEdge, CausalGraph, CausalNode, CausalWorldModel, CounterfactualQuery as CausalQuery,
    EdgeSource,
};
use apeireth_organ::world_model::{
    CounterfactualQuery, LlmTimelineLlm, TimelineContext, TimelineLlm, WorldModel, WorldState,
};

fn live_factory() -> Arc<dyn LlmFactory> {
    Arc::new(OpenAiCompatibleLlmFactory::from_env().expect(
        "OpenAiCompatibleLlmFactory::from_env (需 OPENAI_API_KEY + APEIRETH_OPENAI_MODELS)",
    ))
}

/// W1 真接 LLM: 步级推演 (expand_step) 必须产出非空叙事 + facade simulate 走通。
#[tokio::test]
#[ignore = "requires OPENAI_API_KEY + APEIRETH_OPENAI_MODELS env (DeepSeek live E2E, manual)"]
async fn w1_world_model_simulate_live() {
    let factory = live_factory();
    // 1) 真 LLM 步级断言 (W1 的真接 LLM 路径).
    let llm = LlmTimelineLlm::new(factory.clone(), "deepseek-v4-flash");
    let ctx = TimelineContext {
        start_state: WorldState::default(),
        hypothesis: "主人今晚熬夜写代码".into(),
        prior_narrative: String::new(),
        prior_state: WorldState::default(),
        tick: 0,
    };
    let step = llm.expand_step(&ctx).await.expect("live W1 expand_step");
    assert!(
        !step.narrative.is_empty(),
        "真 LLM 推演步必须产出叙事 (0 装: 空叙事 = 链未启动)"
    );

    // 2) facade 路径: simulate 走同一条真 LLM 链, 返 Ok。0 装口径: 按 v1 语义
    //    state_snapshot 克隆 prior_state (不解析 LLM 的 state 行), 故不強断言字段.
    let wm = WorldModel::new(factory, "deepseek-v4-flash");
    let query = CounterfactualQuery {
        hypothesis: "主人今晚熬夜写代码".into(),
        current_state: "主人精力 80%, 截止日期明天".into(),
    };
    let _state = wm.simulate(query).await.expect("live W1 simulate");
}

/// W2 真接 LLM: 小因果图上的反事实推演跑通, 返非空推演图。
#[tokio::test]
#[ignore = "requires OPENAI_API_KEY + APEIRETH_OPENAI_MODELS env (DeepSeek live E2E, manual)"]
async fn w2_causal_simulate_live() {
    let wm = CausalWorldModel::new(live_factory(), "deepseek-v4-flash");
    // 注入最小因果图: 熬夜 → 效率下降。
    wm.add_entity(CausalNode::from_chain("熬夜写代码"));
    wm.add_entity(CausalNode::from_chain("次日效率下降"));
    wm.add_edge(CausalEdge {
        id: "e1".into(),
        from: "熬夜写代码".into(),
        to: "次日效率下降".into(),
        predicate: "睡眠不足导致效率下降".into(),
        weight: 0.8,
        evidence_count: 3,
        source: EdgeSource::Statistical,
    });
    assert_eq!(wm.node_count(), 2);
    assert_eq!(wm.edge_count(), 1);

    let query = CausalQuery {
        hypothesis: "如果主人今晚熬夜".into(),
        current_graph: wm.snapshot_graph(),
        start_node: "熬夜写代码".into(),
        max_steps: 4,
    };
    let graph = wm
        .simulate_counterfactual(query)
        .await
        .expect("live W2 simulate_counterfactual");
    // 推演在既有图上展开: 节点数 ≥ 起点 2 (LLM 分支可能加边, 但至少保留原图).
    assert!(
        graph.len_nodes() >= 2,
        "推演图必须保留原图节点, got {}",
        graph.len_nodes()
    );
}
