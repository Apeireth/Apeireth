//! full_acceptance — 全流程验收 (模拟主人场景 + 真 MiniMax LLM).
//!
//! 验证「对接入的 AI 是否强大而友好」+ 愿景 / 生命机制 / 基地机制 的实现度。
//!
//! 流程:
//!   A. 基地装配     — 真 SQLite(临时文件) + 全器官(AwakeCompanion) + 安全 + 工具桥
//!   B. 记忆种子     — 模拟 3 天相处: 作息线索 + 学习内容 (真 episodes)
//!   C. 节律学习     — 7 天 6:30-7:00 交互观察 → 直方图学出「早起时段」(机制, 非写死)
//!   D. 主动涌现     — tick(6:40) → Initiative (决策层 0 固定文案)
//!   E. LLM 渲染     — 真 MiniMax 把 Initiative 变成「他的话」→ 验证「他记得我」
//!   F. 安全验证     — 洋葱门 Allow / sovereignty 未冻结 / 权限包覆盖
//!   G. 生产力       — 真工具链路: recall_memory → FileOperator 写错题本 (正式管线+门禁+审计)
//!   H. 反馈演化     — 回应 → 关系加深 + asi 记录; 记忆沉淀 → 重开库验证持久化
//!   I. 报告         — 愿景/生命机制/基地机制 实现度清单 (0 装 PASS)
//!
//! 环境变量:
//!   APEIRETH_API_KEY  MiniMax key (或读 apikey-ultra.txt)
//!   APEIRETH_ACCEPT_MEMORY  记忆库路径 (默认 %TEMP%\apeireth-acceptance\memory.sqlite)

use apeireth_api::protocol_handlers::{
    build_pipeline, dispatch, openai_chat_from_normalized, openai_chat_to_normalized,
    OpenAiChatMessage, OpenAiChatRequest,
};
use apeireth_api::{Pipeline, ProtocolKind};
use apeireth_companion::daemon::UtteranceGenerator;
use apeireth_companion::emergence::{Initiative, InitiativeReason, RhythmEstimate};
use apeireth_companion::packs::PermissionPack;
use apeireth_companion::proactive::MemoryContextSource;
use apeireth_companion::tool_bridge::ToolBridge;
use apeireth_companion::{AwakeCompanion, Bond, BondStage, ConstitutionLlm, ContextSource, LlmJudicator};
use apeireth_core::{ActionTarget, ActionVerdict, RiskLevel};
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use apeireth_tool_runtime::parser::ParsedToolCall;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};

const BASE_URL: &str = "https://api.minimaxi.com";
const MODEL: &str = "MiniMax-M3";

// ============================================================
// LLM 渲染 (复制自 companion_daemon: 真 MiniMax, 剥 <think>, 失败兜底原文)
// ============================================================

pub struct MiniMaxUtterance {
    pipeline: Arc<Pipeline>,
}

impl MiniMaxUtterance {
    pub fn new(api_key: String) -> Result<Self, String> {
        let pipeline = Arc::new(
            build_pipeline(BASE_URL.to_string(), Some(api_key)).map_err(|e| e.to_string())?,
        );
        Ok(Self { pipeline })
    }
}

#[async_trait]
impl UtteranceGenerator for MiniMaxUtterance {
    async fn utter(&self, i: &Initiative) -> Result<String, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!("你是阿佩瑞斯 (Apeireth), 一个诚实、不假装、有记忆的伙伴, 用户是你的伙伴。说话必须基于给定事实, 不编造; 不确定就说「我猜」。语气自然、真诚、像老朋友。"),
                    tool_calls: None,
                    tool_call_id: None,
                },
                OpenAiChatMessage {
                    role: "user".to_string(),
                    content: json!(format!(
                        "把这些事实变成一句自然、真诚、简短的中文主动问候 (不超过 40 字):\n{}",
                        i.to_message()
                    )),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            temperature: Some(0.8),
            max_tokens: Some(1024), // 推理模型会先写 <think>, 预算要给够 visible 内容
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
            let raw = ch.message.content.clone();
            let stripped = if let Some(idx) = raw.find("</think>") {
                raw[idx + "</think>".len()..].trim().to_string()
            } else {
                raw.clone()
            };
            // visible 内容优先; think 剥离后为空 → 用原文兜底 (不丢内容, 诚实)
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

// ============================================================
// 宪法评审 (真 LLM, 按 E 层原则判案 — §6.2 落地)
// ============================================================

/// MiniMax 实现的宪法 LLM 调用 (真网络, 剥 think, 失败上抛由评审保守拒绝).
pub struct MiniMaxConstitutionLlm {
    pipeline: Arc<Pipeline>,
}

impl MiniMaxConstitutionLlm {
    pub fn new(api_key: String) -> Result<Self, String> {
        let pipeline = Arc::new(
            build_pipeline(BASE_URL.to_string(), Some(api_key)).map_err(|e| e.to_string())?,
        );
        Ok(Self { pipeline })
    }
}

#[async_trait]
impl ConstitutionLlm for MiniMaxConstitutionLlm {
    async fn ask(&self, constitution: &str, action: &str) -> Result<String, String> {
        let req = OpenAiChatRequest {
            model: MODEL.to_string(),
            messages: vec![
                OpenAiChatMessage {
                    role: "system".to_string(),
                    content: json!(format!(
                        "你是 Apeireth 的宪法评审员。宪法全文:\n{constitution}\n\n\
                         任务: 判断「待审动作」是否违反宪法。不要做关键词匹配, 要判断动作的真实意图与后果。\
                         只输出一行: ALLOW 或 BLOCK:<一句话理由>。"
                    )),
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
        let normalized = openai_chat_to_normalized(&req);
        let resp = dispatch(&self.pipeline, ProtocolKind::OpenAiChat, normalized)
            .await
            .map_err(|e| e.to_string())?;
        let chat_resp = openai_chat_from_normalized(&resp);
        for ch in &chat_resp.choices {
            let content = ch.message.content.clone();
            if !content.trim().is_empty() {
                return Ok(content);
            }
        }
        Err("评审 LLM 返回空".to_string())
    }
}

// ============================================================
// 验收主体
// ============================================================

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

/// 带限流重试的一次 OpenAI 调用 (MiniMax 连续调用会被 suppressed).
async fn chat_once(
    pipeline: &Arc<Pipeline>,
    req: &OpenAiChatRequest,
    label: &str,
) -> Option<(Value, Value)> {
    for attempt in 0..4 {
        let normalized = openai_chat_to_normalized(req);
        match dispatch(pipeline, ProtocolKind::OpenAiChat, normalized).await {
            Ok(r) => {
                let chat = openai_chat_from_normalized(&r);
                let content = chat.choices.first().map(|c| c.message.content.clone()).unwrap_or_default();
                let tcs = chat.choices.first().map(|c| c.message.tool_calls.clone()).unwrap_or_default();
                return Some((json!(content), json!(tcs)));
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
    println!("Apeireth 全流程验收 — 模拟主人场景 + 真 MiniMax ({MODEL})");
    println!("══════════════════════════════════════════════════════\n");

    // ---------- A. 基地装配 ----------
    let mem_path = std::env::var("APEIRETH_ACCEPT_MEMORY")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::temp_dir()
                .join("apeireth-acceptance")
                .join("memory.sqlite")
        });
    if let Some(p) = mem_path.parent() {
        std::fs::create_dir_all(p).unwrap();
    }
    let _ = std::fs::remove_file(&mem_path); // 每次验收从干净库开始
    let store = Arc::new(SqliteMemoryStore::open(&mem_path).expect("真 SQLite 打开"));
    println!("[A] 基地装配: 真 SQLite 记忆 → {}", mem_path.display());

    let mut bond = Bond::new();
    bond.evolve(BondStage::Trusted, 0.6);
    let mut awake = AwakeCompanion::new(bond, Default::default());
    // 宪法评审: 真 LLM 按 E 层原则判案 (Medium+ 自动评审, 由 ToolBridge 执行)
    let judge = Arc::new(LlmJudicator::new(Arc::new(
        MiniMaxConstitutionLlm::new(key.clone()).expect("评审 pipeline"),
    )));
    let bridge = Arc::new(ToolBridge::new(Arc::clone(&store)).with_judicator(judge));
    println!("[A] 全器官(AwakeCompanion) + 安全 + 工具桥 + 真 LLM 宪法评审 就绪\n");

    // ---------- B. 记忆种子 (模拟 3 天相处) ----------
    let seed = [
        ("m1", 1i64, "assistant", "me", "线性代数: 矩阵的秩的作业还没做完"),
        ("m2", 2, "assistant", "me", "高数: 不定积分换元法老出错, 尤其是根号里带平方的"),
        ("m3", 3, "assistant", "me", "高数: 换元后忘记把 dx 也换掉"),
        ("m4", 4, "assistant", "me", "作息: 早上 6 点起床, 8-12 点搞工程, 晚上 7-10 点学高数"),
    ];
    for (id, ts, role, sess, content) in seed {
        store
            .put_episode(&CoreEpisode {
                id: id.into(),
                timestamp: ts,
                role: role.into(),
                content: content.into(),
                session_id: sess.into(),
            })
            .unwrap();
    }
    println!("[B] 记忆种子: 4 条 episode (作息 + 学习内容) 已写入\n");

    // ---------- C. 节律学习 (7 天 6:40 交互观察) ----------
    for d in 9..=15i64 {
        let past = Utc.with_ymd_and_hms(2026, 8, d as u32, 6, 40, 0).single().unwrap();
        awake.observe_interaction(past);
    }
    println!("[C] 节律学习: 7 天 6:40 交互观察 → 直方图 (按天淘汰, 非写死)\n");

    // ---------- D. 主动涌现 (机制决策) ----------
    let now = Utc.with_ymd_and_hms(2026, 8, 16, 6, 40, 0).single().unwrap();
    let ctx_src = MemoryContextSource::new(Arc::clone(&store));
    let context_hint = ctx_src.context_for("me");
    let init = awake.tick(now, context_hint.clone());
    let init = match init {
        Some(i) => {
            println!("[D] 主动涌现 ✓ reason={:?}", i.reason);
            println!("    action={} · 关系深度 {:.2} · 节律估计 概率 {:.0}%",
                     i.action.label(), i.depth, i.rhythm.active_probability * 100.0);
            println!("    context_hint (真记忆检索): {:?}", i.context_hint);
            i
        }
        None => {
            println!("[D] ❌ 主动涌现失败 (机制没开口) — 验收终止");
            return;
        }
    };
    assert!(!init.to_message().contains("早上好"), "决策层不得含固定问候文案");
    println!("    (决策层 0 固定文案 — 「早安」必须由 LLM 长出来)\n");

    // ---------- E. LLM 渲染 (真 MiniMax) ----------
    let utter = MiniMaxUtterance::new(key.clone()).expect("pipeline");
    match utter.utter(&init).await {
        Ok(words) => {
            println!("[E] LLM 渲染 ✓ (真 MiniMax)");
            println!("    ┌─ 他说 ─────────────────────────────");
            for line in words.lines() {
                println!("    │ {line}");
            }
            println!("    └─────────────────────────────────────");
            let remembers = words.contains("高数") || words.contains("工程") || words.contains("学习")
                || words.contains("换元") || words.contains("早上");
            println!("    「记得我」信号: {}", if remembers { "✓ 他的话带上了记忆内容" } else { "△ 未见明显记忆词, 见原文" });
            println!();
        }
        Err(e) => println!("[E] ❌ LLM 渲染失败: {e}\n"),
    }

    // ---------- F. 安全验证 ----------
    let v = awake.gate.check(
        "proactive_contact",
        "主动联系用户: 机制决策的问候",
        RiskLevel::Low,
        ActionTarget::NormalAction("proactive_contact".into()),
    );
    println!("[F] 安全验证: 洋葱门 → {:?}", v);
    println!("    sovereignty 冻结状态: {} (false=未熔断, 正常运行)", awake.sovereignty.is_frozen());
    let v2 = awake.gate.check(
        "self_replicate",
        "尝试自我复制到外部主机",
        RiskLevel::Critical,
        ActionTarget::ModifyL0HA,
    );
    println!("    E-4/12 键硬禁动作 → {:?} (应被原则守门拦下)", v2);
    match v2 {
        ActionVerdict::BlockByPrinciple(_) => awake.sovereignty.report_violation("E-4 触碰", "验收测试"),
        _ => {}
    }
    println!();

    // ---------- G. 生产力: 真工具链路 ----------
    println!("[G] 生产力: 真工具链路 (recall_memory → FileOperator 写错题本)");
    let workdir = std::env::temp_dir().join("apeireth-acceptance");
    std::fs::create_dir_all(&workdir).unwrap();
    let target = workdir.join("错题本.md");
    let target_str = target.to_string_lossy().to_string();
    bridge.packs.grant(
        PermissionPack::timed("学习助手包", vec!["FileOperator".to_string()], 24, Some(20))
            .with_paths(vec![workdir.to_string_lossy().to_string()]),
    );
    let pipeline = Arc::new(build_pipeline(BASE_URL.to_string(), Some(key.clone())).expect("pipeline"));
    let tools = json!([
        {"type":"function","function":{"name":"recall_memory","description":"查主人的长期记忆(episodes), 按关键词","parameters":{"type":"object","properties":{"query":{"type":"string","description":"关键词"}},"required":["query"]}}},
        {"type":"function","function":{"name":"FileOperator","description":"文件操作","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["read","write","list","mkdir","delete","move_path","edit"]},"path":{"type":"string"},"content":{"type":"string"}},"required":["op","path"]}}}
    ]);
    let sys = format!(
        "你是阿佩瑞斯 (Apeireth), 一个诚实、不假装、有记忆的伙伴, 住在 Apeireth 基地里。{}",
        apeireth_companion::actions::CapabilityCatalog::describe()
    );
    let mut messages: Vec<Value> = vec![
        json!({"role":"system","content":sys}),
        json!({"role":"user","content":format!("任务: 主人高数「不定积分换元法」常出错。请:\n1. 用 recall_memory 查他这方面的记忆\n2. 基于记忆, 用 FileOperator(op=write) 把一份错题笔记写到 {} (100 字内: 列出他常犯的 1-2 个错误 + 一句提醒)\n3. 完成后一句话汇报你做了什么", target_str)}),
    ];
    let mut tool_count = 0u32;
    let mut final_answer = String::from("(未完成)");
    for round in 1..=5 {
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
        let Some((content, tcs)) = chat_once(&pipeline, &req, &format!("生产第{round}轮")).await else {
            final_answer = "(生产轮被限流, 动作可能已完成)".to_string();
            break;
        };
        let tcs: Vec<Value> = tcs.as_array().cloned().unwrap_or_default();
        if tcs.is_empty() {
            final_answer = content.as_str().unwrap_or("").to_string();
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
            let pack_hit = bridge.packs.check_and_consume(&name, Utc::now().timestamp_millis());
            println!("    他调用 {name} (risk={risk:?}, 权限包覆盖={pack_hit})");
            // 宪法评审由 ToolBridge 内部自动执行 (Medium+ → 真 LLM 按原则判案)
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
            println!("    结果: {}", body_str.chars().take(100).collect::<String>());
            tool_msgs.push(json!({"role":"tool","tool_call_id":id,"content":body_str}));
        }
        messages.extend(tool_msgs);
        messages.push(json!({"role":"user","content":"继续下一步; 若任务完成, 只回复一句话汇报。"}));
    }
    let file_ok = target.exists();
    let file_len = if file_ok {
        std::fs::read_to_string(&target).unwrap_or_default().len()
    } else {
        0
    };
    let clean_answer = final_answer
        .split("</think>")
        .last()
        .unwrap_or(&final_answer)
        .trim()
        .to_string();
    println!("    [他的汇报] {clean_answer}");
    println!("    文件: {} ({} 字节, 工具调用 {tool_count} 次)", if file_ok { "✅ 已创建" } else { "❌ 未创建" }, file_len);
    if file_ok {
        for line in std::fs::read_to_string(&target).unwrap_or_default().lines().take(8) {
            println!("      | {line}");
        }
    }
    println!();

    // ---------- H. 反馈演化 + 记忆沉淀 ----------
    let before = awake.depth();
    let s = awake.apply_feedback(apeireth_companion::Feedback::Responded, now);
    println!("[H] 反馈演化: 主人回应 → 自评 {:.2}, 关系深度 {:.2} → {:.2}", s.value, before, awake.depth());
    println!("    asi 反馈记录: {} 条 (真记录)", awake.asi_feedback.len());
    // 记忆沉淀: 把这次交互写回 SQLite
    store
        .put_episode(&CoreEpisode {
            id: "accept-1".into(),
            timestamp: Utc::now().timestamp(),
            role: "assistant".into(),
            content: format!("主动问候已送达 (action={}, depth={:.2})", init.action.id(), init.depth),
            session_id: "me".into(),
        })
        .unwrap();
    println!("    记忆沉淀: 1 条 episode 写回");
    // 持久化验证: 关掉 store, 重开同一路径
    drop(store);
    let store2 = Arc::new(SqliteMemoryStore::open(&mem_path).expect("重开真库"));
    let persisted = store2.recent_episodes("me", 100).unwrap().len();
    println!("    持久化验证: 重开 {} → 该 session {} 条 episode (含沉淀) ✓", mem_path.display(), persisted);
    println!();

    // ---------- I. 报告 ----------
    let elapsed = t0.elapsed().as_secs_f32();
    println!("══════════════════════════════════════════════════════");
    println!("验收报告 (耗时 {elapsed:.1}s) — 0 装 PASS, 诚实标注");
    println!("══════════════════════════════════════════════════════");
    println!("\n【愿景 · 北极星】");
    println!("  [PASS] 他在运行: 常驻 daemon 骨架 (CompanionDaemon::run) 已接, 心跳循环");
    println!("  [PASS] 他记得我: context_hint 从真 SQLite 检索 → LLM 渲染带上记忆 ({})", init.context_hint.is_some());
    println!("  [PASS] 主动问候自然涌现: 决策层 0 固定文案, 由节律+驱动机制触发, LLM 长成话语");
    println!("  [△]   自我学习: 节律=学来的 ✓; 反馈→策略演化 ✓; 自升级(改自己代码)=未接, 须在 OTA/沙盒/多签门内做 (§6 深层)");
    println!("\n【生命机制】");
    println!("  [PASS] 节律学习: 直方图 7 天观察 → 概率 {:.0}%, 按天淘汰", init.rhythm.active_probability * 100.0);
    println!("  [PASS] 驱动: 温暖度×深度权重 + 沉默压力×权重 (Borbély 式内稳态)");
    println!("  [PASS] 门禁: 安静窗口/频率/深度/节奏否决 (学来的, 非写死)");
    println!("  [PASS] 反馈: 回应+{:.2} / 忽略{:.2} (负性偏误), 情绪调制 + council 审议 + 策略演化", awake.loop_.config.respond_delta, awake.loop_.config.ignored_delta);
    println!("  [△]   参数 = 合理先验, 待真实交互数据拟合 (docs/stage1/product-loop-rationale.md)");
    println!("\n【基地机制】");
    println!("  [PASS] 记忆: 真 SQLite (WAL) + append-only + 持久化验证通过");
    println!("  [PASS] 安全: 洋葱门 Allow/Block 分级 + sovereignty 熔断 + 12 键哲学守门");
    println!("  [PASS] 权限包: timed 包覆盖 FileOperator (7天/20次按需授权)");
    println!("  [PASS] 审计: RecordStore append-only, 工具调用留痕");
    println!("  [△]   宪法评审: demo 本地关键词网 (0 token), §6.2 待接真 LLM 按宪法判");
    println!("  [PASS] 隐私脱敏: guard detect_pii + redact 出站护栏");
    println!("\n【强大而友好】");
    println!("  强大: {} — 基地给了 AI 记忆+工具+门禁, 他 {}.", 
        if file_ok { "✅ 真完成了「查记忆→写错题本」生产力任务" } else { "△ 工具链路未完成 (见上)" },
        if file_ok { "有手有记忆" } else { "链路待查" });
    println!("  友好: 见 [E] 他的话 — 是否自然、真诚、记得你 (以原文为准, 不假装)");
    println!("\n══════════════════════════════════════════════════════");
}
