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
//!    presence 回合帧 → 上次可见时刻; 会话账本 → 可续线索; 仪式状态存储未接
//!    ⇒ 仪式 0 候选 (如实, 不编)。开关关 = 不构造供给、不订阅 presence、不读
//!    会话, 走与接线前同一 `start_care_with_feeds` 调用 (逐字节一致)。
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
//! - **仪式状态**: organ `ritual` 只有纯内核 (`RitualState` + `plan_rituals`),
//!   **没有状态存储 API** ⇒ 本供给 [`BioCareInputs::ritual_inputs`] 恒空,
//!   仪式 0 候选 (如实标注, 不编到点状态)。
//! - 开关默认关; 关 = 零订阅、零会话读取、零候选。**不称 DEFAULT ENABLED。**

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
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use apeireth_runtime::canonical::heartbeat::HeartbeatDispatch;
use apeireth_runtime::canonical::{Runtime, Session, SessionStore, TurnRequest};
use apeireth_runtime_assembly::canonical::care_heartbeat::{
    bio_care_assembly_from_env_value, BioCareClock, BioCarePoll,
};
use apeireth_runtime_assembly::canonical::care_reminder::{
    bio_care_candidates_enabled_from_env_value, BioCareInputs, ReunionInputs, RitualInputs,
    BIO_CARE_CANDIDATES_ENABLE_ENV,
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

/// 生物候选真实供给 (W3.3): presence 上次可见 + 会话可续线索。
///
/// 只在 `APEIRETH_ENABLE_BIO_CARE_CANDIDATES=1` 时构造; 构造即订阅 presence
/// (开关关 = 连构造都不发生)。仪式供给恒空 —— organ 无仪式状态存储, 如实。
struct CareBioSupply {
    presence: PresenceMomentProbe,
    clues: Box<dyn ResumeClueSource>,
}

impl BioCareInputs for CareBioSupply {
    fn ritual_inputs(&self) -> RitualInputs {
        // 仪式状态存储未接 (organ ritual 只有纯内核) ⇒ 零状态零排程 = 0 候选。
        // 不填默认排程、不编 last_done_ms。
        RitualInputs::default()
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

/// 宿主装配 (W3.3, 默认关): env 原始值 → (care 接线配置, 生物候选供给面)。
///
/// 非 `"1"` 一律 `None` —— **连供给都不构造** (零 presence 订阅、零会话读取、
/// 零候选), 调用方据此走接线前的 `start_care_with_feeds` 原路径。
fn host_bio_care_assembly(
    raw: Option<&str>,
    presence: &Arc<PresenceService>,
    sessions: Arc<dyn SessionStore>,
    handle: &tokio::runtime::Handle,
    care_session: SessionId,
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
    };
    bio_care_assembly_from_env_value(raw, Box::new(supply), clock)
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
    /// 与接线前逐字节一致。
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
        // ⑤ 生物候选真实供给 (W3.3, 默认关): env 非 "1" = 连会话账本句柄都不取,
        //    不构造供给 (零 presence 订阅/零会话读取), 随后走接线前的原生产者
        //    路径 (逐字节一致)。开关开才把 care 配置注入 engine 并接供给面。
        let bio_raw = std::env::var(BIO_CARE_CANDIDATES_ENABLE_ENV).ok();
        let bio = if bio_care_candidates_enabled_from_env_value(bio_raw.as_deref()) {
            host_bio_care_assembly(
                bio_raw.as_deref(),
                presence,
                Arc::clone(runtime.sessions_arc().store()),
                &handle,
                session,
            )
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
        let producer = match bio {
            // 开关开: 生物候选在规则/提醒之后取供给 → 候选 → 同一纪律管道。
            Some((_, bio_poll)) => CareHeartbeatProducer::start_care_with_feeds_and_bio(
                config,
                engine,
                || SystemCareClock.now_ms(),
                dispatch,
                reminder_feeds,
                bio_poll,
            ),
            // 开关关 (默认): 与接线前同一调用, 零候选零行为变化。
            None => CareHeartbeatProducer::start_care_with_feeds(
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
}

/// 发起缝的标准回合执行 (与常驻宿主同一接法: 心跳线程上同步等回合收束)。
struct RuntimeTurnDispatch {
    handle: tokio::runtime::Handle,
    runtime: Arc<Runtime>,
    session: SessionId,
    errors: Arc<Mutex<Vec<String>>>,
}

impl HeartbeatDispatch for RuntimeTurnDispatch {
    fn dispatch(&mut self, turn_input: &str, _at_ms: i64) {
        let request = TurnRequest::new(self.session, turn_input.to_string());
        if let Err(error) = self.handle.block_on(self.runtime.execute(request)) {
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
    use apeireth_core::kernel::{Timestamp, VirtualClock};
    use apeireth_gateway::{EventBus, PresenceBreath, PresencePad, PresenceSource};
    use apeireth_organ::care::CareTier;
    use apeireth_organ::ritual::RitualConfig;
    use apeireth_protocol::canonical::NormalizedMessage;
    use apeireth_runtime::canonical::InMemorySessionStore;
    use apeireth_runtime_assembly::canonical::care_reminder::bio_care_candidates;

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

    fn supply(last_turn_at_ms: i64, clues: Option<Vec<ResumeThread>>) -> CareBioSupply {
        CareBioSupply {
            presence: PresenceMomentProbe::for_test(last_turn_at_ms),
            clues: Box::new(FixedClues(clues)),
        }
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

    /// ① 开关解析 fail-closed: 默认关, 只认 `"1"`; 关 = 连供给都不构造。
    #[tokio::test]
    async fn host_bio_assembly_is_fail_closed_and_only_accepts_one() {
        let presence = PresenceService::new(EventBus::new(4));
        let sessions: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
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
                )
                .is_none(),
                "非 \"1\" 一律关 (零构造零订阅零读取): {raw:?}"
            );
        }

        let (care_config, _bio) = host_bio_care_assembly(
            Some("1"),
            &presence,
            sessions,
            &tokio::runtime::Handle::current(),
            SessionId::new(),
        )
        .expect("开关开 = 造出供给面");
        assert!(care_config.bio_candidates_enabled());
        assert!(
            !CareConfig::default().bio_candidates_enabled(),
            "缺省配置 = 关"
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

    /// ⑨ 仪式状态存储未接: ritual 供给恒空 ⇒ 仪式 0 候选 (如实, 不编到点状态)。
    #[test]
    fn ritual_stays_empty_until_a_ritual_state_store_is_wired() {
        let last_seen = NOW - i64::try_from(3 * DAY_MS).expect("fits");
        let bio = supply(
            last_seen,
            Some(vec![thread("thread-1", millis_u64(NOW) - HOUR_MS)]),
        );

        let ritual = bio.ritual_inputs();
        assert!(ritual.states.is_empty(), "无仪式状态存储 ⇒ 零状态");
        assert_eq!(ritual.config, RitualConfig::default(), "不填默认排程");

        let candidates = bio_care_candidates(&enabled(), &bio, NOW);
        assert_eq!(candidates.len(), 1);
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.tier != CareTier::Ritual),
            "仪式 0 候选"
        );
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
}
