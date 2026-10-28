//! 心跳关怀常驻宿主接线面 (care-heartbeat-producer.md §7 接线清单 (b))。
//!
//! 常驻宿主 (gateway serve 组合根) 在这里把三个注入点一次接齐:
//!
//! ① **发起闸**: presence 发起闸的装配级读取口 (`PresenceService::
//!    try_take_initiative`) 经 [`FnInitiativeGate`] 桥进 [`CareEngine`] ——
//!    与其他 initiative 生产者同一份防唠叨预算 (同源闸, 不二本账);
//! ② **发起回调**: `dispatch` 接 [`Runtime::execute`] 标准回合 (零新消息通道,
//!    生成的关怀消息进会话流普通转写, 关怀回合照常计费);
//! ③ **自述关怀节**: [`CareHeartbeatSource::status_probe`] 注进
//!    `care_status_probe_with_heartbeat` —— 心跳两列从显式 null 转真值;
//! ④ **提醒信号源** (K 轴 L2/L5 消费端): 天气提醒信号 (`weather-signals.jsonl`)
//!    与守护告警 (`guard-alerts.jsonl`) 的消费桥随心跳一起起 —— 提醒 = 关怀
//!    候选的一种, 走同一套打扰纪律 (守护例外档在内核), 零新消息通道。
//! ⑤ **生物候选供给** (W3.3, 默认关): `APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1`
//!    时才构造 [`CareBioSupply`] 并把心跳接到 `start_care_with_feeds_and_bio`:
//!    presence 回合帧 → 上次可见时刻; 会话账本 → 可续线索; **仪式状态存储**
//!    (`ritual_state.json`, W3.5) → 仪式状态 + 显式排程 (缺省空排程 ⇒ 0 候选)。
//!    开关关 = 不构造供给、不订阅 presence、不读会话、不读写仪式存储, 走与接线前
//!    同一 `start_care_with_feeds` 调用 (逐字节一致)。
//! ⑥ **节律门** (W3.4, 默认关): `APEIRETH_ENABLE_BIO_CIRCADIAN=1` (只认 `"1"`)
//!    时才构造 [`BioCircadianGate`] 并接到
//!    `start_care_with_feeds_and_circadian` /
//!    `start_care_with_feeds_bio_and_circadian`: 心跳最前咨询门, 门不许 = 整拍
//!    `Quiet` (规则 / 提醒 / 生物候选零读取)。**时区显式配置**:
//!    `APEIRETH_BIO_CIRCADIAN_TZ_OFFSET_MIN` (分钟) 缺省 `0` = **UTC (不是本地
//!    时区推断)**; 其余 `CircadianConfig` 字段取内核 `Default` 工程缺省
//!    (07:00 起 / 23:00 睡 / 25 分钟专注 / 5 分钟休息)。坏时区 = 拒绝即帧
//!    (`Err`, 不猜)。开关关 = 不构造门、不读时钟 (连时区 env 都不读), 走与
//!    接线前同一生产者调用 (逐字节一致)。
//!
//! **双闸齐开才起** ([`CareHeartbeatConfig`], 注册即闸 fail-closed):
//! `APEIRETH_ENABLE_CARE=1` 且 `APEIRETH_ENABLE_CARE_HEARTBEAT=1` 才建心跳线程;
//! 关 = 不建线程不 poll 零动作 (自述两列显式 null, 不冒充在跑)。
//!
//! # 生物候选数据供给口径 (W3.3, 0 装诚实)
//!
//! - **上次可见** = presence **回合帧** (`PresenceSignificance::Turn`, 回合完成/
//!   失败) 的 `at`; presence 没有独立的「上次活动」读取口, 而心跳帧每 60s 刷新
//!   `at` —— 心跳帧**不**计入上次可见 (不拿心跳冒充可见), 只做单调取最大
//!   (时钟回拨不回退)。启动后尚无回合帧 = `None` (不编时间)。
//! - **可续线索** = 会话账本的最近会话 (`SessionStore::list`, 与面板同一只读口):
//!   摘要取该会话 `provider_view()` 里**最近一条用户消息**原文 (已过压缩/掩码
//!   折叠, 掩码内容不外泄), 截前 120 字符; 上次触碰 = `updated_at`。权重是
//!   **排序占位** (salience 0.5 / emotion_weight 0.0 —— 无情绪估计来源, 不冒充);
//!   读取失败 / 无用户消息 / 空账本 = 无线索 (不编摘要)。口径边界: 该口是全量
//!   `list` (无轻量 recent 口), 每拍一次, 成本随会话账本增长 (运维侧如实登记;
//!   后续可补按域/按时间窗的轻量读取口, 不在本批改其它 crate)。
//! - **仪式状态** (W3.5 已接存储): `<care_dir>/ritual_state.json` 由
//!   [`RitualStateStore`] 读写 —— StoredDoc 信封 (拒开语义) + 持久档原子写, 与
//!   `care_rules.json` 同一文件账范式。`body.config` = 显式排程; 缺失回落 organ
//!   `RitualConfig::default()` 空排程 ⇒ **默认 0 候选 (不主动)**, 显式配置排程才
//!   有到点候选。坏文件 = fail-closed: 不产候选 + [`CareHeartbeatSource::ritual_state_error`]
//!   留真因, **不静默回退空表、不覆盖损坏文件**。**完成事件源: 待后续从 care
//!   交互 / 工具面接** —— 本批提供显式注入口
//!   [`CareHeartbeatSource::record_ritual_done`] / `record_ritual_skip` (cli 装配
//!   处绑定到 start 的成功路径); 未接事件源前候选不会被自动消账 (同一到点状态
//!   仍由 care 频率/预算/发起闸限流)。
//! - 开关默认关; 关 = 零订阅、零会话读取、零仪式存储读写、零候选。
//!   **不称 DEFAULT ENABLED。** 四级口径: 仪式存储 = **IMPLEMENTED + PRODUCTION
//!   WIRED (装配默认关)** —— 不是 DEFAULT ENABLED, 不是 HARDWARE VALIDATED;
//!   显式排程来源与完成事件源见上 (半程, 如实)。

use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::SessionId;
use apeireth_gateway::{PresenceService, PresenceSignificance, PresenceState};
use apeireth_organ::attachment::ResumeThread;
use apeireth_organ::care::{
    CareClock, CareConfig, CareEngine, CareStore, FnInitiativeGate, RuleFallbackJudge,
    SystemCareClock,
};
use apeireth_organ::circadian::CircadianConfig;
use apeireth_organ::ritual::RitualKind;
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use apeireth_runtime::canonical::heartbeat::HeartbeatDispatch;
use apeireth_runtime::canonical::{Runtime, Session, SessionStore, TurnRequest};
use apeireth_runtime_assembly::canonical::care_heartbeat::bio_circadian::{
    bio_circadian_enabled_from_env_value, bio_circadian_wiring_from_env_value, BioCircadianGate,
    CircadianGateClock, BIO_CIRCADIAN_ENABLE_ENV,
};
use apeireth_runtime_assembly::canonical::care_heartbeat::{
    bio_care_assembly_from_env_value, BioCareClock, BioCarePoll,
};
use apeireth_runtime_assembly::canonical::care_reminder::{
    bio_care_candidates_enabled_from_env_value, BioCareInputs, ReunionInputs, RitualInputs,
    RitualStateStore, BIO_CARE_CANDIDATES_ENABLE_ENV,
};
use apeireth_runtime_assembly::{
    CareHeartbeatConfig, CareHeartbeatProducer, CareHeartbeatStatus, GuardAlertFeed,
    WeatherSignalFeed,
};

/// 发起缝失败留痕的有界容量 (只留最近若干条真因, 不无界长)。
const DISPATCH_ERROR_LOG_MAX: usize = 32;

/// 可续线索扫描上限 (会话账本按最近写入降序; 最多取最近 8 条产线索)。
const RESUME_THREAD_SCAN_MAX: usize = 8;
/// 线索摘要截断长度 (字符数; 只取用户消息原文, 不臆造摘要)。
const RESUME_THREAD_SUMMARY_CHARS_MAX: usize = 120;
/// 线索排序占位显著度 (无可测显著度来源, 不冒充测量值; 排序由新鲜度主导)。
const RESUME_THREAD_SALIENCE: f32 = 0.5;
/// 线索排序占位情绪权重 (无情绪估计来源 ⇒ 0.0, 不冒充)。
const RESUME_THREAD_EMOTION_WEIGHT: f32 = 0.0;

/// 节律门时区偏移 env (分钟): **显式配置**, 缺省 `0` = UTC。
///
/// **不是本地时区推断**: 本面不读系统时区 / 用户设置 / 环境时区; 想接本地
/// 时区必须由宿主人肉配置本 env (例如北京时间 = `480`)。
pub const BIO_CIRCADIAN_TZ_OFFSET_MIN_ENV: &str = "APEIRETH_BIO_CIRCADIAN_TZ_OFFSET_MIN";

/// 节律门时区偏移缺省值 = `0` 分钟 (UTC; 显式口径, 不推断本地时区)。
pub const DEFAULT_BIO_CIRCADIAN_TZ_OFFSET_MIN: i32 = 0;

/// 时区偏移允许范围上限 (分钟 = ±14 小时; 与内核 `CircadianConfig` 夹取口径一致)。
pub const BIO_CIRCADIAN_TZ_OFFSET_MIN_LIMIT: i32 = 14 * 60;

/// presence 「上次可见」探针 (W3.3): 只采回合帧, 心跳帧不计入。
///
/// 只在生物候选开关打开时构造: 订阅 [`PresenceService::subscribe_presence`]
/// (首持有者开流, 与网关保活 pin 同面), 后台挑每帧最新值 —— **只读**, 不回放、
/// 不发布、不改 presence 状态。`at` 单调取最大: 时钟回拨/乱序帧不回退。
struct PresenceMomentProbe {
    last_turn_at_ms: Arc<AtomicI64>,
    watcher: Option<tokio::task::JoinHandle<()>>,
}

impl PresenceMomentProbe {
    /// 订阅 presence 帧面 (失败 = 无时刻来源, 不 panic、不编时间)。
    fn attach(presence: &Arc<PresenceService>, handle: &tokio::runtime::Handle) -> Self {
        let last_turn_at_ms = Arc::new(AtomicI64::new(0));
        // 种子: 订阅前已发布的回合帧 (生产装配早于服务, 通常为空)。
        if let Some(state) = presence.frame_snapshot().value.as_ref() {
            record_turn_frame(&last_turn_at_ms, state);
        }
        let watcher = match presence.subscribe_presence() {
            Ok(subscription) => {
                let mut updates = subscription.updates();
                let last = Arc::clone(&last_turn_at_ms);
                Some(handle.spawn(async move {
                    // 持有租约 = 订阅期间帧面不关; 循环退出即释放。
                    let _lease = subscription;
                    while updates.changed().await.is_ok() {
                        let frame = updates.borrow_and_update().clone();
                        if let Some(state) = frame.value.as_ref() {
                            record_turn_frame(&last, state);
                        }
                    }
                }))
            }
            Err(_) => None,
        };
        Self {
            last_turn_at_ms,
            watcher,
        }
    }

    /// 上次可见时刻 (epoch 毫秒); 尚未见过回合帧 = `None` (不拿心跳帧冒充)。
    fn last_seen_ms(&self) -> Option<u64> {
        u64::try_from(self.last_turn_at_ms.load(Ordering::SeqCst))
            .ok()
            .filter(|at| *at > 0)
    }

    /// 测试口: 直接给一个已知时刻 (无真实 presence 订阅)。
    #[cfg(test)]
    fn for_test(last_turn_at_ms: i64) -> Self {
        Self {
            last_turn_at_ms: Arc::new(AtomicI64::new(last_turn_at_ms)),
            watcher: None,
        }
    }
}

impl Drop for PresenceMomentProbe {
    fn drop(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            watcher.abort();
        }
    }
}

/// 记一帧: 只有回合帧 (`Turn`) 才是「最近可见」; 心跳帧不刷新, 时刻单调不回退。
fn record_turn_frame(last_turn_at_ms: &AtomicI64, state: &PresenceState) {
    if state.significance != PresenceSignificance::Turn {
        return;
    }
    last_turn_at_ms.fetch_max(state.at, Ordering::SeqCst);
}

/// 可续线索来源 (W3.3 供给口): 取不到 = `None` (不编线索)。
trait ResumeClueSource: Send {
    /// 最近可续线索; `None` = 读失败/不可得 (与「读到 0 条」区分)。
    fn resume_threads(&self) -> Option<Vec<ResumeThread>>;
}

/// 会话账本 → 可续线索 (与面板同一 `SessionStore::list` 只读口)。
struct SessionClueSource {
    sessions: Arc<dyn SessionStore>,
    handle: tokio::runtime::Handle,
    /// 关怀回合自身的目标会话: 不算「可续线索」(避免自我重聚)。
    care_session: SessionId,
}

impl ResumeClueSource for SessionClueSource {
    fn resume_threads(&self) -> Option<Vec<ResumeThread>> {
        // 心跳线程是独立 std 线程 (非 tokio worker), 与发起缝同一 block_on 口径。
        let sessions = self.handle.block_on(self.sessions.list()).ok()?;
        Some(resume_threads_from_sessions(&sessions, &self.care_session))
    }
}

/// 最近会话 → 可续线索 (纯函数): 摘要 = 最近一条用户消息原文, 触碰时刻 =
/// `updated_at`; 无用户消息/空摘要/负时间戳的会话跳过 (不编)。
fn resume_threads_from_sessions(
    sessions: &[Session],
    care_session: &SessionId,
) -> Vec<ResumeThread> {
    sessions
        .iter()
        .filter(|session| &session.id != care_session)
        .filter_map(|session| {
            let summary = last_user_message_summary(session)?;
            let last_touched_ms = u64::try_from(session.updated_at.epoch_millis()).ok()?;
            Some(ResumeThread {
                id: session.id.to_string(),
                summary,
                salience: RESUME_THREAD_SALIENCE,
                emotion_weight: RESUME_THREAD_EMOTION_WEIGHT,
                last_touched_ms,
            })
        })
        .take(RESUME_THREAD_SCAN_MAX)
        .collect()
}

/// 会话最近一条用户消息的纯文本摘要 (经 `provider_view` 折叠: 压缩/掩码生效);
/// 截前 [`RESUME_THREAD_SUMMARY_CHARS_MAX`] 字符, 空 = `None`。
fn last_user_message_summary(session: &Session) -> Option<String> {
    let text = session
        .provider_view()
        .iter()
        .rev()
        .filter(|message| message.role == MessageRole::User)
        .map(|message| ContentPart::join_text(&message.content))
        .find(|text| !text.trim().is_empty())?;
    let summary: String = text
        .trim()
        .chars()
        .take(RESUME_THREAD_SUMMARY_CHARS_MAX)
        .collect();
    (!summary.is_empty()).then_some(summary)
}

/// 生物候选真实供给 (W3.3 / W3.5): presence 上次可见 + 会话可续线索 + 仪式状态。
///
/// 只在 `APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1` 时构造; 构造即订阅 presence
/// (开关关 = 连构造都不发生)。仪式状态来自 [`RitualStateStore`]
/// (`<care_dir>/ritual_state.json`, W3.5); 读取失败 = 诊断口留真因 + 本拍
/// 不产仪式候选 (fail-closed, 不静默回退空表)。
struct CareBioSupply {
    presence: PresenceMomentProbe,
    clues: Box<dyn ResumeClueSource>,
    /// 仪式状态存储 (开关开时由装配面构造并绑定, 与显式注入口共享同一实例)。
    ritual: Arc<RitualStateStore>,
    /// 仪式状态读失败真因 (诊断口; 坏文件 fail-closed 留痕)。
    ritual_error: Arc<Mutex<Option<String>>>,
}

impl CareBioSupply {
    /// 记下仪式状态读失败真因 (覆盖旧因 = 最近一次真值; 诊断口读取)。
    fn note_ritual_error(&self, message: String) {
        *self
            .ritual_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(message);
    }

    /// 读成功 = 清掉旧错误 (修复后的文件不再挂旧真因)。
    fn clear_ritual_error(&self) {
        *self
            .ritual_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
}

impl BioCareInputs for CareBioSupply {
    fn ritual_inputs(&self) -> RitualInputs {
        // 仪式状态存储 (W3.5): 读 `<care_dir>/ritual_state.json`; 缺失 = 合法空态,
        // 坏文件 = fail-closed (不产候选 + 留真因, 不编状态、不覆盖损坏文件)。
        match self.ritual.inputs() {
            Ok((config, states)) => {
                self.clear_ritual_error();
                RitualInputs { states, config }
            }
            Err(error) => {
                self.note_ritual_error(format!("{error}"));
                RitualInputs::default()
            }
        }
    }

    fn reunion_inputs(&self) -> Option<ReunionInputs> {
        let last_seen_ms = self.presence.last_seen_ms()?;
        let threads = self.clues.resume_threads()?;
        Some(ReunionInputs {
            last_seen_ms,
            threads,
            policy: apeireth_organ::attachment::ReunionPolicy::default(),
        })
    }
}

/// 宿主装配 (W3.3 / W3.5, 默认关): env 原始值 → (care 接线配置, 生物候选供给面)。
///
/// 非 `"1"` 一律 `None` —— **连供给都不构造** (零 presence 订阅、零会话读取、
/// 零仪式存储读写), 调用方据此走接线前的 `start_care_with_feeds` 原路径。
///
/// `ritual` = 装配面在开关开时构造的仪式状态存储 (`<care_dir>/ritual_state.json`);
/// `ritual_error` = 与供给面共享的读失败真因槽位 (诊断口)。
fn host_bio_care_assembly(
    raw: Option<&str>,
    presence: &Arc<PresenceService>,
    sessions: Arc<dyn SessionStore>,
    handle: &tokio::runtime::Handle,
    care_session: SessionId,
    ritual: Arc<RitualStateStore>,
    ritual_error: Arc<Mutex<Option<String>>>,
) -> Option<(CareConfig, BioCarePoll)> {
    if !bio_care_candidates_enabled_from_env_value(raw) {
        return None;
    }
    let clock: BioCareClock = Arc::new(|| SystemCareClock.now_ms());
    let supply = CareBioSupply {
        presence: PresenceMomentProbe::attach(presence, handle),
        clues: Box::new(SessionClueSource {
            sessions,
            handle: handle.clone(),
            care_session,
        }),
        ritual,
        ritual_error,
    };
    bio_care_assembly_from_env_value(raw, Box::new(supply), clock)
}

/// 节律门显式配置 (W3.4 生产装配): env 原始值 → [`CircadianConfig`]。
///
/// 只从 env 读**时区偏移分钟**; 其余字段取 [`CircadianConfig::default()`] 的
/// 工程缺省 —— **本批所列字段**:
///
/// | 字段 | 取值 | 来源 |
/// | --- | --- | --- |
/// | `tz_offset_minutes` | env 或 `0` (= UTC) | 本函数 (显式, **不推断本地时区**) |
/// | `wake_hour` | `7` | 内核 `Default` |
/// | `sleep_hour` | `23` | 内核 `Default` |
/// | `ultradian_focus_ms` | 25 分钟 | 内核 `Default` |
/// | `ultradian_rest_ms` | 5 分钟 | 内核 `Default` |
///
/// - 缺省 (`None`) = `0` 分钟 = **UTC** (显式口径; 不读系统时区/用户设置);
/// - 非整数 / 超出 `±14h` = `Err` (拒绝即帧, 不猜、不静默夹取)。
pub fn bio_circadian_config_from_env_value(
    raw_tz_offset_min: Option<&str>,
) -> Result<CircadianConfig, String> {
    let tz_offset_minutes = match raw_tz_offset_min {
        None => DEFAULT_BIO_CIRCADIAN_TZ_OFFSET_MIN,
        Some(raw) => {
            let parsed: i32 = raw.trim().parse().map_err(|_| {
                format!(
                    "拒绝配置: {BIO_CIRCADIAN_TZ_OFFSET_MIN_ENV} 必须是整数分钟 (可含负号), got {raw:?}"
                )
            })?;
            if !(-BIO_CIRCADIAN_TZ_OFFSET_MIN_LIMIT..=BIO_CIRCADIAN_TZ_OFFSET_MIN_LIMIT)
                .contains(&parsed)
            {
                return Err(format!(
                    "拒绝配置: {BIO_CIRCADIAN_TZ_OFFSET_MIN_ENV} 必须在 ±{BIO_CIRCADIAN_TZ_OFFSET_MIN_LIMIT} 分钟 (±14h) 内, got {parsed}"
                ));
            }
            parsed
        }
    };
    let base = CircadianConfig::default();
    Ok(CircadianConfig::new(
        base.wake_hour,
        base.sleep_hour,
        tz_offset_minutes,
        base.ultradian_focus_ms,
        base.ultradian_rest_ms,
    ))
}

/// 节律门装配 (W3.4, 默认关): env 原始值 + 显式时钟 + 会话起点 → 门。
///
/// - 开关 [`BIO_CIRCADIAN_ENABLE_ENV`] 非 `"1"` → `Ok(None)`: **不构造门、不读
///   时钟、不解析时区** (调用方据此走原生产者路径, 零行为变化);
/// - 开关开 → 解析时区 (坏值 `Err`, 拒绝即帧) 后经
///   [`bio_circadian_wiring_from_env_value`] 构造门 (懒加载语义: 关 = 不构造)。
///
/// `session_start_ms` = 超日窗口起点 (调用方显式给; 生产口径见
/// [`CareHeartbeatSource::start`]: 装配时刻读一次同一时钟, **不持久化、不跨重启**)。
pub fn host_bio_circadian_assembly(
    enable_raw: Option<&str>,
    tz_offset_raw: Option<&str>,
    clock: CircadianGateClock,
    session_start_ms: i64,
) -> Result<Option<BioCircadianGate>, String> {
    if !bio_circadian_enabled_from_env_value(enable_raw) {
        return Ok(None);
    }
    let config = bio_circadian_config_from_env_value(tz_offset_raw)?;
    Ok(bio_circadian_wiring_from_env_value(
        enable_raw,
        clock,
        config,
        session_start_ms,
    ))
}

/// 心跳关怀生产者的共享句柄 (自述关怀节心跳两列的真值源, 后绑定位)。
///
/// 与发声情绪源同型: 组合根建运行时就把 [`Self::status_probe`] 注进自述关怀节
/// (此时未起 = 两列显式 null, 如实); 常驻宿主装配出 presence 闸与 Runtime 后
/// 调 [`Self::start`] 起生产者 —— 此后自述心跳两列现读生产者真值。
#[derive(Clone, Default)]
pub struct CareHeartbeatSource {
    producer: Arc<Mutex<Option<CareHeartbeatProducer>>>,
    errors: Arc<Mutex<Vec<String>>>,
    /// 仪式状态存储绑定位 (开关开 + `start` 成功 = `Some`; 关/未起 = `None`)。
    ///
    /// 绑定只发生在 `APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1` 的 `start` 成功
    /// 路径: 开关关 = 不构造、不读、不写 (`record_ritual_done` 返回 `Ok(false)`)。
    ritual: Arc<Mutex<Option<Arc<RitualStateStore>>>>,
    /// 仪式状态读失败真因 (与供给面共享; 诊断口, 不静默吞错)。
    ritual_error: Arc<Mutex<Option<String>>>,
}

impl CareHeartbeatSource {
    /// 空绑定位 (未起 = 心跳两列显式 null)。
    pub fn new() -> Self {
        Self::default()
    }

    /// 自述关怀节心跳两列的真值口 (`care_status_probe_with_heartbeat` 的探测口)。
    pub fn status_probe(&self) -> Arc<dyn Fn() -> CareHeartbeatStatus + Send + Sync> {
        let source = self.clone();
        Arc::new(move || source.status())
    }

    /// 心跳态投影 (诊断/测试口, 与自述两列同一真值): 生产者在跑 = 生产者真值;
    /// 未起 = 两列显式 `None`; 停机后「下次 poll」显式 `None` (未运行不冒充在跑),
    /// 「最近心跳命中」保留历史真值 (不抹真账)。
    pub fn status(&self) -> CareHeartbeatStatus {
        let slot = self
            .producer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(producer) = slot.as_ref() else {
            return CareHeartbeatStatus::default();
        };
        let status = producer.status();
        CareHeartbeatStatus {
            next_poll_at_ms: if producer.is_running() {
                status.next_poll_at_ms
            } else {
                None
            },
            ..status
        }
    }

    /// 常驻宿主启动注入点 (care-heartbeat-producer.md §7 接线清单 (b)) ——
    /// 三件一次接齐: 闸经 [`FnInitiativeGate`] 注入 [`CareEngine`] (presence
    /// 同源闸), `dispatch` 接 [`Runtime::execute`] (标准回合照常计费), 心跳
    /// 真值绑进 [`Self::status_probe`] (自述两列 null → 真值)。
    ///
    /// **双闸齐开才起**: `config.enabled = false` = 不建线程不 poll, 返回
    /// `Ok(false)` (零动作零线程, 不冒充); 已起再起 = `Err` (不双开), 停机后
    /// 可再起 (槽已清)。`tokio` 运行时句柄缺席 = 拒绝即帧 (`Err`, 不猜)。
    ///
    /// **生物候选 (W3.3, 默认关)**: `APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1` 时
    /// 才构造 [`CareBioSupply`] 并把 `CareConfig{bio_candidates_enabled:true}`
    /// 注入 engine, 走 `start_care_with_feeds_and_bio`; 其余取值 = 不构造供给
    /// (零 presence 订阅、零会话读取), 走 `start_care_with_feeds` **原路径** ——
    /// 与接线前逐字节一致。**W3.5**: 同一开关下构造并绑定仪式状态存储
    /// (`<care_dir>/ritual_state.json`), 供 [`Self::record_ritual_done`] 写回;
    /// 开关关 = 连存储都不构造 (零读取零写入)。
    ///
    /// **节律门 (W3.4, 默认关)**: `APEIRETH_ENABLE_BIO_CIRCADIAN=1` 时才构造
    /// 节律门并接到 `start_care_with_feeds_and_circadian` /
    /// `start_care_with_feeds_bio_and_circadian`; 时区偏移由
    /// [`BIO_CIRCADIAN_TZ_OFFSET_MIN_ENV`] 显式配置 (缺省 `0` = **UTC, 不推断
    /// 本地时区**; 坏值 `Err` 拒绝即帧); 会话起点 = 本装配点读一次同一时钟
    /// (**不持久化、不跨重启**)。其余取值 = 不构造门、不读时钟, 走原生产者
    /// 路径 (逐字节一致)。
    ///
    /// `session` = 关怀回合的目标会话 (发起 = 标准回合, 生成的关怀消息进该
    /// 会话流的普通转写): 由常驻宿主显式注入 —— 本批最小接线取常驻进程级
    /// 关怀会话, 「进哪条对话」的产品面策略随后续拍板, 缝在此不焊死。
    pub fn start(
        &self,
        config: &CareHeartbeatConfig,
        runtime: Arc<Runtime>,
        presence: &Arc<PresenceService>,
        care_dir: PathBuf,
        session: SessionId,
    ) -> Result<bool, String> {
        if !config.enabled {
            return Ok(false);
        }
        let handle = tokio::runtime::Handle::try_current().map_err(|error| {
            format!("care heartbeat dispatch needs a tokio runtime handle: {error}")
        })?;
        let mut slot = self
            .producer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot.is_some() {
            return Err("care heartbeat producer already started (不双开)".to_string());
        }
        // ① 发起闸同源: presence 装配级读取口 → 闭包闸 → CareEngine (同一份
        //    防唠叨预算; 闸拒绝 = Held 候选不丢, 见关怀内核纪律顺序)。
        let gate = FnInitiativeGate::new({
            let presence = Arc::clone(presence);
            move |now_ms| presence.try_take_initiative(now_ms)
        });
        let engine = CareEngine::new(
            CareStore::new(care_dir.clone()),
            Arc::new(SystemCareClock),
            Arc::new(RuleFallbackJudge),
        )
        .with_initiative_gate(Arc::new(gate));
        // ⑥ 节律门 (W3.4, 默认关) —— 先于 ⑤ 装配: 坏时区在 presence 订阅前
        //    拒绝即帧 (无副作用)。关 = 不构造门、不读时钟 (连时区 env 都不读),
        //    随后走接线前的原生产者调用 (逐字节一致); 开 = 先校验显式时区配置
        //    (坏值拒绝即帧, 不猜), 再以装配时刻读一次同一时钟作超日窗口起点
        //    (**不持久化、不跨重启**; 重启 = 新一轮超日周期; 首帧快照由门构造
        //    再读一次同一时钟)。时区缺省 0 = UTC, 不推断本地时区。
        let circadian: Option<BioCircadianGate> = {
            let enable_raw = std::env::var(BIO_CIRCADIAN_ENABLE_ENV).ok();
            if bio_circadian_enabled_from_env_value(enable_raw.as_deref()) {
                let clock: CircadianGateClock = Arc::new(|| SystemCareClock.now_ms());
                let session_start_ms = clock();
                let tz_raw = std::env::var(BIO_CIRCADIAN_TZ_OFFSET_MIN_ENV).ok();
                host_bio_circadian_assembly(
                    enable_raw.as_deref(),
                    tz_raw.as_deref(),
                    clock,
                    session_start_ms,
                )?
            } else {
                None
            }
        };
        // ⑤ 生物候选真实供给 (W3.3, 默认关): env 非 "1" = 连会话账本句柄都不取,
        //    不构造供给 (零 presence 订阅/零会话读取), 随后走接线前的原生产者
        //    路径 (逐字节一致)。开关开才把 care 配置注入 engine 并接供给面。
        let bio_raw = std::env::var(BIO_CARE_CANDIDATES_ENABLE_ENV).ok();
        // W3.5 仪式状态存储绑定位: 只在「开关开 + 生产者真起」后写入 self。
        let mut ritual_to_bind: Option<Arc<RitualStateStore>> = None;
        let bio = if bio_care_candidates_enabled_from_env_value(bio_raw.as_deref()) {
            // W3.5 仪式状态存储: 只在开关开时构造 (关 = 零读取零写入, 连路径都不碰)。
            let ritual = Arc::new(RitualStateStore::under_data_dir(&care_dir));
            let assembled = host_bio_care_assembly(
                bio_raw.as_deref(),
                presence,
                Arc::clone(runtime.sessions_arc().store()),
                &handle,
                session,
                Arc::clone(&ritual),
                Arc::clone(&self.ritual_error),
            );
            if assembled.is_some() {
                ritual_to_bind = Some(ritual);
            }
            assembled
        } else {
            None
        };
        let engine = match bio.as_ref() {
            Some((care_config, _)) => engine.with_care_config(*care_config),
            None => engine,
        };
        // ② 发起回调 = 标准回合 (Runtime::execute; 零新消息通道, 照常计费)。
        let dispatch = RuntimeTurnDispatch {
            handle,
            runtime,
            session,
            errors: Arc::clone(&self.errors),
        };
        // ③ 注册即闸: enabled 已核 = 起常驻线程, 真值随探针进自述关怀节。
        // ④ 提醒信号源 (K 轴 L2/L5 消费端): 天气提醒 + 守护告警两本账的消费桥
        //    随心跳消费 (同源纪律; 账空 = 零消费零动作)。
        let reminder_feeds: Vec<Arc<dyn apeireth_organ::care::ReminderFeed>> = vec![
            Arc::new(WeatherSignalFeed::under_data_dir(&care_dir)),
            Arc::new(GuardAlertFeed::under_data_dir(&care_dir)),
        ];
        let producer = match (bio, circadian) {
            // 生物候选 + 节律门 (两开关都开): 门最前, 门不许 = 整拍 Quiet。
            (Some((_, bio_poll)), Some(gate)) => {
                CareHeartbeatProducer::start_care_with_feeds_bio_and_circadian(
                    config,
                    engine,
                    || SystemCareClock.now_ms(),
                    dispatch,
                    reminder_feeds,
                    bio_poll,
                    Some(gate),
                )
            }
            // 只有节律门开: 规则/提醒面 + 门 (无生物候选供给)。
            (None, Some(gate)) => CareHeartbeatProducer::start_care_with_feeds_and_circadian(
                config,
                engine,
                || SystemCareClock.now_ms(),
                dispatch,
                reminder_feeds,
                Some(gate),
            ),
            // 门关 (默认) + 生物候选开: 与接线前同一调用, 逐字节一致。
            (Some((_, bio_poll)), None) => CareHeartbeatProducer::start_care_with_feeds_and_bio(
                config,
                engine,
                || SystemCareClock.now_ms(),
                dispatch,
                reminder_feeds,
                bio_poll,
            ),
            // 两开关都关 (默认): 与接线前同一调用, 零候选零行为变化。
            (None, None) => CareHeartbeatProducer::start_care_with_feeds(
                config,
                engine,
                || SystemCareClock.now_ms(),
                dispatch,
                reminder_feeds,
            ),
        };
        let Some(producer) = producer else {
            return Ok(false);
        };
        *slot = Some(producer);
        // 生产者真起才绑定仪式状态存储 (未起 = 注入口保持 Ok(false) 零写入)。
        if let Some(ritual) = ritual_to_bind {
            *self
                .ritual
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(ritual);
        }
        Ok(true)
    }

    /// 心跳线程是否在跑 (未起/停机后 = `false`, 如实)。
    pub fn is_running(&self) -> bool {
        self.producer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(|producer| producer.is_running())
            .unwrap_or(false)
    }

    /// 线程句柄是否已回收 (无泄漏断言口; 未起 = 无线程 = 无泄漏)。
    pub fn thread_reaped(&self) -> bool {
        self.producer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
            .map(|producer| producer.thread_reaped())
            .unwrap_or(true)
    }

    /// 优雅停机: 置停 + join 收心跳线程 (不僵尸); 幂等。返回 `true` = 本次
    /// 真收掉一个线程。
    pub fn shutdown(&self) -> bool {
        self.producer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_mut()
            .map(|producer| producer.shutdown())
            .unwrap_or(false)
    }

    /// 发起缝留痕 (失败即帧式真因, 有界; 诊断/测试口)。
    pub fn dispatch_errors(&self) -> Vec<String> {
        self.errors
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 仪式状态存储是否已绑定 (开关开 + `start` 成功 = `true`; 关/未起 = `false`)。
    ///
    /// 未绑定 = [`Self::record_ritual_done`] / [`Self::record_ritual_skip`] 恒
    /// `Ok(false)` 且零文件操作 (不创建 `ritual_state.json`)。
    pub fn has_ritual_state_store(&self) -> bool {
        self.ritual
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
    }

    /// 显式**仪式完成**注入口 (W3.5; W3.7 复核更新): 生产事件源已存在 ——
    /// `tool.ritual_done` / `tool.ritual_skip` 经 `RitualStateStore` 写
    /// `<care_dir>/ritual_state.json`, 心跳每拍从同路径现读 (文件即通道, 无需总线
    /// 信号); 本口保留为宿主/后续 care 交互的**可选**显式写回点。
    ///
    /// - `kind` = 完成的仪式; `now_ms` = 完成时刻 (Unix 毫秒, 调用方注入;
    ///   存储侧单调: 时钟回拨不回退完成时刻);
    /// - 开关关 / 未 `start` / 供给未绑定 = `Ok(false)`: **零读取零写入**, 不创建
    ///   文件 (默认关行为逐字节不变);
    /// - 已绑定 = 写下 `<care_dir>/ritual_state.json` 并返回 `Ok(true)`; 坏文件 /
    ///   IO 失败 = `Err(真因)` (fail-closed, 不覆盖损坏文件)。
    ///
    /// 仍不做「候选被接受 = 仪式完成」的推定 (候选被交付 ≠ 用户完成了仪式)。
    pub fn record_ritual_done(&self, kind: RitualKind, now_ms: u64) -> Result<bool, String> {
        let store = self
            .ritual
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let Some(store) = store else {
            return Ok(false);
        };
        store
            .record_done(kind, now_ms)
            .map(|_state| true)
            .map_err(|error| error.to_string())
    }

    /// 显式**仪式跳过**注入口 (W3.5): 只给 `skipped_count` +1 (统计字段, 内核
    /// 判定不读它), 不动 `last_done_ms` / 静默 / 连续完成 —— 跳过不是完成, 也
    /// 不产生补课或惩罚。开关关 / 未绑定 = `Ok(false)` 零写入。
    pub fn record_ritual_skip(&self, kind: RitualKind) -> Result<bool, String> {
        let store = self
            .ritual
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let Some(store) = store else {
            return Ok(false);
        };
        store
            .record_skip(kind)
            .map(|_state| true)
            .map_err(|error| error.to_string())
    }

    /// 最近一次仪式状态读失败真因 (`None` = 无错误 / 未读; 坏文件 fail-closed
    /// 的真因在此暴露, 不静默回退空表)。
    pub fn ritual_state_error(&self) -> Option<String> {
        self.ritual_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// 发起缝的标准回合执行 (与常驻宿主同一接法: 心跳线程上同步等回合收束)。
struct RuntimeTurnDispatch {
    handle: tokio::runtime::Handle,
    runtime: Arc<Runtime>,
    session: SessionId,
    errors: Arc<Mutex<Vec<String>>>,
}

/// Submit the producer's ordinary turn through the shared Host and session write gate.
pub async fn dispatch_heartbeat_turn(
    runtime: &Runtime,
    session: SessionId,
    input: &str,
) -> Result<apeireth_runtime::TurnResponse, apeireth_runtime::RuntimeError> {
    let _permit = runtime.sessions().write_permit(session).await;
    runtime.execute(TurnRequest::new(session, input)).await
}

impl HeartbeatDispatch for RuntimeTurnDispatch {
    fn dispatch(&mut self, turn_input: &str, _at_ms: i64) {
        if let Err(error) = self.handle.block_on(dispatch_heartbeat_turn(
            &self.runtime,
            self.session,
            turn_input,
        )) {
            // 失败即帧式留痕: 真因不吞 (回合失败不倒心跳循环, 下拍照常)。
            let mut errors = self
                .errors
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if errors.len() >= DISPATCH_ERROR_LOG_MAX {
                errors.remove(0);
            }
            errors.push(error.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use apeireth_core::kernel::{Timestamp, VirtualClock};
    use apeireth_gateway::{EventBus, PresenceBreath, PresencePad, PresenceSource};
    use apeireth_organ::care::CareTier;
    use apeireth_organ::circadian::{CircadianPhase, ProactiveGate};
    use apeireth_organ::ritual::{
        Cadence, RitualConfig, RitualKind, RitualSchedule, RitualState, Window,
    };
    use apeireth_protocol::canonical::NormalizedMessage;
    use apeireth_runtime::canonical::InMemorySessionStore;
    use apeireth_runtime_assembly::canonical::care_heartbeat::bio_circadian::BioCircadianDecision;
    use apeireth_runtime_assembly::canonical::care_reminder::{
        bio_care_candidates, RitualStateBody,
    };

    use super::*;

    /// 固定注入时刻 (2025-10-09T08:53:20Z); 测试永不读系统时钟。
    const NOW: i64 = 1_760_000_000_000;
    const HOUR_MS: u64 = 3_600_000;
    const DAY_MS: u64 = 24 * HOUR_MS;

    fn enabled() -> CareConfig {
        CareConfig {
            bio_candidates_enabled: true,
        }
    }

    fn millis_u64(value: i64) -> u64 {
        u64::try_from(value).expect("test timestamps are non-negative")
    }

    fn thread(id: &str, last_touched_ms: u64) -> ResumeThread {
        ResumeThread {
            id: id.to_string(),
            summary: format!("线索 {id}"),
            salience: 0.5,
            emotion_weight: 0.0,
            last_touched_ms,
        }
    }

    struct FixedClues(Option<Vec<ResumeThread>>);

    impl ResumeClueSource for FixedClues {
        fn resume_threads(&self) -> Option<Vec<ResumeThread>> {
            self.0.clone()
        }
    }

    /// 空仪式存储: 路径指向一个即删临时目录 —— 文件缺失 = 合法空态, 只读不落盘。
    fn empty_ritual_store() -> Arc<RitualStateStore> {
        let dir = tempfile::tempdir().expect("tempdir");
        Arc::new(RitualStateStore::under_data_dir(dir.path()))
    }

    fn supply_with_ritual(
        ritual: Arc<RitualStateStore>,
        last_turn_at_ms: i64,
        clues: Option<Vec<ResumeThread>>,
    ) -> CareBioSupply {
        CareBioSupply {
            presence: PresenceMomentProbe::for_test(last_turn_at_ms),
            clues: Box::new(FixedClues(clues)),
            ritual,
            ritual_error: Arc::new(Mutex::new(None)),
        }
    }

    fn supply(last_turn_at_ms: i64, clues: Option<Vec<ResumeThread>>) -> CareBioSupply {
        supply_with_ritual(empty_ritual_store(), last_turn_at_ms, clues)
    }

    /// 显式排程: 晨间问候每日到点, 宽限 6 小时 (仅测试注入; 生产缺省不配排程)。
    fn ritual_config() -> RitualConfig {
        RitualConfig::new(vec![RitualSchedule::new(
            RitualKind::MorningCheckIn,
            Cadence::Daily,
            Window::Flexible,
            6 * HOUR_MS,
        )])
    }

    fn frame(at: i64, significance: PresenceSignificance) -> PresenceState {
        PresenceState {
            event_type: "presence_state",
            at,
            pad: PresencePad {
                p: 0.0,
                a: 0.0,
                d: 0.0,
            },
            dominant: "neutral".to_string(),
            intensity: 0.0,
            stance: apeireth_gateway::EmberCognitiveStance::AttentivePresence,
            breath: PresenceBreath {
                period_secs: 6.0,
                amplitude: 0.2,
            },
            significance,
            source: PresenceSource {
                kind: "heuristic_v0",
                confidence: 0.0,
            },
        }
    }

    /// ① 开关解析 fail-closed: 默认关, 只认 `"1"`; 关 = 连供给都不构造,
    /// 仪式状态存储零读取零写入 (不创建 `ritual_state.json`)。
    #[tokio::test]
    async fn host_bio_assembly_is_fail_closed_and_only_accepts_one() {
        let presence = PresenceService::new(EventBus::new(4));
        let sessions: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let dir = tempfile::tempdir().expect("tempdir");
        let ritual = Arc::new(RitualStateStore::under_data_dir(dir.path()));
        let ritual_error = Arc::new(Mutex::new(None));
        for raw in [
            None,
            Some(""),
            Some("0"),
            Some("true"),
            Some("TRUE"),
            Some(" 1"),
        ] {
            assert!(
                host_bio_care_assembly(
                    raw,
                    &presence,
                    Arc::clone(&sessions),
                    &tokio::runtime::Handle::current(),
                    SessionId::new(),
                    Arc::clone(&ritual),
                    Arc::clone(&ritual_error),
                )
                .is_none(),
                "非 \"1\" 一律关 (零构造零订阅零读取): {raw:?}"
            );
            assert!(
                !ritual.path().exists(),
                "默认关 = 零仪式存储读写 (不建文件): {raw:?}"
            );
        }

        let (care_config, _bio) = host_bio_care_assembly(
            Some("1"),
            &presence,
            sessions,
            &tokio::runtime::Handle::current(),
            SessionId::new(),
            ritual,
            ritual_error,
        )
        .expect("开关开 = 造出供给面");
        assert!(care_config.bio_candidates_enabled());
        assert!(
            !CareConfig::default().bio_candidates_enabled(),
            "缺省配置 = 关"
        );
        assert!(
            !dir.path().join("ritual_state.json").exists(),
            "构造供给不读不写: 首拍才读存储"
        );
        assert_eq!(
            BIO_CARE_CANDIDATES_ENABLE_ENV,
            "APEIRETH_ENABLE_BIO_CARE_CANDIDATES"
        );
    }

    /// ② 时间口径: 心跳帧不冒充可见; 时刻单调不回退; 无回合帧 = None (不编)。
    #[test]
    fn heartbeat_frames_never_refresh_last_seen_and_the_moment_is_monotonic() {
        let last = AtomicI64::new(0);
        let turn_at = NOW - i64::try_from(3 * DAY_MS).expect("fits");
        record_turn_frame(&last, &frame(turn_at, PresenceSignificance::Turn));
        record_turn_frame(&last, &frame(NOW, PresenceSignificance::Heartbeat));
        assert_eq!(
            last.load(Ordering::SeqCst),
            turn_at,
            "心跳帧不冒充上次可见 (每 60s 刷新的是帧时刻, 不是活动时刻)"
        );

        record_turn_frame(
            &last,
            &frame(
                turn_at - i64::try_from(HOUR_MS).expect("fits"),
                PresenceSignificance::Turn,
            ),
        );
        assert_eq!(
            last.load(Ordering::SeqCst),
            turn_at,
            "时刻单调: 更早/乱序的回合帧不回退"
        );

        assert_eq!(
            PresenceMomentProbe::for_test(turn_at).last_seen_ms(),
            Some(millis_u64(turn_at))
        );
        assert_eq!(
            PresenceMomentProbe::for_test(0).last_seen_ms(),
            None,
            "未见过回合帧 = None, 不编时间"
        );
    }

    /// ③ fake presence 时刻 (3 天前) + 真实形状线索 → Days 重聚候选 (非 Brief)。
    #[test]
    fn a_three_day_presence_gap_yields_a_days_reunion_candidate() {
        let last_seen = NOW - i64::try_from(3 * DAY_MS).expect("fits");
        let bio = supply(
            last_seen,
            Some(vec![thread("thread-1", millis_u64(NOW) - HOUR_MS)]),
        );

        let candidates = bio_care_candidates(&enabled(), &bio, NOW);
        assert_eq!(candidates.len(), 1, "重聚一条: {candidates:?}");
        assert_eq!(candidates[0].rule_id, "reunion.days");
        assert_eq!(candidates[0].tier, CareTier::Reunion);

        let inputs = bio.reunion_inputs().expect("时刻与线索都可取");
        assert_eq!(inputs.last_seen_ms, millis_u64(last_seen), "时刻原样透传");
        assert_eq!(inputs.threads.len(), 1);
    }

    /// ④ 无 presence 数据 → 无重聚输入 → 零候选 (不拿会话时间冒充可见)。
    #[test]
    fn no_presence_moment_means_no_reunion_inputs_and_no_candidate() {
        let bio = supply(0, Some(vec![thread("thread-1", millis_u64(NOW) - HOUR_MS)]));
        assert!(bio.reunion_inputs().is_none());
        assert!(
            bio_care_candidates(&enabled(), &bio, NOW).is_empty(),
            "无上次可见 = 不产候选"
        );
    }

    /// ⑤ provider 缺线索 (读失败 = None / 读到空) → 重聚不产出。
    #[test]
    fn missing_or_empty_clues_leave_no_reunion_candidate() {
        let last_seen = NOW - i64::try_from(3 * DAY_MS).expect("fits");
        let missing = supply(last_seen, None);
        assert!(missing.reunion_inputs().is_none(), "读失败 = None");
        assert!(bio_care_candidates(&enabled(), &missing, NOW).is_empty());

        let empty = supply(last_seen, Some(Vec::new()));
        let inputs = empty.reunion_inputs().expect("时刻可得, 线索读到 0 条");
        assert!(inputs.threads.is_empty());
        assert!(
            bio_care_candidates(&enabled(), &empty, NOW).is_empty(),
            "无可续线索不产候选"
        );
    }

    /// ⑥ 总开关关 (默认): 有数据也零候选 (内核门, defense-in-depth)。
    #[test]
    fn the_master_switch_off_produces_no_candidate_from_a_populated_supply() {
        let last_seen = NOW - i64::try_from(3 * DAY_MS).expect("fits");
        let bio = supply(
            last_seen,
            Some(vec![thread("thread-1", millis_u64(NOW) - HOUR_MS)]),
        );
        assert!(!CareConfig::default().bio_candidates_enabled());
        assert!(
            bio_care_candidates(&CareConfig::default(), &bio, NOW).is_empty(),
            "开关关: 连供给都不读, 零候选"
        );
    }

    /// ⑦ 会话线索映射: 摘要 = 最近用户消息 (真实内容), 触碰时刻 = updated_at;
    /// 关怀自身会话/无用户消息跳过; 摘要截断 120 字符。
    #[test]
    fn session_clues_map_to_resume_threads_with_real_timestamps() {
        let care_session = SessionId::new();
        let base = Timestamp::from_epoch_millis(NOW - i64::try_from(HOUR_MS).expect("fits"))
            .expect("valid timestamp");
        let clock = VirtualClock::new(base.as_datetime());

        let mut older = Session::new(SessionId::new(), &clock);
        older.append(NormalizedMessage::user("旧问题"), &clock);
        clock.advance(chrono::Duration::minutes(30));
        older.append(NormalizedMessage::user("最近在推进的项目"), &clock);
        older.append(NormalizedMessage::assistant("收到"), &clock);

        let mut care = Session::new(care_session, &clock);
        care.append(NormalizedMessage::user("关怀自身会话"), &clock);

        let mut no_user = Session::new(SessionId::new(), &clock);
        no_user.append(NormalizedMessage::assistant("只有助手话"), &clock);

        let mut long = Session::new(SessionId::new(), &clock);
        long.append(NormalizedMessage::user("长".repeat(300)), &clock);

        let sessions = vec![older, care, no_user, long];
        let threads = resume_threads_from_sessions(&sessions, &care_session);

        assert_eq!(threads.len(), 2, "只保留有用户消息的普通会话: {threads:?}");
        assert_eq!(threads[0].summary, "最近在推进的项目");
        assert_eq!(threads[0].id, sessions[0].id.to_string());
        assert_eq!(
            threads[0].last_touched_ms,
            millis_u64(sessions[0].updated_at.epoch_millis()),
            "触碰时刻 = updated_at 真值"
        );
        assert_eq!(threads[0].emotion_weight, 0.0, "无情绪来源 ⇒ 0.0 (不冒充)");
        assert_eq!(
            threads[1].summary.chars().count(),
            RESUME_THREAD_SUMMARY_CHARS_MAX,
            "摘要截断到 120 字符"
        );
    }

    /// ⑧ 时钟回拨/未来时刻: 原样透传不夹取, classify 归 Brief → 零候选 (不编 gap)。
    #[test]
    fn a_future_presence_moment_degrades_to_brief_without_inventing_time() {
        let last_seen = NOW + i64::try_from(HOUR_MS).expect("fits");
        let bio = supply(
            last_seen,
            Some(vec![thread("thread-1", millis_u64(NOW) - HOUR_MS)]),
        );
        let inputs = bio.reunion_inputs().expect("时刻原样可得");
        assert_eq!(inputs.last_seen_ms, millis_u64(last_seen), "不夹取、不编造");
        assert!(
            bio_care_candidates(&enabled(), &bio, NOW).is_empty(),
            "未来时刻归 Brief (防御性降级), 不演戏"
        );
    }

    /// ⑨ 空仪式存储: 文件缺失 = 合法空态 ⇒ 零状态零排程 = 仪式 0 候选 (不编)。
    #[test]
    fn an_empty_ritual_state_store_yields_zero_ritual_candidates() {
        let last_seen = NOW - i64::try_from(3 * DAY_MS).expect("fits");
        let bio = supply(
            last_seen,
            Some(vec![thread("thread-1", millis_u64(NOW) - HOUR_MS)]),
        );

        let ritual = bio.ritual_inputs();
        assert!(
            ritual.states.is_empty(),
            "缺失文件 ⇒ 零状态 (不编 last_done)"
        );
        assert_eq!(ritual.config, RitualConfig::default(), "不填默认排程");
        assert!(
            bio.ritual_error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none(),
            "缺失文件不是错误"
        );

        let candidates = bio_care_candidates(&enabled(), &bio, NOW);
        assert_eq!(candidates.len(), 1);
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.tier != CareTier::Ritual),
            "仪式 0 候选"
        );
    }

    /// ⑯ W3.5 主路径: 存储里有状态 + 显式排程 ⇒ 开关开时仪式候选出现;
    /// 开关关 = 内核门零候选 (即使存储有数据)。
    #[test]
    fn a_persisted_ritual_state_and_schedule_produce_a_ritual_candidate_when_the_switch_is_on() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(RitualStateStore::under_data_dir(dir.path()));
        store
            .save(&RitualStateBody {
                config: Some(ritual_config()),
                states: vec![RitualState::fresh(RitualKind::MorningCheckIn)],
            })
            .expect("仪式账落盘");
        let bio = supply_with_ritual(Arc::clone(&store), 0, None);

        let ritual = bio.ritual_inputs();
        assert_eq!(
            ritual.states.len(),
            1,
            "状态来自 `<care_dir>/ritual_state.json`"
        );
        assert_eq!(ritual.config, ritual_config(), "显式排程来自存储");
        assert!(
            bio.ritual_error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_none(),
            "读成功不挂错误"
        );

        let candidates = bio_care_candidates(&enabled(), &bio, NOW);
        assert_eq!(
            candidates.len(),
            1,
            "开关开 + 有状态 + 有排程 = 仪式候选: {candidates:?}"
        );
        assert_eq!(candidates[0].tier, CareTier::Ritual);
        assert_eq!(candidates[0].rule_id, "ritual.morning_check_in.due");

        assert!(
            bio_care_candidates(&CareConfig::default(), &bio, NOW).is_empty(),
            "开关关: 连供给都不读, 零候选"
        );
    }

    /// ⑯b R1b-3 端到端（文件即通道）: 模型面工具执行写盘 ⇒ 下一次心跳读盘
    /// 产出仪式候选。跨模块无需新增总线信号变体。
    #[tokio::test]
    async fn a_ritual_tool_write_is_visible_to_the_next_heartbeat_beat() {
        use apeireth_core::kernel::Clock;
        use apeireth_plugin::ToolCapability;
        use apeireth_protocol::canonical::ToolCall;
        use apeireth_runtime_assembly::canonical::ritual_tools::{BioRitualState, RitualDoneTool};

        let dir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(RitualStateStore::under_data_dir(dir.path()));
        // 装配面先落下显式排程 (存储是唯一真源, 心跳侧不填默认排程)。
        store
            .save(&RitualStateBody {
                config: Some(ritual_config()),
                states: Vec::new(),
            })
            .expect("排程落盘");

        let base = Timestamp::from_epoch_millis(NOW).expect("valid timestamp");
        let state = Arc::new(BioRitualState::new(
            RitualStateStore::under_data_dir(dir.path()),
            Arc::new(VirtualClock::new(base.as_datetime())),
        ));
        let tool = RitualDoneTool::new(state);
        let call = ToolCall {
            id: "call-1".to_string(),
            name: "ritual_done".to_string(),
            arguments: serde_json::json!({ "kind": "morning_check_in" }),
        };
        let result = tool.invoke(&call).await;
        assert!(result.is_ok(), "工具执行失败: {result:?}");

        let bio = supply_with_ritual(Arc::clone(&store), 0, None);
        let ritual = bio.ritual_inputs();
        assert_eq!(ritual.states.len(), 1, "工具写盘 ⇒ 下一拍读得到状态");
        assert_eq!(ritual.states[0].kind, RitualKind::MorningCheckIn);
        assert_eq!(
            ritual.states[0].last_done_ms,
            u64::try_from(NOW).expect("fits"),
            "写盘时刻 = 装配注入时钟的值 (不臆造)"
        );

        let just_done = bio_care_candidates(&enabled(), &bio, NOW);
        assert!(
            just_done
                .iter()
                .all(|candidate| candidate.tier != CareTier::Ritual),
            "刚完成的仪式不应立刻再次到点: {just_done:?}"
        );

        // 到点判定按 24h 周期: 两个周期后同一份存储 ⇒ 仪式候选可见。
        let later = NOW + 2 * 24 * 60 * 60 * 1000;
        let candidates = bio_care_candidates(&enabled(), &bio, later);
        assert_eq!(
            candidates.len(),
            1,
            "周期到点后心跳读同一文件 ⇒ 仪式候选: {candidates:?}"
        );
        assert_eq!(candidates[0].tier, CareTier::Ritual);
        assert!(
            candidates[0]
                .rule_id
                .starts_with("ritual.morning_check_in."),
            "候选规则来自同一仪式排程: {}",
            candidates[0].rule_id
        );
    }

    /// ⑰ 坏文件 fail-closed: 不产仪式候选 + 真因留痕 + 不覆盖损坏文件
    /// (不静默回退空表重写)。
    #[test]
    fn a_corrupt_ritual_state_file_fails_closed_and_reports_the_cause() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Arc::new(RitualStateStore::under_data_dir(dir.path()));
        std::fs::write(store.path(), "{ not json").expect("写坏文件");
        let bio = supply_with_ritual(Arc::clone(&store), 0, None);

        let ritual = bio.ritual_inputs();
        assert!(ritual.states.is_empty(), "坏文件 = 不编状态");
        assert_eq!(ritual.config, RitualConfig::default(), "不回退显式排程");
        let reported = bio
            .ritual_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .expect("坏文件真因留痕");
        assert!(reported.contains("refused to open"), "真因: {reported}");

        assert!(
            bio_care_candidates(&enabled(), &bio, NOW).is_empty(),
            "坏文件 = 仪式 0 候选 (fail-closed)"
        );
        assert_eq!(
            std::fs::read_to_string(store.path()).expect("读文件"),
            "{ not json",
            "读取不清洗损坏文件"
        );
    }

    /// ⑱ 显式完成/跳过注入口: 开关关/未绑定 = `Ok(false)` 零写入; 绑定后写回,
    /// 独立实例下次读一致 (真落盘, 不是内存缓存)。
    #[test]
    fn the_explicit_ritual_ports_are_zero_write_noops_until_bound_then_persist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = CareHeartbeatSource::new();
        assert!(!source.has_ritual_state_store(), "未 start = 未绑定");
        assert_eq!(
            source.record_ritual_done(RitualKind::MorningCheckIn, millis_u64(NOW)),
            Ok(false),
            "开关关/未绑定 = 零写入"
        );
        assert_eq!(
            source.record_ritual_skip(RitualKind::MorningCheckIn),
            Ok(false)
        );
        assert!(
            !dir.path().join("ritual_state.json").exists(),
            "未绑定 = 不创建 ritual_state.json"
        );
        assert!(source.ritual_state_error().is_none());

        // `start` 成功路径才绑定 (此处直接注内部槽位, 免起心跳线程)。
        let store = Arc::new(RitualStateStore::under_data_dir(dir.path()));
        *source
            .ritual
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::clone(&store));
        assert!(source.has_ritual_state_store());

        assert_eq!(
            source.record_ritual_done(RitualKind::MorningCheckIn, millis_u64(NOW)),
            Ok(true),
            "绑定后写入成功"
        );
        assert_eq!(
            source.record_ritual_skip(RitualKind::MorningCheckIn),
            Ok(true)
        );

        let (config, states) = RitualStateStore::under_data_dir(dir.path())
            .inputs()
            .expect("新实例装载");
        assert_eq!(config, RitualConfig::default(), "未写排程 = organ 默认");
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].kind, RitualKind::MorningCheckIn);
        assert_eq!(states[0].last_done_ms, millis_u64(NOW), "完成时刻落盘");
        assert_eq!(states[0].consecutive_done, 1);
        assert_eq!(states[0].skipped_count, 1, "跳过计数落盘");
    }

    /// ⑩ 真 PresenceService 回合帧 → 探针 (订阅路径实测): 回合帧入探针,
    /// 后续心跳帧不覆盖 (与 ② 的纯函数口径闭环)。
    #[tokio::test]
    async fn a_real_presence_turn_frame_arrives_at_the_moment_probe() {
        use apeireth_core::kernel::{CapabilityId, RequestId, TraceId};
        use apeireth_runtime::canonical::{RuntimeEvent, RuntimeEventSink};

        let presence = PresenceService::new(EventBus::new(4));
        let probe = PresenceMomentProbe::attach(&presence, &tokio::runtime::Handle::current());
        assert_eq!(probe.last_seen_ms(), None, "订阅前无回合帧 = 不编时间");

        let sink: &dyn RuntimeEventSink = presence.as_ref();
        let session = SessionId::new();
        let trace = TraceId::new();
        sink.emit(RuntimeEvent::TurnStarted {
            session,
            request: RequestId::new(),
            trace,
        });
        sink.emit(RuntimeEvent::TurnCompleted {
            session,
            request: RequestId::new(),
            trace,
            rounds: 1,
            served_by: CapabilityId::new("provider.fake").expect("valid capability id"),
        });

        // 观察任务在单线程 tokio 测试里需要被调度一次: 有界让出, 不空转断言。
        let mut seen = None;
        for _ in 0..100 {
            tokio::task::yield_now().await;
            seen = probe.last_seen_ms();
            if seen.is_some() {
                break;
            }
        }
        assert!(seen.is_some(), "真 presence 回合帧应被探针采集");

        let before = probe.last_seen_ms();
        presence.emit_heartbeat();
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
        assert_eq!(probe.last_seen_ms(), before, "心跳帧不覆盖回合时刻");
    }

    /// ⑪ 节律门开关 fail-closed: 只认 `"1"`; 关 = 不构造门、连时钟都不读
    /// (连时区 env 也不解析)。
    #[test]
    fn bio_circadian_env_switch_is_fail_closed_and_lazy() {
        assert_eq!(BIO_CIRCADIAN_ENABLE_ENV, "APEIRETH_ENABLE_BIO_CIRCADIAN");
        for raw in [
            None,
            Some(""),
            Some("0"),
            Some("true"),
            Some("TRUE"),
            Some("yes"),
            Some("on"),
            Some("2"),
            Some("01"),
        ] {
            let reads = Arc::new(AtomicUsize::new(0));
            let observed = Arc::clone(&reads);
            let clock: CircadianGateClock = Arc::new(move || {
                observed.fetch_add(1, Ordering::SeqCst);
                NOW
            });
            let gate = host_bio_circadian_assembly(raw, Some("480"), clock, NOW)
                .expect("关 = 合法 (不解析时区)");
            assert!(gate.is_none(), "非 \"1\" 一律关: {raw:?}");
            assert_eq!(
                reads.load(Ordering::SeqCst),
                0,
                "关 = 不构造门 = 零时钟读取: {raw:?}"
            );
        }

        let clock: CircadianGateClock = Arc::new(|| NOW);
        let gate = host_bio_circadian_assembly(Some("1"), Some("480"), clock, NOW)
            .expect("开 = 合法")
            .expect("开 = 构造门");
        assert!(gate.enabled());
        assert_eq!(gate.state().config().tz_offset_minutes, 480);
        assert_eq!(gate.ticks(), 0, "构造不咨询, 只初始化首帧快照");
    }

    /// ⑫ 显式时区配置: 缺省 `0` = **UTC (不推断本地时区)**; 坏值/越界拒绝即帧。
    #[test]
    fn the_explicit_timezone_defaults_to_utc_and_rejects_bad_values() {
        assert_eq!(
            BIO_CIRCADIAN_TZ_OFFSET_MIN_ENV,
            "APEIRETH_BIO_CIRCADIAN_TZ_OFFSET_MIN"
        );
        assert_eq!(DEFAULT_BIO_CIRCADIAN_TZ_OFFSET_MIN, 0);

        let base = CircadianConfig::default();
        let utc = bio_circadian_config_from_env_value(None).expect("缺省合法");
        assert_eq!(utc.tz_offset_minutes, 0, "缺省 0 = UTC, 不是本地时区推断");
        // 其余字段 = 内核 `Default` 工程缺省 (本批未开 env 的字段, 已列文档)。
        assert_eq!(utc.wake_hour, base.wake_hour);
        assert_eq!(utc.sleep_hour, base.sleep_hour);
        assert_eq!(utc.ultradian_focus_ms, base.ultradian_focus_ms);
        assert_eq!(utc.ultradian_rest_ms, base.ultradian_rest_ms);

        for (raw, expected) in [
            (Some("480"), 480),
            (Some("0"), 0),
            (Some(" -300 "), -300),
            (Some("840"), 840),
            (Some("-840"), -840),
        ] {
            assert_eq!(
                bio_circadian_config_from_env_value(raw)
                    .expect("显式值合法")
                    .tz_offset_minutes,
                expected,
                "显式时区原样生效: {raw:?}"
            );
        }
        for bad in [
            Some(""),
            Some("abc"),
            Some("8.5"),
            Some("+"),
            Some("841"),
            Some("-841"),
            Some("9999"),
        ] {
            assert!(
                bio_circadian_config_from_env_value(bad).is_err(),
                "坏值/越界拒绝即帧 (不猜、不静默夹取): {bad:?}"
            );
        }
    }

    /// ⑬ 时区生效差异: 同一 epoch, UTC 17:00 = 昼 (放行) / 北京 01:00 = 深夜
    /// (拦下) —— 时区不是猜出来的。
    #[test]
    fn the_timezone_offset_changes_the_gate_outcome_for_the_same_epoch() {
        let now = -25_200_000; // UTC 17:00 = 北京 01:00 (深夜)
        let mut utc =
            host_bio_circadian_assembly(Some("1"), None, Arc::new(move || now), now - 60_000)
                .expect("合法")
                .expect("开");
        let mut beijing = host_bio_circadian_assembly(
            Some("1"),
            Some("480"),
            Arc::new(move || now),
            now - 60_000,
        )
        .expect("合法")
        .expect("开");

        assert_eq!(
            utc.state().inputs().phase,
            CircadianPhase::Day,
            "同一 epoch 在 UTC 视图 = 昼"
        );
        assert_eq!(utc.decide(), BioCircadianDecision::Allowed, "UTC 视图放行");
        assert_eq!(
            beijing.decide(),
            BioCircadianDecision::Suppressed(ProactiveGate::QuietHours),
            "同一 epoch 在北京视图 = 深夜勿扰 (时区显式生效)"
        );
    }

    /// ⑭ 会话起点口径: `session_start_ms` 由调用方显式给 (生产 = 装配时刻读
    /// 一次同一时钟), 构造不改写; 起点决定超日窗口 (26 分钟前 = 休息窗 ⇒ 拦下;
    /// 1 分钟前 = 专注窗 ⇒ 放行)。
    #[test]
    fn the_session_start_is_explicit_and_anchors_the_ultradian_window() {
        let reads = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&reads);
        let clock: CircadianGateClock = Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            NOW
        });
        let mut rest =
            host_bio_circadian_assembly(Some("1"), Some("480"), clock, NOW - 26 * 60_000)
                .expect("合法")
                .expect("开");
        assert_eq!(
            reads.load(Ordering::SeqCst),
            1,
            "构造读一次时钟 (首帧快照); session_start 不由构造改写"
        );
        assert_eq!(rest.state().session_start_ms(), NOW - 26 * 60_000);
        assert_eq!(
            rest.decide(),
            BioCircadianDecision::Suppressed(ProactiveGate::RestWindow),
            "26 分钟前起点 = 25 分钟专注窗已过, 当前休息窗"
        );

        let mut focus =
            host_bio_circadian_assembly(Some("1"), Some("480"), Arc::new(|| NOW), NOW - 60_000)
                .expect("合法")
                .expect("开");
        assert_eq!(
            focus.decide(),
            BioCircadianDecision::Allowed,
            "1 分钟前起点 = 专注窗 + 昼 = 放行"
        );
    }

    /// ⑮ 默认关: 未设开关 = 不构造门; 即使时区 env 是坏值也不解析 (关 = 原
    /// 路径, 零时钟读取)。
    #[test]
    fn the_default_off_path_never_parses_the_timezone_or_touches_the_clock() {
        let reads = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&reads);
        let clock: CircadianGateClock = Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            NOW
        });
        let gate = host_bio_circadian_assembly(None, Some("not-a-number"), clock, NOW)
            .expect("关 = 不解析时区 (坏值也不报错)");
        assert!(gate.is_none(), "默认关 = 不构造门");
        assert_eq!(reads.load(Ordering::SeqCst), 0, "默认关 = 零时钟读取");
    }
}
