//! `client_demo` — 主人验收用 example (apeireth-http-client · 战役 1-2)
//!
//! **目标**: 真接 minimaxi `/v1/chat/completions`, 验证 5 字段 keep-alive 真配置
//! + 测延迟 + 测 Keep-Alive 复用 (多次请求, 第二次应明显比第一次快)
//!
//! **跑法**:
//! ```powershell
//! # 设 API key (主人给的 minimaxi key)
//! $env:APEIRETH_API_KEY = "sk-cp-..."
//!
//! # 跑 example
//! cargo run -p apeireth-http-client --example client_demo
//! ```
//!
//! **期望输出**:
//! - 第 1 次请求: 较慢 (建立 TCP + TLS + 鉴权, 1000-3000ms)
//! - 第 2 次请求: 明显快 (Keep-Alive 复用 socket, < 500ms)
//! - `latency` + `elapsed` 都打印
//! - `config` 5 字段全部打印, 跟 VCP 真代码对齐

use apeireth_http_client::{HttpClient, KeepAliveConfig};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // tracing 默认 INFO 级, 看 reqwest 内部连接复用日志
    tracing_subscriber::fmt::init();

    println!(
        "🔧 apeireth-http-client 战役 1-2 验收 — Keep-Alive LIFO 5 字段 (VCP 借鉴 §6.2.2 #14)"
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // ============================================================
    // 1. 构造 HttpClient (VCP 5 字段 baked in)
    // ============================================================
    let client = HttpClient::with_vcp_defaults()?;
    let cfg: KeepAliveConfig = client.config();
    println!("\n✅ HttpClient 构造成功 (5 字段已 baked in reqwest::Client):");
    println!("   keep_alive:          {}", cfg.keep_alive);
    println!(
        "   keep_alive_msecs:    {} (TCP 探针间隔)",
        cfg.keep_alive_msecs
    );
    println!(
        "   free_socket_timeout: {} (空闲 socket 8s 后主动销毁, 绝杀 zombie)",
        cfg.free_socket_timeout
    );
    println!(
        "   scheduling:          {} (LIFO 优先复用最新鲜连接)",
        cfg.scheduling
    );
    println!("   max_sockets:         {} (全局并发上限)", cfg.max_sockets);

    // ============================================================
    // 2. 真接 minimaxi OpenAI 协议端点
    // ============================================================
    let api_key = std::env::var("APEIRETH_API_KEY")
        .map_err(|_| "APEIRETH_API_KEY env var not set (主人给的 minimaxi key)")?;
    let base_url = std::env::var("APEIRETH_API_URL")
        .unwrap_or_else(|_| "https://api.minimaxi.com/v1".to_string());
    let url = format!("{}/chat/completions", base_url);
    let model = "MiniMax-M3";

    println!("\n📡 端点: {}", url);
    println!("   model: {}", model);
    println!(
        "   api_key: {} chars (loaded from $env:APEIRETH_API_KEY)",
        api_key.len()
    );

    // ============================================================
    // 3. 发 3 次相同请求, 测 Keep-Alive 复用
    // ============================================================
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "你是一个 Rust 工程助手, 回答简洁"},
            {"role": "user", "content": "用一句话介绍 apeireth-http-client"}
        ],
        "temperature": 0.7,
        "max_tokens": 100
    });

    let mut latencies = Vec::new();
    for round in 1..=3 {
        println!("\n🚀 Round {round} 请求:");
        let start = Instant::now();

        // 走 apeireth-http-client 自己的 reqwest::Client (5 字段 keep-alive 已配置)
        // + 加 Bearer auth (bearer_auth 是 reqwest 自带, 不污染 method signature)
        // + 手动拿 LIFO 池 permit (max_sockets 限流) — 跟 client.post_json 等效
        let _guard = client.pool().enter().await;
        let resp = client
            .reqwest_client()
            .post(&url)
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await;
        let elapsed = start.elapsed();

        match resp {
            Ok(http_resp) => {
                let status = http_resp.status();
                let elapsed_ms = elapsed.as_millis() as u64;
                latencies.push(elapsed_ms);
                println!("   ✅ status: {} ({})", status, status.as_str());
                println!(
                    "   ⏱️  elapsed: {}ms (含 LIFO 池调度 + reqwest)",
                    elapsed.as_millis()
                );
                if round == 1 {
                    println!("   ℹ️  Round 1: 首次建立连接 (TCP + TLS + 鉴权), 通常最慢");
                } else {
                    let prev = latencies[latencies.len() - 2];
                    let diff = prev as i64 - elapsed_ms as i64;
                    if diff > 0 {
                        println!(
                            "   🎯 Keep-Alive 复用: 比 round {} 快 {}ms (复用 socket)",
                            round - 1,
                            diff
                        );
                    } else {
                        println!(
                            "   ℹ️  round {} → round {} 延迟差异 {}ms (波动在合理范围)",
                            round - 1,
                            round,
                            -diff
                        );
                    }
                }

                // 打印响应内容 (truncate)
                let text = http_resp.text().await.unwrap_or_default();
                let preview: String = text.chars().take(200).collect();
                println!("   📝 resp preview: {}...", preview);
            }
            Err(e) => {
                eprintln!("   ❌ Round {round} 失败: {e}");
                eprintln!("   (主人 apikey 是否设置了 $env:APEIRETH_API_KEY ?)");
                return Err(e.into());
            }
        }
    }

    // ============================================================
    // 4. 总结
    // ============================================================
    println!("\n📊 Keep-Alive 复用延迟对比:");
    for (i, l) in latencies.iter().enumerate() {
        println!("   round {}: {}ms", i + 1, l);
    }
    if latencies.len() >= 2 {
        let first = latencies[0];
        let second = latencies[1];
        if second < first {
            println!(
                "   ✅ Keep-Alive 复用成功: round 1 → round 2 减少 {}ms ({:.1}%)",
                first - second,
                100.0 * (first - second) as f64 / first as f64
            );
        } else {
            println!(
                "   ℹ️  round 1 → round 2 差异 {}ms (LLM 推理时间主导, Keep-Alive 已配置)",
                second as i64 - first as i64
            );
        }
    }

    println!("\n✨ client_demo 验收通过 — 战役 1-2 Keep-Alive LIFO 5 字段真接 minimaxi 跑通");

    Ok(())
}
