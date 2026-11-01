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
//!
//! ## 失败语义（本批重构的硬约束）
//!
//! wasm 侧 **panic = 整页 abort** ⇒ 引擎不得把用户的坏数据升级成页面崩溃：
//!
//! - **写路径永不 panic**：写失败（派生期写入 / 自激环）以 `Err` 返回给 `try_*`
//!   变体；「无条件」变体（[`crate::Signal::set`] 等）把错误**记录到运行期错误槽**
//!   （[`Runtime::take_flush_error`] 可取走）后继续执行。
//! - **错误绑定到「本次写入期间」**：每次 flush 自增一个**代次**（generation），
//!   错误随代次记录；写路径只取**代次更新**的错误 ⇒ 上一次 flush 留下的陈旧错误
//!   不会污染下一次（与该错误毫无关系的）写入。
//! - **自激尽早判定**：同一次 flush 内，某 effect 若经由「自己（或其下游）的写入」
//!   被再次标脏 ⇒ 判定为环 ⇒ 显式 [`ReactiveError::CycleDetected`]（带环路径）。
//!   **不靠跑满次数上限才发现**（[`MAX_NODE_RUNS_PER_FLUSH`] 只作最后一道兜底）。
//! - **失败不中断整轮调度**：一个 effect 出错时，其余 effect 照常跑完；只有
//!   「兜底上限被撞到」这种轮次级失败才会中止本轮，且**未执行的 id 会重新入队**
//!   （并在 [`FlushReport::effects_skipped`] 里报出数量）。

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashSet};
use std::rc::{Rc, Weak};

use crate::derived::{Derived, DerivedOps};
use crate::effect::{EffectBody, EffectHandle, EffectState, Scope};
use crate::error::ReactiveError;
use crate::graph::{DependencyGraph, NodeId};
use crate::host_scheduler::{HostScheduler, SyncScheduler};
use crate::scheduler::{Phase, Scheduler};
use crate::signal::Signal;

/// 单次 flush 内单个节点的最大执行次数（**兜底上限**，不是主要检测手段）。
///
/// 取值 100 是**设计常量**（非实测）：自激循环的**主**检测手段是因果链判定
/// （见 [`RuntimeInner::mark_effect_dirty`]）⇒ 正常情况下永远不会撞到这里。
/// 本上限只用于兜住「因果链判定漏掉的路径」，避免把宿主挂死。
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

/// 一次写入的因果记录：**谁**写的、写了**哪个**节点。
///
/// 为什么要有「因果链」：自激循环的本质是「写入 ⇒ 标脏 ⇒ 重跑 ⇒ 再写入」的
/// **因果回路**。只记「谁写的」不够（外部写入也会标脏），必须记住「这次写入是
/// 被哪条链引发的」⇒ 某个 effect 若出现在自己引发的因果链里 ⇒ 它写回了自己
/// （或自己的下游）⇒ 环。
#[derive(Debug, Clone)]
pub(crate) struct WriteCause {
    /// 写入者（effect 节点）；`None` = 外部写入（用户代码 / 宿主）。
    pub(crate) writer: Option<NodeId>,
    /// 被写入的节点。
    pub(crate) node: NodeId,
}

/// flush 请求的存活票据：宿主若**丢弃**了 flush 闭包，本票据随之失效。
///
/// 为什么需要：运行时用「已向宿主请求过 flush」这个标记把同一轮的多次写合并成
/// 一次请求（设计文档 §4.2 规则 1）。宿主若把那一轮**丢弃**了（例如队列被清空），
/// 标记会永远卡在「已请求」⇒ 之后任何写入都不再请求 flush ⇒ 运行时**永久不再
/// 刷新**。票据让运行时能**自愈**：宿主丢弃 ⇒ 票据失效 ⇒ 下次写入自动重新请求
/// （**不靠宿主自觉**）。
struct FlushTicket {
    alive: Rc<Cell<bool>>,
}

impl Drop for FlushTicket {
    fn drop(&mut self) {
        self.alive.set(false);
    }
}

/// 运行期错误槽里的一条失败记录。
#[derive(Debug, Clone)]
struct FlushFailure {
    /// 失败发生的 flush **代次**（用于把错误绑定到「本次写入期间」）。
    gen: u64,
    /// 失败本身。
    error: ReactiveError,
    /// 是否**已上报**给调用方（已被 `try_set` / `try_flush` 取走）。
    ///
    /// 已上报的错误在下一次**成功**的 flush 时清掉；未上报的（异步宿主还没来取）
    /// 保留 ⇒ 失败不会被静默吞掉。
    surfaced: bool,
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
    /// effect 节点表（销毁即摘除 ⇒ 表不得单调增长）。
    pub(crate) effects: RefCell<BTreeMap<NodeId, EffectState>>,
    /// 派生节点表（**弱引用**登记 ⇒ 派生句柄丢弃后节点可被回收）。
    pub(crate) derived_nodes: RefCell<BTreeMap<NodeId, Weak<dyn DerivedOps>>>,
    /// 自引用弱指针（请求 flush 时需要构造 [`Runtime`]；用弱引用避免引用环）。
    self_ref: RefCell<Option<Weak<RuntimeInner>>>,
    next_id: Cell<usize>,
    batch_depth: Cell<usize>,
    flushing: Cell<bool>,
    flush_pending: Cell<bool>,
    /// 已向宿主请求过 flush、但尚未真正执行（用于把多次写合并成一次 flush）。
    flush_requested: Cell<bool>,
    /// 当前那次 flush 请求的存活票据（宿主丢弃 ⇒ 票据失效 ⇒ 运行时自愈）。
    flush_ticket: RefCell<Option<Weak<Cell<bool>>>>,
    /// flush **代次**：每次 flush 自增（错误按代次绑定 ⇒ 陈旧错误不污染后续写入）。
    pub(crate) flush_gen: Cell<u64>,
    /// 运行期错误槽（最后一次失败；写路径与宿主都从这里取）。
    error_slot: RefCell<Option<FlushFailure>>,
    /// 单次 flush 内各节点的执行次数（兜底上限用）。
    node_runs: RefCell<BTreeMap<NodeId, usize>>,
    /// 当前正在运行的 effect 的**因果链**（自激判定用）。
    current_causes: RefCell<Vec<WriteCause>>,
    /// 待跑节点的因果链（自激判定用；节点被排入时登记）。
    pending_causes: RefCell<BTreeMap<NodeId, Vec<WriteCause>>>,
    /// 本次 flush 内每个 effect 的**自激次数**（写回自己 ⇒ 计数 +1；>1 ⇒ 环）。
    self_triggers: RefCell<BTreeMap<NodeId, usize>>,
    /// 本次 flush 的累计报告（跟随轮的效果也计入其中）。
    report: RefCell<FlushReport>,
    /// 派生登记次数（用于定期剪掉已失效的弱引用残骸）。
    derived_inserts: Cell<usize>,
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
            flush_ticket: RefCell::new(None),
            flush_gen: Cell::new(0),
            error_slot: RefCell::new(None),
            node_runs: RefCell::new(BTreeMap::new()),
            current_causes: RefCell::new(Vec::new()),
            pending_causes: RefCell::new(BTreeMap::new()),
            self_triggers: RefCell::new(BTreeMap::new()),
            report: RefCell::new(FlushReport::default()),
            derived_inserts: Cell::new(0),
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

    /// 当前**最近**的 effect 追踪帧（写入的「作者」；无 ⇒ 外部写入）。
    pub(crate) fn current_effect(&self) -> Option<NodeId> {
        self.tracking
            .borrow()
            .iter()
            .rev()
            .find_map(|frame| match frame {
                TrackingFrame::Effect(id) => Some(*id),
                TrackingFrame::Derived(_) => None,
            })
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

    /// 登记派生节点（**弱引用** ⇒ 派生句柄丢弃后节点与求值闭包可被回收）。
    pub(crate) fn register_derived(&self, id: NodeId, node: Rc<dyn DerivedOps>) {
        let mut nodes = self.derived_nodes.borrow_mut();
        nodes.insert(id, Rc::downgrade(&node));
        // 定期剪掉已失效的弱引用残骸（否则表项单调增长）。
        let inserts = self.derived_inserts.get() + 1;
        self.derived_inserts.set(inserts);
        if inserts.is_multiple_of(64) {
            nodes.retain(|_, weak| weak.strong_count() > 0);
        }
    }

    /// 该节点是否是派生节点（弱引用已失效 ⇒ 不再是）。
    pub(crate) fn is_derived(&self, node: NodeId) -> bool {
        self.derived_nodes
            .borrow()
            .get(&node)
            .map(|weak| weak.strong_count() > 0)
            .unwrap_or(false)
    }

    /// 该节点是否是 effect 节点。
    pub(crate) fn is_effect(&self, node: NodeId) -> bool {
        self.effects.borrow().contains_key(&node)
    }

    /// 把派生节点标脏（缓存失效）。
    pub(crate) fn invalidate_derived(&self, node: NodeId) {
        let handle = self
            .derived_nodes
            .borrow()
            .get(&node)
            .and_then(|w| w.upgrade());
        if let Some(handle) = handle {
            handle.invalidate();
        }
    }

    /// 把 effect 节点标脏并排入**它自己的阶段**对应的队列。
    ///
    /// 阶段必须跟着任务走：`effect_pre` 的重跑若被排进 post 队列 ⇒ 「渲染前先于
    /// 渲染后」这条硬保证（ADR-0015）就破了。
    ///
    /// ## 自激判定（早于次数上限）
    ///
    /// 若本次写入的**因果链**里已经出现过该 effect ⇒ 它是被自己（或自己的下游）
    /// 的写入标脏的 ⇒ **自激**。
    ///
    /// 为什么允许**第一次**自激重跑：「读了自己的依赖、但写得有条件」是合法写法
    /// （例如 `if v.get() < 10 { v.set(11) }`）⇒ 它**第二次**跑就不会再写 ⇒ 能
    /// 收敛 ⇒ 不该被当成环（否则合法代码被误伤，且那次重跑被吞掉 ⇒ 状态陈旧）。
    ///
    /// **第二次**自激 ⇒ 铁定是环（写多少遍都停不下来）⇒ 显式
    /// [`ReactiveError::CycleDetected`]（带环路径）且**不再排入**（否则无限重跑）。
    /// ⇒ 判定发生在**第 2 遍**而不是「跑满次数上限」（兜底上限只剩最后一道防线）。
    fn mark_effect_dirty(&self, node: NodeId, cause: &[WriteCause]) -> Result<(), ReactiveError> {
        let is_self_trigger = cause.iter().any(|item| item.writer == Some(node));
        let pre = {
            let mut effects = self.effects.borrow_mut();
            match effects.get_mut(&node) {
                Some(state) if !state.disposed => {
                    state.dirty = true;
                    Some(state.pre)
                }
                _ => None,
            }
        };
        // 已销毁 ⇒ 不排入（也不算环）。
        let Some(pre) = pre else {
            return Ok(());
        };
        if is_self_trigger {
            let mut counts = self.self_triggers.borrow_mut();
            let counter = counts.entry(node).or_insert(0);
            *counter += 1;
            if *counter > 1 {
                // 第二次自激 ⇒ 环 ⇒ 显式报错（脏标记保留 ⇒ 状态可见）。
                let path = cycle_path(cause, node);
                return Err(ReactiveError::CycleDetected { path });
            }
            // 第一次自激 ⇒ 允许重跑一次（见上面的说明）。
        }
        // 排入对应队列（阶段随任务走 ⇒ 顺序保证不破）。
        if pre {
            self.scheduler.borrow_mut().schedule_pre(node);
        } else {
            self.scheduler.borrow_mut().schedule_post(node);
        }
        // 登记因果链（节点运行时作为「当前链」使用）；已有的（首次排入）保留。
        self.pending_causes
            .borrow_mut()
            .entry(node)
            .or_insert_with(|| cause.to_vec());
        Ok(())
    }

    /// 本次写入的因果链 = 当前链 ++ 本次写入。
    fn begin_write(&self, source: NodeId) -> Vec<WriteCause> {
        let mut chain = self.current_causes.borrow().clone();
        chain.push(WriteCause {
            writer: self.current_effect(),
            node: source,
        });
        chain
    }

    /// 沿依赖图主动标脏（推）：派生 ⇒ 标脏缓存并继续向下传播；effect ⇒ 排队。
    ///
    /// 返回值：自激环 ⇒ [`ReactiveError::CycleDetected`]（**其余节点照常标脏**，
    /// 失败不中断整轮调度）。
    pub(crate) fn propagate_dirty(&self, source: NodeId) -> Result<(), ReactiveError> {
        let cause = self.begin_write(source);
        let mut first_error: Option<ReactiveError> = None;
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
                if let Err(err) = self.mark_effect_dirty(node, &cause) {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                }
            }
        }
        if let Some(err) = &first_error {
            // 失败必须能被回传（不许静默降级）：记入运行期错误槽。
            self.record_error(err.clone());
        }
        match first_error {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    /// 向宿主请求一次 flush（写操作的触发点，设计文档 §4.2 规则 1）。
    ///
    /// 三条短路：
    /// 1. **flush 进行中** ⇒ 由本次 flush 的循环接管（防重入，不重复请求）；
    /// 2. **批次中** ⇒ 推迟到批次结束（设计文档 §4.2 规则 2）；
    /// 3. **已请求过、宿主仍持有** ⇒ 合并（同一轮的多次写只触发一次 flush）。
    ///
    /// 其余情况 ⇒ 交给宿主调度器决定何时真正执行。
    pub(crate) fn request_flush(&self) {
        self.request_flush_inner(false);
    }

    /// 请求一次 flush（内部用；`is_followup` 标记「这是跟随轮」）。
    ///
    /// 跟随轮 = flush 期间新建的 effect 被提升后补的那一次请求 ⇒ 跟随轮**不再**
    /// 发起跟随（否则同步宿主下会无限自建自跑 ⇒ 栈溢出）。
    pub(crate) fn request_flush_inner(&self, is_followup: bool) {
        if self.flushing.get() {
            // 本次 flush 的循环会把新排入的待办一并跑掉 ⇒ 不必再请求。
            return;
        }
        if self.batch_depth.get() > 0 {
            self.flush_pending.set(true);
            return;
        }
        if self.flush_requested.get() {
            if self.flush_ticket_alive() {
                // 宿主仍持有那一轮 ⇒ 合并。
                return;
            }
            // 宿主把那一轮**丢弃**了 ⇒ 复位并重新请求（自愈：不靠宿主自觉）。
            self.flush_requested.set(false);
        }
        self.flush_requested.set(true);

        let Some(inner) = self
            .self_ref
            .borrow()
            .as_ref()
            .and_then(|weak| weak.upgrade())
        else {
            // 运行时已被释放 ⇒ 没有可 flush 的对象（对象已不存在，不是静默降级）。
            self.flush_requested.set(false);
            return;
        };
        let host = self.host_scheduler.borrow().clone();
        let callback_weak = Rc::downgrade(&inner);
        // 存活票据：宿主若丢弃了这个闭包，票据随之失效 ⇒ 运行时能自愈。
        let alive = Rc::new(Cell::new(true));
        *self.flush_ticket.borrow_mut() = Some(Rc::downgrade(&alive));
        let ticket = FlushTicket { alive };
        host.schedule_flush(Box::new(move || {
            // 闭包持有票据 ⇒ 闭包被丢弃（宿主放弃这一轮）时票据失效。
            let _ticket = ticket;
            let Some(callback_inner) = callback_weak.upgrade() else {
                return;
            };
            let runtime = Runtime::from_inner(callback_inner);
            // 失败已在 `flush_episode` 内记入运行期错误槽（不许静默降级）。
            let _ = runtime.flush_episode(is_followup);
        }));
    }

    /// 宿主是否**仍持有**当前那次 flush 请求（票据是否还活着）。
    fn flush_ticket_alive(&self) -> bool {
        let ticket = self.flush_ticket.borrow();
        match ticket.as_ref().and_then(|weak| weak.upgrade()) {
            Some(alive) => alive.get(),
            None => false,
        }
    }

    /// 复位「已向宿主请求过 flush」的标记（宿主放弃那一轮时用）。
    pub(crate) fn reset_flush_request(&self) {
        self.flush_requested.set(false);
        *self.flush_ticket.borrow_mut() = None;
    }

    /// 记入一条运行期失败（**不许静默降级**：错误必须能被回传）。
    pub(crate) fn record_error(&self, error: ReactiveError) {
        *self.error_slot.borrow_mut() = Some(FlushFailure {
            gen: self.flush_gen.get(),
            error,
            surfaced: false,
        });
    }

    /// 取走**代次新于** `gen_before` 的失败（写路径专用 ⇒ 陈旧错误不污染本次写）。
    pub(crate) fn error_after(&self, gen_before: u64) -> Option<ReactiveError> {
        let mut slot = self.error_slot.borrow_mut();
        let fresh = slot.as_ref().map(|f| f.gen > gen_before).unwrap_or(false);
        if !fresh {
            return None;
        }
        let failure = slot.as_mut()?;
        failure.surfaced = true;
        Some(failure.error.clone())
    }

    /// 取走运行期错误槽里的失败（宿主 / 调用方取走 ⇒ 清空）。
    pub(crate) fn take_error(&self) -> Option<ReactiveError> {
        self.error_slot
            .borrow_mut()
            .take()
            .map(|failure| failure.error)
    }

    /// 取走**本次 flush（该代次）**记录的失败。
    pub(crate) fn error_of_gen(&self, gen: u64) -> Option<ReactiveError> {
        self.error_slot
            .borrow()
            .as_ref()
            .filter(|failure| failure.gen == gen)
            .map(|failure| failure.error.clone())
    }

    /// 清掉**已上报**的失败（成功 flush 收尾时调用）。
    ///
    /// 未上报的失败**保留**（异步宿主还没来取 ⇒ 失败不得被静默吞掉）。
    fn clear_surfaced_error(&self) {
        let mut slot = self.error_slot.borrow_mut();
        if slot.as_ref().map(|f| f.surfaced).unwrap_or(false) {
            *slot = None;
        }
    }

    /// 把错误槽里的失败标记为「已上报」（交付给调用方之后）。
    pub(crate) fn mark_surfaced(&self) {
        if let Some(failure) = self.error_slot.borrow_mut().as_mut() {
            failure.surfaced = true;
        }
    }
}

/// 由因果链构造环路径（首尾为同一节点 ⇒ 可直接渲染成「a -> b -> a」）。
fn cycle_path(cause: &[WriteCause], node: NodeId) -> Vec<NodeId> {
    // 环的起点 = 因果链里**第一次**出现该节点的位置（它写回自己/自己下游的那一步）。
    let start = cause
        .iter()
        .position(|item| item.writer == Some(node))
        .unwrap_or(0);
    let mut path: Vec<NodeId> = Vec::new();
    for item in &cause[start..] {
        if let Some(writer) = item.writer {
            path.push(writer);
        }
        path.push(item.node);
    }
    path.push(node);
    path
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

/// flush 守卫：drop 时退出 flush 态（panic 也不会把运行时卡在 flush 态）。
struct FlushGuard<'a> {
    inner: &'a RuntimeInner,
}

impl Drop for FlushGuard<'_> {
    fn drop(&mut self) {
        self.inner.flushing.set(false);
        self.inner.current_causes.borrow_mut().clear();
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
    /// 本次 flush 中**未执行**（因轮次级失败被跳过）的 effect 数。
    ///
    /// 这些 id 已**重新入队** ⇒ 不会丢；但「pending() 归 0 看不出有东西没跑」
    /// 的老问题由这个字段解决（可观测）。
    pub effects_skipped: usize,
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

    /// 交出一个**使用者形态**的作用域（**给状态持有者**）。
    ///
    /// # 为什么宿主要把运行时「降级」交给状态持有者
    ///
    /// 状态持有者**只需要**建信号 / 派生 ⇒ ⇒ 它**不需要**挂 effect / flush
    /// ⇒ ⇒ 把它**降级**成 [`ReactiveScope`] ⇒ ⇒ 它在**类型层面**就拿不到
    /// 「成为渲染链另一端」的能力 ⇒ ⇒ 「幽灵输入」的结构条件被切断
    /// （见 [`ReactiveScope`] 的文档）。
    ///
    /// # 唯一的移交通道
    ///
    /// 本方法是**唯一**能造出 [`ReactiveScope`] 的入口 ⇒ ⇒ 作用域**必然**
    /// 来自某个**已存在的**运行时 ⇒ ⇒ 状态持有者**要么**被授予宿主的作用域，
    /// **要么**自己另建一个运行时（后者由宿主脚本以精确判据守住）。
    pub fn reactive_scope(&self) -> ReactiveScope {
        ReactiveScope::from_inner(self.inner.clone())
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
    /// **不 panic**：失败（自激环 / 兜底上限）以运行期错误槽报出
    /// （[`Runtime::take_flush_error`] 可取走）⇒ wasm 侧 panic = 整页 abort，
    /// 引擎不得把用户的坏数据升级成页面崩溃。
    ///
    /// 需要在调用点直接拿到错误时用 [`Runtime::try_flush`]。
    pub fn flush(&self) -> FlushReport {
        let (report, _error) = self.flush_episode(false);
        // 调用方拿不到错误本体（这里只给报告）⇒ 错误**不**标记为已上报：
        // 它留在运行期错误槽里等宿主 / 调用方取走（不许静默吞掉）。
        report
    }

    /// 同步跑完当前所有待办；自激循环等错误以返回值报出（不 panic）。
    pub fn try_flush(&self) -> Result<FlushReport, ReactiveError> {
        let (report, error) = self.flush_episode(false);
        match error {
            Some(err) => {
                // 错误已交付给调用方 ⇒ 标记为「已上报」（下次成功 flush 可清掉）。
                self.inner.mark_surfaced();
                Err(err)
            }
            None => Ok(report),
        }
    }

    /// 跑一轮 flush（内部用；返回「报告 + 失败」）。
    fn flush_episode(&self, is_followup: bool) -> (FlushReport, Option<ReactiveError>) {
        // 本次调用**消费掉**之前向宿主请求的那次 flush（无论本次是否真的落盘：
        // 若因批次 / 重入而推迟，本次调用不成立 ⇒ 必须允许后续重新请求，
        // 否则「宿主在批次中途推进队列」会让这次 flush 永久丢失）。
        self.inner.flush_requested.set(false);
        if self.inner.flushing.get() {
            // 重入保护：嵌套 flush 直接返回空报告（外层 flush 会把新待办一并跑掉）。
            return (FlushReport::default(), None);
        }
        if self.inner.batch_depth.get() > 0 {
            // 批次内不落盘：推迟到批次结束（设计文档 §4.2 规则 2）。
            self.inner.flush_pending.set(true);
            return (FlushReport::default(), None);
        }

        // 代次自增：本次 flush 的错误按代次记录 ⇒ 陈旧错误不会污染后续写入。
        self.inner.flush_gen.set(self.inner.flush_gen.get() + 1);
        let gen = self.inner.flush_gen.get();
        self.inner.node_runs.borrow_mut().clear();
        self.inner.pending_causes.borrow_mut().clear();
        self.inner.self_triggers.borrow_mut().clear();
        let merged_before = self.inner.scheduler.borrow().merged_total();
        if !is_followup {
            *self.inner.report.borrow_mut() = FlushReport::default();
        }
        self.inner.flushing.set(true);
        let guard = FlushGuard { inner: &self.inner };
        let inner_result = self.flush_inner();
        // 退出 flush 态（守卫负责；此处显式 drop 以便后续逻辑在非 flush 态下跑）。
        drop(guard);
        // flush 期间新建的 effect ⇒ 提升到正式队列（**按阶段回投** ⇒ 顺序保证不破）。
        self.inner.scheduler.borrow_mut().promote_deferred();
        let merged = self.inner.scheduler.borrow().merged_total() - merged_before;
        self.inner.report.borrow_mut().merged_schedules += merged;

        // 本次 flush 的失败：① 轮次级失败（兜底上限）② 本次 flush 期间记录的失败（自激环）。
        let error = match inner_result {
            Err(err) => Some(err),
            Ok(()) => self.inner.error_of_gen(gen),
        };
        if let Some(err) = &error {
            // 失败必须能被回传（不许静默降级）：记入运行期错误槽（带代次）。
            self.inner.record_error(err.clone());
        } else if !is_followup {
            // 成功 ⇒ 清掉**已上报**的旧错误（未上报的保留：异步宿主还没取走）。
            self.inner.clear_surfaced_error();
        }

        // 跟随轮：flush 期间新建的 effect 已被提升 ⇒ 必须**再请求一次 flush**，
        // 否则它们永远不跑（实测：pending()==1 且子 effect 一次都没跑）。
        // 跟随轮**不再**发起跟随（防同步宿主下无限自建自跑 ⇒ 栈溢出）。
        if !is_followup && self.inner.scheduler.borrow().pending() > 0 {
            self.inner.request_flush_inner(true);
        }

        let report = *self.inner.report.borrow();
        (report, error)
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

    /// 取走运行期错误槽里的失败（若有）。
    ///
    /// 宿主调度器是**异步**时（wasm 微任务），flush 的失败发生在写入之后 ⇒
    /// 由宿主通过本方法取走（**不许静默降级**：失败一定会被报告）。
    ///
    /// 错误**按代次**记录 ⇒ 写路径只取「本次写入期间」产生的错误；本方法取走
    /// 的是「最近一次失败」（异步宿主用它把失败带回给上层）。
    pub fn take_flush_error(&self) -> Option<ReactiveError> {
        self.inner.take_error()
    }

    /// 复位「已向宿主请求过 flush」的标记。
    ///
    /// **何时用**：宿主**丢弃**了那一轮 flush（例如队列被清空）⇒ 运行时的请求标记
    /// 会一直卡在「已请求」⇒ 之后任何写入都不再向宿主请求 flush ⇒ 运行时**永远
    /// 不再刷新**。本方法把标记复位 ⇒ 下一次写入会重新请求。
    ///
    /// 注：运行时**自带自愈**（宿主丢弃 ⇒ 请求票据失效 ⇒ 下次写入自动重新请求），
    /// 本方法给需要**立刻**复位（不等下一次写入）的宿主用。
    pub fn reset_flush_request(&self) {
        self.inner.reset_flush_request();
    }

    /// 当前待办数（渲染前 + 渲染后 + 延迟）。
    pub fn pending(&self) -> usize {
        self.inner.scheduler.borrow().pending()
    }

    /// 上一次 flush 的执行报告（**失败时也有**：失败不打断整轮调度 ⇒ 报告不丢）。
    ///
    /// [`Runtime::try_flush`] 在失败时只返回 `Err`（拿不到报告）⇒ 本方法给出
    /// 那次 flush 的报告（含 [`FlushReport::effects_skipped`]：因轮次级失败而
    /// **未执行**、已重新入队的 effect 数）。
    pub fn last_flush_report(&self) -> FlushReport {
        *self.inner.report.borrow()
    }

    /// 当前**存活**的 effect 数（销毁即摘表 ⇒ 该数不得单调增长）。
    pub fn effect_count(&self) -> usize {
        self.inner.effects.borrow().len()
    }

    /// 当前**存活**的派生节点数（派生句柄丢弃后节点即被回收 ⇒ 该数不得单调增长）。
    pub fn derived_count(&self) -> usize {
        self.inner
            .derived_nodes
            .borrow()
            .values()
            .filter(|weak| weak.strong_count() > 0)
            .count()
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
        let runs = Rc::new(Cell::new(0usize));
        self.inner.effects.borrow_mut().insert(
            id,
            EffectState {
                body: Some(body.clone()),
                dirty: true,
                runs: runs.clone(),
                cleanups: Vec::new(),
                disposed: false,
                pre,
            },
        );
        let handle = EffectHandle {
            id,
            inner: self.inner.clone(),
            runs,
        };
        if self.inner.flushing.get() {
            // flush 期间新建 ⇒ 延迟队列（**带阶段** ⇒ 提升时回投对应队列）。
            let phase = if pre { Phase::Pre } else { Phase::Post };
            self.inner
                .scheduler
                .borrow_mut()
                .schedule_deferred(id, phase);
        } else if pre {
            self.inner.scheduler.borrow_mut().schedule_pre(id);
        } else {
            self.inner.scheduler.borrow_mut().schedule_post(id);
        }
        handle
    }

    /// flush 的两阶段主体。
    fn flush_inner(&self) -> Result<(), ReactiveError> {
        // ① 渲染前阶段（DOM 写入批次挂在这里）。
        loop {
            let batch = self.inner.scheduler.borrow_mut().take_pre();
            if batch.is_empty() {
                break;
            }
            for (index, id) in batch.iter().enumerate() {
                if let Err(err) = self.check_run_budget(*id) {
                    // 轮次级失败 ⇒ 本批**未执行**的 id 重新入队（不得静默丢弃）。
                    self.reschedule(&batch[index..], Phase::Pre);
                    self.inner.report.borrow_mut().effects_skipped += batch.len() - index;
                    return Err(err);
                }
                self.run_effect(*id, true);
            }
        }
        // ② 渲染后阶段（effect）。
        loop {
            let batch = self.inner.scheduler.borrow_mut().take_post();
            if batch.is_empty() {
                break;
            }
            for (index, id) in batch.iter().enumerate() {
                if let Err(err) = self.check_run_budget(*id) {
                    self.reschedule(&batch[index..], Phase::Post);
                    self.inner.report.borrow_mut().effects_skipped += batch.len() - index;
                    return Err(err);
                }
                self.run_effect(*id, false);
            }
        }
        Ok(())
    }

    /// 执行单个 effect。
    fn run_effect(&self, id: NodeId, pre: bool) {
        let (body, runs) = {
            let mut effects = self.inner.effects.borrow_mut();
            match effects.get_mut(&id) {
                Some(state) if !state.disposed => {
                    // **跑前**清脏标记 ⇒ 体里若把自己重新标脏（自激），
                    // 跑完后仍是脏的 ⇒ `is_dirty()` 不撒谎。
                    state.dirty = false;
                    match state.body.clone() {
                        Some(body) => (body, state.runs.clone()),
                        None => return,
                    }
                }
                _ => return,
            }
        };
        // 重跑前清空旧依赖 ⇒ 条件依赖正确（设计文档 §4.1）。
        self.inner.graph.borrow_mut().clear_dependencies(id);
        // 本次运行的因果链（自激判定用）。
        let causes = self
            .inner
            .pending_causes
            .borrow_mut()
            .remove(&id)
            .unwrap_or_default();
        let previous = std::mem::replace(&mut *self.inner.current_causes.borrow_mut(), causes);
        {
            let _guard = self.inner.push_frame(TrackingFrame::Effect(id));
            let mut guard = body.borrow_mut();
            guard();
        }
        *self.inner.current_causes.borrow_mut() = previous;
        runs.set(runs.get() + 1);
        let mut report = self.inner.report.borrow_mut();
        report.effects_run += 1;
        if pre {
            report.effects_pre_run += 1;
        }
    }

    /// 单次 flush 内单节点执行次数上限检查（**兜底**，主检测是因果链判定）。
    fn check_run_budget(&self, id: NodeId) -> Result<(), ReactiveError> {
        let mut runs = self.inner.node_runs.borrow_mut();
        let counter = runs.entry(id).or_insert(0);
        *counter += 1;
        if *counter > MAX_NODE_RUNS_PER_FLUSH {
            return Err(ReactiveError::FlushCycle { node: id });
        }
        Ok(())
    }

    /// 把未执行的 id 重新入队（轮次级失败时调用 ⇒ 工作不丢）。
    fn reschedule(&self, ids: &[NodeId], phase: Phase) {
        let mut scheduler = self.inner.scheduler.borrow_mut();
        for id in ids {
            match phase {
                Phase::Pre => scheduler.schedule_pre(*id),
                Phase::Post => scheduler.schedule_post(*id),
            }
        }
    }
}

/// 响应式作用域（**使用者形态**）—— 状态持有者唯一能持有的运行时能力。
///
/// # 为什么要有这个形态（架构不变量：从「脚本近似检查」升级为「类型层面强制」）
///
/// 2026-10-25 实测到一个**架构级缺陷**：`ui-kit` 的 `BudgetState::new()`
/// **自建**了一个 `Runtime` ⇒ ⇒ 它的草稿 / 反馈信号**不属于**宿主挂 effect
/// 的那个 runtime ⇒ ⇒ **effect 根本不依赖这些信号** ⇒ ⇒ 改草稿**不触发重渲染**
/// ⇒ ⇒ 实测表现为「**幽灵输入**」（用户敲进去的值**引擎不知道**）。
///
/// ⇒ ⇒ 缺陷的**结构条件**是：「**状态持有者手里有一个能挂 effect 的运行时**」
/// ⇒ ⇒ 只要状态持有者**拿不到**这种运行时，它就**不可能**成为渲染链的一端
/// ⇒ ⇒ 它的信号**只可能**建在**被授予的**那个 runtime 上。
///
/// ⇒ ⇒ 本类型就是那条**类型层面的刀**：
///
/// | 形态 | 类型 | 能力 | 谁拿 |
/// |---|---|---|---|
/// | **所有者形态** | [`Runtime`] | 建信号 / 派生 / **挂 effect** / flush / 调度器注入 | **应用根**（宿主入口） |
/// | **使用者形态** | [`ReactiveScope`]（本类型） | **只**能建信号 / 派生 | **状态持有者** |
///
/// ⇒ ⇒ 状态持有者**在类型层面**拿不到 `effect` / `flush` ⇒ ⇒ 它**不可能**
/// 自建一个「渲染链的另一端」⇒ ⇒ 「幽灵输入」的结构条件被切断。
///
/// # 能力边界（**故意不给**的东西）
///
/// - **没有** `effect` / `effect_pre` ⇒ 挂不了渲染任务；
/// - **没有** `flush` / `batch` ⇒ 跑不了渲染循环；
/// - **没有** `set_host_scheduler` ⇒ 换不掉宿主的调度器；
/// - **没有** `new` ⇒ 造不出新的运行时（唯一的来源是 [`Runtime::reactive_scope`]）。
///
/// ⇒ ⇒ 这四条缺失**就是**类型层面的强制（`compile_fail` doc-test 钉住它们）。
///
/// # 类型层面的强制（`compile_fail` doc-test 钉住）
///
/// 下面三段**都编译不过** ⇒ ⇒ 这三条能力**确实**不在使用者形态上：
///
/// 1. **挂不了 effect** ⇒ ⇒ 状态持有者**不可能**成为渲染链的另一端：
///
/// ```compile_fail
/// use apeireth_ui_core::{ReactiveScope, Runtime};
///
/// let runtime = Runtime::new();
/// let scope: ReactiveScope = runtime.reactive_scope();
/// // ← 编译不过：`ReactiveScope` 上没有 `effect` 方法
/// scope.effect(|| {});
/// ```
///
/// 2. **跑不了 flush** ⇒ ⇒ 状态持有者**不可能**自己驱动渲染：
///
/// ```compile_fail
/// use apeireth_ui_core::Runtime;
///
/// let runtime = Runtime::new();
/// let scope = runtime.reactive_scope();
/// // ← 编译不过：`ReactiveScope` 上没有 `flush` 方法
/// scope.flush();
/// ```
///
/// 3. **造不出作用域** ⇒ ⇒ 作用域**必然**来自某个已存在的运行时：
///
/// ```compile_fail
/// use apeireth_ui_core::ReactiveScope;
///
/// // ← 编译不过：没有 `new` 这个关联函数（字段私有）
/// let scope = ReactiveScope::new();
/// ```
///
/// # 仍然保留的入口（诚实登记）
///
/// [`Runtime::new`] 仍是 `pub`（native 测试 / 独立运行时 / 既有 crate 的调用点
/// 不可改）⇒ ⇒ 「状态持有者**显式**自建运行时」这条路径在**编译期**仍可写出来
/// ⇒ ⇒ 它由 `crates-ui/apeireth-ui-demo/harness/scripts/check_runtime_ownership.py`
/// 以**精确判据**守住（不再是近似判据）⇒ ⇒ 且即便写出来，其信号也**无处挂
/// effect**（本类型不给这个能力）⇒ ⇒ 症状（幽灵输入）**不成立**。
#[derive(Clone)]
pub struct ReactiveScope {
    inner: Rc<RuntimeInner>,
}

impl std::fmt::Debug for ReactiveScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReactiveScope")
            .field("nodes", &self.inner.next_id.get())
            .finish()
    }
}

impl ReactiveScope {
    /// 建一个信号（可写状态）。
    ///
    /// ⭐ **信号只能建在被授予的运行时上** ⇒ ⇒ 「状态与渲染同属一个 runtime」
    /// 是**类型层面**的事实，不是约定。
    pub fn signal<T: Clone>(&self, value: T) -> Signal<T> {
        Signal::new(self.inner.clone(), value)
    }

    /// 建一个派生值（惰性求值 + 主动失效）。
    pub fn derived<T: Clone + 'static>(&self, compute: impl Fn() -> T + 'static) -> Derived<T> {
        Derived::new(self.inner.clone(), compute)
    }

    /// 两个作用域是否属于**同一个**运行时（**指针身份**判定）。
    ///
    /// ## 用途
    ///
    /// 宿主侧可据此**断言**「我交给状态持有者的作用域与我自己的运行时是同一个」
    /// ⇒ ⇒ 这是类型层保证之外的**运行期复核**（双保险的第二层）。
    pub fn same_runtime(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    /// 内部：从共享的内部状态构造作用域（仅供 `ui-core` 内部使用）。
    pub(crate) fn from_inner(inner: Rc<RuntimeInner>) -> Self {
        Self { inner }
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

    /// 一次只跑一个 flush 的宿主（便于断言「本轮没跑 / 下一轮才跑」这类时序）。
    struct OneShotHost {
        queue: RefCell<Vec<Box<dyn FnOnce()>>>,
    }

    impl OneShotHost {
        fn new() -> Self {
            Self {
                queue: RefCell::new(Vec::new()),
            }
        }

        fn queued(&self) -> usize {
            self.queue.borrow().len()
        }

        /// 跑一个排队的 flush（没有就返回 `false`）。
        fn run_one(&self) -> bool {
            let task = self.queue.borrow_mut().pop();
            match task {
                Some(task) => {
                    task();
                    true
                }
                None => false,
            }
        }
    }

    impl HostScheduler for OneShotHost {
        fn schedule_flush(&self, flush: Box<dyn FnOnce()>) {
            self.queue.borrow_mut().push(flush);
        }
    }

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

    /// ⭐⭐ **effect 的执行时机**：依赖变更 ⇒ 是**立即重跑**还是**等 flush**？
    ///
    /// ## 为什么钉这条
    ///
    /// 「信号变 ⇒ 界面自动更新」这条链的**时机**决定宿主该怎么调度：
    /// - 若**立即**重跑 ⇒ 宿主不需要 flush（但一帧内多次变更 ⇒ 多次重渲染 ⇒ 会抖）；
    /// - 若**等 flush** ⇒ 宿主在帧边界 flush 一次 ⇒ 多次变更 ⇒ 一次重渲染（不抖）。
    ///
    /// ⇒ 这条测试把**真实语义**钉死 ⇒ ⇒ 宿主与文档都按它写（不许各写各的）。
    #[test]
    fn effect_rerun_timing_is_pinned() {
        let runtime = Runtime::new();
        let value = runtime.signal(1u32);
        let ran = Rc::new(Cell::new(0usize));

        let counter = ran.clone();
        let signal = value.clone();
        runtime.effect(move || {
            let _ = signal.get(); // 依赖收集点
            counter.set(counter.get() + 1);
        });

        // ① 创建后 flush ⇒ 首次执行（core 语义：创建时**不**同步执行）
        assert_eq!(ran.get(), 0, "创建时不应同步执行");
        runtime.flush();
        assert_eq!(ran.get(), 1, "首次执行发生在 flush");

        // ② 依赖变更 ⇒ **立即重跑**（⭐ 本仓库的**真实语义**，此前无测试钉住）
        //   ⇒ ⇒ 「一帧内多次变更 ⇒ 多次重渲染」是**事实**（会抖）
        //   ⇒ ⇒ 想「帧边界只渲染一次」必须由**上层显式批处理**（不是 core 免费给的）。
        value.set(2);
        assert_eq!(
            ran.get(),
            2,
            "⭐ 依赖变更**立即**重跑 effect（core 的真实语义；flush 不是它的前提）"
        );
        runtime.flush();
        assert_eq!(ran.get(), 2, "已同步重跑 ⇒ 随后的 flush 不应重复执行");
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
    fn mutation_during_derivation_is_rejected() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let v = value.clone();
        let derived = runtime.derived(move || {
            v.set(1); // 派生求值期间写信号 ⇒ 硬错误
            0u32
        });

        // 写路径不得 panic（wasm 侧 panic = 整页 abort）⇒ 失败改由错误槽报出。
        let result = catch_unwind(AssertUnwindSafe(|| derived.get()));
        assert!(result.is_ok(), "读路径不得 panic");
        assert_eq!(value.get(), 0, "被拒绝的写不得改值");
        assert!(
            runtime.take_flush_error().is_some(),
            "派生期间的写必须被记录（不许静默降级）"
        );
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

    // ── 本批重构新增的判别测试 ─────────────────────────────────────────────

    /// 自激 effect：写路径与 flush **都不 panic**；失败以 `CycleDetected` 报出，
    /// 且**不靠跑满次数上限**才发现（早判 ⇒ effect 只跑一次）。
    #[test]
    fn self_triggering_effect_is_reported_early_without_panicking() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let runs = Rc::new(Cell::new(0));
        let v = value.clone();
        let r = runs.clone();
        runtime.effect(move || {
            r.set(r.get() + 1);
            let current = v.get();
            v.set(current + 1); // 写自己的依赖 ⇒ 自激
        });

        let result = catch_unwind(AssertUnwindSafe(|| runtime.flush()));
        assert!(
            result.is_ok(),
            "flush 不得 panic（wasm 侧 panic = 整页 abort）"
        );

        let err = runtime
            .take_flush_error()
            .expect("自激必须被报出（不许静默降级）");
        match err {
            ReactiveError::CycleDetected { path } => {
                assert!(path.len() >= 2, "环路径必须可读：{path:?}");
                assert_eq!(path[0], path[path.len() - 1], "首尾必须是同一节点");
            }
            other => panic!("实测错误类型：{other:?}"),
        }
        assert_eq!(
            runs.get(),
            2,
            "自激必须**早判**（第二次自激即报错，不得跑满次数上限）"
        );
    }

    /// 自激 effect 里的写**不得 panic**（写路径永不 abort 页面）。
    #[test]
    fn write_inside_a_self_triggering_effect_does_not_panic() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let v = value.clone();
        runtime.effect(move || {
            let current = v.get();
            v.set(current + 1);
        });

        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = runtime.try_flush();
        }));
        assert!(result.is_ok(), "写路径不得 panic");
    }

    /// 自激失败**不得污染**下一次（无关的）写入。
    ///
    /// 构造用**异步宿主**：flush 的失败发生在写入之后、且**没人取走**那条错误
    /// ⇒ 错误滞留在运行期错误槽里 ⇒ 下一次（无关的）写入若按「槽里有没有错误」
    /// 来判定，就会被陈旧错误误伤。
    #[test]
    fn stale_flush_error_does_not_poison_the_next_write() {
        let runtime = Runtime::new();
        let host = Rc::new(crate::QueuedScheduler::new());
        runtime.set_host_scheduler(host.clone());

        let trap = runtime.signal(0u32);
        let t = trap.clone();
        runtime.effect(move || {
            let current = t.get();
            t.set(current + 1);
        });

        // 写入 ⇒ 宿主排队一次 flush；宿主推进 ⇒ flush 失败（自激）⇒ 错误留在槽里。
        trap.set(1);
        assert_eq!(host.queued(), 1);
        host.drain();

        // 无关的下一次写入：不得拿到**上一次** flush 的陈旧错误。
        let other = runtime.signal(0u32);
        let result = catch_unwind(AssertUnwindSafe(|| other.set(42)));
        assert!(result.is_ok(), "陈旧的 flush 错误不得让无关写入 panic");
        assert!(
            other.try_set(43).is_ok(),
            "陈旧错误不得让无关写入返回 Err（错误必须绑定到本次写入期间）"
        );
        assert_eq!(other.get(), 43, "写入必须真的生效");
    }

    /// 「读了自己的依赖、但写得有条件」是**合法**写法 ⇒ 必须能收敛，且**不得**
    /// 被误判成环（误伤 ⇒ 合法代码被报错 + 那次重跑被吞掉 ⇒ 状态陈旧）。
    #[test]
    fn conditional_self_write_converges_without_being_reported_as_a_cycle() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let runs = Rc::new(Cell::new(0));
        let v = value.clone();
        let r = runs.clone();
        runtime.effect(move || {
            r.set(r.get() + 1);
            let current = v.get();
            if current == 0 {
                v.set(1); // 有条件地写回自己 ⇒ 第二次跑就不再写
            }
        });

        let report = runtime.flush();
        assert!(
            report.effects_run >= 2,
            "必须重跑到收敛（实测：{report:?}）"
        );
        assert_eq!(value.get(), 1, "写回必须真的生效");
        assert!(
            runtime.take_flush_error().is_none(),
            "合法写法不得被误判成环（不许误伤）"
        );
    }

    /// 宿主**丢弃**那一轮 flush 后，运行时必须能恢复（自愈，不靠宿主自觉）。
    #[test]
    fn runtime_recovers_after_the_host_drops_the_queued_flush() {
        let runtime = Runtime::new();
        let host = Rc::new(crate::QueuedScheduler::new());
        runtime.set_host_scheduler(host.clone());

        let value = runtime.signal(0u32);
        value.set(1);
        assert_eq!(host.queued(), 1);

        assert_eq!(host.clear(), 1, "宿主丢弃那一轮");
        value.set(2);
        assert_eq!(
            host.queued(),
            1,
            "宿主丢弃后运行时必须能重新请求 flush（否则永远不再刷新）"
        );
    }

    /// 宿主丢弃那一轮后，**显式**复位也能恢复（契约面）。
    #[test]
    fn reset_flush_request_lets_the_next_write_request_again() {
        let runtime = Runtime::new();
        let host = Rc::new(crate::QueuedScheduler::new());
        runtime.set_host_scheduler(host.clone());

        let value = runtime.signal(0u32);
        value.set(1);
        assert_eq!(host.queued(), 1);
        host.clear();

        runtime.reset_flush_request();
        value.set(2);
        assert_eq!(host.queued(), 1, "显式复位后必须能重新请求");
    }

    /// flush 期间新建的 effect ⇒ 提升后必须**再请求一次 flush**（否则永远不跑）。
    #[test]
    fn effect_created_during_flush_is_scheduled_by_a_followup_flush() {
        let runtime = Runtime::new();
        let host = Rc::new(OneShotHost::new());
        runtime.set_host_scheduler(host.clone());
        let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let trigger = runtime.signal(0u32);

        let o1 = order.clone();
        runtime.effect(move || o1.borrow_mut().push("a"));

        let rt = runtime.clone();
        let o2 = order.clone();
        runtime.effect(move || {
            o2.borrow_mut().push("b");
            let o3 = o2.clone();
            rt.effect(move || o3.borrow_mut().push("c"));
        });

        // 写一次 ⇒ 触发一次 flush。
        trigger.set(1);
        assert_eq!(host.queued(), 1);
        assert!(host.run_one());
        assert_eq!(
            &*order.borrow(),
            &["a", "b"],
            "flush 内新建的 effect 不在本轮跑"
        );
        assert_eq!(host.queued(), 1, "提升后必须再请求一次 flush");

        assert!(host.run_one(), "跟随轮必须被排上");
        assert_eq!(&*order.borrow(), &["a", "b", "c"], "子 effect 必须真的跑");
    }

    /// `effect_pre` 的**重跑**仍必须排在渲染后任务之前（顺序保证不破）。
    ///
    /// 关键构造：两个任务**分别**依赖两个信号，且**先写渲染后那个** ⇒ 旧实现
    /// （重排一律排进 post 队列）会把 pre 排到 post 之后 ⇒ 顺序反了。
    #[test]
    fn effect_pre_stays_before_post_effects_on_rerun() {
        let runtime = Runtime::new();
        let order = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let pre_trigger = runtime.signal(0u32);
        let post_trigger = runtime.signal(0u32);

        let o = order.clone();
        let t = pre_trigger.clone();
        runtime.effect_pre(move || {
            let _ = t.get();
            o.borrow_mut().push("pre");
        });
        let o = order.clone();
        let t = post_trigger.clone();
        runtime.effect(move || {
            let _ = t.get();
            o.borrow_mut().push("post");
        });

        runtime.flush();
        assert_eq!(&*order.borrow(), &["pre", "post"], "首轮顺序");

        // 先写渲染后任务的信号，再写渲染前任务的信号（同一批次 ⇒ 同一轮 flush）。
        runtime.batch(|| {
            post_trigger.set(1);
            pre_trigger.set(1);
        });
        runtime.flush();
        assert_eq!(
            &*order.borrow(),
            &["pre", "post", "pre", "post"],
            "重跑后 pre 仍必须先于 post"
        );
    }

    /// 自激 effect 跑完后**仍是脏的**（`is_dirty()` 不撒谎）。
    #[test]
    fn is_dirty_reflects_the_real_state_after_a_self_triggering_run() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);
        let v = value.clone();
        let handle = runtime.effect(move || {
            let current = v.get();
            v.set(current + 1);
        });

        let _ = runtime.try_flush();
        assert!(handle.is_dirty(), "体里把自己重新标脏 ⇒ 跑完后必须仍是脏的");
    }

    /// 轮次级失败（兜底上限）时，同批**未执行**的 effect 不得被静默丢弃。
    ///
    /// 构造：目标 effect 依赖 `value` 但**不写**它（**不是环** ⇒ 不会早判）⇒
    /// 被一串链式写者反复标脏 ⇒ 在同一轮 flush 里被反复排入 ⇒ 只能靠兜底上限
    /// 兜住 ⇒ 本轮中止 ⇒ 未执行的 id 必须**重新入队**且数量可观测。
    #[test]
    fn run_budget_failure_requeues_the_unexecuted_effects() {
        let runtime = Runtime::new();
        let host = Rc::new(OneShotHost::new());
        runtime.set_host_scheduler(host.clone());
        let value = runtime.signal(0u32);

        // 目标 effect：依赖 `value`，不写它。
        let target_runs = Rc::new(Cell::new(0));
        let v = value.clone();
        let tr = target_runs.clone();
        runtime.effect(move || {
            tr.set(tr.get() + 1);
            let _ = v.get();
        });

        // 链式写者：X(i) 写 `value`（标脏目标）并写 X(i+1) 的触发信号
        // ⇒ 每轮 flush 都把目标重新标脏 ⇒ 目标在同一轮 flush 里被反复排入。
        let writers = MAX_NODE_RUNS_PER_FLUSH + 1;
        let mut next_trigger = runtime.signal(0u32);
        let first_trigger = next_trigger.clone();
        for _ in 0..writers {
            let v = value.clone();
            let mine = next_trigger.clone();
            next_trigger = runtime.signal(0u32);
            let following = next_trigger.clone();
            runtime.effect(move || {
                let _ = mine.get(); // 依赖自己的触发信号 ⇒ 上一个写者能标脏它
                v.set(1); // 标脏目标
                following.set(1); // 点亮下一个写者
            });
        }

        // 第一轮：登记依赖边（此刻链还没形成：下游还没读过触发信号）。
        runtime.flush();
        // 第二轮：点燃链条 ⇒ 目标在同一轮 flush 里被反复排入。
        first_trigger.set(1);
        let before = target_runs.get();
        assert!(host.run_one());
        let ran_in_this_flush = target_runs.get() - before;

        let err = runtime
            .take_flush_error()
            .expect("兜底上限必须被报出（不许静默降级）");
        assert!(
            matches!(err, ReactiveError::FlushCycle { .. }),
            "实测：{err:?}"
        );
        assert!(
            ran_in_this_flush <= MAX_NODE_RUNS_PER_FLUSH,
            "兜底上限必须真的兜住（实测这一轮跑了几遍：{ran_in_this_flush}）"
        );
        assert!(
            ran_in_this_flush > 1,
            "目标必须真的被反复排入（实测这一轮跑了几遍：{ran_in_this_flush}）"
        );
        let report = runtime.last_flush_report();
        assert!(
            report.effects_skipped > 0,
            "未执行的 effect 数必须可观测（实测报告：{report:?}）"
        );
        assert!(
            runtime.pending() > 0,
            "未执行的 effect 必须重新入队（不得静默丢弃）"
        );
    }

    /// 单个 effect 失败**不得中断整轮调度**（其余 effect 照常跑完）。
    #[test]
    fn one_failing_effect_does_not_stop_the_whole_round() {
        let runtime = Runtime::new();
        let value = runtime.signal(0u32);

        // 造一个「每次跑都写自己」的 effect（自激 ⇒ 早判 ⇒ 一次就报）。
        let v = value.clone();
        runtime.effect(move || {
            let current = v.get();
            v.set(current + 1);
        });
        // 再造一个正常 effect（必须照常跑完 ⇒ 失败不中断整轮调度）。
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        runtime.effect(move || {
            r.set(r.get() + 1);
        });

        let _ = runtime.try_flush();
        assert_eq!(runs.get(), 1, "失败不得中断整轮调度");
    }

    /// 销毁 effect ⇒ 运行时表**不得**单调增长（长跑 SPA 的内存账）。
    #[test]
    fn disposed_effects_leave_the_runtime_table() {
        let runtime = Runtime::new();
        let mut handles = Vec::new();
        for _ in 0..4 {
            handles.push(runtime.effect(|| {}));
        }
        runtime.flush();
        assert_eq!(runtime.effect_count(), 4);

        for handle in handles {
            handle.dispose();
        }
        assert_eq!(
            runtime.effect_count(),
            0,
            "销毁后 effect 必须从运行时表里摘掉"
        );
    }

    /// 派生句柄丢弃 ⇒ 运行时**弱引用**登记 ⇒ 派生节点表不得单调增长。
    #[test]
    fn dropped_derived_handles_leave_the_runtime_table() {
        let runtime = Runtime::new();
        {
            let _derived = runtime.derived(|| 1u32);
            assert_eq!(runtime.derived_count(), 1);
        }
        assert_eq!(
            runtime.derived_count(),
            0,
            "派生句柄丢弃后节点必须被回收（弱引用登记）"
        );
    }

    /// 节点销毁 ⇒ 依赖图里的**标签**也不得残留（标签表不得单调增长）。
    #[test]
    fn disposing_a_node_removes_its_label() {
        let runtime = Runtime::new();
        let handle = runtime.effect(|| {});
        runtime.set_label(handle.id(), "视图::待销毁");
        assert_eq!(runtime.graph().label_count(), 1);

        handle.dispose();
        assert_eq!(
            runtime.graph().label_count(),
            0,
            "销毁后标签必须被摘掉（否则标签表只增不减）"
        );
    }

    // ── 使用者形态（[`ReactiveScope`]）的行为断言 ──────────────────────────

    /// **作用域里的信号能被宿主的 effect 驱动**（⇒ 不出现「幽灵输入」）。
    ///
    /// 这就是 2026-10-25 那个缺陷的**行为级反证**：状态持有者拿到的作用域
    /// 建出来的信号，**宿主挂在同一个运行时上的 effect 必须能观察到它**。
    #[test]
    fn scope_signals_are_driven_by_the_host_effect() {
        let runtime = Runtime::new();
        // 宿主把运行时「降级」成使用者形态交给状态持有者。
        let scope = runtime.reactive_scope();
        // 状态持有者在作用域上建草稿信号。
        let draft = scope.signal(String::from(""));
        // 宿主在**同一个运行时**上挂渲染 effect（读草稿 ⇒ 建立依赖边）。
        let observed = Rc::new(RefCell::new(Vec::<String>::new()));
        let sink = observed.clone();
        let watched = draft.clone();
        runtime.effect(move || {
            sink.borrow_mut().push(watched.get());
        });

        runtime.flush();
        assert_eq!(
            observed.borrow().as_slice(),
            &[String::from("")],
            "首次 flush 必须看到初值"
        );

        // 状态持有者改草稿 ⇒ ⇒ 宿主的 effect 必须被驱动（**不是幽灵输入**）。
        draft.set(String::from("87"));
        runtime.flush();
        assert_eq!(
            observed.borrow().as_slice(),
            &[String::from(""), String::from("87")],
            "改草稿必须驱动宿主 effect（否则就是「幽灵输入」）"
        );
    }

    /// 作用域**必然**来自某个已存在的运行时（⇒ 身份可复核）。
    #[test]
    fn a_scope_belongs_to_exactly_one_runtime() {
        let runtime = Runtime::new();
        let scope = runtime.reactive_scope();
        let other = Runtime::new().reactive_scope();

        assert!(
            scope.same_runtime(&runtime.reactive_scope()),
            "同一个运行时的两个作用域必须是同一个运行时"
        );
        assert!(
            !scope.same_runtime(&other),
            "不同运行时的作用域不得被误判成同一个"
        );
    }

    /// **另一个运行时**上的信号**驱动不了**本运行时的 effect（⇒ 结构性隔离）。
    ///
    /// ⇒ ⇒ 这条断言说明：状态持有者若**自建**运行时，它的信号对宿主是**死的**
    /// ⇒ ⇒ 「幽灵输入」的结构条件（信号与 effect 分属两个运行时）在**行为上**
    /// 也可观测 ⇒ ⇒ 宿主脚本必须守住「状态持有者不自建运行时」。
    #[test]
    fn a_signal_from_another_runtime_cannot_drive_this_runtime() {
        let runtime = Runtime::new();
        let foreign = Runtime::new().reactive_scope().signal(0u32);
        let watched = foreign.clone();

        let runs = Rc::new(Cell::new(0));
        let counter = runs.clone();
        runtime.effect(move || {
            let _ = watched.get();
            counter.set(counter.get() + 1);
        });
        runtime.flush();
        assert_eq!(runs.get(), 1, "首次 flush 跑一次");

        // 改**外来**信号 ⇒ ⇒ 本运行时的 effect **不得**被驱动。
        foreign.set(1);
        runtime.flush();
        assert_eq!(
            runs.get(),
            1,
            "外来运行时的信号不得驱动本运行时的 effect（幽灵输入的结构条件）"
        );
    }
}
