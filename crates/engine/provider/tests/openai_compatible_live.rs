//! Live E2E smoke for the generic OpenAI-compatible provider against DeepSeek.
//!
//! 0 装纪律（与 minimax_llm_factory::real_llm_call_smoke 同款）:
//! - `#[ignore]` 默认 `cargo test` **不**跑；
//! - 手动跑: `cargo test -p apeireth-provider --test openai_compatible_live -- --ignored --nocapture`
//! - 需要 `OPENAI_API_KEY` env（DeepSeek 或任意 OpenAI-compatible key）；
//!   端点默认 DeepSeek，可用 `APEIRETH_OPENAI_URL` 覆盖；
//! - 0 commit key / 0 print key / 0 mock 真 LLM；
//! - CI 无 key，**不**跑（per O-6 锚 #9 "0 装诚实: 真接 key"）。

use std::sync::Arc;

use apeireth_plugin::{CredentialResolver, ProviderCapability};
use apeireth_protocol::canonical::{NormalizedMessage, NormalizedRequest};

/// DeepSeek 默认端点（OpenAI-compatible 协议族）。
const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY env (DeepSeek live E2E, manual)"]
async fn deepseek_live_e2e_smoke() {
    // 模型列表: 以 2026-09 DeepSeek /v1/models 实况为准 (deepseek-v4-flash /
    // deepseek-v4-pro / deepseek-v4-flash-vision-exp); 用 flash 便宜档。
    let models = vec!["deepseek-v4-flash".to_string()];
    let http = reqwest::Client::builder().build().expect("reqwest client");
    let plugin =
        apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin::new(
            DEEPSEEK_BASE_URL,
            models,
            http,
            60_000,
        )
        .expect("plugin build");
    // 真接 key via EnvCredentialResolver (默认 OPENAI_API_KEY 映射)。
    plugin.attach_resolver_for_test(Arc::new(
        apeireth_provider::credentials::EnvCredentialResolver::new(),
    ));
    let cap = plugin.provider_for_test();

    let req = NormalizedRequest::new(
        "deepseek-v4-flash",
        vec![NormalizedMessage::user(
            "Reply with the single word 'ok' and nothing else.",
        )],
    );
    let resp = cap.complete(&req).await.expect("live DeepSeek completion");
    assert!(!resp.content.is_empty(), "response content 0 装非空");
    assert!(resp.usage.total_tokens > 0, "usage 0 装报告 token");
    // 0 装诚实: 不打印 key; 内容可打印供目检。
    eprintln!("live content: {}", resp.content);
}

/// Factory 级 live smoke: `OpenAiCompatibleLlmFactory::spawn → instance.complete`
/// 走 LlmInstance 边界（9-organ / Council 的真实注入路径）。
#[tokio::test]
#[ignore = "requires OPENAI_API_KEY + APEIRETH_OPENAI_MODELS env (DeepSeek live E2E, manual)"]
async fn deepseek_live_factory_smoke() {
    use apeireth_plugin::llm_factory::{
        CompletionMessage, CompletionRequest, LlmFactory, LlmInstance,
    };
    use apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory;

    let factory = OpenAiCompatibleLlmFactory::from_env().expect("factory from env");
    let instance = factory
        .spawn(
            apeireth_orchestration::SubagentRole::Reviewer,
            "deepseek-v4-flash",
        )
        .await
        .expect("spawn");
    let req = CompletionRequest {
        system_prompt: "be very brief".into(),
        messages: vec![CompletionMessage {
            role: "user".into(),
            content: "Reply with the single word 'ok' and nothing else.".into(),
        }],
        temperature: 0.0,
        tools: vec![],
        // 0 装经验值: max_tokens=8 时 DeepSeek 会在 finish=length 下返空 content
        // (首个 token 即被截断); 64 安全覆盖 "ok" 回复。
        max_tokens: Some(64),
    };
    let resp = instance
        .complete(req)
        .await
        .expect("live factory completion");
    eprintln!(
        "live factory resp: content={:?} finish={} usage={:?}",
        resp.message.content, resp.finish_reason, resp.usage
    );
    assert!(!resp.message.content.is_empty());
    assert!(resp.usage.total_tokens > 0);
}
