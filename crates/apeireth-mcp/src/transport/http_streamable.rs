//! **apeireth-mcp / HTTP Streamable Transport (V2 skeleton)**
//!
//! **依据**: MCP 2025-03-26 规范 §Transport / Streamable HTTP
//! (newer spec, 2025-06-18 revision; 跟 SSE 双向解耦, 单端点 HTTP POST)
//!
//! **wire protocol (field-level)**:
//! ```text
//! Client → Server (HTTP POST):
//!     POST {endpoint} HTTP/1.1
//!     Host: {host}
//!     Content-Type: application/json
//!     Accept: application/json, text/event-stream
//!     Mcp-Session-Id: {optional session id}
//!
//!     {jsonrpc request body}
//!
//! Server → Client:
//!     HTTP/1.1 200 OK
//!     Content-Type: application/json      ← 同步单响应
//!     {jsonrpc response body}
//!
//!     或:
//!     HTTP/1.1 200 OK
//!     Content-Type: text/event-stream    ← 流式 (含 server-initiated notif)
//!     event: message
//!     data: {jsonrpc response}
//!     event: message
//!     data: {jsonrpc notification}
//! ```
//!
//! **设计 (V2 skeleton, 同步 RPC 模式)**:
//! - `send(line)` — POST {endpoint} with JSON-RPC request body
//!   - 同步等响应: 若 Content-Type=application/json → 收 body 入 `response_buffer`
//!                 若 Content-Type=text/event-stream → 解析 SSE 帧, 所有 `event:message`
//!                   的 data 入 `response_buffer`
//! - `recv()` — 从 `response_buffer` 弹一个; 缓冲空时阻塞等待 (V2 当前实现: 立即返回 None,
//!   显式说明要等下个 send)
//! - `close()` — 清理 reqwest client
//!
//! **V2 skeleton 范围**:
//! - ✅ 单端点 POST + JSON 同步响应
//! - ✅ SSE 流式响应 (单次响应内多帧)
//! - ✅ Mcp-Session-Id 透传 (字段)
//! - ❌ Server-initiated 长连接推送 (后续 task 引入独立 SSE 通道)
//! - ❌ Reconnection / resumability (后续 task)

use super::{Transport, TransportError};
use futures::StreamExt;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

/// **HTTP Streamable Transport (V2 skeleton)**
#[derive(Debug)]
pub struct HttpStreamableTransport {
    /// reqwest client
    client: reqwest::Client,
    /// 单端点 URL (e.g. "https://api.example.com/mcp")
    endpoint: String,
    /// Mcp-Session-Id (服务端在 Initialize 响应中给, 后续 POST 带回去)
    session_id: Arc<Mutex<Option<String>>>,
    /// 已收到的 JSON-RPC 响应/通知 (按 FIFO 顺序)
    response_buffer: Arc<Mutex<VecDeque<String>>>,
    /// 已关闭标记
    closed: bool,
}

impl HttpStreamableTransport {
    /// **构造 + 设置 endpoint** (V2 skeleton 不强制 GET 任何东西)
    pub fn connect(endpoint: impl Into<String>) -> Result<Self, TransportError> {
        let client = reqwest::Client::builder().build().map_err(|e| {
            TransportError::Io(std::io::Error::other(format!("reqwest builder: {e}")))
        })?;
        Ok(Self {
            client,
            endpoint: endpoint.into(),
            session_id: Arc::new(Mutex::new(None)),
            response_buffer: Arc::new(Mutex::new(VecDeque::new())),
            closed: false,
        })
    }

    /// **设 endpoint URL**
    pub fn set_endpoint(&mut self, endpoint: impl Into<String>) {
        self.endpoint = endpoint.into();
    }

    /// **读 session id**
    pub async fn session_id(&self) -> Option<String> {
        self.session_id.lock().await.clone()
    }

    /// **设 session id (server Initialize 响应后调用)**
    pub async fn set_session_id(&mut self, id: impl Into<String>) {
        *self.session_id.lock().await = Some(id.into());
    }
}

#[async_trait::async_trait]
impl Transport for HttpStreamableTransport {
    async fn send(&mut self, line: &str) -> Result<(), TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }

        // 组装请求
        let mut req = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");
        if let Some(sid) = self.session_id.lock().await.clone() {
            req = req.header("Mcp-Session-Id", sid);
        }
        let resp =
            req.body(line.to_string()).send().await.map_err(|e| {
                TransportError::Io(std::io::Error::other(format!("HTTP POST: {e}")))
            })?;

        // 提取 Mcp-Session-Id (如果 server 给)
        if let Some(sid) = resp.headers().get("Mcp-Session-Id") {
            if let Ok(s) = sid.to_str() {
                *self.session_id.lock().await = Some(s.to_string());
            }
        }

        let status = resp.status();
        if !status.is_success() {
            return Err(TransportError::Io(std::io::Error::other(format!(
                "HTTP POST {status}"
            ))));
        }

        // 读 body
        let content_type = resp
            .headers()
            .get("Content-Type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.starts_with("text/event-stream") {
            // 流式响应: 解析 SSE 帧, 所有 event:message 的 data 入 buffer
            let mut stream = resp.bytes_stream();
            let mut pending = String::new();
            while let Some(chunk) = stream.next().await {
                let bytes = chunk.map_err(|e| {
                    TransportError::Io(std::io::Error::other(format!("SSE chunk: {e}")))
                })?;
                pending.push_str(std::str::from_utf8(&bytes).map_err(|e| {
                    TransportError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                })?);

                // 解析所有完整帧
                while let Some(end) = find_frame_sep(&pending) {
                    let frame_text: String = pending.drain(..end).collect();
                    let skip = if pending.starts_with("\r\n\r\n") {
                        4
                    } else {
                        2
                    };
                    let _ = pending.drain(..skip);
                    let frame = parse_sse_frame(&frame_text);
                    if frame.event.as_deref() == Some("message") && !frame.data_lines.is_empty() {
                        let data = frame.data_lines.join("\n");
                        self.response_buffer.lock().await.push_back(data);
                    }
                }
            }
        } else {
            // 同步 JSON 响应: 整个 body 作为一个 message
            let body = resp.text().await.map_err(|e| {
                TransportError::Io(std::io::Error::other(format!("HTTP body: {e}")))
            })?;
            // 空 body 跳过 (e.g. 202 Accepted for notifications)
            if !body.trim().is_empty() {
                self.response_buffer.lock().await.push_back(body);
            }
        }
        Ok(())
    }

    async fn recv(&mut self) -> Result<Option<String>, TransportError> {
        if self.closed {
            return Ok(None);
        }
        // V2 skeleton: 同步 RPC 模式, response_buffer 仅在 send 后填充
        // 缓冲空时立即返回 None (不阻塞) — 真实场景需要 pending_response 跟踪未完成请求
        let mut buf = self.response_buffer.lock().await;
        Ok(buf.pop_front())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.closed = true;
        self.response_buffer.lock().await.clear();
        Ok(())
    }
}

/// **找 SSE 帧分隔符位置** (跟 sse.rs 同款)
fn find_frame_sep(s: &str) -> Option<usize> {
    if let Some(idx) = s.find("\n\n") {
        return Some(idx);
    }
    if let Some(idx) = s.find("\r\n\r\n") {
        return Some(idx);
    }
    None
}

/// **解析一帧 SSE 文本** (跟 sse.rs 同款 — 字段级: WHATWG §9.2)
fn parse_sse_frame(text: &str) -> crate::transport::sse::SseFrame {
    // 复用 sse.rs 的 parse_sse_frame (它不是 pub, 重新实现简化版)
    use crate::transport::sse::SseFrame;
    let mut frame = SseFrame::default();
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if line.starts_with(':') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("event:") {
            frame.event = Some(rest.trim_start().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("data:") {
            let v = rest.strip_prefix(' ').unwrap_or(rest);
            frame.data_lines.push(v.to_string());
        }
        // id / retry / 未知字段忽略
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 1: 构造时不报错
    #[tokio::test]
    async fn connect_succeeds() {
        let t = HttpStreamableTransport::connect("https://example.com/mcp").unwrap();
        assert!(!t.closed);
        assert_eq!(t.endpoint, "https://example.com/mcp");
    }

    /// 测试 2: closed 后 send 报错
    #[tokio::test]
    async fn send_after_close_errors() {
        let mut t = HttpStreamableTransport::connect("https://example.com/mcp").unwrap();
        t.close().await.unwrap();
        let r = t.send("{\"x\":1}").await;
        assert!(r.is_err());
    }

    /// 测试 3: closed 后 recv 返回 None
    #[tokio::test]
    async fn recv_after_close_returns_none() {
        let mut t = HttpStreamableTransport::connect("https://example.com/mcp").unwrap();
        t.close().await.unwrap();
        let r = t.recv().await.unwrap();
        assert!(r.is_none());
    }

    /// 测试 4: empty buffer recv 返回 None (同步 RPC 模式)
    #[tokio::test]
    async fn recv_empty_buffer_returns_none() {
        let mut t = HttpStreamableTransport::connect("https://example.com/mcp").unwrap();
        let r = t.recv().await.unwrap();
        assert!(r.is_none());
    }

    /// 测试 5: session_id 读写
    #[tokio::test]
    async fn session_id_roundtrip() {
        let mut t = HttpStreamableTransport::connect("https://example.com/mcp").unwrap();
        assert!(t.session_id().await.is_none());
        t.set_session_id("sess-abc-123").await;
        assert_eq!(t.session_id().await.as_deref(), Some("sess-abc-123"));
    }

    /// 测试 6: find_frame_sep
    #[test]
    fn find_frame_sep_basic() {
        // "a\nb\n\nrest" → \n\n 在 index 3 ("a\nb" 之后)
        assert_eq!(find_frame_sep("a\nb\n\nrest"), Some(3));
        // "a\r\nb\r\n\r\nrest" → \r\n\r\n 在 index 4 ("a\r\nb\r\n" 之后)
        assert_eq!(find_frame_sep("a\r\nb\r\n\r\nrest"), Some(4));
        assert_eq!(find_frame_sep("no separator"), None);
    }
}
