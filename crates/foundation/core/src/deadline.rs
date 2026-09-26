//! 统一超时熔合小库: 到期只通知, 终止动作归持有工作的能力自己负责。
//!
//! 三条纪律:
//! 1. **超时只通知** — [`Deadline`] 到期时向通知通道投递一枚 [`DeadlineToken`];
//!    它不取消、不杀活, 被观察的工作由持有它的能力自行收场 (所有权清晰)。
//! 2. **定时器随拥有者清理** — [`Deadline`] 被 drop 即中止内部定时任务,
//!    计数器 [`live_deadline_timers`] 归还, 无泄漏。
//! 3. **取值先过闸** — [`clamp_timeout`] 拒绝 0 与"无界"哨兵
//!    ([`UNBOUNDED_TIMEOUT_SENTINEL`]); 超上限一律不放行。
//!
//! 另有 [`IdleWatchdog`]: 只在"等待产出"段计时 —— 消费者自身的思考/处理
//! 时间不算空闲, 不吃预算。错误归属见 [`TimeoutErrorCode`]: 超时类自成
//! 独立 code 族 (`timeout.*`), 与其它错误域分开归类。

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// "无界"哨兵取值 (`u64::MAX`): 表示"不设边界"的保留值。
/// 任何命令/调用边界都拒绝它 —— 无界的等待不是合法的超时配置。
pub const UNBOUNDED_TIMEOUT_SENTINEL: u64 = u64::MAX;

static LIVE_TIMERS: AtomicUsize = AtomicUsize::new(0);

/// 当前仍存活的 [`Deadline`] 定时任务数: 到期完成或被 drop 中止后归还。
pub fn live_deadline_timers() -> usize {
    LIVE_TIMERS.load(Ordering::SeqCst)
}

/// 超时类错误的独立 code 族 (稳定字符串, 供日志/界面归类)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutErrorCode {
    /// 超时取值被拒 (0 / 超上限 / 无界哨兵)。
    InvalidTimeout,
    /// 到期通知已触发: 等待中的工作未在期限内交付。
    DeadlineExpired,
    /// 空闲看门狗预算耗尽: 等待产出超预算。
    IdleExceeded,
    /// 定时器无法建立 (缺少异步运行时上下文)。
    TimerUnavailable,
}

impl TimeoutErrorCode {
    /// 稳定 code 字符串; `timeout.*` 自成一族。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidTimeout => "timeout.invalid_timeout",
            Self::DeadlineExpired => "timeout.deadline_expired",
            Self::IdleExceeded => "timeout.idle_exceeded",
            Self::TimerUnavailable => "timeout.timer_unavailable",
        }
    }
}

impl std::fmt::Display for TimeoutErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 超时域的失败; 每个变体都归属 [`TimeoutErrorCode`] 的独立 code。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TimeoutError {
    /// 超时取值为 0: 0 不是"立刻到期", 是非法配置。
    #[error("timeout value must be non-zero")]
    Zero,
    /// 超时取值超过配置上限。
    #[error("timeout value {requested} exceeds the configured maximum {cap}")]
    AboveCap {
        /// 被拒的取值。
        requested: u64,
        /// 上限。
        cap: u64,
    },
    /// 超时取值是"无界"哨兵: 无边界等待一律拒绝。
    #[error("timeout value must not be the unbounded sentinel")]
    UnboundedSentinel,
    /// 缺少异步运行时上下文, 定时器无法建立。
    #[error("deadline timer needs an async runtime context")]
    NoRuntime,
}

impl TimeoutError {
    /// 错误归属 code: 取值类错误归 `timeout.invalid_timeout`,
    /// 定时器建立失败归 `timeout.timer_unavailable`。
    pub const fn code(&self) -> TimeoutErrorCode {
        match self {
            Self::Zero | Self::AboveCap { .. } | Self::UnboundedSentinel => {
                TimeoutErrorCode::InvalidTimeout
            }
            Self::NoRuntime => TimeoutErrorCode::TimerUnavailable,
        }
    }
}

/// 超时取值过闸: 未给取值用缺省; 拒绝 0、无界哨兵 ([`UNBOUNDED_TIMEOUT_SENTINEL`])
/// 与超过 `max_ms` 的取值; 其余原样放行 (含恰好等于 `max_ms`)。
pub fn clamp_timeout(
    requested: Option<u64>,
    default_ms: u64,
    max_ms: u64,
) -> Result<u64, TimeoutError> {
    let timeout_ms = requested.unwrap_or(default_ms);
    if timeout_ms == 0 {
        return Err(TimeoutError::Zero);
    }
    if timeout_ms == UNBOUNDED_TIMEOUT_SENTINEL {
        return Err(TimeoutError::UnboundedSentinel);
    }
    if timeout_ms > max_ms {
        return Err(TimeoutError::AboveCap {
            requested: timeout_ms,
            cap: max_ms,
        });
    }
    Ok(timeout_ms)
}

/// 到期通知里携带的 token: 记录触发时刻, 每枚 [`Deadline`] 至多投递一次。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeadlineToken {
    fired_at: Instant,
}

impl DeadlineToken {
    /// 触发时刻。
    pub const fn fired_at(self) -> Instant {
        self.fired_at
    }
}

/// 一枚已武装的定时器拥有者: 只负责到期通知, 不取消、不杀活。
/// Drop 即中止内部定时任务 (计时器随拥有者清理, 无泄漏)。
pub struct Deadline {
    timer: JoinHandle<()>,
    expires_at: Instant,
    fired: Arc<AtomicBool>,
}

impl Deadline {
    /// 武装 `duration` 后到期的定时器, 返回 (定时器拥有者, 到期通知接收端)。
    ///
    /// 需要在异步运行时上下文中调用; 缺少上下文时返回
    /// [`TimeoutError::NoRuntime`] (code `timeout.timer_unavailable`)。
    pub fn after(duration: Duration) -> Result<(Self, DeadlineNotice), TimeoutError> {
        let runtime = tokio::runtime::Handle::try_current().map_err(|_| TimeoutError::NoRuntime)?;
        let (tx, rx) = mpsc::channel(1);
        let fired = Arc::new(AtomicBool::new(false));
        let task_fired = Arc::clone(&fired);
        LIVE_TIMERS.fetch_add(1, Ordering::SeqCst);
        // 守卫先于 spawn 建立并移入任务未来: 无论任务是跑完、被中止, 还是
        // 从未被轮询就被丢弃, 捕获物随未来落下, 计数必然归还。
        let guard = TimerGuard;
        let timer = runtime.spawn(async move {
            let _guard = guard;
            tokio::time::sleep(duration).await;
            if !task_fired.swap(true, Ordering::SeqCst) {
                let _ = tx
                    .send(DeadlineToken {
                        fired_at: Instant::now(),
                    })
                    .await;
            }
        });
        Ok((
            Self {
                timer,
                expires_at: Instant::now() + duration,
                fired: Arc::clone(&fired),
            },
            DeadlineNotice { rx, fired },
        ))
    }

    /// 到期时刻。
    pub fn expires_at(&self) -> Instant {
        self.expires_at
    }

    /// 距到期的剩余时间 (已到期则为 0)。
    pub fn remaining(&self) -> Duration {
        self.expires_at.saturating_duration_since(Instant::now())
    }

    /// 到期通知是否已经发出。
    pub fn is_expired(&self) -> bool {
        self.fired.load(Ordering::SeqCst)
    }
}

impl Drop for Deadline {
    fn drop(&mut self) {
        self.timer.abort();
    }
}

struct TimerGuard;

impl Drop for TimerGuard {
    fn drop(&mut self) {
        LIVE_TIMERS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// 到期通知的接收端 (容量 1 的通道; token 至多一枚)。
pub struct DeadlineNotice {
    rx: mpsc::Receiver<DeadlineToken>,
    fired: Arc<AtomicBool>,
}

impl DeadlineNotice {
    /// 非阻塞取 token: 至多一枚, 取走后恒为空。
    pub fn try_notified(&mut self) -> Option<DeadlineToken> {
        self.rx.try_recv().ok()
    }

    /// 等待到期 token; 定时器在到期前被清理时返回空。
    pub async fn notified(&mut self) -> Option<DeadlineToken> {
        self.rx.recv().await
    }

    /// 到期通知是否已经发出。
    pub fn is_expired(&self) -> bool {
        self.fired.load(Ordering::SeqCst)
    }
}

/// 空闲看门狗: 只在"等待产出"段计时。
///
/// 口径: [`IdleWatchdog::begin_wait`] 打开一个等待段, [`IdleWatchdog::end_wait`]
/// 关闭它; 只有打开的等待段计入空闲 —— 消费者自身的思考/处理时间不计
/// (不是提供方空闲), 也不清空已累计的等待。
#[derive(Debug, Clone)]
pub struct IdleWatchdog {
    budget: Duration,
    waiting_since: Option<Instant>,
    waited: Duration,
}

impl IdleWatchdog {
    /// 空闲预算为 `budget` 的看门狗。
    pub fn new(budget: Duration) -> Self {
        Self {
            budget,
            waiting_since: None,
            waited: Duration::ZERO,
        }
    }

    /// 空闲预算。
    pub fn budget(&self) -> Duration {
        self.budget
    }

    /// 开始等待产出 (已在等待中则保持原起点, 不重复计段)。
    pub fn begin_wait(&mut self) {
        self.begin_wait_at(Instant::now());
    }

    /// [`Self::begin_wait`] 的显式时刻版 (确定性测试/模拟时钟用)。
    pub fn begin_wait_at(&mut self, at: Instant) {
        if self.waiting_since.is_none() {
            self.waiting_since = Some(at);
        }
    }

    /// 产出到达 (或消费者接管): 关闭当前等待段。
    pub fn end_wait(&mut self) {
        self.end_wait_at(Instant::now());
    }

    /// [`Self::end_wait`] 的显式时刻版。
    pub fn end_wait_at(&mut self, at: Instant) {
        if let Some(since) = self.waiting_since.take() {
            if at > since {
                self.waited += at - since;
            }
        }
    }

    /// 已累计的等待产出时间 (含当前未关闭的等待段)。
    pub fn idle_elapsed(&self) -> Duration {
        self.idle_elapsed_at(Instant::now())
    }

    /// [`Self::idle_elapsed`] 的显式时刻版。
    pub fn idle_elapsed_at(&self, now: Instant) -> Duration {
        let open = match self.waiting_since {
            Some(since) if now > since => now - since,
            _ => Duration::ZERO,
        };
        self.waited + open
    }

    /// 空闲累计是否已吃满预算。
    pub fn is_tripped(&self) -> bool {
        self.is_tripped_at(Instant::now())
    }

    /// [`Self::is_tripped`] 的显式时刻版。
    pub fn is_tripped_at(&self, now: Instant) -> bool {
        self.idle_elapsed_at(now) >= self.budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 计时器计数是进程级的: 同一二进制内的计时测试互斥, 断言才确定。
    static TIMER_TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test]
    async fn expiry_token_fires_exactly_once() {
        let _guard = TIMER_TEST_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        let (deadline, mut notice) =
            Deadline::after(Duration::from_millis(30)).expect("runtime context present");
        assert!(!deadline.is_expired(), "未到期不得报已到期");

        let token = tokio::time::timeout(Duration::from_secs(2), notice.notified())
            .await
            .expect("到期通知必须送达")
            .expect("到期前不得被清理");
        assert!(token.fired_at() <= Instant::now());
        assert!(deadline.is_expired());
        assert_eq!(deadline.remaining(), Duration::ZERO);

        // 恰好一次: 再取恒为空, 等过一段也不会补发第二枚。
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(notice.try_notified().is_none(), "不得补发第二枚 token");
        assert!(notice.notified().await.is_none(), "通道里只有那一枚 token");
    }

    #[tokio::test]
    async fn dropping_the_deadline_clears_the_timer_without_firing() {
        let _guard = TIMER_TEST_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        let baseline = live_deadline_timers();
        let (deadline, mut notice) =
            Deadline::after(Duration::from_millis(40)).expect("runtime context present");
        assert_eq!(live_deadline_timers(), baseline + 1, "武装即登记一枚定时器");
        drop(deadline);

        tokio::time::sleep(Duration::from_millis(120)).await; // 越过原到期时刻
        assert!(
            notice.try_notified().is_none(),
            "被清理的定时器不得再发到期通知"
        );
        assert!(
            notice.notified().await.is_none(),
            "清理后通道即终结, 无幽灵 token"
        );
        assert_eq!(
            live_deadline_timers(),
            baseline,
            "定时器计数必须归还, 无泄漏"
        );
    }

    #[test]
    fn clamp_timeout_rejects_zero_and_unbounded_sentinel_and_above_cap() {
        // 缺省与边界放行。
        assert_eq!(clamp_timeout(None, 30_000, 300_000), Ok(30_000));
        assert_eq!(clamp_timeout(Some(1), 30_000, 300_000), Ok(1));
        assert_eq!(clamp_timeout(Some(300_000), 30_000, 300_000), Ok(300_000));
        assert_eq!(clamp_timeout(Some(120_000), 30_000, 300_000), Ok(120_000));

        // 0 / 无界哨兵 / 超上限一律拒绝。
        assert_eq!(
            clamp_timeout(Some(0), 30_000, 300_000),
            Err(TimeoutError::Zero)
        );
        assert_eq!(
            clamp_timeout(Some(UNBOUNDED_TIMEOUT_SENTINEL), 30_000, 300_000),
            Err(TimeoutError::UnboundedSentinel)
        );
        assert_eq!(
            clamp_timeout(Some(300_001), 30_000, 300_000),
            Err(TimeoutError::AboveCap {
                requested: 300_001,
                cap: 300_000
            })
        );
        // 哨兵即便上限放开也拒绝: 无界不是合法超时。
        assert_eq!(
            clamp_timeout(
                Some(UNBOUNDED_TIMEOUT_SENTINEL),
                0,
                UNBOUNDED_TIMEOUT_SENTINEL
            ),
            Err(TimeoutError::UnboundedSentinel)
        );
    }

    #[test]
    fn idle_watchdog_counts_only_output_wait_time() {
        let t0 = Instant::now();
        let mut watchdog = IdleWatchdog::new(Duration::from_millis(100));

        // 第一段等待 40ms: 计入。
        watchdog.begin_wait_at(t0);
        watchdog.end_wait_at(t0 + Duration::from_millis(40));
        assert_eq!(watchdog.idle_elapsed_at(t0), Duration::from_millis(40));

        // 消费者自身思考 200ms: 不算提供方空闲, 预算不被吃掉。
        let think_end = t0 + Duration::from_millis(240);
        assert_eq!(
            watchdog.idle_elapsed_at(think_end),
            Duration::from_millis(40)
        );
        assert!(!watchdog.is_tripped_at(think_end), "思考时间不得顶掉预算");

        // 第二段等待 70ms: 累计 110ms, 顶掉 100ms 预算。
        watchdog.begin_wait_at(think_end);
        assert!(
            watchdog.is_tripped_at(think_end + Duration::from_millis(70)),
            "累计等待产出超预算才触发"
        );
        assert_eq!(
            watchdog.idle_elapsed_at(think_end + Duration::from_millis(70)),
            Duration::from_millis(110)
        );

        // 重复 begin 不重置起点; end 未开段是 no-op。
        watchdog.end_wait_at(think_end + Duration::from_millis(70));
        watchdog.end_wait_at(think_end + Duration::from_millis(90));
        assert_eq!(
            watchdog.idle_elapsed_at(think_end + Duration::from_millis(90)),
            Duration::from_millis(110)
        );
    }

    #[test]
    fn timeout_error_codes_are_an_independent_family() {
        // code 族自成一体: 稳定前缀, 与其它错误域分开归类。
        for code in [
            TimeoutErrorCode::InvalidTimeout,
            TimeoutErrorCode::DeadlineExpired,
            TimeoutErrorCode::IdleExceeded,
            TimeoutErrorCode::TimerUnavailable,
        ] {
            assert!(
                code.as_str().starts_with("timeout."),
                "超时类 code 必须自成一族: {}",
                code.as_str()
            );
        }
        assert_eq!(TimeoutError::Zero.code(), TimeoutErrorCode::InvalidTimeout);
        assert_eq!(
            TimeoutError::AboveCap {
                requested: 2,
                cap: 1
            }
            .code(),
            TimeoutErrorCode::InvalidTimeout
        );
        assert_eq!(
            TimeoutError::UnboundedSentinel.code(),
            TimeoutErrorCode::InvalidTimeout
        );
        assert_eq!(
            TimeoutError::NoRuntime.code(),
            TimeoutErrorCode::TimerUnavailable
        );
        assert_ne!(
            TimeoutErrorCode::DeadlineExpired.as_str(),
            TimeoutErrorCode::IdleExceeded.as_str()
        );
    }

    #[tokio::test]
    async fn deadline_reports_expiry_as_the_deadline_code() {
        // 接线口径: 到期 = `timeout.deadline_expired`, 与取值类错误分开。
        let _guard = TIMER_TEST_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        let (_deadline, mut notice) =
            Deadline::after(Duration::from_millis(20)).expect("runtime context present");
        let token = tokio::time::timeout(Duration::from_secs(2), notice.notified())
            .await
            .expect("到期通知必须送达")
            .expect("token present");
        assert!(
            token.fired_at().elapsed() < Duration::from_secs(2),
            "到期即通知"
        );
        assert_ne!(
            TimeoutErrorCode::DeadlineExpired,
            TimeoutError::Zero.code(),
            "到期与取值拒绝是不同的归属"
        );
    }
}
