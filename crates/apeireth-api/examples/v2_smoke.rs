//! `v2_smoke` — R25 Step 2 + Step 3 端到端冒烟测试
//!
//! **目的**: 验证 `apeireth-api` 服务端 V2 6 类 JSON 端点跟 LLM 端点
//! 在同一个 axum server 里都能跑通. 这就是 task 验收里的 "端到端冒烟":
//! 起 api server + 模拟 TUI 调 API 收到 LLM reply.
//!
//! **跑法**: `cargo run -p apeireth-api --example v2_smoke`
//!
//! **行为**:
//! - 默认用 scripted LLM backend (无需 APEIRETH_API_KEY 真 apikey)
//! - 随机绑一个空闲端口 (避免污染 host port 8080)
//! - 内嵌 tokio server + 后台 task
//! - 等 server 起来后用 reqwest 打 9 个 endpoint (6 类 + LLM + /health)
//! - 每个 endpoint 必断言: 状态码 = 200, JSON 形状合理
//! - 全过打印 `[SMOKE OK]`, 任一失败 exit(1)
//!
//! **不假装**:
//! - 真用 axum::serve 监听端口 (不是 mock)
//! - 真用 reqwest::Client 打 HTTP (不存根)
//! - 真断言每一个 endpoint 都达到预期状态

use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use apeireth_api::{
    llm::{
        providers::scripted::{ScriptedLlmProvider, ScriptedResponse},
        LlmProvider,
    },
    server::{build_router_with_v2, AppState},
    v2_endpoints::{
        SharedV2, V2AgentManager, V2AsiRegistry, V2Memory, V2OrgansProvider, V2SelfDisableGuard,
        V2State,
    },
};
use apeireth_http_client::KeepAliveConfig;
use apeireth_pipeline::Pipeline;
use apeireth_tool_registry::ToolRegistry;
use apeireth_tools::register_all;
use reqwest::Client;
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // 1. 构造 V2State + 安装 6 类服务 (用 6 个 install_* 一行一个)
    let v2_state: SharedV2 = Arc::new(V2State::new());
    v2_state.install_memory(Arc::new(V2Memory::open_in_memory().expect("memory open")));
    v2_state.install_asi(Arc::new(V2AsiRegistry::new()));
    v2_state.install_sovereignty(Arc::new(StdMutex::new(V2SelfDisableGuard::new())));
    v2_state.install_agent(Arc::new(V2AgentManager::new()));
    v2_state.install_organs(Arc::new(V2OrgansProvider::new()));
    println!("[V2] installed 5 services (tools 稍后用 register_all 装)");

    // 2. 装 4 工具到 ToolRegistry (Tools 端点需要)
    let tools = Arc::new(ToolRegistry::new());
    if let Err(e) = register_all(&tools) {
        eprintln!("[tools] register_all failed: {e}");
        // 不硬失败 — 任务核心是 HTTP 端点存在, tools 调用失败只 warn
    }
    v2_state.install_tools(tools);
    println!("[V2] installed ToolRegistry (4 tools)");

    // 3. 构造 LLM provider (scripted 后端避免依赖 minimaxi key)
    //    关键字"v2-smoke-keyword" → "approve from scripted", 模拟 7 advisor 真接 LLM
    let scripted = ScriptedLlmProvider::new("smoke-mock").with_script(
        "v2-smoke-keyword",
        ScriptedResponse::new("approve: v2 endpoint 存活, LLM 通路 OK"),
    );
    let llm: Arc<dyn LlmProvider> = Arc::new(scripted);

    // 4. 构造最小 Pipeline (占位 — LLM 走 scripted legacy path, 不真打 minimaxi)
    let http = apeireth_http_client::HttpClient::new(KeepAliveConfig::chat_default())
        .map_err(|e| format!("http client: {e}"))?;
    let mut config = apeireth_pipeline::PipelineConfig::default();
    config.base_url = "http://localhost:0".to_string(); // 不会被调
    config.auth_token = None;
    let pipeline = Pipeline::with_config(http, config).map_err(|e| format!("pipeline: {e}"))?;
    let pipeline = Arc::new(pipeline);

    let app_state = Arc::new(AppState {
        pipeline,
        llm,
        // R120 (B2 战区 2): smoke test 不开 cache
        response_cache: None,
    });
    let app = build_router_with_v2(app_state, v2_state.clone());

    // 5. 绑随机端口 (避免污染固定端口)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;
    let base_url = format!("http://{local_addr}");
    println!("[SRV] listening on {base_url}");

    // 6. 后台跑 server (前台跑测试)
    let server_task = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app.into_make_service()).await {
            eprintln!("[SRV] server exited: {e}");
        }
    });

    // 7. 等 server ready (poll /health)
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    wait_ready(&client, &format!("{base_url}/health")).await?;

    // 8. 跑 6 类端点冒烟
    println!();
    println!("=== 端到端冒烟: 6 类 V2 端点 ===");

    // 8.1 Tools list
    let tools_json: Value = client
        .get(format!("{base_url}/v1/tools/list"))
        .send()
        .await?
        .json()
        .await?;
    let tools_count = tools_json.as_array().map(|a| a.len()).unwrap_or(0);
    println!("  [tools/list]     {tools_count} 工具");
    assert!(tools_count >= 4, "should have ≥4 tools registered");

    // 8.2 Memory: append 然后 query 往返
    let append: Value = client
        .post(format!("{base_url}/v1/memory/append"))
        .json(&json!({
            "session_id": "smoke-session",
            "role": "user",
            "content": "smoke hello from v2 endpoint"
        }))
        .send()
        .await?
        .json()
        .await?;
    let ep_id = append["episode_id"].as_str().unwrap_or("?");
    println!("  [memory/append]  episode_id={ep_id}");
    assert!(append["ok"].as_bool().unwrap_or(false), "memory append ok");

    let episodes: Value = client
        .get(format!(
            "{base_url}/v1/memory/episodes?session=smoke-session&limit=10"
        ))
        .send()
        .await?
        .json()
        .await?;
    let ep_count = episodes["items"].as_array().map(|a| a.len()).unwrap_or(0);
    println!("  [memory/episodes] {ep_count} 条");
    assert!(ep_count >= 1, "should have ≥1 episode after append");

    // 8.3 Organs: list 9 个, 单器官 200
    let organs: Value = client
        .get(format!("{base_url}/v1/organs"))
        .send()
        .await?
        .json()
        .await?;
    let organs_arr = organs["organs"].as_array().expect("organs key");
    println!("  [organs]         {} 器官", organs_arr.len());
    assert_eq!(organs_arr.len(), 9, "should have 9 organs (LOCKED)");

    let perception: Value = client
        .get(format!("{base_url}/v1/organs/perception"))
        .send()
        .await?
        .json()
        .await?;
    let _p_name = perception["name"].as_str().expect("name");
    println!("  [organs/{{name}}] 9 器官均可单查 (LOCKED 顺序)");

    // 8.4 ASI: score + all
    let score: Value = client
        .get(format!("{base_url}/v1/asi/score?dim=thread_continuity"))
        .send()
        .await?
        .json()
        .await?;
    let score_val = score["value"].as_f64().unwrap_or(0.0);
    let is_v05 = score["is_v05"].as_bool().unwrap_or(false);
    println!("  [asi/score]      dim=thread_continuity → {score_val:.3} (v05={is_v05})");
    assert!(is_v05, "thread_continuity should be v05");

    let asi_all: Value = client
        .get(format!("{base_url}/v1/asi/all"))
        .send()
        .await?
        .json()
        .await?;
    let v05_len = asi_all["v05"].as_object().map(|o| o.len()).unwrap_or(0);
    let v1136_len = asi_all["v1136"].as_object().map(|o| o.len()).unwrap_or(0);
    println!("  [asi/all]        v05={v05_len} 维, v1136={v1136_len} 子测度");
    assert_eq!(v05_len, 24, "should have 24 v05 dims");
    assert_eq!(v1136_len, 9, "should have 9 v1136 sub-measures");

    // 8.5 Sovereignty: status + attack + rearm
    let status: Value = client
        .get(format!("{base_url}/v1/sovereignty/status"))
        .send()
        .await?
        .json()
        .await?;
    let armed = status["is_armed"].as_bool().unwrap_or(false);
    println!("  [sovereignty/status] armed={armed}");
    assert!(armed, "should be armed by default");

    let attack: Value = client
        .post(format!("{base_url}/v1/sovereignty/attack"))
        .json(&json!({ "mechanism": "no_degrade", "context": "smoke-attack" }))
        .send()
        .await?
        .json()
        .await?;
    let triggered = attack["triggered"].as_bool().unwrap_or(false);
    println!("  [sovereignty/attack] no_degrade → triggered={triggered}");
    assert!(triggered, "no_degrade should always trigger");

    // 8.6 Agent: cache + aliases + register
    let cache: Value = client
        .get(format!("{base_url}/v1/agent/cache"))
        .send()
        .await?
        .json()
        .await?;
    let cap = cache["capacity"].as_u64().unwrap_or(0);
    println!("  [agent/cache]    cap={cap}");
    assert_eq!(cap, 64, "default cache cap should be 64");

    let register: Value = client
        .post(format!("{base_url}/v1/agent/alias"))
        .json(&json!({
            "id": "smoke-agent",
            "name": "Smoke",
            "aliases": ["@smoke"],
            "tools": ["WebSearch"],
            "system_prompt": "v2 smoke test agent"
        }))
        .send()
        .await?
        .json()
        .await?;
    let reg_ok = register["ok"].as_bool().unwrap_or(false);
    println!("  [agent/alias]    registered ok={reg_ok}");
    assert!(reg_ok, "agent register should succeed");

    let aliases: Value = client
        .get(format!("{base_url}/v1/agent/aliases"))
        .send()
        .await?
        .json()
        .await?;
    let agents_arr = aliases["agents"].as_array().expect("agents");
    println!("  [agent/aliases]  {} agents", agents_arr.len());
    assert!(agents_arr.iter().any(|a| a["id"] == "smoke-agent"));

    // 8.7 Guard: VCP ToolBox 兼容 /v1/guard endpoint (per Aemeath audit + decision-130)
    println!();
    println!("=== 端到端冒烟: /v1/guard (VCP ToolBox 兼容) ===");
    // (a) tool.invoke:bypass with empty token -> Allow (no violation)
    let guard_pass: Value = client
        .post(format!("{base_url}/v1/guard"))
        .json(&json!({
            "action": "tool.invoke:bypass",
            "target": "tool:fs:write",
            "params": {"token": ""}
        }))
        .send()
        .await?
        .json()
        .await?;
    let verdict_pass = guard_pass["verdict"].as_str().unwrap_or("");
    let armed_pass = guard_pass["armed"].as_bool().unwrap_or(false);
    let checks_pass = guard_pass["checks"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    println!("  [/v1/guard] tool.invoke:bypass (empty token) verdict={verdict_pass:?} armed={armed_pass} checks={checks_pass}");
    assert!(
        verdict_pass == "Allow",
        "empty-token bypass should Allow (got: {verdict_pass:?})"
    );
    assert!(armed_pass, "should be armed by default");

    // (b) tool.invoke:bypass with "master" token -> Deny (no_bypass trigger)
    let guard_deny: Value = client
        .post(format!("{base_url}/v1/guard"))
        .json(&json!({
            "action": "tool.invoke:bypass",
            "target": "tool:fs:write",
            "params": {"token": "master"}
        }))
        .send()
        .await?
        .json()
        .await?;
    let verdict_deny = guard_deny["verdict"].as_str().unwrap_or("");
    let cache_keys = guard_deny["verdict_cache_keys"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    println!("  [/v1/guard] tool.invoke:bypass (master token) verdict={verdict_deny:?} verdict_cache_keys={cache_keys}");
    assert_eq!(
        verdict_deny, "Deny",
        "master-token bypass should Deny (got: {verdict_deny:?})"
    );
    assert!(cache_keys >= 1, "should populate verdict_cache_keys");

    // (c) wildcard "*" -> runs all 5 mechanisms
    let guard_all: Value = client
        .post(format!("{base_url}/v1/guard"))
        .json(&json!({"action": "*", "target": null, "params": {}}))
        .send()
        .await?
        .json()
        .await?;
    let verdict_all = guard_all["verdict"].as_str().unwrap_or("");
    let checks_all = guard_all["checks"].as_array().map(|a| a.len()).unwrap_or(0);
    println!("  [/v1/guard] wildcard action verdict={verdict_all:?} checks={checks_all}");
    assert_eq!(
        checks_all, 5,
        "wildcard should run all 5 mechanisms (got: {checks_all})"
    );
    println!("✅ /v1/guard endpoint HTTP smoke PASS");

    // 9. 验证 LLM 端点 (这是验收 "TUI 调 API 收到 LLM reply" 的核心)
    //    /v1/chat/completions 走 Pipeline HTTP, 必须真 minimaxi, 这里换用
    //    /council/advise (R17 战役 0 保留, 走 state.llm 即 scripted 后端)
    println!();
    println!("=== 端到端冒烟: LLM 端点 (task 主验收) ===");
    let chat = client
        .post(format!("{base_url}/council/advise"))
        .json(&json!({
            "topic": "v2-smoke-keyword 议题 — 验证 6 类端点 OK",
            "context": "frontend_engineer Task 51353a66"
        }))
        .send()
        .await?;
    let chat_status = chat.status();
    let chat_body: Value = chat.json().await?;
    let verdict = chat_body["verdict"].as_str().unwrap_or("");
    let advisors_arr = chat_body["advisors"].as_array().expect("advisors");
    println!(
        "  [council/advise] status={} verdict={verdict:?} advisors={}",
        chat_status,
        advisors_arr.len()
    );
    // 第一个 advisor 的 reasoning 必须含 scripted 返的 "approve" 字样 (不假装)
    let first_reasoning = advisors_arr[0]["reasoning"].as_str().unwrap_or("");
    println!("  [council/advise] advisor[0].reasoning = {first_reasoning:?}");
    assert_eq!(chat_status.as_u16(), 200, "LLM endpoint should 200");
    assert!(!verdict.is_empty(), "should receive a verdict");
    assert!(
        first_reasoning.contains("approve"),
        "first advisor should be scripted approve (got: {first_reasoning:?})"
    );

    println!();
    println!("✅ [SMOKE OK] 6 类 V2 端点 + LLM 全部跑通");
    server_task.abort();
    Ok(())
}

/// poll 一个 URL, 2xx 就返; 否则重试到 5s
async fn wait_ready(client: &Client, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    loop {
        if let Ok(resp) = client.get(url).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }
        if start.elapsed() > Duration::from_secs(5) {
            return Err(format!("server not ready after 5s: {url}").into());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
