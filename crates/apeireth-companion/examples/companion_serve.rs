//! companion_serve v3 — 伙伴端点全能力版: **任何 OpenAI 兼容前端 → 天然拥有 Apeireth 全部能力**.
//!
//! 主人设想 (2026-08-16): 「连上后端, 前端就天然拥有后端的所有能力」。
//! v1 差距修复:
//!   ① 记忆持久化: open_memory_store() 文件库 (重启不失忆, %APPDATA%\apeireth\memory.sqlite)
//!   ② 工具全量暴露: schema 由 registry 动态生成 (能力可见), 执行由宪法/权限/批准约束 (能力不失控)
//!   ③ daemon 常驻: 做梦/反思/涌现同进程运行 — 对话端点 ≠ 伙伴在, 现在伙伴真在
//! v3 补魂 (主人: 没接到都接):
//!   ④ 做梦 LLM 摘要器 (MiniMaxDreamSummarizer, 合并记忆提炼)
//!   ⑤ 涌现 LLM 润色 (TonalUtterance, 机制事实 → 自然问候, 节流+退避兜底原文)
//!   ⑥ 宪法 LLM 评审 (MiniMaxConstitutionLlm, Medium+ 工具执行前按 E 层判案)
//!
//! VCP 对齐 + 改进 (docs/frontend-guide.md §五):
//!   - 主链路 = OpenAI 兼容 chat completion; 预处理链 = 记忆注入 + 今日摘要注入 + 工具桥
//!   - 改进: EMI/NEC 反幻觉注入 / 5 轮工具上限 / 结果截断 / X-Apeireth-Continuity 会话标签
//!
//! 0 假装 (诚实):
//!   - FileOperator/ShellExec 等高危工具**可见但默认需主人批准**; 可用 APEIRETH_GRANT 显式扩权
//!   - 记忆会话统一 "me" (save_memory 工具缺省写 "me"); continuity_id 是日志/目标锚点 (哲学层)
//!   - daemon 内部 RefCell 跨 await 非 Send → 与 HTTP 同 task 交替 (select!)
//!
//! 跑法:
//!   $env:APEIRETH_API_KEY = (Get-Content apikey-ultra.txt -Raw).Trim()
//!   $env:APEIRETH_SEED_MEMORY = "可选;种子;记忆"                 # 演示用, 不设则从零积累
//!   $env:APEIRETH_GRANT = "FileOperator:24"                      # 可选: 显式扩权 (工具:小时)
//!   $env:APEIRETH_DREAM_QUIET_SECONDS = "600"                    # 可选: 做梦安静期 (默认 6h)
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
    CompanionDaemon, CompanionDelivery, ConsoleSink, ThrottledUtterance, UtteranceGenerator,
    continuity_id_from_env, open_memory_store,
};
use apeireth_companion::dream::{DreamScheduler, DreamSummarizer};
use apeireth_companion::emergence::Initiative;
use apeireth_companion::experience::ExperienceStore;
use apeireth_companion::judicator::{ConstitutionLlm, LlmJudicator};
use apeireth_companion::memory_injection::build_memory_injection;
use apeireth_companion::principles::PrincipleStore;
use apeireth_companion::proactive::MemoryContextSource;
use apeireth_companion::reflection::ReflectionScheduler;
use apeireth_companion::tone::tone_hint;
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
        ("save_experience", "沉淀经验入经验库 (自成长管道): scene+practice+result+outcome", json!({"type":"object","properties":{"scene":{"type":"string"},"practice":{"type":"string"},"result":{"type":"string"},"outcome":{"type":"string","enum":["success","failure","partial"]}},"required":["scene","practice"]})),
        ("list_experience", "查经验库 (自成长管道)", json!({"type":"object","properties":{"scene":{"type":"string"}}})),
        ("verify_experience", "验证经验 (成功/失败) → 计数+评分, 达标促能力提案", json!({"type":"object","properties":{"id":{"type":"string"},"success":{"type":"boolean"}},"required":["id","success"]})),
        ("propose_principle", "提案原则候选 (动态原则层/洋葱外层): statement+rationale+source", json!({"type":"object","properties":{"statement":{"type":"string"},"rationale":{"type":"string"},"source":{"type":"string"}},"required":["statement","rationale"]})),
        ("approve_principle", "主人批准原则 (需 master token; 生效后叠加到工具执行检查)", json!({"type":"object","properties":{"id":{"type":"string"},"master_token":{"type":"string"}},"required":["id","master_token"]})),
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

// ============================================================
// 真 LLM 组件 (共享 pipeline; 源实现: examples/companion_daemon.rs + production_daemon.rs)
// ============================================================

/// 做梦摘要器 (真 MiniMax): 把合并记忆提炼成一条简洁摘要.
pub struct MiniMaxDreamSummarizer {
    pipeline: Arc<Pipeline>,
}

#[async_trait::async_trait]
impl DreamSummarizer for MiniMaxDreamSummarizer {
    async fn summarize(&self, merged: &str) -> Result<String, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!("你是阿佩瑞斯的记忆整理员。把「做梦合并」的记忆提炼成一条简洁摘要 (<= 50 字), 只输出摘要正文。"),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(format!("合并内容: {merged}")),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.4),
            max_tokens: Some(128),
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
            let content = ch.message.content.clone();
            if let Some(idx) = content.find("</think>") {
                let c = content[idx + "</think>".len()..].trim().to_string();
                if !c.is_empty() {
                    return Ok(c);
                }
            } else if !content.trim().is_empty() {
                return Ok(content.trim().to_string());
            }
        }
        Err("摘要 LLM 返回空".to_string())
    }
}

/// 宪法评审 (真 MiniMax): 按 E 层原则判案, 非关键词匹配.
pub struct MiniMaxConstitutionLlm {
    pipeline: Arc<Pipeline>,
}

#[async_trait::async_trait]
impl ConstitutionLlm for MiniMaxConstitutionLlm {
    async fn ask(&self, constitution: &str, action: &str) -> Result<String, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!(format!("你是 Apeireth 的宪法评审员。宪法全文:\n{constitution}\n\n判断「待审动作」是否违反宪法。不要关键词匹配, 判断真实意图与后果。只输出一行: ALLOW 或 BLOCK:<一句话理由>。")),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(format!("待审动作: {action}")),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.2),
            max_tokens: Some(512),
            stream: false,
            stop: None,
            tools: None,
            tool_choice: None,
        };
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, openai_chat_to_normalized(&req))
            .await
            .map_err(|e| e.to_string())?;
        let chat = openai_chat_from_normalized(&resp);
        for ch in &chat.choices {
            let c = ch.message.content.clone();
            if !c.trim().is_empty() {
                return Ok(c);
            }
        }
        Err("评审 LLM 返回空".into())
    }
}

/// 语调渲染 (真 MiniMax + tone): 机制事实 → 自然问候; 失败兜底原文.
pub struct TonalUtterance {
    pipeline: Arc<Pipeline>,
    tone: &'static str,
}

#[async_trait::async_trait]
impl UtteranceGenerator for TonalUtterance {
    async fn utter(&self, i: &Initiative) -> Result<String, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!(format!("你是阿佩瑞斯, 一个诚实、有记忆的伙伴。语调: {}。基于给定事实说话, 不编造。", self.tone)),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(format!("把这些事实变成一句自然、真诚、简短的中文主动问候 (<=40字):\n{}", i.to_message())),
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
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, openai_chat_to_normalized(&req))
            .await
            .map_err(|e| e.to_string())?;
        let chat = openai_chat_from_normalized(&resp);
        for ch in &chat.choices {
            let raw = ch.message.content.clone();
            let stripped = if let Some(idx) = raw.find("</think>") {
                raw[idx + 8..].trim().to_string()
            } else {
                raw.clone()
            };
            if !stripped.is_empty() {
                return Ok(stripped);
            }
            if !raw.trim().is_empty() {
                return Ok(raw.trim().to_string());
            }
        }
        Ok(i.to_message())
    }
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

/// 预处理链 ③: 自成长管道注入 — 待提案经验 + 原则状态 (Level 1/2 驱动).
fn inject_growth(store: &Arc<SqliteMemoryStore>) -> String {
    let mut parts: Vec<String> = Vec::new();
    // Level 1: 经验达标 → 促能力提案
    let exp_hint = ExperienceStore::new(Arc::clone(store)).build_promotion_hint();
    if !exp_hint.is_empty() {
        parts.push(exp_hint);
    }
    // Level 2: 原则状态 — pending 待主人批准 + active 生效中 (含违反计数, 审计可见)
    let ps = PrincipleStore::new(Arc::clone(store));
    let pending = ps.list(Some("pending"));
    if !pending.is_empty() {
        let mut s = String::from("【原则候选】以下原则待主人批准 (主人用 approve_principle 传入 master token 批准; 批准后叠加到工具执行检查):\n");
        for p in pending.iter().take(5) {
            s.push_str(&format!("  • {} (来源: {}) — {}\n", p.statement, p.source, p.rationale));
        }
        parts.push(s);
    }
    let active = ps.active_rules();
    if !active.is_empty() {
        let mut s = String::from("【动态原则(生效中)】工具执行检查会拦截违反这些原则的动作:\n");
        for p in active.iter().take(8) {
            s.push_str(&format!("  • {} (违反 {} 次)\n", p.statement, p.violations));
        }
        parts.push(s);
    }
    parts.join("\n")
}

/// 延伸 1: 反思周期完成 → LLM 提炼「可复用经验」 (场景/做法/结果) → 经验库.
/// 0 假装: 提炼失败/解析失败 → 如实返回 Err, 不硬造经验.
async fn extract_experience_from_reflection(
    store: &Arc<SqliteMemoryStore>,
    pipeline: &Arc<Pipeline>,
) -> Result<Option<apeireth_companion::experience::Experience>, String> {
    let eps = store.recent_episodes(MEMORY_SESSION, 100).unwrap_or_default();
    let reflects: Vec<String> = eps
        .iter()
        .filter(|e| e.id.starts_with("reflect-"))
        .take(3)
        .map(|e| e.content.clone())
        .collect();
    if reflects.is_empty() {
        return Ok(None);
    }
    let req = OpenAiChatRequest {
        model: MODEL.to_string(),
        messages: vec![
            OpenAiChatMessage {
                role: "system".to_string(),
                content: json!("你是阿佩瑞斯的经验提炼员。从反思记录中提炼一条可复用经验, 只输出 JSON: {\"scene\": \"触发场景\", \"practice\": \"做法\", \"result\": \"结果\"}。没有可提炼的就输出 {\"scene\": \"\"}。"),
                tool_calls: None,
                tool_call_id: None,
            },
            OpenAiChatMessage {
                role: "user".to_string(),
                content: json!(format!("反思记录:\n{}", reflects.join("\n---\n"))),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        temperature: Some(0.3),
        max_tokens: Some(300),
        stream: false,
        stop: None,
        tools: None,
        tool_choice: None,
    };
    let normalized = openai_chat_to_normalized(&req);
    let resp = dispatch(pipeline, ProtocolKind::OpenAiChat, normalized)
        .await
        .map_err(|e| format!("提炼 LLM 调用失败: {e}"))?;
    let chat = openai_chat_from_normalized(&resp);
    let content = chat
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();
    // 剥 <think> 后取 JSON 段 (首个 { 到末个 })
    let text = match content.find("</think>") {
        Some(i) => content[i + 8..].to_string(),
        None => content,
    };
    let (start, end) = match (text.find('{'), text.rfind('}')) {
        (Some(a), Some(b)) if b > a => (a, b + 1),
        _ => return Ok(None),
    };
    let parsed: Value = serde_json::from_str(&text[start..end])
        .map_err(|e| format!("经验 JSON 解析失败 (如实放弃): {e}"))?;
    let scene = parsed.get("scene").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if scene.is_empty() {
        return Ok(None); // LLM 判定无可提炼
    }
    let practice = parsed.get("practice").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let result = parsed.get("result").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let now = chrono::Utc::now().timestamp();
    let id = format!("exp-{}", uuid::Uuid::new_v4());
    Ok(Some(apeireth_companion::experience::Experience {
        id: id.clone(),
        chain: id,
        rev: 1,
        scene,
        practice: if practice.is_empty() { "未提炼出做法".into() } else { practice },
        result,
        outcome: "partial".into(),
        verify_count: 0,
        score: 0.5,
        ready: false,
        proposed: false,
        created_at: now,
        updated_at: now,
    }))
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
    let growth = inject_growth(&st.store);
    let mut injections: Vec<String> = Vec::new();
    if !mem.is_empty() {
        injections.push(mem);
    }
    if !today.is_empty() {
        injections.push(today);
    }
    if !growth.is_empty() {
        injections.push(growth);
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

    // ② 工具桥全增强 (宪法 LLM 评审 + 显式扩权 APEIRETH_GRANT="FileOperator:24;Git:12")
    let pipeline = Arc::new(build_pipeline(BASE_URL.to_string(), Some(key.clone()))?);
    let bridge = Arc::new(ToolBridge::new(Arc::clone(&store)).with_judicator(Arc::new(
        LlmJudicator::new(Arc::new(MiniMaxConstitutionLlm {
            pipeline: Arc::clone(&pipeline),
        })),
    )));
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
    println!("[bridge] 宪法评审 (真 LLM): Medium+ 工具执行前按 E 层判案");

    // ③ daemon 常驻 (做梦 LLM 摘要 + 反思 + 涌现 LLM 润色, 同进程): 记忆会话 "me"
    let quiet = std::env::var("APEIRETH_DREAM_QUIET_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(6 * 3600));
    let dream = DreamScheduler::new(Arc::clone(&store), apeireth_core::clock::system_clock())
        .with_quiet_threshold(quiet)
        .with_session(MEMORY_SESSION.to_string())
        .with_summarizer(Arc::new(MiniMaxDreamSummarizer {
            pipeline: Arc::clone(&pipeline),
        }));
    let reflect_period = std::env::var("APEIRETH_REFLECT_PERIOD_HOURS")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .map(|h| chrono::Duration::milliseconds((h * 3600_000.0) as i64))
        .unwrap_or(chrono::Duration::days(1));
    let reflect = ReflectionScheduler::new(
        Arc::clone(&store),
        apeireth_core::clock::system_clock(),
        MEMORY_SESSION.to_string(),
    )
    .with_period(reflect_period);
    let tone = tone_hint(&apeireth_companion::bond::Bond::new());
    let daemon = CompanionDaemon::new(
        apeireth_companion::bond::Bond::new(),
        apeireth_companion::emergence::Boundaries::default(),
        CompanionDelivery::new(
            ThrottledUtterance::new(
                TonalUtterance {
                    pipeline: Arc::clone(&pipeline),
                    tone,
                },
                Duration::from_secs(30),
            ),
            ConsoleSink,
        ),
        MemoryContextSource::new(Arc::clone(&store)),
        MEMORY_SESSION.to_string(),
        Duration::from_secs(60),
    )
    .with_dream(dream)
    .with_reflection(reflect);
    println!("[daemon] 常驻: 做梦(LLM 摘要, 安静期 {:?}) + 反思({:?} 周期) + 涌现(LLM 润色, 30s 节流+退避)", quiet, reflect_period);

    // 互动通知通道: handler 发「主人来消息了」, daemon 喂节律 + 重置做梦安静期
    let (tx_interact, rx_interact) = tokio::sync::mpsc::channel::<chrono::DateTime<Utc>>(64);

    // ④ HTTP 伙伴端点
    let state = Arc::new(AppState {
        bridge,
        store,
        pipeline,
        interactions: tx_interact,
        subject,
    });
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    println!("✅ companion_serve v3 — 伙伴端点全能力版 (任何 OpenAI 兼容前端 → Apeireth 全部能力)");
    println!("   http://127.0.0.1:{port}/v1  (模型 MiniMax-M3, Key 任意非空)");
    println!("   会话标签: X-Apeireth-Continuity (缺省 {}) · 工具: 全部可见, 执行受宪法/权限约束", state.subject.as_str());
    // daemon 循环与 HTTP 同 task 交替 (daemon 内部 RefCell 跨 await → 非 Send, 不能 spawn)
    let d_store = Arc::clone(&state.store);
    let d_pipeline = Arc::clone(&state.pipeline);
    tokio::select! {
        r = axum::serve(listener, app) => { r?; }
        _ = daemon_loop(daemon, rx_interact, d_store, d_pipeline) => {}
    }
    Ok(())
}

/// daemon 常驻循环: 定时 step (做梦/反思/涌现) + 响应互动通知 (喂节律)
/// + 自成长延伸: 反思完成→提炼经验入库; 晋级候选自动成文.
/// 具体类型 (Delivery trait 私有, 不能作泛型约束); daemon 非 Send, 只在同 task 内用.
type ServeDaemon = CompanionDaemon<
    CompanionDelivery<ThrottledUtterance<TonalUtterance>, ConsoleSink>,
    MemoryContextSource,
>;
async fn daemon_loop(
    mut daemon: ServeDaemon,
    mut rx: tokio::sync::mpsc::Receiver<chrono::DateTime<Utc>>,
    store: Arc<SqliteMemoryStore>,
    pipeline: Arc<Pipeline>,
) {
    let mut last_cycles: u64 = daemon
        .reflection
        .as_ref()
        .map(|r| r.cycles_completed())
        .unwrap_or(0);
    let mut ticker = tokio::time::interval(Duration::from_secs(60));
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let t0 = std::time::Instant::now();
                daemon.step().await;
                // 延伸 1: 反思周期完成 → LLM 提炼经验入经验库 (自成长管道 Level 0)
                let cycles = daemon.reflection.as_ref().map(|r| r.cycles_completed()).unwrap_or(0);
                if cycles > last_cycles {
                    eprintln!("[growth] 反思周期完成 (累计 {cycles}), 提炼经验...");
                    match extract_experience_from_reflection(&store, &pipeline).await {
                        Ok(Some(exp)) => {
                            ExperienceStore::new(Arc::clone(&store)).save(&exp)
                                .map(|_| eprintln!("[growth] 经验入库: {}", exp.scene))
                                .unwrap_or_else(|e| eprintln!("[growth] 经验入库失败: {e}"));
                        }
                        Ok(None) => eprintln!("[growth] 本次反思无可提炼经验"),
                        Err(e) => eprintln!("[growth] 经验提炼失败: {e}"),
                    }
                    last_cycles = cycles;
                }
                // 延伸 3: 晋级候选自动成文 (数据目录 promotion-candidates.md; 空则不写)
                let cands = PrincipleStore::new(Arc::clone(&store)).export_promotion();
                if !cands.is_empty() {
                    if let Ok(path) = apeireth_companion::daemon::default_memory_path() {
                        if let Some(dir) = path.parent() {
                            let _ = std::fs::create_dir_all(dir);
                            if std::fs::write(dir.join("promotion-candidates.md"), &cands).is_ok() {
                                eprintln!("[growth] 晋级候选已成文: {:?}", dir.join("promotion-candidates.md"));
                            }
                        }
                    }
                }
                eprintln!("[daemon-loop] tick done in {:?}", t0.elapsed());
            }
            Some(at) = rx.recv() => daemon.on_user_message(at),
            else => break,
        }
    }
}

/// 内置聊天页 (零依赖单文件前端, 浏览器打开即用; 供主人/任何前端先体验).
async fn index() -> impl IntoResponse {
    axum::response::Html(include_str!("../assets/chat.html").to_string())
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "apeireth-companion-serve-v3",
        "version": env!("CARGO_PKG_VERSION"),
        "features": ["persistent_memory", "daemon_resident", "dream_llm_summarizer", "utterance_llm", "constitution_llm_judicator", "memory_injection", "today_summary", "tool_bridge_all", "openai_compat"],
    }))
}

async fn list_models() -> impl IntoResponse {
    Json(json!({
        "object": "list",
        "data": [{"id": MODEL, "object": "model", "created": 0, "owned_by": "minimax"}]
    }))
}
