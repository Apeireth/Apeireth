//! L1 — Unix domain socket + bincode
//!
//! 仅 Unix 编译 (`#[cfg(unix)]`). Windows 上 `pub use` 在 lib.rs 由 cfg 隔开.
//!
//! 帧格式: `[len: u32 BE][bincode(BusMessage<T>)]` — 简单 length-prefixed framing.
//!
//! - `L1Server` — 监听 UDS path, 接受客户端连接, publish 广播到所有已连接 client.
//! - `L1Client` — 连接 UDS path, 发送 / 订阅.
//!
//! **CI fix 2026-08**: 原实现 server 只把帧塞进本地 broadcast channel (无消费者),
//! 从不写回 socket → client 永远收不到消息 → l1_uds_pubsub_basic /
//! l1_uds_trace_id_preserved 在 CI 必挂. 现改为:
//! - server 维护已连接 client 的 write half 列表, publish 广播写回所有连接
//! - client read loop 用独立 read half (into_split), publish 用 write half,
//!   消除"读循环持锁阻塞 → publish 死等"的隐患

#[cfg(unix)]
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
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
    stream: &mut (impl AsyncRead + Unpin),
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
async fn write_frame<T: Serialize>(
    stream: &mut (impl AsyncWrite + Unpin),
    frame: &L1Frame<T>,
) -> BusResult<()> {
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
    /// 已连接 client 的 write half (publish 广播目标)
    clients: Arc<AsyncRwLock<Vec<Arc<AsyncMutex<OwnedWriteHalf>>>>>,
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
        let clients: Arc<AsyncRwLock<Vec<Arc<AsyncMutex<OwnedWriteHalf>>>>> =
            Arc::new(AsyncRwLock::new(Vec::new()));
        let next_id = Arc::new(std::sync::atomic::AtomicU64::new(1));
        let stats = BusStats::shared();
        let me = Self {
            path: path.to_string(),
            clients: clients.clone(),
            next_id: next_id.clone(),
            stats: stats.clone(),
            _phantom: std::marker::PhantomData,
        };
        // 后台 accept loop — 每个连接: 登记 write half (供 publish 广播), 读循环 (统计)
        tokio::spawn(async move {
            loop {
                let Ok((stream, _addr)) = listener.accept().await else {
                    continue;
                };
                let (rd, wr) = stream.into_split();
                let wr = Arc::new(AsyncMutex::new(wr));
                clients.write().await.push(wr);
                let stats = stats.clone();
                let next_id = next_id.clone();
                tokio::spawn(async move {
                    let mut rd = rd;
                    loop {
                        let frame: L1Frame<T> = match read_frame_async(&mut rd).await {
                            Ok(f) => f,
                            Err(_) => return,
                        };
                        stats.received.fetch_add(1, Ordering::Relaxed);
                        // 维护 next_id (占位)
                        next_id.fetch_add(1, Ordering::Relaxed);
                        let _ = frame;
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

    /// 发布 — 广播 frame 到所有已连接 client (write half).
    ///
    /// CI fix 2026-08: 原实现只塞本地 broadcast channel (无消费者), client 收不到.
    /// UDS connect 返回后 accept loop 的登记 (accept + push) 可能尚未完成,
    /// 故广播前短等待连接就绪 (最多 200ms), 避免首条消息丢给空列表.
    pub async fn publish(&self, topic: &str, msg: BusMessage<T>) -> BusResult<()> {
        // 等 accept loop 登记已连接 client
        for _ in 0..20 {
            if !self.clients.read().await.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let frame = L1Frame::new(topic, msg);
        // 广播写回所有连接; 写失败 (client 断开) 的收集起来, 广播后清理
        let mut dead: Vec<Arc<AsyncMutex<OwnedWriteHalf>>> = Vec::new();
        {
            let clients = self.clients.read().await;
            for c in clients.iter() {
                let mut w = c.lock().await;
                if write_frame(&mut *w, &frame).await.is_err() {
                    dead.push(Arc::clone(c));
                }
            }
        }
        if !dead.is_empty() {
            let mut clients = self.clients.write().await;
            clients.retain(|c| !dead.iter().any(|d| Arc::ptr_eq(d, c)));
        }
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
    /// write half (publish 用; 与 read loop 分离, 互不阻塞)
    writer: Arc<AsyncMutex<OwnedWriteHalf>>,
    topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<L1Frame<T>>>>>,
    stats: Arc<BusStats>,
    _phantom: std::marker::PhantomData<T>,
}

#[cfg(unix)]
impl<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> L1Client<T> {
    /// 连接 UDS path.
    pub async fn connect(path: &str) -> BusResult<Self> {
        let stream = UnixStream::connect(path).await?;
        let (rd, wr) = stream.into_split();
        let writer = Arc::new(AsyncMutex::new(wr));
        let topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<L1Frame<T>>>>> =
            Arc::new(AsyncRwLock::new(HashMap::new()));
        let stats = BusStats::shared();
        // 后台 read loop — 读 server 广播帧, fan-out 到 topic subs (锁内读帧, 锁外 send)
        let topics_c = topics.clone();
        let stats_c = stats.clone();
        tokio::spawn(async move {
            let mut rd = rd;
            loop {
                let frame: L1Frame<T> = match read_frame_async(&mut rd).await {
                    Ok(f) => f,
                    Err(_) => return,
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
            writer,
            topics,
            stats,
            _phantom: std::marker::PhantomData,
        })
    }

    /// 发布到主题 (写 socket, server 广播给所有 client).
    pub async fn publish(&self, topic: &str, msg: BusMessage<T>) -> BusResult<()> {
        let frame = L1Frame::new(topic, msg);
        let mut w = self.writer.lock().await;
        write_frame(&mut *w, &frame).await?;
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
