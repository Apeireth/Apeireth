//! 常驻心跳关怀生产者 (V3-a 收尾一针) 的装配面: 让「它会想起你」真的自动发生。
//!
//! 内核 (V3-a 已备) 是 [`CareEngine::poll`]: 规则触发 + 打扰纪律四件 + 发起闸
//! (try_take_initiative 同源) + 模型情境判定面。本面把它接到运行时
//! `canonical::heartbeat` 调度缝的常驻生产者上:
//!
//! ```text
//! 常驻心跳线程 (间隔可配, 缺省 5 分钟)
//!   │  HeartbeatScheduler 调度缝 (Timer 任务; 心流锁可拦闲散 poll)
//!   ▼
//! CareEngine::poll            ← 纯规则轮询 (缺省判定档零模型调用)
//!   ├─ Idle / Held / Declined → Quiet: 零动作零计费
//!   ├─ Err(拒开真因)          → 失败即帧式留痕, 下拍照常
//!   └─ Initiating             → 标准回合输入 (to_turn_input)
//!        ▼
//! HeartbeatDispatch::dispatch → 标准回合 (真 Runtime) → 关怀消息进会话流
//!                               + 计费落会话配额账本 (照常计费, 无免费通道)
//! ```
//!
//! **开关** ([`CareHeartbeatConfig`]): `APEIRETH_ENABLE_CARE_HEARTBEAT` 注册即闸,
//! 默认关 (fail-closed), 且依赖 `APEIRETH_ENABLE_CARE` (上游关怀总闸) —— 两闸
//! 同时为 `1` 才起线程。poll 间隔 `APEIRETH_CARE_HEARTBEAT_POLL_INTERVAL_SECS`
//! 可配 (缺省 300 秒 = 5 分钟; 非法即拒, 不猜)。
//!
//! **0 装诚实**: 常驻宿主的启动注入点 (presence `try_take_initiative` 同源闸 →
//! [`apeireth_organ::care::CareInitiativeGate`], 发起回调 → `Runtime::execute`)
//! 已在常驻宿主接线 (cli 组合根 `care_heartbeat` 模块, 2026-10-17 随行);
//! 本面仍不自带闸、不自带回合执行权 (组装根注入, 同源不二闸)。

use std::time::Duration;

use apeireth_organ::care::{CareEngine, CarePoll};
use apeireth_runtime::canonical::heartbeat::{
    HeartbeatDispatch, HeartbeatPoll, HeartbeatPollOutcome, HeartbeatProducer,
    HeartbeatProducerConfig,
};

/// 关怀总闸 (`APEIRETH_ENABLE_CARE`): 心跳生产者的上游依赖 (依赖即闸)。
pub const CARE_ENABLE_ENV: &str = "APEIRETH_ENABLE_CARE";

/// 心跳关怀总闸 (`APEIRETH_ENABLE_CARE_HEARTBEAT`): 注册即闸, 默认关
/// (fail-closed) —— 与 [`CARE_ENABLE_ENV`] 同时为 `1` 才起常驻心跳线程。
pub const CARE_HEARTBEAT_ENABLE_ENV: &str = "APEIRETH_ENABLE_CARE_HEARTBEAT";

/// poll 间隔旋钮 (`APEIRETH_CARE_HEARTBEAT_POLL_INTERVAL_SECS`): 正整数秒,
/// 缺省 [`DEFAULT_CARE_HEARTBEAT_POLL_INTERVAL_SECS`]。非法即拒 (不猜)。
pub const CARE_HEARTBEAT_POLL_INTERVAL_ENV: &str = "APEIRETH_CARE_HEARTBEAT_POLL_INTERVAL_SECS";

/// poll 间隔缺省秒数 = 300 (5 分钟)。
pub const DEFAULT_CARE_HEARTBEAT_POLL_INTERVAL_SECS: u64 = 300;

/// 心跳关怀开关 + 节拍配置 (注册即闸的生效值)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareHeartbeatConfig {
    /// 起不起常驻心跳线程: 默认关 (fail-closed), 双闸齐开才为 `true`。
    pub enabled: bool,
    /// poll 间隔 (缺省 5 分钟, 可配)。
    pub poll_interval: Duration,
}

impl Default for CareHeartbeatConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            poll_interval: Duration::from_secs(DEFAULT_CARE_HEARTBEAT_POLL_INTERVAL_SECS),
        }
    }
}

impl CareHeartbeatConfig {
    /// 从 env 原始值解析 (纯函数, 测试可注入)。
    ///
    /// fail-closed: `APEIRETH_ENABLE_CARE=1` **且**
    /// `APEIRETH_ENABLE_CARE_HEARTBEAT=1` 才 `enabled = true`; 其余任何取值
    /// (含缺省/垃圾值) = 关。poll 间隔非法 (非正整数秒) = 拒绝即帧 (`Err`)。
    pub fn from_env_value(
        care: Option<&str>,
        heartbeat: Option<&str>,
        poll_interval_secs: Option<&str>,
    ) -> Result<Self, String> {
        let enabled = care == Some("1") && heartbeat == Some("1");
        let poll_interval = match poll_interval_secs {
            None => Duration::from_secs(DEFAULT_CARE_HEARTBEAT_POLL_INTERVAL_SECS),
            Some(raw) => {
                let secs: u64 = raw.trim().parse().map_err(|_| {
                    format!(
                        "拒绝配置: {CARE_HEARTBEAT_POLL_INTERVAL_ENV} 必须是正整数秒, got {raw:?}"
                    )
                })?;
                if secs == 0 {
                    return Err(format!(
                        "拒绝配置: {CARE_HEARTBEAT_POLL_INTERVAL_ENV} 必须 ≥1 秒, got 0"
                    ));
                }
                Duration::from_secs(secs)
            }
        };
        Ok(Self {
            enabled,
            poll_interval,
        })
    }

    /// 从 env 现值解析 (同 [`Self::from_env_value`] 的 fail-closed 口径)。
    pub fn from_env() -> Result<Self, String> {
        Self::from_env_value(
            std::env::var(CARE_ENABLE_ENV).ok().as_deref(),
            std::env::var(CARE_HEARTBEAT_ENABLE_ENV).ok().as_deref(),
            std::env::var(CARE_HEARTBEAT_POLL_INTERVAL_ENV)
                .ok()
                .as_deref(),
        )
    }
}

/// 心跳态真值 (自述关怀节心跳投影的中间形态: 下次 poll 时刻 / 最近心跳命中)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CareHeartbeatStatus {
    /// 下次 poll 时刻 (epoch 毫秒); `None` = 心跳生产者未运行/未接线 (如实)。
    pub next_poll_at_ms: Option<i64>,
    /// 最近心跳命中 (关怀发起) 时刻; `None` = 尚无心跳命中。
    pub last_hit_at_ms: Option<i64>,
    /// poll 累计 (含零动作与失败; 自述不投影, 诊断/测试真值)。
    pub polls: u64,
    /// 命中发起累计 (自述不投影, 诊断/测试真值)。
    pub hits: u64,
}

/// `CareEngine::poll` → 心跳 poll 面的适配 (零新逻辑: 判定/纪律/发起闸全在内核)。
///
/// poll 本身零模型调用 = 缺省判定档 (`RuleFallbackJudge` 纯规则) 的口径;
/// 注入模型判定档时, 模型调用发生在「候选到点」的判定面 (引擎二), 不在轮询。
pub struct CareHeartbeatPoll {
    engine: CareEngine,
}

impl CareHeartbeatPoll {
    /// 包一层关怀内核。
    pub fn new(engine: CareEngine) -> Self {
        Self { engine }
    }

    /// 内核本体 (喂交互观察等进程内值的入口)。
    pub fn engine_mut(&mut self) -> &mut CareEngine {
        &mut self.engine
    }
}

impl HeartbeatPoll for CareHeartbeatPoll {
    fn poll(&mut self) -> Result<HeartbeatPollOutcome, String> {
        match self.engine.poll() {
            Ok(CarePoll::Initiating(initiative)) => Ok(HeartbeatPollOutcome::Initiate {
                turn_input: initiative.to_turn_input(),
                at_ms: initiative.at_ms,
            }),
            Ok(CarePoll::Idle) | Ok(CarePoll::Held { .. }) | Ok(CarePoll::Declined { .. }) => {
                Ok(HeartbeatPollOutcome::Quiet)
            }
            Err(error) => Err(format!("{error}")),
        }
    }
}

/// 心跳关怀生产者装配句柄 (注册即闸 + 优雅停机收线程)。
pub struct CareHeartbeatProducer {
    producer: HeartbeatProducer,
}

impl CareHeartbeatProducer {
    /// 注册即闸 (fail-closed): `config.enabled = false` = 不建线程不 poll,
    /// 返回 `None` (自述心跳两列显式 null, 不冒充在跑)。
    ///
    /// `poll` = 纯规则轮询面 ([`CareHeartbeatPoll`]); `dispatch` = 标准回合
    /// 执行缝 (组装根接 `Runtime::execute`, 关怀回合照常计费)。
    pub fn start<C, P, D>(
        config: &CareHeartbeatConfig,
        now_ms: C,
        poll: P,
        dispatch: D,
    ) -> Option<Self>
    where
        C: Fn() -> i64 + Send + Sync + 'static,
        P: HeartbeatPoll + 'static,
        D: HeartbeatDispatch + 'static,
    {
        if !config.enabled {
            return None;
        }
        Some(Self {
            producer: HeartbeatProducer::spawn(
                HeartbeatProducerConfig::default().with_poll_interval(config.poll_interval),
                now_ms,
                poll,
                dispatch,
            ),
        })
    }

    /// 关怀内核档: 直接喂 [`CareEngine`] 起 (poll 适配在本装配面)。
    pub fn start_care<C, D>(
        config: &CareHeartbeatConfig,
        engine: CareEngine,
        now_ms: C,
        dispatch: D,
    ) -> Option<Self>
    where
        C: Fn() -> i64 + Send + Sync + 'static,
        D: HeartbeatDispatch + 'static,
    {
        Self::start(config, now_ms, CareHeartbeatPoll::new(engine), dispatch)
    }

    /// 心跳态真值 (自述关怀节心跳投影的来源)。
    pub fn status(&self) -> CareHeartbeatStatus {
        let status = self.producer.status();
        CareHeartbeatStatus {
            next_poll_at_ms: status.next_poll_at_ms,
            last_hit_at_ms: status.last_hit_at_ms,
            polls: status.polls,
            hits: status.hits,
        }
    }

    /// 心跳线程是否仍在运行。
    pub fn is_running(&self) -> bool {
        self.producer.is_running()
    }

    /// 线程句柄是否已回收 (停机清理的无泄漏断言口)。
    pub fn thread_reaped(&self) -> bool {
        self.producer.thread_reaped()
    }

    /// 优雅停机: 唤醒并 join 心跳线程 (不僵尸); 幂等。
    pub fn shutdown(&mut self) -> bool {
        self.producer.shutdown()
    }
}
