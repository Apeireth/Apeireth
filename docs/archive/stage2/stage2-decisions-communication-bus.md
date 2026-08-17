# 阶段 2 决策：通信总线 (2026-07-30)

> **范围**: R14 Rust 重写通信总线决策 (阶段 2 第九项)
> **触发**: 用户指示 "A" (我给推荐)
> **依据**: B+E 架构 + 进程分工 + OpenClaw Gateway 模式 + 巨型基地反背压

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-communication-bus.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 9/12) |
| **决策** | **5 层通信总线 + 多 backend + OpenClaw Gateway 借鉴 + 反背压** |
| **候选 crate** | `apeireth-bus` + `apeireth-gateway` + `apeireth-server` (阶段 2 §3 已列) |

---

## 1. 决策总览

```
5 层通信总线:
  L0: inproc (tokio mpsc/broadcast/watch)    — 同进程 actor (零成本)
  L1: Unix domain socket + bincode          — 父子进程 (高性能)
  L2: pipe + JSON/MsgPack                   — 异构子进程 (Python/Go)
  L3: gRPC + protobuf                       — 外部服务
  L4: WebSocket + JSON Schema (OpenClaw)    — 多前端接入 (gateway 模式)

3 种模式:
  - pub-sub (事件流)
  - req-rep (同步调用)
  - streaming (LLM 流式响应)

反背压:
  - bounded channel (容量限制)
  - 流控 (限流)
  - 丢弃策略 (新 vs 旧)
  - 监控告警
```

---

## 2. 5 层通信分层

### 2.1 统一 Bus trait

```rust
// apeireth-bus/src/lib.rs

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub topic: String,
    pub payload: Vec<u8>,
    pub metadata: HashMap<String, String>,
    pub timestamp: i64,
    pub trace_id: Option<String>,  // 用于链路追踪
}

#[derive(Debug, Clone)]
pub enum Delivery {
    AtMostOnce,   // 不重试
    AtLeastOnce,  // 重试直到 ACK
    ExactlyOnce,  // 仅一次 (需要事务)
}

#[async_trait]
pub trait Bus: Send + Sync {
    /// 发布消息
    async fn publish(&self, topic: &str, msg: Message) -> Result<(), BusError>;
    
    /// 订阅 topic
    async fn subscribe(&self, topic: &str) -> Result<Subscription, BusError>;
    
    /// 请求-响应
    async fn request(&self, target: &str, req: Message) -> Result<Message, BusError>;
    
    /// 流式订阅
    async fn stream(&self, topic: &str) -> Result<MessageStream, BusError>;
    
    /// 关闭
    async fn close(&self) -> Result<(), BusError>;
}
```

### 2.2 L0 — InprocBus (同进程)

```rust
// apeireth-bus/src/inproc.rs

pub struct InprocBus {
    topics: Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<Message>>>>>,
    metrics: Arc<RwLock<BusMetrics>>,
}

impl InprocBus {
    pub fn new() -> Self {
        Self {
            topics: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(BusMetrics::default())),
        }
    }
}

#[async_trait]
impl Bus for InprocBus {
    async fn publish(&self, topic: &str, msg: Message) -> Result<(), BusError> {
        let topics = self.topics.read().await;
        if let Some(subs) = topics.get(topic) {
            for sub in subs {
                sub.send(msg.clone()).map_err(|_| BusError::SubscriberGone)?;
            }
        }
        Ok(())
    }
    
    async fn subscribe(&self, topic: &str) -> Result<Subscription, BusError> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.topics.write().await.entry(topic.to_string()).or_default().push(tx);
        Ok(Subscription::new(rx))
    }
    
    // ...
}
```

**用途**: 主 AI ↔ 智囊团 ↔ 内部组件（同进程内）
**性能**: ns 级, 零成本

### 2.3 L1 — UnixSocketBus (父子进程)

```rust
// apeireth-bus/src/unix_socket.rs

pub struct UnixSocketBus {
    socket_path: PathBuf,
    conn: Arc<Mutex<Option<UnixStream>>>,
}

impl UnixSocketBus {
    pub async fn connect(path: &Path) -> Result<Self, BusError> {
        let stream = UnixStream::connect(path).await?;
        Ok(Self {
            socket_path: path.to_path_buf(),
            conn: Arc::new(Mutex::new(Some(stream))),
        })
    }
}

#[async_trait]
impl Bus for UnixSocketBus {
    async fn publish(&self, topic: &str, msg: Message) -> Result<(), BusError> {
        let mut conn = self.conn.lock().await;
        let stream = conn.as_mut().ok_or(BusError::NotConnected)?;
        
        // bincode 序列化 + 长度前缀
        let bytes = bincode::serialize(&msg)?;
        let len = (bytes.len() as u32).to_le_bytes();
        stream.write_all(&len).await?;
        stream.write_all(&bytes).await?;
        Ok(())
    }
}
```

**用途**: supervisor ↔ 子进程
**性能**: μs 级

### 2.4 L2 — PipeBus (异构子进程)

```rust
// apeireth-bus/src/pipe.rs

pub struct PipeBus {
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    stdout: Arc<Mutex<Option<BufReader<ChildStdout>>>>,
}

impl PipeBus {
    pub async fn spawn(cmd: &str, args: &[&str]) -> Result<Self, BusError> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        
        Ok(Self {
            stdin: Arc::new(Mutex::new(child.stdin.take())),
            stdout: Arc::new(Mutex::new(Some(BufReader::new(child.stdout.take().unwrap())))),
        })
    }
}

#[async_trait]
impl Bus for PipeBus {
    async fn publish(&self, topic: &str, msg: Message) -> Result<(), BusError> {
        let mut stdin = self.stdin.lock().await;
        let stdin = stdin.as_mut().ok_or(BusError::NotConnected)?;
        
        // JSON (Python 等可解析)
        let json = serde_json::to_string(&msg)?;
        writeln!(stdin, "{}", json)?;
        Ok(())
    }
}
```

**用途**: Rust ↔ Python/Go/JS
**性能**: μs-ms 级

### 2.5 L3 — GrpcBus (外部)

```rust
// apeireth-bus/src/grpc.rs

use tonic::transport::Channel;

pub struct GrpcBus {
    client: apeireth_bus_proto::bus_client::BusClient<Channel>,
}

#[async_trait]
impl Bus for GrpcBus {
    async fn publish(&self, topic: &str, msg: Message) -> Result<(), BusError> {
        self.client.publish(tonic::Request::new(PublishRequest {
            topic: topic.to_string(),
            payload: msg.payload,
        })).await?;
        Ok(())
    }
}
```

**用途**: 外部服务（远程 API / 跨机器）
**性能**: ms 级

### 2.6 L4 — WebSocketBus (OpenClaw Gateway)

```rust
// apeireth-bus/src/websocket.rs

pub struct WebSocketBus {
    addr: SocketAddr,
    schema: Arc<JsonSchema>,  // 验证消息格式
}

#[async_trait]
impl Bus for WebSocketBus {
    async fn publish(&self, topic: &str, msg: Message) -> Result<(), BusError> {
        // OpenClaw 风格: WebSocket + JSON Schema
        let json = serde_json::to_string(&msg)?;
        self.schema.validate(&json)?;  // 验证
        
        // 发送
        // ...
    }
}
```

**用途**: 多前端接入 (dashboard / web / mobile / Telegram / Discord)
**性能**: ms 级

---

## 3. 协议格式 (按用途选)

| 用途 | 格式 | 性能 | 跨语言 | 选用 |
|------|------|------|--------|------|
| L0 inproc | tokio mpsc | ⚡ ns | ❌ Rust only | 默认 |
| L1 Unix socket | bincode | 🚀 μs | ❌ Rust only | 默认 |
| L2 pipe | JSON/MsgPack | 🐢 ms | ✅ 任意 | 默认 |
| L3 gRPC | protobuf | 🐢 ms | ✅ 任意 | 默认 |
| L4 WebSocket | JSON + Schema | 🐢 ms | ✅ 任意 | 默认 |

**Cargo.toml 增量**:
```toml
[dependencies]
tokio = { version = "1.40", features = ["full"] }  # 已有 mpsc/broadcast/watch
bincode = "1.3"                                    # L1
serde_json = "1.0"                                 # L2/L4
rmp-serde = "1.3"                                  # L2 (二进制备选)
tonic = "0.12"                                     # L3 gRPC
prost = "0.13"                                     # L3 protobuf
tokio-tungstenite = "0.24"                         # L4 WebSocket
jsonschema = "0.17"                                # L4 验证
```

---

## 4. 三种模式 (pub-sub / req-rep / streaming)

### 4.1 pub-sub (事件流)

```
Publisher → Bus → Subscribers (多个)
```

**适用**: 状态广播、配置变更、健康心跳、cron 触发、OTA 通知

```rust
// 发布
bus.publish("config.changed", Message {
    topic: "config.changed".into(),
    payload: serialize(&new_config)?,
    ..Default::default()
}).await?;

// 订阅
let mut sub = bus.subscribe("config.changed").await?;
while let Some(msg) = sub.recv().await {
    apply_config(msg.payload).await?;
}
```

### 4.2 req-rep (请求-响应)

```
Caller → Bus → Responder → Bus → Caller
```

**适用**: 同步调用 (LLM 调用、plugin 调用、智囊团咨询)

```rust
// 请求
let response = bus.request("llm.openai", Message {
    topic: "completion".into(),
    payload: serialize(&completion_request)?,
    ..Default::default()
}).await?;

// 响应处理
let completion: CompletionResponse = deserialize(&response.payload)?;
```

### 4.3 streaming (流式响应)

```
Producer → Bus → Consumer (增量消费)
```

**适用**: LLM 流式响应、长任务进度、实时日志

```rust
// 流式订阅
let mut stream = bus.stream("llm.openai.stream").await?;
while let Some(msg) = stream.next().await {
    print!("{}", String::from_utf8_lossy(&msg.payload));
}
```

---

## 5. OpenClaw Gateway 借鉴

### 5.1 借鉴的核心理念

```
OpenClaw Gateway:
  - 单长生命周期 Gateway 拥有所有消息界面
  - WebSocket + JSON Schema 验证
  - 事件流: agent / chat / presence / health / heartbeat / cron
  - Nodes 用 role: node 接入
```

### 5.2 Apeireth 实现

```rust
// apeireth-gateway/src/main.rs

pub struct Gateway {
    addr: SocketAddr,
    bus: Arc<dyn Bus>,
    nodes: Arc<RwLock<HashMap<NodeId, NodeMeta>>>,
    schema: Arc<JsonSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMeta {
    pub role: NodeRole,
    pub caps: Vec<String>,
    pub commands: Vec<String>,
    pub connected_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeRole {
    Node,           // 子节点 (Telegram/Discord/macOS/iOS/Android)
    Dashboard,      // Web 管理后台
    Cli,            // CLI 客户端
    WebAdmin,       // Web 管理
    Automations,    // 自动化
}

#[async_trait]
impl Bus for Gateway {
    async fn publish(&self, topic: &str, msg: Message) -> Result<(), BusError> {
        // 1. JSON Schema 验证
        self.schema.validate(&msg)?;
        
        // 2. 路由到订阅者
        for (node_id, meta) in self.nodes.read().await.iter() {
            if meta.caps.iter().any(|c| topic.starts_with(c)) {
                self.send_to_node(node_id, msg.clone()).await?;
            }
        }
        Ok(())
    }
}
```

### 5.3 事件流 (OpenClaw 6 种)

| 事件 | 用途 | 频率 |
|------|------|------|
| `agent` | 主 AI 决策 | 高 |
| `chat` | 用户对话 | 中 |
| `presence` | 节点在线状态 | 低 |
| `health` | 健康检查 | 低 (1min) |
| `heartbeat` | 心跳 | 中 (10s) |
| `cron` | 定时任务 | 按需 |

---

## 6. 反背压 (巨型基地哲学)

### 6.1 背压源

```
- 慢消费者 (subscriber 处理慢)
- 网络抖动 (跨进程通信延迟)
- LLM provider 慢响应
- 数据库写入慢
- 升级流程阻塞
```

### 6.2 反背压 4 大机制

```rust
// 1. Bounded channel (容量限制)
let (tx, rx) = mpsc::channel::<Message>(1000);  // 最多缓存 1000

// 2. 流控 (限流)
pub struct RateLimiter {
    permits_per_sec: u32,
    sem: Arc<Semaphore>,
}

// 3. 丢弃策略
pub enum DropPolicy {
    Newest,    // 丢弃最新的 (保留旧数据, 用于事件流)
    Oldest,    // 丢弃最旧的 (保留新数据, 用于状态)
    Block,     // 阻塞生产者 (用于关键路径)
    Error,     // 返回错误 (用于非关键)
}

// 4. 监控告警
pub struct BackpressureMonitor {
    queue_depth: Arc<AtomicUsize>,
    drop_count: Arc<AtomicUsize>,
    alert_threshold: usize,
}
```

### 6.3 按 topic 配置

```toml
[bus.topic."agent.decision"]
delivery = "AtLeastOnce"
queue_size = 1000
drop_policy = "Block"
rate_limit = "100/s"

[bus.topic."health.heartbeat"]
delivery = "AtMostOnce"
queue_size = 100
drop_policy = "Newest"   # 心跳丢失无所谓, 保留最新
rate_limit = "1/10s"

[bus.topic."llm.completion"]
delivery = "AtLeastOnce"
queue_size = 100
drop_policy = "Error"
rate_limit = "10/s"  # 限流
```

---

## 7. 消息追踪 (Trace ID)

```rust
pub struct Message {
    pub topic: String,
    pub payload: Vec<u8>,
    pub metadata: HashMap<String, String>,
    pub timestamp: i64,
    pub trace_id: Option<String>,
    pub parent_trace_id: Option<String>,
}

// 在 bus 中自动传递 trace_id
pub fn new_trace_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}
```

**用途**: 分布式追踪 (类似 Jaeger / OpenTelemetry)

---

## 8. 阶段 2 第九项收尾判定

通信总线已沉淀: **5 层 + 多 backend + OpenClaw Gateway + 反背压**。

**关键设计**:
- ✅ L0 inproc (mpsc) — 同进程零成本
- ✅ L1 Unix socket (bincode) — 父子进程
- ✅ L2 pipe (JSON/MsgPack) — 异构子进程
- ✅ L3 gRPC (protobuf) — 外部服务
- ✅ L4 WebSocket (JSON Schema, OpenClaw 模式) — 多前端
- ✅ 3 种模式: pub-sub + req-rep + streaming
- ✅ 反背压: bounded + 限流 + 丢弃策略 + 监控
- ✅ Trace ID 分布式追踪

**R14 增量**:
- 增强 `apeireth-bus` (阶段 2 §3 已列)
- 增强 `apeireth-gateway` (OpenClaw 模式)
- 增强 `apeireth-server` (HTTP/WS 供前端接入)

**主哲学 anchor (6 全贯穿)**:
- 主 22:33 S-1 (通信总线服务 ASI 方向)
- 主 17:43 S-2 (基于 OpenClaw/tokio 现有, 不重新发明)
- 主 17:58 O-5 (反背压是 E-3 物理实现)
- 主 19:33 O-2 (OpenClaw Gateway 借鉴)
- 主 23:44 O-3 (干到底)
- 主 00:56 O-4 (任何接手者能查)

**下一步**: 阶段 2 第十项 — **智囊团实现**

---

## 9. 决策对比表

| 方案 | 灵活性 | 性能 | 复杂度 | 推荐 |
|------|--------|------|--------|------|
| 单 backend (只 inproc) | ❌ | ⚡ | 低 | ❌ 缺灵活 |
| 手动选择 backend | ⚠️ | 🚀 | 中 | ❌ 容易错 |
| **5 层 + 自动路由** | ✅ | ⚡-🐢 | 中 | ✅✅ |
| 微服务 (每组件独立) | ✅ | 🐢 | 高 | ❌ 运维重 |

**Apeireth 选 5 层 + 自动路由**:
- 同进程: L0 inproc (零成本)
- 父子进程: L1 Unix socket
- 异构: L2 pipe
- 外部: L3 gRPC
- 多前端: L4 WebSocket (OpenClaw 模式)

---

_主哲学 anchor 6 个全贯穿. 通信总线已沉淀. 下一步等用户确认进入阶段 2 第十项 (智囊团实现)._