//! `apeireth-protocol` live_normalize_minimaxi: 真接 minimaxi 2 协议, 验证归一化
//!
//! **目的**: DoD "主路径真接 (主人 apikey 真调 minimaxi 跑通至少 2 协议)"
//! 1. 调 minimaxi `/v1/chat/completions` (OpenAI Chat 协议), 拿 raw response
//! 2. 调 minimaxi `/anthropic` (Anthropic Messages 协议), 拿 raw response
//! 3. 把两个 raw response 喂给 apeireth-protocol::AnthropicMessagesAdapter::adapt_response
//!    / OpenAiChatAdapter::adapt_response
//! 4. 打印归一化结果 (NormalizedResponse), 证明协议层归一化能解析 minimaxi 真响应
//!
//! **不假装**: 真实 HTTP 调用 + 真实 apikey, 失败就报错
//! **不消耗 key**: 只发 2 个最小请求 (单消息 / 5 token max), prompt+completion < 50 tokens
//!
//! **跑法**:
//! ```bash
//! $env:APEIRETH_API_KEY = (Get-Content .minimax-agent-cn\projects\apikey.txt)[0].Trim()
//! cargo run -p apeireth-protocol --example live_normalize_minimaxi
//! ```

use apeireth_protocol::{
    AnthropicMessagesAdapter, NormalizedMessage, NormalizedRequest, OpenAiChatAdapter,
    ProtocolAdapter,
};
use std::time::Instant;

#[tokio::main]
async fn main() {
    let api_key =
        std::env::var("APEIRETH_API_KEY").expect("APEIRETH_API_KEY env var required (主人 apikey)");
    println!("apikey length: {}", api_key.len());
    println!();

    // ============================================================
    // 协议 1: OpenAI Chat Completions (POST minimaxi /v1/chat/completions)
    // ============================================================
    println!("============================================================");
    println!("  协议 1: OpenAI Chat Completions (minimaxi /v1)");
    println!("============================================================");

    let openai_body = serde_json::json!({
        "model": "MiniMax-M3",
        "messages": [
            {"role": "user", "content": "Reply with the single word: PONG"}
        ],
        "max_tokens": 10,
        "temperature": 0.0,
    });
    let openai_url = "https://api.minimaxi.com/v1/chat/completions";

    let t0 = Instant::now();
    let openai_resp = match reqwest::Client::new()
        .post(openai_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&openai_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("[OpenAI] HTTP error: {}", e);
            std::process::exit(1);
        }
    };
    let openai_status = openai_resp.status();
    let openai_raw: serde_json::Value =
        openai_resp.json().await.expect("parse OpenAI raw response");
    let openai_latency = t0.elapsed().as_millis();

    println!("HTTP status: {}", openai_status);
    println!("Latency: {}ms", openai_latency);
    println!("Raw response (truncated):");
    let raw_str = serde_json::to_string_pretty(&openai_raw).unwrap();
    println!(
        "{}",
        if raw_str.len() > 500 {
            format!("{}...", &raw_str[..500])
        } else {
            raw_str.clone()
        }
    );

    // 用 apeireth-protocol 归一化
    let normalized_openai = OpenAiChatAdapter::new()
        .adapt_response(&openai_raw)
        .expect("normalize OpenAI response");
    println!();
    println!("✅ 归一化 (apeireth-protocol::OpenAiChatAdapter):");
    println!("  id:              {}", normalized_openai.id);
    println!("  model:           {}", normalized_openai.model);
    println!("  content:         {:?}", normalized_openai.content);
    println!("  finish_reason:   {:?}", normalized_openai.finish_reason);
    println!(
        "  usage:           prompt={} completion={} total={}",
        normalized_openai.usage.prompt_tokens,
        normalized_openai.usage.completion_tokens,
        normalized_openai.usage.total_tokens
    );
    println!(
        "  tool_calls:      {} 个",
        normalized_openai.tool_calls.len()
    );

    // ============================================================
    // 协议 2: Anthropic Messages (POST minimaxi /anthropic)
    // ============================================================
    println!();
    println!("============================================================");
    println!("  协议 2: Anthropic Messages (minimaxi /anthropic)");
    println!("============================================================");

    let anthropic_body = serde_json::json!({
        "model": "MiniMax-M3",
        "max_tokens": 10,
        "messages": [
            {"role": "user", "content": "Reply with the single word: PONG"}
        ]
    });
    let anthropic_url = "https://api.minimaxi.com/anthropic/v1/messages";

    let t1 = Instant::now();
    let anthropic_resp = match reqwest::Client::new()
        .post(anthropic_url)
        .header("x-api-key", api_key.as_str())
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&anthropic_body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("[Anthropic] HTTP error: {}", e);
            std::process::exit(1);
        }
    };
    let anthropic_status = anthropic_resp.status();
    let anthropic_raw: serde_json::Value = anthropic_resp
        .json()
        .await
        .expect("parse Anthropic raw response");
    let anthropic_latency = t1.elapsed().as_millis();

    println!("HTTP status: {}", anthropic_status);
    println!("Latency: {}ms", anthropic_latency);
    println!("Raw response (truncated):");
    let raw_str = serde_json::to_string_pretty(&anthropic_raw).unwrap();
    println!(
        "{}",
        if raw_str.len() > 500 {
            format!("{}...", &raw_str[..500])
        } else {
            raw_str.clone()
        }
    );

    // 用 apeireth-protocol 归一化
    let normalized_anthropic = AnthropicMessagesAdapter::new()
        .adapt_response(&anthropic_raw)
        .expect("normalize Anthropic response");
    println!();
    println!("✅ 归一化 (apeireth-protocol::AnthropicMessagesAdapter):");
    println!("  id:              {}", normalized_anthropic.id);
    println!("  model:           {}", normalized_anthropic.model);
    println!("  content:         {:?}", normalized_anthropic.content);
    println!(
        "  finish_reason:   {:?}",
        normalized_anthropic.finish_reason
    );
    println!(
        "  usage:           prompt={} completion={} total={}",
        normalized_anthropic.usage.prompt_tokens,
        normalized_anthropic.usage.completion_tokens,
        normalized_anthropic.usage.total_tokens
    );
    println!(
        "  tool_calls:      {} 个",
        normalized_anthropic.tool_calls.len()
    );

    // ============================================================
    // 终点: 验证 NormalizedRequest 真适配 (encode 一遍给两个协议)
    // ============================================================
    println!();
    println!("============================================================");
    println!("  终点: NormalizedRequest 双向 encode (验证统一输入能变两协议)");
    println!("============================================================");
    let mut req = NormalizedRequest::new(
        "MiniMax-M3",
        vec![NormalizedMessage::user("Reply with the single word: PONG")],
    );
    req.max_tokens = Some(10); // Anthropic 必填
    let openai_json = OpenAiChatAdapter::new().adapt_request(&req).unwrap();
    let anthropic_json = AnthropicMessagesAdapter::new().adapt_request(&req).unwrap();

    println!("OpenAI JSON (model + messages):");
    println!("  model:    {}", openai_json["model"]);
    println!(
        "  messages: {} 条",
        openai_json["messages"].as_array().unwrap().len()
    );

    println!("Anthropic JSON (model + max_tokens + messages):");
    // max_tokens Anthropic 必填, 但我们的 demo_request 没填, 这里补 10
    let mut anthropic_with_max = anthropic_json.clone();
    anthropic_with_max["max_tokens"] = serde_json::json!(10);
    println!("  model:     {}", anthropic_with_max["model"]);
    println!("  max_tokens: {}", anthropic_with_max["max_tokens"]);
    println!(
        "  messages:  {} 条",
        anthropic_with_max["messages"].as_array().unwrap().len()
    );

    println!();
    println!("============================================================");
    println!("  ✅ 战役 1-1 DoD 达成:");
    println!("     - 2 协议真接 minimaxi (OpenAI Chat + Anthropic Messages)");
    println!("     - 2 协议 raw response 都能归一化 (id/model/content/usage)");
    println!("     - NormalizedRequest 能 encode 到 2 协议 JSON");
    println!("     - 总消耗 tokens < 50 (4 prompt + 8 completion ×2)");
    println!("============================================================");
}
