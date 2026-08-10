//! L4 — WebSocket (async-tungstenite) + JSON Schema validation (jsonschema crate)
//!
//! 帧格式 (text frame):
//! `{"topic": "...", "trace_id": ..., "created_at_ms": ..., "payload": <T>}`
//!
//! 可选 schema — 在 client 侧 publish / server 侧 publish 入站时强制 JSON Schema 校验.
//! schema 不通过 → `BusError::SchemaValidation`.

use futures_util::stream::{BoxStream, StreamExt};
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex as AsyncMutex, RwLock as AsyncRwLock};
use tungstenite as _tung;

use crate::{BusError, BusMessage, BusResult, BusStats};

// === JSON Schema validator ===

/// JSON Schema 校验包装.
#[derive(Clone)]
pub struct JsonSchemaValidator {
    inner: Arc<jsonschema::Validator>,
    raw: String,
}

impl std::fmt::Debug for JsonSchemaValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonSchemaValidator")
            .field("raw", &self.raw)
            .finish()
    }
}

impl JsonSchemaValidator {
    /// 编译 JSON Schema.
    pub fn new(schema_json: impl Into<String>) -> BusResult<Self> {
        let raw = schema_json.into();
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| BusError::Config(format!("invalid json schema: {e}")))?;
        let schema = jsonschema::validator_for(&value)
            .map_err(|e| BusError::Config(format!("compile schema failed: {e}")))?;
        Ok(Self {
            inner: Arc::new(schema),
            raw,
        })
    }

    /// 校验一个 serde_json::Value — 返回第一个错误 (如有).
    pub fn validate(&self, v: &serde_json::Value) -> BusResult<()> {
        if let Err(e) = self.inner.validate(v) {
            // jsonschema 0.28 ValidationError: 用 Display → String
            return Err(BusError::SchemaValidation(e.to_string()));
        }
        Ok(())
    }
}

impl Default for JsonSchemaValidator {
    fn default() -> Self {
        // 默认 schema = 接受任意对象
        Self::new(r#"{"type":"object"}"#).expect("default schema must compile")
    }
}

// === WS frame wire-format ===

#[derive(Debug, Clone, Serialize, Deserialize)]
struct L4Frame<T> {
    topic: String,
    /// 消息
    #[serde(flatten)]
    msg: BusMessage<T>,
}

// === 内部 connection bundle ===

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
    tungstenite::Message,
>;

/// L4 — client + server 共用结构.
pub struct L4Bus<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> {
    endpoint: String,
    mode: L4Mode<T>,
    topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<L4Frame<T>>>>>,
    stats: Arc<BusStats>,
    schema: Option<JsonSchemaValidator>,
    _phantom: std::marker::PhantomData<T>,
}

#[derive(Clone)]
enum L4Mode<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> {
    /// 启动了 server, 监听 endpoint
    Server,
    /// 仅连接远端 (纯 client)
    Client {
        write: Arc<AsyncMutex<WsSink>>,
        topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<L4Frame<T>>>>>,
        stats: Arc<BusStats>,
    },
}

impl<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> Clone for L4Bus<T> {
    fn clone(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            mode: self.mode.clone(),
            topics: self.topics.clone(),
            stats: self.stats.clone(),
            schema: self.schema.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>> L4Bus<T> {
    /// 启动 server, listen 127.0.0.1:0.
    pub async fn start_unique() -> BusResult<Self> {
        Self::start_with_schema(None).await
    }

    /// 同上 + 强制 schema 校验.
    pub async fn start_with_schema(schema: Option<String>) -> BusResult<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let endpoint = format!("ws://127.0.0.1:{}", addr.port());
        let topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<L4Frame<T>>>>> =
            Arc::new(AsyncRwLock::new(HashMap::new()));
        let stats = BusStats::shared();
        let schema_v = schema
            .as_ref()
            .map(|s| JsonSchemaValidator::new(s.clone()))
            .transpose()?;
        let topics_c = topics.clone();
        let stats_c = stats.clone();
        let schema_c = schema_v.clone();
        tokio::spawn(async move {
            loop {
                let (tcp, _peer) = match listener.accept().await {
                    Ok(x) => x,
                    Err(_) => continue,
                };
                let topics = topics_c.clone();
                let stats = stats_c.clone();
                let schema = schema_c.clone();
                tokio::spawn(async move {
                    let ws = match tokio_tungstenite::accept_async(tcp).await {
                        Ok(w) => w,
                        Err(_) => return,
                    };
                    let (mut write, mut read) = ws.split();
                    while let Some(msg_res) = read.next().await {
                        let msg = match msg_res {
                            Ok(m) => m,
                            Err(_) => return,
                        };
                        if let tungstenite::Message::Text(text) = msg {
                            stats.received.fetch_add(1, Ordering::Relaxed);
                            let text_str: &str = text.as_str();
                            let frame: L4Frame<T> = match serde_json::from_str(text_str) {
                                Ok(f) => f,
                                Err(e) => {
                                    stats.dropped.fetch_add(1, Ordering::Relaxed);
                                    let payload = format!(r#"{{"err":"{e}"}}"#);
                                    let _ = write.send(tungstenite::Message::text(payload)).await;
                                    continue;
                                }
                            };
                            // 如果有 schema, fan-out 前校验 payload (当为 json-ish 时)
                            if let Some(s) = &schema {
                                let payload_val = serde_json::to_value(&frame.msg.payload)
                                    .unwrap_or(serde_json::Value::Null);
                                if s.validate(&payload_val).is_err() {
                                    stats.dropped.fetch_add(1, Ordering::Relaxed);
                                    let payload = r#"{"err":"schema rejected"}"#.to_string();
                                    let _ = write.send(tungstenite::Message::text(payload)).await;
                                    continue;
                                }
                            }
                            let tx = {
                                let mut map = topics.write().await;
                                map.entry(frame.topic.clone())
                                    .or_insert_with(|| {
                                        let (tx, _) = broadcast::channel(64);
                                        tx
                                    })
                                    .clone()
                            };
                            let _ = tx.send(frame);
                        }
                    }
                });
            }
        });
        Ok(Self {
            endpoint,
            mode: L4Mode::Server,
            topics,
            stats,
            schema: schema_v,
            _phantom: std::marker::PhantomData,
        })
    }

    /// 连接到 ws:// 远端 server.
    pub async fn connect(url: &str, schema_json: Option<String>) -> BusResult<Self> {
        let (ws, _resp) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|e| BusError::Io(e.to_string()))?;
        let topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<L4Frame<T>>>>> =
            Arc::new(AsyncRwLock::new(HashMap::new()));
        let stats = BusStats::shared();
        let schema_v = schema_json
            .map(|s| JsonSchemaValidator::new(s))
            .transpose()?;
        let (write, read) = ws.split();
        let write = Arc::new(AsyncMutex::new(write));
        let topics_c = topics.clone();
        let stats_c = stats.clone();
        tokio::spawn(async move {
            let mut read = read;
            while let Some(msg_res) = read.next().await {
                let msg = match msg_res {
                    Ok(m) => m,
                    Err(_) => return,
                };
                if let tungstenite::Message::Text(text) = msg {
                    stats_c.received.fetch_add(1, Ordering::Relaxed);
                    if let Ok(frame) = serde_json::from_str::<L4Frame<T>>(text.as_str()) {
                        let tx = {
                            let mut map = topics_c.write().await;
                            map.entry(frame.topic.clone())
                                .or_insert_with(|| {
                                    let (tx, _) = broadcast::channel(64);
                                    tx
                                })
                                .clone()
                        };
                        let _ = tx.send(frame);
                    }
                }
            }
        });
        Ok(Self {
            endpoint: url.to_string(),
            mode: L4Mode::Client {
                write,
                topics: topics.clone(),
                stats: stats.clone(),
            },
            topics,
            stats,
            schema: schema_v,
            _phantom: std::marker::PhantomData,
        })
    }

    /// 端点 URL.
    pub fn url(&self) -> &str {
        &self.endpoint
    }

    /// 发布: 在 server-mode 走本地 publish; client-mode 走 ws 发送.
    pub async fn publish(&self, topic: &str, msg: BusMessage<T>) -> BusResult<()> {
        match &self.mode {
            L4Mode::Server => {
                let frame = L4Frame {
                    topic: topic.to_string(),
                    msg: msg.clone(),
                };
                if let Some(s) = &self.schema {
                    let v = serde_json::to_value(&msg.payload)
                        .map_err(|e| BusError::Serde(e.to_string()))?;
                    s.validate(&v)?;
                }
                let tx = {
                    let mut map = self.topics.write().await;
                    map.entry(topic.to_string())
                        .or_insert_with(|| {
                            let (tx, _) = broadcast::channel(64);
                            tx
                        })
                        .clone()
                };
                let _ = tx.send(frame);
                self.stats.sent.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            L4Mode::Client { write, .. } => {
                let frame = L4Frame {
                    topic: topic.to_string(),
                    msg: msg.clone(),
                };
                if let Some(s) = &self.schema {
                    let v = serde_json::to_value(&msg.payload)
                        .map_err(|e| BusError::Serde(e.to_string()))?;
                    s.validate(&v)?;
                }
                let text =
                    serde_json::to_string(&frame).map_err(|e| BusError::Serde(e.to_string()))?;
                let mut w = write.lock().await;
                w.send(tungstenite::Message::text(text))
                    .await
                    .map_err(|e| BusError::Io(e.to_string()))?;
                drop(w);
                self.stats.sent.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
        }
    }

    /// 订阅 (Server-mode: 直接拿本地 broadcast; Client-mode: 取 client topics).
    pub async fn subscribe(
        &self,
        topic: &str,
    ) -> BusResult<BoxStream<'static, BusResult<BusMessage<T>>>> {
        let topics = match &self.mode {
            L4Mode::Server => self.topics.clone(),
            L4Mode::Client { topics, .. } => topics.clone(),
        };
        let rx = {
            let mut map = topics.write().await;
            map.entry(topic.to_string())
                .or_insert_with(|| {
                    let (tx, _) = broadcast::channel(64);
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
                            return Some((Ok::<_, BusError>(frame.msg), rx));
                        }
                        Err(broadcast::error::RecvError::Closed) => return None,
                        Err(e) => return Some((Err(BusError::Codec(e.to_string())), rx)),
                    }
                }
            }
        });
        Ok(Box::pin(stream))
    }

    /// 请求-响应: 与 L0/L1 类似, 先订阅再 publish.
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

// === 单元测试 ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_default_accepts_object() {
        let v = JsonSchemaValidator::default();
        let obj = serde_json::json!({"any": "thing"});
        v.validate(&obj).expect("must accept object");
    }

    #[test]
    fn validator_required_field() {
        let v = JsonSchemaValidator::new(
            r#"{"type":"object","required":["kind"],"properties":{"kind":{"type":"string"}}}"#,
        )
        .unwrap();
        assert!(v.validate(&serde_json::json!({"kind":"a"})).is_ok());
        assert!(v.validate(&serde_json::json!({"other":1})).is_err());
    }

    #[test]
    fn invalid_schema_string_rejected() {
        let r = JsonSchemaValidator::new("not-json-schema");
        assert!(r.is_err());
    }
}
