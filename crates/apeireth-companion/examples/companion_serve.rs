//! companion_serve v2 — 伙伴端点全能力版: **任何 OpenAI 兼容前端 → 天然拥有 Apeireth 全部能力**.
//!
//! 主人设想 (2026-08-16): 「连上后端, 前端就天然拥有后端的所有能力」。
//! v1 差距修复:
//!   ① 记忆持久化: open_memory_store() 文件库 (重启不失忆, %APPDATA%\apeireth\memory.sqlite)
//!   ② 工具全量暴露: schema 由 registry 动态生成 (能力可见), 执行由宪法/权限/批准约束 (能力不失控)
//!   ③ daemon 常驻: 做梦/反思/涌现同进程运行 — 对话端点 ≠ 伙伴在, 现在伙伴真在
//!
//! VCP 对齐 + 改进 (docs/frontend-guide.md §五):
//!   - 主链路 = OpenAI 兼容 chat completion; 预处理链 = 记忆注入 + 今日摘要注入 + 工具桥
//!   - 改进: EMI/NEC 反幻觉注入 / 5 轮工具上限 / 结果截断 / X-Apeireth-Continuity 会话标签
//!
//! 0 假装 (诚实):
//!   - 做梦未接 LLM 摘要器 (合并保持拼接, 诚实降级); 涌现文本为 PlainUtterance 机制原文 (非 LLM 润色)
//!   - FileOperator/ShellExec 等高危工具**可见但默认需主人批准**; 可用 APEIRETH_GRANT 显式扩权
//!   - 记忆会话统一 "me" (save_memory 工具缺省写 "me"); continuity_id 是日志/目标锚点 (哲学层)
//!
//! 跑法:
//!   $env:APEIRETH_API_KEY = (Get-Content apikey-ultra.txt -Raw).Trim()
//!   $env:APEIRETH_SEED_MEMORY = "可选;种子;记忆"                 # 演示用, 不设则从零积累
//!   $env:APEIRETH_GRANT = "FileOperator:24"                      # 可选: 显式扩权 (工具:小时)
//!   cargo run -p apeireth-companion --example companion_serve    # :8090, daemon 同进程常驻

use std::sync::Arc;
use std::time::Duration;

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::daily_summary::build_daily_summary;
use apeireth_companion::daemon::{
    CompanionDaemon, CompanionDelivery, ConsoleSink, PlainUtterance, ThrottledUtterance,
    continuity_id_from_env, open_memory_store,
};
use apeireth_companion::dream::DreamScheduler;
use apeireth_companion::memory_injection::build_memory_injection;
use apeireth_companion::proactive::MemoryContextSource;
use apeireth_companion::reflection::ReflectionScheduler;
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
use chrono::{Local, Utc};
use serde_json::{json, Value};

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";
const MAX_TOOL_ROUNDS: usize = 5;
/// 记忆会话 (save_memory 工具缺省写 "me" — 全库一致).
const MEMORY_SESSION: &str = "me";

/// 已知工具的手写 schema (description/parameters); 未列出的工具给通用 schema (能力仍可见).
fn known_schemas() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        (
            "recall_memory",
            "查主人长期记忆",
            json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),
        ),
        (
            "save_memory",
            "把值得记住的写入记忆 (单条 <= 500 字)",
            json!({"type":"object","properties":{"content":{"type":"string"}},"required":["content"]}),
        ),
        (
            "simulate",
            "沙盘推演: entities 初始状态 + events 事件序列(实体.属性±增量 增减 / =数值 赋值), 返回各步状态",
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
        ("Crawl", "爬取多页+链接提取", json!({"type":"object","properties":{"url":{"type":"string"},"max_pages":{"type":"number"}},"required":["url"]})),
        ("Grep", "内容搜索", json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"}},"required":["pattern"]})),
        ("Git", "Git 操作", json!({"type":"object","properties":{"op":{"type":"string"}},"required":["op"]})),
        ("FileOperator", "文件操作 (read/write/list; 需授权包覆盖路径)", json!({"type":"object","properties":{"op":{"type":"string","enum":["read","write","list"]},"path":{"type":"string"},"content":{"type":"string"}},"required":["op","path"]})),
        ("gh_accel", "GitHub 加速: 节点池实测选最快", json!({"type":"object","properties":{"limit":{"type":"number"},"github_url":{"type":"string"}}})),
        ("dx_check", "换元法 dx 检查 (忘换 dx/混用/缺微分/根号模式)", json!({"type":"object","properties":{"problem":{"type":"string"},"substitution":{"type":"string"},"after":{"type":"string"}},"required":["problem"]})),
        ("ShellExec", "执行 shell 命令 (高危, 需主人批准)", json!({"type":"object","properties":{"command":{"type":"string"}},"required":["command"]})),
    ]
}

/// 全量工具 schema (能力可见): 已注册工具 ∩ 手写 schema, 未覆盖的给通用 schema.
fn tools_schema(registry: &ToolRegistry) -> Vec<Value> {
    let registered: Vec<String> = registry.list();
    let known = known_schemas();
    let mut out: Vec<Value> = known
        .iter()
        .filter(|(name, _, _)| registered.iter().any(|r| r == name))
        .map(|(name, desc, params)| {
            json!({"type":"function","function":{"name":name,"description":desc,"parameters":params}})
        })
        .collect();
    // 已注册但未手写 schema 的工具: 通用 schema (能力可见, 参数由 AI 按名推断)
    for name in registered.iter() {
        if known.iter().any(|(k, _, _)| k == name) {
            continue;
        }
        out.push(json!({
            "type":"function",
            "function":{"name":name,"description":format!("工具 {name} (参数按工具约定传入)"),"parameters":{"type":"object","properties":{}}}
        }));
    }
    out
}

struct AppState {
    bridge: Arc<ToolBridge>,
    store: Arc<SqliteMemoryStore>,
    pipeline: Arc<Pipeline>,
    /// 互动通知通道 (daemon task 持有 daemon, 此处只发「主人来消息了」时刻).
    interactions: tokio::sync::mpsc::Sender<chrono::DateTime<Utc>>,
    subject: String,
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

/// 预处理链 ①: 记忆注入 (EMI/NEC 反幻觉; 查询记忆会话 "me").
fn inject_memory(store: &Arc<SqliteMemoryStore>) -> String {
    let eps = store.recent_episodes(MEMORY_SESSION, 8).unwrap_or_default();
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
    let all = store.recent_episodes(MEMORY_SESSION, 200).unwrap_or_default();
    let pairs: Vec<(&str, &str)> = all
        .iter()
        .filter(|e| e.timestamp >= day_start)
        .map(|e| (e.id.as_str(), e.content.as_str()))
        .collect();
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
    build_daily_summary(&today, &pairs, tool_records).render()
}

/// 伙伴主链路: 喂节律 → 记忆/今日注入 → LLM+工具循环 → OpenAI 兼容响应.
async fn chat_completions(
    State(st): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<OpenAiChatRequest>,
) -> impl IntoResponse {
    let continuity = headers
        .get("x-apeireth-continuity")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&st.subject)
        .to_string();

    // 喂节律: 对话 = 互动 (节律直方图学习作息 + 重置做梦安静期) — 「他在」的感知
    let _ = st.interactions.send(Utc::now()).await;

    let mut messages = req.messages.clone();
    let mem = inject_memory(&st.store);
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
    let mut final_content: String;
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
            "features": ["memory_injection", "today_summary", "tool_bridge", "daemon_resident"],
            "note": "Apeireth 伙伴主链路: 记忆+今日摘要注入, 工具桥执行, daemon 同进程常驻"
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

    // ① 持久记忆库 (文件, 重启不失忆) + 哲学锚点
    let store = open_memory_store().expect("真记忆库");
    let subject = continuity_id_from_env("companion-main");
    println!("[mem] 持久记忆库: 已打开 (重启不失忆) · subject: {subject}");

    // 可选种子记忆 (演示/验证)
    if let Ok(seed) = std::env::var("APEIRETH_SEED_MEMORY") {
        for (i, c) in seed.split(';').filter(|s| !s.trim().is_empty()).enumerate() {
            let _ = store.put_episode(&apeireth_memory::CoreEpisode {
                id: format!("seed-{i}"),
                timestamp: chrono::Utc::now().timestamp(),
                role: "assistant".into(),
                content: c.trim().to_string(),
                session_id: MEMORY_SESSION.to_string(),
            });
        }
        println!("[seed] 已写入种子记忆: {}", seed.replace(';', " | "));
    }

    // ② 工具桥全增强 + 显式扩权 (APEIRETH_GRANT="FileOperator:24;Git:12")
    let bridge = Arc::new(ToolBridge::new(Arc::clone(&store)));
    if let Ok(grants) = std::env::var("APEIRETH_GRANT") {
        for g in grants.split(';').filter(|s| !s.trim().is_empty()) {
            let (tool, hours) = match g.split_once(':') {
                Some((t, h)) => (t.trim(), h.trim().parse().unwrap_or(24)),
                None => (g.trim(), 24),
            };
            bridge.packs.grant(apeireth_companion::packs::PermissionPack::timed(
                "serve 显式扩权",
                vec![tool.to_string()],
                hours,
                None,
            ));
            println!("[grant] {tool}: {hours}h");
        }
    }

    // ③ daemon 常驻 (做梦/反思/涌现, 同进程): 记忆会话 "me" 与 save_memory 缺省一致
    let dream = DreamScheduler::new(Arc::clone(&store), apeireth_core::clock::system_clock())
        .with_quiet_threshold(Duration::from_secs(6 * 3600))
        .with_session(MEMORY_SESSION.to_string());
    let reflect = ReflectionScheduler::new(
        Arc::clone(&store),
        apeireth_core::clock::system_clock(),
        MEMORY_SESSION.to_string(),
    );
    let daemon = CompanionDaemon::new(
        apeireth_companion::bond::Bond::new(),
        apeireth_companion::emergence::Boundaries::default(),
        CompanionDelivery::new(
            ThrottledUtterance::new(PlainUtterance, Duration::from_secs(30)),
            ConsoleSink,
        ),
        MemoryContextSource::new(Arc::clone(&store)),
        MEMORY_SESSION.to_string(),
        Duration::from_secs(60),
    )
    .with_dream(dream)
    .with_reflection(reflect);
    println!("[daemon] 常驻: 做梦(6h 安静) + 反思(24h) + 涌现, tick 60s");
    println!("         (0 假装: 做梦未接 LLM 摘要器=拼接降级; 涌现文本=机制原文非 LLM 润色)");

    // 互动通知通道: handler 发「主人来消息了」, daemon 喂节律 + 重置做梦安静期
    let (tx_interact, rx_interact) = tokio::sync::mpsc::channel::<chrono::DateTime<Utc>>(64);

    // ④ HTTP 伙伴端点
    let pipeline = Arc::new(build_pipeline(BASE_URL.to_string(), Some(key))?);
    let state = Arc::new(AppState {
        bridge,
        store,
        pipeline,
        interactions: tx_interact,
        subject,
    });
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    println!("✅ companion_serve v2 — 伙伴端点全能力版 (任何 OpenAI 兼容前端 → Apeireth 全部能力)");
    println!("   http://127.0.0.1:{port}/v1  (模型 MiniMax-M3, Key 任意非空)");
    println!("   会话标签: X-Apeireth-Continuity (缺省 {}) · 工具: 全部可见, 执行受宪法/权限约束", state.subject.as_str());
    // daemon 循环与 HTTP 同 task 交替 (daemon 内部 RefCell 跨 await → 非 Send, 不能 spawn)
    tokio::select! {
        r = axum::serve(listener, app) => { r?; }
        _ = daemon_loop(daemon, rx_interact) => {}
    }
    Ok(())
}

/// daemon 常驻循环: 定时 step (做梦/反思/涌现) + 响应互动通知 (喂节律).
/// 具体类型 (Delivery trait 私有, 不能作泛型约束); daemon 非 Send, 只在同 task 内用.
type ServeDaemon = CompanionDaemon<
    CompanionDelivery<ThrottledUtterance<PlainUtterance>, ConsoleSink>,
    MemoryContextSource,
>;
async fn daemon_loop(
    mut daemon: ServeDaemon,
    mut rx: tokio::sync::mpsc::Receiver<chrono::DateTime<Utc>>,
) {
    let mut ticker = tokio::time::interval(Duration::from_secs(60));
    loop {
        tokio::select! {
            _ = ticker.tick() => daemon.step().await,
            Some(at) = rx.recv() => daemon.on_user_message(at),
            else => break,
        }
    }
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "apeireth-companion-serve-v2",
        "version": env!("CARGO_PKG_VERSION"),
        "features": ["persistent_memory", "daemon_resident", "memory_injection", "today_summary", "tool_bridge_all", "openai_compat"],
    }))
}

async fn list_models() -> impl IntoResponse {
    Json(json!({
        "object": "list",
        "data": [{"id": MODEL, "object": "model", "created": 0, "owned_by": "minimax"}]
    }))
}
