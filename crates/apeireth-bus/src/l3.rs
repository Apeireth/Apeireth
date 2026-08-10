//! L3 — gRPC (tonic + prost)
//!
//! 5 个 proto message + 3 RPC:
//! - `Publish` (unary): server fan-out 到所有 subscribe 该 topic 的客户端
//! - `Subscribe` (server-stream): 客户端订阅 topic, server 持续 push
//! - `Request` (client-stream): 一问一答, 客户端发一条 BusWire, server 回一条

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::{BoxStream, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc as tmpsc, Mutex as AsyncMutex, RwLock as AsyncRwLock};
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};

use crate::{BusError, BusMessage, BusResult, BusStats};

// === 生成 proto 模块 (由 build.rs 编译 `proto/bus.proto`) ===

pub mod bus_proto {
    tonic::include_proto!("apeireth_bus");
}

use bus_proto::{
    bus_service_client::BusServiceClient, bus_service_server::BusServiceServer, BusWire,
    PublishAck, PublishRequest, SubscribeRequest,
};

/// Convert internal BusMessage<T> ↔ proto BusWire via JSON in bytes payload.
fn msg_to_wire<T: Serialize>(topic: &str, msg: &BusMessage<T>) -> BusResult<BusWire> {
    let payload = serde_json::to_vec(&msg.payload).map_err(|e| BusError::Serde(e.to_string()))?;
    Ok(BusWire {
        topic: topic.to_string(),
        trace_id: msg.trace_id,
        created_at_ms: msg.created_at_ms,
        payload,
    })
}

fn wire_to_msg<T: for<'de> Deserialize<'de>>(wire: BusWire) -> BusResult<BusMessage<T>> {
    let payload: T =
        serde_json::from_slice(&wire.payload).map_err(|e| BusError::Serde(e.to_string()))?;
    Ok(BusMessage {
        trace_id: wire.trace_id,
        payload,
        created_at_ms: wire.created_at_ms,
    })
}

// === Server-side state ===

#[derive(Default)]
struct ServerState {
    /// topic → broadcast::Sender<BusWire> (for Subscribe fan-out)
    topics: HashMap<String, broadcast::Sender<BusWire>>,
    /// For Request (client-stream): a one-shot reply channel keyed by trace_id.
    pending_replies: HashMap<u64, tmpsc::Sender<BusWire>>,
}

impl ServerState {
    fn topic_tx(&mut self, topic: &str) -> broadcast::Sender<BusWire> {
        self.topics
            .entry(topic.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(64);
                tx
            })
            .clone()
    }
}

struct BusServiceImpl {
    state: Arc<AsyncMutex<ServerState>>,
    stats: Arc<BusStats>,
}

#[tonic::async_trait]
impl bus_proto::bus_service_server::BusService for BusServiceImpl {
    async fn publish(
        &self,
        request: Request<PublishRequest>,
    ) -> Result<Response<PublishAck>, Status> {
        let req = request.into_inner();
        let mut st = self.state.lock().await;
        let tx = st.topic_tx(&req.topic);
        let wire = BusWire {
            topic: req.topic.clone(),
            trace_id: req.trace_id,
            created_at_ms: req.created_at_ms,
            payload: req.payload.clone(),
        };
        self.stats.received.fetch_add(1, Ordering::Relaxed);
        // 待 reply: 若 pending_replies 中有该 trace_id 的等待者, 也送一份
        if let Some(reply_tx) = st.pending_replies.get(&req.trace_id) {
            let _ = reply_tx.send(wire.clone()).await;
        }
        // fan-out 失败仅记 dropped (无 active subscribers)
        match tx.send(wire) {
            Ok(n) if n > 0 => {
                self.stats.sent.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                self.stats.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
        Ok(Response::new(PublishAck {
            accepted: true,
            trace_id: req.trace_id,
        }))
    }

    type SubscribeStream = std::pin::Pin<Box<dyn Stream<Item = Result<BusWire, Status>> + Send>>;

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let topic = request.into_inner().topic;
        let state = self.state.clone();
        let stats = self.stats.clone();
        // snapshot rx OUTSIDE lock
        let rx = {
            let mut st = state.lock().await;
            st.topic_tx(&topic).subscribe()
        };
        let topic_for_filter = topic.clone();
        let stream = futures_util::stream::unfold(rx, move |mut rx| {
            let stats = stats.clone();
            let topic_for_filter = topic_for_filter.clone();
            async move {
                loop {
                    match rx.recv().await {
                        Ok(w) => {
                            if w.topic == topic_for_filter {
                                stats.received.fetch_add(1, Ordering::Relaxed);
                                return Some((Ok::<_, Status>(w), rx));
                            }
                            // Skip non-matching topic; continue looping
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => return None,
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(e) => return Some((Err(tonic::Status::internal(e.to_string())), rx)),
                    }
                }
            }
        });
        Ok(Response::new(Box::pin(stream) as Self::SubscribeStream))
    }

    async fn request(
        &self,
        request: Request<Streaming<BusWire>>,
    ) -> Result<Response<BusWire>, Status> {
        let mut stream = request.into_inner();
        let state = self.state.clone();
        // Pull first message, register reply channel keyed by trace_id, wait
        let first = match stream.next().await {
            Some(Ok(w)) => w,
            Some(Err(e)) => return Err(e),
            None => return Err(Status::invalid_argument("empty client stream")),
        };
        let trace_id = first.trace_id;
        let topic = first.topic.clone();
        // Register reply
        let (reply_tx, mut reply_rx) = tmpsc::channel::<BusWire>(4);
        {
            let mut st = state.lock().await;
            st.pending_replies.insert(trace_id, reply_tx);
        }
        // Also fan-out to subscribers (in case caller wants pub-sub style)
        {
            let mut st = state.lock().await;
            let tx = st.topic_tx(&topic);
            let _ = tx.send(first.clone());
        }
        // Wait for a reply (timeout ~5s) — or any publish with this trace_id
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                let mut st = state.lock().await;
                st.pending_replies.remove(&trace_id);
                return Err(Status::deadline_exceeded("request timed out"));
            }
            match tokio::time::timeout(remaining, reply_rx.recv()).await {
                Ok(Some(reply)) => {
                    let mut st = state.lock().await;
                    st.pending_replies.remove(&trace_id);
                    return Ok(Response::new(reply));
                }
                Ok(None) => {
                    let mut st = state.lock().await;
                    st.pending_replies.remove(&trace_id);
                    return Err(Status::unavailable("reply channel closed"));
                }
                Err(_) => continue, // time-out loop check above
            }
        }
    }
}

// === Public L3Bus (client + server facade) ===

/// L3 gRPC bus — 既是 client 又是 server (在一个进程内可 start/connect).
pub struct L3Bus<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> {
    endpoint: String,
    mode: L3Mode,
    state: Arc<AsyncMutex<ServerState>>,
    stats: Arc<BusStats>,
    client: Option<Arc<AsyncMutex<BusServiceClient<tonic::transport::Channel>>>>,
    _phantom: std::marker::PhantomData<T>,
}

#[derive(Clone)]
enum L3Mode {
    Server { _task: Arc<tokio::sync::Notify> },
    Client,
}

impl<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> Clone for L3Bus<T> {
    fn clone(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            mode: self.mode.clone(),
            state: self.state.clone(),
            stats: self.stats.clone(),
            client: self.client.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> L3Bus<T> {
    /// 启动一个 server 在 127.0.0.1:0 (随机端口).
    pub async fn start_unique() -> BusResult<Self> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let local = listener.local_addr()?;
        drop(listener); // tonic will re-bind
        let endpoint = format!("http://127.0.0.1:{}", local.port());
        let state = Arc::new(AsyncMutex::new(ServerState::default()));
        let stats = BusStats::shared();
        let server_state = state.clone();
        let server_stats = stats.clone();
        let server_task = tokio::spawn(async move {
            let result = Server::builder()
                .add_service(BusServiceServer::new(BusServiceImpl {
                    state: server_state,
                    stats: server_stats,
                }))
                .serve(local)
                .await;
            if let Err(e) = result {
                eprintln!("[apeireth-bus L3 server] error: {e}");
            }
        });
        // 给 server 一点点时间 ready
        tokio::time::sleep(Duration::from_millis(80)).await;
        // self-referencing trick: 把 client 也注册到同一个 channel
        let client = BusServiceClient::connect(endpoint.clone()).await?;
        let me = Self {
            endpoint,
            mode: L3Mode::Server {
                _task: Arc::new(tokio::sync::Notify::new()),
            },
            state,
            stats,
            client: Some(Arc::new(AsyncMutex::new(client))),
            _phantom: std::marker::PhantomData,
        };
        me.spawn_keepalive(server_task);
        Ok(me)
    }

    fn spawn_keepalive(&self, task: tokio::task::JoinHandle<()>) {
        let _ = task; // 抑制未用
    }

    /// 连接到远端 gRPC endpoint.
    pub async fn connect(endpoint: &str) -> BusResult<Self> {
        let client = BusServiceClient::connect(endpoint.to_string()).await?;
        Ok(Self {
            endpoint: endpoint.to_string(),
            mode: L3Mode::Client,
            state: Arc::new(AsyncMutex::new(ServerState::default())),
            stats: BusStats::shared(),
            client: Some(Arc::new(AsyncMutex::new(client))),
            _phantom: std::marker::PhantomData,
        })
    }

    /// endpoint URL (e.g. `http://127.0.0.1:54321`).
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    async fn client(&self) -> BusResult<BusServiceClient<tonic::transport::Channel>> {
        let g = self.client.as_ref().ok_or(BusError::Closed)?;
        let c = g.lock().await;
        Ok(c.clone())
    }

    /// 发布 (Pub-Sub): unary Publish → server fan-out 给所有 subscribers.
    pub async fn publish(&self, topic: &str, msg: BusMessage<T>) -> BusResult<()> {
        let mut cli = self.client().await?;
        let req = PublishRequest {
            topic: topic.to_string(),
            trace_id: msg.trace_id,
            created_at_ms: msg.created_at_ms,
            payload: serde_json::to_vec(&msg.payload)
                .map_err(|e| BusError::Serde(e.to_string()))?,
        };
        let ack = cli.publish(req).await?.into_inner();
        self.stats.sent.fetch_add(1, Ordering::Relaxed);
        if !ack.accepted {
            self.stats.dropped.fetch_add(1, Ordering::Relaxed);
            return Err(BusError::Closed);
        }
        Ok(())
    }

    /// 订阅 — 返回 tonic server-stream 转 `BoxStream`.
    pub async fn subscribe(
        &self,
        topic: &str,
    ) -> BusResult<BoxStream<'static, BusResult<BusMessage<T>>>> {
        let mut cli = self.client().await?;
        let resp = cli
            .subscribe(SubscribeRequest {
                topic: topic.to_string(),
            })
            .await?;
        let stream = resp.into_inner();
        let stats_for_map = self.stats.clone();
        let s = stream.map(move |res| match res {
            Ok(w) => {
                stats_for_map.received.fetch_add(1, Ordering::Relaxed);
                wire_to_msg::<T>(w)
            }
            Err(s) => Err(BusError::Codec(s.to_string())),
        });
        Ok(Box::pin(s))
    }

    /// 请求-响应 — 用 client-stream 方式: 发一条 BusWire, 等 server 回一条.
    pub async fn request(
        &self,
        topic: &str,
        msg: BusMessage<T>,
        timeout: Duration,
    ) -> BusResult<BusMessage<T>> {
        use futures_util::stream;
        let mut cli = self.client().await?;
        let wire = msg_to_wire(topic, &msg)?;
        // tonic::IntoStreamingRequest requires Stream<Item = Message> (BusWire),
        // not Stream<Item = Result<Message, Status>>.
        let outbound = stream::iter(vec![wire]);
        let inner = tokio::time::timeout(timeout, cli.request(outbound))
            .await
            .map_err(|_| BusError::Timeout(timeout))??;
        let resp_wire = inner.into_inner();
        let reply_msg = wire_to_msg::<T>(resp_wire)?;
        self.stats.received.fetch_add(1, Ordering::Relaxed);
        Ok(reply_msg)
    }

    /// 共享 stats.
    pub fn stats(&self) -> crate::BusStatsSnapshot {
        self.stats.snapshot()
    }
}

// === 单元测试 ===

#[cfg(test)]
mod tests {
    use super::*;
    use crate::next_trace_id;

    #[test]
    fn msg_to_wire_roundtrip() {
        let msg: BusMessage<String> = BusMessage::with_trace_id(42u64, "hi".into());
        let w = msg_to_wire("topic", &msg).unwrap();
        assert_eq!(w.topic, "topic");
        assert_eq!(w.trace_id, 42);
        let back: BusMessage<String> = wire_to_msg(w).unwrap();
        assert_eq!(back.payload, "hi");
        assert_eq!(back.trace_id, 42);
    }
}
