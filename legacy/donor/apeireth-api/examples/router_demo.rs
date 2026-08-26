//! `router_demo` — MultiLlmRouter + 中间件演示
//!
//! 跑法: cargo run -p apeireth-api --example router_demo
//!
//! 不需要任何 API key (用 ScriptedLlmProvider mock)

use std::sync::Arc;

use apeireth_api::llm::{
    middleware::{LoggingMiddleware, MiddlewareChain, RetryMiddleware},
    providers::scripted::{ScriptedLlmProvider, ScriptedResponse},
    router::MultiLlmRouter,
    ChatMessage, LlmProvider, LlmRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🔧 Apeireth 通用 API 扩展平台 — router_demo");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 1. 两个 scripted provider
    let primary = Arc::new(
        ScriptedLlmProvider::new("primary")
            .with_script("hello", ScriptedResponse::new("from PRIMARY")),
    ) as Arc<dyn LlmProvider>;

    let fallback = Arc::new(
        ScriptedLlmProvider::new("fallback")
            .with_script("hello", ScriptedResponse::new("from FALLBACK"))
            .with_script("goodbye", ScriptedResponse::new("from FALLBACK (goodbye)")),
    ) as Arc<dyn LlmProvider>;

    let router = MultiLlmRouter::new()
        .with_provider(primary)
        .with_provider(fallback)
        .with_fallback(vec!["primary".into(), "fallback".into()]);

    println!("\n📋 Router 状态:");
    println!("   providers: {:?}", router.provider_names());
    println!("   count:     {}", router.provider_count());

    let chain = MiddlewareChain::new()
        .with(Arc::new(LoggingMiddleware::new()))
        .with(Arc::new(RetryMiddleware::default()));
    println!("\n🔗 Middleware chain: {} 个中间件", chain.len());

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📝 测试 1: 'hello' 关键词");
    let req = LlmRequest::new("m", vec![ChatMessage::user("hello world")]);
    let resp = chain
        .run(Arc::new(router) as Arc<dyn LlmProvider>, req)
        .await?;
    println!("✅ 响应: {}", resp.content);
    println!("   provider: {}", resp.provider);

    println!("\n✨ router_demo 跑通");

    Ok(())
}
