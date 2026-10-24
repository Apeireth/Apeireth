//! 运行时：依赖图 + 调度器 + 节点表 + 追踪栈。
//!
//! 设计文档 §4.2 / §4.4 / §4.5 的落点：
//! - 一次 flush 的顺序恒为 **脏传播 → 渲染前任务 → effect**；
//! - 依赖边在**读**时登记、在**重跑前**清空 ⇒ 条件依赖天然正确；
//! - 写信号 ⇒ 沿依赖图**主动标脏**（推），派生求值仍是**惰性**（拉）。
//!
//! ## flush 何时跑：由宿主注入决定
//!
//! 写操作不直接同步渲染，而是向**宿主调度器**请求一次 flush
//! （[`crate::HostScheduler`])。默认实现是同步执行（native 侧）；wasm 侧注入
//! 微任务实现 ⇒ 同一轮事件处理里的多次写合并为一次 flush（设计文档 §4.2
//! 规则 1）。**注入是 trait 边界，不是依赖** ⇒ `ui-core` 仍 0 依赖 web-sys。

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashSet};
use std::rc::{Rc, Weak};

use crate::derived::{Derived, DerivedOps};
use crate::effect::{EffectBody, EffectHandle, EffectState, Scope};
use crate::error::ReactiveError;
use crate::graph::{DependencyGraph, NodeId};
use crate::host_scheduler::{HostScheduler, SyncScheduler};
use crate::scheduler::Scheduler;
use crate::signal::Signal;

/// 单次 flush 内单个节点的最大执行次数（自激循环兜底上限，设计文档 §4.2 规则 8）。
///
/// 取值 100 是**设计常量**（非实测）：作用是兜住「effect 写自己的依赖」这类
/// 自激循环，而不是让它把宿主挂死。
pub const MAX_NODE_RUNS_PER_FLUSH: usize = 100;

/// 追踪帧：当前正在执行的节点（effect 执行中 / 派生求值中）。
#[derive(Debug, Clone, Copy)]
pub(crate) enum TrackingFrame {
    /// effect 执行帧。
    Effect(NodeId),
    /// 派生求值帧。
    Derived(NodeId),
}

impl TrackingFrame {
    /// 本帧对应的节点。
    pub(crate) fn node_id(self) -> NodeId {
        match self {
            TrackingFrame::Effect(id) | TrackingFrame::Derived(id) => id,
        }
    }

    /// 本帧是否是派生求值帧（派生期间禁止写信号）。
    pub(crate) fn is_derived(self) -> bool {
        matches!(self, TrackingFrame::Derived(_))
    }
}

/// 运行期共享状态：依赖图 / 调度器 / 节点表 / 追踪栈。
pub(crate) struct RuntimeInner {
    /// 依赖图（信号 / 派生 / effect 共用）。
    pub(crate) graph: RefCell<DependencyGraph>,
    /// 待办调度器（内部队列合并器：pre / post 双队列）。
    pub(crate) scheduler: RefCell<Scheduler>,
    /// 宿主调度器（flush 触发时机的注入点；默认同步执行）。
    pub(crate) host_scheduler: RefCell<Rc<dyn HostScheduler>>,
    /// 当前追踪栈（读取发生在栈顶节点的追踪域内 ⇒ 登记依赖）。
    pub(crate) tracking: RefCell<Vec<TrackingFrame>>,
    /// effect 节点表。
    pub(crate) effects: RefCell<BTreeMap<NodeId, EffectState>>,
    /// 派生节点表（类型擦除：运行期只负责「标脏」，求值由派生自身完成）。
    pub(crate) derived_nodes: RefCell<BTreeMap<NodeId, Rc<dyn DerivedOps>>>,
    /// 自引用弱指针（请求 flush 时需要构造 [`Runtime`]；用弱引用避免引用环）。
    self_ref: RefCell<Option<Weak<RuntimeInner>>>,
    next_id: Cell<usize>,
    batch_depth: Cell<usize>,
    flushing: Cell<bool>,
    flush_pending: Cell<bool>,
    /// 已向宿主请求过 flush、但尚未真正执行（用于把多次写合并成一次 flush）。
    flush_requested: Cell<bool>,
    /// 上一次 flush 的失败（**不许静默降级**：错误必须能回传给调用方）。
    flush_error: RefCell<Option<ReactiveError>>,
    node_runs: RefCell<BTreeMap<NodeId, usize>>,
}

impl RuntimeInner {
    /// 建一套空的运行期状态。
    pub(crate) fn new() -> Self {
        Self {
            graph: RefCell::new(DependencyGraph::new()),
            scheduler: RefCell::new(Scheduler::new()),
            host_scheduler: RefCell::new(Rc::new(SyncScheduler)),
            tracking: RefCell::new(Vec::new()),
            effects: RefCell::new(BTreeMap::new()),
            derived_nodes: RefCell::new(BTreeMap::new()),
            self_ref: RefCell::new(None),
            next_id: Cell::new(0),
            batch_depth: Cell::new(0),
            flushing: Cell::new(false),
            flush_pending: Cell::new(false),
            flush_requested: Cell::new(false),
            flush_error: RefCell::new(None),
            node_runs: RefCell::new(BTreeMap::new()),
        }
    }

    /// 分配一个新的节点编号。
    pub(crate) fn alloc_id(&self) -> NodeId {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        NodeId(id)
    }

    /// 当前追踪栈顶的节点（无追踪域时为 `None`）。
    pub(crate) fn current_frame(&self) -> Option<NodeId> {
        self.tracking.borrow().last().map(|frame| frame.node_id())
    }

    /// 当前是否处于派生求值域内（派生期间禁止写信号）。
    pub(crate) fn in_derivation(&self) -> bool {
        self.tracking
            .borrow()
            .last()
            .map(|frame| frame.is_derived())
            .unwrap_or(false)
    }

    /// 追踪栈里是否已出现某节点（求值环检测）。
    pub(crate) fn tracking_contains(&self, id: NodeId) -> bool {
        self.tracking
            .borrow()
            .iter()
            .any(|frame| frame.node_id() == id)
    }

    /// 求值环的完整路径（供错误信息使用）。
    pub(crate) fn tracking_path(&self, id: NodeId) -> Vec<NodeId> {
        self.tracking
            .borrow()
            .iter()
            .map(|frame| frame.node_id())
            .chain(std::iter::once(id))
            .collect()
    }

    /// 压入追踪帧（返回的守卫在 drop 时自动弹栈 ⇒ panic 也不会污染追踪栈）。
    pub(crate) fn push_frame(&self, frame: TrackingFrame) -> TrackingGuard<'_> {
        self.tracking.borrow_mut().push(frame);
        TrackingGuard { inner: self }
    }

    /// 登记依赖：当前追踪域 ↔ `node`。
    pub(crate) fn track(&self, node: NodeId) {
        if let Some(frame) = self.current_frame() {
            self.graph.borrow_mut().subscribe(frame, node);
        }
    }

    /// 沿依赖图主动标脏（推）：派生 ⇒ 标脏缓存并继续向下传播；effect ⇒ 排队。
    pub(crate) fn propagate_dirty(&self, source: NodeId) {
        let mut stack: Vec<NodeId> = self.graph.borrow().dependents_of(source);
        let mut seen: HashSet<NodeId> = HashSet::new();
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            if self.is_derived(node) {
                self.invalidate_derived(node);
                stack.extend(self.graph.borrow().dependents_of(node));
            } else if self.is_effect(node) {
                self.mark_effect_dirty(node);
            }
        }
    }

    /// 该节点是否是派生节点。
    pub(crate) fn is_derived(&self, node: NodeId) -> bool {
        self.derived_nodes.borrow().contains_key(&node)
    }

    /// 该节点是否是 effect 节点。
    pub(crate) fn is_effect(&self, node: NodeId) -> bool {
        self.effects.borrow().contains_key(&node)
    }

    /// 把派生节点标脏（缓存失效）。
    pub(crate) fn invalidate_derived(&self, node: NodeId) {
        let handle = self.derived_nodes.borrow().get(&node).cloned();
        if let Some(handle) = handle {
            handle.invalidate();
        }
    }

    /// 把 effect 节点标脏并排入渲染后队列。
    pub(crate) fn mark_effect_dirty(&self, node: NodeId) {
        let mut dirty = false;
        if let Some(state) = self.effects.borrow_mut().get_mut(&node) {
            if !state.disposed {
                state.dirty = true;
                dirty = true;
            }
        }
        if dirty {
            self.scheduler.borrow_mut().schedule_post(node);
        }
    }

    /// 向宿主请求一次 flush（写操作的触发点，设计文档 §4.2 规则 1）。
    ///
    /// 三条短路：
    /// 1. **flush 进行中** ⇒ 由本次 flush 的循环接管（防重入，不重复请求）；
    /// 2. **批次中** ⇒ 推迟到批次结束（设计文档 §4.2 规则 2）；
    /// 3. **已请求过、尚未执行** ⇒ 合并（同一轮的多次写只触发一次 flush）。
    ///
    /// 其余情况 ⇒ 交给宿主调度器决定何时真正执行。
    pub(crate) fn request_flush(&self) {
        if self.flushing.get() {
            // 本次 flush 的循环会把新排入的待办一并跑掉 ⇒ 不必再请求。
            return;
        }
        if self.batch_depth.get() > 0 {
            self.flush_pending.set(true);
            return;
        }
        if self.flush_requested.get() {
            return;
        }
        self.flush_requested.set(true);

        let inner_weak = self
            .self_ref
            .borrow()
            .clone()
            .expect("运行时的自引用弱指针必须已设置（由 Runtime::new 建立）");
        let Some(inner) = inner_weak.upgrade() else {
            // 运行时已被释放 ⇒ 没有可 flush 的对象（不是静默降级：对象已不存在）。
            return;
        };
        let host = self.host_scheduler.borrow().clone();
        let callback_weak = Rc::downgrade(&inner);
        host.schedule_flush(Box::new(move || {
            let Some(callback_inner) = callback_weak.upgrade() else {
                return;
            };
            let runtime = Runtime::from_inner(callback_inner);
            if let Err(err) = runtime.try_flush() {
                // 失败必须能被回传（不许静默降级）。
                *runtime.inner.flush_error.borrow_mut() = Some(err);
            }
        }));
    }

    /// 取走上一次 flush 的失败（若有）。
    pub(crate) fn take_flush_error(&self) -> Option<ReactiveError> {
        self.flush_error.borrow_mut().take()
    }
}

/// 追踪帧守卫：drop 时自动弹栈。
pub(crate) struct TrackingGuard<'a> {
    inner: &'a RuntimeInner,
}

impl Drop for TrackingGuard<'_> {
    fn drop(&mut self) {
        self.inner.tracking.borrow_mut().pop();
    }
}

/// 一次 flush 的执行报告。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlushReport {
    /// 本次 flush 执行的 effect 总数（含渲染前任务）。
    pub effects_run: usize,
    /// 本次 flush 执行的渲染前任务数。
    pub effects_pre_run: usize,
    /// 本次 flush 内因写入而被合并掉的重复排入次数。
    pub merged_schedules: usize,
}

/// 响应式运行时：持有依赖图 / 调度器 / 节点表。
///
/// 运行时是**显式**对象（不依赖全局状态）⇒ 单元测试可各自持有独立运行时，
/// 互不干扰；模块级自由函数（[`crate::signal`] / [`crate::effect`] 等）走线程
/// 本地的默认运行时，只是本结构的语法糖。
#[derive(Clone)]
pub struct Runtime {
    inner: Rc<RuntimeInner>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("nodes", &self.inner.next_id.get())
            .field("pending", &self.pending())
            .finish()
    }
}

impl Runtime {
    /// 建一套空的运行时。
    pub fn new() -> Self {
        let inner = Rc::new(RuntimeInner::new());
        *inner.self_ref.borrow_mut() = Some(Rc::downgrade(&inner));
        Self { inner }
    }

    /// 内部：从共享的内部状态构造运行时（请求 flush 时需要）。
    pub(crate) fn from_inner(inner: Rc<RuntimeInner>) -> Self {
        Self { inner }
    }

    /// 建一个信号（可写状态）。
    pub fn signal<T: Clone>(&self, value: T) -> Signal<T> {
        Signal::new(self.inner.clone(), value)
    }

    /// 建一个派生值（惰性求值 + 主动失效）。
    pub fn derived<T: Clone + 'static>(&self, compute: impl Fn() -> T + 'static) -> Derived<T> {
        Derived::new(self.inner.clone(), compute)
    }

    /// 建一个 effect（渲染后执行）。
    pub fn effect<F: FnMut() + 'static>(&self, f: F) -> EffectHandle {
        self.spawn_effect(f, false)
    }

    /// 建一个渲染前 effect（渲染前执行）。
    pub fn effect_pre<F: FnMut() + 'static>(&self, f: F) -> EffectHandle {
        self.spawn_effect(f, true)
    }

    /// 建一个作用域（组件生命周期边界）。
    pub fn scope(&self) -> Scope {
        Scope::new(Rc::new(self.clone()))
    }

    /// 注入宿主调度器（决定 flush 何时真正跑）。
    ///
    /// 默认实现是**同步执行**（native 侧）；wasm 侧注入微任务实现 ⇒ 同一轮事件
    /// 处理里的多次写合并为一次 flush（设计文档 §4.2 规则 1）。
    ///
    /// **注入是 trait 边界，不是依赖** ⇒ `ui-core` 的 `[dependencies]` 段仍为
    /// 0 条（计划书 §3.2 的机械判据）。
    pub fn set_host_scheduler(&self, scheduler: Rc<dyn HostScheduler>) {
        *self.inner.host_scheduler.borrow_mut() = scheduler;
    }

    /// 同步跑完当前所有待办（测试用；生产侧由宿主调度器驱动）。
    ///
    /// # Panics
    ///
    /// 出现自激循环（单次 flush 内某节点执行次数超上限）时 panic，
    /// 错误类型见 [`ReactiveError::FlushCycle`]。
    pub fn flush(&self) -> FlushReport {
        match self.try_flush() {
            Ok(report) => report,
            Err(err) => panic!("{err}"),
        }
    }

    /// 同步跑完当前所有待办；自激循环等错误以返回值报出（不 panic）。
    pub fn try_flush(&self) -> Result<FlushReport, ReactiveError> {
        // 本次调用**消费掉**之前向宿主请求的那次 flush（无论本次是否真的落盘：
        // 若因批次 / 重入而推迟，本次调用不成立 ⇒ 必须允许后续重新请求，
        // 否则「宿主在批次中途推进队列」会让这次 flush 永久丢失）。
        self.inner.flush_requested.set(false);
        if self.inner.flushing.get() {
            // 重入保护：嵌套 flush 直接返回空报告（外层 flush 会把新待办一并跑掉）。
            return Ok(FlushReport::default());
        }
        if self.inner.batch_depth.get() > 0 {
            // 批次内不落盘：推迟到批次结束（设计文档 §4.2 规则 2）。
            self.inner.flush_pending.set(true);
            return Ok(FlushReport::default());
        }

        self.inner.flushing.set(true);
        self.inner.node_runs.borrow_mut().clear();
        let mut report = FlushReport::default();
        let result = self.flush_inner(&mut report);
        self.inner.flushing.set(false);
        // flush 期间新建的 effect ⇒ 推迟到下一次 flush（防重入）。
        self.inner.scheduler.borrow_mut().promote_deferred();
        result?;
        Ok(report)
    }

    /// 批次：把多次写入合并成一次 flush（可嵌套）。
    pub fn batch<F: FnOnce()>(&self, f: F) {
        self.inner.batch_depth.set(self.inner.batch_depth.get() + 1);
        {
            let _guard = BatchGuard { inner: &self.inner };
            f();
        }
        // 批次结束：批次内请求过的 flush 在此刻落一次（合并写入）。
        if self.inner.batch_depth.get() == 0 && self.inner.flush_pending.get() {
            self.inner.flush_pending.set(false);
            self.inner.request_flush();
        }
    }

    /// 取走上一次 flush 的失败（若有）。
    ///
    /// 宿主调度器是**异步**时（wasm 微任务），flush 的失败发生在写入之后 ⇒
    /// 由宿主通过本方法取走（**不许静默降级**：失败一定会被报告）。
    pub fn take_flush_error(&self) -> Option<ReactiveError> {
        self.inner.take_flush_error()
    }

    /// 当前待办数（渲染前 + 渲染后 + 延迟）。
    pub fn pending(&self) -> usize {
        self.inner.scheduler.borrow().pending()
    }

    /// 读依赖图（只读视图）。
    pub fn graph(&self) -> std::cell::Ref<'_, DependencyGraph> {
        self.inner.graph.borrow()
    }

    /// 读调度器（只读视图）。
    pub fn scheduler(&self) -> std::cell::Ref<'_, Scheduler> {
        self.inner.scheduler.borrow()
    }

    /// 给节点挂人类可读标签（报错时用于定位）。
    pub fn set_label(&self, node: NodeId, label: impl Into<String>) {
        self.inner.graph.borrow_mut().set_label(node, label);
    }

    /// 读节点标签。
    pub fn label(&self, node: NodeId) -> Option<String> {
        self.inner.graph.borrow().label(node).map(str::to_owned)
    }

    /// 把环路径渲染成**人可读**的一行（带节点标签；无标签时退回节点编号）。
    ///
    /// 设计文档 §4.4 的硬要求：「报错必须自带人名」—— 节点可通过
    /// [`Runtime::set_label`] 挂标签，报错路径里带上标签才可定位。
    pub fn describe_cycle_path(&self, path: &[NodeId]) -> String {
        let graph = self.inner.graph.borrow();
        path.iter()
            .map(|node| {
                graph
                    .label(*node)
                    .map(str::to_owned)
                    .unwrap_or_else(|| node.to_string())
            })
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    /// 内部：创建一个 effect 节点并排入对应队列。
    pub(crate) fn spawn_effect<F: FnMut() + 'static>(&self, f: F, pre: bool) -> EffectHandle {
        let id = self.inner.alloc_id();
        let body: EffectBody = Rc::new(RefCell::new(f));
        self.inner.effects.borrow_mut().insert(
            id,
            EffectState {
                body: body.clone(),
                dirty: true,
                runs: 0,
                cleanups: Vec::new(),
                disposed: false,
            },
        );
        let handle = EffectHandle {
            id,
            inner: self.inner.clone(),
        };
        if self.inner.flushing.get() {
            self.inner.scheduler.borrow_mut().schedule_deferred(id);
        } else if pre {
            self.inner.scheduler.borrow_mut().schedule_pre(id);
        } else {
            self.inner.scheduler.borrow_mut().schedule_post(id);
        }
        handle
    }

    /// flush 的两阶段主体。
    fn flush_inner(&self, report: &mut FlushReport) -> Result<(), ReactiveError> {
        // ① 渲染前阶段（DOM 写入批次挂在这里）。
        loop {
            let batch = self.inner.scheduler.borrow_mut().take_pre();
            if batch.is_empty() {
                break;
            }
            for id in batch {
                self.check_run_budget(id)?;
                self.run_effect(id, report);
                report.effects_pre_run += 1;
            }
        }
        // ② 渲染后阶段（effect）。
        loop {
            let batch = self.inner.scheduler.borrow_mut().take_post();
            if batch.is_empty() {
                break;
            }
            for id in batch {
                self.check_run_budget(id)?;
                self.run_effect(id, report);
            }
        }
        Ok(())
    }

    /// 执行单个 effect。
    fn run_effect(&self, id: NodeId, report: &mut FlushReport) {
        let body = {
            let effects = self.inner.effects.borrow();
            match effects.get(&id) {
                Some(state) if !state.disposed => Some(state.body.clone()),
                _ => None,
            }
        };
        let Some(body) = body else {
            return;
        };
        // 重跑前清空旧依赖 ⇒ 条件依赖正确（设计文档 §4.1）。
        self.inner.graph.borrow_mut().clear_dependencies(id);
        {
            let _guard = self.inner.push_frame(TrackingFrame::Effect(id));
            let mut guard = body.borrow_mut();
            guard();
        }
        if let Some(state) = self.inner.effects.borrow_mut().get_mut(&id) {
            state.dirty = false;
            state.runs += 1;
        }
        report.effects_run += 1;
    }

    /// 单次 flush 内单节点执行次数上限检查（自激循环兜底）。
    fn check_run_budget(&self, id: NodeId) -> Result<(), ReactiveError> {
        let mut runs = self.inner.node_runs.borrow_mut();
        let counter = runs.entry(id).or_insert(0);
        *counter += 1;
        if *counter > MAX_NODE_RUNS_PER_FLUSH {
            return Err(ReactiveError::FlushCycle { node: id });
        }
        Ok(())
    }
}

/// 批次守卫：drop 时自动退出批次（panic 也不会把运行时卡在批次态）。
struct BatchGuard<'a> {
    inner: &'a RuntimeInner,
}

impl Drop for BatchGuard<'_> {
    fn drop(&mut self) {
        let depth = self.inner.batch_depth.get();
        if depth > 0 {
            self.inner.batch_depth.set(depth - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Rc;

    #[test]
    fn effect_reruns_when_its_dependency_changes() {
        let runtime = Runtime::new();
        let seen = Rc::new(RefCell::new(Vec::<u32>::new()));
        let value = runtime.signal(1u32);

        let s = seen.clone();
        let v = value.clone();
        runtime.effect(move || {
            s.borrow_mut().push(v.get());
        });

        runtime.flush();
        value.set(2);
        runtime.flush();
        value.set(3);
        runtime.flush();

        assert_eq!(&*seen.borrow(), &[1, 2, 3]);
    }

    #[test]
    fn conditional_dependency_is_rebuilt_on_each_run() {
        let runtime = Runtime::new();
        let toggle = runtime.signal(true);
        let left = runtime.signal(1u32);
        let right = runtime.signal(10u32);
        let seen = Rc::new(RefCell::new(Vec::<u32>::new()));

        let s = seen.clone();
        let t = toggle.clone();
        let l = left.clone();
        let r = right.clone();
        runtime.effect(move || {
            let picked = if t.get() { l.get() } else { r.get() };
            s.borrow_mut().push(picked);
        });

        runtime.flush();
        assert_eq!(&*seen.borrow(), &[1]);

        // 分支切换 ⇒ 旧依赖（右值）失效，左值成为新依赖。
        toggle.set(false);
        runtime.flush();
        assert_eq!(&*seen.borrow(), &[1, 10]);

        // 左值再变不再触发（已不是依赖）。
        left.set(99);
        runtime.flush();
        assert_eq!(&*seen.borrow(), &[1, 10]);

        right.set(11);
        runtime.flush();
        assert_eq!(&*seen.borrow(), &[1, 10, 11]);
    }

    #[test]
    fn batch_defers_flush_until_batch_ends() {
        let runtime = Runtime::new();
        let counter = Rc::new(Cell::new(0));
        let value = runtime.signal(0u32);

        let c = counter.clone();
        let v = value.clone();
        runtime.effect(move || {
            c.set(c.get() + 1);
            let _ = v.get();
        });
        runtime.flush();
        assert_eq!(counter.get(), 1);

        runtime.batch(|| {
            value.set(1);
            value.set(2);
            runtime.flush(); // 批次内的 flush 被推迟
            assert_eq!(counter.get(), 1, "批次内不落盘");
        });
        assert_eq!(counter.get(), 2, "批次结束落一次");
    }

    #[test]
    fn self_triggering_effect_hits_flush_cycle_guard() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let v = value.clone();
        runtime.effect(move || {
            let current = v.get();
            v.set(current + 1); // 写自己的依赖 ⇒ 自激
        });

        let err = runtime.try_flush();
        assert!(
            matches!(err, Err(ReactiveError::FlushCycle { .. })),
            "自激循环必须被单 flush 执行上限兜住，实测：{err:?}"
        );
    }

    #[test]
    fn mutation_during_derivation_is_rejected() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let v = value.clone();
        let derived = runtime.derived(move || {
            v.set(1); // 派生求值期间写信号 ⇒ 硬错误
            0u32
        });

        let result = catch_unwind(AssertUnwindSafe(|| derived.get()));
        assert!(result.is_err(), "派生期间的写必须报错");
        assert_eq!(value.get(), 0, "被拒绝的写不得改值");
    }

    #[test]
    fn graph_exposes_both_directions_of_dependency() {
        let runtime = Runtime::new();
        let left = runtime.signal(1u32);
        let right = runtime.signal(2u32);
        let l = left.clone();
        let r = right.clone();
        let handle = runtime.effect(move || {
            let _ = l.get();
            let _ = r.get();
        });
        runtime.flush();

        let graph = runtime.graph();
        let deps = graph.dependencies_of(handle.id());
        assert!(deps.contains(&left.id()), "effect 依赖左信号");
        assert!(deps.contains(&right.id()), "effect 依赖右信号");
        assert!(graph.dependents_of(left.id()).contains(&handle.id()));
    }

    // ── 判别测试（钉行为不钉字面）──────────────────────────────────────────

    /// 宿主调度器：**一次写 ⇒ 一次 flush 请求**；多次写 ⇒ 合并成一次请求。
    #[test]
    fn multiple_writes_coalesce_into_one_host_flush_request() {
        let runtime = Runtime::new();
        let host = Rc::new(crate::QueuedScheduler::new());
        runtime.set_host_scheduler(host.clone());

        let value = runtime.signal(0u32);
        value.set(1);
        value.set(2);
        value.set(3);

        assert_eq!(host.queued(), 1, "同一轮的多次写必须合并成一次 flush 请求");
        assert_eq!(host.drain(), 1);
        assert!(host.is_empty());
    }

    /// 宿主未推进前，**不得**发生任何重算（flush 时机完全由宿主决定）。
    #[test]
    fn effects_wait_until_the_host_drains_the_queue() {
        let runtime = Runtime::new();
        let host = Rc::new(crate::QueuedScheduler::new());
        runtime.set_host_scheduler(host.clone());

        let value = runtime.signal(0u32);
        let runs = Rc::new(Cell::new(0));
        let v = value.clone();
        let r = runs.clone();
        runtime.effect(move || {
            r.set(r.get() + 1);
            let _ = v.get();
        });

        value.set(1);
        assert_eq!(runs.get(), 0, "宿主未推进 ⇒ 尚未重算");
        assert_eq!(host.queued(), 1);

        host.drain();
        assert_eq!(runs.get(), 1, "宿主推进 ⇒ 重算一次");
    }

    /// 同步默认实现下，写入**当场**触发一次 flush（native 侧默认口径）。
    #[test]
    fn default_scheduler_flushes_synchronously_on_write() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let runs = Rc::new(Cell::new(0));
        let v = value.clone();
        let r = runs.clone();
        runtime.effect(move || {
            r.set(r.get() + 1);
            let _ = v.get();
        });

        value.set(1);
        assert_eq!(runs.get(), 1, "默认同步调度器 ⇒ 写后当场重算");
    }

    /// 批次内**多次标脏**合并为**一次** effect 执行。
    #[test]
    fn batch_merges_repeated_dirty_marks_into_one_run() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let runs = Rc::new(Cell::new(0));

        let v = value.clone();
        let r = runs.clone();
        runtime.effect(move || {
            r.set(r.get() + 1);
            let _ = v.get();
        });
        runtime.flush();
        assert_eq!(runs.get(), 1);

        runtime.batch(|| {
            value.set(1);
            value.set(2);
            value.set(3);
            assert_eq!(runs.get(), 1, "批次内不落盘");
        });
        assert_eq!(runs.get(), 2, "整批只触发一次重跑");
    }

    /// 批次内多次写：effect 只能看到**最终值**（中间值不得被观测到）。
    #[test]
    fn batch_effect_observes_only_the_final_value() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let seen = Rc::new(RefCell::new(Vec::<u32>::new()));

        let v = value.clone();
        let s = seen.clone();
        runtime.effect(move || {
            s.borrow_mut().push(v.get());
        });
        runtime.flush();

        runtime.batch(|| {
            value.set(1);
            value.set(2);
            value.set(3);
        });
        assert_eq!(&*seen.borrow(), &[0, 3], "只观测到批次的最终值");
    }

    /// 嵌套批次：**最外层**结束才落一次 flush。
    #[test]
    fn nested_batches_flush_once_at_the_outermost_end() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let runs = Rc::new(Cell::new(0));

        let v = value.clone();
        let r = runs.clone();
        runtime.effect(move || {
            r.set(r.get() + 1);
            let _ = v.get();
        });
        runtime.flush();
        assert_eq!(runs.get(), 1);

        runtime.batch(|| {
            value.set(1);
            runtime.batch(|| {
                value.set(2);
                assert_eq!(runs.get(), 1, "内层批次结束不落盘");
            });
            assert_eq!(runs.get(), 1, "外层批次仍在进行 ⇒ 不落盘");
        });
        assert_eq!(runs.get(), 2, "最外层结束才落一次");
    }

    /// 条件分支切换时，依赖边必须**真的**换掉（不是「碰巧没触发」）。
    #[test]
    fn branch_switch_replaces_the_dependency_edges() {
        let runtime = Runtime::new();
        let toggle = runtime.signal(true);
        let left = runtime.signal(1u32);
        let right = runtime.signal(10u32);

        let t = toggle.clone();
        let l = left.clone();
        let r = right.clone();
        let handle = runtime.effect(move || {
            let _ = if t.get() { l.get() } else { r.get() };
        });
        runtime.flush();
        assert!(runtime
            .graph()
            .dependencies_of(handle.id())
            .contains(&left.id()));
        assert!(!runtime
            .graph()
            .dependencies_of(handle.id())
            .contains(&right.id()));

        toggle.set(false);
        runtime.flush();
        assert!(
            !runtime
                .graph()
                .dependencies_of(handle.id())
                .contains(&left.id()),
            "旧分支的边必须被摘掉"
        );
        assert!(runtime
            .graph()
            .dependencies_of(handle.id())
            .contains(&right.id()));
    }

    /// 宿主在**批次中途**推进队列时，flush 不得丢失（批次结束后仍要落盘）。
    #[test]
    fn host_advancing_mid_batch_does_not_lose_the_flush() {
        let runtime = Runtime::new();
        let host = Rc::new(crate::QueuedScheduler::new());
        runtime.set_host_scheduler(host.clone());

        let value = runtime.signal(0u32);
        let runs = Rc::new(Cell::new(0));
        let v = value.clone();
        let r = runs.clone();
        runtime.effect(move || {
            r.set(r.get() + 1);
            let _ = v.get();
        });

        value.set(1);
        assert_eq!(host.queued(), 1);

        runtime.batch(|| {
            // 宿主在批次中途推进队列 ⇒ 本次 flush 因「批次中」而推迟。
            host.drain();
            assert_eq!(runs.get(), 0, "批次内不落盘");
        });

        assert_eq!(host.queued(), 1, "批次结束必须重新向宿主请求一次 flush");
        host.drain();
        assert_eq!(runs.get(), 1, "批次结束后必须落盘（flush 不得丢失）");
    }

    /// 自激循环的失败必须**回传给写方**（不许静默降级）。
    #[test]
    fn flush_cycle_failure_is_reported_to_the_writer() {
        let runtime = Runtime::new();
        let trap = runtime.signal(0u32);
        let t = trap.clone();
        runtime.effect(move || {
            let current = t.get();
            t.set(current + 1); // 写自己的依赖 ⇒ 自激
        });

        // 写入 ⇒ 同步 flush ⇒ 自激被单 flush 上限兜住 ⇒ 错误必须回到写方。
        let err = trap
            .try_set(5)
            .expect_err("自激导致的 flush 失败必须回传给写方");
        assert!(
            matches!(err, ReactiveError::FlushCycle { .. }),
            "实测：{err:?}"
        );
    }

    /// 自激循环的错误必须**点名**出问题的节点（设计文档 §4.4 的硬要求）。
    #[test]
    fn flush_cycle_error_names_the_offending_node() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let v = value.clone();
        let handle = runtime.effect(move || {
            let current = v.get();
            v.set(current + 1);
        });

        let err = runtime.try_flush().expect_err("自激必须报错");
        match err {
            ReactiveError::FlushCycle { node } => {
                assert_eq!(node, handle.id(), "必须点名出问题的 effect 节点");
                let described = runtime.describe_cycle_path(&[node]);
                assert!(!described.is_empty(), "节点必须可渲染：{described}");
            }
            other => panic!("实测错误类型：{other:?}"),
        }
    }

    /// 派生期间的写必须被**拒绝**，且**不得**改变值（硬错误，不静默）。
    #[test]
    fn write_during_derivation_is_refused_and_leaves_the_value_untouched() {
        let runtime = Runtime::new();
        let value = runtime.signal(7u32);
        let v = value.clone();
        let derived = runtime.derived(move || {
            let _ = v.try_set(99);
            0u32
        });

        let _ = derived.get();
        assert_eq!(value.get(), 7, "被拒绝的写不得改值");
    }
}
