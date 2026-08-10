//! **apeireth-mcp / SSE Transport (real implementation)**
//!
//! **依据**: MCP 2025-03-26 规范 §Transport / SSE
//!
//! **wire protocol (field-level, 跟 VCP claude-code SSE 字段级对齐)**:
//! ```text
//! Client → Server (HTTP POST):
//!     POST {endpoint} HTTP/1.1
//!     Host: {host}
//!     Content-Type: application/json
//!     Accept: application/json, text/event-stream
//!
//!     {jsonrpc request body, no trailing newline}
//!
//! Server → Client (SSE stream, from initial GET):
//!     HTTP/1.1 200 OK
//!     Content-Type: text/event-stream
//!     Cache-Control: no-cache
//!
//!     event: endpoint
//!     data: /messages?sessionId=xxx
//!
//!     event: message
//!     data: {"jsonrpc":"2.0","id":1,"result":{...}}
//!
//!     event: message
//!     data: {"jsonrpc":"2.0","method":"notifications/..."}
//! ```
//!
//! **SSE 帧格式** (WHATWG HTML §9.2 Server-Sent Events):
//! - 字段: `event:` (事件名), `data:` (载荷, 多行可), `id:` (last-event-id), `retry:` (重连间隔)
//! - 帧分隔: 空行 (`\n\n`)
//! - 注释行: `:` 开头, 忽略
//!
//! **设计**:
//! - `send(line)` — POST 一行 JSON-RPC 请求到 endpoint URL (Content-Type: application/json)
//! - `recv()` — 阻塞读下一帧 SSE message, 返回 `data:` 字段内容 (去掉行尾 `\n`)
//! - `close()` — 关闭 SSE stream + reqwest client
//!
//! **不假装**: 仅按 MCP 2025-03-26 §Transport / SSE 子节实现, 不假装支持
//!   自定义 event types (除 `endpoint` + `message` 外忽略); 重连 (`retry:`) 暂不实现.

use super::{Transport, TransportError};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;

/// **SSE Transport — 真实现**
///
/// 持有 reqwest client + SSE stream + endpoint URL.
#[derive(Debug)]
pub struct SseTransport {
    /// reqwest client (cheap to clone, holds connection pool)
    client: reqwest::Client,
    /// 初始 GET SSE 的 URL (保留供重连)
    _sse_url: String,
    /// POST endpoint URL (从首个 `event:endpoint` 帧中解析)
    endpoint: Arc<Mutex<Option<String>>>,
    /// SSE message stream (frame 解析后的 `data:` 行流)
    stream: Arc<Mutex<Option<FrameStream>>>,
    /// 已关闭标记
    closed: bool,
}

/// **SSE 帧 (event + 多行 data)** — pub 给 http_streamable.rs 复用
#[derive(Debug, Default, Clone)]
pub struct SseFrame {
    pub event: Option<String>,
    pub data_lines: Vec<String>,
}

/// **FrameStream — 从 reqwest bytes_stream 解析 SSE 帧**
///
/// 状态机:
/// - 累积行直到遇到空行 → 提交一帧
/// - 解析 `event: x` / `data: x` / `id: x` / `retry: x` / `:comment`
struct FrameStream {
    raw:
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    pending: String,
}

// FrameStream 含 dyn Stream 无法 derive Debug; 手动空实现 (debug 打印无意义)
impl std::fmt::Debug for FrameStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameStream")
            .field("pending_len", &self.pending.len())
            .finish()
    }
}

impl FrameStream {
    fn new(
        raw: impl futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    ) -> Self {
        Self {
            raw: Box::pin(raw),
            pending: String::new(),
        }
    }

    /// 读下一帧 message (event=message 的 data)
    async fn next_message_data(&mut self) -> Result<Option<String>, TransportError> {
        loop {
            // 优先尝试从 pending 解析完整帧
            if let Some(data) = self.parse_one_frame()? {
                return Ok(Some(data));
            }
            // pending 不够, 拉更多字节
            match self.raw.next().await {
                Some(Ok(bytes)) => {
                    self.pending
                        .push_str(std::str::from_utf8(&bytes).map_err(|e| {
                            TransportError::Io(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                e,
                            ))
                        })?);
                }
                Some(Err(e)) => {
                    return Err(TransportError::Io(std::io::Error::other(e.to_string())))
                }
                None => {
                    // 流结束
                    return Ok(None);
                }
            }
        }
    }

    /// 尝试从 `self.pending` 解析一帧 SSE; 成功返回 Some(data), 不足返回 None
    fn parse_one_frame(&mut self) -> Result<Option<String>, TransportError> {
        // 找第一个空行 (\n\n 或 \r\n\r\n)
        let sep = self.find_frame_sep();
        let Some(end) = sep else { return Ok(None) };

        // 切出 frame 文本 (不含分隔符)
        let frame_text: String = self.pending.drain(..end).collect();
        // 跳过 \n\n 或 \r\n\r\n
        let skip = if self.pending.starts_with("\r\n\r\n") {
            4
        } else {
            2
        };
        let _ = self.pending.drain(..skip);

        let frame = parse_sse_frame(&frame_text);
        // 仅返回 event=message 的 data; 其他 (如 endpoint) 通过 endpoint() 单独取
        if let Some(ev) = &frame.event {
            if ev == "message" {
                return Ok(Some(frame.data_lines.join("\n")));
            }
            // endpoint event: 暴露给 connect 阶段
            if ev == "endpoint" {
                // 不返回, connect() 阶段会主动 next_message_data 反复轮询直到拿到 endpoint
                // 这里简单地: 把 endpoint data 暂存到外层 endpoint 字段 (通过返回特殊字符串)
                // 但我们是 Transport::recv() 通用接口, 不好耦合; 改: connect() 阶段单独读取。
                // 这里直接跳过, 让 connect 重新调 next_message_data 时再处理
                return Ok(None);
            }
        }
        // 无 event 字段或 event 不是 message: 跳过, 继续读下一帧
        Ok(None)
    }

    fn find_frame_sep(&self) -> Option<usize> {
        if let Some(idx) = self.pending.find("\n\n") {
            return Some(idx);
        }
        if let Some(idx) = self.pending.find("\r\n\r\n") {
            return Some(idx);
        }
        None
    }
}

/// **解析一帧 SSE 文本 → SseFrame**
///
/// 字段级: WHATWG HTML §9.2 Server-Sent Events
/// - `event:value` → event = "value"
/// - `data:value` → data_lines.push("value")
/// - `id:value` → (last-event-id, 当前实现忽略)
/// - `retry:value` → (重连间隔, 当前实现忽略)
/// - `:comment` → 忽略
/// - 空行 → 帧结束
fn parse_sse_frame(text: &str) -> SseFrame {
    let mut frame = SseFrame::default();
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix(':') {
            // 注释, 忽略
            let _ = rest;
            continue;
        }
        if let Some(rest) = line.strip_prefix("event:") {
            frame.event = Some(rest.trim_start().to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("data:") {
            // SSE spec: data 字段值去掉前导一个空格
            let v = rest.strip_prefix(' ').unwrap_or(rest);
            frame.data_lines.push(v.to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("id:") {
            let _ = rest; // 当前不实现 last-event-id
            continue;
        }
        if let Some(rest) = line.strip_prefix("retry:") {
            let _ = rest; // 当前不实现 retry
            continue;
        }
        // 未知字段, 忽略 (WHATWG: 忽略未知字段名)
    }
    frame
}

impl SseTransport {
    /// **连接 SSE transport**
    ///
    /// 流程:
    /// 1. `GET {url}` with `Accept: text/event-stream`
    /// 2. 从响应 bytes_stream 中解析 SSE 帧
    /// 3. 读第一帧 `event:endpoint` 拿 POST endpoint URL
    /// 4. 后续 `event:message` 帧的 `data:` 通过 `recv()` 返回
    pub async fn connect(url: impl Into<String>) -> Result<Self, TransportError> {
        let sse_url = url.into();
        let client = reqwest::Client::builder().build().map_err(|e| {
            TransportError::Io(std::io::Error::other(format!("reqwest builder: {e}")))
        })?;

        let resp = client
            .get(&sse_url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .send()
            .await
            .map_err(|e| TransportError::Io(std::io::Error::other(format!("SSE GET: {e}"))))?;

        if !resp.status().is_success() {
            return Err(TransportError::Io(std::io::Error::other(format!(
                "SSE GET HTTP {}",
                resp.status()
            ))));
        }

        let stream = resp.bytes_stream();
        let mut frame_stream = FrameStream::new(stream);

        // 阻塞读首帧 endpoint event
        let endpoint_url = loop {
            // 直接读 raw 直到能 parse 一帧
            let frame = read_next_frame(&mut frame_stream).await?;
            let Some(frame) = frame else {
                return Err(TransportError::Closed);
            };
            if frame.event.as_deref() == Some("endpoint") {
                let data = frame.data_lines.join("\n");
                // endpoint 可以是绝对 URL 或相对路径
                let absolute = absolutize_endpoint(&sse_url, &data);
                break absolute;
            }
            // 非 endpoint 帧先忽略 (罕见)
        };

        let endpoint: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(Some(endpoint_url)));
        let stream_slot: Arc<Mutex<Option<FrameStream>>> = Arc::new(Mutex::new(Some(frame_stream)));

        Ok(Self {
            client,
            _sse_url: sse_url,
            endpoint,
            stream: stream_slot,
            closed: false,
        })
    }

    /// **读 endpoint URL (测试 / 调试用)**
    pub async fn endpoint_url(&self) -> Option<String> {
        self.endpoint.lock().await.clone()
    }
}

/// **读下一帧 (任意 event), 给 connect() 用**
async fn read_next_frame(fs: &mut FrameStream) -> Result<Option<SseFrame>, TransportError> {
    // 内部循环: 一直拉到能 parse 一帧
    loop {
        if let Some(end) = fs.find_frame_sep() {
            let frame_text: String = fs.pending.drain(..end).collect();
            let skip = if fs.pending.starts_with("\r\n\r\n") {
                4
            } else {
                2
            };
            let _ = fs.pending.drain(..skip);
            return Ok(Some(parse_sse_frame(&frame_text)));
        }
        match fs.raw.next().await {
            Some(Ok(bytes)) => {
                fs.pending
                    .push_str(std::str::from_utf8(&bytes).map_err(|e| {
                        TransportError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                    })?);
            }
            Some(Err(e)) => {
                return Err(TransportError::Io(std::io::Error::other(e.to_string())));
            }
            None => return Ok(None),
        }
    }
}

/// **把 endpoint 相对路径转绝对 URL**
fn absolutize_endpoint(base: &str, endpoint: &str) -> String {
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        return endpoint.to_string();
    }
    // 解析 base URL
    let Ok(parsed) = reqwest::Url::parse(base) else {
        return endpoint.to_string();
    };
    // 拼接
    match parsed.join(endpoint) {
        Ok(joined) => joined.to_string(),
        Err(_) => endpoint.to_string(),
    }
}

#[async_trait::async_trait]
impl Transport for SseTransport {
    async fn send(&mut self, line: &str) -> Result<(), TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        let ep = self
            .endpoint
            .lock()
            .await
            .clone()
            .ok_or(TransportError::Closed)?;
        let resp = self
            .client
            .post(&ep)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(line.to_string())
            .send()
            .await
            .map_err(|e| TransportError::Io(std::io::Error::other(format!("SSE POST: {e}"))))?;
        // POST 通常返回 202 Accepted (无响应体, 响应通过 SSE 流回来)
        if !resp.status().is_success() && resp.status().as_u16() != 202 {
            return Err(TransportError::Io(std::io::Error::other(format!(
                "SSE POST HTTP {}",
                resp.status()
            ))));
        }
        Ok(())
    }

    async fn recv(&mut self) -> Result<Option<String>, TransportError> {
        if self.closed {
            return Ok(None);
        }
        let mut guard = self.stream.lock().await;
        let fs = guard.as_mut().ok_or(TransportError::Closed)?;
        loop {
            // 跳过非 message 帧 (如心跳 comment)
            if let Some(data) = fs.next_message_data().await? {
                return Ok(Some(data));
            }
            // next_message_data 内部已经过滤了非 message, 返回 None 表示流结束
            return Ok(None);
        }
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.closed = true;
        let mut guard = self.stream.lock().await;
        *guard = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 1: SSE 帧解析 — 单个 message 帧
    #[test]
    fn parse_sse_frame_message_basic() {
        let raw = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":42}\n\n";
        let f = parse_sse_frame(raw);
        assert_eq!(f.event.as_deref(), Some("message"));
        assert_eq!(f.data_lines.len(), 1);
        assert_eq!(
            f.data_lines[0],
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":42}"
        );
    }

    /// 测试 2: SSE 帧解析 — endpoint 帧
    #[test]
    fn parse_sse_frame_endpoint() {
        let raw = "event: endpoint\ndata: /messages?sessionId=abc123\n\n";
        let f = parse_sse_frame(raw);
        assert_eq!(f.event.as_deref(), Some("endpoint"));
        assert_eq!(f.data_lines[0], "/messages?sessionId=abc123");
    }

    /// 测试 3: SSE 帧解析 — 注释行 (:ping) + 多行 data
    #[test]
    fn parse_sse_frame_comment_and_multiline_data() {
        let raw = ": keepalive\ndata: line1\ndata: line2\n\n";
        let f = parse_sse_frame(raw);
        assert!(f.event.is_none());
        assert_eq!(f.data_lines.len(), 2);
        assert_eq!(f.data_lines[0], "line1");
        assert_eq!(f.data_lines[1], "line2");
    }

    /// 测试 4: SSE 帧解析 — data 前导单空格剥离 (WHATWG spec 只剥一个空格)
    #[test]
    fn parse_sse_frame_data_leading_space_stripped() {
        let raw = "data: {\"k\":1}\n\n";
        let f = parse_sse_frame(raw);
        // WHATWG: 仅剥离 data: 后的一个前导空格
        assert_eq!(f.data_lines[0], "{\"k\":1}");
    }

    /// 测试 4b: SSE 帧解析 — 多前导空格保留 (WHATWG: 只剥一个)
    #[test]
    fn parse_sse_frame_data_multiple_leading_spaces_kept() {
        let raw = "data:   {\"k\":1}\n\n";
        let f = parse_sse_frame(raw);
        // 仅第一个空格被剥离, 剩余两个空格保留
        assert_eq!(f.data_lines[0], "  {\"k\":1}");
    }

    /// 测试 5: endpoint 路径绝对化 — 相对路径
    #[test]
    fn absolutize_endpoint_relative() {
        let base = "https://example.com/api/sse";
        let ep = "/messages?x=1";
        let result = absolutize_endpoint(base, ep);
        assert_eq!(result, "https://example.com/messages?x=1");
    }

    /// 测试 6: endpoint 路径绝对化 — 已是绝对 URL
    #[test]
    fn absolutize_endpoint_already_absolute() {
        let base = "https://example.com/api/sse";
        let ep = "https://other.com/messages";
        assert_eq!(absolutize_endpoint(base, ep), "https://other.com/messages");
    }

    /// 测试 7: FrameStream.find_frame_sep — \n\n 分隔
    #[test]
    fn find_frame_sep_lf_lf() {
        let mut fs = FrameStream {
            raw: Box::pin(futures::stream::empty()),
            pending: "a\nb\n\nrest".into(),
        };
        // \n\n 在位置 3 ("a\nb" 之后)
        assert_eq!(fs.find_frame_sep(), Some(3));
    }

    /// 测试 8: FrameStream.find_frame_sep — \r\n\r\n 分隔
    #[test]
    fn find_frame_sep_crlf_crlf() {
        let mut fs = FrameStream {
            raw: Box::pin(futures::stream::empty()),
            pending: "a\r\nb\r\n\r\nrest".into(),
        };
        // \r\n\r\n 在位置 4 ("a\r\nb\r\n" 之后)
        assert_eq!(fs.find_frame_sep(), Some(4));
    }
}
