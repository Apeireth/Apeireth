//! 宿主调度器：flush 触发时机的**可注入**边界（设计文档 §4.2 规则 1 / §4.7）。
//!
//! 设计文档 §4.7 列的 `set_scheduler(s: Scheduler)`（"注入执行器（wasm: 微任务
//! + rAF；native test: 同步）"）在本 crate 落为 [`HostScheduler`] trait。
//!
//! ## 为什么是 trait 边界而不是直接依赖
//!
//! P1 验收门（计划书 §3.2）的机械判据是「`ui-core` 的 `[dependencies]` 段里
//! `web-sys` / `wasm-bindgen` **0 条**」。若把「微任务」直接写成 `web_sys`
//! 调用，这个判据立刻变红；而 wasm 侧真正需要的只是「**什么时候**跑一次
//! flush」这一个决定权 ⇒ 把它抽象成 trait，宿主自己注入实现即可：
//!
//! ```text
//! ui-core（0 外部依赖）           宿主（wasm 侧，实现 HostScheduler）
//! ─────────────────────           ────────────────────────────────────
//! Signal::set ──► request_flush ──► HostScheduler::schedule_flush(闭包)
//!                                        │
//!                                        └─► 宿主决定此刻跑 / 排进微任务队列
//! ```
//!
//! ⇒ **注入是 trait 边界，不是依赖**：`ui-core` 自身**不引用任何宿主平台 API**，
//! 永远能 native 单测（不需浏览器、不需 wasm target）。
//!
//! ## 命名说明（避免与内部队列混淆）
//!
//! - 本 crate 的 [`crate::Scheduler`] = **内部待办队列的合并器**（pre / post 双队列
//!   + 去重 + 延迟队列），负责「谁先跑」；
//! - 本模块的 [`HostScheduler`] = **宿主注入点**，负责「什么时候跑」。
//!
//! 两者职责不同、不可互相替代。

use std::cell::RefCell;
use std::collections::VecDeque;

/// 宿主调度器：决定「flush 何时真正跑」的注入点。
///
/// 实现方（宿主）拿到的是一个「跑一次 flush」的闭包，由它决定**此刻执行**还是
/// **排队等待**（wasm 侧 = 微任务；native 测试侧 = 同步）。
pub trait HostScheduler {
    /// 请求宿主执行一次 flush。
    ///
    /// 契约（实现方必须遵守）：
    /// - **必须**在返回前或稍后**恰好调用一次** `flush`；
    /// - **不得**在 `flush` 闭包里假设运行时的内部状态（闭包只负责触发一次
    ///   flush；flush 内部的顺序保证由 [`crate::Runtime`] 负责）；
    /// - **不得**吞掉 flush 内部的错误 —— flush 的失败由运行时记录，并通过写入
    ///   路径（`try_set` / `try_update`）或 [`crate::Runtime::take_flush_error`]
    ///   回传给调用方（**不许静默降级**）。
    fn schedule_flush(&self, flush: Box<dyn FnOnce()>);
}

/// 默认宿主调度器：**同步执行** flush。
///
/// 语义（设计文档 §4.2 规则 1）：native 侧「测试里同步 drain」。
/// 注意：**同步执行 ⇒ 一次写触发一次 flush**；需要把多次写合并成一次 flush 时
/// 用 [`crate::Runtime::batch`]（设计文档 §4.2 规则 2），或改注入
/// [`QueuedScheduler`] 之类的异步实现。
#[derive(Debug, Clone, Copy, Default)]
pub struct SyncScheduler;

impl HostScheduler for SyncScheduler {
    fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
        flush();
    }
}

/// 手动驱动的宿主调度器：把 flush 排进队列，由宿主自己决定何时 drain。
///
/// 用途：
/// - **wasm 侧**：宿主把自己的微任务队列接进来（宿主侧实现，不在本 crate）；
/// - **native 侧**：测试里手动推进 ⇒ 可断言「写入后、宿主推进前，尚未发生
///   任何重算」这类**时序**语义。
#[derive(Default)]
pub struct QueuedScheduler {
    queue: RefCell<VecDeque<Box<dyn FnOnce()>>>,
}

impl std::fmt::Debug for QueuedScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueuedScheduler")
            .field("queued", &self.queued())
            .finish()
    }
}

impl QueuedScheduler {
    /// 建一个空队列。
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前排队的 flush 数。
    pub fn queued(&self) -> usize {
        self.queue.borrow().len()
    }

    /// 队列是否为空。
    pub fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }

    /// 执行队列里的全部 flush，返回**实际执行次数**。
    ///
    /// 执行过程中新排入的 flush 也会在本轮被执行（队列被 drain 到空为止）。
    pub fn drain(&self) -> usize {
        let mut executed = 0;
        loop {
            let next = self.queue.borrow_mut().pop_front();
            let Some(task) = next else {
                break;
            };
            task();
            executed += 1;
        }
        executed
    }

    /// 丢弃队列里未执行的 flush，返回**丢弃数**。
    ///
    /// 用于「宿主决定放弃这一轮」的显式路径（不是静默丢弃：调用方拿得到数字）。
    pub fn clear(&self) -> usize {
        let mut guard = self.queue.borrow_mut();
        let dropped = guard.len();
        guard.clear();
        dropped
    }
}

impl HostScheduler for QueuedScheduler {
    fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
        self.queue.borrow_mut().push_back(flush);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn sync_scheduler_runs_the_flush_immediately() {
        let scheduler = SyncScheduler;
        let ran = Rc::new(Cell::new(false));
        let flag = ran.clone();
        scheduler.schedule_flush(Box::new(move || flag.set(true)));
        assert!(ran.get(), "同步实现 ⇒ 调度即执行");
    }

    #[test]
    fn queued_scheduler_defers_until_drained() {
        let scheduler = QueuedScheduler::new();
        let ran = Rc::new(Cell::new(false));
        let flag = ran.clone();
        scheduler.schedule_flush(Box::new(move || flag.set(true)));

        assert!(!ran.get(), "排队实现 ⇒ 调度不等于执行");
        assert_eq!(scheduler.queued(), 1);
        assert!(!scheduler.is_empty());

        assert_eq!(scheduler.drain(), 1, "drain 一次 ⇒ 执行一次");
        assert!(ran.get());
        assert!(scheduler.is_empty());
    }

    #[test]
    fn queued_scheduler_drain_executes_every_queued_flush() {
        let scheduler = QueuedScheduler::new();
        let count = Rc::new(Cell::new(0));
        for _ in 0..3 {
            let cell = count.clone();
            scheduler.schedule_flush(Box::new(move || cell.set(cell.get() + 1)));
        }
        assert_eq!(scheduler.queued(), 3);
        assert_eq!(scheduler.drain(), 3);
        assert_eq!(count.get(), 3);
    }

    #[test]
    fn queued_scheduler_clear_reports_how_many_were_dropped() {
        let scheduler = QueuedScheduler::new();
        let ran = Rc::new(Cell::new(false));
        for _ in 0..2 {
            let flag = ran.clone();
            scheduler.schedule_flush(Box::new(move || flag.set(true)));
        }

        assert_eq!(scheduler.clear(), 2, "丢弃数必须可复核");
        assert!(!ran.get());
        assert_eq!(scheduler.drain(), 0);
    }

    #[test]
    fn sync_scheduler_runs_every_requested_flush_without_queueing() {
        // 合并（coalescing）是**运行时**的职责，不是调度器的 ⇒ 调度器逐个执行。
        let scheduler = SyncScheduler;
        let count = Rc::new(Cell::new(0));
        for _ in 0..2 {
            let cell = count.clone();
            scheduler.schedule_flush(Box::new(move || cell.set(cell.get() + 1)));
        }
        assert_eq!(count.get(), 2, "同步实现逐个执行，不排队");
    }
}
