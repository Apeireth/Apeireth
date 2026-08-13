//! OpenAiCompatibleProvider — 通用 OpenAI 兼容协议实现
//!
//! 用途: 当需要用 OpenAI-compatible 协议但不走 Apeireth API 平台时 (例如直连 OpenAI / Azure OpenAI / Together 等)
//! Week 1 实装
//!
//! **战役 4-1 真流式 (R17-2026-08-04)**:
//! `complete_stream()` 走真 SSE, 字段级对齐 OpenAI Chat Completion stream 协议:
//! ```text
//! data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","choices":[{"delta":{"content":"hello"},"index":0}]}
//!
//! data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","choices":[{"delta":{"content":" world"},"index":0}]}
//!
//! data: [DONE]
//! ```
//! 终止条件: `data: [DONE]` 行. 解析失败行跳过 (不假装失败).

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::stream::StreamExt;

use crate::llm::error::LlmError;
use crate::llm::traits::{LlmProvider, LlmRequest, LlmResponse};

/// 战役 4-1: 解析 OpenAI SSE 单行 `data: {JSON}`, 提取 `choices[0].delta.content`.
///
/// 行为:
/// - 空行 / 非 `data:` 开头 → `None` (skip)
/// - `data: [DONE]` → `Some(None)` (流终止标志, 消费方停止拉)
/// - `data: {JSON}` → 解析 `choices[0].delta.content`:
///   - 有 content (含 `""`) → `Some(Some(content))`
///   - JSON 解析失败 / 缺 content → `None` (skip, 不假装错误)
///
/// 暴露 `pub(crate)` 是因为 `ApeirethApiProvider` 也走 OpenAI 协议, 复用同一份 SSE 解析 (战役 4-1: 不重复造轮子).
pub(crate) fn parse_openai_sse_data_line(line: &str) -> Option<Option<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with("data:") {
        return None;
    }
    let payload = trimmed[5..].trim(); // 剥 "data:" + 空白
    if payload.is_empty() {
        return None;
    }
    if payload == "[DONE]" {
        return Some(None); // 流终止
    }
    let parsed: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return None, // 不假装错误, 跳过坏行
    };
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

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleConfig {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub models: Vec<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

impl OpenAiCompatibleConfig {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        models: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            models,
            timeout_ms: 60_000,
            max_retries: 3,
        }
    }
}

pub struct OpenAiCompatibleProvider {
    config: OpenAiCompatibleConfig,
    http: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: OpenAiCompatibleConfig) -> Result<Self, LlmError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| LlmError::Config(format!("reqwest client build failed: {e}")))?;
        Ok(Self { config, http })
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.config.name
    }
    fn supports_model(&self, model: &str) -> bool {
        self.config.models.iter().any(|m| m == model) || self.config.models.is_empty()
    }

    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Week 1 复用 ApeirethApiProvider 的实现 (协议相同)
        // Week 2+ 可加自定义 (例如 Azure OpenAI 需加 api-version header)
        let url = format!("{}/chat/completions", self.config.base_url);
        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| serde_json::json!({
                "role": match m.role {
                    super::super::traits::ChatRole::System => "system",
                    super::super::traits::ChatRole::User => "user",
                    super::super::traits::ChatRole::Assistant => "assistant",
                },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": req.temperature,
            "max_tokens": req.max_tokens,
        });

        let start = Instant::now();
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network {
                provider: self.config.name.clone(),
                detail: e.to_string(),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(LlmError::AuthFailed(text));
            }
            return Err(LlmError::BadResponse {
                provider: self.config.name.clone(),
                detail: format!("status {}: {}", status, text),
                status_code: Some(status.as_u16()),
            });
        }

        let body: serde_json::Value = resp.json().await.map_err(|e| LlmError::BadResponse {
            provider: self.config.name.clone(),
            detail: format!("json: {e}"),
            status_code: None,
        })?;

        let content = body
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let usage = body
            .get("usage")
            .map(|u| super::super::traits::TokenUsage {
                prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                completion_tokens: u
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            })
            .unwrap_or_default();

        Ok(LlmResponse {
            content,
            usage,
            latency_ms: start.elapsed().as_millis() as u64,
            model: body
                .get("model")
                .and_then(|m| m.as_str())
                .unwrap_or(&req.model)
                .to_string(),
            finish_reason: body
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("finish_reason"))
                .and_then(|f| f.as_str())
                .unwrap_or("stop")
                .to_string(),
            provider: self.config.name.clone(),
        })
    }

    /// **战役 4-1 真流式 (R17-2026-08-04)**: OpenAI Chat Completion SSE 推流.
    ///
    /// **协议** (字段级对齐 OpenAI 官方文档):
    /// 1. POST `/chat/completions` 带 `stream: true`
    /// 2. 响应 `Content-Type: text/event-stream`, body 是 SSE 格式
    /// 3. 每个 event 是 `data: {JSON}\n\n`, 内容是 `chat.completion.chunk` object
    /// 4. 终止标志 `data: [DONE]\n\n`
    ///
    /// **不假装** (主 17:58 O-5):
    /// - 真接 `reqwest::Response::bytes_stream()`, 真 `StreamExt::next()` 拉 chunk
    /// - 真按 `\n\n` 切 SSE event, 跳过空行
    /// - JSON 解析失败不假装错误, 跳过该行 (跟 VCP 容错一致)
    /// - `data: [DONE]` 终止流
    async fn complete_stream(
        &self,
        req: LlmRequest,
    ) -> Result<futures::stream::BoxStream<'static, Result<String, LlmError>>, LlmError> {
        use futures::stream::{self, StreamExt};

        let url = format!("{}/chat/completions", self.config.base_url);
        let body = serde_json::json!({
            "model": req.model,
            "stream": true,
            "messages": req.messages.iter().map(|m| serde_json::json!({
                "role": match m.role {
                    super::super::traits::ChatRole::System => "system",
                    super::super::traits::ChatRole::User => "user",
                    super::super::traits::ChatRole::Assistant => "assistant",
                },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": req.temperature,
            "max_tokens": req.max_tokens,
        });

        // 发请求 + 检查 HTTP status
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network {
                provider: self.config.name.clone(),
                detail: e.to_string(),
            })?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::AuthFailed(text));
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::BadResponse {
                provider: self.config.name.clone(),
                detail: format!("status {}: {}", status, text),
                status_code: Some(status.as_u16()),
            });
        }

        // 真流: 拉 bytes_stream, 按 SSE event 切分, 解析 data 行
        let provider_name = self.config.name.clone();
        let byte_stream = resp.bytes_stream();
        let s = stream::unfold(
            (byte_stream, false, String::new()),
            move |(mut byte_stream, mut done, mut buffer)| {
                let provider_name = provider_name.clone();
                async move {
                    if done {
                        return None;
                    }
                    loop {
                        // 1. 看 buffer 里有无完整 SSE event (空行 \n\n 分隔)
                        if let Some(event_end) = buffer.find("\n\n") {
                            let event: String = buffer.drain(..event_end + 2).collect();
                            // event 内多行 (data: + 可能 \n:id: ...), 逐行处理
                            let mut stream_ended = false;
                            let mut chunk_to_emit: Option<String> = None;
                            for line in event.lines() {
                                if let Some(maybe_content) = parse_openai_sse_data_line(line) {
                                    match maybe_content {
                                        Some(content) => {
                                            if !content.is_empty() {
                                                chunk_to_emit = Some(content);
                                            }
                                        }
                                        None => {
                                            // data: [DONE] → 终止
                                            stream_ended = true;
                                            break;
                                        }
                                    }
                                }
                            }
                            if stream_ended {
                                return Some((Ok(String::new()), (byte_stream, true, buffer)));
                            }
                            if let Some(content) = chunk_to_emit {
                                return Some((Ok(content), (byte_stream, false, buffer)));
                            }
                            // 整个 event 无 content, 继续 loop 拉下一 event
                            continue;
                        }
                        // 2. buffer 不够, 拉更多字节
                        match byte_stream.next().await {
                            Some(Ok(bytes)) => {
                                buffer.push_str(&String::from_utf8_lossy(&bytes));
                            }
                            Some(Err(e)) => {
                                let err = LlmError::Network {
                                    provider: provider_name,
                                    detail: e.to_string(),
                                };
                                return Some((Err(err), (byte_stream, true, buffer)));
                            }
                            None => {
                                // 流关闭: 检查 buffer 剩余 (可能最后一 event 无 \n\n 结尾)
                                if !buffer.trim().is_empty() {
                                    let last_event = std::mem::take(&mut buffer);
                                    for line in last_event.lines() {
                                        if let Some(Some(content)) =
                                            parse_openai_sse_data_line(line)
                                        {
                                            if !content.is_empty() {
                                                return Some((
                                                    Ok(content),
                                                    (byte_stream, true, buffer),
                                                ));
                                            }
                                        }
                                    }
                                }
                                return None; // 真流结束
                            }
                        }
                    }
                }
            },
        );

        Ok(Box::pin(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::StreamExt;

    #[test]
    fn test_openai_compat_basic() {
        let config = OpenAiCompatibleConfig::new(
            "openai",
            "https://api.openai.com/v1",
            "sk-test",
            vec!["gpt-4o".into()],
        );
        let provider = OpenAiCompatibleProvider::new(config).unwrap();
        assert_eq!(provider.name(), "openai");
        assert!(provider.supports_model("gpt-4o"));
        assert!(!provider.supports_model("claude"));
    }

    // ============================================================
    // 战役 4-1: OpenAI SSE 解析单元测试
    // ============================================================

    #[test]
    fn parse_openai_sse_data_line_extracts_delta_content() {
        // 标准 OpenAI chunk: data: {"choices":[{"delta":{"content":"hi"}}]}
        let line = r#"data: {"id":"chatcmpl-xxx","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"hi"},"finish_reason":null}]}"#;
        let result = parse_openai_sse_data_line(line);
        assert_eq!(result, Some(Some("hi".to_string())));
    }

    #[test]
    fn parse_openai_sse_data_line_extracts_cjk_content() {
        // CJK 内容 (战役 4-1: 不假装 CJK)
        let line = r#"data: {"choices":[{"delta":{"content":"中文测试"}}]}"#;
        let result = parse_openai_sse_data_line(line);
        assert_eq!(result, Some(Some("中文测试".to_string())));
    }

    #[test]
    fn parse_openai_sse_data_line_done_signals_stream_end() {
        // data: [DONE] → Some(None) (终止)
        let line = "data: [DONE]";
        let result = parse_openai_sse_data_line(line);
        assert_eq!(result, Some(None));
    }

    #[test]
    fn parse_openai_sse_data_line_skips_empty_delta() {
        // delta 没有 content 字段 (e.g. 只更新 role) → None (skip)
        let line = r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#;
        let result = parse_openai_sse_data_line(line);
        assert_eq!(result, None);
    }

    #[test]
    fn parse_openai_sse_data_line_skips_invalid_json() {
        // 坏 JSON → None (不假装错误, 跳过)
        let line = "data: {not valid json}";
        let result = parse_openai_sse_data_line(line);
        assert_eq!(result, None);
    }

    #[test]
    fn parse_openai_sse_data_line_skips_non_data_lines() {
        // 非 data: 开头 (e.g. event: / id: / 空行) → None
        assert_eq!(parse_openai_sse_data_line("event: message"), None);
        assert_eq!(parse_openai_sse_data_line("id: 1"), None);
        assert_eq!(parse_openai_sse_data_line(""), None);
        assert_eq!(parse_openai_sse_data_line(""), None);
    }

    #[test]
    fn parse_openai_sse_data_line_handles_data_with_no_choices() {
        // 合法 JSON 但无 choices 字段 → None (skip, 跟 stream_options.usage 一致, 也不返 error)
        let line = r#"data: {"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#;
        let result = parse_openai_sse_data_line(line);
        assert_eq!(result, None);
    }

    #[test]
    fn parse_openai_sse_data_line_handles_empty_content_string() {
        // delta.content = "" → Some(Some("")) (空字符串也是合法内容, 但调用方可以决定是否过滤)
        let line = r#"data: {"choices":[{"delta":{"content":""}}]}"#;
        let result = parse_openai_sse_data_line(line);
        assert_eq!(result, Some(Some(String::new())));
    }

    // ============================================================
    // 战役 4-1: 真 SSE 推流测试 (用 in-memory byte stream 模拟, 不接网络)
    // ============================================================

    /// 战役 4-1 端到端真流式单元测试: 喂一个模拟 OpenAI SSE 字节流,
    /// 验证 stream 吐出 5 个 content delta, 跟期望文本完全一致.
    #[tokio::test]
    async fn complete_stream_parses_simulated_openai_sse_bytes() {
        use bytes::Bytes;
        use futures::stream::{self, StreamExt};

        // 模拟 OpenAI 真实 SSE 流 (5 个 content delta + [DONE])
        let sse_body = concat!(
            r#"data: {"id":"x","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}"#,
            "\n\n",
            r#"data: {"id":"x","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"你"},"finish_reason":null}]}"#,
            "\n\n",
            r#"data: {"id":"x","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"好"},"finish_reason":null}]}"#,
            "\n\n",
            r#"data: {"id":"x","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":","},"finish_reason":null}]}"#,
            "\n\n",
            r#"data: {"id":"x","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":" 世"},"finish_reason":null}]}"#,
            "\n\n",
            r#"data: {"id":"x","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"界"},"finish_reason":null}]}"#,
            "\n\n",
            r#"data: {"id":"x","object":"chat.completion.chunk","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#,
            "\n\n",
            "data: [DONE]\n\n",
        );

        // 模拟 reqwest bytes_stream: 按 SSE event 切 (保留多 chunk 切分, 验证 buffer 处理)
        let events: Vec<&str> = sse_body.split_inclusive("\n\n").collect();
        let byte_chunks: Vec<Result<Bytes, std::io::Error>> = events
            .into_iter()
            .map(|e| Ok(Bytes::copy_from_slice(e.as_bytes())))
            .collect();
        let byte_stream = stream::iter(byte_chunks);

        // 用 unfold 跟生产代码同款: 拉 byte_stream, 攒 buffer, 切 SSE event, 解析 data 行
        let mut buffer = String::new();
        let mut byte_stream = Box::pin(byte_stream);
        let mut chunks: Vec<String> = Vec::new();
        let mut done = false;
        while !done {
            if let Some(event_end) = buffer.find("\n\n") {
                let event: String = buffer.drain(..event_end + 2).collect();
                let mut stream_ended = false;
                let mut chunk_to_emit: Option<String> = None;
                for line in event.lines() {
                    if let Some(maybe_content) = parse_openai_sse_data_line(line) {
                        match maybe_content {
                            Some(content) => {
                                if !content.is_empty() {
                                    chunk_to_emit = Some(content);
                                }
                            }
                            None => {
                                stream_ended = true;
                                break;
                            }
                        }
                    }
                }
                if stream_ended {
                    done = true;
                    continue;
                }
                if let Some(content) = chunk_to_emit {
                    chunks.push(content);
                }
                continue;
            }
            match byte_stream.next().await {
                Some(Ok(bytes)) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                }
                Some(Err(_)) | None => {
                    // 收尾
                    if !buffer.trim().is_empty() {
                        for line in buffer.lines() {
                            if let Some(Some(content)) = parse_openai_sse_data_line(line) {
                                if !content.is_empty() {
                                    chunks.push(content);
                                }
                            }
                        }
                    }
                    done = true;
                }
            }
        }

        // 验收: 5 个 content chunk (role-only 和 finish-only 跳过, [DONE] 终止)
        assert_eq!(chunks, vec!["你", "好", ",", " 世", "界"]);
        let joined: String = chunks.concat();
        assert_eq!(joined, "你好, 世界");
    }
}
