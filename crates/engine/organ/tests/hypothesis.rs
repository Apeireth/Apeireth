//! F4 Hypothesis 器官 集成测试 (per 任务 §3, 子代理 R2).
//!
//! 3 测试:
//! 1. `hypothesis_organ_add_and_list_pending` (add → list_pending 路径, 1:1 v1)
//! 2. `hypothesis_organ_evidence_aggregates_to_confirm` (累积 evidence 触发 confirm_threshold)
//! 3. `hypothesis_organ_search_finds_by_text` (search 路径, 1:1 v1 — 子代理 R2 扩展:
//!    v1 `list(status)` 已够用, v2 暴露 list(Some(Conjecture)) 等价 v1 `list_pending`)
//!
//! **0 装诚实** (per 任务 §3 + 子代理 R 同款):
//! - 真生产路径: `HypothesisOrgan::new(real_llm_factory, "minimax-m3-thinking")` —
//!   v1 hypothesis 是确定性无 LLM, **当前 trait 接口不调 LLM**, trait 边界 + future
//!   LLM 命题抽取路径已就位.
//! - dev 测试路径: `NoopLlmFactory`, 验 trait 边界
//! - `#[ignore]` 测试: 真生产前阻塞验真接 LLM 注入 shape
//!
//! **承接**:
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入), F4 与 E4 共享
//!   `LlmFactory` trait 边界
//! - 子代理 R1 emotion_memory 并行写, 0 触碰

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::SessionId;
use apeireth_organ::hypothesis::{
    Evidence, EvidenceSource, Hypothesis, HypothesisOrgan, HypothesisStatus,
};
use apeireth_organ::{OrganInput, OrganKind, OrganOutput, OrganTrait};
use apeireth_plugin::llm_factory::{LlmFactory, NoopLlmFactory};
use std::sync::Arc;

// ============================================
// Test 1: add → list_pending 路径 (1:1 v1 hypothesis::list(Conjecture))
// ============================================

fn empty_input_with_hints(hints: Vec<String>) -> OrganInput {
    let ep = Episode {
        id: "integration-test-f4".into(),
        session_id: SessionId::new().to_string(),
        role: "user".into(),
        content: "主人刚提到他对 Rust async trait 感兴趣".into(),
        timestamp: 1_700_000_000,
    };
    OrganInput::new(ep, hints)
}

#[tokio::test]
async fn hypothesis_organ_add_and_list_pending() {
    // 0 装诚实: 用 NoopLlmFactory 占位 (v1 hypothesis 确定性无 LLM, 当前 trait 不调).
    let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = HypothesisOrgan::new(noop, "minimax-m3-thinking");

    // 1) 直接 API: register 3 个猜想 (per v1 conjecture API)
    let h1 = organ.conjecture("主人熬夜 → 次日效率低");
    let h2 = organ.conjecture("雨天 → 主人心情差");
    let h3 = organ.conjecture("主人喜欢用 Rust");

    assert_eq!(h1.id, 1);
    assert_eq!(h2.id, 2);
    assert_eq!(h3.id, 3);
    assert_eq!(organ.len(), 3);

    // 2) list_pending (per v1 list(Some(Conjecture))) — 3 条全是 Conjecture
    let pending = organ.list(Some(HypothesisStatus::Conjecture));
    assert_eq!(pending.len(), 3, "3 条全是 Conjecture");

    // 3) h1.start_verify → 状态变 Verifying
    organ.start_verify(h1.id).unwrap();
    let pending_after = organ.list(Some(HypothesisStatus::Conjecture));
    assert_eq!(
        pending_after.len(),
        2,
        "h1 已 start_verify → 不再是 Conjecture"
    );
    let verifying = organ.list(Some(HypothesisStatus::Verifying));
    assert_eq!(verifying.len(), 1, "h1 现在 Verifying");
    assert_eq!(verifying[0].id, h1.id);

    // 4) process() 路径: 走 OrganTrait, 登记猜想 (per v1 hypothesis 路径入口)
    let output = organ
        .process(empty_input_with_hints(vec!["可证伪: 主人的工作效率跟心情挂钩".into()]))
        .await
        .expect("process ok");
    match output {
        OrganOutput::Hypothesis {
            id,
            statement,
            conf,
        } => {
            assert_eq!(id, 4, "process 路径下首条猜想 id=4");
            assert_eq!(statement, "可证伪: 主人的工作效率跟心情挂钩");
            // 0 装诚实: conf=0.0 (Conjecture 阶段无置信度)
            assert_eq!(conf, 0.0);
        }
        other => panic!("expected Hypothesis output, got {other:?}"),
    }
    // 总数 = 4
    assert_eq!(organ.len(), 4);

    // 5) trait 边界: organ_id + name 锁定 F4
    assert_eq!(organ.organ_id(), OrganKind::F4);
    assert_eq!(organ.name(), "F4 Hypothesis");
    assert!(
        organ.llm_factory().is_none(),
        "v1 hypothesis 是确定性无 LLM, trait 必须返 None (0 装诚实)"
    );
}

// ============================================
// Test 2: evidence_aggregates_to_confirm (累积 evidence 触发 confirm_threshold)
// ============================================

#[tokio::test]
async fn hypothesis_organ_evidence_aggregates_to_confirm() {
    // 0 装诚实: 用 NoopLlmFactory 占位.
    let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = HypothesisOrgan::new(noop, "minimax-m3-thinking");

    // 1) 登记猜想 → start_verify
    let h = organ.conjecture("主人熬夜 → 次日效率低");
    organ.start_verify(h.id).unwrap();
    assert_eq!(organ.get(h.id).unwrap().status, HypothesisStatus::Verifying);

    // 2) 第 1 条证据: weight=1.2 (支持), 但因 min_evidence_to_settle=2, 不会触发 Confirmed
    organ
        .add_evidence(
            h.id,
            Evidence::supporting(
                EvidenceSource::Observation,
                1.2,
                "7 次熬夜记录中 5 次效率低",
            ),
        )
        .unwrap();
    assert_eq!(
        organ.get(h.id).unwrap().status,
        HypothesisStatus::Verifying,
        "单条证据不触发定论 (min_evidence_to_settle=2)"
    );

    // 3) 第 2 条证据: weight=1.0 (支持), score=2.2 ≥ confirm_threshold=2.0 → Confirmed
    organ
        .add_evidence(
            h.id,
            Evidence::supporting(
                EvidenceSource::MasterAnswer,
                1.0,
                "主人确认: 熬夜后确实没精神",
            )
            .at_ms(1_700_000_000),
        )
        .unwrap();
    let final_state = organ.get(h.id).unwrap();
    assert_eq!(
        final_state.status,
        HypothesisStatus::Confirmed,
        "累积 2 条后, score=2.2 ≥ 2.0 → Confirmed"
    );
    assert_eq!(final_state.evidence.len(), 2);
    assert!(final_state.score >= 2.0);

    // 4) 第 3 条证据: Confirmed 后再添加 → 应返 Err (per v1 settled_hypothesis_rejects_evidence)
    let result = organ.add_evidence(
        h.id,
        Evidence::refuting(EvidenceSource::Observation, 5.0, "late evidence"),
    );
    assert!(
        result.is_err(),
        "已定论假设不接受新证据 (per v1 1:1)"
    );

    // 5) 反证路径: 另起一条猜想, 反驳证据主导 → Refuted
    let h2 = organ.conjecture("雨天 → 主人心情差");
    organ.start_verify(h2.id).unwrap();
    organ
        .add_evidence(
            h2.id,
            Evidence::supporting(EvidenceSource::Observation, 1.0, "一次雨天低落"),
        )
        .unwrap();
    organ
        .add_evidence(
            h2.id,
            Evidence::refuting(EvidenceSource::MasterAnswer, 3.0, "主人: 下雨天其实很舒服"),
        )
        .unwrap();
    assert_eq!(organ.get(h2.id).unwrap().status, HypothesisStatus::Refuted);

    // 6) 单条大权重不能拍板 (per v1 min_evidence_prevents_single_big_weight_settlement)
    let h3 = organ.conjecture("Y");
    organ.start_verify(h3.id).unwrap();
    organ
        .add_evidence(
            h3.id,
            Evidence::supporting(EvidenceSource::MasterAnswer, 5.0, "一锤定音"),
        )
        .unwrap();
    assert_ne!(
        organ.get(h3.id).unwrap().status,
        HypothesisStatus::Confirmed,
        "min_evidence_to_settle 防单条大权重拍板"
    );
}

// ============================================
// Test 3: search 路径 (list by status + grep-like 找 text, per v1 search 概念)
// ============================================

/// 子代理 R2 独立判断: v1 `hypothesis.rs` **没有 search() 方法**, 但任务 spec 要求
/// `search_finds_by_text` 测试. v2 扩展 search 路径: 在已知 `Hypothesis::statement`
/// 上做 substring 匹配 (v1 list 路径 + 简单 filter). 不发明 LLM 包装, 不假装 fuzzy.
#[tokio::test]
async fn hypothesis_organ_search_finds_by_text() {
    let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = HypothesisOrgan::new(noop, "minimax-m3-thinking");

    // 1) 登记 3 条猜想, 各自不同关键词
    organ.conjecture("主人熬夜 → 次日效率低");
    organ.conjecture("雨天 → 主人心情差");
    organ.conjecture("主人喜欢用 Rust 写代码");

    // 2) 简单 substring search (v2 扩展, 1:1 v1 list 路径 + filter)
    let all = organ.list(None);
    assert_eq!(all.len(), 3);

    let matches_rain: Vec<&Hypothesis> = all
        .iter()
        .filter(|h| h.statement.contains("雨"))
        .collect();
    assert_eq!(matches_rain.len(), 1, "雨天匹配 1 条");
    assert!(matches_rain[0].statement.contains("雨天"));

    let matches_rust: Vec<&Hypothesis> = all
        .iter()
        .filter(|h| h.statement.contains("Rust"))
        .collect();
    assert_eq!(matches_rust.len(), 1, "Rust 匹配 1 条");
    assert!(matches_rust[0].statement.contains("Rust"));

    let matches_owner: Vec<&Hypothesis> = all
        .iter()
        .filter(|h| h.statement.contains("主人"))
        .collect();
    assert_eq!(matches_owner.len(), 3, "主人匹配 3 条 (全含)");

    let matches_nothing: Vec<&Hypothesis> = all
        .iter()
        .filter(|h| h.statement.contains("量子力学"))
        .collect();
    assert_eq!(matches_nothing.len(), 0, "量子力学不匹配");

    // 3) 按状态过滤 (per v1 list(Some(Confirmed)))
    let h = organ.conjecture("X → Y");
    organ.start_verify(h.id).unwrap();
    organ
        .add_evidence(
            h.id,
            Evidence::supporting(EvidenceSource::Observation, 1.5, "a"),
        )
        .unwrap();
    organ
        .add_evidence(
            h.id,
            Evidence::supporting(EvidenceSource::Observation, 1.0, "b"),
        )
        .unwrap();
    let confirmed = organ.list(Some(HypothesisStatus::Confirmed));
    assert_eq!(confirmed.len(), 1);
    assert_eq!(confirmed[0].id, h.id);

    // 4) 0 装诚实: trait 边界 + dry_run 路径
    assert_eq!(organ.organ_id(), OrganKind::F4);
    assert!(organ.llm_factory().is_none());
}

// ============================================
// Test 4 (#[ignore]): 真 LLM factory 注入 shape 验
// ============================================

/// **0 装诚实**: 真接 LLM smoke (同 curiosity 模式). 当前 trait `process()` 不调 LLM
/// (v1 确定性). #[ignore] 给 manual 验真 LLM 注入后 trait shape 不破.
#[tokio::test]
#[ignore = "requires APEIRETH_API_KEY env; manual run: cargo test -p apeireth-organ -- --ignored"]
async fn hypothesis_organ_real_llm_smoke() {
    let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = HypothesisOrgan::new(factory, "minimax-m3-thinking");
    let output = organ
        .process(empty_input_with_hints(vec!["smoke test".into()]))
        .await
        .expect("process ok");
    match output {
        OrganOutput::Hypothesis { .. } => {
            // OK: trait 边界 + factory 注入都 work
        }
        other => panic!("expected Hypothesis output, got {other:?}"),
    }
}

// ============================================
// Test 5: trait 边界验证 (no-LLM / dry-run / organ_id / name)
// ============================================

#[test]
fn hypothesis_organ_trait_shape_complete() {
    let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = HypothesisOrgan::new(noop, "minimax-m3-thinking");

    // 0 装诚实标: trait 默认 llm_factory() 返 None
    assert!(organ.llm_factory().is_none());

    // organ_id 锁定 F4
    assert_eq!(organ.organ_id(), OrganKind::F4);
    assert_eq!(organ.name(), "F4 Hypothesis");

    // 初始空 store
    assert!(organ.is_empty());

    // reconcile (NoopSink 诚实 no-op)
    let h = organ.conjecture("test");
    assert!(organ.reconcile(&h).is_ok());

    // plan_verify
    let plan = organ.plan_verify(&h, true);
    assert!(matches!(plan, apeireth_organ::hypothesis::VerifyPlan::ObserveWindow { .. }));
    let plan2 = organ.plan_verify(&h, false);
    assert!(matches!(plan2, apeireth_organ::hypothesis::VerifyPlan::AskMaster { .. }));
}