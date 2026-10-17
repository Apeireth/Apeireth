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
//!    `care_status_probe_with_heartbeat` —— 心跳两列从显式 null 转真值。
//!
//! **双闸齐开才起** ([`CareHeartbeatConfig`], 注册即闸 fail-closed):
//! `APEIRETH_ENABLE_CARE=1` 且 `APEIRETH_ENABLE_CARE_HEARTBEAT=1` 才建心跳线程;
//! 关 = 不建线程不 poll 零动作 (自述两列显式 null, 不冒充在跑)。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::SessionId;
use apeireth_gateway::PresenceService;
use apeireth_organ::care::{
    CareClock, CareEngine, CareStore, FnInitiativeGate, RuleFallbackJudge, SystemCareClock,
};
use apeireth_runtime::canonical::heartbeat::HeartbeatDispatch;
use apeireth_runtime::canonical::{Runtime, TurnRequest};
use apeireth_runtime_assembly::{CareHeartbeatConfig, CareHeartbeatProducer, CareHeartbeatStatus};

/// 发起缝失败留痕的有界容量 (只留最近若干条真因, 不无界长)。
const DISPATCH_ERROR_LOG_MAX: usize = 32;

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
            CareStore::new(care_dir),
            Arc::new(SystemCareClock),
            Arc::new(RuleFallbackJudge),
        )
        .with_initiative_gate(Arc::new(gate));
        // ② 发起回调 = 标准回合 (Runtime::execute; 零新消息通道, 照常计费)。
        let dispatch = RuntimeTurnDispatch {
            handle,
            runtime,
            session,
            errors: Arc::clone(&self.errors),
        };
        // ③ 注册即闸: enabled 已核 = 起常驻线程, 真值随探针进自述关怀节。
        let Some(producer) = CareHeartbeatProducer::start_care(
            config,
            engine,
            || SystemCareClock.now_ms(),
            dispatch,
        ) else {
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
