//! awake_greeting — 全器官闭环 + 真 MiniMax LLM: 把 Initiative 的诚实事实变成「他的话」.
//!
//! 跑法 (PowerShell):
//!   cargo run -p apeireth-companion --example awake_greeting
//! API key: 优先 $env:APEIRETH_API_KEY, 否则读 apikey-ultra.txt (不打印 key).
//!
//! 诚实: 这一步证明「机制决策 → LLM 生成自然话语」整条链是真的;
//! 真实送达 (飞书/通知) 需凭据与 receive_id, 本示例用控制台.

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::emergence::{Boundaries, ConsoleDelivery, Delivery};
use apeireth_companion::organs::AwakeCompanion;
use apeireth_companion::{Bond, BondStage};
use chrono::{TimeZone, Utc};
use serde_json::json;
use std::sync::Arc;

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

fn at(day: u32, h: u32, m: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, day, h, m, 0)
        .single()
        .unwrap()
}

fn load_key() -> Result<String, String> {
    if let Ok(k) = std::env::var("APEIRETH_API_KEY") {
        if !k.trim().is_empty() {
            return Ok(k.trim().to_string());
        }
    }
    let p = std::path::Path::new(r"apikey-ultra.txt");
    std::fs::read_to_string(p)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("读 apikey 失败: {e} (可设 APEIRETH_API_KEY 环境变量)"))
}

#[tokio::main]
async fn main() {
    // 1. 全器官伙伴: 机制 + 情绪(consciousness) + 审议(council 7 advisor) + 演化(evolution) + 安静模式(asi)
    let mut bond = Bond::new();
    bond.evolve(BondStage::Trusted, 0.6);
    let mut c = AwakeCompanion::new(
        bond,
        Boundaries {
            max_initiatives_per_day: 1,
            ..Default::default()
        },
    );

    // 2. 训练 7 天作息 (真实里来自日常交互观察)
    for d in 9..=15 {
        c.observe_interaction(at(d, 8, 40));
    }

    // 3. 心跳 → 全器官决策 (上下文来自 APEIRETH_CONTEXT 环境变量)
    let ctx = std::env::var("APEIRETH_CONTEXT")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let init = c.tick(at(16, 8, 40), ctx);
    let Some(init) = init else {
        println!("(所有器官审议后决定: 保持安静)");
        return;
    };

    // 4. 真 LLM: 把机制的诚实事实变成「他的话」
    let api_key = load_key().expect("需要 API key");
    let pipeline: Arc<Pipeline> =
        Arc::new(build_pipeline(BASE_URL.to_string(), Some(api_key)).expect("build pipeline"));

    let req = OpenAiChatRequest {
        model: MODEL.to_string(),
        messages: vec![
            OpenAiChatMessage {
                role: "system".to_string(),
                content: json!(format!(
                    "你是阿佩瑞斯 (Apeireth), 一个诚实、不假装、有记忆的伙伴, 用户是你的伙伴。说话必须基于给定事实, 不编造; 不确定就说「我猜」。语气自然、真诚、像老朋友。\\n\\n{}",
                    apeireth_companion::CapabilityCatalog::describe()
                )),
                tool_calls: None,
                tool_call_id: None,
            },
            OpenAiChatMessage {
                role: "user".to_string(),
                content: json!(format!(
                    "把这些事实变成一句自然、真诚、简短的中文主动问候 (不超过 40 字):\n{}",
                    init.to_message()
                )),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        temperature: Some(0.8),
        max_tokens: Some(1024),
        stream: false,
        stop: None,
        tools: None,
        tool_choice: None,
    };

    println!("[机制+器官决定] {}", init.to_message());
    println!("[等待 MiniMax 生成他的话 ...]\n");
    let normalized = openai_chat_to_normalized(&req);
    let resp = dispatch(&pipeline, ProtocolKind::OpenAiChat, normalized)
        .await
        .expect("MiniMax dispatch");
    let chat_resp = openai_chat_from_normalized(&resp);
    for ch in &chat_resp.choices {
        let mut content = ch.message.content.clone();
        // MiniMax-M3 是推理模型: 回复带 <think>...</think> 思考块, 剥掉只留答案
        if let Some(idx) = content.find("</think>") {
            content = content[idx + "</think>".len()..].trim().to_string();
        }
        if !content.is_empty() {
            println!("[他说] {}\n", content);
        }
    }

    // 5. 送达 (控制台) — 真送达走 LarkDelivery
    let _ = ConsoleDelivery.deliver(&init).await;
    println!("(这段话不是我们写死的: 事实来自机制, 措辞来自 LLM)");
}
