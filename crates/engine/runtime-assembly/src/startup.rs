//! 启动路径分段计时 (B-P1 冷启动冲刺): 计时器**注入**的分段记录面。
//!
//! 用途: 组合根启动路径 (`apeireth session` 启动到 ready) 的各段耗时可测、
//! 可断言 —— 会话库打开 / 会话迁移 / 认知库打开 / 认知迁移 / 附属表 schema /
//! 模块构造 / 运行时组装。生产注入系统计时器 ([`SystemStartupTimer`]),
//! 测试注入脚本计时器 ([`ScriptedTimer`]) 后逐段断言精确耗时与先后顺序
//! (分段计时不靠 sleep、不靠真实时钟)。
//!
//! 口径: 每段只测一次「进入→离开」的单调时钟差; 分段互不重叠时
//! [`StartupTimeline::total_ns`] = 各段之和。计时器调用本身纳秒级。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 启动期各段 (顺序 = 生产启动路径的实际推进顺序)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StartupPhase {
    /// 会话库打开: `SqliteSessionStore::open` 的连接池建立段。
    SessionPoolOpen,
    /// 会话库迁移: 存储 schema 迁移执行段。
    SessionMigrations,
    /// 认知库打开: 认知侧连接池建立段。
    CognitivePoolOpen,
    /// 认知库迁移: 记忆 schema 迁移执行段。
    CognitiveMigrations,
    /// 认知库附属表 schema: 各 store 的 `ensure_schema` 段。
    CognitiveStoreSchema,
    /// 模块构造: `ProductionModules::build` 段。
    ModulesBuild,
    /// 运行时组装: `register_into` + `build().await` 到 ready 段。
    RuntimeAssembly,
}

/// 可注入计时器: 单调纳秒读数。生产用 [`SystemStartupTimer`],
/// 测试用 [`ScriptedTimer`] (脚本读数, 断言精确)。
pub trait StartupTimer: Send + Sync {
    /// 当前单调纳秒读数 (只需单调, 不要求墙钟对齐)。
    fn now_ns(&self) -> u64;
}

/// 系统计时器 (生产注入): 首次读数起算的单调时钟。
pub struct SystemStartupTimer {
    origin: std::time::Instant,
}

impl SystemStartupTimer {
    /// 构造系统计时器 (origin = 现在)。
    pub fn new() -> Self {
        Self {
            origin: std::time::Instant::now(),
        }
    }
}

impl Default for SystemStartupTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl StartupTimer for SystemStartupTimer {
    fn now_ns(&self) -> u64 {
        self.origin.elapsed().as_nanos() as u64
    }
}

/// 脚本计时器 (测试注入): 游标由测试显式推进, 读数完全确定。
///
/// 测试在被测闭包内 `advance_by` 推进游标, 即可对每段耗时做**精确**断言
/// (不依赖真实时钟、不 sleep)。
pub struct ScriptedTimer {
    cursor: AtomicU64,
}

impl ScriptedTimer {
    /// 构造脚本计时器 (初始读数 0)。
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            cursor: AtomicU64::new(0),
        })
    }

    /// 游标前进 (纳秒)。
    pub fn advance_by(&self, ns: u64) {
        self.cursor.fetch_add(ns, Ordering::SeqCst);
    }

    /// 游标置值 (纳秒)。
    pub fn set_ns(&self, ns: u64) {
        self.cursor.store(ns, Ordering::SeqCst);
    }
}

impl Default for ScriptedTimer {
    fn default() -> Self {
        Self {
            cursor: AtomicU64::new(0),
        }
    }
}

impl StartupTimer for ScriptedTimer {
    fn now_ns(&self) -> u64 {
        self.cursor.load(Ordering::SeqCst)
    }
}

/// 一段启动耗时记录。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartupSpan {
    /// 被测段。
    pub phase: StartupPhase,
    /// 该段耗时 (纳秒)。
    pub duration_ns: u64,
}

/// 分段计时记录面: 注入计时器, 逐段落账。
pub struct StartupTimeline {
    timer: Arc<dyn StartupTimer>,
    spans: Mutex<Vec<StartupSpan>>,
}

impl StartupTimeline {
    /// 用注入计时器构造记录面。
    pub fn new(timer: Arc<dyn StartupTimer>) -> Arc<Self> {
        Arc::new(Self {
            timer,
            spans: Mutex::new(Vec::new()),
        })
    }

    /// 系统计时器记录面 (生产用)。
    pub fn system() -> Arc<Self> {
        Self::new(Arc::new(SystemStartupTimer::new()))
    }

    /// 同步段计时: 进入/离开各读一次计时器, 落一段账。
    pub fn measure<T>(&self, phase: StartupPhase, run: impl FnOnce() -> T) -> T {
        let start = self.timer.now_ns();
        let result = run();
        self.record_since(phase, start);
        result
    }

    /// 异步段计时 (await 前后各读一次计时器)。
    pub async fn measure_async<T>(
        &self,
        phase: StartupPhase,
        run: impl std::future::Future<Output = T>,
    ) -> T {
        let start = self.timer.now_ns();
        let result = run.await;
        self.record_since(phase, start);
        result
    }

    fn record_since(&self, phase: StartupPhase, start_ns: u64) {
        let end_ns = self.timer.now_ns();
        let duration_ns = end_ns.saturating_sub(start_ns);
        self.spans
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(StartupSpan { phase, duration_ns });
    }

    /// 全部段记录 (按落账顺序)。
    pub fn spans(&self) -> Vec<StartupSpan> {
        self.spans
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 某段的累计耗时 (同段多次落账时求和; 未出现 = 0)。
    pub fn duration_ns(&self, phase: StartupPhase) -> u64 {
        self.spans()
            .into_iter()
            .filter(|span| span.phase == phase)
            .map(|span| span.duration_ns)
            .sum()
    }

    /// 各段耗时之和 (纳秒)。
    pub fn total_ns(&self) -> u64 {
        self.spans().into_iter().map(|span| span.duration_ns).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injected_scripted_timer_makes_each_span_duration_exactly_assertable() {
        let timer = ScriptedTimer::new();
        let timeline = StartupTimeline::new(timer.clone());

        timeline.measure(StartupPhase::SessionPoolOpen, || {
            timer.advance_by(100);
        });
        timeline.measure(StartupPhase::SessionMigrations, || {
            timer.advance_by(7);
        });

        let spans = timeline.spans();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].phase, StartupPhase::SessionPoolOpen);
        assert_eq!(spans[0].duration_ns, 100, "脚本计时器下逐段耗时精确可断言");
        assert_eq!(spans[1].phase, StartupPhase::SessionMigrations);
        assert_eq!(spans[1].duration_ns, 7);
        assert_eq!(timeline.total_ns(), 107);
        assert_eq!(timeline.duration_ns(StartupPhase::SessionPoolOpen), 100);
        assert_eq!(timeline.duration_ns(StartupPhase::RuntimeAssembly), 0);
    }

    #[test]
    fn async_measure_records_spans_around_await() {
        let timer = ScriptedTimer::new();
        let timeline = StartupTimeline::new(timer.clone());

        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        rt.block_on(
            timeline.measure_async(StartupPhase::RuntimeAssembly, async {
                timer.advance_by(42);
            }),
        );

        let spans = timeline.spans();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].phase, StartupPhase::RuntimeAssembly);
        assert_eq!(spans[0].duration_ns, 42);
    }

    #[test]
    fn same_phase_can_be_recorded_more_than_once() {
        let timer = ScriptedTimer::new();
        let timeline = StartupTimeline::new(timer.clone());
        timeline.measure(StartupPhase::ModulesBuild, || timer.advance_by(3));
        timeline.measure(StartupPhase::ModulesBuild, || timer.advance_by(4));
        assert_eq!(timeline.duration_ns(StartupPhase::ModulesBuild), 7);
        assert_eq!(timeline.spans().len(), 2);
    }

    #[test]
    fn system_timer_produces_non_decreasing_readings() {
        let timer = SystemStartupTimer::new();
        let first = timer.now_ns();
        let second = timer.now_ns();
        assert!(second >= first);
    }
}
