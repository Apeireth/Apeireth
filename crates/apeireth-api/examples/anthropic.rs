//! `anthropic` — R17 战役 1-4 Anthropic Messages API 端到端验证
//!
//! 走 `/v1/messages` 端点 (minimaxi 接受 Bearer 鉴权的 Anthropic 协议),
//! 顶层 `system` 字段, `messages` 数组, 响应 `content[]` 是块数组。
//!
//! 跟 `openai_chat` 对比: 鉴权 Bearer (minimaxi proxy 接受), 消息格式不同
//!
//! 跑法:
//! ```powershell
//! $env:APEIRETH_API_KEY = "<主人给的 key>"
//! cargo run -p apeireth-api --example anthropic
//! ```

use apeireth_api::protocol_handlers::{
    anthropic_from_normalized, anthropic_to_normalized, build_pipeline, AnthropicRequest,
};
use apeireth_api::ProtocolKind;
use serde_json::json;
use std::time::Instant;

const MINIMAXI_BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🔧 Apeireth 自研 API 接入平台 — R17 战役 1-4 anthropic");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let api_key =
        std::env::var("APEIRETH_API_KEY").map_err(|_| "APEIRETH_API_KEY env var not set")?;

    let pipeline = build_pipeline(MINIMAXI_BASE_URL.to_string(), Some(api_key.clone()))?;
    println!("\n✅ Pipeline: VCP 5 字段 Keep-Alive LIFO + 5 步管线");
    println!("   base_url: {MINIMAXI_BASE_URL}");
    println!("   protocol: Anthropic Messages API (/v1/messages)");
    println!("   model:    {MODEL}");

    // Anthropic Messages 风格: 顶层 system + messages 数组
    let req = AnthropicRequest {
        model: MODEL.to_string(),
        system: Some("你是一个 Rust 工程助手, 回答简洁".to_string()),
        messages: vec![apeireth_api::protocol_handlers::AnthropicMessage {
            role: "user".to_string(),
            content: json!("用一句话介绍 Anthropic Messages API 协议"),
            tool_call_id: None,
        }],
        max_tokens: 200, // Anthropic 必填
        temperature: Some(0.7),
        stream: false,
        stop_sequences: None,
        tools: None,
    };

    let normalized = anthropic_to_normalized(&req);
    println!("\n📡 发送请求 (Anthropic Messages API 协议)...");
    let start = Instant::now();
    let normalized_resp = apeireth_api::protocol_handlers::dispatch(
        &pipeline,
        ProtocolKind::AnthropicMessages,
        normalized,
    )
    .await
    .map_err(|e| format!("dispatch: {e}"))?;
    let latency = start.elapsed().as_millis();

    let resp = anthropic_from_normalized(&normalized_resp);

    println!("\n📝 响应 (content[0].text):");
    println!("   id:     {}", resp.id);
    println!("   type:   {}", resp.kind);
    println!("   model:  {}", resp.model);
    println!(
        "   text:   {}",
        resp.content[0]["text"].as_str().unwrap_or("")
    );
    println!("   stop_reason: {}", resp.stop_reason);
    println!("\n📊 元数据:");
    println!("   latency: {}ms", latency);
    println!("\n🎫 Token 使用 (Anthropic input/output):");
    println!("   input_tokens:  {}", resp.usage.input_tokens);
    println!("   output_tokens: {}", resp.usage.output_tokens);

    println!("\n✨ anthropic 验收通过 (R17 战役 1-4)");
    Ok(())
}
