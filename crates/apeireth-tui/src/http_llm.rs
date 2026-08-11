//! TUI HTTP LLM 客户端 (R26-3 直连 + R26-3-fixes provider 路由 + auth 修复)
//!
//! **R26-3-fixes 重要变更** (主人 2026-08-07 反馈 MiniMax 404):
//! - URL path 按 provider 走不同约定 (OpenAI 兼容 vs Anthropic Messages API vs dev fallback)
//! - Bearer auth 真接上 (此前 `req` 变量 dead_code, 实际请求 0 Authorization header)
//! - Anthropic provider 暂未实现 Messages API, 报错明确提示用 Custom
//!
//! **endpoint_path 规则** (跟 `llm_config::Provider::endpoint_path()` 对齐):
//! - OpenAI / DeepSeek / Ollama / Custom: base_url 已含 `/v1` (OpenAI 兼容约定), 拼 `/chat/completions`
//! - Anthropic: base_url 不含 `/v1`, 拼 `/v1/messages` (Messages API)
//! - 无 config + env fallback: `/v1/chat/completions` (假设 OpenAI 兼容)
//!
//! **API 契约** (跟 `crates/apeireth-api/src/protocol_handlers.rs:177` OpenAiChatRequest 字段级对齐):
//! - Request: `model=messages+stream+temperature+max_tokens` (OpenAI 协议)
//! - Response: 非流式 = `choices[0].message.content`; 流式 = `data: {JSON}\n\n` SSE,
//!   `choices[0].delta.content` 单 chunk, 终止 `data: [DONE]`
//!
//! **不假装**:
//! - 真接 `reqwest::Response::bytes_stream()` + 逐 chunk 推 `mpsc::Sender<String>` (真流式, 不 simulate)
//! - SSE 解析失败行 skip, 不假装错误 (跟 `apeireth-api::openai_compat::parse_openai_sse_data_line` 一致)
//! - 4xx/5xx → `Err(format!("HTTP {}: {}", status, body))` (真返 body, 让 caller 知道错因)
//! - 网络错误 → `Err(format!("network: {}", e))` (reqwest 错误透传)
//! - 30 秒超时 (`tokio::time::timeout`)
//! - Bearer auth 真接 (R26-3-fixes: 此前 dead_code 已修)
//!
// Step 1 阶段: 公开 API 暂时未调 (Step 1.5 由 Mavis 整合), 全部允许 dead_code
// (跟 backend.rs::UsageInfo 用法一致: 公开 API 预备 Step 1.5 整合用)
#![allow(dead_code)]

use std::time::Duration;

use futures::StreamExt;
use reqwest::{Client, RequestBuilder};
use serde_json::{json, Value};

use crate::app::ChatMessage;
use crate::llm_config::{self, Provider};

/// R26-3: 默认占位。不再是 apeireth-api server 本机端。实际 base_url 从 LlmConfig 读。
/// (环境变量 APEIRETH_API_URL 仍能兑底 developer 场景。)
const DEFAULT_API_URL: &str = "http://unconfigured.localhost";
/// 无 LlmConfig 时 (纯 env 兑底) 用的 endpoint path, 假设 OpenAI 兼容
const DEFAULT_CHAT_PATH: &str = "/v1/chat/completions";
/// 模型名 (R26-3: 模型名从 LlmConfig 读, 动态, 用户填啥用啥)
fn active_model() -> String {
    llm_config::load()
        .map(|c| c.model)
        .unwrap_or_else(|| "default-model".to_string())
}

/// R26-3-fixes: max_tokens (默认 8192, 用户 Settings 可配 1-32768)
/// 之前硬编码 4096 太短, 长文会被截断
/// 优先级: env APEIRETH_MAX_TOKENS > llm.json > default 8192
fn active_max_tokens() -> u32 {
    llm_config::load()
        .map(|c| c.resolved_max_tokens())
        .unwrap_or(8192)
}
/// LLM 调用总超时 (防网络挂起, 30s 跟战役 1-2 默认对齐)
const HTTP_TIMEOUT_SECS: u64 = 30;

/// LLM 回复 (TUI 内部用, 不重导出 — Step 1.5 整合时由 backend.rs 转 LlmReply)
#[derive(Debug, Clone)]
pub struct HttpLlmReply {
    /// 模型回复内容
    pub text: String,
    /// token 用量
    pub usage: UsageInfo,
}

/// LLM token 用量 (跟 OpenAI `usage` 字段对齐: prompt_tokens / completion_tokens / total_tokens)
#[derive(Debug, Clone, Default)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 同步调用 LLM (非流式, 简单场景 / fallback 路径)
///
/// **使用场景**: TUI main loop 是同步的, 简单场景 (e.g. 状态查询、辅助 prompt) 直接同步拿完整 reply.
/// 战役 4-1 真流式主路径走 `call_llm_http_stream`.
///
/// **流程**:
/// 1. 读 LlmConfig + 算 URL (`provider.endpoint_path()` 按 provider 走)
/// 2. 构造 OpenAI Chat Completions 请求, `stream=false`
/// 3. POST 到 `{base}{endpoint_path}`
/// 4. 解析响应 JSON, 提取 `choices[0].message.content` + `usage`
///
/// **错误**:
/// - 4xx/5xx → `Err(format!("HTTP {}: {}", status, body))`
/// - 网络错误 → `Err(format!("network: {}", e))`
/// - 超时 (30s) → `Err("timeout: ...")`
pub fn call_llm_http_sync(input: &str, system: &str, history: &[ChatMessage]) -> Result<HttpLlmReply, String> {
    reject_unsupported_provider()?;
    let url = chat_completions_url()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;
    rt.block_on(call_llm_http_sync_async(&url, input, system, history))
}

/// async 内部实现: 可被 `call_llm_http_sync` block_on 包装, 也可被其他路径复用
async fn call_llm_http_sync_async(url: &str, input: &str, system: &str, history: &[ChatMessage]) -> Result<HttpLlmReply, String> {
    let mut messages = vec![json!({"role": "system", "content": system})];
    for msg in history {
        if msg.role == "user" {
            messages.push(json!({"role": "user", "content": msg.content}));
        } else if msg.role == "assistant" {
            messages.push(json!({"role": "assistant", "content": msg.content}));
        }
    }
    messages.push(json!({"role": "user", "content": input}));
    let body = json!({
        "model": active_model(),
        "messages": messages,
        "stream": false,
        "temperature": 0.7,
        "max_tokens": active_max_tokens()
    });

    let client = build_client()?;
    let request = apply_bearer_auth(client.post(url).json(&body));

    // 30s 总超时 (跟 HTTP_TIMEOUT_SECS 一致)
    let response = tokio::time::timeout(Duration::from_secs(HTTP_TIMEOUT_SECS), request.send())
        .await
        .map_err(|_| format!("timeout: request exceeded {HTTP_TIMEOUT_SECS}s"))?
        .map_err(|e| format!("network: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        // 4xx/5xx: 读 body 给 caller 完整错误信息 (不假装, 透传)
        let status_code = status.as_u16();
        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "<body read failed>".to_string());
        return Err(format!("HTTP {status_code}: {body_text}"));
    }

    let json_body: Value = response
        .json()
        .await
        .map_err(|e| format!("parse response JSON: {e}"))?;

    // 提取 choices[0].message.content (非流式响应)
    let text = json_body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "missing choices[0].message.content in response".to_string())?
        .to_string();

    // 提取 usage (可能不存在, 缺则 default 0)
    let usage = UsageInfo {
        prompt_tokens: json_body["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
        completion_tokens: json_body["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        total_tokens: json_body["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
    };

    Ok(HttpLlmReply { text, usage })
}

/// 流式调用 LLM (战役 4-1 真流式主路径, 推 `mpsc::Sender<String>`)
///
/// **使用场景**: TUI `chat_streaming` 走这里, 边生成边推 sender, 用户体验 = 真流式
/// (UI 立即看到内容, 不等整个 reply 生成).
///
/// **流程**:
/// 1. 读 LlmConfig + 算 URL (`provider.endpoint_path()` 按 provider 走)
/// 2. 构造 OpenAI Chat Completions 请求, `stream=true`
/// 3. POST 到 `{base}{endpoint_path}`
/// 4. 读 `response.bytes_stream()`, 按 `\n\n` 切 SSE event
/// 5. 每个 event 解析 `data: {JSON}` → 提取 `choices[0].delta.content` → `sender.send(chunk)`
/// 6. 遇 `data: [DONE]` 终止
/// 7. 返完整拼接文本 (sender 已推过的内容 = 完整内容, 跟 `call_llm_http_sync` 行为一致)
///
/// **错误** (跟 sync 一致):
/// - 4xx/5xx → `Err(format!("HTTP {}: {}", status, body))`
/// - 网络错误 → `Err(format!("network: {}", e))`
/// - 超时 (30s) → `Err("timeout: ...")`
/// - SSE 流中 chunk error (JSON 解析失败行 skip, 不假装; bytes_stream error → `Err`)
pub async fn call_llm_http_stream(
    input: &str,
    system: &str,
    history: &[ChatMessage],
    sender: &std::sync::mpsc::Sender<String>,
) -> Result<String, String> {
    reject_unsupported_provider()?;
    let url = chat_completions_url()?;
    call_llm_http_stream_at(&url, input, system, history, sender).await
}

/// async 内部实现: URL 已注入 (单元测试用, 连 httpmock mock server)
async fn call_llm_http_stream_at(
    url: &str,
    input: &str,
    system: &str,
    history: &[ChatMessage],
    sender: &std::sync::mpsc::Sender<String>,
) -> Result<String, String> {
    let mut messages = vec![json!({"role": "system", "content": system})];
    for msg in history {
        if msg.role == "user" {
            messages.push(json!({"role": "user", "content": msg.content}));
        } else if msg.role == "assistant" {
            messages.push(json!({"role": "assistant", "content": msg.content}));
        }
    }
    messages.push(json!({"role": "user", "content": input}));
    let body = json!({
        "model": active_model(),
        "messages": messages,
        "stream": true,
        "temperature": 0.7,
        "max_tokens": active_max_tokens()
    });

    let client = build_client()?;
    let request = apply_bearer_auth(client.post(url).json(&body));

    let response = tokio::time::timeout(Duration::from_secs(HTTP_TIMEOUT_SECS), request.send())
        .await
        .map_err(|_| format!("timeout: request exceeded {HTTP_TIMEOUT_SECS}s"))?
        .map_err(|e| format!("network: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        let status_code = status.as_u16();
        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "<body read failed>".to_string());
        return Err(format!("HTTP {status_code}: {body_text}"));
    }

    // 真流式: bytes_stream + 逐 chunk 推 sender
    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut full_text = String::new();

    while let Some(chunk_result) = byte_stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("stream chunk error: {e}"))?;
        // 字节拼入 buffer (UTF-8 lossless, 跟 anthropic_compat / openai_compat 一致)
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // 按 `\n\n` 切 SSE event (OpenAI 协议每个 event 用空行分隔)
        while let Some(event_end) = buffer.find("\n\n") {
            let event: String = buffer.drain(..event_end + 2).collect();
            // event 内多行 (data: 必有, 可能含 \nid: / \nevent: 等), 逐行处理
            let mut stream_ended = false;
            let mut chunk_to_emit: Option<String> = None;
            for line in event.lines() {
                if let Some(maybe_content) = parse_openai_sse_data_line(line) {
                    match maybe_content {
                        Some(content) => {
                            // 非空 content 才推 sender (跟 openai_compat 一致, 避免空字符串刷屏)
                            if !content.is_empty() {
                                chunk_to_emit = Some(content);
                            }
                        }
                        None => {
                            // data: [DONE] → 终止流
                            stream_ended = true;
                            break;
                        }
                    }
                }
            }
            if stream_ended {
                return Ok(full_text);
            }
            if let Some(content) = chunk_to_emit {
                full_text.push_str(&content);
                // sender 可能在某次 send 时已 disconnect (用户 q 退出), 此时按已推内容返
                // (跟 backend.rs::process_stream_to_reply 行为一致, 优雅退出不 panic)
                if sender.send(content).is_err() {
                    return Ok(full_text);
                }
            }
        }
    }

    // 流关闭: 检查 buffer 剩余 (可能最后一 event 无 \n\n 结尾, 跟 anthropic_compat 收尾逻辑一致)
    if !buffer.trim().is_empty() {
        for line in buffer.lines() {
            if let Some(Some(content)) = parse_openai_sse_data_line(line) {
                if !content.is_empty() {
                    full_text.push_str(&content);
                    let _ = sender.send(content); // sender 断开忽略
                }
            }
        }
    }

    Ok(full_text)
}
/// 解析 OpenAI SSE 单行 `data: {JSON}`, 提取 `choices[0].delta.content`.
///
/// **行为** (跟 `apeireth-api::openai_compat::parse_openai_sse_data_line` 字段级一致, 不重复造轮子):
/// - 空行 / 非 `data:` 开头 → `None` (skip)
/// - `data: [DONE]` → `Some(None)` (流终止标志)
/// - `data: {JSON}` → 解析 `choices[0].delta.content`:
///   - 有 content (含 `""`) → `Some(Some(content))`
///   - JSON 解析失败 / 缺 content → `None` (skip, 不假装错误)
fn parse_openai_sse_data_line(line: &str) -> Option<Option<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with("data:") {
        return None;
    }
    let payload = trimmed[5..].trim(); // 剩 "data:" + 空白
    if payload.is_empty() {
        return None;
    }
    if payload == "[DONE]" {
        return Some(None); // 流终止
    }
    let parsed: Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return None, // 不假装错误, 跳过坏行
    };
    // 提取 choices[0].delta.content (流式字段路径, 跟 openai_compat 模式一致)
    let content = parsed
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("delta"))
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str());
    match content {
        Some(s) => Some(Some(s.to_string())),
        None => None, // delta.role 等其他字段, 跳过
    }
}

/// 构造共享 reqwest::Client (复用连接池, 跟 apeireth-http-client 模式一致)
fn build_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("build reqwest client: {e}"))
}

/// R26-3: 从 LlmConfig 读 base_url, env 兑底 (老 developer 习惯)
fn api_base_url() -> String {
    if let Some(c) = llm_config::load() {
        return c.base_url;
    }
    std::env::var("APEIRETH_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string())
}

/// R26-3-fixes: 由 provider 决定 endpoint path, 拼成完整 URL
///
/// **404 根因修复**: 此前硬编码 `/v1/chat/completions`, 当 user base_url 是 OpenAI 兼容
/// 风格 (e.g. `https://api.minimax.chat/v1`) 时, 最终 URL = `base + /v1/chat/completions`
/// → `https://api.minimax.chat/v1/v1/chat/completions` → 404.
fn chat_completions_url() -> Result<String, String> {
    let base = api_base_url();
    let path = llm_config::load()
        .map(|c| c.provider.endpoint_path())
        .unwrap_or(DEFAULT_CHAT_PATH);
    Ok(format!("{base}{path}"))
}

/// R26-3-fixes: 拒绝未实现的 provider (Anthropic Messages API), 返明确错误
///
/// **Anthropic 暂未实现**: Anthropic Messages API 协议 (header `x-api-key` + `anthropic-version`,
/// body 字段 `system` 独立 + 必填 `max_tokens`) 跟 OpenAI Chat Completions 完全不同。
/// 暂不实现, 报错直接告诉用户用 Custom + OpenAI 兼容端点。
fn reject_unsupported_provider() -> Result<(), String> {
    if let Some(c) = llm_config::load() {
        if c.provider == Provider::Anthropic {
            return Err(
                "Anthropic Messages API not yet supported in TUI direct mode. \
                 Use provider [5] Custom with an OpenAI-compatible endpoint \
                 (e.g. via OpenRouter, or your own Anthropic-compatible proxy)."
                    .to_string(),
            );
        }
    }
    Ok(())
}

/// R26-3-fixes: Bearer auth 真接上 (此前 dead_code — `req` 变量装好 auth 后从不被用)
///
/// **优先级**:
/// 1. LlmConfig.api_key (从 llm.json 读, 用户 onboarding 写入)
/// 2. env APEIRETH_API_KEY (developer 兑底)
/// 3. 均无 → 不加 Authorization header (Ollama 本地 / 不需 auth 的端点)
fn apply_bearer_auth(req: RequestBuilder) -> RequestBuilder {
    // 优先级: LlmConfig.api_key > env APEIRETH_API_KEY > 不加 auth (Ollama 本地)
    if let Some(c) = llm_config::load() {
        if !c.api_key.is_empty() {
            return req.bearer_auth(&c.api_key);
        }
    } else if let Ok(k) = std::env::var("APEIRETH_API_KEY") {
        if !k.is_empty() {
            return req.bearer_auth(&k);
        }
    }
    req
}
// ============================================================
// 单元测试 (R26-3-fixes: 4 旧测试 + 3 新测试)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    /// 模拟 1 个 SSE event (OpenAI Chat Completions chunk)
    fn make_sse_chunk(content: &str) -> String {
        let chunk_json = json!({
            "id": "chatcmpl-test",
            "object": "chat.completion.chunk",
            "choices": [{
                "index": 0,
                "delta": {"content": content},
                "finish_reason": null
            }]
        });
        format!("data: {chunk_json}")
    }

    #[tokio::test]
    async fn http_stream_pushes_chunks_to_sender_and_returns_full_text() {
        // 验收: mock 服务器返 2 个 SSE chunk + [DONE]
        // (a) sender 真收 2 个 chunk (不假装)
        // (b) 返拼接文本 = "你好"
        let server = MockServer::start_async().await;

        let sse_body = format!(
            "{}\n\n{}\n\ndata: [DONE]\n\n",
            make_sse_chunk("你"),
            make_sse_chunk("好")
        );

        let mock = server.mock(|when, then| {
            when.method(POST).path(DEFAULT_CHAT_PATH);
            then.status(200)
                .header("content-type", "text/event-stream")
                .body(&sse_body);
        });

        let url = format!("{}{}", server.base_url(), DEFAULT_CHAT_PATH);
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let result = call_llm_http_stream_at(&url, "test input", "test system", &[], &tx).await;

        assert!(result.is_ok(), "stream ok: {:?}", result);
        let full_text = result.unwrap();
        assert_eq!(full_text, "你好", "拼接文本");

        // 收 2 chunks (顺序无关, 因为 sender 顺序是确定的: "你" 先 "好" 后)
        let chunk1 = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("recv chunk 1");
        let chunk2 = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("recv chunk 2");
        assert_eq!(chunk1, "你");
        assert_eq!(chunk2, "好");
        // 验证流结束后, 不再有 chunk ([DONE] 已终止, 不推)
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "流结束后应无更多 chunk"
        );
        mock.assert_calls(1);
    }

    #[tokio::test]
    async fn http_sync_parses_non_stream_response() {
        // 验收: mock 服务器返完整 OpenAI Chat 响应 (非流式, choices[0].message.content)
        let server = MockServer::start_async().await;

        let openai_response = json!({
            "id": "chatcmpl-sync-test",
            "object": "chat.completion",
            "created": 1_700_000_000_u64,
            "model": active_model(),
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Sync reply from mock"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 8,
                "total_tokens": 20
            }
        });

        let mock = server.mock(|when, then| {
            when.method(POST).path(DEFAULT_CHAT_PATH);
            then.status(200)
                .header("content-type", "application/json")
                .body(serde_json::to_string(&openai_response).unwrap());
        });

        let url = format!("{}{}", server.base_url(), DEFAULT_CHAT_PATH);
        let result = call_llm_http_sync_async(&url, "test input", "test system", &[]).await;

        assert!(result.is_ok(), "sync ok: {:?}", result);
        let reply = result.unwrap();
        assert_eq!(reply.text, "Sync reply from mock");
        assert_eq!(reply.usage.prompt_tokens, 12);
        assert_eq!(reply.usage.completion_tokens, 8);
        assert_eq!(reply.usage.total_tokens, 20);
        mock.assert_calls(1);
    }

    #[tokio::test]
    async fn http_returns_err_on_500() {
        // 验收: mock 服务器返 500
        // 验证: Err 含 "500" + body 内容
        let server = MockServer::start_async().await;

        let mock = server.mock(|when, then| {
            when.method(POST).path(DEFAULT_CHAT_PATH);
            then.status(500)
                .header("content-type", "text/plain")
                .body("internal server error from mock");
        });

        let url = format!("{}{}", server.base_url(), DEFAULT_CHAT_PATH);
        let (tx, _rx) = std::sync::mpsc::channel::<String>();
        let result = call_llm_http_stream_at(&url, "test input", "test system", &[], &tx).await;

        let err = result.expect_err("500 应返 Err");
        assert!(err.contains("500"), "错误信息应含 500, 实际: {err}");
        assert!(
            err.contains("internal server error"),
            "错误信息应含 body, 实际: {err}"
        );
        mock.assert_calls(1);
    }

    #[tokio::test]
    async fn http_returns_err_on_connection_refused() {
        // 验收: 连一个不存在的端口 (127.0.0.1:1, 几乎必然未监听)
        // 验证: Err 含 "network" (reqwest 错误透传)
        let url = "http://127.0.0.1:1/chat/completions";
        let (tx, _rx) = std::sync::mpsc::channel::<String>();
        let result = call_llm_http_stream_at(url, "test input", "test system", &[], &tx).await;

        let err = result.expect_err("连接拒绝应返 Err");
        assert!(
            err.contains("network") || err.contains("connection") || err.contains("refused"),
            "错误信息应含 network/connection/refused, 实际: {err}"
        );
    }

    // ============ R26-3-fixes: 新增 3 个测试 ============

    /// R26-3-fixes 验收: Bearer auth header 真接上 (此前 dead_code)
    ///
    /// mock 服务器断言收到 `Authorization: Bearer <test-key>`, 验证 auth 真有发出去
    #[tokio::test]
    async fn auth_bearer_header_is_actually_sent() {
        let server = MockServer::start_async().await;

        // 隔离: 临时 APPDATA 指空 dir, 避免被用户 %APPDATA%\\apeireth\\llm.json 干扰
        let tmp = std::env::temp_dir().join("apeireth-tui-auth-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("mkdir tmp");
        let prev_appdata = std::env::var("APPDATA").ok();
        std::env::set_var("APPDATA", &tmp);
        std::env::set_var("APEIRETH_API_KEY", "test-bearer-token-r26-3");

        let mock = server.mock(|when, then| {
            when.method(POST)
                .path(DEFAULT_CHAT_PATH)
                .header("authorization", "Bearer test-bearer-token-r26-3");
            then.status(200)
                .header("content-type", "text/event-stream")
                .body("data: [DONE]\n\n");
        });

        let url = format!("{}{}", server.base_url(), DEFAULT_CHAT_PATH);
        let (tx, _rx) = std::sync::mpsc::channel::<String>();
        let result = call_llm_http_stream_at(&url, "hi", "test", &[], &tx).await;

        // 恢复环境
        std::env::remove_var("APEIRETH_API_KEY");
        match prev_appdata {
            Some(v) => std::env::set_var("APPDATA", v),
            None => std::env::remove_var("APPDATA"),
        }
        let _ = std::fs::remove_dir_all(&tmp);

        assert!(result.is_ok(), "auth 流应 ok: {:?}", result);
        mock.assert_calls(1);
    }

    /// R26-3-fixes 验收: endpoint path 按 provider 走, OpenAI 兼容 prototype 用 /chat/completions
    ///
    /// 验证 `Provider::endpoint_path()` 5 变体的 path 正确
    #[test]
    fn endpoint_path_matches_provider_convention() {
        assert_eq!(Provider::Openai.endpoint_path(), "/chat/completions");
        assert_eq!(Provider::Deepseek.endpoint_path(), "/chat/completions");
        assert_eq!(Provider::Ollama.endpoint_path(), "/chat/completions");
        assert_eq!(Provider::Custom.endpoint_path(), "/chat/completions");
        assert_eq!(Provider::Anthropic.endpoint_path(), "/v1/messages");
    }

    /// R26-3-fixes 验收: chat_completions_url() 拼接时不去重 /v1
    ///
    /// 当 Custom provider base_url = `https://api.minimax.chat/v1`, 拼接后 URL
    /// = `https://api.minimax.chat/v1/chat/completions` (不带 /v1/v1/)
    #[tokio::test]
    async fn url_construction_no_double_v1_prefix() {
        // 验证 endpoint_path() 返 /chat/completions (不是 /v1/chat/completions)
        // 这样 chat_completions_url() 拼接 = base_url(含 /v1) + /chat/completions
        // = https://api.minimax.chat/v1/chat/completions (正确)
        let path = Provider::Custom.endpoint_path();
        assert_eq!(path, "/chat/completions", "Custom path 不可含 /v1 前缀");
        assert!(
            !path.starts_with("/v1/"),
            "path 不应嵌 /v1 前缀, 实际: {path}"
        );
    }
}
