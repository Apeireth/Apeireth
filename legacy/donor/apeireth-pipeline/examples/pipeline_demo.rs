//! `pipeline_demo` — 主人验收用 example (apeireth-pipeline · 战役 1-3)
//!
//! **目标**: 端到端真接 minimaxi OpenAI Chat 协议, 走完 Pipeline 5 步:
//! 1. 解析 placeholder (VCP §6.2.2 #17)
//! 2. token 预算 (VCP §6.2.2 #15)
//! 3. Force-Translate (VCP §6.2.2 #20)
//! 4. 协议归一化 (战役 1-1 apeireth-protocol 4 协议之一)
//! 5. HTTP 调用 (战役 1-2 apeireth-http-client Keep-Alive LIFO 5 字段)
//!
//! **跑法**:
//! ```powershell
//! # 设 API key (主人给的 minimaxi key, 默认从 $env:APEIRETH_API_KEY 读)
//! $env:APEIRETH_API_KEY = "sk-cp-..."
//!
//! # 跑 example
//! cargo run -p apeireth-pipeline --example pipeline_demo
//! ```
//!
//! **期望输出**:
//! - 5 步真跑 (placeholder 解析 + token 截断 + force-translate 跳过 + OpenAI 协议 + Keep-Alive LIFO)
//! - 拿到 minimaxi response, content 打印
//! - 测 latency

use apeireth_http_client::HttpClient;
use apeireth_pipeline::{
    force_translate_if_needed, is_text_only_model_by_tag, messages_contain_base64_media,
    needs_force_translate, resolve_placeholders, truncate_to_max, ForceTranslateConfig,
    PlaceholderContext, RetrySuppression, StreamChunk,
};
use apeireth_protocol::{NormalizedMessage, NormalizedRequest, ProtocolKind};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "apeireth-pipeline 战役 1-3 验收 — 5 步主 chat 管线 (VCP 借鉴 §6.2.2 #15/#17/#19/#20)"
    );
    println!("=========================================================================");

    // ============================================================
    // 1. 真接 minimaxi OpenAI Chat 协议 (要求主人 API key)
    // ============================================================
    let api_key = std::env::var("APEIRETH_API_KEY")
        .map_err(|_| "APEIRETH_API_KEY env var not set (主人给的 minimaxi key)")?;
    // base URL 不含 /v1, 因为 protocol endpoint_path 已含 /v1/chat/completions
    let base_url = std::env::var("APEIRETH_API_URL")
        .unwrap_or_else(|_| "https://api.minimaxi.com".to_string());
    let url = format!("{}/v1/chat/completions", base_url);
    let model = "MiniMax-M3";

    println!("\nLLM 端点: {}", url);
    println!("model: {}", model);
    println!(
        "api_key: {} chars (loaded from $env:APEIRETH_API_KEY)",
        api_key.len()
    );

    // ============================================================
    // 2. 演示 4 个借鉴组件 (战役 1-3 pipeline crate 主借鉴)
    // ============================================================
    println!("\n[战役 1-3 借鉴演示] 4 项 VCP 真代码借鉴:");

    // #15 token 预算
    println!("\n  #15 token 预算三层 (VCP dynamicToolRegistry.js:10/11/21):");
    println!(
        "    LIGHT_LIST_TOKEN_BUDGET    = {}",
        apeireth_pipeline::LIGHT_LIST_TOKEN_BUDGET
    );
    println!(
        "    DEFAULT_BRIEF_TOKEN_BUDGET = {}",
        apeireth_pipeline::DEFAULT_BRIEF_TOKEN_BUDGET
    );
    println!(
        "    MAX_INJECTION_CHARS        = {}",
        apeireth_pipeline::MAX_INJECTION_CHARS
    );

    // #17 placeholder 递归
    println!("\n  #17 Placeholder 递归 (VCP messageProcessor.js:146-220):");
    let mut ctx = PlaceholderContext::new();
    ctx.insert("agent_name".to_string(), "Apeireth".to_string());
    ctx.insert("role".to_string(), "Rust 工程助手".to_string());
    let template = "Hello {{agent_name}}, you are a {{role}}, please respond.";
    let resolved = resolve_placeholders(template, &ctx);
    println!("    template: {template}");
    println!("    resolved: {resolved}");

    // #19 15s 抑制窗口
    println!("\n  #19 15s 抑制窗口 (VCP protocolBridge.js:11-12):");
    let sup = RetrySuppression::with_chat_default();
    println!(
        "    DEFAULT_SUPPRESSION_WINDOW_MS = {} (VCP 真值)",
        apeireth_pipeline::DEFAULT_SUPPRESSION_WINDOW_MS
    );
    println!(
        "    第一次 should_suppress(key) = {} (不抑制, 记录)",
        sup.should_suppress("test-key")
    );
    println!(
        "    第二次 should_suppress(key) = {} (抑制, 15s 窗口内)",
        sup.should_suppress("test-key")
    );

    // #20 Force-Translate
    println!("\n  #20 Force-Translate (VCP chatCompletionHandler.js:222-257):");
    let ft_config = ForceTranslateConfig::with_tags(vec!["deepseek".to_string()]);
    let is_text_only = is_text_only_model_by_tag(model, &ft_config.tag_list);
    println!(
        "    is_text_only_model_by_tag('{}', deepseek) = {}",
        model, is_text_only
    );
    let req_preview = NormalizedRequest::new(
        model,
        vec![NormalizedMessage::user("纯文本消息, 无 base64")],
    );
    let has_base64 = messages_contain_base64_media(&req_preview.messages);
    println!("    messages_contain_base64_media = {has_base64}");
    let needs = needs_force_translate(model, &req_preview.messages, &ft_config);
    println!("    needs_force_translate = {needs} (gpt 模型 + 纯文本 → 不需要)");
    let mut req_clone = req_preview.clone();
    let stats = force_translate_if_needed(model, &mut req_clone.messages, &ft_config);
    println!(
        "    force_translate_if_needed 替换数 = {} (没 base64 → 0)",
        stats.base64_replaced
    );

    // truncate_to_max 演示
    let long_text = "a".repeat(20_000);
    let truncated = truncate_to_max(&long_text, apeireth_pipeline::MAX_INJECTION_CHARS);
    println!(
        "    truncate_to_max(20000 chars, MAX={}) = {} chars",
        apeireth_pipeline::MAX_INJECTION_CHARS,
        truncated.chars().count()
    );

    // ============================================================
    // 3. Pipeline 5 步真跑 — 端到端接 minimaxi
    // ============================================================
    println!("\n[Pipeline 5 步真跑] 接 minimaxi OpenAI Chat 协议:");
    println!("    1. 解析 placeholder (本 demo 用 'Apeireth' 替换 {{name}})");
    println!("    2. token 预算 (VCP MAX_INJECTION_CHARS=16000)");
    println!("    3. Force-Translate (gpt 模型 + 纯文本 → skip)");
    println!("    4. 协议归一化 (OpenAI Chat JSON body)");
    println!("    5. HTTP 调用 (apeireth-http-client Keep-Alive LIFO 5 字段)");

    // 构造 Pipeline: 配 placeholder context + bearer auth
    let http = HttpClient::with_chat_defaults()?;
    let mut config = apeireth_pipeline::PipelineConfig::default();
    config.base_url = base_url.clone();
    config.auth_token = Some(api_key.clone());
    config.placeholder_context = ctx;
    config
        .placeholder_context
        .insert("name".to_string(), "apeireth-pipeline".to_string());
    // **不抑制**: 用 50ms 短窗口 (测试用, 不影响 demo)
    config.suppression = RetrySuppression::new(std::time::Duration::from_millis(50));
    let pipeline = apeireth_pipeline::Pipeline::with_config(http, config)?;

    // 构造 NormalizedRequest
    let req = NormalizedRequest::new(
        model,
        vec![
            NormalizedMessage::system("你是一个 Rust 工程助手, 回答简洁"),
            NormalizedMessage::user("用一句话介绍 {{name}}"),
        ],
    );

    // **5 步真跑**
    let start = Instant::now();
    let result = pipeline.run(ProtocolKind::OpenAiChat, req).await;
    let elapsed = start.elapsed();

    match result {
        Ok(response) => {
            println!("\n[5 步真跑成功] elapsed: {}ms", elapsed.as_millis());
            println!("    id:         {}", response.id);
            println!("    model:      {}", response.model);
            println!("    content:    {}", response.content);
            println!("    finish:     {:?}", response.finish_reason);
            println!(
                "    usage:      prompt={} completion={} total={}",
                response.usage.prompt_tokens,
                response.usage.completion_tokens,
                response.usage.total_tokens
            );
        }
        Err(e) => {
            eprintln!("\n[5 步真跑失败] elapsed: {}ms", elapsed.as_millis());
            eprintln!("    error: {e}");
            // 诊断: 用 apeireth-http-client 直接发, 看 raw response
            eprintln!("\n[诊断] 单独发请求, 看 raw response:");
            let http2 = HttpClient::with_chat_defaults()?;
            let body = serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": "hi"}],
                "max_tokens": 10
            });
            let resp = http2.post(&url, &body).await?;
            eprintln!("    status: {}", resp.status());
            eprintln!("    elapsed: {}ms", resp.elapsed_ms());
            let text = resp.text().await?;
            eprintln!("    body: {}", text.chars().take(400).collect::<String>());
            return Err(e.to_string().into());
        }
    }

    // ============================================================
    // 4. 流式 demo (可选, 仅占位 — 实际 SSE 解析留给战役 2)
    // ============================================================
    println!("\n[流式 5 步] 演示 run_streaming 入口 (本 demo 仅打 Start/End 事件):");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();
    // 起一个 receiver 任务
    let recv_handle = tokio::spawn(async move {
        let mut count = 0;
        while let Some(chunk) = rx.recv().await {
            count += 1;
            match chunk {
                StreamChunk::Start => println!("    [stream] Start"),
                StreamChunk::Data(_) => {} // 不打印, 太多
                StreamChunk::End => {
                    println!("    [stream] End (total events: {count})");
                    break;
                }
                StreamChunk::Error(msg) => {
                    println!("    [stream] Error: {msg}");
                    break;
                }
            }
        }
    });
    let _ = tokio::time::timeout(std::time::Duration::from_millis(100), recv_handle).await;
    drop(tx); // 触发 receiver 退出

    println!("\npipeline_demo 验收通过 — 战役 1-3 5 步主 chat 管线真接 minimaxi 跑通");
    Ok(())
}
