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

/// live 测试用的模型名（**默认仍是 DeepSeek**，行为不变）。
///
/// # 为什么可配化（2026-10-31 · 副总指挥交接请求③）
///
/// 此前本文件**硬编码** `deepseek-v4-flash` ⇒ ⇒ 换厂商（MiniMax / 其它
/// openai-compatible）**复跑不了** ⇒ ⇒ 台账挂账 #9 会被「厂商 key 被撤销」
/// 这种**与代码无关**的原因卡住。
///
/// env 约定（与 `OpenAiCompatibleLlmFactory::from_env` 一致）：
/// * `APEIRETH_OPENAI_MODELS` ⇒ 逗号分隔的模型列表（默认 `deepseek-v4-flash`）
/// * `APEIRETH_OPENAI_MODEL` ⇒ **单模型覆盖**（默认取列表第一个）
///
/// ⚠️ **0 装纪律不变**：不打印 key、不落 key、默认 `#[ignore]` 不跑。
/// ⚠️ 同族实现见 `crates/engine/provider/tests/openai_compatible_live.rs` 与
///   `crates/engine/organ/tests/organ_live_llm.rs`（同形三处都钉）。
fn live_model() -> String {
    if let Ok(model) = std::env::var("APEIRETH_OPENAI_MODEL") {
        let model = model.trim().to_string();
        if !model.is_empty() {
            return model;
        }
    }
    std::env::var("APEIRETH_OPENAI_MODELS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .find(|item| !item.is_empty())
                .unwrap_or_default()
                .to_string()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "deepseek-v4-flash".to_string())
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY + APEIRETH_OPENAI_MODELS env (DeepSeek live E2E, manual)"]
async fn council_7_advisor_live_decide() {
    use apeireth_plugin::llm_factory::{CompletionMessage, CompletionRequest, LlmInstance};

    let inner: Arc<dyn PluginLlmFactory> = Arc::new(
        OpenAiCompatibleLlmFactory::from_env()
            .expect("factory from env (需 OPENAI_API_KEY + APEIRETH_OPENAI_MODELS)"),
    );

    // 0 装护栏 (2026-10-06 核销批堵洞): 先做一次最小真 LLM 往返, **证明通道活着**。
    // 否则 key 撤销/断网时 7 个 advisor 全部秒失败并汇聚成 `DeferToHuman` 降级出口,
    // 本测试会**假绿** (实锤: 401 那次 0.67s "通过" vs 真跑历史 5.0s)。通道死必须
    // 在此显式炸, 不许降级冒充 live。
    let probe = inner
        .spawn(
            apeireth_orchestration::SubagentRole::Reviewer,
            &live_model(),
        )
        .await
        .expect("probe spawn");
    let probe_resp = probe
        .complete(CompletionRequest {
            system_prompt: "be very brief".into(),
            messages: vec![CompletionMessage {
                role: "user".into(),
                content: "Reply with the single word 'ok' and nothing else.".into(),
            }],
            temperature: 0.0,
            tools: vec![],
            // 思考型模型 reasoning_content 吃预算 (canonical_openai_compatible.rs
            // adapt_request 注释: 2048 仍空 1/3, 4096 稳) —— 探针要"真内容", 给足
            // reasoning 余量; 给 64 会把预算喂光、content 落空 (2026-10-10 实锤)。
            max_tokens: Some(2048),
        })
        .await
        .expect("probe completion: channel dead must fail loudly, never degrade into a green");
    assert!(
        !probe_resp.message.content.is_empty(),
        "probe must return real content before council live runs"
    );

    let factory: Arc<dyn apeireth_orchestration::llm::LlmFactory> =
        Arc::new(MirrorLlmFactory::new(inner));
    let council = Council::with_factory(factory, &live_model());
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
