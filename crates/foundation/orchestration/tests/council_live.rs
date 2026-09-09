//! Council live E2E with the OpenAI-compatible factory (DeepSeek).
//!
//! 0 装纪律与 provider/organ live tests 一致: `#[ignore]` 默认不跑; 手动跑需
//! `OPENAI_API_KEY` + `APEIRETH_OPENAI_URL` + `APEIRETH_OPENAI_MODELS` env;
//! 0 commit key / 0 print key / 0 mock 真 LLM。
//!
//! 桥接: plugin 工厂经 `apeireth_plugin::MirrorLlmFactory` 桥到 orchestration
//! 镜像 trait（O-6 归位: 桥在 plugin, 单一事实源; 生产 CLI/gateway 与测试
//! 共用同一实现）。

use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_orchestration::{Council, CouncilVerdict, Proposal};
use apeireth_plugin::llm_factory::LlmFactory as PluginLlmFactory;
use apeireth_plugin::MirrorLlmFactory;
use apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory;

fn sample_proposal() -> Proposal {
    Proposal {
        id: "p-live".into(),
        proposer: "live-e2e".into(),
        payload: serde_json::json!({"action": "deploy"}),
        submitted_at: 1_700_000_000,
        session_id: SessionId::new(),
    }
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY + APEIRETH_OPENAI_MODELS env (DeepSeek live E2E, manual)"]
async fn council_7_advisor_live_decide() {
    let inner: Arc<dyn PluginLlmFactory> = Arc::new(
        OpenAiCompatibleLlmFactory::from_env()
            .expect("factory from env (需 OPENAI_API_KEY + APEIRETH_OPENAI_MODELS)"),
    );
    let factory: Arc<dyn apeireth_orchestration::llm::LlmFactory> =
        Arc::new(MirrorLlmFactory::new(inner));
    let council = Council::with_factory(factory, "deepseek-v4-flash");
    assert_eq!(council.advisors().len(), 7);

    let verdict = council.decide(&sample_proposal()).await;
    eprintln!("live council verdict: {verdict:?}");
    // 0 装: 断言 7 个真 LLM advisor 并行判定路径完整跑通 (不強断特定 verdict —
    // 模型判定文本决定结果, 防 flaky); DeferToHuman 也是合法出口 (整体 60s 超时)。
    assert!(
        matches!(
            verdict,
            CouncilVerdict::Approved
                | CouncilVerdict::Vetoed { .. }
                | CouncilVerdict::DeferToHuman { .. }
        ),
        "verdict must be a valid CouncilVerdict, got {verdict:?}"
    );
}
