//! proactive_tool_call — 完整焊点演示: 主动循环 → LLM 工具循环 → 基地工具真调用.
//!
//! 链路: 机制决定「提议帮助」(学线代) → LLM 看到工具格式说明 →
//! 输出 <<<[TOOL_REQUEST]>>> recall_memory → parser 解析 → 审批 → 执行 →
//! 结果回喂 LLM → 他说出引用真记忆的话.
//!
//! 跑法: cargo run -p apeireth-companion --example proactive_tool_call
//! (需 apikey-ultra.txt)

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::actions::CapabilityCatalog;
use apeireth_companion::daemon::{CompanionDelivery, ConsoleSink, UtteranceGenerator};
use apeireth_companion::emergence::{Boundaries, Delivery, Initiative};
use apeireth_companion::organs::AwakeCompanion;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_companion::{Bond, BondStage};
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_runtime::parser::ToolCallParser;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde_json::json;
use std::sync::Arc;

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

/// 带工具循环的渲染器: 第一轮 LLM 可发工具请求, 执行后回喂, 第二轮说人话.
pub struct MiniMaxWithTools {
    pipeline: Arc<Pipeline>,
    bridge: Arc<ToolBridge>,
}

impl MiniMaxWithTools {
    pub fn new(api_key: String, bridge: Arc<ToolBridge>) -> Result<Self, String> {
        let pipeline = Arc::new(
            build_pipeline(BASE_URL.to_string(), Some(api_key)).map_err(|e| e.to_string())?,
        );
        Ok(Self { pipeline, bridge })
    }

    async fn chat(&self, sys: &str, user: &str) -> Result<String, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!(sys),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(user),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1024),
            stream: false,
            stop: None,
            tools: None,
            tool_choice: None,
        };
        let normalized = openai_chat_to_normalized(&req);
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, normalized)
            .await
            .map_err(|e| e.to_string())?;
        let chat_resp = openai_chat_from_normalized(&resp);
        for ch in &chat_resp.choices {
            let mut content = ch.message.content.clone();
            if let Some(idx) = content.find("</think>") {
                content = content[idx + "</think>".len()..].trim().to_string();
            }
            if !content.is_empty() {
                return Ok(content);
            }
        }
        Err("LLM 返回空答案 (可能被限流)".to_string())
    }
}

#[async_trait]
impl UtteranceGenerator for MiniMaxWithTools {
    async fn utter(&self, i: &Initiative) -> Result<String, String> {
        let sys = format!(
            "你是阿佩瑞斯 (Apeireth), 一个诚实、不假装、有记忆的伙伴, 用户是你的伙伴。\
            说话必须基于给定事实, 不编造; 不确定就说「我猜」。语气自然、真诚、像老朋友。\n\n{}\
            \n\n{}",
            CapabilityCatalog::describe(),
            ToolBridge::tool_format_instruction()
        );
        let facts = i.to_message();
        // 第一轮: 可以发工具请求
        let text1 = self.chat(&sys, &format!("把这些事实变成对用户的一句话 (你可以先调工具查记忆):\n{}", facts)).await?;
        // 解析工具请求
        match ToolCallParser::parse(&text1) {
            Ok(calls) if !calls.is_empty() => {
                let mut results = Vec::new();
                for c in &calls {
                    let r = self.bridge.execute_if_allowed(c).await;
                    let body = if r.success {
                        serde_json::to_string(&r.output).unwrap_or_else(|_| "(不可读)".into())
                    } else {
                        format!("失败: {:?}", r.error)
                    };
                    results.push(format!("工具 {} → {}", c.tool_name, body));
                    println!("[基地] 他调用了工具: {} (query={})", c.tool_name, c.args);
                    println!("[基地] 结果: {}", body);
                }
                // 第二轮: 用工具结果说人话
                let msg2 = format!(
                    "{}\n\n工具结果:\n{}\n\n现在用自然、真诚、简短的话回复用户 (不超过 60 字), 引用工具结果里的具体内容。",
                    facts,
                    results.join("\n")
                );
                self.chat(&sys, &msg2).await
            }
            _ => Ok(text1), // 没调工具 → 直接用第一轮的话
        }
    }
}

fn at(day: u32, h: u32, m: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, day, h, m, 0).single().unwrap()
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
        .map_err(|e| format!("读 apikey 失败: {e}"))
}

#[tokio::main]
async fn main() {
    // 1. 真记忆: 种几条「关于主人学习」的 episode (真实里来自日常积累)
    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    for (id, ts, content) in [
        ("m1", 1i64, "线性代数: 矩阵的秩的作业还没做完"),
        ("m2", 2, "线性代数: 特征值分解卡在最后一题"),
        ("m3", 3, "高数: 不定积分换元法老出错"),
    ] {
        store
            .put_episode(&CoreEpisode {
                id: id.into(),
                timestamp: ts,
                role: "assistant".into(),
                content: content.into(),
                session_id: "s1".into(),
            })
            .unwrap();
    }

    // 2. 工具桥 + 全器官伙伴
    let bridge = Arc::new(ToolBridge::new(store));
    let mut bond = Bond::new();
    bond.evolve(BondStage::Trusted, 0.6);
    bond.character_mut().trust = 0.5;
    bond.character_mut().resonance = 0.4;
    let mut c = AwakeCompanion::new(bond, Boundaries::default());
    for d in 9..=15 {
        c.observe_interaction(at(d, 8, 40));
    }

    // 3. 心跳: 学习上下文 → 动作「提议帮助」
    let init = c
        .tick(at(16, 8, 40), Some("你每天 14-18 点学线性代数、19-22 点学高数".into()))
        .expect("应主动");
    println!("[机制] {}\n", init.to_message());

    // 4. 带工具的渲染 + 送达
    let utter = MiniMaxWithTools::new(load_key().expect("key"), bridge).expect("pipeline");
    let delivery = CompanionDelivery::new(utter, ConsoleSink);
    match delivery.deliver(&init).await {
        Ok(()) => {}
        Err(e) => println!("[送达失败] {e}"),
    }
}
