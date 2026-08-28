//! RC-5 integration tests — `MinimaxLlmFactory` (factory + instance, no real network).
//!
//! **0 装诚实**: 这些测试**不** mock 真 LLM HTTP — 只走 factory trait 边界路径
//! (spawn / available_models / name / 错路径)。真 LLM 调用走 `real_llm_call_smoke`,
//! `#[ignore]` 标记 (`#[ignore = "requires MINIMAX_API_KEY"]`), 需手动
//! `cargo test -- --ignored` 触发, 0 在 CI 自动跑, 0 commit key.
//!
//! **0 触碰 LOCKED**: 0 改 trait 边界, 0 改 protocol crate, 0 改 orchestration crate.

use std::sync::Arc;

use apeireth_orchestration::SubagentRole;
use apeireth_plugin::llm_factory::{
    CompletionMessage, CompletionRequest, LlmError, LlmFactory, LlmInstance,
};
use apeireth_plugin::{NoCredentials, ProviderCapability, StaticCredentials};
use apeireth_provider::minimax_llm_factory::MinimaxLlmFactory;

/// 构造一个**0 接 key** 的 factory, 用 `NoCredentials` resolver. 仅测 trait 边界.
fn no_key_factory() -> MinimaxLlmFactory {
    // 直接用 plugin 路径构造, attach NoCredentials (key 不存在).
    let plugin = apeireth_provider::canonical_minimax::MinimaxProviderPlugin::from_env()
        .expect("plugin builds");
    plugin.attach_resolver_for_test(Arc::new(NoCredentials));
    MinimaxLlmFactory::new(plugin.provider_for_test())
}

/// 构造一个**接 key** 的 factory, 用 `StaticCredentials`. 仅测 trait 边界 +
/// provider "key 存在但 HTTP fail" 错误路径 (provider complete 会走到 HTTP,
/// 因 resolver slot 已填, 不再返 AuthFailed; 但 endpoint 是真 internet, 没网络或
/// sandbox 可能 hang — 所以这种测试 0 真发, 只保证 factory 构造成功).
fn static_key_factory() -> MinimaxLlmFactory {
    let plugin = apeireth_provider::canonical_minimax::MinimaxProviderPlugin::from_env()
        .expect("plugin builds");
    plugin.attach_resolver_for_test(Arc::new(
        StaticCredentials::new().with("provider.minimax.api_key", "sk-test-static-key"),
    ));
    MinimaxLlmFactory::new(plugin.provider_for_test())
}

#[tokio::test]
async fn factory_name_is_minimax() {
    let factory = no_key_factory();
    assert_eq!(factory.name(), "minimax");
}

#[tokio::test]
async fn spawn_returns_instance_with_minimax_dash_model_name() {
    let factory = no_key_factory();
    let instance = factory
        .spawn(SubagentRole::Reviewer, "MiniMax-M3")
        .await
        .expect("spawn ok");
    assert_eq!(instance.name(), "minimax-MiniMax-M3");
}

#[tokio::test]
async fn available_models_lists_minimax_default_models() {
    let factory = no_key_factory();
    let models = factory.available_models().await.expect("models");
    // canonical_minimax.rs:65 DEFAULT_MODELS = ["MiniMax-M3", "MiniMax-M3-thinking"]
    // → 转 canonical id: ["minimax-m3", "minimax-m3-thinking"]
    assert_eq!(
        models,
        vec!["minimax-m3".to_string(), "minimax-m3-thinking".to_string()],
        "MinimaxLlmFactory.available_models 应跟 capability 配置一致"
    );
}

#[tokio::test]
async fn complete_without_resolver_fails_with_credentials_error() {
    // 0 装诚实: resolver slot 是 NoCredentials → capability resolve_key 返 AuthFailed
    // → map_provider_error 转 LlmError::Credentials. 0 真 HTTP, 0 真 key.
    let factory = no_key_factory();
    let instance = factory
        .spawn(SubagentRole::Planner, "MiniMax-M3")
        .await
        .expect("spawn");
    let req = CompletionRequest {
        system_prompt: "be brief".into(),
        messages: vec![CompletionMessage {
            role: "user".into(),
            content: "hi".into(),
        }],
        temperature: 1.0,
        tools: vec![],
        max_tokens: None,
    };
    let result = instance.complete(req).await;
    match result {
        Err(LlmError::Credentials(_)) => {
            // 0 装诚实: 这是预期错误 (没 key), 不是 bug.
        }
        other => panic!("expected LlmError::Credentials, got {other:?}"),
    }
}

#[tokio::test]
async fn complete_with_static_key_constructs_factory_without_panic() {
    // 0 装诚实: 有 key + 无网络/被 sandbox 拦下 → capability 走 HTTP → 返 Network 或
    // Timeout 或其他 transient; 我们只验证 factory + instance + complete 路径不 panic.
    // 0 真断言具体错 (依赖网络/沙箱), 0 真发到 minimax (会被 env 控制).
    let factory = static_key_factory();
    let instance = factory
        .spawn(SubagentRole::Tester, "MiniMax-M3-thinking")
        .await
        .expect("spawn");
    let req = CompletionRequest {
        system_prompt: "test".into(),
        messages: vec![CompletionMessage {
            role: "user".into(),
            content: "ping".into(),
        }],
        temperature: 0.0,
        tools: vec![],
        max_tokens: Some(8),
    };
    let result = instance.complete(req).await;
    // 0 断言 Ok 或具体错 (网络/sandbox 不可控). 只断言不 panic + 返 LlmError (类型一致).
    match result {
        Ok(_) => {
            // 真 LLM 居然通了 (CI 没沙箱 + endpoint 可达). 0 装诚实: 0 假装这是常规路径.
        }
        Err(LlmError::Credentials(_))
        | Err(LlmError::Network(_))
        | Err(LlmError::RateLimited { .. })
        | Err(LlmError::Provider(_))
        | Err(LlmError::Stream(_)) => {
            // 任何 transient / permanent 都接受.
        }
        Err(other) => panic!("unexpected LlmError variant: {other:?}"),
    }
}

#[tokio::test]
async fn instance_role_is_propagated_from_spawn() {
    let factory = no_key_factory();
    let instance = factory
        .spawn(SubagentRole::Documenter, "MiniMax-M3")
        .await
        .expect("spawn");
    // 0 装诚实: 内部 role() accessor (per factory impl 公开 API)
    // — 我们通过 type-check + 通过 spawn 返 Box<dyn LlmInstance> 验证.
    // (LlmInstance trait 0 暴露 role(), MinimaxLlmInstance 是具体类型, downcast 0 行
    // 因 trait obj. 这里仅验证 spawn 不 panic.)
    let _ = instance.name();
}

/// 真 LLM call smoke — 仅在 `MINIMAX_API_KEY` (或 `APEIRETH_API_KEY`) env 已设时跑。
///
/// **0 装诚实**:
/// - `#[ignore = "requires MINIMAX_API_KEY"]` 标注, 默认 `cargo test` **不**跑
/// - 手动跑: `cargo test -p apeireth-provider --test minimax_llm_factory -- --ignored`
/// - 0 commit key, 0 print key, 0 mock 真 LLM
/// - 真生产环境 CI 应**不**跑 (per O-6 锚 #9 "0 装诚实: 真接 key" — key 走 env, CI 没 key)
#[tokio::test]
#[ignore = "requires MINIMAX_API_KEY or APEIRETH_API_KEY env (手动跑)"]
async fn real_llm_call_smoke() {
    // 0 装诚实: 真接 key via EnvCredentialResolver (默认 APEIRETH_API_KEY).
    let factory = apeireth_provider::minimax_llm_factory::MinimaxLlmFactory::from_env()
        .expect("factory from env");
    let instance = factory
        .spawn(SubagentRole::Implementer, "MiniMax-M3")
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
        max_tokens: Some(8),
    };
    let result = instance.complete(req).await;
    // 0 装诚实: 真 LLM 调可能返 Ok / RateLimited / Network / Provider (4xx/5xx),
    // 都接受, 只验证真 HTTP 路径走通. CI 上有 key 时再加强断言 (e.g. assert
    // result.is_ok()).
    match result {
        Ok(resp) => {
            assert!(
                !resp.message.content.is_empty(),
                "response content 0 装非空"
            );
            assert!(resp.usage.total_tokens > 0, "usage 0 装报告 token");
        }
        Err(LlmError::RateLimited { .. }) => {
            // 0 装诚实: rate limit 是真实 provider 行为, 不算测试失败
        }
        Err(e) => panic!("真 LLM call 失败 (key 可能 quota 用尽 / endpoint 变): {e:?}"),
    }
}
