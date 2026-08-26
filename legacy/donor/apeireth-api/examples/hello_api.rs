//! `hello_api` — 主人验收用 example (Apeireth 自研 API 接入平台 · R17)
//!
//! **R17 改造后**: 默认直连 minimaxi 开放平台 OpenAI Chat Completion 协议端点
//! (`https://api.minimaxi.com/v1`), 不再依赖 NewAPI 进程.
//!
//! 跑法:
//! ```powershell
//! # 只设 key 即可, base_url 默认 minimaxi /v1
//! $env:APEIRETH_API_KEY = "<主人给的 key>"
//! cargo run -p apeireth-api --example hello_api
//!
//! # 想换别的 OpenAI-compatible 平台? 改 APEIRETH_API_URL
//! # $env:APEIRETH_API_URL = "https://api.openai.com/v1"
//! # $env:APEIRETH_API_MODELS = "gpt-4o,gpt-4o-mini"
//! ```
//!
//! 期望输出: LLM 响应 + usage + latency + finish_reason

use apeireth_api::llm::{
    ApeirethApiConfig, ApeirethApiProvider, ChatMessage, LlmProvider, LlmRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🔧 Apeireth 通用 API 扩展平台 — hello_api 验收");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let config = ApeirethApiConfig::from_env()?;
    println!("\n✅ Provider: ApeirethApiProvider");
    println!("   base_url: {}", config.base_url);
    println!("   models:   {:?}", config.models);
    println!("   timeout:  {}ms", config.timeout_ms);

    let provider = ApeirethApiProvider::new(config)?;
    println!("\n📡 发送测试请求...");

    let req = LlmRequest::new(
        &provider.models()[0],
        vec![
            ChatMessage::system("你是一个 Rust 工程助手, 回答简洁"),
            ChatMessage::user("用一句话介绍 Apeireth 通用 API 扩展平台"),
        ],
    )
    .with_temperature(0.7)
    .with_max_tokens(200);

    println!("   model: {}", req.model);
    println!("   temperature: {}", req.temperature);
    println!("   max_tokens: {}", req.max_tokens);

    let start = std::time::Instant::now();
    let resp = provider.complete(req).await?;
    let total_elapsed = start.elapsed().as_millis() as u64;

    println!("\n📝 响应内容:");
    println!("   {}", resp.content);

    println!("\n📊 元数据:");
    println!("   provider:      {}", resp.provider);
    println!("   model:         {}", resp.model);
    println!("   finish_reason: {}", resp.finish_reason);
    println!("   latency:       {}ms (含 trace + log)", resp.latency_ms);
    println!("   total_elapsed: {}ms", total_elapsed);
    println!("\n🎫 Token 使用:");
    println!("   prompt_tokens:     {}", resp.usage.prompt_tokens);
    println!("   completion_tokens: {}", resp.usage.completion_tokens);
    println!("   total_tokens:      {}", resp.usage.total_tokens);

    println!("\n✨ hello_api 验收通过");

    Ok(())
}
