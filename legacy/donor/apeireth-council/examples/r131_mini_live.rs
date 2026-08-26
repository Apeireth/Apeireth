//! R131.1 mini 真接验证: 单 member + 单 LLM call, 同步 block_on 路径 vs 异步直跑路径
//!
//! 跑法:
//! ```powershell
//! $env:APEIRETH_MINIMAX_LIVE_TEST = "1"
//! $env:APEIRETH_MINIMAX_API_KEY = (Get-Content .openclaw\apikey.txt).Trim()
//! $env:APEIRETH_MINIMAX_URL = "https://api.minimaxi.com/anthropic"
//! cargo run -p apeireth-council --release --example r131_mini_live
//! ```

use apeireth_api::llm::providers::anthropic_compat::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider,
};
use apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest};
use std::time::Instant;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() != "1" {
        println!("[skip] set APEIRETH_MINIMAX_LIVE_TEST=1");
        return Ok(());
    }
    let key = std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_default();
    let cfg = AnthropicCompatibleConfig::new(
        key,
        std::env::var("APEIRETH_MINIMAX_URL")
            .unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
        vec!["MiniMax-M3".to_string()],
    );
    let p = AnthropicCompatibleProvider::new(cfg)?;
    let prompt = "Reply with exactly one of: StrongApprove / Approve / Neutral / Disapprove / StrongDisapprove / Abstain. Then 5-word reason.".to_string();

    println!("=== R131.1 mini 5 连发 (async current_thread) ===");
    for i in 0..5 {
        let t0 = Instant::now();
        let r = p
            .complete(LlmRequest::new(
                "MiniMax-M3",
                vec![ChatMessage::user(prompt.clone())],
            ))
            .await?;
        println!("[{}] {}ms {:?}", i + 1, t0.elapsed().as_millis(), r.content);
    }
    Ok(())
}
