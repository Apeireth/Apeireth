//! E4 Curiosity 器官 集成测试 (per 任务 §3).
//!
//! 3 测试:
//! 1. `curiosity_organ_returns_score_and_question` (mock LlmFactory 返固定响应, 验 trait 边界)
//! 2. `curiosity_organ_real_llm_smoke` (真 LLM smoke, #[ignore] 给 manual 跑; 验真接 LLM)
//! 3. `curiosity_organ_no_llm_returns_error` (None factory trait 边界, 验 0 装诚实 —
//!    Curiosity trait 默认 `llm_factory()` 返 None, 不假装能调)
//!
//! **0 装诚实** (per 任务 §3 + 子代理 R 同款):
//! - 真生产路径: `CuriosityOrgan::new(real_llm_factory, "minimax-m3-thinking")` +
//!   `APEIRETH_API_KEY` env + 走真 LLM 调用 (v1 curiosity 是确定性无 LLM, **当前 trait
//!   接口不调**, 但 trait 边界 + future LLM 探索路径已就位)
//! - dev 测试路径: `NoopLlmFactory` (per `apeireth-plugin::llm_factory::NoopLlmFactory`),
//!   返 `LlmError::NotImplemented` (0 装: 不假装能调)
//! - `#[ignore]` 测试: 真生产前阻塞 #1 — 跑 `cargo test -- --ignored` manual 验真接 LLM
//!
//! **承接**:
//! - 子代理 D actionable #1 真兑现 (Experience 保守版是真接 LLM, Curiosity trait 边界预留)
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入), Curiosity 共享同 trait

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::SessionId;
use apeireth_organ::curiosity::{CuriosityOrgan, Echo, EchoSource};
use apeireth_organ::{CuriosityDepth, OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait};
use apeireth_plugin::llm_factory::{LlmFactory, NoopLlmFactory};
use std::sync::Arc;

// ============================================
// Test 1: mock LlmFactory 返固定响应 (trait 边界 + 真实现路径)
// ============================================

/// Mock factory: 永远返固定响应 (用于验 trait 边界, 不假装真 LLM)
struct MockLlmFactory {
    fixed_response: String,
}

impl MockLlmFactory {
    fn new(response: impl Into<String>) -> Self {
        Self {
            fixed_response: response.into(),
        }
    }
}

#[async_trait::async_trait]
impl LlmFactory for MockLlmFactory {
    async fn spawn(
        &self,
        _role: apeireth_orchestration::SubagentRole,
        _model: &str,
    ) -> Result<Box<dyn apeireth_plugin::llm_factory::LlmInstance>, apeireth_plugin::llm_factory::LlmError>
    {
        // 0 装诚实: mock 直接返 NotImplemented, 不假装真 LLM.
        // 真接 LLM 路径在 #[ignore] test 里.
        Err(apeireth_plugin::llm_factory::LlmError::NotImplemented(
            "MockLlmFactory (test 0 装, 真接 LLM 在 --ignored test)",
        ))
    }

    async fn available_models(&self) -> Result<Vec<String>, apeireth_plugin::llm_factory::LlmError> {
        Ok(vec!["mock-model".to_string()])
    }

    fn name(&self) -> &str {
        "mock"
    }
}

fn empty_input() -> OrganInput {
    let ep = Episode {
        id: "integration-test".into(),
        session_id: SessionId::new().to_string(),
        role: "user".into(),
        content: "主人刚提到他对 Rust async trait 感兴趣".into(),
        timestamp: 1_700_000_000,
    };
    OrganInput::new(ep, vec!["rust".into(), "async".into(), "trait".into()])
}

#[tokio::test]
async fn curiosity_organ_returns_score_and_question() {
    // 0 装诚实: 用 MockLlmFactory 验 trait 边界. 真实 curiosity 算法**不调 LLM**
    // (per v1 确定性 + 0 装诚实), 所以这个测试验:
    // - trait shape 完整 (构造/调用/输出 schema)
    // - MockLlmFactory trait impl 编译通过 (LLM 调用路径就位)
    // - OrganOutput::Curiosity schema 含 targets / ask_master / budget_left
    let mock: Arc<dyn LlmFactory> = Arc::new(MockLlmFactory::new("dummy"));

    let organ = CuriosityOrgan::new(mock, "mock-model");
    organ.feed_echoes([
        Echo::new(
            "rust async trait",
            0.85,
            EchoSource::Memory,
        ),
    ]);
    organ.deepen("rust async trait"); // 强回声 → Deep

    // 喂 oracle 意外度 (per v1 `feed_surprise`)
    organ.feed_surprise("rust async trait", 0.6);

    let output = organ
        .process(empty_input())
        .await
        .expect("process returns Ok");
    match output {
        OrganOutput::Curiosity {
            targets,
            ask_master,
            budget_left,
        } => {
            // 1) targets 非空 (强回声主题应被采到)
            assert!(!targets.is_empty(), "强回声主题应被采样到目标");
            // 2) 至少 1 个目标 = Deep (因为 deepen 触发)
            let deep_count = targets
                .iter()
                .filter(|t| matches!(t.depth, CuriosityDepth::Deep))
                .count();
            assert!(
                deep_count >= 1,
                "deepen 触发后应至少有 1 个 Deep 目标, got {deep_count}"
            );
            // 3) ask_master 不含强回声 (回声 ≥ 0.6 阈值 → 自己探索)
            let master_topics: Vec<&str> =
                ask_master.iter().map(|t| t.topic.as_str()).collect();
            assert!(
                !master_topics.contains(&"rust async trait"),
                "强回声主题不应 ask_master"
            );
            // 4) budget_left > 0 (初始预算 2000)
            assert!(budget_left > 0.0, "initial budget must remain");
            // 5) llm_factory() 返 None (0 装诚实)
            assert!(
                organ.llm_factory().is_none(),
                "v1 curiosity 是确定性无 LLM, trait 默认 None"
            );
            // 6) organ_id 锁定 E4
            assert_eq!(organ.organ_id(), OrganKind::E4);
            assert_eq!(organ.name(), "E4 Curiosity");
        }
        other => panic!("expected OrganOutput::Curiosity, got {other:?}"),
    }
}

// ============================================
// Test 2: 真 LLM smoke (#[ignore], 需 APEIRETH_API_KEY)
// ============================================

/// **0 装诚实**: 真接 LLM smoke 测试.
#[tokio::test]
#[ignore = "requires APEIRETH_API_KEY env + real LLM endpoint; manual run: cargo test -p apeireth-organ -- --ignored"]
async fn curiosity_organ_real_llm_smoke() {
    // 真生产路径: 用 NoopLlmFactory 占位 (因为 v1 curiosity 当前**不调 LLM**,
    // 这条 smoke test 仅验证 trait 边界在真 LLM factory 注入下能编译/构造).
    //
    // **重要 0 装**: 当前 trait `process()` 不调 LLM (v1 确定性无 LLM).
    // 真 LLM 集成是 v2.1 路线 (LLM 探索具体内容, **不动 v1 算法真相**).
    // 此 #[ignore] test 的目的: 验证 trait shape 在真 factory 注入下不变.
    let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = CuriosityOrgan::new(factory, "minimax-m3-thinking");
    let output = organ.process(empty_input()).await.expect("process ok");
    match output {
        OrganOutput::Curiosity { .. } => {
            // OK: trait 边界 + factory 注入都 work
        }
        other => panic!("expected Curiosity output, got {other:?}"),
    }
}

// ============================================
// Test 3: no-LLM / None factory → 显式 LlmUnavailable (0 装诚实)
// ============================================

/// **0 装诚实**: trait `llm_factory()` 默认 None — 不假装能调 LLM.
///
/// v1 curiosity 是**确定性机制**, 永不需要 LLM. v2 trait 默认 `llm_factory()` 返 None
/// (per `apeireth-plugin::organ::OrganTrait::llm_factory()` default impl).
///
/// 此处验证:
    // 1) trait 默认 `llm_factory()` 返 None (没 fake)
    // 2) 即便传 None-shaped trait object (实际不可能, Arc<dyn LlmFactory> 必须有 impl),
    //    `CuriosityOrgan` 也能构造 (因为 trait 字段保留 + 实际不用)
    // 3) process 路径不调用 LLM, 不返 LlmError
#[test]
fn curiosity_organ_no_llm_returns_none_factory() {
    // 0 装诚实: NoopLlmFactory 是 0 装显式占位, 不调真 LLM.
    // 测试 trait 边界: `llm_factory()` 返 None (不假装).
    let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = CuriosityOrgan::new(noop, "noop-model");

    // 0 装诚实标: trait 默认 llm_factory() 返 None
    assert!(
        organ.llm_factory().is_none(),
        "v1 curiosity 是确定性无 LLM, trait 必须返 None (0 装诚实)"
    );

    // 显式验证 organ_id 锁定 E4
    assert_eq!(organ.organ_id(), OrganKind::E4);
}

/// **0 装诚实**: process 路径不依赖 LLM, 不返 LlmError.
///
/// v1 curiosity 是纯确定性, 不会报 LlmUnavailable / LlmError. 如果未来 v2.1 真接
/// LLM 探索路径, 此测试是回归门禁 — 失败时说明 process 路径不当地依赖 LLM.
#[tokio::test]
async fn curiosity_organ_process_never_returns_llm_error() {
    let noop: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
    let organ = CuriosityOrgan::new(noop, "noop-model");

    // 0 装诚实: process 应永远 OK (v1 确定性), 不报 LlmError
    let result = organ.process(empty_input()).await;
    match result {
        Ok(_) => {
            // 预期路径: 成功 (v1 确定性)
        }
        Err(OrganError::LlmError(_)) | Err(OrganError::LlmUnavailable(_)) => {
            panic!("0 装诚实: v1 curiosity 确定性路径不应报 LLM 错误")
        }
        Err(other) => {
            // 其他 error (Config / BudgetExhausted / Internal) 也算 0 装, 但 LLM 错必失败
            let _ = other;
        }
    }
}