//! `openai_responses` — R17 战役 1-4 OpenAI Responses API 端到端验证
//!
//! 跟 `openai_chat` 不同: 走 `/v1/responses` 端点 (codex 风格),
//! 顶层 `instructions` 替代 system message, `input` 数组替代 `messages` 数组,
//! 响应 `output[]` 是结构化数组。
//!
//! 走 `apeireth_pipeline::Pipeline` 5 步管线, 验证 minimaxi `/v1/responses` 200 OK。
//!
//! 跑法:
//! ```powershell
//! $env:APEIRETH_API_KEY = "<主人给的 key>"
//! cargo run -p apeireth-api --example openai_responses
//! ```

use apeireth_api::protocol_handlers::{
    build_pipeline, openai_responses_from_normalized, openai_responses_to_normalized,
    OpenAiResponsesRequest,
};
use apeireth_api::ProtocolKind;
use serde_json::json;
use std::time::Instant;

const MINIMAXI_BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🔧 Apeireth 自研 API 接入平台 — R17 战役 1-4 openai_responses");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let api_key =
        std::env::var("APEIRETH_API_KEY").map_err(|_| "APEIRETH_API_KEY env var not set")?;

    let pipeline = build_pipeline(MINIMAXI_BASE_URL.to_string(), Some(api_key.clone()))?;
    println!("\n✅ Pipeline: VCP 5 字段 Keep-Alive LIFO + 5 步管线");
    println!("   base_url: {MINIMAXI_BASE_URL}");
    println!("   protocol: OpenAI Responses API (/v1/responses) — codex 风格");
    println!("   model:    {MODEL}");

    // OpenAI Responses API 风格: 顶层 instructions + input 数组
    let req = OpenAiResponsesRequest {
        model: MODEL.to_string(),
        input: json!([
            {"role": "user", "content": "用一句话介绍 OpenAI Responses API 协议"},
        ]),
        instructions: Some("你是一个 Rust 工程助手, 回答简洁".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(200),
        stream: false,
        tools: None,
        tool_choice: None,
    };

    let normalized = openai_responses_to_normalized(&req);
    println!("\n📡 发送请求 (OpenAI Responses API 协议)...");
    let start = Instant::now();
    let normalized_resp = apeireth_api::protocol_handlers::dispatch(
        &pipeline,
        ProtocolKind::OpenAiResponses,
        normalized,
    )
    .await
    .map_err(|e| format!("dispatch: {e}"))?;
    let latency = start.elapsed().as_millis();

    let resp = openai_responses_from_normalized(&normalized_resp);

    println!("\n📝 响应 (output[0].content[0].text):");
    println!("   id:     {}", resp.id);
    println!("   model:  {}", resp.model);
    println!("   status: {}", resp.status);
    println!(
        "   text:   {}",
        resp.output[0]["content"][0]["text"].as_str().unwrap_or("")
    );
    println!("\n📊 元数据:");
    println!("   latency: {}ms", latency);
    println!("\n🎫 Token 使用:");
    println!("   input_tokens:  {}", resp.usage.input_tokens);
    println!("   output_tokens: {}", resp.usage.output_tokens);
    println!("   total_tokens:  {}", resp.usage.total_tokens);

    println!("\n✨ openai_responses 验收通过 (R17 战役 1-4)");
    Ok(())
}
