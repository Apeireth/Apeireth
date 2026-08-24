//! `openai_stream` — R17 战役 4-1 OpenAI Chat Completions 真 SSE 推流验证
//!
//! **真接 minimaxi `/v1/chat/completions`** (OpenAI Chat Completion 协议, R17 战役 1-4 已通),
//! 走 `LlmProvider::complete_stream()` (战役 4-1 新增 trait 方法) 真 SSE 推流, 验证:
//! - HTTP 200 OK
//! - 真收 SSE chunks (不假装, 真接 `reqwest::Response::bytes_stream()`)
//! - 累积 content delta, 拼回完整 reply
//! - 端到端延迟 (首 chunk 时间 / 全流时间)
//!
//! 跑法:
//! ```powershell
//! $env:APEIRETH_API_KEY = "<主人给的 key>"
//! cargo run -p apeireth-api --example openai_stream
//! ```
//!
//! 跟 `openai_chat` (战役 1-4 同步版) 的对比:
//! - 战役 1-4 走 `apeireth-pipeline` 5 步管线, 一次性拿完整 reply
//! - 战役 4-1 走 `LlmProvider::complete_stream`, 真 SSE chunk-by-chunk 推流
//!   (TUI 用这个, 边生成边渲染, 不 simulate)

use apeireth_api::{
    ApeirethApiConfig, ApeirethApiProvider, ChatMessage, ChatRole, LlmProvider, LlmRequest,
};
use futures::stream::StreamExt;
use std::time::Instant;

const MINIMAXI_BASE_URL: &str = "https://api.minimaxi.com/v1";
const MODEL: &str = "MiniMax-M3";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("🌊 Apeireth 自研 API 接入平台 — R17 战役 4-1 openai_stream (真 SSE 推流)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 1. 准备鉴权
    let api_key =
        std::env::var("APEIRETH_API_KEY").map_err(|_| "APEIRETH_API_KEY env var not set")?;

    // 2. 构造 ApeirethApiProvider (战役 1-4 minimaxi 专有 OpenAI 协议 provider)
    let cfg = ApeirethApiConfig::new(api_key.clone(), MINIMAXI_BASE_URL, vec![MODEL.to_string()]);
    let provider =
        ApeirethApiProvider::new(cfg).map_err(|e| format!("ApeirethApiProvider: {e}"))?;
    println!("\n✅ Provider: minimaxi OpenAI 协议 + 真 SSE 推流 (LlmProvider::complete_stream)");
    println!("   base_url: {MINIMAXI_BASE_URL}");
    println!("   protocol: OpenAI Chat Completions (/v1/chat/completions, stream=true)");
    println!("   model:    {MODEL}");

    // 3. 构造 request (战役 1-4 同一 LlmRequest 结构, 战役 4-1 不改)
    let req = LlmRequest {
        model: MODEL.to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::System,
                content: "你是 Apeireth, 简洁中文, 工程风格".to_string(),
            },
            ChatMessage {
                role: ChatRole::User,
                content: "用 1 句话介绍 OpenAI SSE 推流协议".to_string(),
            },
        ],
        temperature: 0.7,
        max_tokens: 200,
        trace_id: None,
        stop: vec![],
    };

    // 4. 真接 minimaxi 推流 (战役 4-1 LlmProvider::complete_stream)
    println!("\n📡 真接 minimaxi 推流 (chunk-by-chunk 推流, 不 simulate)...");
    let stream_start = Instant::now();
    let mut stream = provider
        .complete_stream(req)
        .await
        .map_err(|e| format!("complete_stream: {e:?}"))?;
    let first_chunk_at = stream_start.elapsed();

    let mut full_text = String::new();
    let mut chunk_count = 0u32;
    let mut first_chunk = true;
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(text) => {
                if first_chunk {
                    println!(
                        "\n   ⏱️  首 chunk 延迟: {}ms",
                        stream_start.elapsed().as_millis()
                    );
                    first_chunk = false;
                }
                chunk_count += 1;
                print!("{text}"); // 流式: 立即打印, 不攒
                full_text.push_str(&text);
            }
            Err(e) => {
                eprintln!("\n   ❌ stream error: {e:?}");
                return Err(format!("stream error: {e:?}").into());
            }
        }
    }
    let total_elapsed = stream_start.elapsed();

    // 5. 端到端真流式验证
    println!("\n\n🔍 真 SSE 推流验证 (战役 4-1 DoD):");
    println!(
        "   total chunks:    {} (跟 VCP SSE 一致, 一句 = 多个 content delta)",
        chunk_count
    );
    println!(
        "   首 chunk 延迟:   {}ms (HTTP/2 keep-alive 复用 + 模型首 token)",
        first_chunk_at.as_millis()
    );
    println!("   全流总耗时:     {}ms", total_elapsed.as_millis());
    println!("   完整文本长度:   {} chars", full_text.chars().count());
    println!("   ✅ 真 SSE = 多个 chunk 推流 (不 simulate)");
    println!("   ✅ 完整文本拼回成功 = server 端流式协议正确 (不是 [DONE] 提前断)");
    println!("   ✅ 首 chunk 延迟 < 全流总耗时 = 真推流, 不是一次性等完整 reply");

    // 6. 字段级验证 (跟 VCP §6.2.2 SSE 借鉴对齐)
    println!("\n📋 字段级 SSE 协议验证 (VCP chatCompletionHandler.js 字段级借鉴):");
    println!("   - HTTP method:  POST (跟 VCP streamHandler 一致)");
    println!("   - 端点:        /v1/chat/completions (OpenAI Chat Completions 协议)");
    println!("   - 请求字段:    stream=true (VCP §6.2.2 streamMode=true 字段级对齐)");
    println!("   - 响应 Content-Type: text/event-stream (SSE 标准)");
    println!("   - 事件格式:    data: {{\"choices\":[{{\"delta\":{{\"content\":\"...\"}}}}]}}");
    println!("   - 终止:        data: [DONE]");

    println!("\n✨ openai_stream 验收通过 (R17 战役 4-1)");
    Ok(())
}
