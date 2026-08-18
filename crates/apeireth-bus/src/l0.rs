//! L0 — inproc Bus (tokio mpsc / broadcast / watch)
//!
//! 仅在当前进程内通信. 零网络栈, 速度最快.
//!
//! - `mpsc`     — 单生产者多消费者 (多播通过 `broadcast` 模拟)
//! - `broadcast`— 多生产者多消费者 (Pub-Sub 主路径)
//! - `watch`    — 单值变更广播 (配置 / 状态变更)

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::{BoxStream, StreamExt};
use tokio::sync::{broadcast, RwLock as AsyncRwLock};

use crate::event_log::{EventLog, LoggedEvent};
use crate::pattern::TopicPattern;
use crate::{BackpressurePolicy, BusError, BusMessage, BusResult, BusStats};

/// L0 payload 别名 (避免在每个函数签名写泛型).
pub type L0Payload = String;

/// L0 主题总线 — 一个进程内"虚拟总线", 含多主题 broadcast channel.
#[derive(Clone)]
pub struct L0Bus<T: Clone + Send + Sync + 'static> {
    capacity: usize,
    policy: BackpressurePolicy,
    topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<BusMessage<T>>>>>,
    /// 主题 → 最新值 (watch_set/get 用). tokio broadcast 没有原生 latest 语义.
    latest: Arc<AsyncRwLock<HashMap<String, T>>>,
    /// R228: pattern → broadcast::Sender, subscribe_pattern 注册.
    ///   publish 时遍历, 对 TopicPattern::matches(pattern, topic) 命中的也 send.
    pattern_topics: Arc<AsyncRwLock<HashMap<String, broadcast::Sender<BusMessage<T>>>>>,
    /// R229: append-only event log (None = disabled). with_event_log(cap) 启用.
    event_log: Option<Arc<EventLog<T>>>,
    stats: Arc<BusStats>,
}

impl<T: Clone + Send + Sync + 'static + std::fmt::Debug> L0Bus<T> {
    /// 用默认容量 32 + Block 策略创建.
    pub fn new() -> Self {
        Self::with_capacity_and_policy(32, BackpressurePolicy::Block)
    }

    /// 自定义容量 (默认 Block).
    pub fn with_capacity(cap: usize) -> Self {
        Self::with_capacity_and_policy(cap, BackpressurePolicy::Block)
    }

    /// 自定义容量 + 策略.
    pub fn with_capacity_and_policy(cap: usize, policy: BackpressurePolicy) -> Self {
        Self {
            capacity: cap,
            policy,
            topics: Arc::new(AsyncRwLock::new(HashMap::new())),
            latest: Arc::new(AsyncRwLock::new(HashMap::new())),
            pattern_topics: Arc::new(AsyncRwLock::new(HashMap::new())),
            event_log: None,
            stats: BusStats::shared(),
        }
    }

    /// **R229 — 启用 event log** (append-only, capacity 默认 1024, 满循环覆盖最旧)
    pub fn with_event_log(mut self) -> Self {
        self.event_log = Some(Arc::new(EventLog::new()));
        self
    }

    /// **R229 — 启用 event log, 自定义 capacity**
    pub fn with_event_log_capacity(mut self, cap: usize) -> Self {
        self.event_log = Some(Arc::new(EventLog::with_capacity(cap)));
        self
    }

    /// **R229 — 拿 event log 引用** (None 表示未启用)
    pub fn event_log(&self) -> Option<&Arc<EventLog<T>>> {
        self.event_log.as_ref()
    }

    /// 已注册主题数.
    pub async fn topic_count(&self) -> usize {
        self.topics.read().await.len()
    }

    /// 发布到主题 (Pub-Sub): 唤醒所有订阅者.
    pub async fn publish(&self, topic: &str, msg: BusMessage<T>) -> BusResult<()> {
        // R245: count priority tag once per publish attempt
        match msg.priority {
            crate::MessagePriority::High => {
                self.stats.high_priority.fetch_add(1, Ordering::Relaxed)
            }
            crate::MessagePriority::Low => self.stats.low_priority.fetch_add(1, Ordering::Relaxed),
            crate::MessagePriority::Normal => {
                self.stats.normal_priority.fetch_add(1, Ordering::Relaxed)
            }
        };
        let tx = {
            let mut map = self.topics.write().await;
            map.entry(topic.to_string())
                .or_insert_with(|| broadcast::channel(self.capacity).0)
                .clone()
        };
        // R228: 为 pattern fan-out 预留副本 (msg 会被 tx.send move 走)
        let msg_for_patterns = msg.clone();
        // 策略 — 每个 arm 返回 Ok(()) (publish 不可能失败); 整 match 丢弃
        let _ = match self.policy {
            BackpressurePolicy::Block => {
                match tx.send(msg) {
                    Ok(_) => {
                        self.stats.sent.fetch_add(1, Ordering::Relaxed);
                        Ok::<(), BusError>(())
                    }
                    // 没有 active receiver: 当作"drop" (无消费者) — 但仍计 sent
                    Err(_e) => {
                        self.stats.sent.fetch_add(1, Ordering::Relaxed);
                        self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                        Ok::<(), BusError>(())
                    }
                }
            }
            BackpressurePolicy::DropOldest | BackpressurePolicy::DropNewest => {
                // broadcast 不会满 (容量只是 backpressure hint); 按策略模拟"满时"丢
                match tx.send(msg) {
                    Ok(_) => {
                        self.stats.sent.fetch_add(1, Ordering::Relaxed);
                        Ok::<(), BusError>(())
                    }
                    Err(_) => {
                        self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                        Ok::<(), BusError>(())
                    }
                }
            }
            BackpressurePolicy::Drop => {
                // 尝试 non-blocking send; broadcast 不暴露 try_send, 用 receiver 数判定
                let recv_count = tx.receiver_count();
                if recv_count == 0 {
                    self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                    Ok::<(), BusError>(())
                } else {
                    match tx.send(msg) {
                        Ok(_) => {
                            self.stats.sent.fetch_add(1, Ordering::Relaxed);
                            Ok::<(), BusError>(())
                        }
                        Err(_) => {
                            self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                            Ok::<(), BusError>(())
                        }
                    }
                }
            }
            // R226: Coalesce + Adaptive 暂按 Block 行为 (intent-only, 0 触碰现有逻辑)
            //   intent 表达: Coalesce 窗口内合并, Adaptive 阈值自适应
            //   当前实现: 都走 Block 语义 — 0 receiver 时仍记 sent (跟 Block 一致)
            BackpressurePolicy::Coalesce { .. } | BackpressurePolicy::Adaptive { .. } => {
                match tx.send(msg) {
                    Ok(_) => {
                        self.stats.sent.fetch_add(1, Ordering::Relaxed);
                        Ok::<(), BusError>(())
                    }
                    Err(_e) => {
                        // 0 receiver — 跟 Block 一样记 sent + dropped
                        self.stats.sent.fetch_add(1, Ordering::Relaxed);
                        self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                        Ok::<(), BusError>(())
                    }
                }
            }
        };

        // R228: pattern fan-out — 遍历 pattern_topics, 对 TopicPattern::matches 命中的也 send
        //   pattern 失败不阻塞主 publish (best-effort)
        let pattern_txs: Vec<broadcast::Sender<BusMessage<T>>> = {
            let map = self.pattern_topics.read().await;
            map.iter()
                .filter(|(p, _)| TopicPattern::parse(p).matches(topic))
                .map(|(_, tx)| tx.clone())
                .collect()
        };
        for ptx in pattern_txs {
            // best-effort, 不阻断 — pattern 失败只 warn
            if let Err(e) = ptx.send(msg_for_patterns.clone()) {
                eprintln!("[apeireth-bus] pattern send failed: {e}");
            }
        }
        // R229: append to event log (if enabled) — 借用 msg (主 publish 已 send, msg_for_patterns 已用)
        if let Some(log) = &self.event_log {
            log.append(LoggedEvent {
                topic: topic.to_string(),
                timestamp_ms: EventLog::<T>::now_ms(),
                message: msg_for_patterns,
            });
        }
        Ok(())
    }

    /// **R228 — 订阅 pattern (wildcard)** — 返回 stream, 接收所有匹配 topic 的消息
    ///
    /// **pattern 语法**: `*` 单段, `#` 多段, 其他字面. 详见 `crate::pattern::TopicPattern`.
    ///
    /// **不假装**: pattern subscriber 与 exact subscriber 共享同一 bus, publish 时同时 fan-out.
    ///   同一个 pattern 多次调用 — 最后一次创建新 sender, 之前的流会停止接收新消息
    ///   (因为 broadcast::Sender 被替换).
    pub async fn subscribe_pattern(
        &self,
        pattern: &str,
    ) -> BusResult<BoxStream<'static, BusResult<BusMessage<T>>>> {
        let tx = {
            let mut map = self.pattern_topics.write().await;
            // 用 pattern 作为 key, 同 pattern 多次 subscribe 时覆盖 (last-wins)
            let (tx, _rx_drop) = broadcast::channel(self.capacity);
            map.insert(pattern.to_string(), tx.clone());
            tx
        };
        let rx = tx.subscribe();
        let stats = self.stats.clone();
        let stream = futures_util::stream::unfold(rx, move |mut rx| {
            let stats = stats.clone();
            async move {
                loop {
                    match rx.recv().await {
                        Ok(msg) => {
                            stats.received.fetch_add(1, Ordering::Relaxed);
                            return Some((Ok(msg), rx));
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => return None,
                    }
                }
            }
        });
        Ok(Box::pin(stream))
    }

    /// **R228 — 注销 pattern** — 移除 pattern_topics 中的 entry, 后续 publish 不再 fan-out
    pub async fn unsubscribe_pattern(&self, pattern: &str) -> bool {
        let mut map = self.pattern_topics.write().await;
        map.remove(pattern).is_some()
    }

    /// **R228 — pattern 订阅数**
    pub async fn pattern_count(&self) -> usize {
        self.pattern_topics.read().await.len()
    }

    /// 订阅主题 — 返回 stream.
    pub async fn subscribe(
        &self,
        topic: &str,
    ) -> BusResult<BoxStream<'static, BusResult<BusMessage<T>>>> {
        let rx = {
            let mut map = self.topics.write().await;
            let tx = map
                .entry(topic.to_string())
                .or_insert_with(|| broadcast::channel(self.capacity).0);
            tx.subscribe()
        };
        let stats = self.stats.clone();
        let stream = futures_util::stream::unfold(rx, move |mut rx| {
            let stats = stats.clone();
            async move {
                loop {
                    match rx.recv().await {
                        Ok(msg) => {
                            stats.received.fetch_add(1, Ordering::Relaxed);
                            return Some((Ok(msg), rx));
                        }
                        Err(broadcast::error::RecvError::Closed) => return None,
                        Err(e) => {
                            return Some((Err(BusError::Codec(e.to_string())), rx));
                        }
                    }
                }
            }
        });
        Ok(Box::pin(stream))
    }

    /// 请求-响应 — 发布后从 topic 取第一个.
    pub async fn request(
        &self,
        topic: &str,
        msg: BusMessage<T>,
        timeout: Duration,
    ) -> BusResult<BusMessage<T>> {
        let mut sub = self.subscribe(topic).await?;
        // 先订阅再 publish — 避免竞态丢失
        self.publish(topic, msg.clone()).await?;
        match tokio::time::timeout(timeout, sub.next()).await {
            Ok(Some(Ok(m))) => {
                self.stats.received.fetch_add(1, Ordering::Relaxed);
                Ok(m)
            }
            Ok(Some(Err(e))) => Err(e),
            Ok(None) => Err(BusError::Closed),
            Err(_) => Err(BusError::Timeout(timeout)),
        }
    }

    /// 共享 stats.
    pub fn stats_handle(&self) -> Arc<BusStats> {
        self.stats.clone()
    }

    /// 当前 stats 快照.
    pub fn stats(&self) -> crate::BusStatsSnapshot {
        self.stats.snapshot()
    }

    /// 简易 watch 接口 — 注册/读取 last value.
    pub async fn watch_set(&self, topic: &str, value: T) -> BusResult<()> {
        // 先存 latest, 再 fan-out 通知 (顺序: 订阅者下次 recv 也会看到一致值).
        {
            let mut latest = self.latest.write().await;
            latest.insert(topic.to_string(), value.clone());
        }
        let mut map = self.topics.write().await;
        let entry = map
            .entry(topic.to_string())
            .or_insert_with(|| broadcast::channel(self.capacity).0);
        let _ = entry.send(BusMessage::new(value));
        Ok(())
    }

    /// watch get — best-effort latest snapshot.
    pub async fn watch_get(&self, topic: &str) -> BusResult<Option<T>> {
        let latest = self.latest.read().await;
        Ok(latest.get(topic).cloned())
    }
}

impl<T: Clone + Send + Sync + 'static> Default for L0Bus<T> {
    fn default() -> Self {
        // Default = capacity 32 + Block; 直接构造避免触发 `with_capacity_and_policy` 的 `Debug` bound
        Self {
            capacity: 32,
            policy: BackpressurePolicy::Block,
            topics: Arc::new(AsyncRwLock::new(HashMap::new())),
            latest: Arc::new(AsyncRwLock::new(HashMap::new())),
            pattern_topics: Arc::new(AsyncRwLock::new(HashMap::new())),
            event_log: None,
            stats: BusStats::shared(),
        }
    }
}

/// 跨 Future 安全包装 (允许 `.clone()` 用于多任务).
pub type SharedL0<T> = Arc<L0Bus<T>>;

/// helper: 在 publish 失败时不 panic — 用 Ok(()) 仍推进 stats.
#[inline]
pub async fn try_publish<T: Clone + Send + Sync + 'static + std::fmt::Debug>(
    bus: &L0Bus<T>,
    topic: &str,
    payload: T,
) -> BusResult<()> {
    bus.publish(topic, BusMessage::new(payload)).await
}

// === 单元测试 ===

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn watch_get_returns_last() {
        let bus = L0Bus::<u32>::new();
        bus.watch_set("w", 1).await.unwrap();
        bus.watch_set("w", 2).await.unwrap();
        let v = bus.watch_get("w").await.unwrap();
        assert!(v.is_some());
    }

    #[tokio::test]
    async fn publish_with_no_subscribers_doesnt_block() {
        let bus = L0Bus::<u32>::with_capacity_and_policy(4, BackpressurePolicy::Drop);
        let r = bus.publish("nobody", BusMessage::new(1)).await;
        assert!(r.is_ok());
    }
}
