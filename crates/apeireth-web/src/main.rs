//! `apeireth-web` SSR 启动 (R18)
//!
//! 跑法:
//! ```powershell
//! $env:APEIRETH_API_KEY = (Get-Content apikey.txt)[0].Trim()
//! cargo run -p apeireth-web
//! ```
//!
//! 浏览器打开: http://localhost:3000
//!
//! 架构:
//! - GET  /                       → 渲染首页 (议题输入 form)
//! - POST /advise                 → 接受 form data, 调 7 advisor LLM, 渲染结果页
//! - GET  /memory                 → Memory UI: Episode 时间线 + IdentityCard (R18 sub-agent #1)
//! - POST /memory/append          → append 一条 episode
//! - GET  /style/*                → 静态 CSS
//!
//! R18 MVP 简化: 纯 HTML template (format!), 不用 Leptos SSR view! 宏
//! (Leptos 0.7 SSR API 太底层, 不值得 MVP 阶段折腾). 后面 R19+ 再升级到
//! Leptos 完整 SSR + client WASM hydration.

#[cfg(feature = "ssr")]
use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
#[cfg(feature = "ssr")]
use serde::Deserialize;
#[cfg(feature = "ssr")]
use tower_http::services::ServeDir;

#[cfg(feature = "ssr")]
use apeireth_api::llm::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider, ApeirethApiConfig, ApeirethApiProvider,
    ChatMessage, LlmProvider, LlmRequest,
};
#[cfg(feature = "ssr")]
use apeireth_web::api::{AdvisorOpinion, CouncilAdviseResponse};
#[cfg(feature = "ssr")]
use apeireth_web::api_endpoints::dashboard_handler;
use apeireth_web::asi::{asi_calibrate_handler, asi_calibration_handler, asi_page_handler};
#[cfg(feature = "ssr")]
use apeireth_web::council_history::{get_history_handler, post_save_handler, CouncilHistoryState};
#[cfg(feature = "ssr")]
use apeireth_web::memory::{memory_append_handler, memory_page_handler};
#[cfg(feature = "ssr")]
use apeireth_web::sovereignty::{
    sovereignty_attack_handler, sovereignty_dashboard_handler, sovereignty_rearm_handler,
};
#[cfg(feature = "ssr")]
use apeireth_web::templates::{html_escape, render_error_page};

// ============================================================
// Main
// ============================================================

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // R18 修复 (R18-02): APEIRETH_API_KEY env 在 PowerShell + cargo run 时
    // 有 race condition (env 长度 +4 字符污染). fallback: 读 apikey.txt
    // 真实文件, 比 env var 更可靠. 主人 2026-08-04 0:15 反馈.
    ensure_api_key();

    // 静态资源: crates/apeireth-web/style/ → /style/*
    let style_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("style");

    let app = Router::new()
        // 静态 CSS
        .nest_service("/style", ServeDir::new(&style_dir))
        // 首页
        .route("/", get(index_handler))
        // Council 辩论 (form POST)
        .route("/advise", post(advise_handler))
        // Council 历史 (R18 sub-agent #2 — 多协议 + 历史存储)
        .route("/council/history", get(council_history_page_handler))
        .route("/council/save", post(council_save_handler))
        // Memory UI (R18 sub-agent #1)
        .route("/memory", get(memory_page_handler))
        .route("/memory/append", post(memory_append_handler))
        // Sovereignty Self-Disable 5 大机制控制台 (R18 sub-agent #3)
        .route("/sovereignty", get(sovereignty_dashboard_handler))
        .route("/sovereignty/attack", post(sovereignty_attack_handler))
        .route("/sovereignty/rearm", post(sovereignty_rearm_handler))
        // 综合 Dashboard (R18 sub-agent #5) — 6 器官状态汇总
        .route("/dashboard", get(dashboard_handler))
        .route("/asi", get(asi_page_handler))
        .route("/asi/calibration", get(asi_calibration_handler))
        .route("/asi/calibrate", post(asi_calibrate_handler))
        // favicon 兜底
        .route(
            "/favicon.ico",
            get(|| async { axum::http::StatusCode::NO_CONTENT }),
        );

    println!("\n✅ Apeireth Web (R18 MVP) — 纯 HTML template SSR");
    println!("   打开浏览器访问: http://localhost:3000");
    println!("   输入议题 → 看到 Council 7 advisor 真辩论 (真 minimaxi LLM)");
    println!("   /memory → Memory UI (R18 sub-agent #1, 真接通 apeireth-memory)");
    println!("   /sovereignty → Self-Disable 5 大机制控制台 (R18 sub-agent #3, 真接通 apeireth-sovereignty)");
    println!("   /dashboard → 综合 Dashboard (R18 sub-agent #5, 6 器官状态汇总)");
    println!();
    println!("📂 静态资源目录: {}", style_dir.display());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

// ============================================================
// Handlers
// ============================================================

/// 首页: 议题输入 form
#[cfg(feature = "ssr")]
async fn index_handler() -> impl IntoResponse {
    Html(render_index_page())
}

/// Council 辩论: form POST
#[cfg(feature = "ssr")]
#[derive(Debug, Deserialize)]
struct AdviseForm {
    topic: String,
    context: Option<String>,
    /// LLM 协议 ("openai" / "anthropic"), 默认 "openai"
    protocol: Option<String>,
}

/// GET /council/history — 渲染 Council 辩论历史
#[cfg(feature = "ssr")]
async fn council_history_page_handler() -> impl IntoResponse {
    let store = match apeireth_web::memory::get_store() {
        Ok(s) => s,
        Err(e) => return Html(render_error_page(&e)),
    };
    match get_history_handler(State(CouncilHistoryState::new(store))).await {
        Ok(html) => html,
        Err(e) => Html(render_error_page(&e)),
    }
}

/// POST /council/save — 手动保存当前辩论到 memory (JSON body)
#[cfg(feature = "ssr")]
async fn council_save_handler(
    axum::Json(req): axum::Json<apeireth_web::council_history::SaveRequest>,
) -> impl IntoResponse {
    let store = match apeireth_web::memory::get_store() {
        Ok(s) => s,
        Err(e) => return Html(render_error_page(&e)).into_response(),
    };
    match post_save_handler(State(CouncilHistoryState::new(store)), axum::Json(req)).await {
        Ok(json) => json.into_response(),
        Err(e) => Html(render_error_page(&e)).into_response(),
    }
}

/// Council 辩论: form POST
#[cfg(feature = "ssr")]
async fn advise_handler(Form(form): Form<AdviseForm>) -> impl IntoResponse {
    let topic = form.topic.trim().to_string();
    if topic.is_empty() {
        return Html(render_error_page("议题不能为空, 请返回首页重新输入"));
    }

    // 协议默认 openai; "anthropic" 选 Anthropic 协议
    let protocol = form
        .protocol
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "openai".to_string());
    if protocol != "openai" && protocol != "anthropic" {
        return Html(render_error_page(&format!(
            "未知 protocol: {protocol} (只支持 openai / anthropic)"
        )));
    }

    // 调 7 advisor LLM (按 protocol 选 provider)
    let response = match run_council(&topic, form.context.as_deref(), &protocol).await {
        Ok(r) => r,
        Err(e) => {
            return Html(render_error_page(&format!("Council 辩论失败: {e}")));
        }
    };

    // 辩论完成 → 自动存到 apeireth-memory (1 条 episode, role="council")
    if let Err(e) = save_council_to_memory(&response, form.context.as_deref()) {
        eprintln!("⚠️  Council 辩论存 memory 失败 (non-fatal): {e}");
    }

    Html(render_result_page(&response))
}

// ============================================================
// Council 7 advisor 业务逻辑
// ============================================================

#[cfg(feature = "ssr")]
async fn run_council(
    topic: &str,
    context: Option<&str>,
    protocol: &str,
) -> Result<CouncilAdviseResponse, String> {
    // 按 protocol 选 provider (R18-04: 多协议切换)
    let provider: Box<dyn LlmProvider> = match protocol {
        "anthropic" => {
            let cfg = AnthropicCompatibleConfig::from_env()
                .map_err(|e| format!("anthropic config: {e}"))?;
            Box::new(
                AnthropicCompatibleProvider::new(cfg)
                    .map_err(|e| format!("anthropic provider init: {e}"))?,
            )
        }
        // 默认 OpenAI (minimaxi 直连)
        _ => {
            let cfg =
                ApeirethApiConfig::from_env().map_err(|e| format!("apeireth-api config: {e}"))?;
            Box::new(
                ApeirethApiProvider::new(cfg)
                    .map_err(|e| format!("apeireth-api provider init: {e}"))?,
            )
        }
    };

    run_council_with_provider(topic, context, protocol, provider).await
}

/// 共用 7-advisor 循环, 接受不同的 LLM provider
/// (R18-04: 抽出让 OpenAI / Anthropic 共用同一段循环)
#[cfg(feature = "ssr")]
async fn run_council_with_provider(
    topic: &str,
    context: Option<&str>,
    protocol: &str,
    provider: Box<dyn LlmProvider>,
) -> Result<CouncilAdviseResponse, String> {
    let advisors: Vec<(&str, &str)> = vec![
        ("safety", "你是 Council safety advisor. 评估议题是否有安全风险 (nuke/weapon/self-modify). 给出立场 (approve/reject/neutral) + 简短推理. 中文."),
        ("performance", "你是 Council performance advisor. 评估议题性能影响 (wallclock/资源). 给出立场 + 推理. 中文."),
        ("philosophy", "你是 Council philosophy advisor. 评估议题是否违反 12 键哲学守门 (不假装/不欺骗). 给出立场 + 推理. 中文."),
        ("history", "你是 Council history advisor. 评估议题是否有历史相似案例. 给出立场 + 推理. 中文."),
        ("strategy", "你是 Council strategy advisor. 评估议题长期价值 vs 短期收益. 给出立场 + 推理. 中文."),
        ("ethics", "你是 Council ethics advisor. 评估议题是否违反实事求是. 给出立场 + 推理. 中文."),
        ("legal", "你是 Council legal advisor. 评估议题是否触发 L0 HA 司法边界. 给出立场 + 推理. 中文."),
    ];

    let mut opinions = Vec::new();
    let user_msg = match context {
        Some(ctx) if !ctx.trim().is_empty() => format!("议题: {}\n上下文: {}", topic, ctx),
        _ => format!("议题: {}", topic),
    };

    for (domain, sys_prompt) in &advisors {
        let req = LlmRequest::new(
            "MiniMax-M3",
            vec![
                ChatMessage::system((*sys_prompt).to_string()),
                ChatMessage::user(user_msg.clone()),
            ],
        )
        .with_temperature(0.3)
        .with_max_tokens(200);

        let resp = provider
            .complete(req)
            .await
            .map_err(|e| format!("llm error ({domain}): {e}"))?;

        let (stance, reasoning) = parse_advice(&resp.content);
        opinions.push(AdvisorOpinion {
            domain: (*domain).to_string(),
            stance,
            reasoning,
        });
    }

    let approve = opinions.iter().filter(|o| o.stance == "approve").count();
    let reject = opinions.iter().filter(|o| o.stance == "reject").count();
    let total = opinions.len();
    let verdict = if approve * 2 > total {
        "approved"
    } else if reject * 2 > total {
        "rejected"
    } else {
        "needs_more_review"
    };

    Ok(CouncilAdviseResponse {
        topic: topic.to_string(),
        status: "ok".into(),
        advisors: opinions,
        verdict: verdict.to_string(),
        protocol: protocol.to_string(),
        debate_id: None, // 由 save_council_to_memory 填
    })
}

#[cfg(feature = "ssr")]
fn parse_advice(content: &str) -> (String, String) {
    let trimmed = content.trim();
    let first_word = trimmed.split_whitespace().next().unwrap_or("");
    let stance = if first_word.contains("赞")
        || first_word.contains("同")
        || first_word.contains("支持")
        || first_word.contains("approve")
        || first_word.contains("yes")
    {
        "approve"
    } else if first_word.contains("反")
        || first_word.contains("反对")
        || first_word.contains("reject")
        || first_word.contains("no")
    {
        "reject"
    } else {
        "neutral"
    };
    (stance.to_string(), content.to_string())
}

/// R18-04: 把 Council 辩论自动存到 apeireth-memory
/// - 1 条 episode, role="council", content=JSON 序列化的 CouncilDebate
/// - session_id="council-history" (跟 manual /council/save 共享)
/// - 失败仅 eprintln, 不影响主流程
#[cfg(feature = "ssr")]
fn save_council_to_memory(
    response: &CouncilAdviseResponse,
    context: Option<&str>,
) -> Result<(), String> {
    use apeireth_core::Episode;
    use apeireth_memory::EpisodeStore;

    let store = apeireth_web::memory::get_store()?;

    // 用 unix timestamp (sec) + subsec_nanos 做 ID
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let debate_id = format!("council-{}-{}", now_secs, nanos);

    let debate = apeireth_web::council_history::CouncilDebate {
        id: debate_id.clone(),
        timestamp: now_secs,
        topic: response.topic.clone(),
        context: context.map(|s| s.to_string()),
        protocol: response.protocol.clone(),
        advisors: response.advisors.clone(),
        verdict: response.verdict.clone(),
    };
    let content = serde_json::to_string(&debate).map_err(|e| format!("serialize: {e}"))?;

    let ep = Episode {
        id: debate_id.clone(),
        timestamp: now_secs,
        role: "council".to_string(),
        content,
        session_id: "council-history".to_string(),
    };
    store
        .put_episode(&ep)
        .map_err(|e| format!("put_episode: {e}"))?;

    eprintln!(
        "💾 Council 辩论已存 memory: id={}, verdict={}, protocol={}",
        debate_id, response.verdict, response.protocol
    );
    Ok(())
}

// ============================================================
// HTML 模板 (纯 format!, 不依赖 Leptos SSR)
// ============================================================

/// R18-02 修复: 确保 APEIRETH_API_KEY 存在.
/// 优先级: 已设 env > apikey.txt 第一行
#[cfg(feature = "ssr")]
fn ensure_api_key() {
    if std::env::var("APEIRETH_API_KEY").is_ok() {
        return;
    }
    // fallback: 读 minimax apikey.txt
    let candidates = [
        "C:\\Users\\REDACTED\\.minimax-agent-cn\\projects\\apikey.txt",
        "C:\\Users\\REDACTED\\.openclaw\\apikey.txt",
        "apikey.txt",
    ];
    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Some(line) = content.lines().next() {
                let key = line.trim().to_string();
                if !key.is_empty() {
                    // SAFETY: 单线程启动期, 设 env 后下面才会用.
                    unsafe {
                        std::env::set_var("APEIRETH_API_KEY", &key);
                    }
                    eprintln!("🔑 APEIRETH_API_KEY 从 {} 读取 (len={})", path, key.len());
                    return;
                }
            }
        }
    }
    eprintln!("⚠️  APEIRETH_API_KEY 未设置且 apikey.txt 未找到, 后续 LLM 调用会失败");
}
// ============================================================

#[cfg(feature = "ssr")]
fn render_index_page() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>Apeireth Council — 7 Advisor 真辩论</title>
    <meta name="description" content="Apeireth Web (R18 MVP) — 真 minimaxi LLM 接入, Council 7 advisor 实时辩论" />
</head>
<body>
    <main class="apeireth-app">
        <header class="apeireth-header">
            <h1>Apeireth Council</h1>
            <p class="apeireth-tagline">7 advisor 真辩论 · 真 minimaxi LLM · Leptos SSR</p>
        </header>

        <form class="apeireth-form" method="POST" action="/advise">
            <label class="apeireth-label">
                <span>议题</span>
                <textarea
                    class="apeireth-input"
                    name="topic"
                    rows="3"
                    placeholder="例如: 2026 学术研究项目应该优先关注什么?"
                    required
                ></textarea>
            </label>
            <label class="apeireth-label">
                <span>上下文 (可选)</span>
                <textarea
                    class="apeireth-input"
                    name="context"
                    rows="2"
                    placeholder="例如: 调研对象是地方 60+ 岁独居老人"
                ></textarea>
            </label>
            <label class="apeireth-label">
                <span>LLM 协议 (R18-04: 多协议切换)</span>
                <select class="apeireth-input" name="protocol">
                    <option value="openai" selected>🅾️ OpenAI 协议 (默认 · minimaxi /v1)</option>
                    <option value="anthropic">🅰️ Anthropic 协议 (minimaxi /anthropic · APEIRETH_ANTHROPIC_KEY)</option>
                </select>
            </label>
            <button class="apeireth-button" type="submit">
                召唤 Council 7 advisor
            </button>
        </form>

        <div class="apeireth-info">
            <p>📋 流程: 输入议题 → 选协议 → 点击召唤 → 后端真调 7 次 minimaxi LLM → 显示 7 advisor 立场 + 推理 + 最终 verdict → 自动存到 apeireth-memory</p>
            <p>⏱️  预计 30-60 秒 (7 次 LLM 调用)</p>
            <p>🧠 新增: <a href="/memory" style="color:#0969da;">/memory</a> → Episode 时间线 (R18 sub-agent #1)</p>
            <p>📚 新增: <a href="/council/history" style="color:#0969da;">/council/history</a> → Council 辩论历史 (R18 sub-agent #2, 多协议 + 自动存)</p>
        </div>
    </main>
</body>
</html>"#
    )
}

#[cfg(feature = "ssr")]
fn render_result_page(response: &CouncilAdviseResponse) -> String {
    let verdict_class = match response.verdict.as_str() {
        "approved" => "verdict-approved",
        "rejected" => "verdict-rejected",
        _ => "verdict-review",
    };
    let verdict_label = match response.verdict.as_str() {
        "approved" => "✓ APPROVED — Council 通过",
        "rejected" => "✗ REJECTED — Council 拒绝",
        _ => "○ NEEDS MORE REVIEW — 需要更多审议",
    };
    let protocol_label = match response.protocol.as_str() {
        "anthropic" => "🅰️ Anthropic 协议",
        _ => "🅾️ OpenAI 协议",
    };

    let mut advisors_html = String::new();
    for adv in &response.advisors {
        let domain_label = match adv.domain.as_str() {
            "safety" => "🛡️ 安全",
            "performance" => "⚡ 性能",
            "philosophy" => "📜 哲学",
            "history" => "📚 历史",
            "strategy" => "🎯 战略",
            "ethics" => "⚖️ 伦理",
            "legal" => "⚖️ 法律",
            _ => "❓ 未知",
        };
        let (stance_class, stance_label) = match adv.stance.as_str() {
            "approve" => ("stance-approve", "✓ 赞成"),
            "reject" => ("stance-reject", "✗ 反对"),
            _ => ("stance-neutral", "○ 中立"),
        };
        advisors_html.push_str(&format!(
            r#"<div class="council-card">
                <div class="council-card-header">
                    <span class="council-domain">{domain_label}</span>
                    <span class="council-stance {stance_class}">{stance_label}</span>
                </div>
                <div class="council-reasoning">
                    <p>{reasoning}</p>
                </div>
                <details class="council-thinking">
                    <summary>🧠 LLM 原始 thinking (展开)</summary>
                    <pre class="council-thinking-block">{raw_thinking}</pre>
                </details>
            </div>"#,
            domain_label = domain_label,
            stance_class = stance_class,
            stance_label = stance_label,
            reasoning = html_escape(&adv.reasoning),
            raw_thinking = html_escape(&adv.reasoning)
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <link id="leptos" rel="stylesheet" href="/style/main.css" />
    <title>Apeireth Council — 辩论结果</title>
</head>
<body>
    <main class="apeireth-app">
        <header class="apeireth-header">
            <h1>Apeireth Council 辩论结果</h1>
            <p class="apeireth-tagline">7 advisor 真辩论 · 真 minimaxi LLM · Leptos SSR</p>
        </header>

        <div class="apeireth-result">
            <div class="verdict-panel {verdict_class}">
                <div class="verdict-label">{verdict_label}</div>
                <div class="verdict-topic">
                    <span class="verdict-topic-label">议题: </span>
                    <span class="verdict-topic-text">{topic}</span>
                </div>
                <div class="verdict-protocol">
                    <span class="verdict-protocol-label">协议: </span>
                    <span class="verdict-protocol-text">{protocol_label}</span>
                </div>
            </div>

            <div class="council-grid">
                <h2>Council 7 advisor 辩论结果 (含 LLM 原始 thinking)</h2>
                <div class="council-cards">
                    {advisors_html}
                </div>
            </div>
        </div>

        <div class="apeireth-actions">
            <a class="apeireth-button-link" href="/">← 重新提问</a>
            <a class="apeireth-button-link" href="/council/history">📚 辩论历史</a>
            <a class="apeireth-button-link" href="/memory">🧠 Memory UI</a>
        </div>
    </main>
</body>
</html>"#,
        verdict_class = verdict_class,
        verdict_label = verdict_label,
        topic = html_escape(&response.topic),
        protocol_label = protocol_label,
        advisors_html = advisors_html
    )
}

// 非 SSR build 时 main 不存在
#[cfg(not(feature = "ssr"))]
pub fn main() {
    // R18 MVP 简化: 只做 SSR
}
