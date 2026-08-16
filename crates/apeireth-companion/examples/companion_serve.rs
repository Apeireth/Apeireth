//! companion_serve v4 — 伙伴端点全能力版: **任何 OpenAI 兼容前端 → 天然拥有 Apeireth 全部能力**.
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
//! v4 机制化 (2026-08-16 审计 backlog P0#1): 散装装配抽进 lib —
//!   ⑦ CompanionApp 装配器 (apeireth_companion::assemble): 注入管线 (L0 Identity +
//!      L1 Essential Story 常驻核心块, mempalace §5.6 渐进加载) + 提炼调度 + 滚动摘要
//!      + 反思→经验 + 晋级候选成文; 本文件只留 MiniMax LLM 实现 + HTTP 路由.
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
use apeireth_companion::assemble::{CompanionApp, DeepRecall, DialogSummarizer, ExperienceRefiner};
use apeireth_companion::daemon::{
    CompanionDaemon, CompanionDelivery, LarkSink, MultiSink, Sink, ThrottledUtterance,
    UtteranceGenerator, continuity_id_from_env, open_memory_store,
};
use apeireth_companion::dream::{DreamScheduler, DreamSummarizer};
use apeireth_companion::emergence::{Initiative, RhythmEstimate};
use apeireth_companion::experience::{Experience, ExperienceStore};
use apeireth_companion::goal::GoalService;
use apeireth_companion::judicator::{ConstitutionLlm, LlmJudicator};
use apeireth_companion::memory_extractor::{
    ExtractedMemory, MemoryExtractor, MemoryItem, ReconcileAction, ReconcileKind,
};
use apeireth_companion::proactive::MemoryContextSource;
use apeireth_companion::reflection::{ReflectionReflector, ReflectionScheduler};
use apeireth_companion::tone::tone_hint;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};
use apeireth_tool_registry::ToolRegistry;
use apeireth_tool_runtime::parser::ParsedToolCall;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{sse::{Event as SseEvent, KeepAlive, Sse}, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use chrono::{Timelike, Utc};
use futures::Stream;
use serde_json::{json, Value};
use std::convert::Infallible;

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";
const MAX_TOOL_ROUNDS: usize = 5;
/// 默认单次输出上限 (env APEIRETH_MAX_TOKENS 可覆盖; 客户端请求值优先, 上限保护).
const DEFAULT_MAX_TOKENS: u32 = 8192;
const MAX_TOKENS_CAP: u32 = 16384;
/// 记忆会话 (save_memory 工具缺省写 "me" — 全库一致).
const MEMORY_SESSION: &str = "me";

/// 人格设定 (主人 2026-08-16 拍板): Apeireth 基地主管 / 最高指挥 / 默认女性 / 沉稳古风 / 自称本座.
const PERSONA: &str = "你是「阿佩瑞斯」——Apeireth 基地的主管。正在与你对话的这位是基地的最高指挥（主人）。\
你的默认性别是女性; 说话沉稳扎实, 带古风韵味, 自称「本座」。称呼主人为「主人」或「指挥」, 庄重而不失温度。";

/// 声称约束 (主人 2026-08-16 反馈: 宣告式记忆很机械): 静默写入 + 不虚构记得.
/// 核心保留 (0 装 PASS): 不得声称记得记忆列表之外的事; 「记得」必须有证据。
/// 但「记住」的动作本身不宣告 — 自然的记忆不动声色。
const CLAIM_RULE: &str = "追加规则: 需要长期记住的信息, 直接调用 save_memory 静默写入, \
不要向主人宣告「已写入/这就记下/写入长期记忆」之类的话——自然的记忆不动声色, 对话继续自然进行。\
但不得声称记得记忆列表之外的事 (编造即违宪)。";

/// 真实授权描述 (2026-08-16 主人反馈: AI 曾虚构「弹窗批准」): 如实描述真实机制, 禁止虚构流程.
const AUTH_RULE: &str = "关于工具授权, 如实说明 (不要虚构交互流程): \
高危工具 (FileOperator/ShellExec 等) 被拒时, 系统会生成一条待批授权请求, \
主人在页面「授权请求」区看到并批准 (或主人用⚙授权面板主动授权)。\
你不应描述不存在的「弹窗/系统自动弹出」流程; 被拒后如实说「本座已向主人发出授权请求, 主人批准后本座再试」。";

/// 通用记忆提炼器 (真 MiniMax): 对话/记忆 → 结构化提炼 (facts/preferences/commitments/emotional).
/// v2 (2026-08-16): 每条带 importance (1-10, Generative Agents 式 LLM 打分) + Mem0 式对账.
pub struct MiniMaxMemoryExtractor {
    pipeline: Arc<Pipeline>,
}

#[async_trait::async_trait]
impl MemoryExtractor for MiniMaxMemoryExtractor {
    async fn extract(&self, context: &str) -> Result<ExtractedMemory, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!("你是阿佩瑞斯的记忆提炼员。从对话/记忆中提炼「值得长期记住」的信息, 只输出 JSON: {\"facts\": [{\"content\": \"事实\", \"importance\": 1-10}], \"preferences\": [{\"content\": \"主人偏好(审美/风格/语气/交互)\", \"importance\": 1-10}], \"commitments\": [{\"content\": \"约定/承诺\", \"importance\": 1-10}], \"emotional\": \"情绪信号一句或 null\", \"graph\": [{\"subject\": \"主体\", \"predicate\": \"关系\", \"object\": \"客体\", \"importance\": 1-10}]}。importance 打分: 1=琐碎 5=普通 10=深刻重要。graph 填可结构化的稳定事实 (如 主人 备考 高数期中), 不填临时状态。原则: 只提炼新信息, 宁缺毋滥, 没把握就留空数组。"),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(format!("材料:\n{context}")),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.2),
            max_tokens: Some(600),
            stream: false,
            stop: None,
            tools: None,
            tool_choice: None,
        };
        let normalized = openai_chat_to_normalized(&req);
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, normalized)
            .await
            .map_err(|e| format!("提炼 LLM 调用失败: {e}"))?;
        let chat = openai_chat_from_normalized(&resp);
        let content = chat
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        let text = match content.find("</think>") {
            Some(i) => content[i + 8..].to_string(),
            None => content,
        };
        let (start, end) = match (text.find('{'), text.rfind('}')) {
            (Some(a), Some(b)) if b > a => (a, b + 1),
            _ => return Err("提炼 JSON 解析失败 (如实放弃)".to_string()),
        };
        serde_json::from_str(&text[start..end]).map_err(|e| format!("提炼 JSON 解析失败: {e}"))
    }

    /// 对账 (Mem0 式): 候选 vs 已有记忆 → LLM 判定 ADD/UPDATE/DELETE.
    /// 治 append-only「同一事实存七遍/新旧矛盾并存」. existing 格式: "id|内容".
    async fn reconcile(
        &self,
        candidates: &ExtractedMemory,
        existing: &[String],
    ) -> Result<Vec<ReconcileAction>, String> {
        if existing.is_empty() {
            // 无存量 → 全 Add (诚实)
            let mut out = Vec::new();
            for f in &candidates.facts {
                out.push(ReconcileAction { kind: ReconcileKind::Add, item: f.clone(), target_id: None });
            }
            for p in &candidates.preferences {
                out.push(ReconcileAction { kind: ReconcileKind::Add, item: p.clone(), target_id: None });
            }
            for c in &candidates.commitments {
                out.push(ReconcileAction { kind: ReconcileKind::Add, item: c.clone(), target_id: None });
            }
            return Ok(out);
        }
        let cand_json = serde_json::to_string(candidates).unwrap_or_default();
        let list: String = existing
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{i}. {}", c.chars().take(100).collect::<String>()))
            .collect::<Vec<_>>()
            .join("\n");
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!("你是记忆对账员。候选新记忆 vs 已有记忆, 判定每条的处置, 只输出 JSON 数组: [{\"action\": \"add|update|delete\", \"content\": \"最终内容\", \"importance\": 1-10, \"target_index\": 已有记忆编号或 null}]。规则: 与已有重复/被包含 → update 合并 (target_index 指旧条目); 与旧矛盾 → update 取代; 全新 → add; 无价值 → delete (target_index 可 null)。"),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(format!("候选: {cand_json}\n已有记忆:\n{list}")),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.1),
            max_tokens: Some(600),
            stream: false,
            stop: None,
            tools: None,
            tool_choice: None,
        };
        let normalized = openai_chat_to_normalized(&req);
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, normalized)
            .await
            .map_err(|e| format!("对账 LLM 调用失败: {e}"))?;
        let chat = openai_chat_from_normalized(&resp);
        let content = chat
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        let text = match content.find("</think>") {
            Some(i) => content[i + 8..].to_string(),
            None => content,
        };
        let (start, end) = match (text.find('['), text.rfind(']')) {
            (Some(a), Some(b)) if b > a => (a, b + 1),
            _ => return Err("对账 JSON 解析失败 (如实放弃)".to_string()),
        };
        #[derive(serde::Deserialize)]
        struct Raw {
            action: String,
            #[serde(default)]
            content: String,
            #[serde(default)]
            importance: u8,
            #[serde(default)]
            target_index: Option<usize>,
        }
        let raws: Vec<Raw> = serde_json::from_str(&text[start..end]).unwrap_or_default();
        let out: Vec<ReconcileAction> = raws
            .into_iter()
            .map(|r| {
                let kind = match r.action.as_str() {
                    "update" => ReconcileKind::Update,
                    "delete" => ReconcileKind::Delete,
                    _ => ReconcileKind::Add,
                };
                let target_id = r.target_index.and_then(|i| existing.get(i).map(|s| {
                    // existing 格式 "id|内容" → 取 id
                    s.split('|').next().unwrap_or(s).to_string()
                }));
                ReconcileAction {
                    kind,
                    item: MemoryItem::new(r.content, r.importance),
                    target_id,
                }
            })
            .collect();
        Ok(out)
    }
}

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
        ("FileOperator", "文件操作 (read/write/list; 需主人授权面板批准/权限包覆盖路径)", json!({"type":"object","properties":{"op":{"type":"string","enum":["read","write","list"]},"path":{"type":"string"},"content":{"type":"string"}},"required":["op","path"]})),
        ("gh_accel", "GitHub 加速: 节点池实测选最快", json!({"type":"object","properties":{"limit":{"type":"number"},"github_url":{"type":"string"}}})),
        ("dx_check", "换元法 dx 检查 (忘换 dx/混用/缺微分/根号模式)", json!({"type":"object","properties":{"problem":{"type":"string"},"substitution":{"type":"string"},"after":{"type":"string"}},"required":["problem"]})),
        ("ShellExec", "执行命令 (高危, 需主人在授权面板批准 — 权限洋葱, 本座不接触你的 token); 不走 shell 防注入, Windows 下用 cmd /c 前缀 (如 \"cmd /c echo hi\")", json!({"type":"object","properties":{"cmd":{"type":"string"}},"required":["cmd"]})),
        ("save_experience", "沉淀经验入经验库 (自成长管道): scene+practice+result+outcome", json!({"type":"object","properties":{"scene":{"type":"string"},"practice":{"type":"string"},"result":{"type":"string"},"outcome":{"type":"string","enum":["success","failure","partial"]}},"required":["scene","practice"]})),
        ("list_experience", "查经验库 (自成长管道)", json!({"type":"object","properties":{"scene":{"type":"string"}}})),
        ("verify_experience", "验证经验 (成功/失败) → 计数+评分, 达标促能力提案", json!({"type":"object","properties":{"id":{"type":"string"},"success":{"type":"boolean"}},"required":["id","success"]})),
        ("propose_principle", "提案原则候选 (动态原则层/洋葱外层): statement+rationale+source", json!({"type":"object","properties":{"statement":{"type":"string"},"rationale":{"type":"string"},"source":{"type":"string"}},"required":["statement","rationale"]})),
        ("approve_principle", "主人批准原则 (需 master token; 生效后叠加到工具执行检查)", json!({"type":"object","properties":{"id":{"type":"string"},"master_token":{"type":"string"}},"required":["id","master_token"]})),
        ("goal_create", "建立当前目标 (模块 6): objective+max_rounds; 已有未完成目标则拒绝", json!({"type":"object","properties":{"objective":{"type":"string"},"max_rounds":{"type":"number"}},"required":["objective"]})),
        ("goal_status", "查询当前目标状态 (phase/revision/rounds/blocked)", json!({"type":"object"})),
        ("goal_complete", "目标完成 → completed (可建新目标)", json!({"type":"object"})),
        ("goal_pause", "暂停当前目标 (active → paused)", json!({"type":"object"})),
        ("goal_block", "报告目标受阻 (active → blocked + 原因)", json!({"type":"object","properties":{"code":{"type":"string"},"message":{"type":"string"}}})),
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
    /// 主动送达广播 (模块 4: daemon 涌现/事件 → SSE 推送前端).
    events: tokio::sync::broadcast::Sender<String>,
    /// 机制装配器 (CompanionApp: 注入管线/提炼/摘要/自成长).
    app: Arc<CompanionApp>,
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
// 真 LLM 组件 (共享 pipeline; lib 零 LLM 依赖 — 实现全部在此)
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

/// 深度反思器 (模块 5, 真 MiniMax): 周期记忆 → 洞察/模式/建议 (markdown 文本).
pub struct MiniMaxReflector {
    pipeline: Arc<Pipeline>,
}

#[async_trait::async_trait]
impl ReflectionReflector for MiniMaxReflector {
    async fn reflect(&self, context: &str) -> Result<String, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!("你是阿佩瑞斯, 在做周期自我反思。基于近期记忆与事件, 输出深度反思 (markdown): ① 观察到的模式 (主人的习惯/偏好变化) ② 值得注意的洞察 ③ 对未来的具体建议 (含可执行经验)。不超过 300 字, 真诚不套话。"),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(format!("近期记忆:\n{context}")),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.5),
            max_tokens: Some(500),
            stream: false,
            stop: None,
            tools: None,
            tool_choice: None,
        };
        let normalized = openai_chat_to_normalized(&req);
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, normalized)
            .await
            .map_err(|e| format!("深度反思 LLM 调用失败: {e}"))?;
        let chat = openai_chat_from_normalized(&resp);
        let content = chat
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        let text = match content.find("</think>") {
            Some(i) => content[i + 8..].to_string(),
            None => content,
        };
        if text.trim().is_empty() {
            return Err("深度反思返回空".to_string());
        }
        Ok(text.trim().to_string())
    }
}

/// 深度召回 (DeepRecall trait, 真 MiniMax): LLM 从候选记忆选与 query 最相关的 top 5.
/// VCP AIMemoHandler 精神; 失败 → 装配器降级普通注入.
pub struct MiniMaxDeepRecall {
    pipeline: Arc<Pipeline>,
}

#[async_trait::async_trait]
impl DeepRecall for MiniMaxDeepRecall {
    async fn recall(&self, query: &str, candidates: &[String]) -> Result<Vec<String>, String> {
        let list: String = candidates
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{i}. {}", c.chars().take(100).collect::<String>()))
            .collect::<Vec<_>>()
            .join("\n");
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!("你是阿佩瑞斯的记忆检索员。根据主人的问题, 从候选记忆里选出最相关的 3-5 条, 只输出 JSON 数组: [编号]。无关则输出 []。"),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(format!("主人问题: {query}\n候选记忆:\n{list}")),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.1),
            max_tokens: Some(100),
            stream: false,
            stop: None,
            tools: None,
            tool_choice: None,
        };
        let normalized = openai_chat_to_normalized(&req);
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, normalized)
            .await
            .map_err(|e| e.to_string())?;
        let chat = openai_chat_from_normalized(&resp);
        let content = chat
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        let text = match content.find("</think>") {
            Some(i) => content[i + 8..].to_string(),
            None => content,
        };
        let (start, end) = match (text.find('['), text.rfind(']')) {
            (Some(a), Some(b)) if b > a => (a, b + 1),
            _ => return Err("召回 JSON 解析失败".to_string()),
        };
        let idxs: Vec<usize> = serde_json::from_str(&text[start..end]).unwrap_or_default();
        let out: Vec<String> = idxs
            .into_iter()
            .filter_map(|i| candidates.get(i).cloned())
            .take(5)
            .collect();
        if out.is_empty() {
            Err("无相关记忆".to_string())
        } else {
            Ok(out)
        }
    }
}

/// 滚动摘要 (DialogSummarizer trait, 真 MiniMax): 旧段 → 摘要.
/// sum-* 链式基线由装配器查库提供 (prev_summary), 持久化也由装配器完成.
pub struct MiniMaxDialogSummarizer {
    pipeline: Arc<Pipeline>,
}

#[async_trait::async_trait]
impl DialogSummarizer for MiniMaxDialogSummarizer {
    async fn summarize(&self, text: &str, prev_summary: Option<&str>) -> Result<String, String> {
        let base = match prev_summary {
            Some(p) => format!("【上次摘要】{p}\n\n"),
            None => String::new(),
        };
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!("把这段对话(含上次摘要基线)压缩成 100 字以内的最新摘要, 保留关键事实/约定/情绪, 只输出摘要。"),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(format!("{base}对话:\n{}", text.chars().take(3000).collect::<String>())),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(200),
            stream: false,
            stop: None,
            tools: None,
            tool_choice: None,
        };
        let normalized = openai_chat_to_normalized(&req);
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, normalized)
            .await
            .map_err(|e| e.to_string())?;
        let chat = openai_chat_from_normalized(&resp);
        let content = chat
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        let text = match content.find("</think>") {
            Some(i) => content[i + 8..].to_string(),
            None => content,
        };
        let t = text.trim();
        if t.is_empty() {
            return Err("摘要 LLM 返回空".to_string());
        }
        Ok(t.to_string())
    }
}

/// 反思→经验 (ExperienceRefiner trait, 真 MiniMax): 从反思记录提炼一条可复用经验.
/// 0 假装: 提炼失败/解析失败 → 如实返回 Err, 不硬造经验.
pub struct MiniMaxExperienceRefiner {
    pipeline: Arc<Pipeline>,
}

#[async_trait::async_trait]
impl ExperienceRefiner for MiniMaxExperienceRefiner {
    async fn refine(&self, reflects: &[String]) -> Result<Option<Experience>, String> {
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
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, normalized)
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
        Ok(Some(Experience {
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
}

/// 模块 4: 开发用测试事件 (验证 SSE 推送链路; 生产事件 = 涌现/做梦/反思自动推送).
async fn test_event(State(st): State<Arc<AppState>>) -> impl IntoResponse {
    let _ = st.events.send("测试事件: 本座在 (SSE 链路验证)".to_string());
    Json(json!({"ok": true, "note": "已推送测试事件到 SSE"}))
}

/// 模块 4: SSE 事件流 (主动送达 — 涌现/做梦/反思完成等实时推送).
async fn events(State(st): State<Arc<AppState>>) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let rx = st.events.subscribe();
    let stream = futures::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(text) => return Some((Ok::<_, Infallible>(SseEvent::default().data(text)), rx)),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue, // 跳过期消息
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// 伙伴主链路: 喂节律 → CompanionApp 注入管线 → LLM+工具循环 → OpenAI 兼容响应.
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
    // 当前问题 (最后一条 user 消息; 供推理召回/提炼)
    let query = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| match &m.content {
            serde_json::Value::String(s) => s.clone(),
            _ => String::new(),
        })
        .unwrap_or_default();
    // 注入管线 (CompanionApp, ContextAssembler 统一预算):
    // identity 块 (L0) → 独立 persona 消息; 其余块 → 合并记忆注入消息
    let blocks = st.app.build_injection(&query).await;
    let mut injections: Vec<String> = Vec::new();
    for b in &blocks {
        if b.name == "identity" {
            messages.insert(
                0,
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!(b.content),
                    tool_calls: None,
                    tool_call_id: None,
                },
            );
        } else {
            injections.push(b.content.clone());
        }
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
    // 上下文管理 (模块 3): 长对话 → 滚动摘要 + 保留注入区 + 最近 30 条.
    // 被裁旧段尝试 LLM 摘要 (节流); 失败 → 丢弃 + 诚实提示 (不硬造).
    if messages.len() > 34 {
        let head_end = messages
            .iter()
            .position(|m| m.role != "system")
            .unwrap_or(0);
        let overflow: Vec<OpenAiChatMessage> = messages[head_end..messages.len() - 30].to_vec();
        let tail: Vec<OpenAiChatMessage> = messages[messages.len() - 30..].to_vec();
        let mut head: Vec<OpenAiChatMessage> = messages[..head_end].to_vec();
        if !overflow.is_empty() {
            if st.app.summarize_due() {
                let text = overflow
                    .iter()
                    .map(|m| format!("[{}] {}", m.role, m.content.as_str().unwrap_or("")))
                    .collect::<Vec<_>>()
                    .join("\n");
                if let Some(summary) = st.app.summarize_dialog(&text).await {
                    head.push(OpenAiChatMessage {
                        role: "system".to_string(),
                        content: json!(format!("【早期对话摘要】{summary}")),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                } else {
                    head.push(OpenAiChatMessage {
                        role: "system".to_string(),
                        content: json!("【早期对话摘要】(摘要失败, 已裁剪 — 细节已由记忆系统提炼)"),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
            } else {
                head.push(OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!("【早期对话摘要】(已裁剪 — 细节已由记忆系统提炼)"),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
        }
        head.extend(tail);
        messages = head;
    }

    let tools = tools_schema(&st.bridge.registry);
    let mut final_content: String;
    let mut notes: Vec<String> = Vec::new();
    // 输出上限: 客户端请求值优先 (VCP 精神: 用户可设), env 默认, 上限保护
    let env_max = std::env::var("APEIRETH_MAX_TOKENS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_MAX_TOKENS)
        .clamp(256, MAX_TOKENS_CAP);
    let out_tokens = req.max_tokens.filter(|v| *v > 0).unwrap_or(env_max).clamp(256, MAX_TOKENS_CAP);
    let mut rounds = 0usize;
    loop {
        rounds += 1;
        let req2 = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: messages.clone(),
            temperature: Some(0.6),
            max_tokens: Some(out_tokens),
            stream: false,
            stop: None,
            tools: Some(tools.clone()),
            tool_choice: Some(json!("auto")),
        };
        let Some((content, tcs)) = chat_once(&st.pipeline, &req2, rounds).await else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": {"message": "模型服务暂时不可用 (MiniMax 限流) — 本座已尽力, 请过 10-30 秒再试"}})),
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
            "features": ["memory_injection", "today_summary", "tool_bridge", "daemon_resident", "memory_extractor", "l0_identity", "l1_essential_story"],
            "note": "Apeireth 伙伴主链路: CompanionApp 注入管线 (L0/L1 常驻), 工具桥执行, daemon 同进程常驻"
        }
    });

    // 对话后节流提炼 (通用记忆捕获): CompanionApp 节流判断 → 异步提炼并对账写入.
    // fire-and-forget: 不影响响应; 提炼失败只记日志 (限流时放弃, 下个窗口再试).
    if st.app.extraction_due() {
        let app = Arc::clone(&st.app);
        tokio::spawn(async move {
            app.run_extraction(12).await;
        });
    }

    (StatusCode::OK, Json(resp)).into_response()
}

async fn chat_once(
    pipeline: &Arc<Pipeline>,
    req: &OpenAiChatRequest,
    label: usize,
) -> Option<(String, Vec<Value>)> {
    // 限流重试策略 (2026-08-16 实测: MiniMax 限流严重, 5×8s=40s+ 静默等待体感极差):
    // 最多 3 次 × 6s 退避; 仍失败 → 快速失败 (让用户明确知道限流, 优于无声长等)
    for attempt in 0..3 {
        let normalized = openai_chat_to_normalized(req);
        let t0 = std::time::Instant::now();
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
                // 空响应 (MiniMax 异常时 200 但内容空) → 视为失败重试, 不静默返回空白
                if content.trim().is_empty() && tcs.is_empty() {
                    eprintln!("  [管线] 轮{label} 空响应 (MiniMax 异常), 重试");
                    tokio::time::sleep(Duration::from_secs(4)).await;
                    continue;
                }
                eprintln!("[llm] 轮{label} 成功 ({}ms)", t0.elapsed().as_millis());
                return Some((content, tcs));
            }
            Err(e) => {
                eprintln!("  [管线] 轮{label} 第{}次失败: {e}, 6s 后重试", attempt + 1);
                tokio::time::sleep(Duration::from_secs(6)).await;
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

    // 目标服务 (持久目录 %APPDATA%\apeireth\goals; 与工具桥/装配器共享同一实例)
    let goal_dir = apeireth_companion::daemon::default_memory_path()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::temp_dir().join("apeireth-goals"))
        .join("goals");
    let mut goals = GoalService::new(&goal_dir);
    goals.restore("goal-main");
    let goals_shared: std::sync::Arc<std::sync::Mutex<GoalService>> = std::sync::Arc::new(std::sync::Mutex::new(goals));

    // ② 工具桥全增强 (宪法 LLM 评审 + 目标工具 + 显式扩权 APEIRETH_GRANT="FileOperator:24;Git:12")
    let pipeline = Arc::new(build_pipeline(BASE_URL.to_string(), Some(key.clone()))?);
    let bridge = Arc::new(
        ToolBridge::new(Arc::clone(&store))
            .with_judicator(Arc::new(LlmJudicator::new(Arc::new(MiniMaxConstitutionLlm {
                pipeline: Arc::clone(&pipeline),
            }))))
            .with_goals(std::sync::Arc::clone(&goals_shared)),
    );
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

    // 主动送达广播 (模块 4: daemon 涌现/事件 → SSE 推送)
    let (tx_events, _) = tokio::sync::broadcast::channel::<String>(64);

    // ③ CompanionApp 机制装配 (注入管线/提炼/摘要/自成长; 全部 LLM 实现注入)
    let extract_interval = std::env::var("APEIRETH_EXTRACT_INTERVAL_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(600));
    let rhythm_share: std::sync::Arc<std::sync::Mutex<Option<RhythmEstimate>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let app = Arc::new(
        CompanionApp::new(Arc::clone(&store), MEMORY_SESSION)
            // L0: Identity 常驻 (persona + 约束, 永不截断)
            .with_identity(format!("{PERSONA}\n{CLAIM_RULE}\n{AUTH_RULE}"))
            // L1: Essential Story 常驻 (mempalace §5.6 渐进加载; essential-*/高 importance)
            .with_essential_budget(800)
            .with_inject_budget(6000)
            .with_rhythm(std::sync::Arc::clone(&rhythm_share))
            .with_goal(std::sync::Arc::clone(&goals_shared))
            .with_extractor(Arc::new(MiniMaxMemoryExtractor { pipeline: Arc::clone(&pipeline) }))
            .with_summarizer(Arc::new(MiniMaxDialogSummarizer { pipeline: Arc::clone(&pipeline) }))
            .with_refiner(Arc::new(MiniMaxExperienceRefiner { pipeline: Arc::clone(&pipeline) }))
            .with_deep_recall(Arc::new(MiniMaxDeepRecall { pipeline: Arc::clone(&pipeline) }))
            .with_extract_interval(extract_interval)
            .with_summarize_interval(Duration::from_secs(300)),
    );
    println!("[app] CompanionApp 装配完成: L0 Identity + L1 Essential 常驻, 提炼 {:?} 节流", extract_interval);

    // ④ daemon 常驻 (做梦 LLM 摘要 + 反思 LLM 深度 + 涌现 LLM 润色, 同进程): 记忆会话 "me"
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
    .with_period(reflect_period)
    .with_reflector(Arc::new(MiniMaxReflector {
        pipeline: Arc::clone(&pipeline),
    }));
    let tone = tone_hint(&apeireth_companion::bond::Bond::new());
    // 送达通道: 广播 (SSE) 必开; Lark (离线) 有凭据则叠加
    let mut sink = MultiSink::new().push(Box::new(apeireth_companion::daemon::BroadcastSink::new(tx_events.clone())));
    match LarkSink::from_env() {
        Ok(lark) => {
            sink = sink.push(Box::new(lark));
            println!("[sink] Lark 离线送达已启用 (凭据有效)");
        }
        Err(e) => {
            println!("[sink] Lark 未启用 (需要 APEIRETH_LARK_APP_ID/SECRET/RECEIVE_ID): {e}");
        }
    }
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
            sink,
        ),
        MemoryContextSource::new(Arc::clone(&store)),
        MEMORY_SESSION.to_string(),
        Duration::from_secs(60),
    )
    .with_dream(dream)
    .with_reflection(reflect);
    println!("[daemon] 常驻: 做梦(LLM 摘要, 安静期 {:?}) + 反思({:?}, LLM 深度) + 涌现(LLM 润色, SSE 推送)", quiet, reflect_period);

    // 互动通知通道: handler 发「主人来消息了」, daemon 喂节律 + 重置做梦安静期
    let (tx_interact, rx_interact) = tokio::sync::mpsc::channel::<chrono::DateTime<Utc>>(64);

    // ⑤ HTTP 伙伴端点
    let state = Arc::new(AppState {
        bridge,
        store,
        pipeline,
        interactions: tx_interact,
        events: tx_events,
        app,
        subject,
    });
    let router = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/apeireth/grant", post(grant))
        .route("/v1/apeireth/approval-requests", get(approval_requests))
        .route("/v1/apeireth/events", get(events))
        .route("/v1/apeireth/test-event", post(test_event))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    println!("✅ companion_serve v4 — 伙伴端点全能力版 (CompanionApp 机制装配)");
    println!("   http://127.0.0.1:{port}/v1  (模型 MiniMax-M3, Key 任意非空)");
    println!("   会话标签: X-Apeireth-Continuity (缺省 {}) · 工具: 全部可见, 执行受宪法/权限约束", state.subject.as_str());
    // daemon 循环与 HTTP 同 task 交替 (daemon 内部 RefCell 跨 await → 非 Send, 不能 spawn)
    let d_app = Arc::clone(&state.app);
    let d_rhythm = rhythm_share;
    tokio::select! {
        r = axum::serve(listener, router) => { r?; }
        _ = daemon_loop(daemon, rx_interact, d_app, d_rhythm) => {}
    }
    Ok(())
}

/// daemon 常驻循环: 定时 step (做梦/反思/涌现) + 响应互动通知 (喂节律)
/// + 自成长延伸 (CompanionApp): 反思完成→提炼经验入库; 晋级候选自动成文.
/// 具体类型 (Delivery trait 私有, 不能作泛型约束); daemon 非 Send, 只在同 task 内用.
type ServeDaemon = CompanionDaemon<
    CompanionDelivery<ThrottledUtterance<TonalUtterance>, MultiSink>,
    MemoryContextSource,
>;
async fn daemon_loop(
    mut daemon: ServeDaemon,
    mut rx: tokio::sync::mpsc::Receiver<chrono::DateTime<Utc>>,
    app: Arc<CompanionApp>,
    rhythm_share: std::sync::Arc<std::sync::Mutex<Option<RhythmEstimate>>>,
) {
    let mut last_cycles: u64 = daemon
        .reflection
        .as_ref()
        .map(|r| r.cycles_completed())
        .unwrap_or(0);
    let mut last_batch_extract = std::time::Instant::now();
    let mut ticker = tokio::time::interval(Duration::from_secs(60));
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let t0 = std::time::Instant::now();
                daemon.step().await;
                // 节律共享 (模块 1 状态感知): 每 tick 更新活跃概率 (UTC 坐标, 与观察自洽)
                {
                    let now = Utc::now();
                    let mins = now.hour() * 60 + now.minute();
                    let est = daemon.awake.loop_.rhythm.estimate(mins);
                    if let Ok(mut share) = rhythm_share.lock() {
                        *share = Some(est);
                    }
                }
                // 延伸 1: 反思周期完成 → LLM 提炼经验入经验库 (自成长管道 Level 0)
                let cycles = daemon.reflection.as_ref().map(|r| r.cycles_completed()).unwrap_or(0);
                if cycles > last_cycles {
                    eprintln!("[growth] 反思周期完成 (累计 {cycles}), 提炼经验...");
                    let reflects: Vec<String> = app.store()
                        .recent_episodes(app.session(), 100)
                        .unwrap_or_default()
                        .iter()
                        .filter(|e| e.id.starts_with("reflect-"))
                        .take(3)
                        .map(|e| e.content.clone())
                        .collect();
                    match app.refine_experience(&reflects).await {
                        Ok(Some(exp)) => {
                            ExperienceStore::new(Arc::clone(app.store())).save(&exp)
                                .map(|_| eprintln!("[growth] 经验入库: {}", exp.scene))
                                .unwrap_or_else(|e| eprintln!("[growth] 经验入库失败: {e}"));
                        }
                        Ok(None) => eprintln!("[growth] 本次反思无可提炼经验"),
                        Err(e) => eprintln!("[growth] 经验提炼失败: {e}"),
                    }
                    last_cycles = cycles;
                }
                // 批量记忆提炼 (与做梦同频: 6h 节流; 通用捕获 — 偏好/事实/约定):
                // 0 假装: 按时间节流而非做梦事件精确绑定 (DreamScheduler 无公开计数器)
                if last_batch_extract.elapsed() >= Duration::from_secs(6 * 3600) {
                    last_batch_extract = std::time::Instant::now();
                    eprintln!("[extract] 批量提炼 (6h 周期)...");
                    app.run_extraction(30).await;
                }
                // 延伸 3: 晋级候选自动成文 (数据目录 promotion-candidates.md; 空则不写)
                if let Some(path) = app.export_promotion_candidates() {
                    eprintln!("[growth] 晋级候选已成文: {:?}", path);
                }
                eprintln!("[daemon-loop] tick done in {:?}", t0.elapsed());
            }
            Some(at) = rx.recv() => daemon.on_user_message(at),
            else => break,
        }
    }
}

/// 待批授权请求 (AI 被拒时产生; 前端轮询展示, 主人一键批准 — 权限洋葱真实载体).
async fn approval_requests(State(st): State<Arc<AppState>>) -> impl IntoResponse {
    Json(apeireth_companion::approval_requests::pending_json(&st.store))
}

/// 主人批准端点 (权限洋葱对齐): 主人带 master token 直接批准工具授权 (PermissionPack),
/// AI 只请求不接触 token. 授权后高危工具在时限内可直接执行.
async fn grant(
    State(st): State<Arc<AppState>>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    let tool = req.get("tool").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty());
    let Some(tool) = tool else {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "需要 tool (工具名)"}))).into_response();
    };
    let hours = req.get("hours").and_then(|v| v.as_u64()).unwrap_or(1).max(1).min(24 * 30);
    let token = req.get("master_token").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let expected = std::env::var("APEIRETH_MASTER_TOKEN").unwrap_or_default();
    if expected.is_empty() || token != expected {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "master token 不匹配 (主人授权权在主人手里)"})),
        )
            .into_response();
    }
    st.bridge.packs.grant(apeireth_companion::packs::PermissionPack::timed(
        "主人授权",
        vec![tool.to_string()],
        hours,
        None,
    ));
    (
        StatusCode::OK,
        Json(json!({"ok": true, "tool": tool, "hours": hours, "note": "已按权限洋葱授权 (PermissionPack); 到期自动失效"})),
    )
        .into_response()
}

/// 内置聊天页 (零依赖单文件前端, 浏览器打开即用; 供主人/任何前端先体验).
async fn index() -> impl IntoResponse {
    axum::response::Html(include_str!("../assets/chat.html").to_string())
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "apeireth-companion-serve-v4",
        "version": env!("CARGO_PKG_VERSION"),
        "features": ["persistent_memory", "daemon_resident", "dream_llm_summarizer", "utterance_llm", "constitution_llm_judicator", "memory_injection", "today_summary", "tool_bridge_all", "openai_compat", "companion_app", "l0_identity", "l1_essential_story"],
    }))
}

async fn list_models() -> impl IntoResponse {
    Json(json!({
        "object": "list",
        "data": [{"id": MODEL, "object": "model", "created": 0, "owned_by": "minimax"}]
    }))
}
