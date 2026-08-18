//! L2 — stdin/stdout pipe + JSON / MsgPack
//!
//! 跨平台 (std::process). 子进程作为 echo server, 父进程经 stdin/stdout
//! 与子进程交换 length-prefixed framed 消息.
//!
//! 帧格式 (字节):
//! - `codec = JSON`     → 1 byte tag (0x01) + `[len: u32 BE][json_bytes]`
//! - `codec = MsgPack`  → 1 byte tag (0x02) + `[len: u32 BE][msgpack_bytes]`
//!
//! 子进程 `--bus-echo-json` / `--bus-echo-msgpack` 由 lib.rs 的 `echo_server_main` 启用.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex as AsyncMutex;

use crate::{BusError, BusMessage, BusResult, BusStats};

/// L2 codec 选择.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeCodec {
    /// JSON over pipe
    Json,
    /// MsgPack over pipe (rmp-serde)
    MsgPack,
}

impl PipeCodec {
    fn tag(self) -> u8 {
        match self {
            PipeCodec::Json => 0x01,
            PipeCodec::MsgPack => 0x02,
        }
    }
    fn from_tag(t: u8) -> BusResult<Self> {
        match t {
            0x01 => Ok(Self::Json),
            0x02 => Ok(Self::MsgPack),
            _ => Err(BusError::Codec(format!("unknown codec tag {t}"))),
        }
    }
}

/// L2 transport 配置.
#[derive(Debug, Clone)]
pub struct L2Config {
    /// 子进程命令
    pub cmd: String,
    /// 子进程参数
    pub args: Vec<String>,
    /// 使用的 codec
    pub codec: PipeCodec,
    /// 连接超时
    pub connect_timeout: Duration,
}

/// L2 transport — 与子进程 (echo server) 通信.
pub struct L2Transport<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    codec: PipeCodec,
    stats: Arc<BusStats>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> L2Transport<T> {
    /// 启动子进程并把 stdin/stdout 接管.
    pub async fn spawn(cfg: L2Config) -> BusResult<Self> {
        let mut cmd = Command::new(&cfg.cmd);
        cmd.args(&cfg.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = tokio::process::Command::from(cmd)
            .spawn()
            .map_err(|e| BusError::Io(e.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| BusError::Io("missing child stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BusError::Io("missing child stdout".into()))?;
        Ok(Self {
            child,
            stdin,
            stdout,
            codec: cfg.codec,
            stats: BusStats::shared(),
            _phantom: std::marker::PhantomData,
        })
    }

    async fn write_one(&mut self, msg: &BusMessage<T>, topic: &str) -> BusResult<()> {
        // frame = tag + len + body
        let body = match self.codec {
            PipeCodec::Json => {
                let frame = L2Frame {
                    topic: topic.to_string(),
                    msg: msg.clone(),
                };
                serde_json::to_vec(&frame).map_err(|e| BusError::Serde(e.to_string()))?
            }
            PipeCodec::MsgPack => {
                let frame = L2Frame {
                    topic: topic.to_string(),
                    msg: msg.clone(),
                };
                rmp_serde::to_vec(&frame).map_err(|e| BusError::Codec(e.to_string()))?
            }
        };
        let tag = self.codec.tag();
        let len = (body.len() as u32).to_be_bytes();
        self.stdin.write_all(&[tag]).await?;
        self.stdin.write_all(&len).await?;
        self.stdin.write_all(&body).await?;
        self.stdin.flush().await?;
        self.stats.sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn read_one(&mut self) -> BusResult<L2Frame<T>> {
        let mut tag = [0u8; 1];
        self.stdout.read_exact(&mut tag).await?;
        let codec = PipeCodec::from_tag(tag[0])?;
        let mut len_buf = [0u8; 4];
        self.stdout.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > 16 * 1024 * 1024 {
            return Err(BusError::Codec(format!("frame too large: {len}")));
        }
        let mut buf = vec![0u8; len];
        self.stdout.read_exact(&mut buf).await?;
        self.stats.received.fetch_add(1, Ordering::Relaxed);
        match codec {
            PipeCodec::Json => {
                let frame: L2Frame<T> =
                    serde_json::from_slice(&buf).map_err(|e| BusError::Serde(e.to_string()))?;
                Ok(frame)
            }
            PipeCodec::MsgPack => {
                let frame: L2Frame<T> =
                    rmp_serde::from_slice(&buf).map_err(|e| BusError::Codec(e.to_string()))?;
                Ok(frame)
            }
        }
    }

    /// 发布 — 子进程 fan-out 仅作 echo (单接收方语义).
    pub async fn publish(&mut self, topic: &str, msg: BusMessage<T>) -> BusResult<()> {
        self.write_one(&msg, topic).await
    }

    /// 订阅 (Streaming): L2 stdin/stdout 是单工顺序协议, 本实现仅支持 req-rep.
    /// 多订阅者请用多个 L2Transport 实例.

    /// 请求-响应.
    pub async fn request(
        &mut self,
        topic: &str,
        msg: BusMessage<T>,
        timeout: Duration,
    ) -> BusResult<BusMessage<T>> {
        self.publish(topic, msg.clone()).await?;
        match tokio::time::timeout(timeout, self.read_one()).await {
            Ok(Ok(frame)) => {
                if frame.msg.trace_id == msg.trace_id || frame.topic == topic {
                    Ok(frame.msg)
                } else {
                    Ok(frame.msg)
                }
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(BusError::Timeout(timeout)),
        }
    }

    /// 共享 stats.
    pub fn stats(&self) -> crate::BusStatsSnapshot {
        self.stats.snapshot()
    }
}

impl<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> Drop
    for L2Transport<T>
{
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct L2Frame<T> {
    topic: String,
    msg: BusMessage<T>,
}

// === 子进程 echo server (sync std) ===
//
// 当父进程 spawn 一个 args 含 `--bus-echo-json` / `--bus-echo-msgpack` 的子进程时,
// 子进程 main() 应在解析完 args 后调 `echo_server_main(codec)`.
//
// 我们提供一个公开入口, 宿主 crate 调用方使用.

/// Echo-server 回送: read frame → write frame (同样 codec, 同 trace_id).
pub fn echo_server_main(codec: PipeCodec) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut sin = stdin.lock();
    let mut sout = stdout.lock();
    let mut len_buf = [0u8; 4];
    let mut tag_buf = [0u8; 1];
    loop {
        // read tag
        if sin.read_exact(&mut tag_buf).is_err() {
            return Ok(()); // EOF
        }
        let Ok(got_codec) = PipeCodec::from_tag(tag_buf[0]) else {
            return Ok(());
        };
        if got_codec != codec {
            // 不同 codec 不互通; 简单退出
            return Ok(());
        }
        // read len
        if sin.read_exact(&mut len_buf).is_err() {
            return Ok(());
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > 16 * 1024 * 1024 {
            return Ok(());
        }
        let mut body = vec![0u8; len];
        if sin.read_exact(&mut body).is_err() {
            return Ok(());
        }
        // echo: 直接回写同样字节
        let _ = sout.write_all(&tag_buf);
        let _ = sout.write_all(&len_buf);
        let _ = sout.write_all(&body);
        let _ = sout.flush();
    }
}

/// 便捷 wrapper: 解析 args, 决定是否进入 echo server, 否则返回 false (走普通 main).
pub fn try_run_echo_server(args: &[String]) -> bool {
    if args.iter().any(|a| a == "--bus-echo-json") {
        let _ = echo_server_main(PipeCodec::Json);
        return true;
    }
    if args.iter().any(|a| a == "--bus-echo-msgpack") {
        let _ = echo_server_main(PipeCodec::MsgPack);
        return true;
    }
    false
}

// === 单元测试 ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_tag_roundtrip() {
        assert_eq!(
            PipeCodec::from_tag(PipeCodec::Json.tag()).unwrap(),
            PipeCodec::Json
        );
        assert_eq!(
            PipeCodec::from_tag(PipeCodec::MsgPack.tag()).unwrap(),
            PipeCodec::MsgPack
        );
        assert!(PipeCodec::from_tag(0xff).is_err());
    }

    #[test]
    fn echo_server_args_detection() {
        assert!(try_run_echo_server(&["--bus-echo-json".to_string()]));
        assert!(!try_run_echo_server(&["--nope".to_string()]));
    }
}
