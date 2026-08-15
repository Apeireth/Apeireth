//! multi_turn_agent — §6.3 真 LLM 多轮 function calling 循环 + 续行快照断点续传.
//!
//! 流程:
//!   1. 每轮 LLM+工具循环前, 把 messages(LLM 上下文) + 轮次 原子保存到 ContinuationStore
//!   2. 支持崩溃演示: `APEIRETH_CRASH_AFTER_TURNS=N` → 第 N 轮保存后退出 (模拟进程崩溃)
//!   3. 支持断点续传: `APEIRETH_RESUME_SNAPSHOT=<id>` → 启动时 consume 快照, 从断点继续
//!   4. 任务: recall_memory → FileOperator 写错题本 (走正式管线 + 门禁)
//!
//! 跑法 (真 MiniMax, 需 apikey):
//!   第一次 (崩溃演示): cargo run -p apeireth-companion --example multi_turn_agent -- --crash-after=1
//!   第二次 (断点续传):  cargo run -p apeireth-companion --example multi_turn_agent -- --resume=multiturn
//!
//! 环境变量: APEIRETH_API_KEY (或读 apikey-ultra.txt)

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::continuation::{ContinuationSnapshot, ContinuationStore};
use apeireth_companion::packs::PermissionPack;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_runtime::parser::ParsedToolCall;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";
const SNAP_ID: &str = "multiturn";

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

async fn chat_once(
    pipeline: &Arc<Pipeline>,
    req: &OpenAiChatRequest,
    label: &str,
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
                eprintln!("  [管线] {label} 第{}次失败: {e}, 8s 后重试", attempt + 1);
                tokio::time::sleep(Duration::from_secs(8)).await;
            }
        }
    }
    None
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    // 支持 --x=value 与 --x value 两种形式
    let flag_val = |flag: &str| -> Option<String> {
        args.iter()
            .find_map(|a| a.strip_prefix(&format!("{flag}=")).map(|s| s.to_string()))
            .or_else(|| {
                args.iter()
                    .position(|a| a == flag)
                    .and_then(|i| args.get(i + 1))
                    .cloned()
            })
    };
    let crash_after: Option<u64> = flag_val("--crash-after").and_then(|v| v.parse().ok());
    let resume: Option<String> = flag_val("--resume");

    let key = load_key().expect("需要 MiniMax key");
    let store_dir = std::env::temp_dir().join("apeireth-continuation-demo");
    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let workdir = std::env::temp_dir().join("apeireth-multiturn");
    std::fs::create_dir_all(&workdir).unwrap();
    let target = workdir.join("错题本.md");

    let bridge = Arc::new(ToolBridge::new(Arc::clone(&store)));
    bridge.packs.grant(
        PermissionPack::timed("多轮包", vec!["FileOperator".to_string()], 24, Some(10))
            .with_paths(vec![workdir.to_string_lossy().to_string()]),
    );
    // 记忆种子
    for (id, content) in [
        ("m1", "高数: 不定积分换元法老出错, 根号里带平方的"),
        ("m2", "高数: 换元后忘记把 dx 也换掉"),
    ] {
        store
            .put_episode(&apeireth_memory::CoreEpisode {
                id: id.into(),
                timestamp: 1,
                role: "assistant".into(),
                content: content.into(),
                session_id: "me".into(),
            })
            .unwrap();
    }

    let pipeline = Arc::new(build_pipeline(BASE_URL.to_string(), Some(key)).expect("pipeline"));
    let tools = json!([
        {"type":"function","function":{"name":"recall_memory","description":"查长期记忆","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}},
        {"type":"function","function":{"name":"FileOperator","description":"文件操作","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["read","write","list"]},"path":{"type":"string"},"content":{"type":"string"}},"required":["op","path"]}}}
    ]);

    let snap_store = ContinuationStore::new(&store_dir);

    // ---- 断点续传: 恢复上次快照 (模拟崩溃后重启) ----
    let mut messages: Vec<Value>;
    let mut turn: u64;
    let mut fresh = true;
    if let Some(id) = resume {
        match snap_store.consume(&id) {
            Ok(s) => {
                println!("[续传] 恢复快照 {id}: turn={}, messages={} 条", s.turn, s.messages.len());
                messages = s.messages;
                turn = s.turn;
                fresh = false;
            }
            Err(e) => {
                println!("[续传] 无快照可恢复 ({e}), 全新开始");
                messages = Vec::new();
                turn = 0;
            }
        }
    } else {
        messages = Vec::new();
        turn = 0;
    }

    if fresh {
        messages = vec![
            json!({"role":"user","content":format!("任务: 主人高数「不定积分换元法」常出错。请:\n1. 用 recall_memory 查他这方面的记忆\n2. 基于记忆, 用 FileOperator 把错题笔记写到 {} (100 字内)\n3. 完成后一句话汇报", target.to_string_lossy())}),
        ];
        turn = 1;
    }
    // 稳定前缀: system prompt 逐字节不变 (Prompt Cache 最大化命中)
    let sys = format!(
        "你是阿佩瑞斯 (Apeireth), 一个诚实、有记忆的伙伴, 住在 Apeireth 基地。{}",
        apeireth_companion::actions::CapabilityCatalog::describe()
    );

    let mut tool_count = 0u32;
    let mut final_answer = String::from("(未完成)");
    for round in 1..=8u64 {
        // 每轮循环前: 原子保存续行快照 (断点 = 此处)
        let snap = ContinuationSnapshot {
            id: SNAP_ID.into(),
            session_id: "me".into(),
            messages: messages.clone(),
            pending_tool_call: None,
            saved_at_ms: chrono::Utc::now().timestamp_millis(),
            turn,
        };
        snap_store.save(&snap).expect("保存快照");
        println!("[快照] turn={turn} 已保存 (messages {} 条)", messages.len());

        // 崩溃演示: 本轮结束后退出 (模拟进程崩溃)
        if let Some(c) = crash_after {
            if turn == c {
                println!("[崩溃演示] 第 {turn} 轮后保存快照并退出 (模拟崩溃) — 下次 --resume={SNAP_ID} 续传");
                println!("[快照文件] {}", store_dir.join(format!("{SNAP_ID}.json")).display());
                return;
            }
        }

        // 动态字段单一注入点 (时间等): 经 build_messages 插到最新 user 前, 稳定前缀不变
        let dynamic_note = chrono::Utc::now().format("%Y-%m-%d %H:%M (%A)").to_string();
        let req_msgs = apeireth_companion::build_messages(&sys, &messages, Some(&dynamic_note));
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: req_msgs
                .iter()
                .map(|v| OpenAiChatMessage {
                    role: v["role"].as_str().unwrap_or("user").to_string(),
                    content: v["content"].clone(),
                    tool_calls: v.get("tool_calls").and_then(|x| x.as_array()).cloned(),
                    tool_call_id: v.get("tool_call_id").and_then(|x| x.as_str()).map(|s| s.to_string()),
                })
                .collect(),
            temperature: Some(0.5),
            max_tokens: Some(1024),
            stream: false,
            stop: None,
            tools: Some(tools.as_array().cloned().unwrap_or_default()),
            tool_choice: Some(json!("auto")),
        };
        let Some((content, tcs)) = chat_once(&pipeline, &req, &format!("第{turn}轮")).await else {
            final_answer = "(被限流)".to_string();
            break;
        };
        if tcs.is_empty() {
            let answer = content.split("</think>").last().unwrap_or(&content).trim().to_string();
            if answer.is_empty() {
                messages.push(json!({"role":"user","content":"你刚才没有输出, 请继续执行任务。"}));
                turn += 1;
                continue;
            }
            final_answer = answer;
            break;
        }
        messages.push(json!({"role": "assistant", "content": content, "tool_calls": tcs}));
        let mut tool_msgs: Vec<Value> = Vec::new();
        for tc in &tcs {
            tool_count += 1;
            let id = tc["id"].clone();
            let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
            let args: Value =
                serde_json::from_str(tc["function"]["arguments"].as_str().unwrap_or("{}")).unwrap_or(json!({}));
            let risk = ToolBridge::tool_risk(&name);
            let pack_hit = bridge.packs.check_and_consume(&name, chrono::Utc::now().timestamp_millis());
            println!("  他调用 {name} (risk={risk:?}, 权限包={pack_hit})");
            let r = bridge
                .execute_if_allowed(&ParsedToolCall {
                    tool_name: name.clone(),
                    args: args.clone(),
                    raw_marker: String::new(),
                    archery: false,
                    archery_no_reply: false,
                })
                .await;
            let body_str = if r.success {
                serde_json::to_string(&r.output).unwrap_or_default()
            } else {
                format!("失败: {:?}", r.error)
            };
            println!("  结果: {}", body_str.chars().take(100).collect::<String>());
            tool_msgs.push(json!({"role":"tool","tool_call_id":id,"content":body_str}));
        }
        messages.extend(tool_msgs);
        messages.push(json!({"role":"user","content":"继续; 任务全部完成后再回复一句话汇报。"}));
        turn += 1;
    }

    let file_ok = target.exists();
    println!("\n=== 结果 ===");
    println!("[他的汇报] {final_answer}");
    println!("文件: {} ({} 字节, 工具调用 {tool_count} 次)", if file_ok { "✅" } else { "❌" }, std::fs::read_to_string(&target).unwrap_or_default().len());
    println!("快照剩余: {:?}", snap_store.list());
    if file_ok {
        for line in std::fs::read_to_string(&target).unwrap_or_default().lines().take(8) {
            println!("  | {line}");
        }
    }
}
