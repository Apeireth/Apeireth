//! C1 规范流的**权威缓冲** (后端合同 C 批 · C1 段 · 裁决 16)。
//!
//! 归属: 权威缓冲落**运行时**注册表, 网关只做投影 (裁决 16) ⇒ 网关不持有
//! 第二份缓冲, 断线重连的续发依据也只来自本注册表。
//!
//! 目标形态:
//! - 按 (session_id, request_id) 索引一条流; 帧序号 `sequence` 从 1 单调递增
//!   (权威分配, 客户端不可自选);
//! - 权威缓冲是**有界环形缓冲** ([`STREAM_BUFFER_CAPACITY`] 帧), 超出即淘汰最旧帧;
//! - 每个订阅者一条有界 mpsc 队列 ([`STREAM_SUBSCRIBER_QUEUE_CAPACITY`]) +
//!   `try_send` ⇒ **慢订阅者丢帧 + 计数, 永不阻塞发布方**;
//! - 终帧只允许发布一次 (重复发布 ⇒ 报错), 终帧之后不再接受增量;
//! - [`StreamRegistry::replay`] 返回窗口内帧, 或返回
//!   [`ReplayOutcome::ResyncRequired`] —— **显式事实, 不是错误** (裁决 17)。
//!
//! 不做什么 (诚实边界):
//! - 缓冲是**进程内内存**: 进程重启即失, 不做落盘;
//! - 不做 stop/cancel 端点与幂等 (C2 批);
//! - 不做「一个 operation 只调一次模型」去重 (C2 批);
//! - 注册表条目无淘汰策略 (进程内长期驻留, 由 C2 批的会话生命周期收口)。

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use apeireth_protocol::canonical_stream::{
    C1DeltaFrame, C1FinalFrame, C1Frame, C1StreamState, C1_FINAL_EVENT,
};
use parking_lot::Mutex;
use tokio::sync::mpsc;

/// 权威环形缓冲的容量 (帧数)。超出即淘汰最旧帧。
pub const STREAM_BUFFER_CAPACITY: usize = 256;

/// 每个订阅者的有界队列容量 (帧数)。队列满 ⇒ 丢该帧并计数, 不阻塞发布方。
pub const STREAM_SUBSCRIBER_QUEUE_CAPACITY: usize = 64;

/// 注册表操作失败。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StreamError {
    /// 同一条流重复注册 (一条 (session_id, request_id) 只允许一条流)。
    #[error("stream already registered: {session_id}/{request_id}")]
    AlreadyRegistered {
        /// 会话 id。
        session_id: String,
        /// 请求 id。
        request_id: String,
    },
    /// 流不存在 (句柄指向的 (session_id, request_id) 未注册)。
    #[error("unknown stream: {session_id}/{request_id}")]
    UnknownStream {
        /// 会话 id。
        session_id: String,
        /// 请求 id。
        request_id: String,
    },
    /// 终帧已发布过 (终帧只允许发布一次)。
    #[error("final frame already published: {session_id}/{request_id}")]
    FinalAlreadyPublished {
        /// 会话 id。
        session_id: String,
        /// 请求 id。
        request_id: String,
    },
    /// 流已到终态 (终帧之后再发增量 ⇒ 拒)。
    #[error("stream already finalized: {session_id}/{request_id}")]
    StreamFinalized {
        /// 会话 id。
        session_id: String,
        /// 请求 id。
        request_id: String,
    },
}

/// 流身份键: (session_id, request_id)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StreamKey {
    /// 会话 id。
    pub session_id: String,
    /// 请求 id。
    pub request_id: String,
}

/// 指向注册表内一条流的句柄 (只带身份, 不带状态)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StreamHandle {
    key: StreamKey,
}

impl StreamHandle {
    /// 构造句柄 (不注册; 注册走 [`StreamRegistry::register`])。
    pub fn new(session_id: impl Into<String>, request_id: impl Into<String>) -> Self {
        Self {
            key: StreamKey {
                session_id: session_id.into(),
                request_id: request_id.into(),
            },
        }
    }

    /// 会话 id。
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.key.session_id
    }

    /// 请求 id。
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.key.request_id
    }

    /// 身份键。
    #[must_use]
    pub fn key(&self) -> &StreamKey {
        &self.key
    }
}

/// 重放结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayOutcome {
    /// 窗口内的帧 (序号严格大于请求的 `after_sequence`)。
    Window(Vec<C1Frame>),
    /// 请求的序号已早于环形窗口 ⇒ **显式事实** (不是错误): 客户端需要重同步。
    ResyncRequired {
        /// 窗口内最早的可用序号 (客户端可据此从窗口最早帧续发, 或改取回执)。
        oldest_available_sequence: u64,
    },
}

/// 一条流的权威状态。
#[derive(Debug)]
struct StreamState {
    /// 下一个待分配序号 (从 1 开始)。
    next_sequence: u64,
    /// 权威环形缓冲 (有界, 淘汰最旧帧)。
    buffer: VecDeque<C1Frame>,
    /// 终帧 (只允许一次; 存在即流已终结)。
    final_frame: Option<C1FinalFrame>,
    /// 订阅者 (每个一条有界队列)。
    subscribers: Vec<Subscriber>,
    /// 下一个订阅者 id。
    next_subscriber_id: u64,
}

impl StreamState {
    fn new() -> Self {
        Self {
            next_sequence: 1,
            buffer: VecDeque::with_capacity(STREAM_BUFFER_CAPACITY),
            final_frame: None,
            subscribers: Vec::new(),
            next_subscriber_id: 1,
        }
    }

    /// 窗口内最早的可用序号 (缓冲空 ⇒ 下一个待分配序号)。
    fn oldest_available_sequence(&self) -> u64 {
        self.buffer
            .front()
            .map(C1Frame::sequence)
            .unwrap_or(self.next_sequence)
    }
}

/// 一个订阅者 (有界队列 + 丢帧计数)。
#[derive(Debug)]
struct Subscriber {
    id: u64,
    tx: mpsc::Sender<C1Frame>,
    dropped: Arc<AtomicU64>,
}

/// C1 规范流的权威注册表 (进程内)。
#[derive(Debug, Default)]
pub struct StreamRegistry {
    streams: Mutex<HashMap<StreamKey, Arc<Mutex<StreamState>>>>,
}

impl StreamRegistry {
    /// 造一个空注册表。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一条流 (同一条流重复注册 ⇒ [`StreamError::AlreadyRegistered`]）。
    pub fn register(
        &self,
        session_id: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Result<StreamHandle, StreamError> {
        let key = StreamKey {
            session_id: session_id.into(),
            request_id: request_id.into(),
        };
        let mut streams = self.streams.lock();
        if streams.contains_key(&key) {
            return Err(StreamError::AlreadyRegistered {
                session_id: key.session_id.clone(),
                request_id: key.request_id.clone(),
            });
        }
        streams.insert(key.clone(), Arc::new(Mutex::new(StreamState::new())));
        Ok(StreamHandle { key })
    }

    /// 查一条已注册的流 (未注册 ⇒ `None`)。
    #[must_use]
    pub fn handle(&self, session_id: &str, request_id: &str) -> Option<StreamHandle> {
        let key = StreamKey {
            session_id: session_id.to_string(),
            request_id: request_id.to_string(),
        };
        self.streams
            .lock()
            .contains_key(&key)
            .then_some(StreamHandle { key })
    }

    /// 已注册的流条数 (自省用)。
    #[must_use]
    pub fn stream_count(&self) -> usize {
        self.streams.lock().len()
    }

    /// 发布一帧增量 (权威分配序号, 从 1 单调递增)。
    ///
    /// 终帧发布后再发增量 ⇒ [`StreamError::StreamFinalized`]。
    pub fn publish_delta(
        &self,
        handle: &StreamHandle,
        trace_id: Option<&str>,
        delta: &str,
    ) -> Result<u64, StreamError> {
        let entry = self.entry(handle)?;
        let mut state = entry.lock();
        if state.final_frame.is_some() {
            return Err(StreamError::StreamFinalized {
                session_id: handle.key.session_id.clone(),
                request_id: handle.key.request_id.clone(),
            });
        }
        let sequence = state.next_sequence;
        state.next_sequence += 1;
        let frame = C1Frame::Delta(C1DeltaFrame::new(
            handle.key.session_id.clone(),
            handle.key.request_id.clone(),
            trace_id,
            sequence,
            delta,
        ));
        broadcast(&mut state, &frame);
        Ok(sequence)
    }

    /// 发布终帧 (只允许一次)。
    ///
    /// 权威回填 `event` / `session_id` / `request_id` / `sequence` 四项
    /// (调用方给的 `trace_id` 与内容位原样保留) ⇒ 帧身份与序号永远由权威侧定。
    /// 第二次发布 ⇒ [`StreamError::FinalAlreadyPublished`]。
    pub fn publish_final(
        &self,
        handle: &StreamHandle,
        frame: &mut C1FinalFrame,
    ) -> Result<(), StreamError> {
        let entry = self.entry(handle)?;
        let mut state = entry.lock();
        if state.final_frame.is_some() {
            return Err(StreamError::FinalAlreadyPublished {
                session_id: handle.key.session_id.clone(),
                request_id: handle.key.request_id.clone(),
            });
        }
        let sequence = state.next_sequence;
        state.next_sequence += 1;
        frame.event = C1_FINAL_EVENT.to_string();
        frame.session_id = handle.key.session_id.clone();
        frame.request_id = handle.key.request_id.clone();
        frame.sequence = sequence;
        let stored = frame.clone();
        state.final_frame = Some(stored.clone());
        broadcast(&mut state, &C1Frame::Final(stored));
        // 终帧之后本流不再有帧 ⇒ 关闭所有订阅者通道 (订阅者读完终帧即见通道关闭)。
        state.subscribers.clear();
        Ok(())
    }

    /// 订阅一条流 (拿到一条有界队列的接收端)。
    ///
    /// 流已终态时: 订阅者立即收到终帧, 随后通道关闭。
    pub fn subscribe(&self, handle: &StreamHandle) -> Result<StreamSubscription, StreamError> {
        let entry = self.entry(handle)?;
        let mut state = entry.lock();
        let (tx, rx) = mpsc::channel(STREAM_SUBSCRIBER_QUEUE_CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        let id = state.next_subscriber_id;
        state.next_subscriber_id += 1;
        match state.final_frame.clone() {
            Some(final_frame) => {
                // 终态流: 终帧只投一次, 随后 `tx` 随本作用域结束被丢弃 ⇒ 通道关闭。
                let _ = tx.try_send(C1Frame::Final(final_frame));
            }
            None => state.subscribers.push(Subscriber {
                id,
                tx,
                dropped: dropped.clone(),
            }),
        }
        Ok(StreamSubscription { id, rx, dropped })
    }

    /// 重放: 取回窗口内序号严格大于 `after_sequence` 的帧。
    ///
    /// - 窗口内 ⇒ [`ReplayOutcome::Window`];
    /// - 早于窗口 ⇒ [`ReplayOutcome::ResyncRequired`] (**显式事实, 不是错误**)。
    pub fn replay(
        &self,
        handle: &StreamHandle,
        after_sequence: u64,
    ) -> Result<ReplayOutcome, StreamError> {
        let entry = self.entry(handle)?;
        let state = entry.lock();
        let Some(oldest) = state.buffer.front().map(C1Frame::sequence) else {
            // 窗口空 ⇒ 该流一帧未发 ⇒ 没有可重放的历史, 也没有可丢失的历史。
            return Ok(ReplayOutcome::Window(Vec::new()));
        };
        let needed = after_sequence.saturating_add(1);
        if needed < oldest {
            return Ok(ReplayOutcome::ResyncRequired {
                oldest_available_sequence: oldest,
            });
        }
        Ok(ReplayOutcome::Window(
            state
                .buffer
                .iter()
                .filter(|frame| frame.sequence() > after_sequence)
                .cloned()
                .collect(),
        ))
    }

    /// 查一条流的状态 (自省用: 窗口容量、当前序号、终态)。
    pub fn stream_snapshot(&self, handle: &StreamHandle) -> Option<StreamSnapshot> {
        let entry = self.entry(handle).ok()?;
        let state = entry.lock();
        Some(StreamSnapshot {
            buffered_frames: state.buffer.len(),
            oldest_available_sequence: state.oldest_available_sequence(),
            next_sequence: state.next_sequence,
            finalized: state.final_frame.is_some(),
            subscriber_count: state.subscribers.len(),
        })
    }

    /// 解析句柄 → 流槽位 (未注册 ⇒ [`StreamError::UnknownStream`])。
    fn entry(&self, handle: &StreamHandle) -> Result<Arc<Mutex<StreamState>>, StreamError> {
        self.streams
            .lock()
            .get(&handle.key)
            .cloned()
            .ok_or_else(|| StreamError::UnknownStream {
                session_id: handle.key.session_id.clone(),
                request_id: handle.key.request_id.clone(),
            })
    }
}

/// 一条流的自省快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSnapshot {
    /// 窗口内当前帧数。
    pub buffered_frames: usize,
    /// 窗口内最早的可用序号。
    pub oldest_available_sequence: u64,
    /// 下一个待分配序号。
    pub next_sequence: u64,
    /// 是否已发布终帧。
    pub finalized: bool,
    /// 当前订阅者数。
    pub subscriber_count: usize,
}

/// 帧入窗 + 广播 (慢订阅者丢帧并计数, 发布方永不阻塞)。
fn broadcast(state: &mut StreamState, frame: &C1Frame) {
    state.buffer.push_back(frame.clone());
    while state.buffer.len() > STREAM_BUFFER_CAPACITY {
        state.buffer.pop_front();
    }
    let mut closed = Vec::new();
    for subscriber in &state.subscribers {
        match subscriber.tx.try_send(frame.clone()) {
            Ok(()) => {}
            // 队列满 ⇒ 丢这一帧并计数 (不阻塞发布方, 不假装送达)。
            Err(mpsc::error::TrySendError::Full(_)) => {
                subscriber.dropped.fetch_add(1, Ordering::Relaxed);
            }
            // 订阅者已断开 ⇒ 回收该订阅者。
            Err(mpsc::error::TrySendError::Closed(_)) => closed.push(subscriber.id),
        }
    }
    if !closed.is_empty() {
        state
            .subscribers
            .retain(|subscriber| !closed.contains(&subscriber.id));
    }
}

/// 一条订阅 (有界队列接收端 + 丢帧计数)。
#[derive(Debug)]
pub struct StreamSubscription {
    id: u64,
    rx: mpsc::Receiver<C1Frame>,
    dropped: Arc<AtomicU64>,
}

impl StreamSubscription {
    /// 订阅 id (注册表内自增)。
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// 等下一帧 (发布方终帧之后 ⇒ 通道关闭 ⇒ 返回 `None`)。
    pub async fn recv(&mut self) -> Option<C1Frame> {
        self.rx.recv().await
    }

    /// 非阻塞取一帧 (队列空 ⇒ `None`)。
    pub fn try_recv(&mut self) -> Option<C1Frame> {
        self.rx.try_recv().ok()
    }

    /// 取出接收端 (交给传输层投影; 订阅本身随之失效)。
    ///
    /// 丢帧计数不随本次移动丢失: 注册表侧仍持有一份计数句柄。
    pub fn into_receiver(self) -> mpsc::Receiver<C1Frame> {
        self.rx
    }

    /// 因队列满而丢掉的帧数 (慢订阅者的显式计数)。
    #[must_use]
    pub fn dropped_frames(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

static GLOBAL_REGISTRY: OnceLock<StreamRegistry> = OnceLock::new();

/// 进程内全局注册表 (网关只做投影, 不持有第二份缓冲 ⇒ 只能取这一份)。
///
/// 之所以是全局而不是挂在网关状态上: 网关状态 (`GatewayState`) 定义在
/// `crates/adapters/gateway/src/panels.rs`, 该文件在本批**禁改** ⇒ 本段用
/// `OnceLock` 单例暴露权威缓冲 (裁决 16 的「权威缓冲落运行时」不因此改变)。
#[must_use]
pub fn global_registry() -> &'static StreamRegistry {
    GLOBAL_REGISTRY.get_or_init(StreamRegistry::new)
}

/// 帧的终态判定 (终帧 ⇒ 流结束)。
#[must_use]
pub fn frame_is_terminal(frame: &C1Frame) -> bool {
    match frame {
        C1Frame::Delta(delta) => delta.state.is_terminal(),
        C1Frame::Final(_) => true,
    }
}

/// 帧状态 (增量帧 ⇒ `running`; 终帧 ⇒ 其终态)。
#[must_use]
pub fn frame_state(frame: &C1Frame) -> C1StreamState {
    match frame {
        C1Frame::Delta(delta) => delta.state,
        C1Frame::Final(final_frame) => final_frame.state,
    }
}
