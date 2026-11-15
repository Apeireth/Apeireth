//! Live E2E smoke: 生产 LlmFactory 的厂商私有请求字段开关，对真实 OpenAI-compatible 端点。
//!
//! 0 装纪律（与同族 `openai_compatible_live.rs` / `organ_live_llm.rs` / `council_live.rs` 同款）:
//! - `#[ignore]` 默认 `cargo test` **不**跑；
//! - 手动跑：`cargo test -p apeireth-provider --test openai_compatible_vendor_body_live -- --ignored --nocapture`
//! - 需要 `OPENAI_API_KEY` env（任意 OpenAI-compatible key）；
//! - 0 commit key / 0 print key / 0 mock 真 LLM；
//! - CI 无 key，**不**跑（per O-6 锚 #9「0 装诚实: 真接 key」）。
//!
//! ⭐ 这条用例覆盖的是**生产装配路径**（`OpenAiCompatibleLlmFactory::from_env_with_resolver`），
//! 不是评测工具路径：生产工厂在**构造期**读 `APEIRETH_OPENAI_THINKING`（推理模式开关），
//! 设了就挂到 capability 上（只对支持该厂商私有字段的端点才有意义，发给不认识它的端点会被 400），
//! 没设就**不发**任何厂商私有字段 ⇒ 请求体与接线前逐字节一致。
//!
//! 「没设 ⇒ 不发」这条不变量由**不联网**的 `vendor_body_factory_wiring.rs` 用 mock 服务器钉住；
//! 本文件只负责「设了 ⇒ 真的能到厂商并拿到应答」这一段真实链路。

use std::sync::Arc;

use apeireth_orchestration::SubagentRole;
use apeireth_plugin::llm_factory::{CompletionMessage, CompletionRequest, LlmFactory, LlmInstance};

/// live 端点（与生产工厂 `from_env` 的 env 约定一致）。
fn live_base_url() -> String {
    std::env::var("APEIRETH_OPENAI_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string())
}

/// live 模型名（`APEIRETH_OPENAI_MODEL` 优先，其次 `APEIRETH_OPENAI_MODELS` 的第一项）。
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
#[ignore = "requires OPENAI_API_KEY env (live vendor-body E2E, manual)"]
async fn the_production_factory_sends_the_vendor_field_and_the_vendor_answers() {
    // 生产装配路径：工厂从 env 构造（key 走 resolver per-turn 解析，工厂 0 持有 key）。
    let factory = apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory::from_env_with_resolver(
        Arc::new(apeireth_provider::credentials::EnvCredentialResolver::new()),
    )
    .expect("生产工厂构造（非法厂商开关取值应在构造期报错，而不是这里）");

    // 厂商私有字段的实际装载情况（只打印键名与取值类型，不打印 key）。
    let extra_body = factory.capability().extra_body();
    let vendor_keys: Vec<String> = extra_body
        .as_ref()
        .and_then(|body| body.as_object())
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default();
    println!(
        "LIVE endpoint = {} model = {} vendor_field_keys = {vendor_keys:?}",
        live_base_url(),
        live_model()
    );

    let model = live_model();
    let instance = factory
        .spawn(SubagentRole::Planner, &model)
        .await
        .expect("spawn instance");

    let response = instance
        .complete(CompletionRequest {
            system_prompt: "system".into(),
            messages: vec![CompletionMessage {
                role: "user".into(),
                content: "Reply with the single word 'ok' and nothing else.".into(),
            }],
            temperature: 1.0,
            tools: vec![],
            max_tokens: None,
        })
        .await
        .expect("live completion（厂商应答）");

    assert!(
        !response.message.content.trim().is_empty(),
        "厂商应答内容为空"
    );
    println!(
        "LIVE ok model = {} content_len = {} vendor_field_keys = {vendor_keys:?} usage = {}/{} tokens",
        model,
        response.message.content.trim().len(),
        response.usage.prompt_tokens,
        response.usage.completion_tokens
    );
}
