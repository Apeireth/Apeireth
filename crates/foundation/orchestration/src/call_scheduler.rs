//! 并行调用分类调度 [`CallScheduler`]: 判定器 + 互斥屏障 + 有界滚动池 + 模型序提交 + 取消补果。
//!
//! # 问题
//!
//! 模型一次发出多个工具调用时, 全串行让等待时间线性叠加, 全并行又会两类翻车:
//! 互斥调用 (写同一资源 / 有副作用) 相互撞车, 以及结果乱序交付毒害模型对
//! 执行次序的理解。本模块在"哪些调用并行"这一层给出确定答案:
//!
//! 1. **判定器** ([`ConcurrencySafety::is_concurrency_safe`]): `(工具, 参数)`
//!    判定一次调用是否可与其它调用并发。**默认失败关闭** —— 白名单
//!    ([`SafetyWhitelist`]) 为空时一切调用判为 [`ConcurrencyClass::Exclusive`],
//!    只有显式放行的规则才能把调用判为 [`ConcurrencyClass::Parallel`]。
//! 2. **互斥屏障**: [`ConcurrencyClass::Exclusive`] 调用独占执行 —— 启动前
//!    排干一切在飞调用, 执行期间禁止任何其它调用启动, 与前后调用成串行屏障。
//! 3. **有界滚动池**: [`ConcurrencyClass::Parallel`] 调用进滚动池并发执行,
//!    并发上限可配 ([`SchedulerConfig::max_parallel`]), 超限排队不丢弃;
//!    任一调用完成即释放槽位、滚动启动下一条。
//! 4. **结果提交保持模型序**: 派发可重叠、完成可乱序, 但 [`CallBatch::next_outcome`]
//!    严格按下标 (模型原始顺序) 交付, 乱序完成也按序交付。
//! 5. **取消补果** ([`CallBatch::abort`]): 未启动的调用补
//!    [`CancelKind::NotStarted`] 合成结果; 已启动的调用由统一超时件
//!    ([`apeireth_core::deadline::Deadline`]) 给出收场窗口 —— 窗口内完成的
//!    交付真实结果, 到期未完的被取消并补 [`CancelKind::WindDownExpired`]
//!    合成结果。无论哪条路径, 每个调用恰好一条结果, 顺序仍是模型序 ——
//!    回放与重试语义因此保持完整。
//!
//! # 分工边界 (与相邻机制语义互补, 互不替代)
//!
//! - **单调用执行阶段** (治理评估 / 冻结放行 / 单次执行的阶段划分): 本模块
//!   不看单个调用内部怎么走, 执行体整体以 [`CallRunner`] 不透明交付; 本模块
//!   只决定"哪些调用并行、何时启动、按什么顺序交付结果"。
//! - **统一超时** (见 `apeireth_core::deadline`): 到期只通知不杀活的纪律不变;
//!   本模块是被通知后选择收场动作的持有方, 不重复实现超时。
//! - **调用数量上限** (派发面的单轮截断): 截断发生在调度之前; 进入本模块的
//!   调用集合就是最终集合, 本模块不增不减。
//!
//! # 运行时上下文
//!
//! [`CallScheduler::start_batch`] 以任务方式并发执行调用, 必须在异步运行时
//! 上下文中调用。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::deadline::Deadline;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// 滚动池的缺省并发上限。
pub const DEFAULT_MAX_PARALLEL: usize = 4;

/// 取消后给已启动调用的缺省收场窗口 ([`CallBatch::abort`] 语义 5)。
pub const DEFAULT_WIND_DOWN: Duration = Duration::from_millis(1_000);

/// 一次调用的并发分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcurrencyClass {
    /// 独占: 与任何在飞调用互斥, 前后成串行屏障。
    Exclusive,
    /// 可并发: 进有界滚动池。
    Parallel,
}

/// 调用并发安全性判定器: `(工具, 参数)` 判定一次调用是否可与其它调用并发。
///
/// 纪律: **默认失败关闭** —— 没有显式放行规则时必须返回 `false`。判定只看
/// 工具名与参数, 不看调用 id 或时序; 同一 `(工具, 参数)` 的两次调用必须得到
/// 同一判定, 否则调度不可复现。
pub trait ConcurrencySafety: Send + Sync {
    /// `true` 当且仅当该 `(工具, 参数)` 组合被显式放行为可并发。
    fn is_concurrency_safe(&self, tool: &str, args: &serde_json::Value) -> bool;

    /// 判定并发分类: 放行即 [`ConcurrencyClass::Parallel`], 其余
    /// ([`ConcurrencyClass::Exclusive`], 含失败关闭) 一律独占。
    fn classify(&self, tool: &str, args: &serde_json::Value) -> ConcurrencyClass {
        if self.is_concurrency_safe(tool, args) {
            ConcurrencyClass::Parallel
        } else {
            ConcurrencyClass::Exclusive
        }
    }
}

/// 一条显式放行规则: 工具名全等 + 可选参数谓词同时满足才放行。
pub struct SafetyRule {
    tool: String,
    arg_filter: Option<Arc<dyn Fn(&serde_json::Value) -> bool + Send + Sync>>,
}

impl SafetyRule {
    /// 放行该工具的一切参数组合。
    pub fn for_tool(tool: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            arg_filter: None,
        }
    }

    /// 仅当 `filter(args)` 为 `true` 时放行该工具的该参数组合。
    pub fn for_tool_when(
        tool: impl Into<String>,
        filter: impl Fn(&serde_json::Value) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            tool: tool.into(),
            arg_filter: Some(Arc::new(filter)),
        }
    }

    /// 本规则对应的工具名。
    pub fn tool(&self) -> &str {
        &self.tool
    }

    fn matches(&self, tool: &str, args: &serde_json::Value) -> bool {
        self.tool == tool && self.arg_filter.as_ref().is_none_or(|filter| filter(args))
    }
}

/// 显式放行白名单: 逐条规则全等匹配, 没有规则命中即失败关闭。
///
/// 空白名单 (`Default`) 判定一切调用为不可并发 —— 这是刻意的缺省: 并发是
/// 显式授予的特权, 不是默认行为。
#[derive(Default)]
pub struct SafetyWhitelist {
    rules: Vec<SafetyRule>,
}

impl SafetyWhitelist {
    /// 空白名单 (失败关闭)。
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一条放行规则。
    #[must_use]
    pub fn allow(mut self, rule: SafetyRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// 当前规则条数。
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl ConcurrencySafety for SafetyWhitelist {
    fn is_concurrency_safe(&self, tool: &str, args: &serde_json::Value) -> bool {
        self.rules.iter().any(|rule| rule.matches(tool, args))
    }
}

/// 进入调度的一次调用的最小描述: 调度只需要身份、工具名与参数。
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledCall {
    /// 调用 id (与结果一一对应)。
    pub id: String,
    /// 模型侧工具名。
    pub tool: String,
    /// 调用参数 (判定器输入)。
    pub args: serde_json::Value,
}

impl ScheduledCall {
    /// 一次 `(id, 工具, 参数)` 的调用描述。
    pub fn new(id: impl Into<String>, tool: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            tool: tool.into(),
            args,
        }
    }
}

/// 调用执行体: 按槽位下标启动一次调用, 返回 `'static` 结果 future。
///
/// 执行体是"单个调用怎么走"的整段不透明交付 —— 本模块不看内部阶段。
pub trait CallRunner<R>: Send + Sync {
    /// 启动第 `index` 个调用; 返回的 future 在独立任务里被驱动至完成。
    fn start(&self, index: usize, call: &ScheduledCall) -> Pin<Box<dyn Future<Output = R> + Send>>;
}

impl<R, F> CallRunner<R> for F
where
    F: Fn(usize, &ScheduledCall) -> Pin<Box<dyn Future<Output = R> + Send>> + Send + Sync,
{
    fn start(&self, index: usize, call: &ScheduledCall) -> Pin<Box<dyn Future<Output = R> + Send>> {
        self(index, call)
    }
}

/// 取消补果的成因: 区分"从未启动"与"启动后收场超时"。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelKind {
    /// 未启动: 批次被取消时仍在排队, 执行体从未运行。
    NotStarted,
    /// 已启动但收场窗口内未交付: 执行体被取消, 可能已部分执行。
    WindDownExpired,
}

/// 一个槽位的最终去向: 真实结果或合成取消结果。
#[derive(Debug, Clone, PartialEq)]
pub enum SlotOutcome<R> {
    /// 执行体交付的真实结果 (含收场窗口内完成的)。
    Completed(R),
    /// 合成取消结果: 调用方据此补错误结果, 保证每个调用恰好一条结果。
    Cancelled(CancelKind),
}

impl<R> SlotOutcome<R> {
    /// 真实结果 (取消时为空)。
    pub fn completed(self) -> Option<R> {
        match self {
            Self::Completed(result) => Some(result),
            Self::Cancelled(_) => None,
        }
    }

    /// 取消成因 (真实结果时为空)。
    pub fn cancel_kind(&self) -> Option<CancelKind> {
        match self {
            Self::Completed(_) => None,
            Self::Cancelled(kind) => Some(*kind),
        }
    }
}

/// 调度器配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerConfig {
    /// 滚动池并发上限 (`>= 1`; 超限排队)。
    pub max_parallel: usize,
    /// 取消后给已启动调用的收场窗口。
    pub wind_down: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_parallel: DEFAULT_MAX_PARALLEL,
            wind_down: DEFAULT_WIND_DOWN,
        }
    }
}

/// 调度器配置被拒的成因 (缺省即拒, 不静默修正)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerConfigError {
    /// 并发上限为 0: 滚动池必须至少容纳一个调用。
    NonPositiveParallelism,
}

impl std::fmt::Display for SchedulerConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonPositiveParallelism => f.write_str(
                "max_parallel must be at least 1: a pool that admits nothing is not a pool",
            ),
        }
    }
}

impl std::error::Error for SchedulerConfigError {}

/// 分类调度器: 判定分类 + 派发批次。
///
/// 判定来自注入的 [`ConcurrencySafety`]; 批次执行 ([`CallBatch`]) 实现互斥
/// 屏障、有界滚动池、模型序提交与取消补果。
pub struct CallScheduler {
    safety: Arc<dyn ConcurrencySafety>,
    config: SchedulerConfig,
}

impl CallScheduler {
    /// 用给定判定器与配置组装调度器; 并发上限为 0 时拒绝 ([`SchedulerConfigError`])。
    pub fn new(
        safety: Arc<dyn ConcurrencySafety>,
        config: SchedulerConfig,
    ) -> Result<Self, SchedulerConfigError> {
        if config.max_parallel == 0 {
            return Err(SchedulerConfigError::NonPositiveParallelism);
        }
        Ok(Self { safety, config })
    }

    /// 判定器。
    pub fn safety(&self) -> &Arc<dyn ConcurrencySafety> {
        &self.safety
    }

    /// 滚动池并发上限。
    pub fn max_parallel(&self) -> usize {
        self.config.max_parallel
    }

    /// 取消收场窗口。
    pub fn wind_down(&self) -> Duration {
        self.config.wind_down
    }

    /// 启动一批调用的分类调度。
    ///
    /// 派发立即开始 (与调用方取结果重叠); [`CallBatch::next_outcome`] 按模型
    /// 序交付, [`CallBatch::abort`] 触发取消补果。必须在异步运行时上下文中
    /// 调用。
    pub fn start_batch<R: Send + 'static>(
        &self,
        calls: Vec<ScheduledCall>,
        runner: Arc<dyn CallRunner<R>>,
    ) -> CallBatch<R> {
        let classes = calls
            .iter()
            .map(|call| self.safety.classify(&call.tool, &call.args))
            .collect();
        CallBatch::spawn(calls, classes, runner, self.config)
    }
}

impl ConcurrencySafety for CallScheduler {
    fn is_concurrency_safe(&self, tool: &str, args: &serde_json::Value) -> bool {
        self.safety.is_concurrency_safe(tool, args)
    }
}

/// 槽位状态机: 排队 → 在飞 → 已定局。
enum SlotState<R> {
    /// 已排队, 尚未启动 (取消时补 [`CancelKind::NotStarted`])。
    Queued,
    /// 在飞: 执行体任务已启动。
    InFlight(JoinHandle<()>),
    /// 已定局; `None` 仅为交付时的取走瞬间。
    Settled(Option<SlotOutcome<R>>),
}

/// 一批调用的调度句柄: 模型序交付 + 取消补果。
pub struct CallBatch<R: Send + 'static> {
    calls: Vec<ScheduledCall>,
    classes: Vec<ConcurrencyClass>,
    slots: Vec<SlotState<R>>,
    runner: Arc<dyn CallRunner<R>>,
    commit_cursor: usize,
    launch_frontier: usize,
    in_flight: usize,
    exclusive_in_flight: bool,
    max_parallel: usize,
    wind_down: Duration,
    aborted: bool,
    wind_down_timer: Option<(Deadline, apeireth_core::deadline::DeadlineNotice)>,
    completed_tx: mpsc::UnboundedSender<(usize, R)>,
    completed_rx: mpsc::UnboundedReceiver<(usize, R)>,
}

impl<R: Send + 'static> CallBatch<R> {
    fn spawn(
        calls: Vec<ScheduledCall>,
        classes: Vec<ConcurrencyClass>,
        runner: Arc<dyn CallRunner<R>>,
        config: SchedulerConfig,
    ) -> Self {
        let (completed_tx, completed_rx) = mpsc::unbounded_channel();
        let slots = calls.iter().map(|_| SlotState::Queued).collect();
        let mut batch = Self {
            calls,
            classes,
            slots,
            runner,
            commit_cursor: 0,
            launch_frontier: 0,
            in_flight: 0,
            exclusive_in_flight: false,
            max_parallel: config.max_parallel,
            wind_down: config.wind_down,
            aborted: false,
            wind_down_timer: None,
            completed_tx,
            completed_rx,
        };
        batch.launch_ready();
        batch
    }

    /// 批次里的调用数 (每个调用恰好交付一次 [`SlotOutcome`])。
    pub fn len(&self) -> usize {
        self.calls.len()
    }

    /// 批次是否为空。
    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    /// 是否已进入取消补果。
    pub fn is_aborted(&self) -> bool {
        self.aborted
    }

    /// 按模型序交付下一个槽位的定局结果; 全部交付后返回 `None`。
    ///
    /// 严格按下标递增交付: 即使后位调用先完成, 也等前位定局后才交付。
    pub async fn next_outcome(&mut self) -> Option<(usize, SlotOutcome<R>)> {
        loop {
            self.launch_ready();
            if self.commit_cursor >= self.slots.len() {
                return None;
            }
            let index = self.commit_cursor;
            if let SlotState::Settled(slot) = &mut self.slots[index] {
                if let Some(outcome) = slot.take() {
                    self.commit_cursor += 1;
                    return Some((index, outcome));
                }
            }
            self.await_progress().await;
        }
    }

    /// 取消补果: 未启动的补 [`CancelKind::NotStarted`], 已启动的走统一超时件
    /// 收场 —— 收场窗口内完成的仍交付真实结果, 到期未完的取消并补
    /// [`CancelKind::WindDownExpired`]。幂等: 重复调用不改已定局结果。
    /// 收场定时器无法建立时立刻收场, 不让在飞调用悬空。
    pub fn abort(&mut self) {
        if self.aborted {
            return;
        }
        self.aborted = true;
        for slot in &mut self.slots {
            if matches!(slot, SlotState::Queued) {
                *slot = SlotState::Settled(Some(SlotOutcome::Cancelled(CancelKind::NotStarted)));
            }
        }
        self.wind_down_timer = Deadline::after(self.wind_down)
            .ok()
            .map(|(deadline, notice)| (deadline, notice));
        if self.wind_down_timer.is_none() {
            self.settle_in_flight_after_wind_down();
        }
    }

    /// 等一次调度进展 (完成回执或收场窗口到期), 并把进展折进槽位状态。
    async fn await_progress(&mut self) {
        enum Wake<R> {
            Completed(Option<(usize, R)>),
            WindDownFired,
        }

        let wake = {
            let completed_rx = &mut self.completed_rx;
            let wind_down_timer = &mut self.wind_down_timer;
            tokio::select! {
                maybe = completed_rx.recv() => Wake::Completed(maybe),
                _ = async {
                    match wind_down_timer {
                        Some((_, notice)) => {
                            notice.notified().await;
                        }
                        None => std::future::pending().await,
                    }
                } => Wake::WindDownFired,
            }
        };

        match wake {
            Wake::Completed(Some((index, result))) => self.settle_completed(index, result),
            Wake::Completed(None) => self.settle_in_flight_after_wind_down(),
            Wake::WindDownFired => self.settle_in_flight_after_wind_down(),
        }
    }

    /// 执行体完成回执: 定局为真实结果并释放槽位。
    ///
    /// 先定局者赢: 槽位已被收场窗口判为取消时, 迟到回执丢弃, 取消结果保持
    /// (收场窗口是唯一仲裁者, 定局不可回改)。
    fn settle_completed(&mut self, index: usize, result: R) {
        if matches!(self.slots[index], SlotState::Settled(_)) {
            return;
        }
        let previous = std::mem::replace(
            &mut self.slots[index],
            SlotState::Settled(Some(SlotOutcome::Completed(result))),
        );
        if let SlotState::InFlight(handle) = previous {
            drop(handle);
            self.release_slot(index);
        }
    }

    /// 收场窗口到期 (或完成通道关闭): 先取走已到的完成回执, 在飞调用取消并补
    /// [`CancelKind::WindDownExpired`], 仍在排队的补 [`CancelKind::NotStarted`]。
    fn settle_in_flight_after_wind_down(&mut self) {
        while let Ok((index, result)) = self.completed_rx.try_recv() {
            self.settle_completed(index, result);
        }
        for index in 0..self.slots.len() {
            let previous = std::mem::replace(
                &mut self.slots[index],
                SlotState::Settled(Some(SlotOutcome::Cancelled(CancelKind::WindDownExpired))),
            );
            match previous {
                SlotState::InFlight(handle) => {
                    handle.abort();
                    self.release_slot(index);
                }
                SlotState::Queued => {
                    self.slots[index] =
                        SlotState::Settled(Some(SlotOutcome::Cancelled(CancelKind::NotStarted)));
                }
                SlotState::Settled(previous) => {
                    self.slots[index] = SlotState::Settled(previous);
                }
            }
        }
        self.aborted = true;
    }

    fn release_slot(&mut self, index: usize) {
        self.in_flight = self.in_flight.saturating_sub(1);
        if self.classes[index] == ConcurrencyClass::Exclusive {
            self.exclusive_in_flight = false;
        }
    }

    /// 滚动启动: 队首可启动才启动 (FIFO, 超限排队)。
    ///
    /// 屏障纪律: 独占调用只在零在飞时启动, 且在飞期间不启动任何其它调用;
    /// 并行调用受 `max_parallel` 约束, 排在独占调用之后时等屏障排空。
    fn launch_ready(&mut self) {
        while !self.aborted && self.launch_frontier < self.slots.len() {
            let class = self.classes[self.launch_frontier];
            let admit = match class {
                ConcurrencyClass::Exclusive => self.in_flight == 0,
                ConcurrencyClass::Parallel => {
                    !self.exclusive_in_flight && self.in_flight < self.max_parallel
                }
            };
            if !admit {
                return;
            }
            self.launch_next();
        }
    }

    fn launch_next(&mut self) {
        let index = self.launch_frontier;
        let call = self.calls[index].clone();
        let runner = Arc::clone(&self.runner);
        let completed_tx = self.completed_tx.clone();
        let handle = tokio::spawn(async move {
            let result = runner.start(index, &call).await;
            let _ = completed_tx.send((index, result));
        });
        self.slots[index] = SlotState::InFlight(handle);
        self.in_flight += 1;
        if self.classes[index] == ConcurrencyClass::Exclusive {
            self.exclusive_in_flight = true;
        }
        self.launch_frontier += 1;
    }
}

impl<R: Send + 'static> Drop for CallBatch<R> {
    /// 句柄被弃 = 调用方不再收结果: 在飞执行体立即取消, 不留孤儿任务。
    fn drop(&mut self) {
        for slot in &self.slots {
            if let SlotState::InFlight(handle) = slot {
                handle.abort();
            }
        }
    }
}
