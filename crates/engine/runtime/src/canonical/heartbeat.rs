//! AI 自驱心跳调度器 (HeartbeatScheduler) + 常驻心跳生产者 (HeartbeatProducer).
//!
//! 统一管理智能体的自主唤醒、环境定时感知、异步后台任务轮询与用户交互事件，
//! 支持 5 大触发源、5 级优先级抢占式二叉堆队列与“心流锁”保护机制.
//!
//! 常驻心跳生产者 (关怀生产者同一条待接线通道) 就接在这个调度缝上: 定时
//! poll (纯规则轮询, 零模型调用) → 命中发起标准回合 → 优雅停机收线程。

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// 心跳事件触发源类型.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeartbeatTriggerSource {
    /// 定时器时钟滴答 (Cron / Interval)
    Timer,
    /// 外部环境感知事件 (屏幕/窗口变化/文件变动)
    EnvironmentEvent,
    /// 内部子 Agent 或器官协同请求
    InternalAgent,
    /// 用户主动开口或交互输入 (最高优先级)
    UserInteraction,
    /// 异步后台任务完成回调
    AsyncTaskCallback,
}

/// 心跳任务项 (实现优先级反向排序以支持 BinaryHeap 作为最大堆).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatTask {
    pub id: String,
    pub source: HeartbeatTriggerSource,
    /// 优先级 (0 ~ 255, 数值越高优先级越高)
    pub priority: u8,
    pub payload: String,
    pub scheduled_at_ms: u64,
}

impl Ord for HeartbeatTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.scheduled_at_ms.cmp(&self.scheduled_at_ms))
    }
}

impl PartialOrd for HeartbeatTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 心流锁状态 (FlowLock).
///
/// 当智能体处于深度思考或关键连贯任务时，心流锁会阻止低优先级的闲散事件打断执行.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FlowLock {
    pub is_locked: bool,
    pub minimum_preemption_priority: u8,
    pub locked_reason: String,
}

/// 心跳调度器.
#[derive(Debug, Clone, Default)]
pub struct HeartbeatScheduler {
    task_queue: BinaryHeap<HeartbeatTask>,
    flow_lock: FlowLock,
}

impl HeartbeatScheduler {
    pub fn new() -> Self {
        Self {
            task_queue: BinaryHeap::new(),
            flow_lock: FlowLock::default(),
        }
    }

    /// 启用心流锁，保护当前任务不被低于 `min_priority` 的低优先级事件打断.
    pub fn acquire_flow_lock(&mut self, min_priority: u8, reason: &str) {
        self.flow_lock = FlowLock {
            is_locked: true,
            minimum_preemption_priority: min_priority,
            locked_reason: reason.to_string(),
        };
    }

    /// 释放心流锁.
    pub fn release_flow_lock(&mut self) {
        self.flow_lock = FlowLock::default();
    }

    /// 调度一个新心跳任务入队.
    pub fn schedule_task(&mut self, task: HeartbeatTask) {
        self.task_queue.push(task);
    }

    /// 弹出下一个应执行的高优先级任务 (受心流锁与时间戳约束).
    pub fn poll_next_task(&mut self, now_ms: u64) -> Option<HeartbeatTask> {
        if let Some(top) = self.task_queue.peek() {
            // 尚未到达预定执行时间
            if top.scheduled_at_ms > now_ms {
                return None;
            }

            // 若心流锁已锁定且任务优先级不足以抢占，则暂不弹出
            if self.flow_lock.is_locked && top.priority < self.flow_lock.minimum_preemption_priority
            {
                return None;
            }
        }

        self.task_queue.pop()
    }

    /// 当前队列待处理任务数.
    pub fn pending_count(&self) -> usize {
        self.task_queue.len()
    }
}

/// 心跳生产者 poll 间隔缺省: 5 分钟 (可配, 见 [`HeartbeatProducerConfig`])。
pub const DEFAULT_HEARTBEAT_POLL_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// 心流锁拦下 poll 时的重试拍缺省: 1 秒 (短拍快试, 候选不丢)。
pub const DEFAULT_HEARTBEAT_FLOW_LOCK_RETRY: Duration = Duration::from_secs(1);

/// 心跳 poll 任务在调度缝里的优先级: 闲散事件级 (低于用户交互与关键任务)。
///
/// [`HeartbeatScheduler::acquire_flow_lock`] 锁到高于本值的最小抢占优先级时,
/// 本轮 poll 在调度缝被拦下 (深度工作不插话), 短拍重试 (候选不丢)。
pub const HEARTBEAT_POLL_PRIORITY: u8 = 20;

/// 心跳 poll 的稳定任务 id (调度缝留痕)。
pub const HEARTBEAT_POLL_TASK_ID: &str = "heartbeat_poll";

/// 心跳 poll 的稳定任务载荷 (调度缝留痕)。
pub const HEARTBEAT_POLL_PAYLOAD: &str = "care.poll";

/// 一次心跳 poll 的产出 (poll = 纯规则轮询, 零模型调用)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeartbeatPollOutcome {
    /// 零动作 (无到点候选 / 打扰纪律拦下 / 发起闸拦下 / 判定说不): 零回合零计费。
    Quiet,
    /// 发起: 全闸放行, 标准回合输入已备好 → 走标准回合 (零新消息通道)。
    Initiate {
        /// 标准回合输入。
        turn_input: String,
        /// 发起时刻 (epoch 毫秒, poll 侧时钟真值)。
        at_ms: i64,
    },
}

/// 心跳 poll 面 (每 tick 调用一次; 组装根把关怀内核的 `poll` 接进来)。
pub trait HeartbeatPoll: Send {
    /// 一次 poll。`Err` = 本轮 poll 失败 (真因字符串; 失败即帧式留痕, 不吞错)。
    fn poll(&mut self) -> Result<HeartbeatPollOutcome, String>;
}

/// 发起面 (标准回合的执行缝: 常驻宿主接 canonical agent loop / `Runtime::execute`)。
pub trait HeartbeatDispatch: Send {
    /// 发起一个标准回合 (`turn_input` = 标准回合输入; `at_ms` = 发起时刻)。
    fn dispatch(&mut self, turn_input: &str, at_ms: i64);
}

/// 心跳生产者运行态 (自述「心跳态」的真值口: 下次 poll 时刻 / 最近心跳命中)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeartbeatProducerStatus {
    /// 下次 poll 时刻 (epoch 毫秒); `None` = 尚未排程。
    pub next_poll_at_ms: Option<i64>,
    /// 最近一次心跳命中 (发起) 时刻; `None` = 尚无命中。
    pub last_hit_at_ms: Option<i64>,
    /// 最近一次 poll 时刻; `None` = 尚未 poll。
    pub last_poll_at_ms: Option<i64>,
    /// poll 累计 (含零动作与失败)。
    pub polls: u64,
    /// 命中发起累计。
    pub hits: u64,
    /// 最近一次 poll 失败真因; `None` = 最近 poll 正常 (下一成功 poll 清除)。
    pub last_error: Option<String>,
}

/// 心跳生产者配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeartbeatProducerConfig {
    /// poll 间隔 (缺省 [`DEFAULT_HEARTBEAT_POLL_INTERVAL`] = 5 分钟, 可配)。
    pub poll_interval: Duration,
    /// 心流锁拦下 poll 时的重试拍 (缺省 [`DEFAULT_HEARTBEAT_FLOW_LOCK_RETRY`])。
    pub flow_lock_retry: Duration,
}

impl Default for HeartbeatProducerConfig {
    fn default() -> Self {
        Self {
            poll_interval: DEFAULT_HEARTBEAT_POLL_INTERVAL,
            flow_lock_retry: DEFAULT_HEARTBEAT_FLOW_LOCK_RETRY,
        }
    }
}

impl HeartbeatProducerConfig {
    /// 配 poll 间隔 (可配面)。
    #[must_use]
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// 配心流锁重试拍 (测试/嵌入面)。
    #[must_use]
    pub fn with_flow_lock_retry(mut self, retry: Duration) -> Self {
        self.flow_lock_retry = retry;
        self
    }
}

struct HeartbeatProducerShared {
    stop: Mutex<bool>,
    stop_cv: Condvar,
    status: Mutex<HeartbeatProducerStatus>,
}

impl HeartbeatProducerShared {
    fn new() -> Self {
        Self {
            stop: Mutex::new(false),
            stop_cv: Condvar::new(),
            status: Mutex::new(HeartbeatProducerStatus::default()),
        }
    }

    fn lock_stop(&self) -> std::sync::MutexGuard<'_, bool> {
        self.stop.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn lock_status(&self) -> std::sync::MutexGuard<'_, HeartbeatProducerStatus> {
        self.status.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// 常驻心跳生产者: 定时 poll (走 [`HeartbeatScheduler`] 调度缝) → 命中发起
/// 标准回合 → 优雅停机收线程 (不僵尸)。
///
/// # 时序 (一次 tick)
///
/// ```text
/// wait(Condvar, poll_interval) ── stop ──> 退出 (线程回收, 不僵尸)
///        │ 到点
///        ▼
/// HeartbeatScheduler::schedule_task(Timer, priority = HEARTBEAT_POLL_PRIORITY)
///        ▼
/// HeartbeatScheduler::poll_next_task(now)
///   ├─ None (心流锁拦下) → 短拍重试 (深度工作不插话, 候选不丢)
///   └─ Some(task)
///        ▼
/// HeartbeatPoll::poll()          ← 纯规则轮询, 零模型调用
///   ├─ Err(真因)   → 失败即帧式留痕 (status.last_error), 下拍照常
///   ├─ Quiet       → 零动作零计费
///   └─ Initiate    → HeartbeatDispatch::dispatch(标准回合输入) → 进会话流
/// ```
///
/// 节拍走真实单调时钟 (不随注入时钟漂移); 状态时刻戳走注入时钟 `now_ms`
/// (生产 = 系统时钟, 测试 = 冻结时钟, 两拍同一口径)。
pub struct HeartbeatProducer {
    config: HeartbeatProducerConfig,
    shared: Arc<HeartbeatProducerShared>,
    scheduler: Arc<Mutex<HeartbeatScheduler>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl HeartbeatProducer {
    /// 起常驻心跳线程 (首个 poll 即刻, 之后每 `poll_interval` 一拍)。
    ///
    /// `now_ms` = 心跳侧时钟 (状态时刻戳与调度缝共用); `poll` = 纯规则轮询面;
    /// `dispatch` = 标准回合执行缝 (发起即走标准回合, 零新消息通道)。
    pub fn spawn<C, P, D>(config: HeartbeatProducerConfig, now_ms: C, poll: P, dispatch: D) -> Self
    where
        C: Fn() -> i64 + Send + Sync + 'static,
        P: HeartbeatPoll + 'static,
        D: HeartbeatDispatch + 'static,
    {
        let shared = Arc::new(HeartbeatProducerShared::new());
        let scheduler = Arc::new(Mutex::new(HeartbeatScheduler::new()));
        let handle = {
            let shared = Arc::clone(&shared);
            let scheduler = Arc::clone(&scheduler);
            let run_config = config.clone();
            std::thread::Builder::new()
                .name("apeireth-heartbeat-poll".to_string())
                .spawn(move || {
                    run_producer_loop(&shared, &scheduler, &run_config, now_ms, poll, dispatch);
                })
                .expect("heartbeat producer thread spawns")
        };
        Self {
            config,
            shared,
            scheduler,
            handle: Some(handle),
        }
    }

    /// poll 间隔 (可配真值)。
    pub fn poll_interval(&self) -> Duration {
        self.config.poll_interval
    }

    /// 调度缝共享口 (心流锁持有者经它拦/放闲散 poll; 调度队列同源留痕)。
    pub fn scheduler(&self) -> Arc<Mutex<HeartbeatScheduler>> {
        Arc::clone(&self.scheduler)
    }

    /// 运行态真值 (自述关怀节心跳投影的来源: 下次 poll 时刻 / 最近心跳命中)。
    pub fn status(&self) -> HeartbeatProducerStatus {
        self.shared.lock_status().clone()
    }

    /// 心跳线程是否仍在运行。
    pub fn is_running(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
    }

    /// 线程句柄是否已回收 (停机清理的无泄漏断言口: `true` = 句柄不外泄)。
    pub fn thread_reaped(&self) -> bool {
        self.handle.is_none()
    }

    /// 优雅停机: 唤醒并 join 心跳线程 (不僵尸); 幂等。
    ///
    /// 返回 `true` = 本次真收掉了一个线程; `false` = 已收过 (幂等空操作)。
    pub fn shutdown(&mut self) -> bool {
        {
            let mut stopped = self.shared.lock_stop();
            *stopped = true;
        }
        self.shared.stop_cv.notify_all();
        match self.handle.take() {
            Some(handle) => {
                let _ = handle.join();
                true
            }
            None => false,
        }
    }
}

impl Drop for HeartbeatProducer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// 心跳线程主循环 (时序见 [`HeartbeatProducer`])。
fn run_producer_loop<C, P, D>(
    shared: &Arc<HeartbeatProducerShared>,
    scheduler: &Arc<Mutex<HeartbeatScheduler>>,
    config: &HeartbeatProducerConfig,
    now_ms: C,
    mut poll: P,
    mut dispatch: D,
) where
    C: Fn() -> i64,
    P: HeartbeatPoll,
    D: HeartbeatDispatch,
{
    let interval_ms = duration_ms(config.poll_interval);
    let retry_ms = duration_ms(config.flow_lock_retry);
    // 首拍即刻: 常驻一起就查一遍 (到点就发, 不漏)。
    let mut next_poll_at = Instant::now();
    loop {
        let now = Instant::now();
        if now < next_poll_at {
            if sleep_until(shared, next_poll_at.duration_since(now)) {
                break;
            }
            continue;
        }

        // 到点: 把这一拍交给调度缝 (心流锁可拦闲散事件)。
        let clock_ms = now_ms();
        let seam_now = u64::try_from(clock_ms).unwrap_or(0);
        let mut scheduler = scheduler.lock().unwrap_or_else(|p| p.into_inner());
        if scheduler.pending_count() == 0 {
            scheduler.schedule_task(HeartbeatTask {
                id: HEARTBEAT_POLL_TASK_ID.to_string(),
                source: HeartbeatTriggerSource::Timer,
                priority: HEARTBEAT_POLL_PRIORITY,
                payload: HEARTBEAT_POLL_PAYLOAD.to_string(),
                scheduled_at_ms: seam_now,
            });
        }
        if scheduler.poll_next_task(seam_now).is_none() {
            // 心流锁拦下本轮 poll (深度工作不插话): 短拍重试, 候选不丢。
            drop(scheduler);
            next_poll_at = Instant::now() + config.flow_lock_retry;
            shared.lock_status().next_poll_at_ms = Some(clock_ms + retry_ms);
            continue;
        }
        drop(scheduler);

        // 一轮 poll (纯规则轮询, 零模型调用)。
        let clock_ms = now_ms();
        next_poll_at = Instant::now() + config.poll_interval;
        {
            let mut status = shared.lock_status();
            status.last_poll_at_ms = Some(clock_ms);
            status.next_poll_at_ms = Some(clock_ms + interval_ms);
            status.polls += 1;
        }
        match poll.poll() {
            Ok(HeartbeatPollOutcome::Quiet) => {
                // 下一成功 poll 清除旧失败真因 (失败即帧式留痕, 不粘滞)。
                shared.lock_status().last_error = None;
            }
            Ok(HeartbeatPollOutcome::Initiate { turn_input, at_ms }) => {
                {
                    let mut status = shared.lock_status();
                    status.hits += 1;
                    status.last_hit_at_ms = Some(at_ms);
                    status.last_error = None;
                }
                dispatch.dispatch(&turn_input, at_ms);
            }
            Err(reason) => {
                shared.lock_status().last_error = Some(reason);
            }
        }

        if sleep_until(shared, config.poll_interval) {
            break;
        }
    }
}

/// 睡到下一拍; `true` = 停机请求已到 (退出循环)。
fn sleep_until(shared: &HeartbeatProducerShared, duration: Duration) -> bool {
    let stopped = shared.lock_stop();
    if *stopped {
        return true;
    }
    match shared.stop_cv.wait_timeout(stopped, duration) {
        Ok((guard, _)) => *guard,
        Err(poisoned) => *poisoned.into_inner().0,
    }
}

fn duration_ms(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_priority_preemption_and_flow_lock() {
        let mut scheduler = HeartbeatScheduler::new();

        let low_task = HeartbeatTask {
            id: "cron_1".to_string(),
            source: HeartbeatTriggerSource::Timer,
            priority: 10,
            payload: "检查天气".to_string(),
            scheduled_at_ms: 1000,
        };

        let high_task = HeartbeatTask {
            id: "user_1".to_string(),
            source: HeartbeatTriggerSource::UserInteraction,
            priority: 100,
            payload: "主人叫我".to_string(),
            scheduled_at_ms: 1000,
        };

        scheduler.schedule_task(low_task);
        scheduler.schedule_task(high_task);

        // 1. 高优先级任务优先被弹出
        let first = scheduler.poll_next_task(1000).unwrap();
        assert_eq!(first.id, "user_1");

        // 2. 模拟进入深度思考并获取心流锁 (要求优先级 >= 50 才能打断)
        scheduler.acquire_flow_lock(50, "正在进行代码重构思考");

        // 此时队列中只剩 priority=10 的低优先级任务，心流锁生效，返回 None
        assert!(scheduler.poll_next_task(1000).is_none());
        assert_eq!(scheduler.pending_count(), 1);

        // 3. 释放心流锁后，低优先级任务正常弹出
        scheduler.release_flow_lock();
        let second = scheduler.poll_next_task(1000).unwrap();
        assert_eq!(second.id, "cron_1");
    }

    struct FnPoll<F>(F);
    impl<F> HeartbeatPoll for FnPoll<F>
    where
        F: FnMut() -> Result<HeartbeatPollOutcome, String> + Send,
    {
        fn poll(&mut self) -> Result<HeartbeatPollOutcome, String> {
            (self.0)()
        }
    }

    struct FnDispatch<F>(F);
    impl<F> HeartbeatDispatch for FnDispatch<F>
    where
        F: FnMut(&str, i64) + Send,
    {
        fn dispatch(&mut self, turn_input: &str, at_ms: i64) {
            (self.0)(turn_input, at_ms)
        }
    }

    fn fast_config() -> HeartbeatProducerConfig {
        HeartbeatProducerConfig::default()
            .with_poll_interval(Duration::from_millis(20))
            .with_flow_lock_retry(Duration::from_millis(20))
    }

    fn wait_for(what: &str, deadline: Duration, mut cond: impl FnMut() -> bool) {
        let until = Instant::now() + deadline;
        while Instant::now() < until {
            if cond() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(cond(), "超时未等到: {what}");
    }

    /// poll 间隔缺省 5 分钟, 可配; 自述「下次 poll 时刻」真值 = 时钟 + 间隔。
    #[test]
    fn the_poll_interval_defaults_to_five_minutes_and_is_configurable() {
        assert_eq!(
            HeartbeatProducerConfig::default().poll_interval,
            Duration::from_secs(300),
            "缺省 5 分钟"
        );
        let mut producer = HeartbeatProducer::spawn(
            HeartbeatProducerConfig::default().with_poll_interval(Duration::from_secs(45)),
            || 1_000,
            FnPoll(|| Ok(HeartbeatPollOutcome::Quiet)),
            FnDispatch(|_turn_input: &str, _at_ms: i64| {}),
        );
        assert_eq!(producer.poll_interval(), Duration::from_secs(45));
        assert!(producer.is_running(), "常驻心跳线程已起");
        wait_for("首拍", Duration::from_secs(5), || {
            producer.status().polls >= 1
        });
        assert_eq!(
            producer.status().next_poll_at_ms,
            Some(1_000 + 45_000),
            "下次 poll 时刻 = 时钟 + 间隔 (如实)"
        );
        assert!(producer.shutdown());
    }

    /// 命中 → 发起标准回合输入 → 发起留痕 (最近心跳命中真值)。
    #[test]
    fn a_hit_dispatches_the_standard_turn_input_and_records_the_hit() {
        let dispatched: Arc<Mutex<Vec<(String, i64)>>> = Arc::new(Mutex::new(Vec::new()));
        let ticks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut producer = HeartbeatProducer::spawn(
            fast_config(),
            || 7_000,
            FnPoll({
                let ticks = Arc::clone(&ticks);
                move || {
                    let tick = ticks.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if tick == 1 {
                        Ok(HeartbeatPollOutcome::Initiate {
                            turn_input: "[care:check-in] 关怀回合输入".to_string(),
                            at_ms: 7_001,
                        })
                    } else {
                        Ok(HeartbeatPollOutcome::Quiet)
                    }
                }
            }),
            FnDispatch({
                let dispatched = Arc::clone(&dispatched);
                move |turn_input: &str, at_ms: i64| {
                    dispatched
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .push((turn_input.to_string(), at_ms));
                }
            }),
        );
        wait_for("命中发起", Duration::from_secs(5), || {
            !dispatched
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .is_empty()
        });
        assert!(producer.shutdown(), "停机收线程");
        assert_eq!(
            *dispatched.lock().unwrap_or_else(|p| p.into_inner()),
            vec![("[care:check-in] 关怀回合输入".to_string(), 7_001)]
        );
        let status = producer.status();
        assert_eq!(status.hits, 1);
        assert_eq!(status.last_hit_at_ms, Some(7_001), "最近心跳命中真值");
    }

    /// 零动作 poll = 零发起零计费。
    #[test]
    fn zero_action_polls_never_dispatch() {
        let dispatched = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let ticks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut producer = HeartbeatProducer::spawn(
            fast_config(),
            || 0,
            FnPoll({
                let ticks = Arc::clone(&ticks);
                move || {
                    ticks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(HeartbeatPollOutcome::Quiet)
                }
            }),
            FnDispatch({
                let dispatched = Arc::clone(&dispatched);
                move |_turn_input: &str, _at_ms: i64| {
                    dispatched.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }),
        );
        wait_for("若干拍", Duration::from_secs(5), || {
            ticks.load(std::sync::atomic::Ordering::SeqCst) >= 4
        });
        assert!(producer.shutdown());
        assert_eq!(
            dispatched.load(std::sync::atomic::Ordering::SeqCst),
            0,
            "零动作 = 零发起 (零回合零计费)"
        );
        assert_eq!(producer.status().hits, 0);
    }

    /// 优雅停机: 线程句柄无泄漏 (join 回收), 不僵尸 (停机后零 poll), 幂等。
    #[test]
    fn shutdown_reaps_the_thread_handle_without_leaking_and_stops_ghost_ticks() {
        let mut producers: Vec<HeartbeatProducer> = (0..8)
            .map(|_| {
                HeartbeatProducer::spawn(
                    fast_config(),
                    || 0,
                    FnPoll(|| Ok(HeartbeatPollOutcome::Quiet)),
                    FnDispatch(|_turn_input: &str, _at_ms: i64| {}),
                )
            })
            .collect();
        for producer in &producers {
            assert!(producer.is_running(), "常驻心跳线程在跑");
            assert!(!producer.thread_reaped(), "运行中句柄未回收 (如实)");
        }
        for producer in &mut producers {
            assert!(producer.shutdown(), "第一次停机真收掉线程");
            assert!(!producer.is_running(), "停机后不僵尸");
            assert!(producer.thread_reaped(), "线程句柄无泄漏 (join 已回收)");
        }
        for producer in &mut producers {
            assert!(!producer.shutdown(), "停机幂等");
        }
        // 停机后无幽灵 tick: poll 计数冻结。
        let first = &producers[0];
        let frozen = first.status().polls;
        std::thread::sleep(Duration::from_millis(120));
        assert_eq!(first.status().polls, frozen, "停机后不再 poll (不僵尸)");
    }

    /// poll 失败即帧式留痕 (真因不吞), 循环不倒 (下拍照常)。
    #[test]
    fn poll_failures_are_reported_honestly_and_do_not_kill_the_loop() {
        let ticks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut producer = HeartbeatProducer::spawn(
            fast_config(),
            || 0,
            FnPoll({
                let ticks = Arc::clone(&ticks);
                move || {
                    ticks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err("care_rules.json refused to open: bad doc".to_string())
                }
            }),
            FnDispatch(|_turn_input: &str, _at_ms: i64| {}),
        );
        wait_for("若干拍", Duration::from_secs(5), || {
            ticks.load(std::sync::atomic::Ordering::SeqCst) >= 3
        });
        assert!(producer.shutdown());
        assert_eq!(
            producer.status().last_error.as_deref(),
            Some("care_rules.json refused to open: bad doc"),
            "失败真因留痕 (不吞错)"
        );
        assert!(producer.status().polls >= 3, "失败不杀循环 (下拍照常)");
        assert_eq!(producer.status().hits, 0);
    }

    /// 调度缝的心流锁拦下闲散 poll (深度工作不插话), 放锁后短拍重试恢复。
    #[test]
    fn the_flow_lock_at_the_scheduling_seam_holds_the_poll_until_released() {
        let ticks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut producer = HeartbeatProducer::spawn(
            fast_config(),
            || 0,
            FnPoll({
                let ticks = Arc::clone(&ticks);
                move || {
                    ticks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(HeartbeatPollOutcome::Quiet)
                }
            }),
            FnDispatch(|_turn_input: &str, _at_ms: i64| {}),
        );
        wait_for("首拍", Duration::from_secs(5), || {
            ticks.load(std::sync::atomic::Ordering::SeqCst) >= 1
        });
        producer
            .scheduler()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .acquire_flow_lock(HEARTBEAT_POLL_PRIORITY + 1, "深度思考不插话");
        std::thread::sleep(Duration::from_millis(100)); // 排空在途拍
        let held = ticks.load(std::sync::atomic::Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(
            ticks.load(std::sync::atomic::Ordering::SeqCst),
            held,
            "心流锁拦下闲散 poll (不插话)"
        );
        producer
            .scheduler()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .release_flow_lock();
        wait_for("放锁恢复", Duration::from_secs(5), || {
            ticks.load(std::sync::atomic::Ordering::SeqCst) > held
        });
        assert!(producer.shutdown());
    }
}
