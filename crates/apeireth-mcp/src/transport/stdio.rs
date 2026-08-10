//! **apeireth-mcp / stdio Transport**
//!
//! **设计**:
//! - MCP 官方推荐的 stdio transport:子进程 stdin/stdout 上行帧化 JSON
//! - 服务端模式: 从当前进程的 `stdin` 读, 写到 `stdout`
//! - 客户端模式: `spawn_child(cmd, args)` 启动 MCP server 子进程,
//!   把子进程 stdin/stdout 当作全双工流
//!
//! **行帧**: 一行一帧 (末尾 `\n`), UTF-8 JSON, 帧内不允许裸 `\n`
//! (MCP 2025-03-26 规范 §Transport / stdio)

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use super::{Transport, TransportError};

/// **stdio Transport**
///
/// 内部两个模式:
/// - `Current` — 用当前进程的 stdin/stdout (服务端常用)
/// - `Child(cmd)` — spawn 一个子进程, 用其 stdin/stdout (客户端常用)
pub enum StdioTransport {
    /// 当前进程 stdio
    Current {
        reader: BufReader<Stdin>,
        writer: Stdout,
        closed: bool,
    },
    /// 子进程 stdio
    Child {
        reader: BufReader<ChildStdout>,
        writer: ChildStdin,
        child: Child,
        closed: bool,
    },
}

impl std::fmt::Debug for StdioTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Current { closed, .. } => {
                f.debug_struct("Current").field("closed", closed).finish()
            }
            Self::Child { closed, .. } => f.debug_struct("Child").field("closed", closed).finish(),
        }
    }
}

impl StdioTransport {
    /// **服务端模式**: 用当前进程的 stdin/stdout
    pub fn current() -> Self {
        Self::Current {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
            closed: false,
        }
    }

    /// **客户端模式**: spawn 子进程, 用其 stdin/stdout
    ///
    /// **注意**: spawn 失败返回 `TransportError::Spawn`
    pub fn spawn_child(program: &str, args: &[&str]) -> Result<Self, TransportError> {
        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());
        let mut child = cmd
            .spawn()
            .map_err(|e| TransportError::Spawn(format!("{program}: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TransportError::Spawn("no stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TransportError::Spawn("no stdout".into()))?;
        Ok(Self::Child {
            reader: BufReader::new(stdout),
            writer: stdin,
            child,
            closed: false,
        })
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&mut self, line: &str) -> Result<(), TransportError> {
        match self {
            Self::Current { writer, closed, .. } => {
                if *closed {
                    return Err(TransportError::Closed);
                }
                writer.write_all(line.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
                Ok(())
            }
            Self::Child { writer, closed, .. } => {
                if *closed {
                    return Err(TransportError::Closed);
                }
                writer.write_all(line.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
                Ok(())
            }
        }
    }

    async fn recv(&mut self) -> Result<Option<String>, TransportError> {
        let mut buf = String::new();
        let read_result = match self {
            Self::Current { reader, closed, .. } => {
                if *closed {
                    return Ok(None);
                }
                reader.read_line(&mut buf).await
            }
            Self::Child { reader, closed, .. } => {
                if *closed {
                    return Ok(None);
                }
                reader.read_line(&mut buf).await
            }
        };
        match read_result {
            Ok(0) => {
                if let Self::Child { closed, .. } = self {
                    *closed = true;
                }
                Ok(None)
            }
            Ok(_) => {
                let trimmed = buf.trim_end_matches(['\n', '\r']).to_string();
                Ok(Some(trimmed))
            }
            Err(e) => Err(TransportError::Io(e)),
        }
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        match self {
            Self::Current { writer, closed, .. } => {
                *closed = true;
                writer.shutdown().await?;
                Ok(())
            }
            Self::Child {
                writer,
                child,
                closed,
                ..
            } => {
                *closed = true;
                let _ = writer.shutdown().await;
                let _ = child.kill().await;
                Ok(())
            }
        }
    }
}
