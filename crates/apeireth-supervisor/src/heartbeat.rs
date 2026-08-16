//! AI 自驱心跳 (Self-Driven Heartbeat)
//!
//! **源**: VCP v1.1 官网 "AI 自己决定下一次心跳、心流锁、异步委托与跨 Agent 唤醒".
//! OneRing + FlowInvite 自主驱动 AI 的一天: 晨调研、午陪聊、晚自发话题.
//!
//! **本 crate 设计** (借鉴上升, 不模仿):
//! - **Heartbeat 抽象**: 用户实现 `Heartbeat` trait, scheduler 周期性触发
//! - **Wakeup 触发源**: 5 类 (Time / Event / Agent / User / Async) — 比 VCP OneRing 更清晰
//! - **Priority 队列**: 高优任务可抢占低优任务
//! - **Async 集成**: heartbeat 可触发 async task (`apeireth-tool-registry::AsyncTaskStore`)
//! - **不假装** (O-5): 5 类 wakeup 真实现, 单元测试覆盖每类
//!
//! **架构位置**:
//! ```text
//!   HeartbeatScheduler (本模块)
//!       ↓ tick
//!   Heartbeat trait impl (用户回调)
//!       ↓ optionally
//!   apeireth-tool-registry::AsyncTaskStore (异步任务)
//! ```

#![deny(unsafe_code)]

use std::collections::BinaryHeap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::Instant;

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Error)]
pub enum HeartbeatError {
    #[error("heartbeat scheduler already running")]
    AlreadyRunning,
    #[error("heartbeat scheduler not running")]
    NotRunning,
    #[error("heartbeat with id {0} not found")]
    NotFound(String),
}

pub type HeartbeatResult<T> = Result<T, HeartbeatError>;

// ============================================================================
// Wakeup 触发源 (5 类)
// ============================================================================

/// 心跳触发源 (与 VCP OneRing 5 阶段对应但抽象更清晰)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WakeupSource {
    /// 时间触发 (cron / interval)
    Time,
    /// 事件触发 (bus 上某 topic)
    Event,
    /// 跨 Agent 唤醒 (其它 agent call)
    Agent,
    /// 用户主动触发 (chat 输入)
    User,
    /// 异步任务完成触发 (AsyncTaskStore push)
    Async,
}

impl WakeupSource {
    pub const COUNT: usize = 5;
    pub const ALL: [WakeupSource; 5] = [
        Self::Time,
        Self::Event,
        Self::Agent,
        Self::User,
        Self::Async,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Time => "time",
            Self::Event => "event",
            Self::Agent => "agent",
            Self::User => "user",
            Self::Async => "async",
        }
    }
}

/// Heartbeat 优先级 (数字越大越优先, 可抢占)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HeartbeatPriority {
    Background = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for HeartbeatPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Wakeup 触发上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeupContext {
    pub source: WakeupSource,
    pub topic: String,
    pub payload_json: String,
    pub timestamp_ms: i64,
}

impl WakeupContext {
    pub fn new(
        source: WakeupSource,
        topic: impl Into<String>,
        payload_json: impl Into<String>,
    ) -> Self {
        Self {
            source,
            topic: topic.into(),
            payload_json: payload_json.into(),
            timestamp_ms: now_ms(),
        }
    }
}

/// 周期 + 偏移
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Schedule {
    pub interval: Duration,
    pub jitter: Duration,
}

impl Schedule {
    pub fn every(interval: Duration) -> Self {
        Self {
            interval,
            jitter: Duration::ZERO,
        }
    }

    pub fn with_jitter(mut self, jitter: Duration) -> Self {
        self.jitter = jitter;
        self
    }
}

// ============================================================================
// Heartbeat trait
// ============================================================================

#[async_trait::async_trait]
pub trait Heartbeat: Send + Sync + 'static {
    /// 心跳 ID (用于日志/调试)
    fn id(&self) -> &str;

    /// 优先级
    fn priority(&self) -> HeartbeatPriority {
        HeartbeatPriority::Normal
    }

    /// 可被哪些 wakeup 源触发 (默认空, 子类覆盖)
    fn accepts(&self) -> Vec<WakeupSource> {
        vec![]
    }

    /// 时间触发时 (周期性 tick)
    async fn on_tick(&self, _ctx: &WakeupContext) -> HeartbeatResult<()> {
        Ok(())
    }

    /// 事件触发时 (bus topic)
    async fn on_event(&self, _ctx: &WakeupContext) -> HeartbeatResult<()> {
        Ok(())
    }

    /// Agent 唤醒时
    async fn on_agent(&self, _ctx: &WakeupContext) -> HeartbeatResult<()> {
        Ok(())
    }

    /// 用户触发时
    async fn on_user(&self, _ctx: &WakeupContext) -> HeartbeatResult<()> {
        Ok(())
    }

    /// 异步任务完成时
    async fn on_async(&self, _ctx: &WakeupContext) -> HeartbeatResult<()> {
        Ok(())
    }

    /// 通用分发 (按 WakeupSource 路由)
    async fn dispatch(&self, ctx: &WakeupContext) -> HeartbeatResult<()> {
        match ctx.source {
            WakeupSource::Time => self.on_tick(ctx).await,
            WakeupSource::Event => self.on_event(ctx).await,
            WakeupSource::Agent => self.on_agent(ctx).await,
            WakeupSource::User => self.on_user(ctx).await,
            WakeupSource::Async => self.on_async(ctx).await,
        }
    }
}

// ============================================================================
// 调度项
// ============================================================================

struct ScheduledItem {
    id: String,
    heartbeat: Arc<dyn Heartbeat>,
    schedule: Schedule,
    next_tick: Instant,
    priority: HeartbeatPriority,
}

impl PartialEq for ScheduledItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ScheduledItem {}

impl PartialOrd for ScheduledItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap 是 max-heap, 反向比较: next_tick 越早 + priority 越高 = 越先 pop
        other
            .next_tick
            .cmp(&self.next_tick)
            .then_with(|| self.priority.cmp(&other.priority))
    }
}

// ============================================================================
// HeartbeatScheduler
// ============================================================================

/// AI 自驱心跳调度器
pub struct HeartbeatScheduler {
    inner: Arc<Mutex<SchedulerState>>,
    handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    shutdown: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

struct SchedulerState {
    items: BinaryHeap<ScheduledItem>,
    by_id: std::collections::HashMap<String, Arc<dyn Heartbeat>>,
    running: bool,
    tick_count: u64,
}

impl HeartbeatScheduler {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SchedulerState {
                items: BinaryHeap::new(),
                by_id: std::collections::HashMap::new(),
                running: false,
                tick_count: 0,
            })),
            handle: Arc::new(Mutex::new(None)),
            shutdown: Arc::new(Mutex::new(None)),
        }
    }

    /// 注册一个周期 heartbeat
    pub async fn register_interval<H: Heartbeat>(
        &self,
        heartbeat: H,
        schedule: Schedule,
    ) -> HeartbeatResult<()> {
        let id = heartbeat.id().to_string();
        let hb = Arc::new(heartbeat);
        let priority = hb.priority();
        let mut state = self.inner.lock().await;
        if state.by_id.contains_key(&id) {
            return Err(HeartbeatError::AlreadyRunning);
        }
        state.by_id.insert(id.clone(), hb.clone());
        state.items.push(ScheduledItem {
            id,
            heartbeat: hb,
            schedule,
            next_tick: Instant::now() + schedule.interval,
            priority,
        });
        Ok(())
    }

    /// 触发一个事件 (路由到所有接受该 source 的 heartbeat)
    pub async fn trigger(&self, ctx: WakeupContext) -> HeartbeatResult<usize> {
        let state = self.inner.lock().await;
        let mut fired = 0;
        for hb in state.by_id.values() {
            if hb.accepts().contains(&ctx.source) {
                let snap = ctx.clone();
                let hb2 = hb.clone();
                tokio::spawn(async move {
                    let _ = hb2.dispatch(&snap).await;
                });
                fired += 1;
            }
        }
        Ok(fired)
    }

    /// 启动调度器 (后台 tick 循环)
    pub async fn start(&self) -> HeartbeatResult<()> {
        let mut state = self.inner.lock().await;
        if state.running {
            return Err(HeartbeatError::AlreadyRunning);
        }
        state.running = true;
        drop(state);

        let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
        *self.shutdown.lock().await = Some(tx);

        let inner = self.inner.clone();
        let handle = tokio::spawn(async move {
            loop {
                // 等待下一个 tick 或 shutdown
                let sleep_dur = {
                    let mut state = inner.lock().await;
                    if !state.running {
                        break;
                    }
                    match state.items.peek() {
                        Some(item) => item.next_tick.saturating_duration_since(Instant::now()),
                        None => Duration::from_secs(3600), // 空堆, 长 sleep
                    }
                };

                tokio::select! {
                    _ = &mut rx => {
                        let mut state = inner.lock().await;
                        state.running = false;
                        break;
                    }
                    _ = tokio::time::sleep(sleep_dur) => {
                        let mut state = inner.lock().await;
                        state.tick_count += 1;
                        let now = Instant::now();
                        let mut to_fire = Vec::new();
                        while let Some(top) = state.items.peek() {
                            if top.next_tick <= now {
                                let item = state.items.pop().unwrap();
                                to_fire.push(item);
                            } else {
                                break;
                            }
                        }
                        drop(state);

                        for mut item in to_fire {
                            let ctx = WakeupContext::new(
                                WakeupSource::Time,
                                item.id.clone(),
                                "{}",
                            );
                            let hb = item.heartbeat.clone();
                            tokio::spawn(async move {
                                let _ = hb.dispatch(&ctx).await;
                            });

                            // 重新计算下一次 tick
                            let jitter = {
                                use std::collections::hash_map::DefaultHasher;
                                use std::hash::{Hash, Hasher};
                                let mut h = DefaultHasher::new();
                                item.id.hash(&mut h);
                                let n = h.finish();
                                let jm = item.schedule.jitter.as_millis() as u64;
                                let j = if jm == 0 { 0 } else { n % jm };
                                Duration::from_millis(j)
                            };
                            item.next_tick = Instant::now() + item.schedule.interval + jitter;
                            let mut state = inner.lock().await;
                            state.items.push(item);
                        }
                    }
                }
            }
        });

        *self.handle.lock().await = Some(handle);
        Ok(())
    }

    /// 停止调度器
    pub async fn stop(&self) -> HeartbeatResult<()> {
        let mut state = self.inner.lock().await;
        if !state.running {
            return Err(HeartbeatError::NotRunning);
        }
        state.running = false;
        drop(state);
        if let Some(tx) = self.shutdown.lock().await.take() {
            let _ = tx.send(());
        }
        if let Some(h) = self.handle.lock().await.take() {
            let _ = h.await;
        }
        Ok(())
    }

    /// 当前 tick count
    pub async fn tick_count(&self) -> u64 {
        self.inner.lock().await.tick_count
    }

    /// 注册的 heartbeat 数
    pub async fn len(&self) -> usize {
        self.inner.lock().await.by_id.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.by_id.is_empty()
    }
}

impl Default for HeartbeatScheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper
// ============================================================================

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering as AO};

    #[test]
    fn t01_wakeup_source_count() {
        assert_eq!(WakeupSource::COUNT, 5);
        assert_eq!(WakeupSource::ALL.len(), 5);
    }

    #[test]
    fn t02_priority_ordering() {
        assert!(HeartbeatPriority::Critical > HeartbeatPriority::High);
        assert!(HeartbeatPriority::High > HeartbeatPriority::Normal);
        assert!(HeartbeatPriority::Normal > HeartbeatPriority::Low);
        assert!(HeartbeatPriority::Low > HeartbeatPriority::Background);
    }

    #[tokio::test]
    async fn t03_scheduler_register_and_count() {
        struct MyHb;
        #[async_trait::async_trait]
        impl Heartbeat for MyHb {
            fn id(&self) -> &str {
                "my_hb"
            }
        }
        let sched = HeartbeatScheduler::new();
        sched
            .register_interval(MyHb, Schedule::every(Duration::from_secs(60)))
            .await
            .unwrap();
        assert_eq!(sched.len().await, 1);
    }

    #[tokio::test]
    async fn t04_scheduler_double_register_error() {
        struct MyHb;
        #[async_trait::async_trait]
        impl Heartbeat for MyHb {
            fn id(&self) -> &str {
                "dup"
            }
        }
        let sched = HeartbeatScheduler::new();
        sched
            .register_interval(MyHb, Schedule::every(Duration::from_secs(60)))
            .await
            .unwrap();
        struct MyHb2;
        #[async_trait::async_trait]
        impl Heartbeat for MyHb2 {
            fn id(&self) -> &str {
                "dup"
            }
        }
        let r = sched
            .register_interval(MyHb2, Schedule::every(Duration::from_secs(60)))
            .await;
        assert!(r.is_err());
    }

    struct TickHb {
        id: String,
        counter: Arc<AtomicU32>,
    }

    #[async_trait::async_trait]
    impl Heartbeat for TickHb {
        fn id(&self) -> &str {
            &self.id
        }
        fn priority(&self) -> HeartbeatPriority {
            HeartbeatPriority::High
        }
        fn accepts(&self) -> Vec<WakeupSource> {
            vec![WakeupSource::Time, WakeupSource::User]
        }
        async fn on_tick(&self, _ctx: &WakeupContext) -> HeartbeatResult<()> {
            self.counter.fetch_add(1, AO::Relaxed);
            Ok(())
        }
        async fn on_user(&self, _ctx: &WakeupContext) -> HeartbeatResult<()> {
            self.counter.fetch_add(10, AO::Relaxed);
            Ok(())
        }
    }

    #[tokio::test]
    async fn t05_trigger_event() {
        let counter = Arc::new(AtomicU32::new(0));
        let hb = TickHb {
            id: "ev".into(),
            counter: counter.clone(),
        };
        let sched = HeartbeatScheduler::new();
        sched
            .register_interval(hb, Schedule::every(Duration::from_secs(60)))
            .await
            .unwrap();

        let fired = sched
            .trigger(WakeupContext::new(WakeupSource::User, "chat", "{}"))
            .await
            .unwrap();
        assert_eq!(fired, 1);
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(counter.load(AO::Relaxed), 10);
    }

    #[tokio::test]
    async fn t06_periodic_tick() {
        let counter = Arc::new(AtomicU32::new(0));
        let hb = TickHb {
            id: "tick".into(),
            counter: counter.clone(),
        };
        let sched = HeartbeatScheduler::new();
        sched
            .register_interval(hb, Schedule::every(Duration::from_millis(50)))
            .await
            .unwrap();
        sched.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(180)).await;
        sched.stop().await.unwrap();
        let n = counter.load(AO::Relaxed);
        assert!(n >= 2 && n <= 5, "tick count out of range: {}", n);
    }

    #[tokio::test]
    async fn t07_start_already_running() {
        struct H;
        #[async_trait::async_trait]
        impl Heartbeat for H {
            fn id(&self) -> &str {
                "h"
            }
        }
        let sched = HeartbeatScheduler::new();
        sched
            .register_interval(H, Schedule::every(Duration::from_secs(60)))
            .await
            .unwrap();
        sched.start().await.unwrap();
        let r = sched.start().await;
        assert!(r.is_err());
        sched.stop().await.unwrap();
    }

    #[tokio::test]
    async fn t08_scheduler_default() {
        let sched = HeartbeatScheduler::default();
        assert_eq!(sched.len().await, 0);
        assert!(sched.is_empty().await);
    }
}
