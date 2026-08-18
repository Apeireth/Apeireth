//! L1 — Unix domain socket + bincode
//!
//! 仅 Unix 编译 (`#[cfg(unix)]`). Windows 上 `pub use` 在 lib.rs 由 cfg 隔开.
//!
//! 帧格式: `[len: u32 BE][bincode(BusMessage<T>)]` — 简单 length-prefixed framing.
//!
//! - `L1Server` — 监听 UDS path, 接受客户端连接, fan-out 多主题 broadcast.
//! - `L1Client` — 连接 UDS path, 发送 / 订阅.

#[cfg(unix)]
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, Mutex as AsyncMutex, RwLock as AsyncRwLock};

use crate::{BusError, BusMessage, BusResult, BusStats};

/// L1 帧包装 — 同时记录 trace_id 便于跨链路校验.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct L1Frame<T> {
    /// 主题
    topic: String,
    /// 消息
    msg: BusMessage<T>,
}

impl<T> L1Frame<T> {
    fn new(topic: impl Into<String>, msg: BusMessage<T>) -> Self {
        Self {
            topic: topic.into(),
            msg,
        }
    }
}

#[cfg(unix)]
fn read_frame<T: for<'de> Deserialize<'de>>(_stream: &mut UnixStream) -> BusResult<L1Frame<T>> {
    // 同步式 read_frame — 在 async context 里调用的话改用 read_frame_async
    unreachable!("use read_frame_async");
}

#[cfg(unix)]
async fn read_frame_async<T: for<'de> Deserialize<'de>>(
    stream: &mut UnixStream,
) -> BusResult<L1Frame<T>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 16 * 1024 * 1024 {
        return Err(BusError::Codec(format!("frame too large: {len}")));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    let (frame, _): (L1Frame<T>, usize) =
        bincode::serde::decode_from_slice(&buf, bincode::config::standard())
            .map_err(|e| BusError::Codec(e.to_string()))?;
    Ok(frame)
}

#[cfg(unix)]
async fn write_frame<T: Serialize>(stream: &mut UnixStream, frame: &L1Frame<T>) -> BusResult<()> {
    let bytes = bincode::serde::encode_to_vec(frame, bincode::config::standard())
        .map_err(|e| BusError::Codec(e.to_string()))?;
    let len = (bytes.len() as u32).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;
    Ok(())
}

// === L1 Server (Unix only) ===

/// L1 Server — Unix domain socket 监听端.
#[cfg(unix)]
pub struct L1Server<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> {
    path: String,
    topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<L1Frame<T>>>>>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
    stats: Arc<BusStats>,
    _phantom: std::marker::PhantomData<T>,
}

#[cfg(unix)]
impl<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> L1Server<T> {
    /// 绑定 UDS path (若已存在会被移除).
    pub async fn bind(path: &str) -> BusResult<Self> {
        let p = Path::new(path);
        if p.exists() {
            let _ = std::fs::remove_file(p);
        }
        let listener = UnixListener::bind(p)?;
        let topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<L1Frame<T>>>>> =
            Arc::new(AsyncRwLock::new(HashMap::new()));
        let next_id = Arc::new(std::sync::atomic::AtomicU64::new(1));
        let stats = BusStats::shared();
        let me = Self {
            path: path.to_string(),
            topics: topics.clone(),
            next_id: next_id.clone(),
            stats: stats.clone(),
            _phantom: std::marker::PhantomData,
        };
        // 后台 accept loop — 把每条收到的 frame fan-out 到 topic subscribers
        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _addr)) = listener.accept().await else {
                    continue;
                };
                let topics = topics.clone();
                let stats = stats.clone();
                let next_id = next_id.clone();
                tokio::spawn(async move {
                    loop {
                        let frame: L1Frame<T> = match read_frame_async(&mut stream).await {
                            Ok(f) => f,
                            Err(_) => return,
                        };
                        stats.received.fetch_add(1, Ordering::Relaxed);
                        let tx = {
                            let mut map = topics.write().await;
                            map.entry(frame.topic.clone())
                                .or_insert_with(|| {
                                    let (tx, _) = broadcast::channel::<L1Frame<T>>(64);
                                    tx
                                })
                                .clone()
                        };
                        let _ = tx.send(frame);
                        // 维护 next_id (占位)
                        next_id.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });
        Ok(me)
    }

    /// UDS path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// 发布 — 等价于 client.publish, 但免去回环路径.
    pub async fn publish(&self, topic: &str, msg: BusMessage<T>) -> BusResult<()> {
        let frame = L1Frame::new(topic, msg);
        let tx = {
            let mut map = self.topics.write().await;
            map.entry(topic.to_string())
                .or_insert_with(|| {
                    let (tx, _) = broadcast::channel::<L1Frame<T>>(64);
                    tx
                })
                .clone()
        };
        let _ = tx.send(frame);
        self.stats.sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// 共享 stats.
    pub fn stats(&self) -> crate::BusStatsSnapshot {
        self.stats.snapshot()
    }
}

#[cfg(unix)]
impl<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> Drop
    for L1Server<T>
{
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// === L1 Client (Unix only) ===

/// L1 Client — Unix domain socket 连接端.
#[cfg(unix)]
pub struct L1Client<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> {
    path: String,
    stream: Arc<AsyncMutex<UnixStream>>,
    topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<L1Frame<T>>>>>,
    stats: Arc<BusStats>,
    _phantom: std::marker::PhantomData<T>,
}

#[cfg(unix)]
impl<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> L1Client<T> {
    /// 连接 UDS path.
    pub async fn connect(path: &str) -> BusResult<Self> {
        let stream = Arc::new(AsyncMutex::new(UnixStream::connect(path).await?));
        let topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<L1Frame<T>>>>> =
            Arc::new(AsyncRwLock::new(HashMap::new()));
        let stats = BusStats::shared();
        // 后台 read loop — fan-out frames to topic subs (锁内读帧, 锁外 send)
        let stream_r = stream.clone();
        let topics_c = topics.clone();
        let stats_c = stats.clone();
        tokio::spawn(async move {
            loop {
                let frame: L1Frame<T> = {
                    let mut s = stream_r.lock().await;
                    match read_frame_async(&mut s).await {
                        Ok(f) => f,
                        Err(_) => return,
                    }
                };
                stats_c.received.fetch_add(1, Ordering::Relaxed);
                let tx = {
                    let mut map = topics_c.write().await;
                    map.entry(frame.topic.clone())
                        .or_insert_with(|| {
                            let (tx, _) = broadcast::channel::<L1Frame<T>>(64);
                            tx
                        })
                        .clone()
                };
                let _ = tx.send(frame);
            }
        });
        Ok(Self {
            path: path.to_string(),
            stream,
            topics,
            stats,
            _phantom: std::marker::PhantomData,
        })
    }

    /// 发布到主题.
    pub async fn publish(&self, topic: &str, msg: BusMessage<T>) -> BusResult<()> {
        let frame = L1Frame::new(topic, msg);
        let mut g = self.stream.lock().await;
        write_frame(&mut g, &frame).await?;
        drop(g);
        self.stats.sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// 订阅.
    pub async fn subscribe(
        &self,
        topic: &str,
    ) -> BusResult<futures_util::stream::BoxStream<'static, BusResult<BusMessage<T>>>> {
        let rx = {
            let mut map = self.topics.write().await;
            map.entry(topic.to_string())
                .or_insert_with(|| {
                    let (tx, _) = broadcast::channel::<L1Frame<T>>(64);
                    tx
                })
                .subscribe()
        };
        let stats = self.stats.clone();
        let stream = futures_util::stream::unfold(rx, move |mut rx| {
            let stats = stats.clone();
            async move {
                loop {
                    match rx.recv().await {
                        Ok(frame) => {
                            stats.received.fetch_add(1, Ordering::Relaxed);
                            return Some((Ok(frame.msg), rx));
                        }
                        Err(broadcast::error::RecvError::Closed) => return None,
                        Err(e) => return Some((Err(BusError::Codec(e.to_string())), rx)),
                    }
                }
            }
        });
        Ok(Box::pin(stream))
    }

    /// 请求-响应: 在 topic 上 publish + 等首个回包.
    pub async fn request(
        &self,
        topic: &str,
        msg: BusMessage<T>,
        timeout: Duration,
    ) -> BusResult<BusMessage<T>> {
        let mut sub = self.subscribe(topic).await?;
        self.publish(topic, msg.clone()).await?;
        match tokio::time::timeout(timeout, sub.next()).await {
            Ok(Some(Ok(m))) => Ok(m),
            Ok(Some(Err(e))) => Err(e),
            Ok(None) => Err(BusError::Closed),
            Err(_) => Err(BusError::Timeout(timeout)),
        }
    }

    /// 共享 stats.
    pub fn stats(&self) -> crate::BusStatsSnapshot {
        self.stats.snapshot()
    }
}

// === Windows stub: 编译时 cfg 跳过 + runtime 返回 stub error ===

/// Windows 占位类型 — compile-time 通过 `#[cfg(unix)]` 在 lib.rs 隔离,
/// 这里对 Windows 提供 *non-pub* stub 让 modules 文件自身可编译.
#[cfg(not(unix))]
#[allow(dead_code)]
fn _unsupported_stub() -> BusError {
    BusError::Unsupported("L1 (Unix domain socket) is not supported on this platform".into())
}
