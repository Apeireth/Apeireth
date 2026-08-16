//! companion_serve — 伙伴端点: **任何 OpenAI 兼容前端 → Apeireth 主链路**.
//!
//! 设想 (主人 2026-08-16): 「任何前端都能接入 Apeireth」。
//! VCP 对齐 (docs/stage2/17-APEIRETH-VS-VCP-CONSUMER-PLAN.md):
//!   VCP `chatCompletionHandler.js` 主链路 + `messagePreprocessors` — 所有协议走同一条
//!   主链路, 对客户端透明可用全部 VCP 能力 (插件/RAG/角色)。我们同构:
//!   - 主链路 = OpenAI 兼容 chat completion (前端原生支持)
//!   - 预处理链 (对齐 messagePreprocessors, 异常不影响主请求):
//!       ① 记忆注入 (EMI/NEC 反幻觉, build_memory_injection)
//!       ② 今日摘要注入 (build_daily_summary — 能答「我今天干了什么」)
//!       ③ 工具桥 (ToolBridge: recall/save/simulate/forecast/audit_log/gh_accel/dx_check...)
//!
//! 跑法:
//!   $env:APEIRETH_API_KEY = (Get-Content apikey-ultra.txt -Raw).Trim()
//!   $env:APEIRETH_SEED_MEMORY = "主人高数换元法常错: 根号带平方+忘换 dx"
//!   cargo run -p apeireth-companion --example companion_serve     # 默认 :8090
//!
//! 前端对接 (LobeChat/NextChat/Open WebUI):
//!   API 地址 http://127.0.0.1:8090/v1 · Key 任意非空 · 模型 MiniMax-M3
//!   会话隔离: 请求头 `X-Apeireth-Continuity: <id>` (缺省 "me")。
//!
//! 0 假装: 只暴露日常包/白名单内的低风险工具 (FileOperator/ShellExec 等需显式扩权);
//! 工具调用上限 5 轮 (防失控); 记忆与工具结果如实返回, 不修饰。

use std::sync::Arc;
use std::time::Duration;

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::daily_summary::build_daily_summary;
use apeireth_companion::memory_injection::build_memory_injection;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_memory::{EpisodeStore, HistoryStream, SqliteMemoryStore};
use apeireth_tool_registry::ToolRegistry;
use apeireth_tool_runtime::parser::ParsedToolCall;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Local;
use serde_json::{json, Value};

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";
const MAX_TOOL_ROUNDS: usize = 5;
/// 暴露给前端的工具白名单 (低风险, 日常包/白名单已覆盖).
const ALLOWED_TOOLS: &[&str] = &[
    "recall_memory",
    "save_memory",
    "simulate",
    "forecast",
    "audit_log",
    "WebSearch",
    "WebFetch",
    "Grep",
    "Git",
];

struct AppState {
    bridge: Arc<ToolBridge>,
    store: Arc<SqliteMemoryStore>,
    pipeline: Arc<Pipeline>,
}

fn load_key() -> Result<String, String> {
    if let Ok(k) = std::env::var("APEIRETH_API_KEY") {
        if !k.trim().is_empty() {
            return Ok(k.trim().to_string());
        }
    }
    std::fs::read_to_string(r"apikey-ultra.txt")
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("读 apikey 失败: {e}"))
}

/// OpenAI 工具 schema (由已注册工具 ∩ 白名单 动态生成).
fn tools_schema(registry: &ToolRegistry) -> Vec<Value> {
    let registered: Vec<String> = registry.list();
    let brief: &[(&str, &str, Value)] = &[
        (
            "recall_memory",
            "查主人长期记忆",
            json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),
        ),
        (
            "save_memory",
            "把值得记住的写入记忆",
            json!({"type":"object","properties":{"content":{"type":"string"}},"required":["content"]}),
        ),
        (
            "simulate",
            "沙盘推演: entities 初始状态 + events 事件序列(实体.属性±增量 / =赋值), 返回各步状态",
            json!({"type":"object","properties":{"entities":{"type":"object"},"events":{"type":"array","items":{"type":"string"}}},"required":["entities","events"]}),
        ),
        (
            "forecast",
            "登记可证伪预测: statement+probability(0..1)+deadline_hours",
            json!({"type":"object","properties":{"statement":{"type":"string"},"probability":{"type":"number"},"deadline_hours":{"type":"number"}},"required":["statement","probability","deadline_hours"]}),
        ),
        (
            "audit_log",
            "查询工具调用留痕 (审计)",
            json!({"type":"object","properties":{"tool_name":{"type":"string"},"limit":{"type":"number"}}}),
        ),
        ("WebSearch", "搜索网页", json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})),
        ("WebFetch", "抓取单页内容", json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]})),
        ("Grep", "内容搜索", json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"}},"required":["pattern"]})),
        ("Git", "Git 操作", json!({"type":"object","properties":{"op":{"type":"string"}},"required":["op"]})),
    ];
    brief
        .iter()
        .filter(|(name, _, _)| registered.iter().any(|r| r == name))
        .map(|(name, desc, params)| {
            json!({"type":"function","function":{"name":name,"description":desc,"parameters":params}})
        })
        .collect()
}

/// 预处理链 ①: 记忆注入 (对齐 VCP messagePreprocessors).
fn inject_memory(store: &Arc<SqliteMemoryStore>, continuity: &str) -> String {
    let eps = store.recent_episodes(continuity, 8).unwrap_or_default();
    let entries: Vec<String> = eps.iter().map(|e| e.content.clone()).collect();
    build_memory_injection(&entries)
}

/// 预处理链 ②: 今日摘要注入.
fn inject_today(store: &Arc<SqliteMemoryStore>) -> String {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let day_start = Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|d| d.and_local_timezone(Local).unwrap().timestamp())
        .unwrap_or(0);
    // 今日 episodes
    let all = store.recent_episodes("me", 200).unwrap_or_default();
    let pairs: Vec<(&str, &str)> = all
        .iter()
        .filter(|e| e.timestamp >= day_start)
        .map(|e| (e.id.as_str(), e.content.as_str()))
        .collect();
    // 今日工具调用数 (action_stream)
    let tool_records = match store.conn() {
        Ok(conn) => {
            let stream = apeireth_memory::ActionStream::new(&conn);
            stream
                .list_recent(200, false)
                .map(|es| es.iter().filter(|e| e.created_at >= day_start).count())
                .unwrap_or(0)
        }
        Err(_) => 0,
    };
    let s = build_daily_summary(&today, &pairs, tool_records);
    s.render()
}

/// 伙伴主链路: 记忆/今日注入 → LLM+工具循环 → OpenAI 兼容响应.
async fn chat_completions(
    State(st): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<OpenAiChatRequest>,
) -> impl IntoResponse {
    let continuity = headers
        .get("x-apeireth-continuity")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("me")
        .to_string();

    let mut messages = req.messages.clone();
    // 预处理链: 记忆注入 + 今日摘要 (作为前置 system 消息; 异常不影响主请求)
    let mem = inject_memory(&st.store, &continuity);
    let today = inject_today(&st.store);
    let mut injections: Vec<String> = Vec::new();
    if !mem.is_empty() {
        injections.push(mem);
    }
    if !today.is_empty() {
        injections.push(today);
    }
    if !injections.is_empty() {
        let block = injections.join("\n");
        messages.insert(
            0,
            OpenAiChatMessage {
                role: "system".to_string(),
                content: json!(format!(
                    "以下是 Apeireth 记忆系统注入的上下文 (只作参考, 若与用户当前说法冲突以用户为准):\n{block}"
                )),
                tool_calls: None,
                tool_call_id: None,
            },
        );
    }

    let tools = tools_schema(&st.bridge.registry);
    let mut final_content = String::new();
    let mut notes: Vec<String> = Vec::new();
    let mut rounds = 0usize;
    loop {
        rounds += 1;
        let req2 = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: messages.clone(),
            temperature: Some(0.6),
            max_tokens: Some(2000),
            stream: false,
            stop: None,
            tools: Some(tools.clone()),
            tool_choice: Some(json!("auto")),
        };
        // 调 LLM (带退避重试, MiniMax 限流)
        let Some((content, tcs)) = chat_once(&st.pipeline, &req2, rounds).await else {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": {"message": "LLM 调用失败 (限流/网络), 请重试"}})),
            )
                .into_response();
        };
        if tcs.is_empty() {
            final_content = content.split("</think>").last().unwrap_or(&content).trim().to_string();
            break;
        }
        // 执行工具调用
        messages.push(OpenAiChatMessage {
            role: "assistant".to_string(),
            content: json!(content),
            tool_calls: Some(tcs.clone()),
            tool_call_id: None,
        });
        let mut tool_msgs = Vec::new();
        for tc in &tcs {
            let id = tc["id"].clone();
            let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
            let args: Value =
                serde_json::from_str(tc["function"]["arguments"].as_str().unwrap_or("{}"))
                    .unwrap_or(json!({}));
            let call = ParsedToolCall {
                tool_name: name.clone(),
                args: args.clone(),
                raw_marker: String::new(),
                archery: false,
                archery_no_reply: false,
            };
            let r = st.bridge.execute_if_allowed(&call).await;
            let body = if r.success {
                serde_json::to_string(&r.output).unwrap_or_default()
            } else {
                format!("工具失败: {:?}", r.error)
            };
            let truncated: String = body.chars().take(4000).collect();
            notes.push(format!("[{name}] 已执行"));
            tool_msgs.push(OpenAiChatMessage {
                role: "tool".to_string(),
                content: json!(truncated),
                tool_calls: None,
                tool_call_id: Some(id.as_str().unwrap_or("").to_string()),
            });
        }
        messages.extend(tool_msgs);
        if rounds >= MAX_TOOL_ROUNDS {
            final_content = "工具循环达到上限, 已停止。请让主人再发一条消息继续。".to_string();
            break;
        }
    }

    let resp = json!({
        "id": format!("chatcmpl-apeireth-{}", uuid::Uuid::new_v4()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": MODEL,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": final_content},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
        "x_apeireth": {
            "continuity": continuity,
            "tool_rounds": rounds,
            "tools_executed": notes,
            "note": "Apeireth 伙伴主链路 (VCP messagePreprocessors 对齐): 记忆+今日摘要注入, 工具桥执行"
        }
    });
    (StatusCode::OK, Json(resp)).into_response()
}

async fn chat_once(
    pipeline: &Arc<Pipeline>,
    req: &OpenAiChatRequest,
    label: usize,
) -> Option<(String, Vec<Value>)> {
    for attempt in 0..5 {
        let normalized = openai_chat_to_normalized(req);
        match dispatch(pipeline, ProtocolKind::OpenAiChat, normalized).await {
            Ok(r) => {
                let chat = openai_chat_from_normalized(&r);
                let content = chat
                    .choices
                    .first()
                    .map(|c| c.message.content.clone())
                    .unwrap_or_default();
                let tcs = chat
                    .choices
                    .first()
                    .map(|c| c.message.tool_calls.clone())
                    .unwrap_or_default()
                    .unwrap_or_default();
                return Some((content, tcs));
            }
            Err(e) => {
                eprintln!("  [管线] 轮{label} 第{}次失败: {e}, 8s 后重试", attempt + 1);
                tokio::time::sleep(Duration::from_secs(8)).await;
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = load_key()?;
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8090);

    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    // 可选种子记忆 (演示/验证用; 不设则从零开始积累)
    if let Ok(seed) = std::env::var("APEIRETH_SEED_MEMORY") {
        for (i, c) in seed.split(';').filter(|s| !s.trim().is_empty()).enumerate() {
            let _ = store.put_episode(&apeireth_memory::CoreEpisode {
                id: format!("seed-{i}"),
                timestamp: chrono::Utc::now().timestamp(),
                role: "assistant".into(),
                content: c.trim().to_string(),
                session_id: "me".into(),
            });
        }
        println!("[seed] 已写入种子记忆: {}", seed.replace(';', " | "));
    }

    let bridge = Arc::new(ToolBridge::new(Arc::clone(&store)));
    let pipeline = Arc::new(build_pipeline(BASE_URL.to_string(), Some(key))?);
    let state = Arc::new(AppState {
        bridge,
        store,
        pipeline,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    println!("✅ companion_serve — 伙伴端点 (任何 OpenAI 兼容前端可接入)");
    println!("   http://127.0.0.1:{port}/v1  (模型 MiniMax-M3, Key 任意非空)");
    println!("   会话隔离: X-Apeireth-Continuity 请求头 (缺省 me)");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "apeireth-companion-serve",
        "version": env!("CARGO_PKG_VERSION"),
        "features": ["memory_injection", "today_summary", "tool_bridge", "openai_compat"],
    }))
}

async fn list_models() -> impl IntoResponse {
    Json(json!({
        "object": "list",
        "data": [{"id": MODEL, "object": "model", "created": 0, "owned_by": "minimax"}]
    }))
}
