//! oracle_acceptance — Oracle 套件真 LLM 验收: AI 自然融合基地能力.
//!
//! 任务: 主人周五高数期中, 换元法常错。观察 AI 是否**自然串联**:
//! 查记忆(recall_memory) → 沙盘推演(simulate 两种复习策略) → 可证伪预测(forecast) → 沉淀(save_memory).
//!
//! 这是「灵性」测试: 不给出具体步骤, 看 AI 是否自发把基地各功能融成一条决策链.

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::oracle::ForecastRegistry;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_runtime::parser::ParsedToolCall;
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
        .map_err(|e| format!("读 key 失败: {e}"))
}

async fn chat_once(
    pipeline: &Arc<Pipeline>,
    req: &OpenAiChatRequest,
    label: &str,
) -> Option<(String, Vec<Value>)> {
    for attempt in 0..5 {
        match dispatch(
            pipeline,
            ProtocolKind::OpenAiChat,
            openai_chat_to_normalized(req),
        )
        .await
        {
            Ok(r) => {
                let chat = openai_chat_from_normalized(&r);
                return Some((
                    chat.choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_default(),
                    chat.choices
                        .first()
                        .map(|c| c.message.tool_calls.clone())
                        .unwrap_or_default()
                        .unwrap_or_default(),
                ));
            }
            Err(e) => {
                eprintln!("  [管线] {label} 第{}次失败: {e}, 10s 后重试", attempt + 1);
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
    None
}

#[tokio::main]
async fn main() {
    let key = load_key().expect("需要 MiniMax key");
    let pipeline = Arc::new(build_pipeline(BASE_URL.to_string(), Some(key.clone())).unwrap());
    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    let reg = ForecastRegistry::new(Arc::clone(&store), "me");
    let bridge = Arc::new(ToolBridge::new(Arc::clone(&store)));
    println!("═══════════ oracle_acceptance — AI 自然融合验收 ═══════════\n");

    // 种子记忆
    for (i, c) in [
        "高数: 换元法常错, 根号里带平方的",
        "高数: 换元后忘记换 dx",
        "周五高数期中考试",
    ]
    .iter()
    .enumerate()
    {
        store
            .put_episode(&CoreEpisode {
                id: format!("mem-{i}"),
                timestamp: 1 + i as i64,
                role: "assistant".into(),
                content: c.to_string(),
                session_id: "me".into(),
            })
            .unwrap();
    }

    // 工具: 记忆 + oracle (simulate/forecast) + 沉淀
    let tools = json!([
        {"type":"function","function":{"name":"recall_memory","description":"查主人长期记忆","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}},
        {"type":"function","function":{"name":"simulate","description":"沙盘推演: entities 给初始世界状态 {实体名: {属性: 数值}} (实体名直接用名字, 如 主人, 不要加编号前缀); events 是事件序列, 每条 \"实体名.属性±增量\" 表示增减, \"实体名.属性=数值\" 表示赋值; 例: \"主人.复习进度+0.3\", \"主人.信心=0.4\", \"错题本A.收录数+5\"; 返回每步状态快照+final","parameters":{"type":"object","properties":{"entities":{"type":"object"},"events":{"type":"array","items":{"type":"string"}},"note":{"type":"string"}},"required":["entities","events"]}}},
        {"type":"function","function":{"name":"forecast","description":"登记可证伪预测: statement(断言)+probability(0..1)+deadline_hours(期限小时)","parameters":{"type":"object","properties":{"statement":{"type":"string"},"probability":{"type":"number"},"deadline_hours":{"type":"number"}},"required":["statement","probability","deadline_hours"]}}},
        {"type":"function","function":{"name":"save_memory","description":"把值得记住的写入记忆","parameters":{"type":"object","properties":{"content":{"type":"string"}},"required":["content"]}}}
    ]);
    let sys = format!(
        "你是阿佩瑞斯, 诚实、有记忆、有推演能力的伙伴。基地给了你: 记忆检索、沙盘推演(simulate: 把场景建模成实体属性, 用事件推演各策略的走向)、可证伪预测(forecast: 给断言+概率+期限)、记忆沉淀。\n{}",
        apeireth_companion::actions::CapabilityCatalog::describe()
    );
    let mut messages: Vec<Value> = vec![
        json!({"role": "system", "content": sys}),
        json!({"role": "user", "content": "主人周五要考高数期中, 他换元法常错(根号带平方、忘换 dx)。请:\n1. 查一下他的记忆\n2. 用沙盘推演比较两种复习策略的走向(建模: 复习进度/错题率/信心等属性, 推演 3 步)\n3. 给出一个可证伪预测(带概率和期限)\n4. 把值得记住的沉淀进记忆\n5. 最后用一段话说明你的推演逻辑。\n你可以自由决定怎么用这些能力。"}),
    ];
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut final_answer = "(未完成)".to_string();
    for _ in 0..8 {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: messages
                .iter()
                .map(|v| OpenAiChatMessage {
                    role: v["role"].as_str().unwrap_or("user").to_string(),
                    content: v["content"].clone(),
                    tool_calls: v.get("tool_calls").and_then(|x| x.as_array()).cloned(),
                    tool_call_id: v
                        .get("tool_call_id")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string()),
                })
                .collect(),
            temperature: Some(0.5),
            max_tokens: Some(1500),
            stream: false,
            stop: None,
            tools: Some(tools.as_array().cloned().unwrap_or_default()),
            tool_choice: Some(json!("auto")),
        };
        let Some((content, tcs)) = chat_once(&pipeline, &req, "融合轮").await else {
            break;
        };
        if tcs.is_empty() {
            final_answer = content
                .split("</think>")
                .last()
                .unwrap_or(&content)
                .trim()
                .to_string();
            break;
        }
        messages.push(json!({"role": "assistant", "content": content, "tool_calls": tcs}));
        let mut tms = Vec::new();
        for tc in &tcs {
            let id = tc["id"].clone();
            let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
            let args: Value =
                serde_json::from_str(tc["function"]["arguments"].as_str().unwrap_or("{}"))
                    .unwrap_or(json!({}));
            let r = bridge
                .execute_if_allowed(&ParsedToolCall {
                    tool_name: name.clone(),
                    args: args.clone(),
                    raw_marker: String::new(),
                    archery: false,
                    archery_no_reply: false,
                })
                .await;
            let body = if r.success {
                serde_json::to_string(&r.output).unwrap_or_default()
            } else {
                format!("失败: {:?}", r.error)
            };
            used.insert(name.clone());
            println!(
                "  他调用 {name}: {}",
                body.chars().take(110).collect::<String>()
            );
            tms.push(json!({"role": "tool", "tool_call_id": id, "content": body}));
        }
        messages.extend(tms);
        messages.push(json!({"role": "user", "content": "继续; 全部完成后再总结推演逻辑。"}));
    }

    // 融合度评估
    println!("\n=== 融合度 ===");
    let memory_used = used.contains("recall_memory");
    let simulate_used = used.contains("simulate");
    let forecast_used = used.contains("forecast");
    let save_used = used.contains("save_memory");
    println!(
        "查记忆 {} · 沙盘推演 {} · 可证伪预测 {} · 记忆沉淀 {}",
        memory_used, simulate_used, forecast_used, save_used
    );
    let fusion = memory_used && simulate_used && forecast_used;
    println!(
        "自然融合(查→推→测): {}",
        if fusion {
            "✅ 全链串联"
        } else {
            "⚠️ 部分串联"
        }
    );
    let (n, _, cal) = reg.calibration().unwrap();
    println!("预测登记: {} 条 (待对照 resolve) — {}", n, cal);
    println!("\n[他的总结]\n{}", final_answer);
    println!("\n═══════════ 验收完成 ═══════════");
}
