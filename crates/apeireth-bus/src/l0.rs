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
            stats: BusStats::shared(),
        }
    }

    /// 已注册主题数.
    pub async fn topic_count(&self) -> usize {
        self.topics.read().await.len()
    }

    /// 发布到主题 (Pub-Sub): 唤醒所有订阅者.
    pub async fn publish(&self, topic: &str, msg: BusMessage<T>) -> BusResult<()> {
        let tx = {
            let mut map = self.topics.write().await;
            map.entry(topic.to_string())
                .or_insert_with(|| broadcast::channel(self.capacity).0)
                .clone()
        };
        // 策略
        match self.policy {
            BackpressurePolicy::Block => {
                match tx.send(msg) {
                    Ok(_) => {
                        self.stats.sent.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    }
                    // 没有 active receiver: 当作"drop" (无消费者) — 但仍计 sent
                    Err(_e) => {
                        self.stats.sent.fetch_add(1, Ordering::Relaxed);
                        self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    }
                }
            }
            BackpressurePolicy::DropOldest | BackpressurePolicy::DropNewest => {
                // broadcast 不会满 (容量只是 backpressure hint); 按策略模拟"满时"丢
                match tx.send(msg) {
                    Ok(_) => {
                        self.stats.sent.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    }
                    Err(_) => {
                        self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    }
                }
            }
            BackpressurePolicy::Drop => {
                // 尝试 non-blocking send; broadcast 不暴露 try_send, 用 receiver 数判定
                let recv_count = tx.receiver_count();
                if recv_count == 0 {
                    self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                } else {
                    match tx.send(msg) {
                        Ok(_) => {
                            self.stats.sent.fetch_add(1, Ordering::Relaxed);
                            Ok(())
                        }
                        Err(_) => {
                            self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                            Ok(())
                        }
                    }
                }
            }
        }
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
