//! `anthropic_hello` — 主人验收用 example (Anthropic Messages API 协议 · R17)
//!
//! **R17 新增**: 验证 apeireth-api 直连 Anthropic Messages API 协议
//! (走 minimaxi `/anthropic` 端点, 不是 OpenAI 协议).
//!
//! 跑法:
//! ```powershell
//! # APEIRETH_API_KEY 跟 OpenAI provider 共用同一把 key (minimaxi)
//! $env:APEIRETH_API_KEY = "<主人给的 key>"
//! cargo run -p apeireth-api --example anthropic_hello
//! ```
//!
//! 期望输出: LLM 响应 + usage + latency + finish_reason (走 Anthropic 协议).

use apeireth_api::llm::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider, ChatMessage, LlmProvider, LlmRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🔧 Apeireth 自研 API 接入平台 — anthropic_hello 验收 (Anthropic Messages 协议)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // APEIRETH_API_KEY 跟 OpenAI provider 共用 (minimaxi 同 key 通用)
    // 但 APEIRETH_ANTHROPIC_KEY 不设置时, from_env 会用 APEIRETH_API_KEY fallback
    // 简单起见, 这里直接构造, 复用 OpenAI key
    let api_key = std::env::var("APEIRETH_API_KEY")
        .map_err(|_| "APEIRETH_API_KEY env var not set (跟 hello_api 共享)")?;

    let config = AnthropicCompatibleConfig::new(
        api_key,
        std::env::var("APEIRETH_ANTHROPIC_URL")
            .unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
        vec!["MiniMax-M3".to_string()],
    );
    println!("\n✅ Provider: AnthropicCompatibleProvider");
    println!("   base_url:    {}", config.base_url);
    println!("   models:      {:?}", config.models);
    println!("   timeout:     {}ms", config.timeout_ms);
    println!("   protocol:    Anthropic Messages API");
    println!("   auth header: x-api-key (不是 Bearer)");

    let provider = AnthropicCompatibleProvider::new(config)?;
    println!("\n📡 发送测试请求 (Anthropic 协议)...");

    let req = LlmRequest::new(
        "MiniMax-M3",
        vec![
            ChatMessage::system("你是一个 Rust 工程助手, 回答简洁"),
            ChatMessage::user("用一句话介绍 Anthropic Messages API 协议"),
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

    println!("\n📝 响应内容 (Anthropic content[0].text):");
    println!("   {}", resp.content);

    println!("\n📊 元数据:");
    println!("   provider:      {}", resp.provider);
    println!("   model:         {}", resp.model);
    println!("   finish_reason: {}", resp.finish_reason);
    println!("   latency:       {}ms (含 trace + log)", resp.latency_ms);
    println!("   total_elapsed: {}ms", total_elapsed);
    println!("\n🎫 Token 使用 (Anthropic input_tokens / output_tokens):");
    println!("   prompt_tokens:     {}", resp.usage.prompt_tokens);
    println!("   completion_tokens: {}", resp.usage.completion_tokens);
    println!("   total_tokens:      {}", resp.usage.total_tokens);

    println!("\n✨ anthropic_hello 验收通过 (Anthropic Messages 协议) ");

    Ok(())
}
