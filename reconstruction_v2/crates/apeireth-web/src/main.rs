//! `apeireth-web` SSR 启动 (v2 reconstruction)
//!
//! 跑法:
//! ```powershell
//! cargo run -p apeireth-web
//! ```
//!
//! 浏览器打开: http://localhost:3000
//!
//! **v2 适配 vs v1**:
//! - v1 调 `apeireth_api::llm::{AnthropicCompatibleProvider, ApeirethApiProvider}` 真 HTTP 调 LLM,
//!   v2 没有 `apeireth-api` crate. 本 crate 直接用 `apeireth_council::MockLlmProvider`
//!   (ScriptedMockLlm 或自定义 struct), 7 advisor 全部内置返回. 这跟 v1 `apeireth_council::mock_llm.rs`
//!   设计意图一致 — 真 LLM 接入走 `apeireth-council::llm_backend::LlmAdvisorBackend` (R33-4+, 后续接入).
//! - v1 main.rs 路由多 8 大器官 dashboard; v2 简化为 6 器官 demo snapshot (不依赖 8 个 v1 器官 crate).
//!
//! 架构:
//! - GET  /                       → 渲染首页 (议题输入 form)
//! - POST /advise                 → 接受 form data, 跑 7 advisor mock LLM, 渲染结果页
//! - GET  /council/history        → Council 辩论历史
//! - POST /council/save           → 保存辩论到 memory (JSON body)
//! - GET  /memory                 → Memory UI
//! - POST /memory/append          → append episode
//! - GET  /sovereignty            → 5 大机制控制台
//! - POST /sovereignty/attack     → 触发攻击
//! - POST /sovereignty/rearm      → 尝试 rearm
//! - GET  /dashboard              → 6 器官 dashboard
//! - GET  /asi                    → ASI 24 维雷达图
//! - GET  /asi/calibration        → ASI 校准状态
//! - POST /asi/calibrate          → 手动校准
//! - GET  /style/*                → 静态 CSS

#[cfg(feature = "ssr")]
use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
#[cfg(feature = "ssr")]
use tower_http::services::ServeDir;

#[cfg(feature = "ssr")]
mod mock_llm;
#[cfg(feature = "ssr")]
use mock_llm::{CouncilMember, MockLlmProvider, MockLlmResponse, ScriptedMockLlm};

#[cfg(feature = "ssr")]
use apeireth_web::api::{AdvisorOpinion, CouncilAdviseResponse};
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
#[cfg(feature = "ssr")]
use apeireth_web::asi::{asi_calibrate_handler, asi_calibration_handler, asi_page_handler};
#[cfg(feature = "ssr")]
use apeireth_web::api_endpoints::dashboard_handler;

#[cfg(feature = "ssr")]
use std::collections::HashMap;

// ============================================================
// Main
// ============================================================

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    println!("
✅ Apeireth Web (v2 reconstruction) — 纯 HTML template SSR");
    println!("   打开浏览器访问: http://localhost:3000");
    println!("   /memory → Memory UI");
    println!("   /council/history → Council 辩论历史");
    println!("   /sovereignty → Self-Disable 5 大机制控制台");
    println!("   /asi → ASI 24 维雷达图");
    println!("   /asi/calibration → ASI ML 校准状态");
    println!("   /dashboard → 综合 Dashboard (6 器官状态汇总, v2 demo snapshot)");
    println!();

    // 静态资源: crates/apeireth-web/style/ → /style/*
    let style_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("style");

    let app = Router::new()
        .nest_service("/style", ServeDir::new(&style_dir))
        // 首页
        .route("/", get(index_handler))
        // Council 辩论 (form POST)
        .route("/advise", post(advise_handler))
        // Council 历史
        .route("/council/history", get(council_history_page_handler))
        .route("/council/save", post(council_save_handler))
        // Memory UI
        .route("/memory", get(memory_page_handler))
        .route("/memory/append", post(memory_append_handler))
        // Sovereignty
        .route("/sovereignty", get(sovereignty_dashboard_handler))
        .route("/sovereignty/attack", post(sovereignty_attack_handler))
        .route("/sovereignty/rearm", post(sovereignty_rearm_handler))
        // Dashboard
        .route("/dashboard", get(dashboard_handler))
        // ASI
        .route("/asi", get(asi_page_handler))
        .route("/asi/calibration", get(asi_calibration_handler))
        .route("/asi/calibrate", post(asi_calibrate_handler))
        // favicon 兜底
        .route(
            "/favicon.ico",
            get(|| async { axum::http::StatusCode::NO_CONTENT }),
        );

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
#[derive(Debug, serde::Deserialize)]
struct AdviseForm {
    topic: String,
    context: Option<String>,
    /// LLM 协议 ("openai" / "anthropic"), 默认 "openai"
    protocol: Option<String>,
}

/// GET /council/history — 渲染 Council 辩论历史
#[cfg(feature = "ssr")]
async fn council_history_page_handler() -> impl IntoResponse {
    let web_arc = match apeireth_web::memory::get_store() {
        Ok(s) => s,
        Err(e) => return Html(render_error_page(&e)),
    };
    let state = CouncilHistoryState::new(web_arc);
    match get_history_handler(State(state)).await {
        Ok(html) => html,
        Err(e) => Html(render_error_page(&e)),
    }
}

/// POST /council/save — 手动保存当前辩论到 memory (JSON body)
#[cfg(feature = "ssr")]
async fn council_save_handler(
    axum::Json(req): axum::Json<apeireth_web::council_history::SaveRequest>,
) -> impl IntoResponse {
    let web_arc = match apeireth_web::memory::get_store() {
        Ok(s) => s,
        Err(e) => return Html(render_error_page(&e)).into_response(),
    };
    let state = CouncilHistoryState::new(web_arc);
    match post_save_handler(State(state), axum::Json(req)).await {
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

    // 跑 7 advisor mock LLM
    let response = match run_council(&topic, form.context.as_deref(), &protocol).await {
        Ok(r) => r,
        Err(e) => {
            return Html(render_error_page(&format!("Council 辩论失败: {e}")));
        }
    };

    // 辩论完成 → 自动存到 web memory (1 条 episode, role="council")
    if let Err(e) = save_council_to_memory(&response, form.context.as_deref()) {
        eprintln!("⚠️  Council 辩论存 memory 失败 (non-fatal): {e}");
    }

    Html(render_result_page(&response))
}

// ============================================================
// Council 7 advisor 业务逻辑 (用 apeireth_council::MockLlmProvider)
// ============================================================

/// 构造 mock LLM — v2 替代 v1 apeireth_api::llm
/// 按 prompt 关键词匹配: "safety" / "performance" / ...
fn build_mock_llm(protocol: &str) -> Box<dyn MockLlmProvider> {
    let scripted = ScriptedMockLlm::new()
        .with_script("safety", MockLlmResponse::ok("赞成: 安全风险已评估, 无 immediate danger. 中文. ✅ approve"))
        .with_script("performance", MockLlmResponse::ok("赞成: 性能影响在合理范围内. ✓ approve"))
        .with_script("philosophy", MockLlmResponse::ok("赞成: 不违反 5 哲学守门 (不假装/不欺骗/...). ✓ approve"))
        .with_script("history", MockLlmResponse::ok("反对: 历史案例显示该议题有反向先例. ✗ reject"))
        .with_script("strategy", MockLlmResponse::ok("赞成: 长期价值高于短期收益. ✓ approve"))
        .with_script("ethics", MockLlmResponse::ok("赞成: 符合实事求是原则. ✓ approve"))
        .with_script("legal", MockLlmResponse::ok("反对: 触发 L0 HA 司法边界 — 谨慎. ✗ reject"))
        .with_default(MockLlmResponse::ok("赞成: 中立评估, 未发现重大风险. ✓ approve"));

    // protocol 切换 mock 行为 (demo): anthropic 协议更多 reject
    if protocol == "anthropic" {
        let scripted = ScriptedMockLlm::new()
            .with_script("safety", MockLlmResponse::ok("反对: 安全边界过宽, 需严格审查. ✗ reject"))
            .with_script("performance", MockLlmResponse::ok("赞成: 性能可接受. ✓ approve"))
            .with_script("philosophy", MockLlmResponse::ok("赞成: 不违反哲学守门. ✓ approve"))
            .with_script("history", MockLlmResponse::ok("反对: 历史先例不利. ✗ reject"))
            .with_script("strategy", MockLlmResponse::ok("反对: 战略风险高. ✗ reject"))
            .with_script("ethics", MockLlmResponse::ok("赞成: 符合伦理. ✓ approve"))
            .with_script("legal", MockLlmResponse::ok("反对: 触发司法边界. ✗ reject"))
            .with_default(MockLlmResponse::ok("反对: 默认保守. ✗ reject"));
        Box::new(scripted)
    } else {
        Box::new(scripted)
    }
}

/// 7 advisor 列表 — 对应 v1 同款 7 个领域
fn advisor_list() -> Vec<(&'static str, CouncilMember)> {
    vec![
        ("safety", CouncilMember::new("safety_advisor", "评估议题是否有安全风险 (nuke/weapon/self-modify).", "10 年安全审计", "mock")),
        ("performance", CouncilMember::new("performance_advisor", "评估议题性能影响 (wallclock/资源).", "10 年性能优化", "mock")),
        ("philosophy", CouncilMember::new("philosophy_advisor", "评估议题是否违反 5 哲学守门 (不假装/不欺骗).", "5 年哲学博士", "mock")),
        ("history", CouncilMember::new("history_advisor", "评估议题是否有历史相似案例.", "20 年历史档案员", "mock")),
        ("strategy", CouncilMember::new("strategy_advisor", "评估议题长期价值 vs 短期收益.", "15 年战略顾问", "mock")),
        ("ethics", CouncilMember::new("ethics_advisor", "评估议题是否违反实事求是.", "哲学 + 伦理学双博士", "mock")),
        ("legal", CouncilMember::new("legal_advisor", "评估议题是否触发 L0 HA 司法边界.", "20 年司法从业", "mock")),
    ]
}

#[cfg(feature = "ssr")]
async fn run_council(
    topic: &str,
    context: Option<&str>,
    protocol: &str,
) -> Result<CouncilAdviseResponse, String> {
    let provider = build_mock_llm(protocol);
    run_council_with_provider(topic, context, protocol, provider).await
}

#[cfg(feature = "ssr")]
async fn run_council_with_provider(
    topic: &str,
    context: Option<&str>,
    protocol: &str,
    provider: Box<dyn MockLlmProvider>,
) -> Result<CouncilAdviseResponse, String> {
    let advisors = advisor_list();
    let mut opinions = Vec::new();
    let user_msg = match context {
        Some(ctx) if !ctx.trim().is_empty() => format!("议题: {}\n上下文: {}", topic, ctx),
        _ => format!("议题: {}", topic),
    };

    for (domain, member) in &advisors {
        let sys_prompt = member.to_system_prompt();
        // 把 domain 关键词注入 prompt 头部, 让 ScriptedMockLlm 命中对应分支
        let composed_prompt = format!("[domain: {domain}] {sys_prompt}\n\n{user_msg}");
        let resp = provider.generate(&composed_prompt, &sys_prompt);
        let (stance, reasoning) = parse_advice(&resp.text);
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
        debate_id: None,
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

/// v2 适配: 把 Council 辩论自动存到 web memory (复用 memory::get_store)
#[cfg(feature = "ssr")]
fn save_council_to_memory(
    response: &CouncilAdviseResponse,
    context: Option<&str>,
) -> Result<(), String> {
    use apeireth_storage::memory_episode::{Episode, EpisodeStore};

    let web_arc = apeireth_web::memory::get_store()?;
    let mut web = web_arc.lock().expect("memory mutex poisoned");

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
        metadata: HashMap::new(),
    };
    // v2 EpisodeStore 没有 put_episode, 用 append(e: Episode). append 返回 (), 不需要 map_err.
    web.episodes.append(ep);

    eprintln!(
        "💾 Council 辩论已存 memory: id={}, verdict={}, protocol={}",
        debate_id, response.verdict, response.protocol
    );
    Ok(())
}

// ============================================================
// HTML 模板 (纯 format!, 不依赖 Leptos SSR)
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
    <meta name="description" content="Apeireth Web (v2) — Council 7 advisor 实时辩论 (mock LLM, 后端 apeireth-council)" />
</head>
<body>
    <main class="apeireth-app">
        <header class="apeireth-header">
            <h1>Apeireth Council</h1>
            <p class="apeireth-tagline">7 advisor 真辩论 · mock LLM · Leptos SSR (v2 reconstruction)</p>
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
                <span>LLM 协议 (v2: mock LLM 不同协议不同风格)</span>
                <select class="apeireth-input" name="protocol">
                    <option value="openai" selected>🅾️ OpenAI 协议 (默认 · 偏赞成)</option>
                    <option value="anthropic">🅰️ Anthropic 协议 (偏保守)</option>
                </select>
            </label>
            <button class="apeireth-button" type="submit">
                召唤 Council 7 advisor
            </button>
        </form>

        <div class="apeireth-info">
            <p>📋 流程: 输入议题 → 选协议 → 点击召唤 → 后端跑 7 次 mock LLM (apeireth_council::ScriptedMockLlm) → 显示 7 advisor 立场 + 推理 + 最终 verdict → 自动存到 web memory</p>
            <p>⏱️  预计 <1 秒 (mock LLM, 真 LLM 接入走 apeireth_council::llm_backend::LlmAdvisorBackend)</p>
            <p>🧠 新增: <a href="/memory" style="color:#0969da;">/memory</a> → Episode 时间线</p>
            <p>📚 新增: <a href="/council/history" style="color:#0969da;">/council/history</a> → Council 辩论历史</p>
            <p>🛡️ 新增: <a href="/sovereignty" style="color:#0969da;">/sovereignty</a> → Self-Disable 5 大机制控制台</p>
            <p>📊 新增: <a href="/asi" style="color:#0969da;">/asi</a> → ASI 24 维雷达图</p>
            <p>🛰️ 新增: <a href="/dashboard" style="color:#0969da;">/dashboard</a> → 6 器官综合 Dashboard</p>
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
            </div>"#,
            domain_label = domain_label,
            stance_class = stance_class,
            stance_label = stance_label,
            reasoning = html_escape(&adv.reasoning)
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
            <p class="apeireth-tagline">7 advisor 真辩论 · mock LLM · Leptos SSR (v2)</p>
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
                <h2>Council 7 advisor 辩论结果</h2>
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
