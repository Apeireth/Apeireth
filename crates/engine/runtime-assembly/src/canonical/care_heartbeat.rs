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
//! # 生物机制候选装配 (W3.2, 默认关)
//!
//! [`BioCarePoll`] 把仪式 (SG-22) / 重聚 (SG-19) 候选供给接进同一拍心跳:
//!
//! ```text
//! CareHeartbeatPoll::poll
//!   ├─ CareEngine::poll               (规则兜底)
//!   ├─ ReminderFeed 消费              (天气提醒 / 守护告警)
//!   └─ 生物候选 (最后; 不抢生存告警的优先位)
//!        BioCareInputs → bio_care_candidates → CareEngine::poll_bio_candidate
//!        (同一条纪律管道: 勿扰 / 专注 / 频率 / 预算 / 发起闸, 不绕过)
//! ```
//!
//! 开关双门 (default-off, fail-closed):
//! ① 装配门 [`bio_care_assembly_from_env_value`] —— `APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1`
//! 才造出 [`BioCarePoll`] 并给出 [`CareConfig`]; ② 内核门 —— engine 的
//! [`CareConfig::bio_candidates_enabled`] 关时本面**连供给口都不读**, 内核
//! `poll_bio_candidate` 亦直接 Idle (defense-in-depth)。
//!
//! **0 装诚实 (W3.2 生物候选)**: 生产数据供给 (presence 上次可见 / memory
//! replay 可续线索 / 仪式状态存储) **未接** (W3.3), 装配缺省 =
//! [`NoBioCareInputs`] 空供给 ⇒ 开关开也不产候选。开关默认关 = 既有 care 面
//! 输出逐字节不变。
//!
//! **0 装诚实**: 常驻宿主的启动注入点 (presence `try_take_initiative` 同源闸 →
//! [`apeireth_organ::care::CareInitiativeGate`], 发起回调 → `Runtime::execute`)
//! 已在常驻宿主接线 (cli 组合根 `care_heartbeat` 模块, 2026-10-17 随行);
//! 本面仍不自带闸、不自带回合执行权 (组装根注入, 同源不二闸)。

// 生物机制族 W3.3 新增装配面 (调质 → 语气 / 节律门)。
//
// 本批禁改 `canonical.rs` (另有 agent 占用, 注册由 Lead 后续做), 因此两个新文件先挂在
// 本模块下编译, 对外路径 = `canonical::care_heartbeat::{bio_tone, bio_circadian}`。
// **Lead 注册时二选一, 勿重复声明**:
// ① 在 `canonical.rs` 写 `pub use care_heartbeat::{bio_circadian, bio_tone};` (不重复
//    编译, 推荐); 或
// ② 把下面两行 `#[path]` 声明迁到 `canonical.rs` 并删除这里的两行。
#[path = "bio_circadian.rs"]
pub mod bio_circadian;
#[path = "bio_tone.rs"]
pub mod bio_tone;

use std::sync::Arc;
use std::time::Duration;

use apeireth_organ::care::{CareConfig, CareEngine, CareInitiative, CarePoll, ReminderFeed};
use apeireth_runtime::canonical::heartbeat::{
    HeartbeatDispatch, HeartbeatPoll, HeartbeatPollOutcome, HeartbeatProducer,
    HeartbeatProducerConfig,
};

use self::bio_circadian::{BioCircadianDecision, BioCircadianGate};
use super::care_reminder::{
    bio_care_candidates, bio_care_candidates_enabled_from_env_value,
    bio_care_config_from_env_value, BioCareInputs, NoBioCareInputs, BIO_CARE_CANDIDATES_ENABLE_ENV,
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

/// 生物候选时刻时钟 (与心跳状态同一注入时钟; 测试可拨, 生产 = 系统时钟)。
pub type BioCareClock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// 生物机制候选轮次供给面 (W3.2 装配缝): 供给口 + 时钟一次交齐。
///
/// 本面只做「取供给 → 产候选 → 喂内核纪律管道」; 开关 (env) 与内核档同源于
/// [`bio_care_assembly_from_env_value`], 纪律裁决全在
/// [`CareEngine::poll_bio_candidate`]。
pub struct BioCarePoll {
    inputs: Box<dyn BioCareInputs>,
    clock: BioCareClock,
}

impl BioCarePoll {
    /// 绑定供给口 + 时钟 (候选时刻与心跳状态同一时钟源, 不另读系统时钟)。
    #[must_use]
    pub fn new(inputs: Box<dyn BioCareInputs>, clock: BioCareClock) -> Self {
        Self { inputs, clock }
    }

    /// 供给口引用 (每拍各取一次仪式/重聚输入; 不缓存、不跨拍复用)。
    fn inputs(&self) -> &dyn BioCareInputs {
        self.inputs.as_ref()
    }

    /// 当前候选时刻 (注入时钟真值)。
    fn now_ms(&self) -> i64 {
        (self.clock)()
    }
}

impl std::fmt::Debug for BioCarePoll {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BioCarePoll")
            .finish_non_exhaustive()
    }
}

/// 生物候选装配 (W3.2, 默认关): env 开关 → (care 接线配置, 轮次供给面)。
///
/// - `APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1` → `Some((CareConfig{true}, BioCarePoll))`;
///   组装根把 `CareConfig` 注入 engine (`with_care_config`), 并把 [`BioCarePoll`]
///   接进心跳 poll;
/// - 缺省 / 任何非 `"1"` 取值 → `None` (engine 保持默认关, 供给口零读取, 零行为
///   变化)。
///
/// **数据供给边界**: 生产缺省供给 [`NoBioCareInputs`] = 恒空 ⇒ 即使开关打开
/// 也不产候选; 真实数据供给待 W3.3 (presence / memory replay / 仪式状态存储)。
#[must_use]
pub fn bio_care_assembly_from_env_value(
    raw: Option<&str>,
    inputs: Box<dyn BioCareInputs>,
    clock: BioCareClock,
) -> Option<(CareConfig, BioCarePoll)> {
    if !bio_care_candidates_enabled_from_env_value(raw) {
        return None;
    }
    let care_config = bio_care_config_from_env_value(raw);
    Some((care_config, BioCarePoll::new(inputs, clock)))
}

/// 从 env 现值装配 (同 [`bio_care_assembly_from_env_value`] 的 fail-closed 口径)。
#[must_use]
pub fn bio_care_assembly_from_env(
    inputs: Box<dyn BioCareInputs>,
    clock: BioCareClock,
) -> Option<(CareConfig, BioCarePoll)> {
    bio_care_assembly_from_env_value(
        std::env::var(BIO_CARE_CANDIDATES_ENABLE_ENV)
            .ok()
            .as_deref(),
        inputs,
        clock,
    )
}

/// 装配缺省空供给档 (W3.3 前的诚实默认: 开关开也不产候选)。
#[must_use]
pub fn no_bio_care_inputs() -> Box<dyn BioCareInputs> {
    Box::new(NoBioCareInputs)
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

/// `CareEngine::poll` + 提醒侧消费 + 生物候选轮次 → 心跳 poll 面的适配
/// (零新逻辑: 判定/纪律/发起闸全在内核)。
///
/// poll 本身零模型调用 = 缺省判定档 (`RuleFallbackJudge` 纯规则) 的口径;
/// 注入模型判定档时, 模型调用发生在「候选到点」的判定面 (引擎二), 不在轮询。
///
/// 消费顺序 (固定, 防唠叨):
///
/// 0. 节律门 (W3.3, 默认关: 未注入 [`BioCircadianGate`] = 与旧行为一致):
///    开关开且门不许 (`QuietHours` / `RestWindow` / `ConsolidationDue`) → 直接
///    `Quiet`, 规则/提醒/生物候选一拍都不读; 门决策留在 gate 观测字段, 用于区分
///    「未到点」与「被节律门拦下」;
/// 1. 规则轮询 [`CareEngine::poll`];
/// 2. 提醒侧 (K 轴 L2/L5 同源接缝): 逐源消费天气提醒信号 / 守护告警;
/// 3. 生物机制候选 (W3.2, 默认关: 未注入 [`BioCarePoll`] = 与旧行为一致):
///    仪式 → 重聚, 每条喂 [`CareEngine::poll_bio_candidate`] 走同一套纪律。
///
/// 每拍至多发起一个回合 (首个发起即停); 生存告警/提醒永远先于仪式/重聚。
pub struct CareHeartbeatPoll {
    engine: CareEngine,
    reminders: Vec<Arc<dyn ReminderFeed>>,
    bio_care: Option<BioCarePoll>,
    circadian: Option<BioCircadianGate>,
}

impl CareHeartbeatPoll {
    /// 包一层关怀内核 (纯规则轮询档)。
    pub fn new(engine: CareEngine) -> Self {
        Self {
            engine,
            reminders: Vec::new(),
            bio_care: None,
            circadian: None,
        }
    }

    /// 注入一个提醒信号源 (可多次叠加: 天气账 / 守护告警账各一)。
    #[must_use]
    pub fn with_reminder_feed(mut self, feed: Arc<dyn ReminderFeed>) -> Self {
        self.reminders.push(feed);
        self
    }

    /// 注入生物机制候选供给面 (W3.2; 未注入 = 旧行为, 连供给口都不存在)。
    ///
    /// 开关另由 engine 的 [`CareConfig`] 把守: engine 未开 = 本面零供给读取。
    #[must_use]
    pub fn with_bio_care(mut self, bio_care: BioCarePoll) -> Self {
        self.bio_care = Some(bio_care);
        self
    }

    /// 注入节律门 (W3.3; 未注入 = 旧行为)。
    ///
    /// 开关是第二道门: gate 自身 `enabled = false` 时 [`BioCircadianGate::decide`]
    /// 返回 `Disabled` 且不读时钟, poll 继续走原逻辑 (逐字节不变)。
    #[must_use]
    pub fn with_circadian_gate(mut self, gate: BioCircadianGate) -> Self {
        self.circadian = Some(gate);
        self
    }

    /// 最近一次节律门决策 (观测口: 心跳 `Quiet` 时区分「未到点」与「被节律门拦下」)。
    ///
    /// `None` = 未注入节律门或尚未 poll; `Some(Allowed)` + `Quiet` = 门放行但无到点
    /// 候选; `Some(Suppressed(gate))` + `Quiet` = 被节律门拦下 (具体档位可读)。
    #[must_use]
    pub fn circadian_decision(&self) -> Option<BioCircadianDecision> {
        self.circadian
            .as_ref()
            .and_then(BioCircadianGate::last_decision)
    }

    /// 内核本体 (喂交互观察等进程内值的入口)。
    pub fn engine_mut(&mut self) -> &mut CareEngine {
        &mut self.engine
    }
}

/// 一轮生物候选: 开关门 → 取供给 → 产候选 → 逐条走 `poll_bio_candidate`
/// (勿扰 / 专注 / 频率 / 预算 / 发起闸 → 判定 → 台账, 全在内核)。
///
/// - 开关关 (engine 默认) = 直接返回, **不读供给口** (零副作用);
/// - 至多一个发起 (心跳一次只发一回合): 首个 [`CarePoll::Initiating`] 即停;
/// - 拦下 ([`CarePoll::Held`]) / 说不 ([`CarePoll::Declined`]) 继续下一条
///   (Held 候选不丢, 下拍重试; Declined 台账已留痕);
/// - 内核拒开 = `Err(真因)` (失败即帧, 不吞错)。
fn bio_care_tick(
    engine: &mut CareEngine,
    bio_care: &BioCarePoll,
) -> Result<Option<CareInitiative>, String> {
    let care_config = engine.care_config();
    if !care_config.bio_candidates_enabled() {
        return Ok(None);
    }
    let now_ms = bio_care.now_ms();
    for candidate in bio_care_candidates(&care_config, bio_care.inputs(), now_ms) {
        match engine.poll_bio_candidate(&candidate) {
            Ok(CarePoll::Initiating(initiative)) => return Ok(Some(*initiative)),
            Ok(CarePoll::Idle) | Ok(CarePoll::Held { .. }) | Ok(CarePoll::Declined { .. }) => {}
            Err(error) => return Err(format!("{error}")),
        }
    }
    Ok(None)
}

impl HeartbeatPoll for CareHeartbeatPoll {
    fn poll(&mut self) -> Result<HeartbeatPollOutcome, String> {
        // 节律门 (W3.3, 默认关): 最前咨询 —— 开关关 = 不读时钟零变化; 开关开且门
        // 不许 = 直接 Quiet (不新增 outcome 类型), 规则/提醒/生物候选一拍都不读。
        // 「未到点」与「被节律门拦下」都是 Quiet, 区分靠 gate 的 last_decision
        // 观测字段 (`Allowed` vs `Suppressed(_)`, 见 circadian_decision())。
        if let Some(gate) = self.circadian.as_mut() {
            if gate.decide().is_suppressed() {
                return Ok(HeartbeatPollOutcome::Quiet);
            }
        }
        match self.engine.poll() {
            Ok(CarePoll::Initiating(initiative)) => {
                return Ok(HeartbeatPollOutcome::Initiate {
                    turn_input: initiative.to_turn_input(),
                    at_ms: initiative.at_ms,
                });
            }
            Ok(CarePoll::Idle) | Ok(CarePoll::Held { .. }) | Ok(CarePoll::Declined { .. }) => {}
            Err(error) => return Err(format!("{error}")),
        }
        // 提醒侧消费 (同源纪律): 规则面无发起才轮到信号; 拦下不标消费 (候选
        // 不丢, 下拍重试); 至多一个发起 (心跳一次只发一回合)。
        for index in 0..self.reminders.len() {
            let feed = Arc::clone(&self.reminders[index]);
            match apeireth_organ::care::consume_reminders(&mut self.engine, feed.as_ref()) {
                Ok(Some(initiative)) => {
                    return Ok(HeartbeatPollOutcome::Initiate {
                        turn_input: initiative.to_turn_input(),
                        at_ms: initiative.at_ms,
                    });
                }
                Ok(None) => {}
                Err(error) => return Err(format!("{error}")),
            }
        }
        // 生物机制候选 (W3.2, 默认关): 最后一位 —— 仪式/重聚不抢生存告警与
        // 提醒的优先位。开关关 = 零供给读取零候选 (既有行为逐字节不变)。
        if let Some(bio_care) = self.bio_care.as_ref() {
            if let Some(initiative) = bio_care_tick(&mut self.engine, bio_care)? {
                return Ok(HeartbeatPollOutcome::Initiate {
                    turn_input: initiative.to_turn_input(),
                    at_ms: initiative.at_ms,
                });
            }
        }
        Ok(HeartbeatPollOutcome::Quiet)
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
        Self::start_care_with_feeds(config, engine, now_ms, dispatch, Vec::new())
    }

    /// 关怀内核档 + **提醒信号源** (K 轴 L2 提醒 / L5 守护告警消费端): 规则
    /// 轮询后逐源消费提醒/告警信号 (同源纪律, 守护例外档在内核)。
    ///
    /// 信号源从两本信号账取数 (`weather-signals.jsonl` / `guard-alerts.jsonl`
    /// 的消费桥, 见 [`super::care_reminder`]); 传空 `feeds` = 纯规则档
    /// ([`Self::start_care`] 旧行为)。消费随心跳开关走 (注册即闸), 账空 = 零
    /// 消费零动作。
    pub fn start_care_with_feeds<C, D>(
        config: &CareHeartbeatConfig,
        engine: CareEngine,
        now_ms: C,
        dispatch: D,
        feeds: Vec<Arc<dyn ReminderFeed>>,
    ) -> Option<Self>
    where
        C: Fn() -> i64 + Send + Sync + 'static,
        D: HeartbeatDispatch + 'static,
    {
        Self::start(config, now_ms, Self::care_poll(engine, feeds), dispatch)
    }

    /// 关怀内核 + 提醒信号源 + **生物机制候选** (W3.2 装配点): 每拍在规则/提醒
    /// 之后取供给 → 产仪式/重聚候选 → 喂 [`CareEngine::poll_bio_candidate`]
    /// (同一条纪律管道, 不绕过勿扰/频率/预算/发起闸)。
    ///
    /// 开关双门: [`BioCarePoll`] 由 [`bio_care_assembly_from_env_value`] 造出
    /// (env 默认关), engine 的 [`CareConfig`] 亦须打开 —— 任一关 = 零候选。
    /// `bio_care` 的供给口可以用 [`NoBioCareInputs`] 空供给 (生产缺省, 数据源
    /// 未接 ⇒ 零候选) 或宿主注入的真实供给 (W3.3: presence / memory replay)。
    pub fn start_care_with_feeds_and_bio<C, D>(
        config: &CareHeartbeatConfig,
        engine: CareEngine,
        now_ms: C,
        dispatch: D,
        feeds: Vec<Arc<dyn ReminderFeed>>,
        bio_care: BioCarePoll,
    ) -> Option<Self>
    where
        C: Fn() -> i64 + Send + Sync + 'static,
        D: HeartbeatDispatch + 'static,
    {
        let poll = Self::care_poll(engine, feeds).with_bio_care(bio_care);
        Self::start(config, now_ms, poll, dispatch)
    }

    /// 纯规则/提醒 poll 面 (两个 start 变体共用的组装步)。
    fn care_poll(engine: CareEngine, feeds: Vec<Arc<dyn ReminderFeed>>) -> CareHeartbeatPoll {
        let mut poll = CareHeartbeatPoll::new(engine);
        for feed in feeds {
            poll = poll.with_reminder_feed(feed);
        }
        poll
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

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use apeireth_organ::attachment::ResumeThread;
    use apeireth_organ::care::{
        CareClock, CareDiscipline, CareHold, CareRecord, CareStore, FrozenCareClock,
        RuleFallbackJudge,
    };
    use apeireth_organ::ritual::{Cadence, RitualKind, RitualSchedule, RitualState, Window};

    use super::super::care_reminder::{
        bio_care_candidates, BioCareInputs, ReunionInputs, RitualInputs,
    };
    use super::*;

    /// 固定注入时刻 (2025-10-09T08:53:20Z, 默认勿扰窗外); 测试永不读系统时钟。
    const NOW: i64 = 1_760_000_000_000;
    const NOW_U64: u64 = 1_760_000_000_000;
    const HOUR_MS: u64 = 3_600_000;
    const DAY_MS: u64 = 24 * HOUR_MS;

    fn open_discipline() -> CareDiscipline {
        CareDiscipline {
            quiet_start_minutes: None,
            quiet_end_minutes: None,
            ..CareDiscipline::default()
        }
    }

    fn quiet_discipline() -> CareDiscipline {
        // 0..1439 分钟 = 除 23:59 外的整日静音 (引擎现在时刻 = NOW)。
        CareDiscipline {
            quiet_start_minutes: Some(0),
            quiet_end_minutes: Some(1439),
            ..CareDiscipline::default()
        }
    }

    fn engine_with(dir: &Path, bio_enabled: bool, discipline: CareDiscipline) -> CareEngine {
        let store = CareStore::new(dir);
        let mut body = store.list().expect("空表体");
        body.discipline = discipline;
        store.save(&body).expect("落纪律");
        CareEngine::new(
            CareStore::new(dir),
            Arc::new(FrozenCareClock::new(NOW)),
            Arc::new(RuleFallbackJudge),
        )
        .with_care_config(CareConfig {
            bio_candidates_enabled: bio_enabled,
        })
    }

    fn due_ritual_inputs() -> RitualInputs {
        RitualInputs {
            states: vec![RitualState::fresh(RitualKind::MorningCheckIn)],
            config: apeireth_organ::ritual::RitualConfig::new(vec![RitualSchedule::new(
                RitualKind::MorningCheckIn,
                Cadence::Daily,
                Window::Flexible,
                6 * HOUR_MS,
            )]),
        }
    }

    fn reunion_inputs() -> ReunionInputs {
        ReunionInputs {
            last_seen_ms: NOW_U64 - 3 * DAY_MS,
            threads: vec![ResumeThread {
                id: "thread-1".to_string(),
                summary: "共同项目".to_string(),
                salience: 0.8,
                emotion_weight: 0.6,
                last_touched_ms: NOW_U64 - HOUR_MS,
            }],
            policy: apeireth_organ::attachment::ReunionPolicy::default(),
        }
    }

    /// 记录供给读取次数的 fake 供给 (证明开关关时零读取)。
    struct FakeInputs {
        ritual: RitualInputs,
        reunion: Option<ReunionInputs>,
        reads: Arc<AtomicUsize>,
    }

    impl FakeInputs {
        fn ritual_only(reads: &Arc<AtomicUsize>) -> Self {
            Self {
                ritual: due_ritual_inputs(),
                reunion: None,
                reads: Arc::clone(reads),
            }
        }

        fn both(reads: &Arc<AtomicUsize>) -> Self {
            Self {
                ritual: due_ritual_inputs(),
                reunion: Some(reunion_inputs()),
                reads: Arc::clone(reads),
            }
        }

        fn reunion_only(reads: &Arc<AtomicUsize>) -> Self {
            Self {
                ritual: RitualInputs::default(),
                reunion: Some(reunion_inputs()),
                reads: Arc::clone(reads),
            }
        }
    }

    impl BioCareInputs for FakeInputs {
        fn ritual_inputs(&self) -> RitualInputs {
            self.reads.fetch_add(1, Ordering::SeqCst);
            self.ritual.clone()
        }

        fn reunion_inputs(&self) -> Option<ReunionInputs> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            self.reunion.clone()
        }
    }

    fn bio_poll(inputs: impl BioCareInputs + 'static, at_ms: i64) -> BioCarePoll {
        let clock: BioCareClock = Arc::new(move || at_ms);
        BioCarePoll::new(Box::new(inputs), clock)
    }

    fn store_history(dir: &Path) -> Vec<CareRecord> {
        CareStore::new(dir).list().expect("表体").history
    }

    #[test]
    fn bio_off_produces_zero_supply_reads_and_keeps_the_old_quiet_outcome() {
        let dir = tempfile::tempdir().expect("tempdir");
        let reads = Arc::new(AtomicUsize::new(0));
        let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), false, open_discipline()))
            .with_bio_care(bio_poll(FakeInputs::both(&reads), NOW));

        assert_eq!(
            poll.poll().expect("poll"),
            HeartbeatPollOutcome::Quiet,
            "开关关 = 既有输出 (Quiet) 不变"
        );
        assert_eq!(reads.load(Ordering::SeqCst), 0, "开关关: 供给口零读取");
        assert!(store_history(dir.path()).is_empty(), "开关关: 零候选零台账");

        // 同一供给 + 内核开关打开 = 两候选按序进管道; 只发起第一条 (仪式)。
        let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), true, open_discipline()))
            .with_bio_care(bio_poll(FakeInputs::both(&reads), NOW));
        let outcome = poll.poll().expect("poll");
        let HeartbeatPollOutcome::Initiate { turn_input, at_ms } = outcome else {
            panic!("开关开 + 有数据 = 发起: {outcome:?}");
        };
        assert_eq!(at_ms, NOW);
        assert!(
            turn_input.contains("[care:ritual.morning_check_in.due]"),
            "仪式候选进标准回合输入: {turn_input}"
        );
        assert!(reads.load(Ordering::SeqCst) >= 1, "开关开: 供给口被读取");
    }

    #[test]
    fn a_ritual_candidate_goes_through_the_care_pipeline() {
        let dir = tempfile::tempdir().expect("tempdir");
        let reads = Arc::new(AtomicUsize::new(0));
        let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), true, open_discipline()))
            .with_bio_care(bio_poll(FakeInputs::ritual_only(&reads), NOW));

        let outcome = poll.poll().expect("poll");
        let HeartbeatPollOutcome::Initiate { turn_input, at_ms } = outcome else {
            panic!("到点仪式必须发起: {outcome:?}");
        };
        assert_eq!(at_ms, NOW);
        assert!(
            turn_input.contains("仪式 晨间问候"),
            "标准回合输入带仪式语义: {turn_input}"
        );
        let history = store_history(dir.path());
        assert_eq!(history.len(), 1, "发起留痕一条: {history:?}");
        assert_eq!(history[0].rule_id, "ritual.morning_check_in.due");
        assert_eq!(history[0].kind, "ritual", "台账 kind = ritual 档");
        assert!(history[0].spoke, "规则兜底 = 该说");
        let body = CareStore::new(dir.path()).list().expect("表体");
        assert!(
            body.fired_at.is_empty(),
            "信号档不碰规则发起锚点: {:?}",
            body.fired_at
        );
    }

    #[test]
    fn a_non_brief_reunion_candidate_goes_through_the_care_pipeline() {
        let dir = tempfile::tempdir().expect("tempdir");
        let reads = Arc::new(AtomicUsize::new(0));
        let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), true, open_discipline()))
            .with_bio_care(bio_poll(FakeInputs::reunion_only(&reads), NOW));

        let outcome = poll.poll().expect("poll");
        let HeartbeatPollOutcome::Initiate { turn_input, .. } = outcome else {
            panic!("三日分离 + 线索 = 发起: {outcome:?}");
        };
        assert!(
            turn_input.contains("[care:reunion.days]"),
            "重聚候选 (非 Brief) 进标准回合输入: {turn_input}"
        );
        assert!(
            turn_input.contains("你回来了"),
            "重聚文案 = 既有安全模板: {turn_input}"
        );
        let history = store_history(dir.path());
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].kind, "reunion");
    }

    #[test]
    fn no_supplied_data_means_no_candidates_even_with_the_switch_on() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), true, open_discipline()))
            .with_bio_care(bio_poll(NoBioCareInputs, NOW));

        assert_eq!(
            poll.poll().expect("poll"),
            HeartbeatPollOutcome::Quiet,
            "空供给 = 零候选 (生产数据源未接的诚实默认)"
        );
        assert!(store_history(dir.path()).is_empty());
    }

    #[test]
    fn the_quiet_window_still_holds_bio_candidates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let reads = Arc::new(AtomicUsize::new(0));
        let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), true, quiet_discipline()))
            .with_bio_care(bio_poll(FakeInputs::both(&reads), NOW));

        assert_eq!(
            poll.poll().expect("poll"),
            HeartbeatPollOutcome::Quiet,
            "勿扰窗口内 = 不发起"
        );
        assert!(
            reads.load(Ordering::SeqCst) >= 1,
            "候选已产 (供给被读) 但被纪律拦下"
        );
        assert!(
            store_history(dir.path()).is_empty(),
            "拦下不写台账, 候选不丢"
        );

        // 直接问内核: 同一候选被勿扰窗口拦下 (纪律在管道里, 不在装配面)。
        let mut engine = engine_with(dir.path(), true, quiet_discipline());
        let candidate = bio_care_candidates(
            &CareConfig {
                bio_candidates_enabled: true,
            },
            &FakeInputs::both(&reads),
            NOW,
        )
        .into_iter()
        .next()
        .expect("仪式候选");
        assert_eq!(
            engine.poll_bio_candidate(&candidate).expect("内核裁决"),
            CarePoll::Held {
                rule_id: candidate.rule_id.clone(),
                hold: CareHold::QuietWindow,
            },
            "勿扰窗口 = Held(QuietWindow), 装配面不绕过纪律"
        );
    }

    #[test]
    fn the_frequency_cap_still_holds_bio_candidates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = CareStore::new(dir.path());
        let mut body = store.list().expect("表体");
        body.discipline = open_discipline();
        body.history.push(CareRecord {
            rule_id: "seed".to_string(),
            at_ms: NOW - i64::try_from(HOUR_MS).expect("fits"),
            judge: "rule_fallback".to_string(),
            spoke: true,
            note: "seed".to_string(),
            kind: "rule".to_string(),
        });
        store.save(&body).expect("落台账");

        let reads = Arc::new(AtomicUsize::new(0));
        let mut poll = CareHeartbeatPoll::new(engine_with(dir.path(), true, open_discipline()))
            .with_bio_care(bio_poll(FakeInputs::ritual_only(&reads), NOW));
        assert_eq!(
            poll.poll().expect("poll"),
            HeartbeatPollOutcome::Quiet,
            "4h≤1 频率已满 = 不发起"
        );
        assert_eq!(
            store_history(dir.path()).len(),
            1,
            "拦下不追加台账 (候选不丢, 下拍重试)"
        );
    }

    #[test]
    fn the_assembly_switch_is_fail_closed_and_gates_the_supply() {
        let inputs = || Box::new(NoBioCareInputs) as Box<dyn BioCareInputs>;
        let clock: BioCareClock = Arc::new(|| NOW);

        for raw in [None, Some("0"), Some("true"), Some("")] {
            assert!(
                bio_care_assembly_from_env_value(raw, inputs(), Arc::clone(&clock)).is_none(),
                "非 \"1\" = 关: {raw:?}"
            );
        }
        let (care_config, _bio) =
            bio_care_assembly_from_env_value(Some("1"), inputs(), clock).expect("开");
        assert!(care_config.bio_candidates_enabled());
        assert_eq!(
            BIO_CARE_CANDIDATES_ENABLE_ENV,
            "APEIRETH_ENABLE_BIO_CARE_CANDIDATES"
        );
    }
}
