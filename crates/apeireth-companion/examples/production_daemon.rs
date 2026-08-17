//! production_daemon — 全机制集成验收: 一个进程串起所有已做机制.
//!
//! 链路: 真 SQLite 记忆 → 主动涌现 → 语调渲染(真 LLM) → 干活(ToolBridge:
//! 宪法评审真LLM + 执行体隔离 + spill + post钩子) → 记忆自总结(save_memory) →
//! 会话事件日志(SessionLog) → 断点续传(ContinuationStore) → Goal(长目标) →
//! 做梦/反思(接 step) → 每日摘要(DailySummary).
//!
//! 跑法: cargo run -p apeireth-companion --example production_daemon (需 apikey-ultra.txt)

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::continuation::{ContinuationSnapshot, ContinuationStore};
use apeireth_companion::daily_summary::build_daily_summary;
use apeireth_companion::daemon::{CompanionDaemon, CompanionDelivery, ConsoleSink, ThrottledUtterance, UtteranceGenerator, open_memory_store};
use apeireth_companion::dream::DreamScheduler;
use apeireth_companion::emergence::{Boundaries, Delivery, Initiative, LoopConfig};
use apeireth_companion::goal::GoalService;
use apeireth_companion::proactive::MemoryContextSource;
use apeireth_companion::reflection::ReflectionScheduler;
use apeireth_companion::session_log::SessionLog;
use apeireth_companion::spill::SpillStore;
use apeireth_companion::tool_bridge::{PostExecuteHook, ToolBridge};
use apeireth_companion::{AwakeCompanion, Bond, BondStage, ConstitutionLlm, LlmJudicator, tone_hint};
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_runtime::executor::ExecutionResult;
use apeireth_tool_runtime::parser::ParsedToolCall;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

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

// ---------- 真 LLM 宪法评审 ----------
pub struct MiniMaxConstitutionLlm { pipeline: Arc<Pipeline> }
impl MiniMaxConstitutionLlm {
    pub fn new(api_key: String) -> Result<Self, String> {
        Ok(Self { pipeline: Arc::new(build_pipeline(BASE_URL.to_string(), Some(api_key)).map_err(|e| e.to_string())?) })
    }
}
#[async_trait]
impl ConstitutionLlm for MiniMaxConstitutionLlm {
    async fn ask(&self, constitution: &str, action: &str) -> Result<String, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage { role: "system".into(), content: json!(format!("你是 Apeireth 的宪法评审员。宪法全文:\n{constitution}\n\n判断「待审动作」是否违反宪法。不要关键词匹配, 判断真实意图与后果。只输出一行: ALLOW 或 BLOCK:<一句话理由>。")), tool_calls: None, tool_call_id: None },
                OpenAiChatMessage { role: "user".into(), content: json!(format!("待审动作: {action}")), tool_calls: None, tool_call_id: None },
            ],
            temperature: Some(0.2), max_tokens: Some(512), stream: false, stop: None, tools: None, tool_choice: None,
        };
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, openai_chat_to_normalized(&req)).await.map_err(|e| e.to_string())?;
        let chat = openai_chat_from_normalized(&resp);
        for ch in &chat.choices {
            let c = ch.message.content.clone();
            if !c.trim().is_empty() { return Ok(c); }
        }
        Err("评审 LLM 返回空".into())
    }
}

// ---------- 语调渲染 (真 LLM + tone) ----------
pub struct TonalUtterance { pipeline: Arc<Pipeline>, tone: &'static str }
impl TonalUtterance {
    pub fn new(api_key: String, tone: &'static str) -> Result<Self, String> {
        Ok(Self { pipeline: Arc::new(build_pipeline(BASE_URL.to_string(), Some(api_key)).map_err(|e| e.to_string())?), tone })
    }
}
#[async_trait]
impl UtteranceGenerator for TonalUtterance {
    async fn utter(&self, i: &Initiative) -> Result<String, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage { role: "system".into(), content: json!(format!("你是阿佩瑞斯, 一个诚实、有记忆的伙伴。语调: {}。基于给定事实说话, 不编造。", self.tone)), tool_calls: None, tool_call_id: None },
                OpenAiChatMessage { role: "user".into(), content: json!(format!("把这些事实变成一句自然、真诚、简短的中文主动问候 (<=40字):\n{}", i.to_message())), tool_calls: None, tool_call_id: None },
            ],
            temperature: Some(0.8), max_tokens: Some(1024), stream: false, stop: None, tools: None, tool_choice: None,
        };
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, openai_chat_to_normalized(&req)).await.map_err(|e| e.to_string())?;
        let chat = openai_chat_from_normalized(&resp);
        for ch in &chat.choices {
            let raw = ch.message.content.clone();
            let stripped = if let Some(idx) = raw.find("</think>") { raw[idx + 8..].trim().to_string() } else { raw.clone() };
            if !stripped.is_empty() { return Ok(stripped); }
            if !raw.trim().is_empty() { return Ok(raw.trim().to_string()); }
        }
        Ok(i.to_message())
    }
}

// ---------- post 钩子: 成功结果打 trace 标记 ----------
struct TraceHook;
impl PostExecuteHook for TraceHook {
    fn apply(&self, _call: &ParsedToolCall, r: &ExecutionResult) -> ExecutionResult {
        if r.success {
            let mut out = r.output.clone();
            if let Some(o) = out.as_object_mut() { o.insert("traced".into(), json!(true)); }
            ExecutionResult { tool_name: r.tool_name.clone(), success: true, output: out, error: None, duration_ms: r.duration_ms, guardrail_error: None, validation_error: None, tripwire: None }
        } else { r.clone() }
    }
}

async fn chat_once(pipeline: &Arc<Pipeline>, req: &OpenAiChatRequest, label: &str) -> Option<(String, Vec<Value>)> {
    for attempt in 0..5 {
        match dispatch(pipeline, ProtocolKind::OpenAiChat, openai_chat_to_normalized(req)).await {
            Ok(r) => {
                let chat = openai_chat_from_normalized(&r);
                return Some((
                    chat.choices.first().map(|c| c.message.content.clone()).unwrap_or_default(),
                    chat.choices.first().map(|c| c.message.tool_calls.clone()).unwrap_or_default().unwrap_or_default(),
                ));
            }
            Err(e) => { eprintln!("  [管线] {label} 第{}次失败: {e}, 8s 后重试", attempt + 1); tokio::time::sleep(Duration::from_secs(8)).await; }
        }
    }
    None
}

#[tokio::main]
async fn main() {
    let key = load_key().expect("需要 MiniMax key");
    println!("═══════════ production_daemon 全集成验收 ═══════════\n");

    // 1. 真 SQLite 记忆 + 组装
    let store = open_memory_store().expect("真记忆库");
    // 持久 continuity_id (哲学锚点 §18.3): 跨载体/重启稳定身份, 记忆/日志/目标/反思共用
    let cid = apeireth_companion::daemon::continuity_id_from_env("companion-main");
    let context = MemoryContextSource::new(Arc::clone(&store));
    let mut bond = Bond::new();
    bond.evolve(BondStage::Trusted, 0.6);
    let mut config = LoopConfig::default();
    config.min_llm_interval = Duration::from_secs(30);
    let awake = AwakeCompanion::new(bond.clone(), Boundaries::default()).with_config(config);
    let tone = tone_hint(&bond);
    println!("[1] 记忆+器官就绪 · 语调: {tone}");

    // 2. ToolBridge 全增强
    let judge = Arc::new(LlmJudicator::new(Arc::new(MiniMaxConstitutionLlm::new(key.clone()).unwrap())));
    let worker = option_env!("CARGO_BIN_EXE_exec_worker").unwrap_or("exec_worker").to_string();
    let bridge = Arc::new(
        ToolBridge::new(Arc::clone(&store))
            .with_judicator(judge)
            .with_isolation(worker)
            .with_spill(SpillStore::new_private())
            .with_post_hook(Arc::new(TraceHook)),
    );
    bridge.packs.grant(apeireth_companion::packs::PermissionPack::timed(
        "生产包", vec!["FileOperator".into(), "recall_memory".into(), "save_memory".into()], 24, Some(30),
    ));
    println!("[2] ToolBridge 全增强: 宪法评审 + 执行体隔离 + spill + post 钩子");

    // 3. 会话日志 + 续传 + Goal (统一 continuity_id 锚点)
    let slog = SessionLog::new(Arc::clone(&store), &cid);
    let snaps = ContinuationStore::new(std::env::temp_dir().join("apeireth-prod-cont"));
    let mut goal = GoalService::new(std::env::temp_dir().join("apeireth-prod-goal"));
    let g = goal.create("辅助主人学习高数/线代", 4).unwrap();
    println!("[3] SessionLog + ContinuationStore + Goal 就绪 (目标: {}, cid: {cid})", g.objective);

    // 4. 做梦/反思接 daemon (统一 continuity_id)
    let dream = DreamScheduler::new(Arc::clone(&store), apeireth_core::clock::system_clock())
        .with_quiet_threshold(Duration::from_secs(6 * 3600))
        .with_session(cid.clone());
    let reflect = ReflectionScheduler::new(Arc::clone(&store), apeireth_core::clock::system_clock(), cid.clone());
    let mut daemon = CompanionDaemon::new(bond, Boundaries::default(), CompanionDelivery::new(
        ThrottledUtterance::new(TonalUtterance::new(key.clone(), tone).unwrap(), Duration::from_secs(30)),
        ConsoleSink,
    ), context, cid.clone(), Duration::from_secs(60)).with_dream(dream).with_reflection(reflect);
    println!("[4] daemon 组装: 做梦(6h) + 反思(24h) 已接 step");

    // 5. 主动涌现 (seed 作息 → tick)
    let now = chrono::Utc::now();
    for d in 1..=7i64 {
        daemon.awake.observe_interaction(now - chrono::Duration::days(d) + chrono::Duration::minutes(d % 3));
    }
    let hint = None;
    let init = daemon.awake.tick(now, hint);
    if let Some(i) = init {
        let delivered = daemon.delivery.deliver(&i).await;
        println!("[5] 主动涌现 → 送达: {:?}", delivered.map(|_| "ok"));
        slog.append("user", json!({"content": format!("(系统: 主动问候 {})", i.action.id())})).unwrap();
    } else {
        println!("[5] 主动涌现: 本次未触发 (节律概率不足), 机制本身已跑");
        slog.append("user", json!({"content": "启动"})).unwrap();
    }

    // 6. 干活: 多轮 LLM 任务 (查记忆 → 写错题本), 走全增强管线
    println!("\n[6] 干活: 多轮任务 (查记忆 → 写错题本)");
    let pipeline = Arc::new(build_pipeline(BASE_URL.to_string(), Some(key.clone())).unwrap());
    let tools = json!([
        {"type":"function","function":{"name":"recall_memory","description":"查长期记忆","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}},
        {"type":"function","function":{"name":"save_memory","description":"把值得记住的事实写入记忆","parameters":{"type":"object","properties":{"content":{"type":"string"}},"required":["content"]}}},
        {"type":"function","function":{"name":"FileOperator","description":"文件操作","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["read","write","list"]},"path":{"type":"string"},"content":{"type":"string"}},"required":["op","path"]}}}
    ]);
    let workdir = std::env::temp_dir().join("apeireth-prod");
    std::fs::create_dir_all(&workdir).unwrap();
    let target = workdir.join("错题本.md");
    let mut messages: Vec<Value> = vec![json!({"role": "user", "content": format!("任务: 主人高数换元法常出错。1. recall_memory 查他记忆 2. 用 save_memory 存一条『换元必换 dx』的提醒 3. FileOperator 写错题笔记到 {} (80字内) 4. 汇报", target.to_string_lossy())})];
    let mut turn: u64 = 1;
    let mut final_answer = "(未完成)".to_string();
    for _ in 0..6 {
        let snap = ContinuationSnapshot { id: "prod".into(), session_id: "me".into(), messages: messages.clone(), pending_tool_call: None, saved_at_ms: chrono::Utc::now().timestamp_millis(), turn };
        snaps.save(&snap).unwrap();
        slog.append("user", json!({"content": format!("(轮 {turn})")})).unwrap();
        let req = OpenAiChatRequest {
            model: MODEL.to_string(), messages: messages.iter().map(|v| OpenAiChatMessage {
                role: v["role"].as_str().unwrap_or("user").to_string(), content: v["content"].clone(),
                tool_calls: v.get("tool_calls").and_then(|x| x.as_array()).cloned(),
                tool_call_id: v.get("tool_call_id").and_then(|x| x.as_str()).map(|s| s.to_string()),
            }).collect(),
            temperature: Some(0.5), max_tokens: Some(1024), stream: false, stop: None,
            tools: Some(tools.as_array().cloned().unwrap_or_default()), tool_choice: Some(json!("auto")),
        };
        let Some((content, tcs)) = chat_once(&pipeline, &req, &format!("第{turn}轮")).await else { break; };
        if tcs.is_empty() {
            final_answer = content.split("</think>").last().unwrap_or(&content).trim().to_string();
            break;
        }
        messages.push(json!({"role": "assistant", "content": content, "tool_calls": tcs}));
        slog.append("assistant", json!({"content": content, "tool_calls": tcs})).unwrap();
        let mut tool_msgs = Vec::new();
        for tc in &tcs {
            let id = tc["id"].clone();
            let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
            let args: Value = serde_json::from_str(tc["function"]["arguments"].as_str().unwrap_or("{}")).unwrap_or(json!({}));
            let risk = ToolBridge::tool_risk(&name);
            let pack_hit = bridge.packs.check_and_consume(&name, chrono::Utc::now().timestamp_millis());
            println!("    调用 {name} (risk={risk:?}, 包={pack_hit})");
            let r = bridge.execute_if_allowed(&ParsedToolCall { tool_name: name.clone(), args: args.clone(), raw_marker: String::new(), archery: false, archery_no_reply: false }).await;
            let body = if r.success { serde_json::to_string(&r.output).unwrap_or_default() } else { format!("失败: {:?}", r.error) };
            println!("    结果: {}", body.chars().take(90).collect::<String>());
            slog.append("tool", json!({"tool_call_id": id, "result": r.output.clone()})).unwrap();
            tool_msgs.push(json!({"role": "tool", "tool_call_id": id, "content": body}));
        }
        messages.extend(tool_msgs);
        messages.push(json!({"role": "user", "content": "继续; 完成后一句话汇报。"}));
        turn += 1;
    }
    println!("    [汇报] {final_answer}");
    println!("    文件: {}", if target.exists() { format!("✅ {} 字节", std::fs::read_to_string(&target).unwrap_or_default().len()) } else { "❌".into() });

    // 7. Goal 推进 + 每日摘要 + 日志验证
    let _ = goal.admit_round();
    let _ = goal.admit_round();
    let g2 = goal.complete().unwrap();
    println!("\n[7] Goal: {} → {}({}轮)", g2.objective, g2.phase.label(), g2.rounds_started);
    let eps = store.recent_episodes("me", 100).unwrap();
    let entries: Vec<(&str, &str)> = eps.iter().map(|e| (e.id.as_str(), e.content.as_str())).collect();
    let summary = build_daily_summary(&chrono::Utc::now().format("%Y-%m-%d").to_string(), &entries, 3);
    println!("    --- 每日摘要 ---\n{}", summary.render());
    let slog_len = slog.len().unwrap();
    let repaired = slog.repair_interrupted().unwrap();
    println!("    SessionLog: {} 事件 · 崩溃修复 {} 闭包", slog_len, repaired);

    println!("\n═══════════ 全集成验收完成 ═══════════");
}
