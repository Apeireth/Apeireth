//! W3 Causal Edge Mining 器官 集成测试 (per 任务 §3).
//!
//! 3 测试:
//! 1. `edge_miner_organ_observe_event_increments_weight` (observe_event 累计权重路径)
//! 2. `edge_miner_organ_get_top_edges_returns_highest_weight` (get_top_edges 排序路径)
//! 3. `edge_miner_organ_decay_reduces_weights_over_time` (decay_weights 时间衰减路径)
//!
//! **0 装诚实** (per 任务 §4 + 子代理 R 同款):
//! - W3 是**确定性被动观察路径**, 无 LLM 依赖 (per v1 doc 第 179 行).
//! - trait `llm_factory()` 返 None, 不假装"W3 也用 LLM".
//! - dev 测试路径: `NoopLlmFactory` (占位未来扩展字段, 当前算法不用).

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::SessionId;
use apeireth_organ::causal_world_model_edges::{EdgeKind, EdgeMinerOrgan, FactRecord};
use apeireth_organ::{OrganKind, OrganOutput, OrganTrait};
use apeireth_plugin::llm_factory::NoopLlmFactory;
use std::sync::Arc;

fn test_factory() -> Arc<dyn apeireth_plugin::llm_factory::LlmFactory> {
    Arc::new(NoopLlmFactory)
}

fn empty_input() -> apeireth_organ::OrganInput {
    let ep = Episode {
        id: "test-episode-w3".into(),
        session_id: SessionId::new().to_string(),
        role: "user".into(),
        content: "".into(),
        timestamp: 0,
    };
    apeireth_organ::OrganInput::new(ep, vec![])
}

fn build_causal_facts(n_pairs: usize) -> Vec<FactRecord> {
    let mut facts = Vec::new();
    for i in 0..n_pairs {
        let ts_base = 1_000_000 + i as i64 * 100;
        facts.push(FactRecord::new("主人", "行为", "熬夜", ts_base));
        facts.push(FactRecord::new("熬夜", "导致", "效率低", ts_base + 60));
    }
    facts
}

// ============================================
// Test 1: observe_event 累计权重路径
// ============================================

#[tokio::test]
async fn edge_miner_organ_observe_event_increments_weight() {
    let organ = EdgeMinerOrgan::new(test_factory(), "minimax-m3");

    // 3 次观察同一边: 权重累加 + 证据数 +1.
    organ.observe_event("熬夜", "效率低", EdgeKind::Correlates, 0.3);
    organ.observe_event("熬夜", "效率低", EdgeKind::Correlates, 0.5);
    organ.observe_event("熬夜", "效率低", EdgeKind::Correlates, 0.2);

    let top = organ.get_top_edges(10);
    assert_eq!(top.len(), 1, "应只有 1 条唯一边");
    assert_eq!(top[0].from, "熬夜");
    assert_eq!(top[0].to, "效率低");
    assert_eq!(top[0].observation_count, 3, "3 次 → 证据数 = 3");
    assert!(
        (top[0].total_weight - 1.0).abs() < 1e-5,
        "权重累计 0.3+0.5+0.2 = 1.0, got {}",
        top[0].total_weight
    );

    // total_edges 应计 1 条 (per v1 任务说明 unique pairs).
    assert_eq!(organ.total_edges(), 1);
}

// ============================================
// Test 2: get_top_edges 排序路径 — 返回最高权重
// ============================================

#[tokio::test]
async fn edge_miner_organ_get_top_edges_returns_highest_weight() {
    let organ = EdgeMinerOrgan::new(test_factory(), "minimax-m3");

    // 喂 3 条不同权重边 (单次观察, 避免累计叠加).
    organ.observe_event("熬夜", "效率低", EdgeKind::Correlates, 0.5);
    organ.observe_event("运动", "心情好", EdgeKind::Correlates, 0.9);
    organ.observe_event("拖延", "焦虑", EdgeKind::Correlates, 0.2);

    // Top-1 应是权重最高的"运动→心情好".
    let top1 = organ.get_top_edges(1);
    assert_eq!(top1.len(), 1);
    assert_eq!(top1[0].from, "运动");
    assert_eq!(top1[0].to, "心情好");
    assert!((top1[0].total_weight - 0.9).abs() < 1e-5);

    // Top-2 应加 0.5 (熬夜→效率低).
    let top2 = organ.get_top_edges(2);
    assert_eq!(top2.len(), 2);
    assert_eq!(top2[1].from, "熬夜");

    // 全部 top3 应按权重降序: 0.9 > 0.5 > 0.2.
    let top_all = organ.get_top_edges(10);
    assert_eq!(top_all.len(), 3);
    let weights: Vec<f32> = top_all.iter().map(|e| e.total_weight).collect();
    assert!(weights[0] > weights[1], "权重应降序: {:?}", weights);
    assert!(weights[1] > weights[2], "权重应降序: {:?}", weights);
    assert_eq!(top_all[0].from, "运动", "权重 0.9 第一");
    assert_eq!(top_all[1].from, "熬夜", "权重 0.5 第二");
    assert_eq!(top_all[2].from, "拖延", "权重 0.2 第三");
}

// ============================================
// Test 3: decay_weights 路径 — 权重随时间衰减
// ============================================

#[tokio::test]
async fn edge_miner_organ_decay_reduces_weights_over_time() {
    let organ = EdgeMinerOrgan::new(test_factory(), "minimax-m3");

    // 喂时间线 (7 对 → 触发挖掘, 边权重 = 条件概率).
    let facts = build_causal_facts(7);
    let (edges, _) = organ.feed_timeline(&facts);
    assert!(!edges.is_empty(), "应有挖掘边");

    let before = organ.get_top_edges(10);
    assert!(!before.is_empty());
    let weight_before = before[0].total_weight;
    assert!(weight_before > 0.0, "权重初始 > 0");

    // 1000 秒后衰减 (1e6 ms = 1000 秒, factor = 0.99^1000 ≈ 4.3e-5).
    organ.decay_weights(1_000_000);
    let after = organ.get_top_edges(10);
    let weight_after = after[0].total_weight;

    assert!(
        weight_after < weight_before,
        "衰减后权重应下降: before={}, after={}",
        weight_before,
        weight_after
    );
    assert!(weight_after > 0.0, "衰减不归零: after={}", weight_after);

    // 多次衰减 → 权重继续下降 (单调).
    organ.decay_weights(1_000_000);
    let after_more = organ.get_top_edges(10);
    let weight_after_more = after_more[0].total_weight;
    assert!(
        weight_after_more < weight_after,
        "继续衰减应继续下降: prev={}, after={}",
        weight_after,
        weight_after_more
    );
}

// ============================================
// 0 装诚实附验 (per 任务 §4)
// ============================================

/// 0 装诚实: W3 被动路径, llm_factory() 返 None.
#[tokio::test]
async fn w3_llm_factory_returns_none_per_v1_truth() {
    let organ = EdgeMinerOrgan::new(test_factory(), "minimax-m3");
    assert!(
        organ.llm_factory().is_none(),
        "v1 W3 是确定性被动观察, v2 不假装能调 LLM (0 装诱导预防)"
    );
}

/// 0 装诚实: organ_id + name 锁定 W3.
#[tokio::test]
async fn w3_name_and_organ_id_locked() {
    let organ = EdgeMinerOrgan::new(test_factory(), "minimax-m3");
    assert_eq!(organ.name(), "W3 Causal Edge Miner");
    assert_eq!(organ.organ_id(), OrganKind::W3);
}

/// 0 装诚实: process() 走 WorldModel 输出, counterfactual 空 (W3 0 反事实).
#[tokio::test]
async fn w3_process_outputs_world_model_with_empty_counterfactual() {
    let organ = EdgeMinerOrgan::new(test_factory(), "minimax-m3");
    let facts = build_causal_facts(7);
    let (_, _) = organ.feed_timeline(&facts);

    let output = organ.process(empty_input()).await.expect("process ok");
    match output {
        OrganOutput::WorldModel {
            edges,
            counterfactual,
        } => {
            assert!(!edges.is_empty(), "挖掘后应有边");
            assert!(
                counterfactual.is_empty(),
                "W3 被动路径 0 反事实推演 (那是 W2 主动 MCTS 的活, 0 装诚实)"
            );
            // 验证 plugin CausalEdge schema 1:1 翻译.
            for e in &edges {
                assert!(!e.cause.is_empty());
                assert!(!e.effect.is_empty());
                assert!(e.conf > 0.0 && e.conf <= 1.0);
                assert_eq!(e.source, "Statistical", "W3 主路径 = Statistical");
            }
        }
        other => panic!("expected WorldModel output, got {other:?}"),
    }
}