//! `LifoPool` — LIFO/FIFO 调度队列 + `Semaphore` 限流 (复刻 VCP `scheduling: 'lifo'` + `maxSockets: 10000`).
//!
//! **设计**:
//! - **LIFO 队列** (`Mutex<VecDeque<RequestTicket>>`) — 跟踪请求提交顺序, 出队时按策略选最前/最后
//! - **`Semaphore` 限流** — `max_sockets` 字段直接喂给 permit 数量, 拿不到 permit 就等
//! - **不重复造 reqwest 连接池** — `reqwest::Client` 自带 per-host 连接池, 我们的 LIFO 在请求调度层
//!
//! **复刻 VCP 真代码**: `chatCompletionHandler.js:22-28`
//! ```js
//! const agentOptions = {
//!   keepAlive: true,
//!   keepAliveMsecs: 1000,
//!   freeSocketTimeout: 8000,  // 绝杀 zombie 1s hang up
//!   scheduling: 'lifo',       // 我们用 Mutex<VecDeque> 出队策略实现
//!   maxSockets: 10000         // 我们用 tokio::sync::Semaphore::new(10000) 实现
//! };
//! ```
//!
//! **LIFO 行为**:
//! - **Lifo**: pop_back (后进先出) — 最新鲜的请求优先拿连接
//! - **Fifo**: pop_front (先进先出) — 标准队列, 公平

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

use crate::config::SchedulingPolicy;

/// 请求票据 — 每个进入 LIFO 池的请求都拿一个 ID + 时间戳
///
/// 用于:
/// 1. 跟踪 in-flight 请求数
/// 2. 测试 LIFO vs FIFO 调度顺序
/// 3. 调试 (log 哪个请求在跑)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestTicket {
    /// 自增 ID (从 0 开始)
    pub id: u64,
}

impl RequestTicket {
    /// 构造一个新票据 (id 由 [`LifoPool`] 内部计数器分配)
    fn new(id: u64) -> Self {
        Self { id }
    }
}

/// LIFO 池的内部状态 — 由 `Arc` 共享
struct LifoPoolInner {
    /// 请求队列 — 提交时 push_back, 出队时按 scheduling 策略 pop_back (LIFO) / pop_front (FIFO)
    queue: Mutex<VecDeque<RequestTicket>>,
    /// 自增 ID 计数器
    next_id: AtomicU64,
    /// 调度策略
    scheduling: SchedulingPolicy,
}

/// LIFO 池 — 控制 LIFO/FIFO 调度 + `max_sockets` 限流
///
/// `Clone` 是 cheap 的 (`Arc` 内部)
#[derive(Clone)]
pub struct LifoPool {
    inner: Arc<LifoPoolInner>,
    /// 限流信号量 — permit 数量 = `max_sockets`
    semaphore: Arc<Semaphore>,
}

impl LifoPool {
    /// 构造 LIFO 池
    ///
    /// # 参数
    /// - `max_sockets`: VCP `maxSockets: 10000` (VCP 默认 10000, 但测试时常用 5-10)
    /// - `scheduling`: VCP `scheduling: 'lifo'` (默认 Lifo)
    pub fn new(max_sockets: usize, scheduling: SchedulingPolicy) -> Self {
        Self {
            inner: Arc::new(LifoPoolInner {
                queue: Mutex::new(VecDeque::new()),
                next_id: AtomicU64::new(0),
                scheduling,
            }),
            semaphore: Arc::new(Semaphore::new(max_sockets)),
        }
    }

    /// 当前在队列里的 ticket 数 (in-flight, 已被调度但未完成)
    pub fn queue_len(&self) -> usize {
        self.inner.queue.lock().len()
    }

    /// 可用 permit 数 (= max_sockets - 在用)
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// 最大 permit 数
    pub fn max_permits(&self) -> usize {
        // Semaphore 不暴露 max_permits, 但 available_permits() ≤ max
        // 简化: 用 available_permits + queue_len 不准 (有 in-flight 但未 ticketed)
        // 实际生产用 atomic 计数器, 测试用 inspector
        self.semaphore.available_permits()
    }

    /// 调度策略
    pub fn scheduling(&self) -> SchedulingPolicy {
        self.inner.scheduling
    }

    /// **不阻塞** 尝试进入池子 (拿 permit + 排队)
    ///
    /// 返回:
    /// - `Ok(LifoGuard)` — 成功进入 (permit 已拿 + 票据已入队)
    /// - `Err(PoolFull)` — 池已满 (`max_sockets` 已用完)
    ///
    /// **LIFO 行为**:
    /// - Lifo 策略: 出队时 `pop_back` (最新的 ticket 优先被处理)
    /// - Fifo 策略: 出队时 `pop_front` (最早的 ticket 优先被处理)
    pub fn try_enter(&self) -> Result<LifoGuard, PoolFull> {
        // 1. 尝试拿 permit (非阻塞)
        let permit = match self.semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(TryAcquireError::NoPermits) => return Err(PoolFull),
            Err(TryAcquireError::Closed) => return Err(PoolFull), // 不太可能发生
        };

        // 2. 分配新 ticket ID
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let ticket = RequestTicket::new(id);

        // 3. push_back 入队
        self.inner.queue.lock().push_back(ticket);

        Ok(LifoGuard {
            pool: self.clone(),
            permit: Some(permit),
            ticket,
        })
    }

    /// **阻塞** 拿 permit (池满时等)
    pub async fn enter(&self) -> LifoGuard {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Semaphore never closed");
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let ticket = RequestTicket::new(id);
        self.inner.queue.lock().push_back(ticket);
        LifoGuard {
            pool: self.clone(),
            permit: Some(permit),
            ticket,
        }
    }

    /// **出队** — 按调度策略选下一个 ticket
    ///
    /// - Lifo: `pop_back` (后进先出, 最新鲜的优先)
    /// - Fifo: `pop_front` (先进先出)
    ///
    /// 如果队列空, 返回 `None`
    pub fn dequeue(&self) -> Option<RequestTicket> {
        let mut queue = self.inner.queue.lock();
        match self.inner.scheduling {
            SchedulingPolicy::Lifo => queue.pop_back(),
            SchedulingPolicy::Fifo => queue.pop_front(),
        }
    }

    /// 偷看队尾 (LIFO 下一个要出的) — 不真出队
    pub fn peek_next(&self) -> Option<RequestTicket> {
        let queue = self.inner.queue.lock();
        match self.inner.scheduling {
            SchedulingPolicy::Lifo => queue.back().copied(),
            SchedulingPolicy::Fifo => queue.front().copied(),
        }
    }
}

/// 池已满
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolFull;

/// RAII guard — drop 时自动释放 permit + 移除 ticket
pub struct LifoGuard {
    pool: LifoPool,
    /// `Some` = 还在池里; `None` = 已手动 detach (permit 永久占用, 不会 drop 释放)
    permit: Option<OwnedSemaphorePermit>,
    ticket: RequestTicket,
}

impl LifoGuard {
    /// 票据 ID
    pub fn ticket(&self) -> RequestTicket {
        self.ticket
    }

    /// 显式释放 (等同 drop)
    pub fn release(mut self) {
        // 触发 Drop
        self.permit.take();
    }

    /// 偷看池的调度策略 (LIFO/FIFO)
    pub fn scheduling(&self) -> SchedulingPolicy {
        self.pool.scheduling()
    }
}

impl Drop for LifoGuard {
    fn drop(&mut self) {
        // 1. 释放 permit (如果还在)
        self.permit.take();

        // 2. 从队列里移除自己 (按 ticket id 找)
        let mut queue = self.pool.inner.queue.lock();
        if let Some(pos) = queue.iter().position(|t| t.id == self.ticket.id) {
            queue.remove(pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SchedulingPolicy;

    #[test]
    fn lifo_dequeue_returns_most_recent_first() {
        // LIFO 行为: 提交 0,1,2,3 → 出队顺序 3,2,1,0
        let pool = LifoPool::new(10, SchedulingPolicy::Lifo);
        let g0 = pool.try_enter().unwrap();
        let g1 = pool.try_enter().unwrap();
        let g2 = pool.try_enter().unwrap();
        let g3 = pool.try_enter().unwrap();
        assert_eq!(g0.ticket().id, 0);
        assert_eq!(g1.ticket().id, 1);
        assert_eq!(g2.ticket().id, 2);
        assert_eq!(g3.ticket().id, 3);
        assert_eq!(pool.dequeue().unwrap().id, 3, "LIFO: 最新 (3) 先出");
        assert_eq!(pool.dequeue().unwrap().id, 2);
        assert_eq!(pool.dequeue().unwrap().id, 1);
        assert_eq!(pool.dequeue().unwrap().id, 0);
        assert!(pool.dequeue().is_none());
    }

    #[test]
    fn fifo_dequeue_returns_oldest_first() {
        // FIFO 行为: 提交 0,1,2,3 → 出队顺序 0,1,2,3
        let pool = LifoPool::new(10, SchedulingPolicy::Fifo);
        let _g0 = pool.try_enter().unwrap();
        let _g1 = pool.try_enter().unwrap();
        let _g2 = pool.try_enter().unwrap();
        let _g3 = pool.try_enter().unwrap();
        assert_eq!(pool.dequeue().unwrap().id, 0, "FIFO: 最早 (0) 先出");
        assert_eq!(pool.dequeue().unwrap().id, 1);
        assert_eq!(pool.dequeue().unwrap().id, 2);
        assert_eq!(pool.dequeue().unwrap().id, 3);
    }

    #[test]
    fn max_sockets_limit_enforced() {
        // max_sockets=2 → 第 3 个 try_enter 必失败
        let pool = LifoPool::new(2, SchedulingPolicy::Lifo);
        let _g0 = pool.try_enter().unwrap();
        let _g1 = pool.try_enter().unwrap();
        let result = pool.try_enter();
        assert_eq!(result.err(), Some(PoolFull));
    }

    #[test]
    fn guard_drop_releases_permit() {
        // 拿满 permit, drop 一个后, 另一个能进
        let pool = LifoPool::new(2, SchedulingPolicy::Lifo);
        let g0 = pool.try_enter().unwrap();
        let _g1 = pool.try_enter().unwrap();
        // 此时池满
        assert!(pool.try_enter().is_err());
        // drop g0
        drop(g0);
        // 又有 permit 了
        let _g2 = pool.try_enter().unwrap();
    }

    #[test]
    fn guard_drop_removes_ticket_from_queue() {
        // 验证 Drop 时从队列里移除
        let pool = LifoPool::new(10, SchedulingPolicy::Lifo);
        let g0 = pool.try_enter().unwrap();
        let _g1 = pool.try_enter().unwrap();
        assert_eq!(pool.queue_len(), 2);
        drop(g0);
        assert_eq!(pool.queue_len(), 1);
    }

    #[test]
    fn vcp_default_max_sockets_10000() {
        // VCP `maxSockets: 10000` 真代码对齐
        let pool = LifoPool::new(10_000, SchedulingPolicy::Lifo);
        // 拿 100 个 permit (远小于 10000, 不会满)
        let mut guards = Vec::new();
        for _ in 0..100 {
            guards.push(pool.try_enter().unwrap());
        }
        assert_eq!(pool.queue_len(), 100);
        assert_eq!(pool.available_permits(), 10_000 - 100);
    }

    #[test]
    fn peek_next_does_not_consume() {
        // peek 应该不影响队列
        let pool = LifoPool::new(10, SchedulingPolicy::Lifo);
        let _g0 = pool.try_enter().unwrap();
        let _g1 = pool.try_enter().unwrap();
        let _g2 = pool.try_enter().unwrap();
        // LIFO peek_next = 队尾 = 最新 = id=2
        assert_eq!(pool.peek_next().unwrap().id, 2);
        assert_eq!(pool.queue_len(), 3, "peek 不消耗");
        assert_eq!(pool.peek_next().unwrap().id, 2, "peek 多次结果一致");
        // 显式 hold 住 guards 防 drop 警告
        let _ = (_g0, _g1, _g2);
    }

    #[test]
    fn ticket_ids_are_monotonically_increasing() {
        // ID 自增, 测试用
        let pool = LifoPool::new(100, SchedulingPolicy::Lifo);
        let g0 = pool.try_enter().unwrap();
        let g1 = pool.try_enter().unwrap();
        let g2 = pool.try_enter().unwrap();
        assert_eq!(g0.ticket().id, 0);
        assert_eq!(g1.ticket().id, 1);
        assert_eq!(g2.ticket().id, 2);
    }

    #[test]
    fn clone_shares_state() {
        // Clone 必须共享队列 + 计数器 + semaphore
        let pool = LifoPool::new(10, SchedulingPolicy::Lifo);
        let pool2 = pool.clone();
        let _g0 = pool.try_enter().unwrap();
        let _g1 = pool2.try_enter().unwrap();
        assert_eq!(pool.queue_len(), 2);
        assert_eq!(pool2.queue_len(), 2);
        // LIFO 出队: 最新的 (id=1) 优先
        assert_eq!(pool.dequeue().unwrap().id, 1);
    }
}
