//! R128: CLI minimax chat example — real OpenAI Chat Completions POST.
//!
//! 走 apeireth-api 5 步 pipeline + Keep-Alive LIFO, 让 shell pipeline 能直接吃 LLM.
//!
//! 运行 (PowerShell):
//! ```powershell
//! $env:APEIRETH_API_KEY = "<key>"
//! cargo run -p apeireth-cli --example minimax_chat -- "What is Rust async?"
//! ```
use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::ProtocolKind;
use apeireth_api::Pipeline;
use std::sync::Arc;
use std::time::Instant;
use serde_json::json;

const MINIMAXI_BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("APEIRETH_API_KEY")
        .map_err(|_| "APEIRETH_API_KEY env var not set")?;
    let prompt = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if prompt.is_empty() {
        eprintln!("usage: cargo run -p apeireth-cli --example minimax_chat -- <prompt>");
        std::process::exit(2);
    }
    let pipeline: Arc<Pipeline> =
        Arc::new(build_pipeline(MINIMAXI_BASE_URL.to_string(), Some(api_key.clone()))?);
    println!("=== minimax chat ===
  base_url: {}
  model:    {}
  prompt:   {}
", MINIMAXI_BASE_URL, MODEL, prompt);

    let req = OpenAiChatRequest {
        model: MODEL.to_string(),
        messages: vec![
            OpenAiChatMessage {
                role: "system".to_string(),
                content: json!("You are a concise Rust engineering assistant."),
                tool_calls: None,
                tool_call_id: None,
            },
            OpenAiChatMessage {
                role: "user".to_string(),
                content: json!(prompt),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        temperature: Some(0.7),
        max_tokens: Some(512),
        stream: false,
        stop: None,
        tools: None,
        tool_choice: None,
    };
    let normalized = openai_chat_to_normalized(&req);
    let start = Instant::now();
    let resp = dispatch(&pipeline, ProtocolKind::OpenAiChat, normalized).await?;
    let latency_ms = start.elapsed().as_millis();
    let chat_resp = openai_chat_from_normalized(&resp);
    for c in &chat_resp.choices {
        if !c.message.content.is_empty() {
            println!("
> {}
", c.message.content);
        }
    }
    println!(
        "usage: prompt={} completion={} total={} latency_ms={}",
        chat_resp.usage.prompt_tokens,
        chat_resp.usage.completion_tokens,
        chat_resp.usage.total_tokens,
        latency_ms
    );
    Ok(())
}
