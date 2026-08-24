//! Central - 中央调度 (从 v1.0 apeireth-central 5.9K LOC 收敛)
//!
//! 0 装 PASS: 简化 task queue + worker pool, 完整 v1.0 era (priority queue, rate limit) 不做.

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub payload: serde_json::Value,
    pub enqueued_at_ms: i64,
}

#[derive(Default)]
pub struct CentralDispatcher {
    queue: Arc<Mutex<VecDeque<Job>>>,
    notify: Arc<Notify>,
}

impl CentralDispatcher {
    pub fn new() -> Self { Self::default() }

    /// 0 装 PASS: 真入队 (VecDeque push_back)
    pub async fn submit(&self, job: Job) {
        self.queue.lock().await.push_back(job);
        self.notify.notify_one();
    }

    /// 0 装 PASS: 真阻塞等待 + 弹出
    pub async fn take(&self) -> Job {
        loop {
            // fast path
            if let Some(j) = self.queue.lock().await.pop_front() { return j; }
            // wait
            self.notify.notified().await;
        }
    }

    pub async fn len(&self) -> usize { self.queue.lock().await.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_submit_take() {
        let d = CentralDispatcher::new();
        d.submit(Job { id: "j1".into(), payload: serde_json::json!({}), enqueued_at_ms: 0 }).await;
        let j = d.take().await;
        assert_eq!(j.id, "j1");
    }
    #[tokio::test]
    async fn test_len() {
        let d = CentralDispatcher::new();
        assert_eq!(d.len().await, 0);
        d.submit(Job { id: "a".into(), payload: serde_json::json!({}), enqueued_at_ms: 0 }).await;
        d.submit(Job { id: "b".into(), payload: serde_json::json!({}), enqueued_at_ms: 0 }).await;
        assert_eq!(d.len().await, 2);
    }
}
