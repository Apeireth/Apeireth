//! companion_daemon — 总装: 全器官 + 真 MiniMax 渲染 + 生产记忆 (真 SQLite) + 通道, 常驻运行.
//!
//! 环境变量:
//!   APEIRETH_API_KEY              MiniMax key (或读 apikey-ultra.txt)
//!   APEIRETH_TICK_SECS            心跳间隔秒 (默认 60)
//!   APEIRETH_MAX_TICKS            跑 N 轮后退出 (默认无限)
//!   APEIRETH_SEED_DEMO=1          demo 种子: 预填最近 7 天「现在这个时刻」的作息观察 (诚实标注)
//!   APEIRETH_MEMORY_PATH          记忆库 SQLite 路径 (默认 %APPDATA%\apeireth\memory.sqlite)
//!   APEIRETH_SUBJECT              记忆检索用的 continuity/session id (默认 "me")
//!   APEIRETH_MIN_LLM_INTERVAL_SECS 两次主动 (LLM 渲染) 最短间隔秒 (默认 60)
//!   APEIRETH_SINK=lark            送达通道: lark → 飞书 (需 APEIRETH_LARK_* 凭据), 默认 console
//!   APEIRETH_LARK_APP_ID          (APEIRETH_SINK=lark) 飞书应用 App ID
//!   APEIRETH_LARK_APP_SECRET      (APEIRETH_SINK=lark) 飞书应用 App Secret
//!   APEIRETH_LARK_RECEIVE_ID      (APEIRETH_SINK=lark) 会话/用户 open_id
//!   APEIRETH_LARK_BASE_URL        (可选, 默认 https://open.feishu.cn/open-apis)
//!
//! stdin: 任意内容 = 一次用户交互; "r" = 回应上次主动; "quit" = 退出.

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::daemon::{
    CompanionDaemon, CompanionDelivery, ConsoleSink, LarkSink, ThrottledUtterance,
    UtteranceGenerator, open_memory_store,
};
use apeireth_companion::emergence::{Boundaries, Delivery, Initiative, LoopConfig};
use apeireth_companion::proactive::MemoryContextSource;
use apeireth_companion::{AwakeCompanion, Bond, BondStage};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

/// 真 MiniMax 渲染: 机制事实 → 自然的话 (推理模型, 剥 <think> 块, 失败兜底原文).
pub struct MiniMaxUtterance {
    pipeline: Arc<Pipeline>,
}

impl MiniMaxUtterance {
    pub fn new(api_key: String) -> Result<Self, String> {
        let pipeline = Arc::new(
            build_pipeline(BASE_URL.to_string(), Some(api_key)).map_err(|e| e.to_string())?,
        );
        Ok(Self { pipeline })
    }
}

#[async_trait]
impl UtteranceGenerator for MiniMaxUtterance {
    async fn utter(&self, i: &Initiative) -> Result<String, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!("你是阿佩瑞斯 (Apeireth), 一个诚实、不假装、有记忆的伙伴, 用户是你的伙伴。说话必须基于给定事实, 不编造; 不确定就说「我猜」。语气自然、真诚、像老朋友。"),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(format!(
                        "把这些事实变成一句自然、真诚、简短的中文主动问候 (不超过 40 字):\n{}",
                        i.to_message()
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
        Ok(i.to_message()) // 兜底: 机制的诚实原文
    }
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
        .map_err(|e| format!("读 apikey 失败: {e} (可设 APEIRETH_API_KEY)"))
}

#[tokio::main]
async fn main() {
    let tick_secs = std::env::var("APEIRETH_TICK_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);
    let max_ticks = std::env::var("APEIRETH_MAX_TICKS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());

    let mut bond = Bond::new();
    bond.evolve(BondStage::Trusted, 0.6);

    // 生产记忆: 真 SQLite 路径 (APEIRETH_MEMORY_PATH 或默认用户数据目录), 空库从零开始长
    let store = open_memory_store().expect("打开生产记忆库失败");
    let context = MemoryContextSource::new(store);
    let subject = std::env::var("APEIRETH_SUBJECT")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "me".to_string());

    // LLM 节流: 机制层门禁 (LoopConfig.min_llm_interval) + 渲染层退避 (ThrottledUtterance)
    // —— MiniMax 对连续调用限流 (suppressed, 2026-08-15 生产力实测), 双层防护
    let min_llm_interval = Duration::from_secs(
        std::env::var("APEIRETH_MIN_LLM_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60),
    );
    let mut config = LoopConfig::default();
    config.min_llm_interval = min_llm_interval;
    let awake = AwakeCompanion::new(bond, Boundaries::default()).with_config(config);

    let utter = MiniMaxUtterance::new(load_key().expect("需要 API key")).expect("build pipeline");
    let utter = ThrottledUtterance::new(utter, min_llm_interval);

    // 通道: APEIRETH_SINK=lark → 真飞书 (需凭据); 默认 console
    let sink_mode = std::env::var("APEIRETH_SINK").unwrap_or_default();
    let delivery: Box<dyn Delivery> = if sink_mode.trim() == "lark" {
        let sink = LarkSink::from_env().expect(
            "APEIRETH_SINK=lark 需要 APEIRETH_LARK_APP_ID / APEIRETH_LARK_APP_SECRET / APEIRETH_LARK_RECEIVE_ID",
        );
        Box::new(CompanionDelivery::new(utter, sink))
    } else {
        Box::new(CompanionDelivery::new(utter, ConsoleSink))
    };

    let mut daemon = CompanionDaemon {
        awake,
        delivery,
        context,
        subject,
        tick_interval: Duration::from_secs(tick_secs),
        dream: None, // 做梦调度需注入时钟的 store, 生产 daemon 下一轮接
    };

    // demo 种子 (诚实: 预填 7 天「现在这个时刻」的作息, 让演示立刻能看到主动)
    if std::env::var("APEIRETH_SEED_DEMO").is_ok() {
        let now = Utc::now();
        for d in 1..=7i64 {
            let past = now - chrono::Duration::days(d) + chrono::Duration::minutes(d % 3);
            daemon.awake.observe_interaction(past);
        }
        println!("[daemon] demo 种子: 已预填 7 天「这个时刻」的作息观察 (仅演示用)");
    }

    println!(
        "[daemon] 启动: 每 {}s 一次心跳. stdin: 任意内容=交互, r=回应上次主动, quit=退出",
        tick_secs
    );

    // stdin 任务
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx.send(line);
        }
    });

    let mut interval = tokio::time::interval(Duration::from_secs(tick_secs));
    let mut ticks: u64 = 0;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                ticks += 1;
                daemon.step().await;
                if let Some(m) = max_ticks {
                    if ticks >= m {
                        println!("[daemon] 达到 max_ticks, 退出");
                        break;
                    }
                }
            }
            Some(line) = rx.recv() => {
                let line = line.trim().to_string();
                if line == "quit" {
                    println!("[daemon] 退出");
                    break;
                } else if line == "r" {
                    let s = daemon.on_feedback(true, Utc::now());
                    println!("[daemon] 记一次「回应」→ 自评 {:.2}, 关系深度 {:.2}", s.value, daemon.awake.depth());
                } else if !line.is_empty() {
                    daemon.on_user_message(Utc::now());
                    println!("[daemon] 记一次交互 (节律 +1, 关系保持)");
                }
            }
        }
    }
}
