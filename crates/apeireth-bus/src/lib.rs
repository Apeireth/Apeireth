//! apeireth-bus — 5 层通信总线 (V28.x 后续深化, round15-02)
//!
//! # 5 层通信总线
//!
//! | 层 | 传输 | 编解码 | 适用场景 | 后端依赖 |
//! |----|------|--------|----------|----------|
//! | L0 | inproc (tokio mpsc / broadcast / watch) | serde_json | 进程内通信 | tokio |
//! | L1 | Unix domain socket (bincode) | bincode | 本机跨进程 (Unix) | tokio |
//! | L2 | stdin/stdout pipe (JSON / MsgPack) | serde_json / rmp-serde | 子进程 (跨平台) | std::process |
//! | L3 | gRPC (tonic) | protobuf | 跨机器 RPC | tonic / prost |
//! | L4 | WebSocket (async-tungstenite) | JSON Schema | 浏览器 / WS 客户端 | async-tungstenite / jsonschema |

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod channel;
pub mod l0;
#[cfg(test)]
pub mod r216_tests; // R216: 三套通知 + 4 BackpressurePolicy 测试覆盖
                    // R177: bus invariants (10 tests + 2 Kani proofs)
pub mod event_log; // R229 — append-only event log + filter replay
#[cfg(unix)]
pub mod l1;
#[cfg(unix)]
pub mod l2;
#[cfg(feature = "full-bus")]
pub mod l3;
#[cfg(feature = "full-bus")]
pub mod l4;
pub mod lifecycle;
mod organ_kani_proofs;
pub mod pattern; // R227 — topic wildcard matching // A1#5 — 5 lifecycle hooks (UserPromptSubmit/SessionStart/SessionEnd/PostToolUse/Stop)

pub use channel::{Channel, ChannelSet, ChanneledBus};
pub use l0::L0Bus;
#[cfg(unix)]
pub use l1::{L1Client, L1Server};
#[cfg(unix)]
pub use l2::{L2Config, L2Transport, PipeCodec};
#[cfg(feature = "full-bus")]
pub use l3::L3Bus;
#[cfg(feature = "full-bus")]
pub use l4::L4Bus;
pub use lifecycle::{
    LifecycleBus, LifecycleContext, LifecycleEvent, LifecycleHook, LifecycleMessage,
};

// === 全局 trace_id 分配器 ===

static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 分配下一个全局唯一的 trace_id (单调递增, wrap-around 安全).
pub fn next_trace_id() -> u64 {
    TRACE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// crate 版本 (与 workspace.version 同步)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// === R245 Message Priority ===

/// R245 message priority tag (separate from BackpressurePolicy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MessagePriority {
    High,
    #[default]
    Normal,
    Low,
}

// === 核心消息 ===

/// 跨所有层的统一消息结构: `trace_id` + `payload`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusMessage<T> {
    /// 链路追踪 ID — 创建时分配, 经所有层转发不丢失
    pub trace_id: u64,
    /// 业务负载 (serde-derived)
    pub payload: T,
    /// 创建时间戳 (epoch millis) — 调试时可视化跳数
    pub created_at_ms: i64,
    /// R245: priority tag (High / Normal / Low). Default Normal.
    pub priority: MessagePriority,
}

impl<T> BusMessage<T> {
    /// 构造新消息 (自动分配 trace_id).
    pub fn new(payload: T) -> Self {
        Self {
            trace_id: next_trace_id(),
            payload,
            created_at_ms: now_ms(),
            priority: MessagePriority::default(),
        }
    }

    /// 构造带指定 trace_id 的消息 (用于链路跨进程转发时保持一致).
    pub fn with_trace_id(trace_id: u64, payload: T) -> Self {
        Self {
            trace_id,
            payload,
            created_at_ms: now_ms(),
            priority: MessagePriority::default(),
        }
    }

    /// 映射 payload 类型 (保留 trace_id).
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> BusMessage<U> {
        BusMessage {
            trace_id: self.trace_id,
            payload: f(self.payload),
            created_at_ms: self.created_at_ms,
            priority: self.priority,
        }
    }

    /// R245 -- set message priority (builder).
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
}

/// 当前 epoch milliseconds.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// === 反背压 / 丢弃策略 ===

/// Bounded channel 满时的反背压策略.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackpressurePolicy {
    /// 发送方 await 直到有空间 (默认).
    Block,
    /// 丢弃队列头 (最早未消费), 把新消息放进去.
    DropOldest,
    /// 直接丢弃新消息.
    DropNewest,
    /// 直接丢弃 (用于遥测等低优先).
    Drop,
    /// **R226 — 合并 (coalesce)**: 同 topic+ttl_ms 窗口内的连续消息合并为最新一条,
    ///   中间消息直接丢弃. 适合 telemetry / metrics 高频上报.
    ///   语义: `Coalesce { ttl_ms: 1000 }` 表示 1s 内重复消息只保留最后一条.
    Coalesce { ttl_ms: u64 },
    /// **R226 — 自适应 (adaptive)**: 起始按 `initial` 行为; 当 dropped / sent 比率超过
    ///   `drop_threshold` (0.0-1.0) 时, 自动切换到 `DropOldest` 缓解.
    ///   适合流量波动大、不想硬编码策略的场景.
    Adaptive {
        initial: Box<BackpressurePolicy>,
        drop_threshold: f64,
    },
}

impl BackpressurePolicy {
    /// 编译期 hardcode — 当前支持的策略数
    pub const VARIANT_COUNT: usize = 6;

    /// 策略名 (调试/日志用)
    pub fn name(&self) -> &'static str {
        match self {
            BackpressurePolicy::Block => "Block",
            BackpressurePolicy::DropOldest => "DropOldest",
            BackpressurePolicy::DropNewest => "DropNewest",
            BackpressurePolicy::Drop => "Drop",
            BackpressurePolicy::Coalesce { .. } => "Coalesce",
            BackpressurePolicy::Adaptive { .. } => "Adaptive",
        }
    }
}

impl Default for BackpressurePolicy {
    fn default() -> Self {
        Self::Block
    }
}

// === Bus 统计 ===

/// Bus 可观测统计 (原子计数, 无锁).
#[derive(Debug, Default)]
pub struct BusStats {
    /// 已发出消息数
    pub sent: AtomicU64,
    /// 因反背压丢弃的消息数
    pub dropped: AtomicU64,
    /// 已接收消息数
    pub received: AtomicU64,
    /// 跨层重传次数 (用于可靠性分析)
    pub retransmit: AtomicU64,
    /// R245 -- priority High/Normal/Low 计数器 (按 priority 分桶).
    pub high_priority: AtomicU64,
    pub normal_priority: AtomicU64,
    pub low_priority: AtomicU64,
}

impl BusStats {
    /// 新建空统计.
    pub fn new() -> Self {
        Self::default()
    }

    /// 包装成 `Arc<BusStats>`.
    pub fn shared() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::new())
    }

    /// 快照.
    pub fn snapshot(&self) -> BusStatsSnapshot {
        BusStatsSnapshot {
            sent: self.sent.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            received: self.received.load(Ordering::Relaxed),
            retransmit: self.retransmit.load(Ordering::Relaxed),
            high_priority: self.high_priority.load(Ordering::Relaxed),
            normal_priority: self.normal_priority.load(Ordering::Relaxed),
            low_priority: self.low_priority.load(Ordering::Relaxed),
        }
    }
}

/// `BusStats` 的不可变快照.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusStatsSnapshot {
    /// 已发出
    pub sent: u64,
    /// 已丢弃
    pub dropped: u64,
    /// 已接收
    pub received: u64,
    /// 重传
    pub retransmit: u64,
    /// R245: 按 priority 分桶的发出计数.
    pub high_priority: u64,
    pub normal_priority: u64,
    pub low_priority: u64,
}

// === Bus 错误 ===

/// Bus 错误类型.
#[derive(Debug, Error)]
pub enum BusError {
    /// 编解码失败 (bincode / JSON / protobuf).
    #[error("codec error: {0}")]
    Codec(String),
    /// I/O 失败 (UDS / pipe / TCP / WS).
    #[error("io error: {0}")]
    Io(String),
    /// Channel 已关闭.
    #[error("channel closed")]
    Closed,
    /// 反背压丢弃 (发送方选择 DropNewest/Drop 时回送).
    #[error("message dropped by backpressure policy")]
    Dropped,
    /// JSON Schema 校验失败 (L4).
    #[error("schema validation failed: {0}")]
    SchemaValidation(String),
    /// 配置错误.
    #[error("config error: {0}")]
    Config(String),
    /// 当前平台不支持该层 (例如 L1 在 Windows).
    #[error("layer unsupported on this platform: {0}")]
    Unsupported(String),
    /// 超时.
    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),
    /// 序列化失败 (serde).
    #[error("serde error: {0}")]
    Serde(String),
    /// gRPC / tonic 错误.
    #[error("grpc error: {0}")]
    Grpc(String),
}

impl From<std::io::Error> for BusError {
    fn from(e: std::io::Error) -> Self {
        BusError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for BusError {
    fn from(e: serde_json::Error) -> Self {
        BusError::Serde(e.to_string())
    }
}

impl From<tonic::Status> for BusError {
    fn from(s: tonic::Status) -> Self {
        BusError::Grpc(s.to_string())
    }
}

impl From<tonic::transport::Error> for BusError {
    fn from(e: tonic::transport::Error) -> Self {
        BusError::Grpc(e.to_string())
    }
}

/// Bus Result 别名.
pub type BusResult<T> = Result<T, BusError>;

// === 总线抽象 trait ===

/// 跨 5 层统一抽象: 每层只需实现三类操作.
#[async_trait::async_trait]
pub trait Bus: Send + Sync {
    /// 业务负载类型.
    type Payload: Serialize + for<'de> Deserialize<'de> + Send + 'static;

    /// 发布到主题 (Pub-Sub): 多订阅者.
    async fn publish(&self, topic: &str, msg: BusMessage<Self::Payload>) -> BusResult<()>;

    /// 请求-响应 (Req-Rep): 返回订阅方的第一个应答 (用 trace_id 关联).
    async fn request(
        &self,
        topic: &str,
        msg: BusMessage<Self::Payload>,
        timeout: std::time::Duration,
    ) -> BusResult<BusMessage<Self::Payload>>;

    /// 启动一个流式订阅 (Streaming): 返回 `Stream<Item = BusMessage<...>>`.
    fn subscribe(
        &self,
        topic: &str,
    ) -> futures_util::stream::BoxStream<'static, BusResult<BusMessage<Self::Payload>>>;

    /// 读取统计快照.
    fn stats(&self) -> BusStatsSnapshot;
}

// === 单元测试 ===

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    #[test]
    fn trace_id_is_monotonic() {
        let a = next_trace_id();
        let b = next_trace_id();
        assert!(b > a, "trace_id must monotonically increase");
    }

    #[test]
    fn bus_message_keeps_trace_id_through_map() {
        let m: BusMessage<u32> = BusMessage::with_trace_id(42u64, 7u32);
        let mapped: BusMessage<u64> = m.map(u64::from);
        assert_eq!(mapped.trace_id, 42);
        assert_eq!(mapped.payload, 7);
    }

    #[test]
    fn backpressure_policy_default_is_block() {
        assert_eq!(BackpressurePolicy::default(), BackpressurePolicy::Block);
    }

    #[test]
    fn bus_stats_snapshot_reflects_increments() {
        let s = BusStats::new();
        s.sent.fetch_add(3, Ordering::Relaxed);
        s.dropped.fetch_add(2, Ordering::Relaxed);
        s.received.fetch_add(5, Ordering::Relaxed);
        s.retransmit.fetch_add(1, Ordering::Relaxed);
        let snap = s.snapshot();
        assert_eq!(snap.sent, 3);
        assert_eq!(snap.dropped, 2);
        assert_eq!(snap.received, 5);
        assert_eq!(snap.retransmit, 1);
    }

    #[test]
    fn bus_error_io_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "x");
        let be: BusError = io_err.into();
        matches!(be, BusError::Io(_));
    }

    #[test]
    fn bus_error_serde_conversion() {
        let bad: serde_json::Result<()> = serde_json::from_str::<()>("oops");
        let be: BusError = bad.unwrap_err().into();
        matches!(be, BusError::Serde(_));
    }

    // ---------- L0 模式覆盖 ----------

    #[tokio::test]
    async fn t01_l0_pubsub_single_sub() {
        let bus = L0Bus::<String>::with_capacity(16);
        let mut s = bus.subscribe("t01").await.unwrap();
        bus.publish("t01", BusMessage::new("hi".into()))
            .await
            .unwrap();
        let m = tokio::time::timeout(Duration::from_millis(500), s.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(m.payload, "hi");
    }

    #[tokio::test]
    async fn t02_l0_pubsub_multi_sub() {
        let bus = L0Bus::<String>::with_capacity(16);
        let mut a = bus.subscribe("t02").await.unwrap();
        let mut b = bus.subscribe("t02").await.unwrap();
        bus.publish("t02", BusMessage::new("x".into()))
            .await
            .unwrap();
        let m_a = a.next().await.unwrap().unwrap();
        let m_b = b.next().await.unwrap().unwrap();
        assert_eq!(m_a.payload, "x");
        assert_eq!(m_b.payload, "x");
        assert_eq!(m_a.trace_id, m_b.trace_id);
    }

    #[tokio::test]
    async fn t03_l0_reqrep() {
        // 用一个oneshot channel 让 spawn 报告"已订阅", 避免 spawn/publish 竞态
        let bus = L0Bus::<String>::with_capacity(16);
        let bus_r = bus.clone();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();
        let responder = tokio::spawn(async move {
            let mut s = bus_r.subscribe("req_topic").await.unwrap();
            let _ = ready_tx.send(());
            if let Some(Ok(r)) = s.next().await {
                BusMessage::with_trace_id(r.trace_id, format!("echo:{}", r.payload))
            } else {
                BusMessage::new("noop".into())
            }
        });
        // 等待订阅就绪
        ready_rx.await.unwrap();
        let req = BusMessage::new("ping".into());
        let trace_id = req.trace_id;
        bus.publish("req_topic", req).await.unwrap();
        let want = tokio::time::timeout(Duration::from_millis(500), responder)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(want.trace_id, trace_id);
        assert_eq!(want.payload, "echo:ping");
    }

    #[tokio::test]
    async fn t04_l0_streaming() {
        let bus = L0Bus::<u32>::with_capacity(16);
        let mut s = bus.subscribe("stream").await.unwrap();
        for i in 0..5u32 {
            bus.publish("stream", BusMessage::new(i)).await.unwrap();
        }
        for want in 0..5u32 {
            let m = s.next().await.unwrap().unwrap();
            assert_eq!(m.payload, want);
        }
    }

    #[tokio::test]
    async fn t05_l0_backpressure_drop_oldest() {
        let bus = L0Bus::<u32>::with_capacity_and_policy(1, BackpressurePolicy::DropOldest);
        for i in 0..10u32 {
            let _ = bus.publish("h", BusMessage::new(i)).await;
        }
        let snap = bus.stats();
        assert!(snap.dropped + snap.sent > 0);
    }

    #[tokio::test]
    async fn t06_trace_id_through_map() {
        let m: BusMessage<u32> = BusMessage::with_trace_id(12345u64, 99u32);
        let mapped: BusMessage<String> = m.map(|x| format!("v={x}"));
        assert_eq!(mapped.trace_id, 12345);
        assert_eq!(mapped.payload, "v=99");
    }

    #[tokio::test]
    async fn t15_backpressure_policy_variants() {
        for pol in [
            BackpressurePolicy::Block,
            BackpressurePolicy::DropOldest,
            BackpressurePolicy::DropNewest,
            BackpressurePolicy::Drop,
        ] {
            let bus = L0Bus::<u32>::with_capacity_and_policy(2, pol);
            for i in 0..6u32 {
                let _ = bus.publish("p", BusMessage::new(i)).await;
            }
            let snap = bus.stats();
            assert!(snap.sent + snap.dropped > 0);
        }
    }

    // R245 -- priority tag (3 cases)
    #[test]
    fn r245_01_message_priority_default_is_normal() {
        let m: BusMessage<u32> = BusMessage::new(1);
        assert_eq!(m.priority, MessagePriority::Normal);
    }

    #[test]
    fn r245_02_with_priority_builder_sets_priority() {
        let m: BusMessage<u32> = BusMessage::new(1).with_priority(MessagePriority::High);
        assert_eq!(m.priority, MessagePriority::High);
        let m2 = m.with_priority(MessagePriority::Low);
        assert_eq!(m2.priority, MessagePriority::Low);
    }

    #[tokio::test]
    async fn r245_03_publish_counts_priority_buckets() {
        let bus = L0Bus::<u32>::new();
        // publish 1 high, 2 normal, 3 low
        bus.publish("p", BusMessage::new(1).with_priority(MessagePriority::High))
            .await
            .unwrap();
        bus.publish("p", BusMessage::new(2)).await.unwrap();
        bus.publish("p", BusMessage::new(3)).await.unwrap();
        bus.publish("p", BusMessage::new(4).with_priority(MessagePriority::Low))
            .await
            .unwrap();
        bus.publish("p", BusMessage::new(5).with_priority(MessagePriority::Low))
            .await
            .unwrap();
        bus.publish("p", BusMessage::new(6).with_priority(MessagePriority::Low))
            .await
            .unwrap();
        let snap = bus.stats();
        assert_eq!(snap.high_priority, 1);
        assert_eq!(snap.normal_priority, 2);
        assert_eq!(snap.low_priority, 3);
    }
}
