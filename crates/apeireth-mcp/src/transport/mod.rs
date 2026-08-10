//! **apeireth-mcp / Transport trait + 共享错误**
//!
//! **设计**: Transport 是一个行帧化 (line-delimited JSON) 的全双工字节流抽象。
//! stdio / SSE / 内存管道 (in-memory) 都实现同一接口, 协议层不必关心底层载体。
//!
//! **不假装**: 不假设流式 / 半双工 — `recv` 返回 `None` 表示对端 EOF。

use async_trait::async_trait;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub mod http_streamable;
pub mod sse;
pub mod stdio;

pub use http_streamable::HttpStreamableTransport;
pub use sse::SseTransport;
pub use stdio::StdioTransport;

/// **Transport 层错误**
#[derive(Debug, Error)]
pub enum TransportError {
    /// IO 错误 (stdin/stdout/pipe)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// 对端已关闭 (EOF)
    #[error("transport closed by peer")]
    Closed,

    /// JSON 解析失败 (用于 stdio transport 解析行)
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// 子进程启动失败 (stdio transport spawn child)
    #[error("spawn child process failed: {0}")]
    Spawn(String),

    /// 不支持的操作 (skeleton 阶段用)
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
}

/// **Transport trait — 行帧化全双工字节流**
///
/// **设计**:
/// - `send(line)` — 写入一行 JSON (末尾带 `\n`), 不附加额外字节
/// - `recv()` — 阻塞读一行, 返回 `None` 表示对端 EOF
/// - 实现必须 `Send` (tokio task 之间移动)
#[async_trait]
pub trait Transport: Send {
    /// 发送一行 (call 端保证已含末尾 `\n`, transport 内部不再补)
    async fn send(&mut self, line: &str) -> Result<(), TransportError>;

    /// 接收一行 (不含末尾 `\n`); `Ok(None)` 表示对端关闭
    async fn recv(&mut self) -> Result<Option<String>, TransportError>;

    /// 主动关闭 transport (释放资源)
    async fn close(&mut self) -> Result<(), TransportError>;
}

// ============================================================
// TransportKind — 多传输模式统一工厂入口
// ============================================================

/// **Transport 类型枚举 — 多传输模式统一入口**
///
/// **设计**: 上层 (lib.rs McpClient::connect_*) 根据配置选一种, 工厂 `connect()` 产出
/// `Box<dyn Transport>`, 协议层不感知底层是 stdio / SSE / HTTP-streamable / 内存管道。
///
/// **字段级参考**: MCP 2025-03-26 §Transport 列了 3 种主要传输 (stdio / SSE / HTTP-streamable),
///  + 内存管道用于测试/example。
#[derive(Debug, Clone)]
pub enum TransportKind {
    /// **stdio** — spawn 子进程 + 标准输入输出 (MCP 最常见, 字段级对齐 VCP claude-code)
    Stdio { cmd: String, args: Vec<String> },
    /// **stdio current** — 用当前进程 stdin/stdout (e.g. 服务端就是当前可执行文件)
    StdioCurrent,
    /// **SSE** — GET {url} + 服务端推送 + 客户端 POST 反向通道 (MCP 2025-03-26 §Transport / SSE)
    Sse { url: String },
    /// **HTTP-streamable** — 单端点 HTTP POST + JSON-or-SSE 响应
    /// (MCP 2025-03-26 §Transport / Streamable HTTP, 2025-06-18 revision)
    HttpStreamable { url: String },
    /// **Memory** — 单进程内双向管道 (测试/example 用)
    Memory,
}

/// **从 `TransportKind` 构造 `Box<dyn Transport>`**
///
/// **注意**: `Memory` 返回一对 transport (用于 client+server), 而其他返回单个;
///   当前接口返回单个, Memory 的对端由调用方用 `tokio::io::duplex` 自取。
pub async fn connect(kind: TransportKind) -> Result<Box<dyn Transport>, TransportError> {
    match kind {
        TransportKind::Stdio { cmd, args } => {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let t = StdioTransport::spawn_child(&cmd, &arg_refs)?;
            Ok(Box::new(t))
        }
        TransportKind::StdioCurrent => {
            let t = StdioTransport::current();
            Ok(Box::new(t))
        }
        TransportKind::Sse { url } => {
            let t = SseTransport::connect(url).await?;
            Ok(Box::new(t))
        }
        TransportKind::HttpStreamable { url } => {
            let t = HttpStreamableTransport::connect(url)?;
            Ok(Box::new(t))
        }
        TransportKind::Memory => {
            // Memory 是双向管道, 上层应该自己 duplex + split
            // 这里返回 client 端, server 端由调用方用 tokio::io::duplex 同样 split
            // (实际使用场景少; 测试/example 直接用 tokio::io::duplex 更直观)
            Err(TransportError::NotImplemented(
                "TransportKind::Memory: use tokio::io::duplex + MemoryTransport::new directly",
            ))
        }
    }
}

// ============================================================
// 内存管道 Transport (用于 example / 单进程内 client+server 互调)
// ============================================================

/// **内存双向管道 Transport**
///
/// 用 `tokio::io::duplex(buffer_size)` 创建一对 `DuplexStream`,
/// 一端给 client, 一端给 server, 单进程内演示 MCP 端到端流程。
///
/// **不是真 wire**, 仅用于 example 与单测; 真生产环境用 `StdioTransport` / `SseTransport`。
pub struct MemoryTransport {
    reader: BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    writer: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    closed: bool,
}

impl MemoryTransport {
    /// 从 `DuplexStream` 构造 (通常由 `tokio::io::duplex` 配合 `split` 产出)
    pub fn new(stream: tokio::io::DuplexStream) -> Self {
        let (r, w) = tokio::io::split(stream);
        Self {
            reader: BufReader::new(r),
            writer: w,
            closed: false,
        }
    }
}

#[async_trait]
impl Transport for MemoryTransport {
    async fn send(&mut self, line: &str) -> Result<(), TransportError> {
        if self.closed {
            return Err(TransportError::Closed);
        }
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Option<String>, TransportError> {
        if self.closed {
            return Ok(None);
        }
        let mut buf = String::new();
        match self.reader.read_line(&mut buf).await {
            Ok(0) => {
                self.closed = true;
                Ok(None)
            }
            Ok(_) => {
                // 去掉行尾 \n / \r\n
                let trimmed = buf.trim_end_matches(['\n', '\r']).to_string();
                Ok(Some(trimmed))
            }
            Err(e) => Err(TransportError::Io(e)),
        }
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.closed = true;
        self.writer.shutdown().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_transport_roundtrip() {
        let (a, b) = tokio::io::duplex(1024);
        let mut ta = MemoryTransport::new(a);
        let mut tb = MemoryTransport::new(b);

        ta.send("hello").await.unwrap();
        let line = tb.recv().await.unwrap();
        assert_eq!(line.as_deref(), Some("hello"));

        tb.send("world\n").await.unwrap();
        let line = ta.recv().await.unwrap();
        assert_eq!(line.as_deref(), Some("world"));
    }

    #[tokio::test]
    async fn memory_transport_eof() {
        let (a, b) = tokio::io::duplex(64);
        let mut ta = MemoryTransport::new(a);
        let tb = MemoryTransport::new(b);
        drop(tb); // 关闭对端
                  // 给 reader 一小段时间探测 EOF (tokio::io::duplex 在对端 drop 时立即返回 0)
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let line = ta.recv().await.unwrap();
        assert!(line.is_none());
    }

    #[tokio::test]
    async fn memory_transport_close_then_send_errors() {
        let (a, _b) = tokio::io::duplex(64);
        let mut ta = MemoryTransport::new(a);
        ta.close().await.unwrap();
        assert!(ta.send("x").await.is_err());
    }
}
