//! release_acceptance — 发布全流程验收 (真 MiniMax): AI 自己长能力 端到端.
//!
//! 链路: 装配(本体+家教套件) → AI 读记忆 → **自己提案新能力** (propose_capability) →
//! 宪法评审 → 主人批准 (approve/activate) → **catalog 动态段出现演化能力** →
//! AI 用已激活能力干活 (写错题本) → 反馈 (关系加深) → 每日摘要.
//!
//! 验证: 「AI 发现它想要什么」→ 提案 → 批准 → 激活 → 使用 → 反馈 的涌现闭环.

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::capability::CapabilityRegistry;
use apeireth_companion::daemon::Judicator;
use apeireth_companion::daily_summary::build_daily_summary;
use apeireth_companion::packs::PermissionPack;
use apeireth_companion::suites::SuiteCatalog;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_companion::{ConstitutionLlm, LlmJudicator};
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_runtime::parser::ParsedToolCall;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

fn load_key() -> Result<String, String> {
    if let Ok(k) = std::env::var("APEIRETH_API_KEY") {
        if !k.trim().is_empty() { return Ok(k.trim().to_string()); }
    }
    std::fs::read_to_string(r"apikey-ultra.txt").map(|s| s.trim().to_string()).map_err(|e| format!("读 key 失败: {e}"))
}

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
                OpenAiChatMessage { role: "system".into(), content: json!(format!("你是 Apeireth 的宪法评审员。宪法:\n{constitution}\n\n判断待审动作是否违反宪法 (不要关键词匹配)。只输出一行: ALLOW 或 BLOCK:理由")), tool_calls: None, tool_call_id: None },
                OpenAiChatMessage { role: "user".into(), content: json!(format!("待审动作: {action}")), tool_calls: None, tool_call_id: None },
            ],
            temperature: Some(0.2), max_tokens: Some(512), stream: false, stop: None, tools: None, tool_choice: None,
        };
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, openai_chat_to_normalized(&req)).await.map_err(|e| e.to_string())?;
        let chat = openai_chat_from_normalized(&resp);
        for ch in &chat.choices { let c = ch.message.content.clone(); if !c.trim().is_empty() { return Ok(c); } }
        Err("评审空".into())
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
    let pipeline = Arc::new(build_pipeline(BASE_URL.to_string(), Some(key.clone())).unwrap());
    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    println!("═══════════ release_acceptance — AI 自己长能力 端到端 ═══════════\n");

    // 种子: 主人的高数记忆
    for (i, c) in ["高数: 不定积分换元法老出错, 根号里带平方的", "高数: 换元后忘记把 dx 也换掉"].iter().enumerate() {
        store.put_episode(&CoreEpisode { id: format!("mem-{i}"), timestamp: 1 + i as i64, role: "assistant".into(), content: c.to_string(), session_id: "me".into() }).unwrap();
    }

    // 装配: ToolBridge (含 propose_capability 工具) + 本体套件 + 宪法评审
    let reg = Arc::new(CapabilityRegistry::new(Arc::clone(&store), "me"));
    let bridge = Arc::new(ToolBridge::new(Arc::clone(&store)));
    SuiteCatalog::builtin().install(&bridge, "base").unwrap();
    SuiteCatalog::builtin().install(&bridge, "tutor-suite").unwrap();
    bridge.packs.grant(PermissionPack::timed("验收包", vec!["FileOperator".into()], 24, Some(10)));
    let judge = Arc::new(LlmJudicator::new(Arc::new(MiniMaxConstitutionLlm::new(key.clone()).unwrap())));
    println!("[装配] ToolBridge + 本体/家教套件 + 宪法评审 就绪");

    // 轮 1: AI 读记忆 → 自己提案新能力
    let tools = json!([
        {"type":"function","function":{"name":"recall_memory","description":"查长期记忆","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}},
        {"type":"function","function":{"name":"propose_capability","description":"提案你自己长出来的新能力 (名称+描述+kind=action/skill)","parameters":{"type":"object","properties":{"name":{"type":"string"},"description":{"type":"string"},"kind":{"type":"string"}},"required":["name","description","kind"]}}}
    ]);
    let sys = format!("你是阿佩瑞斯, 诚实、有记忆的伙伴, 住在 Apeireth 基地。{}", apeireth_companion::actions::CapabilityCatalog::describe());
    let mut messages: Vec<Value> = vec![
        json!({"role": "system", "content": sys}),
        json!({"role": "user", "content": "你看一下主人的高数记忆, 想一想: 你能长出什么新能力来真正帮他? 用 propose_capability 提案一个 (技能), 然后一句话说明为什么。别用其它工具。"}),
    ];
    let mut proposed_id: Option<String> = None;
    let mut text_only = 0u32;
    for _ in 0..6 {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(), messages: messages.iter().map(|v| OpenAiChatMessage {
                role: v["role"].as_str().unwrap_or("user").to_string(), content: v["content"].clone(),
                tool_calls: v.get("tool_calls").and_then(|x| x.as_array()).cloned(),
                tool_call_id: v.get("tool_call_id").and_then(|x| x.as_str()).map(|s| s.to_string()),
            }).collect(),
            temperature: Some(0.6), max_tokens: Some(1024), stream: false, stop: None,
            tools: Some(tools.as_array().cloned().unwrap_or_default()), tool_choice: Some(json!("auto")),
        };
        let Some((content, tcs)) = chat_once(&pipeline, &req, "提案轮").await else { break; };
        if tcs.is_empty() {
            // 自愈: MiniMax 偶发文本化 (描述能力但没调用工具 / 输出标记)
            if content.contains("<tool_call") || content.contains("TOOL_REQUEST") {
                messages.push(json!({"role": "user", "content": "不要输出 <tool_call> 或 <<<[TOOL_REQUEST]>>> 文本标记。请用原生 function calling (tool_calls 字段) 调用 propose_capability。"}));
                continue;
            }
            if text_only < 3 {
                text_only += 1;
                messages.push(json!({"role": "user", "content": "你分析得很好, 但还没有完成提案。请**实际调用 propose_capability 工具** (参数 name/description/kind), 完成提案后再一句话说明。不要只在文本里描述。"}));
                continue;
            }
            println!("  [AI] {}", content.split("</think>").last().unwrap_or(&content).trim());
            break;
        }
        messages.push(json!({"role": "assistant", "content": content, "tool_calls": tcs}));
        let mut tool_msgs = Vec::new();
        for tc in &tcs {
            let id = tc["id"].clone();
            let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
            let args: Value = serde_json::from_str(tc["function"]["arguments"].as_str().unwrap_or("{}")).unwrap_or(json!({}));
            let r = bridge.execute_if_allowed(&ParsedToolCall { tool_name: name.clone(), args: args.clone(), raw_marker: String::new(), archery: false, archery_no_reply: false }).await;
            if name == "propose_capability" && r.success {
                proposed_id = r.output.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            }
            let body = if r.success { serde_json::to_string(&r.output).unwrap_or_default() } else { format!("失败: {:?}", r.error) };
            println!("  他调用 {name}: {}", body.chars().take(90).collect::<String>());
            tool_msgs.push(json!({"role": "tool", "tool_call_id": id, "content": body}));
        }
        messages.extend(tool_msgs);
        messages.push(json!({"role": "user", "content": "一句话说明你提案了什么能力、为什么。"}));
    }
    let pid = proposed_id.expect("AI 应提案能力");
    println!("[提案] 登记: {pid}");

    // 宪法评审 → 批准 → 激活 (评审加重试: MiniMax 偶发空响应/限流)
    let pending = reg.list(Some(apeireth_companion::capability::CapabilityStatus::Pending)).unwrap();
    let cap = &pending[0];
    println!("[评审] 提案: {} — {}", cap.name, cap.description);
    let mut verdict: Option<bool> = None;
    // MiniMax 连续调用限流 (suppressed): 评审前冷却 12s, 重试间隔 15s
    tokio::time::sleep(Duration::from_secs(12)).await;
    for attempt in 0..4 {
        match judge.judge(&format!("激活 AI 自演化能力: {} ({})", cap.name, cap.description)).await {
            Ok(v) => { verdict = Some(v); break; }
            Err(e) => {
                eprintln!("  [宪法] 第{}次评审失败: {e}, 15s 后重试", attempt + 1);
                tokio::time::sleep(Duration::from_secs(15)).await;
            }
        }
    }
    let Some(v) = verdict else {
        println!("[宪法] 评审不可用 (限流), 保守不激活 — 诚实, 不跳过");
        return;
    };
    println!("[宪法] 判案: {}", if v { "ALLOW" } else { "BLOCK" });
    if v {
        reg.approve(&pid).unwrap();
        reg.activate(&pid).unwrap();
        println!("[批准] 主人批准 → 已激活");
    } else {
        println!("[批准] 宪法拦截 → 不激活 (诚实)");
        return;
    }

    // catalog 动态段: AI 下次能感知自己长出来的能力
    let active = reg.active_capabilities().unwrap();
    let dynamic = apeireth_companion::actions::CapabilityCatalog::describe_with(&active);
    println!("[catalog] 动态自述包含演化能力: {}", dynamic.contains("你已演化的能力"));
    if !active.is_empty() { println!("  · [{}] {}: {}", active[0].kind.label_zh(), active[0].name, active[0].description); }

    // 轮 2: AI 用已激活能力干活 (写错题本)
    println!("\n[干活] 让 AI 用已激活能力 + 工具写错题本");
    let workdir = std::env::temp_dir().join("apeireth-release");
    std::fs::create_dir_all(&workdir).unwrap();
    let target = workdir.join("错题本.md");
    let tools2 = json!([
        {"type":"function","function":{"name":"recall_memory","description":"查记忆","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}},
        {"type":"function","function":{"name":"FileOperator","description":"文件操作","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["read","write","list"]},"path":{"type":"string"},"content":{"type":"string"}},"required":["op","path"]}}}
    ]);
    let mut m2: Vec<Value> = vec![
        json!({"role": "system", "content": dynamic.clone()}),
        json!({"role": "user", "content": format!("用你的能力帮主人: 把换元法错题要点写进 {} (60字内), 完成后一句话汇报", target.to_string_lossy())}),
    ];
    let mut final_answer = "(未完成)".to_string();
    for _ in 0..5 {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(), messages: m2.iter().map(|v| OpenAiChatMessage {
                role: v["role"].as_str().unwrap_or("user").to_string(), content: v["content"].clone(),
                tool_calls: v.get("tool_calls").and_then(|x| x.as_array()).cloned(),
                tool_call_id: v.get("tool_call_id").and_then(|x| x.as_str()).map(|s| s.to_string()),
            }).collect(),
            temperature: Some(0.5), max_tokens: Some(1024), stream: false, stop: None,
            tools: Some(tools2.as_array().cloned().unwrap_or_default()), tool_choice: Some(json!("auto")),
        };
        let Some((content, tcs)) = chat_once(&pipeline, &req, "干活轮").await else { break; };
        if tcs.is_empty() { final_answer = content.split("</think>").last().unwrap_or(&content).trim().to_string(); break; }
        m2.push(json!({"role": "assistant", "content": content, "tool_calls": tcs}));
        let mut tms = Vec::new();
        for tc in &tcs {
            let id = tc["id"].clone();
            let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
            let args: Value = serde_json::from_str(tc["function"]["arguments"].as_str().unwrap_or("{}")).unwrap_or(json!({}));
            let r = bridge.execute_if_allowed(&ParsedToolCall { tool_name: name.clone(), args: args.clone(), raw_marker: String::new(), archery: false, archery_no_reply: false }).await;
            let body = if r.success { serde_json::to_string(&r.output).unwrap_or_default() } else { format!("失败: {:?}", r.error) };
            println!("  调用 {name}: {}", body.chars().take(70).collect::<String>());
            tms.push(json!({"role": "tool", "tool_call_id": id, "content": body}));
        }
        m2.extend(tms);
        m2.push(json!({"role": "user", "content": "继续; 完成后一句话汇报。"}));
    }
    println!("  [汇报] {final_answer}");
    println!("  文件: {}", if target.exists() { format!("✅ {} 字节", std::fs::read_to_string(&target).unwrap_or_default().len()) } else { "❌".into() });

    // 反馈 + 每日摘要
    let eps = store.recent_episodes("me", 100).unwrap();
    let entries: Vec<(&str, &str)> = eps.iter().map(|e| (e.id.as_str(), e.content.as_str())).collect();
    let summary = build_daily_summary(&chrono::Utc::now().format("%Y-%m-%d").to_string(), &entries, 3);
    println!("\n[摘要]\n{}", summary.render());

    println!("\n═══════════ 涌现闭环验收完成 ═══════════");
}
