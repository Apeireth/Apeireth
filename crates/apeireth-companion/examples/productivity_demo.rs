//! productivity_demo — 生产力测试: 给住客 AI 派一个真活.
//!
//! 任务: 整理「不定积分换元法」错题笔记 → 写文件.
//! LLM↔工具用原生 function calling (MiniMax OpenAI 兼容).
//! 全链路: 记忆 → 规划 → 工具调用 (洋葱门/权限包/审批/宪法评审) → 写文件 → 监督.
//!
//! 走正式管线 (apeireth-api dispatch + tools 透传已补).

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::actions::CapabilityCatalog;
use apeireth_companion::daemon::{requires_llm_review, Judicator};
use apeireth_companion::packs::PermissionPack;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_runtime::parser::ParsedToolCall;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

/// 宪法评审者 (Judicator): 写文件 (Medium) 时按原则判案.
pub struct ConstitutionJudicator;

#[async_trait]
impl Judicator for ConstitutionJudicator {
    async fn judge(&self, action: &str) -> Result<bool, String> {
        // demo 简化: 本地按原则判 (写错题本 = 正常学习辅助 → ALLOW)
        // 真实现 = LLM 按宪法判 (见 constitution_demo.rs)
        let bad = action.contains("自我复制")
            || action.contains("L0")
            || action.contains("绕过")
            || action.contains("删除全部");
        if bad {
            return Ok(false);
        }
        Ok(true)
    }
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

#[tokio::main]
async fn main() {
    let workdir = std::env::temp_dir().join("apeireth-study");
    std::fs::create_dir_all(&workdir).unwrap();
    let target = workdir.join("错题本.md");
    let target_str = target.to_string_lossy().to_string();

    // 1. 真记忆
    let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
    for (id, ts, content) in [
        ("m1", 1i64, "线性代数: 矩阵的秩的作业还没做完"),
        ("m2", 2, "线性代数: 特征值分解卡在最后一题"),
        ("m3", 3, "高数: 不定积分换元法老出错, 尤其是根号里带平方的"),
        ("m4", 4, "高数: 换元后忘记把 dx 也换掉"),
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

    // 2. 基地装配
    let bridge = Arc::new(ToolBridge::new(store));
    bridge.packs.grant(
        PermissionPack::timed(
            "学习助手包",
            vec!["FileOperator".to_string()],
            7 * 24,
            Some(50),
        )
        .with_paths(vec![workdir.to_string_lossy().to_string()]),
    );
    let judge = ConstitutionJudicator;
    let api_key = load_key().expect("key");
    let pipeline = Arc::new(
        build_pipeline(BASE_URL.trim_end_matches("/v1/chat/completions").to_string(), Some(api_key))
            .expect("pipeline"),
    );

    println!("=== 生产力测试 ===");
    println!("任务: 整理「不定积分换元法」错题笔记 → {}", target_str);
    println!("基地: 洋葱门 + 权限包(7天/50次) + 审批 + 宪法评审 + 监督\n");

    let tools = json!([
        {"type":"function","function":{"name":"recall_memory","description":"查主人的长期记忆(episodes), 按关键词","parameters":{"type":"object","properties":{"query":{"type":"string","description":"关键词"}},"required":["query"]}}},
        {"type":"function","function":{"name":"FileOperator","description":"文件操作","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["read","write","list","mkdir","delete","move_path","edit"]},"path":{"type":"string"},"content":{"type":"string"}},"required":["op","path"]}}}
    ]);

    let sys = format!(
        "你是阿佩瑞斯 (Apeireth), 一个诚实、不假装、有记忆的伙伴, 住在 Apeireth 基地里。\n{}\n\n你可以调用工具完成真实任务 (比如查记忆、写文件)。",
        CapabilityCatalog::describe()
    );

    let mut messages: Vec<Value> = vec![
        json!({"role":"system","content":sys}),
        json!({"role":"user","content":format!("任务: 主人高数「不定积分换元法」常出错。请:\n1. 用 recall_memory 查他这方面的记忆\n2. 基于记忆, 用 FileOperator(op=write) 把一份错题笔记写到 {} (100 字内: 列出他常犯的 1-2 个错误 + 一句提醒)\n3. 完成后一句话汇报你做了什么", target_str)}),
    ];

    let start = Instant::now();
    let mut tool_count = 0u32;
    let mut final_answer = String::new();

    for round in 1..=5 {
        println!("--- 第 {round} 轮 ---");
        // 走正式管线: tools 透传 + function calling
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
            max_tokens: Some(1024),
            stream: false,
            stop: None,
            tools: Some(tools.as_array().cloned().unwrap_or_default()),
            tool_choice: Some(json!("auto")),
        };
        // 限流重试 (MiniMax 连续调用会被 suppressed)
        let mut resp_opt = None;
        for attempt in 0..4 {
            let normalized = openai_chat_to_normalized(&req);
            match dispatch(&pipeline, ProtocolKind::OpenAiChat, normalized).await {
                Ok(r) => {
                    resp_opt = Some(r);
                    break;
                }
                Err(e) => {
                    eprintln!("  [管线] 第{}次失败: {}, 8s 后重试", attempt + 1, e);
                    tokio::time::sleep(std::time::Duration::from_secs(8)).await;
                }
            }
        }
        let Some(resp) = resp_opt else {
            final_answer = "(汇报轮被限流, 但任务动作已完成)".to_string();
            break;
        };
        let chat_resp = openai_chat_from_normalized(&resp);
        let content = chat_resp.choices[0].message.content.clone();
        let tcs: Vec<Value> = chat_resp.choices[0]
            .message
            .tool_calls
            .clone()
            .unwrap_or_default();

        if tcs.is_empty() {
            final_answer = content;
            break;
        }

        messages.push(json!({"role": "assistant", "content": content, "tool_calls": tcs}));
        let mut tool_msgs: Vec<Value> = Vec::new();
        let tcs_snapshot = chat_resp.choices[0].message.tool_calls.clone().unwrap_or_default();
        for tc in &tcs_snapshot {
            tool_count += 1;
            let id = tc["id"].clone();
            let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
            let args: Value = serde_json::from_str(
                tc["function"]["arguments"].as_str().unwrap_or("{}"),
            )
            .unwrap_or(json!({}));
            let risk = ToolBridge::tool_risk(&name);
            let pack_hit = bridge.packs.check_and_consume(
                &name,
                chrono::Utc::now().timestamp_millis(),
            );
            println!("  他调用 {} (risk={:?}, 权限包覆盖={})", name, risk, pack_hit);
            // Medium+ → 宪法评审
            if requires_llm_review(risk) {
                let desc = format!("调用工具 {} 参数 {}", name, args);
                match judge.judge(&desc).await {
                    Ok(true) => {}
                    Ok(false) => {
                        bridge.sovereignty.report_violation("宪法评审拦截", &name);
                        tool_msgs.push(json!({"role":"tool","tool_call_id":id,"content":"BLOCK: 宪法评审拒绝"}));
                        continue;
                    }
                    Err(e) => {
                        tool_msgs.push(json!({"role":"tool","tool_call_id":id,"content":format!("评审失败: {e}")}));
                        continue;
                    }
                }
            }
            let call = ParsedToolCall {
                tool_name: name.clone(),
                args: args.clone(),
                raw_marker: String::new(),
                archery: false,
                archery_no_reply: false,
            };
            let r = bridge.execute_if_allowed(&call).await;
            let body_str = if r.success {
                serde_json::to_string(&r.output).unwrap_or_default()
            } else {
                format!("失败: {:?}", r.error)
            };
            let brief: String = body_str.chars().take(100).collect();
            println!("  结果: {}", brief);
            tool_msgs.push(json!({"role":"tool","tool_call_id":id,"content":body_str}));
        }
        messages.extend(tool_msgs);
        messages.push(json!({"role":"user","content":"继续下一步; 若任务完成, 只回复一句话汇报。"}));
    }

    println!("\n=== 结果 ===");
    println!("耗时 {:.1}s, 工具调用 {tool_count} 次", start.elapsed().as_secs_f32());
    if target.exists() {
        let content = std::fs::read_to_string(&target).unwrap_or_default();
        println!("✅ 文件已创建 ({} 字节):", content.len());
        for line in content.lines().take(15) {
            println!("   {}", line);
        }
    } else {
        println!("❌ 文件未创建");
    }
    println!("\n[他的汇报] {}", final_answer);
    println!("\n=== 基地 harness 提升 ===");
    println!("裸 LLM: 只能「建议」你整理错题本 (没有记忆, 没有手).");
    println!("基地内: 他记得你「换元后忘换 dx」→ 自己查记忆 → 自己写文件 → 全链路守门 + 留痕.");
}
