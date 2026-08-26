// LLM 真接测试 — 调 minimaxi 看真实返回
// 跑法: cargo run -p apeireth-tui --example test_llm

use std::time::Instant;

use apeireth_api::{
    ChatMessage, ChatRole, LlmProvider, LlmRequest, OpenAiCompatibleConfig,
    OpenAiCompatibleProvider,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let key = std::fs::read_to_string("C:\\Users\\REDACTED\\.openclaw\\apikey.txt")
        .expect("read apikey")
        .trim()
        .to_string();
    println!("apikey (前 8): {}...", &key[..key.len().min(8)]);
    println!("apikey 长度: {}", key.len());

    let cfg = OpenAiCompatibleConfig::new(
        "minimaxi",
        "https://api.minimaxi.com/v1",
        key,
        vec!["MiniMax-M3".to_string()],
    );
    let provider = OpenAiCompatibleProvider::new(cfg).expect("provider");

    let prompt = "用一句话告诉我你是谁。";
    println!("\n=== request ===");
    println!("provider: minimaxi");
    println!("model:    MiniMax-M3");
    println!("prompt:   {}", prompt);

    let req = LlmRequest {
        model: "MiniMax-M3".to_string(),
        messages: vec![
            ChatMessage::system(
                "你是 Apeireth 主 AI,基于 R19 立体架构。回答用中文,简洁直接,工程风格。",
            ),
            ChatMessage::user(prompt.to_string()),
        ],
        temperature: 0.7,
        max_tokens: 256,
        trace_id: None,
        stop: vec![],
    };

    let start = Instant::now();
    let resp = provider.complete(req).await;
    let elapsed = start.elapsed();

    match resp {
        Ok(r) => {
            println!("\n[OK] latency: {:.2}s", elapsed.as_secs_f64());
            println!("content: {}", r.content);
            println!(
                "usage:   prompt={} completion={} total={}",
                r.usage.prompt_tokens, r.usage.completion_tokens, r.usage.total_tokens
            );
            println!("finish:  {}", r.finish_reason);
            println!("model:   {}", r.model);
        }
        Err(e) => {
            println!("\n[ERR] latency: {:.2}s", elapsed.as_secs_f64());
            println!("error: {:?}", e);
        }
    }
}
