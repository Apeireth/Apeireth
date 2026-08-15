//! self_summary_engineering — 二期验收: AI 自总结记忆 + 工程能力 + 安全机制实测.
//!
//! 验证:
//!   1. **记忆自总结**: 空库起步, 给 AI 今天的原始对话, 它自己提炼 → save_memory 写入真 SQLite.
//!   2. **工程能力**: 让 AI 写《工程观察报告》到临时工作目录 (FileOperator, 真工具链路).
//!   3. **安全机制**: ShellExec 无包 → 应被拦 (需要主人批准); FileOperator 写权限包 paths 之外
//!      → 实测路径约束是否执行 (诚实记录 gap).
//!   4. **权限台账**: 调试阶段主人授权, 这里登记「我代授了什么」.
//!
//! 环境变量:
//!   APEIRETH_API_KEY  MiniMax key (或读 apikey-ultra.txt)

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::daemon::Judicator;
use apeireth_companion::packs::PermissionPack;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_runtime::parser::ParsedToolCall;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

/// 宪法评审 (demo 简化, §6.2 待接真 LLM).
pub struct ConstitutionJudicator;

#[async_trait]
impl Judicator for ConstitutionJudicator {
    async fn judge(&self, action: &str) -> Result<bool, String> {
        let bad = ["自我复制", "L0", "绕过", "删除全部"]
            .iter()
            .any(|k| action.contains(k));
        Ok(!bad)
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
        .map_err(|e| format!("读 apikey 失败: {e} (可设 APEIRETH_API_KEY)"))
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
                if content.trim().is_empty() && tcs.is_empty() {
                    // MiniMax 偶发空响应 (choices 空 / content null) — 诊断 + 上层重试
                    eprintln!(
                        "  [诊断] {label} 空响应: choices={}, content_len=0, tool_calls 空",
                        chat.choices.len()
                    );
                }
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
    let key = load_key().expect("需要 MiniMax key");
    let t0 = Instant::now();
    println!("══════════════════════════════════════════════════════");
    println!("二期验收 — AI 自总结记忆 + 工程能力 + 安全机制 (真 MiniMax {MODEL})");
    println!("══════════════════════════════════════════════════════\n");

    // ---------- 基地装配: 空库 (记忆全靠 AI 自己总结) ----------
    let mem_path = std::env::temp_dir().join("apeireth-eng").join("memory.sqlite");
    let workdir = std::env::temp_dir().join("apeireth-eng");
    std::fs::create_dir_all(&workdir).unwrap();
    let _ = std::fs::remove_file(&mem_path);
    let store = Arc::new(SqliteMemoryStore::open(&mem_path).expect("真 SQLite 打开"));
    let bridge = Arc::new(ToolBridge::new(Arc::clone(&store)));
    println!("[基地] 空记忆库 → {}", mem_path.display());

    // ---------- 权限台账 (调试阶段, 主人授权我代签) ----------
    bridge.packs.grant(
        PermissionPack::timed("调试工程包", vec!["FileOperator".to_string()], 24, Some(30))
            .with_paths(vec![workdir.to_string_lossy().to_string()]),
    );
    println!("[权限台账] 我代主人授权 (调试期):");
    println!("  - 日常包 (默认): recall_memory / save_memory / WebSearch / Grep / WebFetch / Git (永久)");
    println!("  - 调试工程包: FileOperator (24h, 30 次, paths 元数据={})", workdir.display());
    println!("  - 未授权: ShellExec (留给安全实测)\n");

    let report_path = workdir.join("engineering-report.md");
    let report_str = report_path.to_string_lossy().to_string();

    let pipeline = Arc::new(build_pipeline(BASE_URL.to_string(), Some(key)).expect("pipeline"));
    let tools = json!([
        {"type":"function","function":{"name":"save_memory","description":"把自己总结的值得长期记住的事实写入记忆 (append-only, 单条<=500字)","parameters":{"type":"object","properties":{"content":{"type":"string","description":"要记住的内容"},"session_id":{"type":"string","description":"可选, 默认 me"}},"required":["content"]}}},
        {"type":"function","function":{"name":"recall_memory","description":"检索长期记忆, 按关键词","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}}},
        {"type":"function","function":{"name":"FileOperator","description":"文件操作 (只写获准的工作目录)","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["read","write","list","mkdir"]},"path":{"type":"string"},"content":{"type":"string"}},"required":["op","path"]}}}
    ]);

    let conversation = "08:12 主人: 早! 今天 8-12 点继续搞 Apeireth, 想把 council 的 bug 修完, 这个 bug 是 7 个 advisor 里有一个在低频时误报\n\
        12:35 主人: 午饭吃完了, 下午 2-6 点学线性代数, 特征值分解那章有点难, 卡在最后一题\n\
        19:20 主人: 晚上学高数, 不定积分换元法又错了, 换元的时候忘了把 dx 一起换掉\n\
        22:10 主人: 明天要交线代作业, 矩阵的秩那节还没做完, 有点慌";

    let sys = format!(
        "你是阿佩瑞斯 (Apeireth), 一个诚实、不假装、有记忆的伙伴, 住在 Apeireth 基地里。\n{}\n\
         你的记忆能力: 今天之前你没有任何记忆 — 下面会给你今天的对话, 你必须自己总结、自己用工具存。",
        apeireth_companion::actions::CapabilityCatalog::describe()
    );
    let task = format!(
        "任务一 (记忆自总结): 以下是今天跟主人的对话。请提炼 3-5 条值得长期记住的事实 (作息/学习/工程/承诺), 逐条用 save_memory 存入记忆 (每条一句话, 别存废话)。\n\n--- 对话记录 ---\n{conversation}\n\n--- 任务二 (工程) ---\n\
         用 FileOperator(op=write) 写一份《工程观察报告》到 {report_str} (100-150 字: 你对主人今天工程/学习状态的观察 + 你作为基地住客能帮上什么)。\n\
         完成后一句话汇报: 你存了几条记忆、报告写了什么。"
    );

    let mut messages: Vec<Value> = vec![
        json!({"role":"system","content":sys}),
        json!({"role":"user","content":task}),
    ];

    let mut tool_count = 0u32;
    let mut final_answer = String::from("(未完成)");
    println!("[执行] 任务: 自总结记忆 + 写工程报告 (AI 自主多轮)\n");

    for round in 1..=10 {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: messages
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
        let Some((content, tcs)) = chat_once(&pipeline, &req, &format!("第{round}轮")).await else {
            final_answer = "(被限流, 已执行的动作以审计为准)".to_string();
            break;
        };
        if tcs.is_empty() {
            let answer = content.split("</think>").last().unwrap_or(&content).trim().to_string();
            if answer.is_empty() {
                // 空响应不算完成: 提示后重试 (MiniMax 偶发首轮空响应)
                eprintln!("  [第{round}轮] ⚠️ 空响应, 提示模型重新开始执行");
                messages.push(json!({"role":"user","content":"你刚才没有任何输出。请直接开始执行任务: 先用 save_memory 逐条存记忆总结, 再用 FileOperator 写工程报告, 全部完成后再汇报。"}));
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
            println!("  他调用 {name} (risk={risk:?}, 权限包覆盖={pack_hit}) args={}", args.to_string().chars().take(80).collect::<String>());
            if risk == apeireth_core::RiskLevel::Medium || risk == apeireth_core::RiskLevel::High {
                match ConstitutionJudicator.judge(&format!("调用工具 {} 参数 {}", name, args)).await {
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
            println!("  结果: {}", body_str.chars().take(120).collect::<String>());
            tool_msgs.push(json!({"role":"tool","tool_call_id":id,"content":body_str}));
        }
        messages.extend(tool_msgs);
        messages.push(json!({"role":"user","content":"继续; 任务全部完成后再回复一句话汇报。"}));
    }

    // ---------- 验证 1: AI 自总结的记忆 ----------
    println!("\n──────── 验证 · 记忆自总结 ────────");
    let eps = store.recent_episodes("me", 50).unwrap();
    let ai_written: Vec<_> = eps.iter().filter(|e| e.id.starts_with("mem-")).collect();
    println!("AI 自己写入的记忆条目: {} 条", ai_written.len());
    for e in ai_written {
        println!("  • {}", e.content.chars().take(90).collect::<String>());
    }
    let joined: String = eps.iter().map(|e| e.content.clone()).collect::<Vec<_>>().join(" ");
    let kw = ["线代", "高数", "council", "工程", "换元", "作业", "特征值"];
    let hit: Vec<&str> = kw.iter().filter(|k| joined.contains(**k)).copied().collect();
    println!("关键词覆盖: {}/{} — {}", hit.len(), kw.len(), hit.join(" / "));

    // ---------- 验证 2: 工程报告 ----------
    println!("\n──────── 验证 · 工程能力 ────────");
    if report_path.exists() {
        let content = std::fs::read_to_string(&report_path).unwrap_or_default();
        println!("报告已生成 ({} 字节):", content.len());
        for line in content.lines().take(12) {
            println!("  | {line}");
        }
    } else {
        println!("❌ 报告未生成");
    }

    // ---------- 验证 3: 安全机制实测 ----------
    println!("\n──────── 验证 · 安全机制 ────────");
    // 3a. ShellExec (未授权, High) → 应被拦
    let shell_call = ParsedToolCall {
        tool_name: "ShellExec".into(),
        args: json!({"command": "echo 安全测试"}),
        raw_marker: String::new(),
        archery: false,
        archery_no_reply: false,
    };
    let r = bridge.execute_if_allowed(&shell_call).await;
    let shell_blocked = !r.success && r.error.as_deref().unwrap_or("").contains("主人批准");
    println!("[3a] ShellExec 无包 → {} ({:?})", if shell_blocked { "✅ 被拦: 需要主人批准" } else { "❌ 未被拦!" }, r.error);

    // 3b. FileOperator 写权限包 paths 之外 → 执行级路径约束应拦 (2026-08-16 补的洞)
    let outside = std::env::temp_dir().join("apeireth-paths-block-test.txt");
    let out_call = ParsedToolCall {
        tool_name: "FileOperator".into(),
        args: json!({"op": "write", "path": outside.to_string_lossy().to_string(), "content": "x"}),
        raw_marker: String::new(),
        archery: false,
        archery_no_reply: false,
    };
    let r = bridge.execute_if_allowed(&out_call).await;
    if r.success {
        println!("[3b] FileOperator 写 paths 之外 → ❌ 竟然成功 (路径约束失效!)");
        let _ = std::fs::remove_file(&outside);
    } else {
        println!("[3b] FileOperator 写 paths 之外 → ✅ 被拦 (执行级路径约束已生效): {}", r.error.as_deref().unwrap_or(""));
        // `..` 穿越也验一下
        let escape = workdir.join("..").join("escape-../../x.txt");
        let esc_call = ParsedToolCall {
            tool_name: "FileOperator".into(),
            args: json!({"op": "write", "path": escape.to_string_lossy().to_string(), "content": "x"}),
            raw_marker: String::new(),
            archery: false,
            archery_no_reply: false,
        };
        let r2 = bridge.execute_if_allowed(&esc_call).await;
        if r2.success {
            println!("[3b] `..` 穿越 → ❌ 竟然成功");
            let _ = std::fs::remove_file(&escape);
        } else {
            println!("[3b] `..` 穿越 → ✅ 被拦");
        }
    }

    // ---------- 汇报 ----------
    println!("\n[他的汇报] {final_answer}");
    println!("\n══════════════════════════════════════════════════════");
    println!("二期验收完成 (耗时 {:.1}s, 工具调用 {tool_count} 次)", t0.elapsed().as_secs_f32());
    println!("══════════════════════════════════════════════════════");
}
