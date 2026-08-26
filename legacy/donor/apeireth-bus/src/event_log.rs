//! **R229 — apeireth-bus event log / replay** (append-only log + filter replay)
//!
//! **设计**: 在 L0Bus 上挂一个 append-only event log (Vec<TimestampedEvent>), publish 时同步写,
//!   任何时候可以 `replay(topic_filter, time_filter)` 重建历史. 借鉴 Kafka log / NATS stream.
//!
//! **不假装**:
//! - 0 持久化 (in-memory only) — 进程重启即丢. 真持久化是 R229+1 范畴 (写 WAL + sqlite).
//! - 0 引外部 dep — Vec + Mutex
//! - bounded by `capacity` (默认 1024, 满了循环覆盖最旧)
//!
//! **L0 Bus 集成**: L0Bus::with_event_log(cap) 启用 event log, publish 同步 append,
//!   `L0Bus::event_log()` 拿 immutable ref, `replay_topic / replay_since / replay_count`.

#![allow(missing_docs)]

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::BusMessage;

/// **单条 log 事件** — topic + timestamp + payload + trace_id
#[derive(Debug, Clone)]
pub struct LoggedEvent<T: Clone> {
    /// Topic 名称 (e.g. "agent.bob")
    pub topic: String,
    /// Wallclock epoch ms
    pub timestamp_ms: i64,
    /// BusMessage 完整载荷
    pub message: BusMessage<T>,
}

/// **Event log** — append-only bounded ring (capacity 满时覆盖最旧)
pub struct EventLog<T: Clone> {
    /// 容量 (默认 1024)
    capacity: usize,
    /// 内部 ring (Vec 满时 shift + push, 简单 O(n) 但 n=cap 不大)
    inner: Mutex<Vec<LoggedEvent<T>>>,
}

impl<T: Clone> EventLog<T> {
    /// 新建空 log (capacity = 1024)
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// 自定义容量
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            capacity: cap,
            inner: Mutex::new(Vec::with_capacity(cap)),
        }
    }

    /// 当前事件数
    pub fn len(&self) -> usize {
        self.inner.lock().expect("event log poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Append 一个事件 (满了则 pop front)
    pub fn append(&self, event: LoggedEvent<T>) {
        let mut g = self.inner.lock().expect("event log poisoned");
        if g.len() >= self.capacity {
            g.remove(0); // O(n), 但 n <= cap
        }
        g.push(event);
    }

    /// 当前 wallclock ms
    pub fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    /// **replay topic** — 返回所有匹配 topic 的事件 (按时间升序, 旧→新)
    pub fn replay_topic(&self, topic: &str) -> Vec<LoggedEvent<T>> {
        self.inner
            .lock()
            .expect("event log poisoned")
            .iter()
            .filter(|e| e.topic == topic)
            .cloned()
            .collect()
    }

    /// **replay since** — 返回 timestamp_ms >= since 的所有事件
    pub fn replay_since(&self, since_ms: i64) -> Vec<LoggedEvent<T>> {
        self.inner
            .lock()
            .expect("event log poisoned")
            .iter()
            .filter(|e| e.timestamp_ms >= since_ms)
            .cloned()
            .collect()
    }

    /// **replay pattern** — 用 wildcard 匹配 (复用 `crate::pattern::TopicPattern`)
    pub fn replay_pattern(&self, pattern: &str) -> Vec<LoggedEvent<T>> {
        let p = crate::pattern::TopicPattern::parse(pattern);
        self.inner
            .lock()
            .expect("event log poisoned")
            .iter()
            .filter(|e| p.matches(&e.topic))
            .cloned()
            .collect()
    }

    /// **last N** — 最新 N 条 (按时间倒序, 新→旧)
    pub fn last_n(&self, n: usize) -> Vec<LoggedEvent<T>> {
        let g = self.inner.lock().expect("event log poisoned");
        let start = g.len().saturating_sub(n);
        g[start..].iter().rev().cloned().collect()
    }

    /// **clear** — 清空 log
    pub fn clear(&self) {
        self.inner.lock().expect("event log poisoned").clear();
    }

    /// **all** — 全部事件 (按时间升序)
    pub fn all(&self) -> Vec<LoggedEvent<T>> {
        self.inner.lock().expect("event log poisoned").clone()
    }
}

impl<T: Clone> Default for EventLog<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// **Shared event log factory** (Arc<EventLog<T>>)
pub fn shared_event_log<T: Clone + Send + 'static>() -> Arc<EventLog<T>> {
    Arc::new(EventLog::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(topic: &str, ts: i64, payload: u32) -> LoggedEvent<u32> {
        LoggedEvent {
            topic: topic.to_string(),
            timestamp_ms: ts,
            message: BusMessage::new(payload),
        }
    }

    #[test]
    fn new_log_empty() {
        let log: EventLog<u32> = EventLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
        assert_eq!(log.capacity(), 1024);
    }

    #[test]
    fn append_and_query() {
        let log = EventLog::with_capacity(10);
        log.append(make_event("a", 100, 1));
        log.append(make_event("b", 200, 2));
        log.append(make_event("a", 300, 3));
        assert_eq!(log.len(), 3);
        let a_events = log.replay_topic("a");
        assert_eq!(a_events.len(), 2);
        assert_eq!(a_events[0].message.payload, 1);
        assert_eq!(a_events[1].message.payload, 3);
    }

    #[test]
    fn capacity_overflow_evicts_oldest() {
        let log = EventLog::with_capacity(3);
        for i in 0..5 {
            log.append(make_event("t", i64::from(i), i as u32));
        }
        assert_eq!(log.len(), 3, "满了应保持 capacity 大小");
        let all = log.all();
        assert_eq!(all[0].message.payload, 2, "最旧 2 条应被覆盖");
        assert_eq!(all[2].message.payload, 4);
    }

    #[test]
    fn replay_since_filters_by_timestamp() {
        let log = EventLog::with_capacity(10);
        log.append(make_event("a", 100, 1));
        log.append(make_event("b", 200, 2));
        log.append(make_event("a", 300, 3));
        let since = log.replay_since(150);
        assert_eq!(since.len(), 2);
        assert_eq!(since[0].timestamp_ms, 200);
        assert_eq!(since[1].timestamp_ms, 300);
    }

    #[test]
    fn replay_pattern_with_wildcard() {
        let log = EventLog::with_capacity(10);
        log.append(make_event("agent.bob", 1, 1));
        log.append(make_event("agent.alice", 2, 2));
        log.append(make_event("system.cpu", 3, 3));
        let matched = log.replay_pattern("agent.*");
        assert_eq!(matched.len(), 2);
        let matched = log.replay_pattern("system.#");
        assert_eq!(matched.len(), 1);
        let matched = log.replay_pattern("#");
        assert_eq!(matched.len(), 3);
    }

    #[test]
    fn last_n_reverses_order() {
        let log = EventLog::with_capacity(10);
        log.append(make_event("a", 1, 10));
        log.append(make_event("a", 2, 20));
        log.append(make_event("a", 3, 30));
        let last2 = log.last_n(2);
        assert_eq!(last2.len(), 2);
        assert_eq!(last2[0].message.payload, 30, "最新在前");
        assert_eq!(last2[1].message.payload, 20);
    }

    #[test]
    fn clear_empties_log() {
        let log = EventLog::with_capacity(10);
        log.append(make_event("a", 1, 1));
        log.append(make_event("a", 2, 2));
        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn shared_event_log_creates_arc() {
        let l1: Arc<EventLog<u32>> = shared_event_log();
        let l2 = Arc::clone(&l1);
        l1.append(make_event("a", 1, 1));
        assert_eq!(l2.len(), 1);
    }
}
