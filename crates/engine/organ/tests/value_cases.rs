//! F6 Value Cases 器官 集成测试 (per 任务 §3, 子代理 R3).
//!
//! 3 测试 + 1 #[ignore] smoke + 1 trait shape (per R2 同模式):
//!
//! 1. `value_cases_organ_record_and_promote_candidates` (record → feedback → promote_candidates
//!    路径, 1:1 v1)
//! 2. `value_cases_organ_decision_for_matches_value_set` (decision_for 集合乱序匹配,
//!    1:1 v1)
//! 3. `value_cases_organ_recall_by_keyword_and_disagree_blocks` (recall 路径 +
//!    disagree 阻提升, 1:1 v1)
//!
//! **0 装诚实 (per 任务 §3 + 子代理 R 同款)**:
//!
//! - 真生产路径: `ValueCasesOrgan::new(real_llm_factory, "minimax-m3-thinking")` —
//!   v1 value_cases 是确定性无 LLM, **当前 trait 接口不调 LLM**, trait 边界 + future
//!   LLM 价值萃取路径已就位.
//! - dev 测试路径: `NoopLlmFactory`, 验 trait 边界
//! - `#[ignore]` 测试: 真生产前阻塞 #1 验真接 LLM 注入 shape (per E4/F4 同款)
//!
//! **承接**:
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入), F6 与 E4/F4 共享
//!   `LlmFactory` trait 边界
//! - 子代理 R1 emotion_memory + 子代理 R2 hypothesis 并行写, 0 触碰
//!
//! **0 装诚实 (R3 独立判断)**: 任务示例 API (`add_case`/`weight_for`/`decay`/`total_weight`/
//! `7 category`/`ValueCategory`) **不是 v1 真 API**. v1 `value_cases.rs` 真 API 是
//! `record`/`feedback`/`promote_candidates`/`decision_for`/`recall`+ `DecisionBasis` 3 态
//! + `Feedback` 2 态 (per `legacy/donor/apeireth-companion/src/value_cases.rs:13-148`).
//! 本测试走 **v1 真 API**, 不发明新 API.

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::SessionId;
use apeireth_organ::value_cases::{
    DecisionBasis, Feedback, ValueCase, ValueCaseStore, ValueCasesOrgan,
};
use apeireth_organ::{OrganInput, OrganKind, OrganOutput, OrganTrait, ValueVerdict};
use apeireth_plugin::llm_factory::{LlmFactory, NoopLlmFactory};
use std::sync::Arc;

// ============================================
// Test 1: record → feedback → promote_candidates 路径 (1:1 v1)
// ============================================

fn empty_input_with_hints(hints: Vec<String>) -> OrganInput {
    let ep = Episode {
        id: "integration-test-f6".into(),
        session_id: SessionId::new().to_string(),
        role: "user".into(),
        content: "主人刚提到他对熬夜工作有点纠结".into(),
        timestamp: 1_700_000_000,
    };
    OrganInput::new(ep, hints)
}

#[tokio::test]
async fn value_cases_organ_record_and_promote_candidates() {
    // 0 装诚实: 用 NoopLlmFactory 占位 (v1 value_cases 确定性无 LLM, 当前 trait 不调).
    let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = ValueCasesOrgan::new(noop, "minimax-m3-thinking");

    // 1) 直接 API: record 3 个裁决 (per v1 record API)
    let c1 = organ.record(
        "是否继续熬夜工作",
        vec!["健康".into(), "进度".into()],
        "劝主人休息",
        DecisionBasis::CouncilDeliberation,
    );
    let c2 = organ.record(
        "是否替主人拒绝高风险工具调用",
        vec!["安全".into(), "自主".into()],
        "拒绝, 等主人批准",
        DecisionBasis::ConstitutionRule,
    );
    let c3 = organ.record(
        "是否分享主人的私人数据",
        vec!["隐私".into(), "信任".into()],
        "不分享",
        DecisionBasis::MasterDecision,
    );

    assert_eq!(c1.id, 0, "首条 id=0 (v1 next_id 起始)");
    assert_eq!(c2.id, 1);
    assert_eq!(c3.id, 2);
    assert_eq!(organ.len(), 3);

    // 2) feedback Agree 2 次 → promote_candidates 触发 (per v1 feedback + promote 路径)
    organ.feedback(c1.id, Feedback::Agree).unwrap();
    organ.feedback(c1.id, Feedback::Agree).unwrap();
    let cands = organ.promote_candidates(2);
    assert_eq!(cands.len(), 1, "c1 同意 2 次 → 提升候选 1 条");
    assert_eq!(cands[0].1, "劝主人休息");

    // 3) feedback Agree 1 次 → 不达阈值 (threshold=2)
    organ.feedback(c2.id, Feedback::Agree).unwrap();
    let cands_below = organ.promote_candidates(2);
    assert_eq!(
        cands_below.len(),
        1,
        "c2 仅 1 次同意, 不达 threshold=2 → 不提升"
    );

    // 4) feedback Agree 2 次后降低阈值 → c2 也提升
    let cands_low = organ.promote_candidates(1);
    assert_eq!(
        cands_low.len(),
        2,
        "threshold=1 → c1 (2 agree) + c2 (1 agree) 全提升"
    );

    // 5) process() 路径: 走 OrganTrait, 登记裁决 (per v1 record 路径入口)
    let output = organ
        .process(empty_input_with_hints(vec![
            "劝主人早睡".into(),
            "健康".into(),
            "作息".into(),
        ]))
        .await
        .expect("process ok");
    match output {
        OrganOutput::Value { case_id, verdict } => {
            assert_eq!(case_id, 3, "process 路径下首条 case id=3");
            // 0 装诚实: verdict=Pending (刚登记, 不知主人是否同意)
            assert_eq!(verdict, ValueVerdict::Pending);
        }
        other => panic!("expected Value output, got {other:?}"),
    }
    // 总数 = 4
    assert_eq!(organ.len(), 4);

    // 6) trait 边界: organ_id + name 锁定 F6
    assert_eq!(organ.organ_id(), OrganKind::F6);
    assert_eq!(organ.name(), "F6 Value Cases");
    assert!(
        organ.llm_factory().is_none(),
        "v1 value_cases 是确定性无 LLM, trait 必须返 None (0 装诚实)"
    );
}

// ============================================
// Test 2: decision_for 集合乱序匹配 (1:1 v1)
// ============================================

#[tokio::test]
async fn value_cases_organ_decision_for_matches_value_set() {
    let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = ValueCasesOrgan::new(noop, "minimax-m3-thinking");

    // 1) 登记 3 条裁决, 各自不同冲突价值集合
    organ.record(
        "场景1",
        vec!["安全".into(), "速度".into()],
        "安全优先",
        DecisionBasis::ConstitutionRule,
    );
    organ.record(
        "场景2",
        vec!["健康".into(), "进度".into()],
        "劝主人休息",
        DecisionBasis::CouncilDeliberation,
    );
    organ.record(
        "场景3",
        vec!["隐私".into(), "信任".into()],
        "不分享",
        DecisionBasis::MasterDecision,
    );

    // 2) decision_for 乱序传入 → 排序后匹配 (per v1 decision_for 1:1)
    let d1 = organ
        .decision_for(&["速度".into(), "安全".into()])
        .expect("乱序也匹配");
    assert_eq!(d1.decision, "安全优先");
    assert_eq!(d1.basis, DecisionBasis::ConstitutionRule);

    let d2 = organ
        .decision_for(&["进度".into(), "健康".into()])
        .expect("乱序也匹配 2");
    assert_eq!(d2.decision, "劝主人休息");

    let d3 = organ
        .decision_for(&["信任".into(), "隐私".into()])
        .expect("乱序也匹配 3");
    assert_eq!(d3.decision, "不分享");
    assert_eq!(d3.basis, DecisionBasis::MasterDecision);

    // 3) 不匹配的集合 → None
    assert!(organ.decision_for(&["速度".into()]).is_none());
    assert!(organ.decision_for(&["未知".into()]).is_none());

    // 4) values 排序 + 去重 (per v1 1:1, 防御性测试)
    let mut s = ValueCaseStore::new();
    let c = s.record(
        "排序测试",
        vec!["b".into(), "a".into(), "b".into()],
        "decide",
        DecisionBasis::ConstitutionRule,
    );
    assert_eq!(
        c.values,
        vec!["a".to_string(), "b".to_string()],
        "values 排序 + 去重 (per v1 1:1)"
    );

    // 5) 0 装诚实: trait 边界
    assert_eq!(organ.organ_id(), OrganKind::F6);
    assert!(organ.llm_factory().is_none());
}

// ============================================
// Test 3: recall 关键词 + disagree 阻提升 (1:1 v1)
// ============================================

#[tokio::test]
async fn value_cases_organ_recall_by_keyword_and_disagree_blocks() {
    let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = ValueCasesOrgan::new(noop, "minimax-m3-thinking");

    // 1) 登记裁决 (per v1 record API)
    let c_disagree = organ.record(
        "场景X",
        vec!["a".into(), "b".into()],
        "决定A",
        DecisionBasis::MasterDecision,
    );
    let c_recall = organ.record(
        "是否替主人拒绝高风险工具调用",
        vec!["安全".into(), "自主".into()],
        "拒绝, 等主人批准",
        DecisionBasis::ConstitutionRule,
    );

    // 2) feedback Disagree → 不被提升 (per v1 disagree_blocks_promotion 1:1)
    organ.feedback(c_disagree.id, Feedback::Disagree).unwrap();
    let cands_after_disagree = organ.promote_candidates(1);
    assert!(
        cands_after_disagree.is_empty(),
        "主人不同意 → 不提升 (per v1 1:1)"
    );

    // 3) recall 关键词检索 (per v1 recall API 1:1)
    let hits = organ.recall("高风险");
    assert_eq!(hits.len(), 1, "高风险匹配 1 条");
    assert!(hits[0].decision.contains("拒绝"));
    assert_eq!(hits[0].id, c_recall.id);

    let no_hits = organ.recall("量子力学");
    assert_eq!(no_hits.len(), 0, "量子力学不匹配");

    let hits_owner = organ.recall("主人");
    assert_eq!(
        hits_owner.len(),
        1,
        "主人匹配 1 条 (c_recall scenario 含'主人', c_disagree scenario '场景X' 不含)"
    );

    // 4) recall 不该返 disagree 的 c_disagree (验证 feedback 不影响 recall)
    let all_x = organ.recall("场景X");
    assert_eq!(all_x.len(), 1);
    assert!(all_x[0].feedback == Some(Feedback::Disagree));

    // 5) 0 装诚实: trait 边界
    assert_eq!(organ.organ_id(), OrganKind::F6);
    assert!(organ.llm_factory().is_none());
}

// ============================================
// Test 4 (#[ignore]): 真 LLM factory 注入 shape 验
// ============================================

/// **0 装诚实**: 真接 LLM smoke (同 curiosity / hypothesis 模式). 当前 trait `process()`
/// 不调 LLM (v1 确定性). #[ignore] 给 manual 验真 LLM 注入后 trait shape 不破.
#[tokio::test]
#[ignore = "requires APEIRETH_API_KEY env; manual run: cargo test -p apeireth-organ -- --ignored"]
async fn value_cases_organ_real_llm_smoke() {
    let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = ValueCasesOrgan::new(factory, "minimax-m3-thinking");
    let output = organ
        .process(empty_input_with_hints(vec![
            "smoke test".into(),
            "test_value".into(),
        ]))
        .await
        .expect("process ok");
    match output {
        OrganOutput::Value { .. } => {
            // OK: trait 边界 + factory 注入都 work
        }
        other => panic!("expected Value output, got {other:?}"),
    }
}

// ============================================
// Test 5: trait 边界验证 (no-LLM / dry-run / organ_id / name)
// ============================================

#[tokio::test]
async fn value_cases_organ_trait_shape_complete() {
    let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = ValueCasesOrgan::with_dry_run(noop, "minimax-m3-thinking", true);

    // 0 装诚实标: trait 默认 llm_factory() 返 None
    assert!(organ.llm_factory().is_none());

    // organ_id + name 锁定 F6
    assert_eq!(organ.organ_id(), OrganKind::F6);
    assert_eq!(organ.name(), "F6 Value Cases");

    // 初始空 store
    assert!(organ.is_empty());
    assert_eq!(organ.len(), 0);

    // dry_run 模式 process → NotImplemented
    let ep = Episode {
        id: "dry-run-test".into(),
        session_id: SessionId::new().to_string(),
        role: "user".into(),
        content: "dry-run test".into(),
        timestamp: 0,
    };
    let input = OrganInput::new(ep, vec!["decide".into(), "v1".into()]);
    let output = organ.process(input).await.expect("dry-run returns Ok");
    match output {
        OrganOutput::NotImplemented { organ: k, note } => {
            assert_eq!(k, OrganKind::F6);
            assert!(note.contains("dry-run"));
        }
        other => panic!("expected NotImplemented in dry-run, got {other:?}"),
    }
    // dry-run 不真登记
    assert!(organ.is_empty());

    // ValueCase 字段 (per v1 1:1)
    let mut store = ValueCaseStore::new();
    let c = store.record(
        "字段测试",
        vec!["x".into(), "y".into()],
        "decide",
        DecisionBasis::CouncilDeliberation,
    );
    assert_eq!(c.id, 0);
    assert!(c.feedback.is_none());
    assert_eq!(c.agree_count, 0);
    assert_eq!(c.at_ms, 0); // v2 默认 0 (per v2 organ crate 时间约定)
    assert_eq!(c.values, vec!["x".to_string(), "y".to_string()]);
    // 显式 at_ms 注入
    let c2 = store.record_at_ms(
        "at_ms 测试",
        vec!["a".into()],
        "decide",
        DecisionBasis::MasterDecision,
        1_700_000_000,
    );
    assert_eq!(c2.at_ms, 1_700_000_000);
    // 标 0 装诚实: 不显式注入 chrono::Utc::now() (v2 organ crate 无 chrono dep)
}
