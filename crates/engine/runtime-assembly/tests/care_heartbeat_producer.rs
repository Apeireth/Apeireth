//! 常驻心跳关怀生产者 (V3-a 收尾一针) 的装配级证明。
//!
//! 走真 [`CareHeartbeatProducer`] 常驻线程 + 真 [`CareEngine`] 关怀内核 +
//! 真 [`Runtime`] 标准回合: 心跳命中 → 标准回合进会话流 (零新消息通道),
//! 关怀回合照常计费 (会话配额账本), 心跳 poll 本身零模型调用 (纯规则轮询)。
//!
//! 口径: 发起闸 (try_take_initiative 同源) 由组装根注入 —— 本文件用
//! [`FnInitiativeGate`] 钉闸语义 (拒绝 = 拦下不丢候选, 放行才发起);
//! 生产接线 = 常驻宿主把 presence 的 `try_take_initiative` 接进同一端口。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_governance::AllowAll;
use apeireth_organ::care::{
    CareClock, CareDiscipline, CareEngine, CareRule, CareStore, CareTrigger, FnInitiativeGate,
    FrozenCareClock, RuleFallbackJudge,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse,
    NormalizedUsage, ToolCall,
};
use apeireth_runtime::canonical::heartbeat::{
    HeartbeatDispatch, HeartbeatPoll, HeartbeatPollOutcome,
};
use apeireth_runtime::canonical::{Runtime, TurnRequest};
use apeireth_runtime_assembly::{
    care_status_probe, care_status_probe_with_heartbeat, CareHeartbeatConfig,
    CareHeartbeatProducer, CareHeartbeatStatus, ProductionBackends, ProductionModules,
    ProductionModulesConfig, CARE_ENABLE_ENV, CARE_HEARTBEAT_ENABLE_ENV,
    CARE_HEARTBEAT_POLL_INTERVAL_ENV, DEFAULT_CARE_HEARTBEAT_POLL_INTERVAL_SECS,
};
use serde_json::{json, Value};

const MODEL: &str = "fake-model-care-heartbeat";

/// 心跳拍测试口径: 25 毫秒一拍 (真间隔, 不假睡)。
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// 冻结时钟锚 (与 V3-a 竖切同一锚点)。
const T0: i64 = 1_786_838_400_000;

struct FakeProvider {
    id: CapabilityId,
    text: &'static str,
    calls: AtomicUsize,
}

impl FakeProvider {
    fn new(text: &'static str) -> Arc<Self> {
        Arc::new(Self {
            id: CapabilityId::new("provider.fake").unwrap(),
            text,
            calls: AtomicUsize::new(0),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl ProviderCapability for FakeProvider {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        vec![
            ModelDescriptor::new(ModelId::new(MODEL).unwrap(), self.id.clone())
                .with_feature(ModelFeature::ToolCalls),
        ]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(NormalizedResponse {
            id: "resp_care_hb".to_string(),
            model: request.model.clone(),
            content: self.text.to_string(),
            finish_reason: Some(apeireth_protocol::canonical::NormalizedFinishReason::Stop),
            usage: NormalizedUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                prompt_cache_hit_tokens: None,
            },
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}

struct ProviderPlugin {
    manifest: PluginManifest,
    provider: Arc<FakeProvider>,
}

impl ProviderPlugin {
    fn new(provider: Arc<FakeProvider>) -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("vendor.fake").unwrap(),
                "1.0.0",
                "Fake provider",
            )
            .declare_capability(
                provider.id().clone(),
                CapabilityKind::Provider,
                "Scripted completions",
            )
            .unwrap(),
            provider,
        })
    }
}

#[async_trait::async_trait]
impl Plugin for ProviderPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        vec![Arc::clone(&self.provider) as Arc<dyn ProviderCapability>]
    }
}

/// 标准回合执行缝的真接线: 真 `Runtime::execute` (与常驻宿主同一接法:
/// 发起回调在心跳线程上同步等回合收束)。
struct RuntimeTurnDispatch {
    handle: tokio::runtime::Handle,
    runtime: Arc<Runtime>,
    session_id: SessionId,
    turns: Arc<AtomicUsize>,
    errors: Arc<Mutex<Vec<String>>>,
}

impl HeartbeatDispatch for RuntimeTurnDispatch {
    fn dispatch(&mut self, turn_input: &str, _at_ms: i64) {
        let request = TurnRequest::new(self.session_id, turn_input.to_string());
        match self.handle.block_on(self.runtime.execute(request)) {
            Ok(_response) => {
                self.turns.fetch_add(1, Ordering::SeqCst);
            }
            Err(error) => {
                self.errors
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .push(format!("{error}"));
            }
        }
    }
}

/// 只计数的发起缝 (不落回合; 命中留痕的真值断言用)。
struct CountingDispatch {
    turns: Arc<AtomicUsize>,
}

impl HeartbeatDispatch for CountingDispatch {
    fn dispatch(&mut self, _turn_input: &str, _at_ms: i64) {
        self.turns.fetch_add(1, Ordering::SeqCst);
    }
}

/// 只计数的 poll 面 (开关/生命周期类用例不进关怀内核)。
struct QuietPoll {
    ticks: Arc<AtomicUsize>,
}

impl HeartbeatPoll for QuietPoll {
    fn poll(&mut self) -> Result<HeartbeatPollOutcome, String> {
        self.ticks.fetch_add(1, Ordering::SeqCst);
        Ok(HeartbeatPollOutcome::Quiet)
    }
}

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(T0).unwrap().as_datetime(),
    ))
}

fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn test_config() -> CareHeartbeatConfig {
    CareHeartbeatConfig {
        enabled: true,
        poll_interval: TEST_POLL_INTERVAL,
    }
}

/// 间隔规则 (1 小时) 到点的关怀规则面; `quiet_off` = 显式关勿扰窗口
/// (本文件盯心跳路径, 勿扰纪律单独一测)。
fn due_care_dir(quiet_off: bool) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = CareStore::new(dir.path());
    store
        .add_rule(CareRule {
            id: "check-in".to_string(),
            trigger: CareTrigger::Interval { hours: 1 },
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: T0,
        })
        .expect("add rule");
    if quiet_off {
        let mut body = store.list().expect("list");
        body.discipline = CareDiscipline {
            quiet_start_minutes: None,
            quiet_end_minutes: None,
            ..CareDiscipline::default()
        };
        store.save(&body).expect("save discipline");
    }
    dir
}

fn care_engine(dir: &std::path::Path, clock: Arc<FrozenCareClock>) -> CareEngine {
    CareEngine::new(CareStore::new(dir), clock, Arc::new(RuleFallbackJudge))
}

async fn wait_for(what: &str, deadline: Duration, mut cond: impl FnMut() -> bool) {
    let until = std::time::Instant::now() + deadline;
    while std::time::Instant::now() < until {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(cond(), "超时未等到: {what}");
}

async fn invoke_self_status(modules: &ProductionModules) -> Value {
    let self_status = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .cloned()
        .expect("tool.self_status");
    let call = ToolCall {
        id: "call_status".into(),
        name: "self_status".into(),
        arguments: json!({}),
    };
    let result = self_status.invoke(&call).await;
    serde_json::from_str(&result.render()).expect("self report json")
}

/// 心跳命中 → 标准回合进会话流 (真 Runtime, 零新消息通道) + poll 零模型调用。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_heartbeat_hit_enters_the_conversation_flow_via_a_standard_turn() {
    let dir = due_care_dir(true);
    let clock = Arc::new(FrozenCareClock::new(T0 + 3_600_000));
    let engine = care_engine(dir.path(), Arc::clone(&clock));
    let provider = FakeProvider::new("最近挺好, 别担心~");
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_governance(Arc::new(AllowAll))
            .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
            .with_default_model(MODEL)
            .build()
            .await
            .expect("runtime builds"),
    );
    let session_id = SessionId::new();
    let turns = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(Mutex::new(Vec::new()));
    let dispatch = RuntimeTurnDispatch {
        handle: tokio::runtime::Handle::current(),
        runtime: Arc::clone(&runtime),
        session_id,
        turns: Arc::clone(&turns),
        errors: Arc::clone(&errors),
    };
    let mut producer = CareHeartbeatProducer::start_care(
        &test_config(),
        engine,
        {
            let clock = Arc::clone(&clock);
            move || clock.now_ms()
        },
        dispatch,
    )
    .expect("开关开 = 注册即起");

    wait_for("心跳命中发起", Duration::from_secs(10), || {
        turns.load(Ordering::SeqCst) >= 1
    })
    .await;
    assert!(producer.shutdown(), "停机收心跳线程");
    assert!(
        errors.lock().unwrap_or_else(|p| p.into_inner()).is_empty(),
        "关怀回合无错: {:?}",
        errors.lock().unwrap_or_else(|p| p.into_inner())
    );

    // 会话流: 关怀回合输入与生成的关怀消息都进普通转写 (与人说的话同一条流)。
    let session = runtime
        .sessions()
        .load_or_create(session_id)
        .await
        .expect("session");
    let texts: Vec<String> = session
        .messages
        .iter()
        .map(|message| ContentPart::join_text(&message.content))
        .collect();
    assert!(
        texts.iter().any(|text| text.contains("[care:check-in]")),
        "关怀回合输入进会话流: {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text.contains("最近挺好, 别担心~")),
        "生成的关怀消息进会话流: {texts:?}"
    );
    // 计费诚实: 心跳 poll 本身零模型调用 —— 模型只在标准回合里被叫。
    assert_eq!(
        provider.call_count(),
        turns.load(Ordering::SeqCst),
        "poll 零模型调用: 模型调用数 = 标准回合数"
    );
    assert_eq!(provider.call_count(), 1, "只发起一次 (间隔规则回锚)");
}

/// 打扰纪律在心跳路径生效: 勿扰窗口内的到点候选被拦下 —— 零回合零模型调用,
/// 候选不丢 (不写台账不消耗发起锚点)。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_disturbance_discipline_holds_on_the_heartbeat_path() {
    // 冻结时钟 23:13 UTC, 默认勿扰窗口 23:00-8:00 → 纪律①拦下。
    let dir = due_care_dir(false);
    let clock = Arc::new(FrozenCareClock::new(T0 + 3_600_000));
    let engine = care_engine(dir.path(), Arc::clone(&clock));
    let provider = FakeProvider::new("不该出现");
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_governance(Arc::new(AllowAll))
            .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
            .with_default_model(MODEL)
            .build()
            .await
            .expect("runtime builds"),
    );
    let turns = Arc::new(AtomicUsize::new(0));
    let dispatch = RuntimeTurnDispatch {
        handle: tokio::runtime::Handle::current(),
        runtime,
        session_id: SessionId::new(),
        turns: Arc::clone(&turns),
        errors: Arc::new(Mutex::new(Vec::new())),
    };
    let handle = Arc::new(Mutex::new(
        CareHeartbeatProducer::start_care(
            &test_config(),
            engine,
            {
                let clock = Arc::clone(&clock);
                move || clock.now_ms()
            },
            dispatch,
        )
        .expect("开关开 = 注册即起"),
    ));

    wait_for("若干拍 poll", Duration::from_secs(10), || {
        handle
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .status()
            .polls
            >= 3
    })
    .await;
    handle.lock().unwrap_or_else(|p| p.into_inner()).shutdown();
    assert_eq!(turns.load(Ordering::SeqCst), 0, "勿扰窗口内不发起");
    assert_eq!(provider.call_count(), 0, "拦下 = 零模型调用零计费");
    let body = CareStore::new(dir.path()).list().expect("list");
    assert!(
        body.history.is_empty(),
        "候选不丢不写台账: {:?}",
        body.history
    );
    assert!(
        body.fired_at.is_empty(),
        "发起锚点不消耗: {:?}",
        body.fired_at
    );
}

/// 无规则命中 = 零动作零计费: poll 照跑 (纯规则轮询), 零回合零模型调用零会话。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_due_rule_means_zero_action_and_zero_billing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let clock = Arc::new(FrozenCareClock::new(T0));
    let engine = care_engine(dir.path(), Arc::clone(&clock));
    let provider = FakeProvider::new("不该出现");
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_governance(Arc::new(AllowAll))
            .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
            .with_default_model(MODEL)
            .build()
            .await
            .expect("runtime builds"),
    );
    let session_id = SessionId::new();
    let turns = Arc::new(AtomicUsize::new(0));
    let dispatch = RuntimeTurnDispatch {
        handle: tokio::runtime::Handle::current(),
        runtime: Arc::clone(&runtime),
        session_id,
        turns: Arc::clone(&turns),
        errors: Arc::new(Mutex::new(Vec::new())),
    };
    let handle = Arc::new(Mutex::new(
        CareHeartbeatProducer::start_care(
            &test_config(),
            engine,
            {
                let clock = Arc::clone(&clock);
                move || clock.now_ms()
            },
            dispatch,
        )
        .expect("开关开 = 注册即起"),
    ));

    wait_for("若干拍 poll", Duration::from_secs(10), || {
        handle
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .status()
            .polls
            >= 3
    })
    .await;
    let status = handle.lock().unwrap_or_else(|p| p.into_inner()).status();
    handle.lock().unwrap_or_else(|p| p.into_inner()).shutdown();
    assert_eq!(status.hits, 0, "零命中");
    assert_eq!(turns.load(Ordering::SeqCst), 0, "零发起");
    assert_eq!(provider.call_count(), 0, "poll 零模型调用 = 零计费");
    // 零会话 = 会话配额账本零消耗 (无免费通道也不白烧)。
    assert!(
        runtime
            .sessions()
            .load(&session_id)
            .await
            .expect("load")
            .is_none(),
        "零动作 = 零会话零账本消耗"
    );
}

/// 开关 fail-closed: `APEIRETH_ENABLE_CARE_HEARTBEAT` 注册即闸默认关,
/// 依赖 `APEIRETH_ENABLE_CARE` (上游闸不开 = 心跳不起); 间隔可配, 非法即拒。
#[tokio::test]
async fn the_switch_is_fail_closed_and_depends_on_the_care_switch() {
    // 双闸齐开才开; 任何缺省/垃圾值 = 关 (fail-closed)。
    let off = CareHeartbeatConfig::from_env_value(None, None, None).expect("缺省合法");
    assert!(!off.enabled, "默认关 (fail-closed)");
    let heartbeat_only = CareHeartbeatConfig::from_env_value(None, Some("1"), None).expect("合法");
    assert!(
        !heartbeat_only.enabled,
        "依赖 {CARE_ENABLE_ENV}: 上游闸不开 = 心跳不起"
    );
    let care_only = CareHeartbeatConfig::from_env_value(Some("1"), Some("0"), None).expect("合法");
    assert!(!care_only.enabled, "总闸非 1 = 关");
    let garbage = CareHeartbeatConfig::from_env_value(Some("1"), Some("true"), None).expect("合法");
    assert!(!garbage.enabled, "垃圾值不放行 (fail-closed)");
    let on = CareHeartbeatConfig::from_env_value(Some("1"), Some("1"), None).expect("合法");
    assert!(on.enabled, "双闸齐开才开");
    assert_eq!(
        on.poll_interval,
        Duration::from_secs(DEFAULT_CARE_HEARTBEAT_POLL_INTERVAL_SECS),
        "间隔缺省 5 分钟"
    );

    // 间隔可配 + 非法即拒 (不猜)。
    let fast = CareHeartbeatConfig::from_env_value(Some("1"), Some("1"), Some("45")).expect("合法");
    assert_eq!(fast.poll_interval, Duration::from_secs(45));
    assert!(
        CareHeartbeatConfig::from_env_value(Some("1"), Some("1"), Some("abc")).is_err(),
        "非整数秒拒绝即帧"
    );
    assert!(
        CareHeartbeatConfig::from_env_value(Some("1"), Some("1"), Some("0")).is_err(),
        "0 秒拒绝即帧"
    );
    assert_eq!(CARE_HEARTBEAT_ENABLE_ENV, "APEIRETH_ENABLE_CARE_HEARTBEAT");
    assert_eq!(CARE_ENABLE_ENV, "APEIRETH_ENABLE_CARE");
    assert_eq!(
        CARE_HEARTBEAT_POLL_INTERVAL_ENV,
        "APEIRETH_CARE_HEARTBEAT_POLL_INTERVAL_SECS"
    );

    // 注册即闸: 关 = 不建线程不 poll (显式 None, 不是假运行)。
    let ticks = Arc::new(AtomicUsize::new(0));
    let producer = CareHeartbeatProducer::start(
        &CareHeartbeatConfig::default(),
        || T0,
        QuietPoll {
            ticks: Arc::clone(&ticks),
        },
        CountingDispatch {
            turns: Arc::new(AtomicUsize::new(0)),
        },
    );
    assert!(producer.is_none(), "开关关 = 不注册 (fail-closed)");
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(ticks.load(Ordering::SeqCst), 0, "未注册 = 零 poll");
}

/// 停机清理: 心跳线程随优雅停机收 (join 回收, 线程句柄无泄漏), 不僵尸
/// (停机后零 poll 零发起), 幂等。
#[tokio::test]
async fn shutdown_reaps_the_heartbeat_thread_without_leaking() {
    let mut producers: Vec<CareHeartbeatProducer> = (0..4)
        .map(|_| {
            CareHeartbeatProducer::start(
                &test_config(),
                || T0,
                QuietPoll {
                    ticks: Arc::new(AtomicUsize::new(0)),
                },
                CountingDispatch {
                    turns: Arc::new(AtomicUsize::new(0)),
                },
            )
            .expect("开关开 = 注册即起")
        })
        .collect();
    for producer in &producers {
        assert!(producer.is_running(), "常驻心跳线程在跑");
        assert!(!producer.thread_reaped(), "运行中句柄未回收 (如实)");
    }
    {
        let producer = &producers[0];
        let before = producer.status().polls;
        tokio::time::sleep(Duration::from_millis(120)).await;
        let after = producer.status().polls;
        assert!(after > before, "停机前在 poll: {before} -> {after}");
    }
    for producer in &mut producers {
        assert!(producer.shutdown(), "第一次停机真收掉线程");
        assert!(!producer.is_running(), "停机后不僵尸");
        assert!(producer.thread_reaped(), "线程句柄无泄漏 (join 已回收)");
    }
    for producer in &mut producers {
        assert!(!producer.shutdown(), "停机幂等");
    }
    // 停机后零 poll (不僵尸, 无幽灵 tick)。
    let ticks_frozen_at = producers[0].status().polls;
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(
        producers[0].status().polls,
        ticks_frozen_at,
        "停机后零 poll (不僵尸, 无幽灵 tick)"
    );
}

/// poll 间隔可配: 配 200 毫秒一拍, 自述「下次 poll 时刻」= 时钟 + 间隔 (如实)。
#[tokio::test]
async fn the_poll_interval_is_configurable_and_reported_honestly() {
    let ticks = Arc::new(AtomicUsize::new(0));
    let handle = Arc::new(Mutex::new(
        CareHeartbeatProducer::start(
            &CareHeartbeatConfig {
                enabled: true,
                poll_interval: Duration::from_millis(200),
            },
            || T0,
            QuietPoll {
                ticks: Arc::clone(&ticks),
            },
            CountingDispatch {
                turns: Arc::new(AtomicUsize::new(0)),
            },
        )
        .expect("开关开 = 注册即起"),
    ));
    wait_for("首拍", Duration::from_secs(10), || {
        handle
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .status()
            .polls
            >= 1
    })
    .await;
    let status = handle.lock().unwrap_or_else(|p| p.into_inner()).status();
    handle.lock().unwrap_or_else(|p| p.into_inner()).shutdown();
    assert_eq!(
        status.next_poll_at_ms,
        Some(T0 + 200),
        "下次 poll 时刻 = 时钟 + 间隔 (200ms 档)"
    );
    assert_eq!(
        CareHeartbeatConfig::default().poll_interval,
        Duration::from_secs(300),
        "缺省 5 分钟"
    );
}

/// 发起闸 (try_take_initiative 同源) 在心跳路径生效: 拒绝 = 拦下不丢候选
/// (零回合零计费), 放行才发起并消耗发起锚点。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_initiative_gate_gates_the_initiation_without_losing_the_candidate() {
    let dir = due_care_dir(true);
    let clock = Arc::new(FrozenCareClock::new(T0 + 3_600_000));
    let provider = FakeProvider::new("关怀一句");
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_governance(Arc::new(AllowAll))
            .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
            .with_default_model(MODEL)
            .build()
            .await
            .expect("runtime builds"),
    );
    let turns = Arc::new(AtomicUsize::new(0));
    let dispatch = RuntimeTurnDispatch {
        handle: tokio::runtime::Handle::current(),
        runtime,
        session_id: SessionId::new(),
        turns: Arc::clone(&turns),
        errors: Arc::new(Mutex::new(Vec::new())),
    };
    // 闸脚本: 先拒 (防唠叨预算用尽), 后放 (窗口滑过)。
    let admitted = Arc::new(AtomicUsize::new(0));
    let consulted = Arc::new(AtomicUsize::new(0));
    let gate = FnInitiativeGate::new({
        let admitted = Arc::clone(&admitted);
        let consulted = Arc::clone(&consulted);
        move |_now_ms| {
            consulted.fetch_add(1, Ordering::SeqCst);
            admitted.load(Ordering::SeqCst) > 0
        }
    });
    let engine = care_engine(dir.path(), Arc::clone(&clock)).with_initiative_gate(Arc::new(gate));
    let handle = Arc::new(Mutex::new(
        CareHeartbeatProducer::start_care(
            &test_config(),
            engine,
            {
                let clock = Arc::clone(&clock);
                move || clock.now_ms()
            },
            dispatch,
        )
        .expect("开关开 = 注册即起"),
    ));

    wait_for("闸被问询若干次", Duration::from_secs(10), || {
        consulted.load(Ordering::SeqCst) >= 3
            && handle
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .status()
                .polls
                >= 3
    })
    .await;
    assert_eq!(turns.load(Ordering::SeqCst), 0, "闸拒绝 = 不发起");
    assert_eq!(provider.call_count(), 0, "闸拒绝 = 零模型调用零计费");
    let body = CareStore::new(dir.path()).list().expect("list");
    assert!(
        body.history.is_empty(),
        "闸拦下不写台账: {:?}",
        body.history
    );
    assert!(body.fired_at.is_empty(), "候选不丢: {:?}", body.fired_at);

    admitted.fetch_add(1, Ordering::SeqCst);
    wait_for("闸放行发起", Duration::from_secs(10), || {
        turns.load(Ordering::SeqCst) >= 1
    })
    .await;
    let status = handle.lock().unwrap_or_else(|p| p.into_inner()).status();
    handle.lock().unwrap_or_else(|p| p.into_inner()).shutdown();
    assert_eq!(status.hits, 1, "放行后恰发起一次");
    assert_eq!(
        status.last_hit_at_ms,
        Some(T0 + 3_600_000),
        "最近心跳命中真值"
    );
    let body = CareStore::new(dir.path()).list().expect("list");
    assert_eq!(body.history.len(), 1, "发起留痕恰一条");
    assert_eq!(
        body.last_fired("check-in"),
        Some(T0 + 3_600_000),
        "发起锚点消耗 (间隔回锚)"
    );
}

/// 计费诚实: 心跳发起的关怀回合照常计费 —— 计量落会话配额账本 (与普通回合同
/// 一本账); poll 本身零模型调用。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_heartbeat_care_turn_is_billed_on_the_shared_session_ledger() {
    let dir = due_care_dir(true);
    let clock = Arc::new(FrozenCareClock::new(T0 + 3_600_000));
    let engine = care_engine(dir.path(), Arc::clone(&clock));
    let provider = FakeProvider::new("关怀一句");
    let runtime = Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_governance(Arc::new(AllowAll))
            .with_plugin(ProviderPlugin::new(Arc::clone(&provider)))
            .with_default_model(MODEL)
            .build()
            .await
            .expect("runtime builds"),
    );
    let session_id = SessionId::new();
    let turns = Arc::new(AtomicUsize::new(0));
    let dispatch = RuntimeTurnDispatch {
        handle: tokio::runtime::Handle::current(),
        runtime: Arc::clone(&runtime),
        session_id,
        turns: Arc::clone(&turns),
        errors: Arc::new(Mutex::new(Vec::new())),
    };
    let mut producer = CareHeartbeatProducer::start_care(
        &test_config(),
        engine,
        {
            let clock = Arc::clone(&clock);
            move || clock.now_ms()
        },
        dispatch,
    )
    .expect("开关开 = 注册即起");
    wait_for("心跳命中发起", Duration::from_secs(10), || {
        turns.load(Ordering::SeqCst) >= 1
    })
    .await;
    assert!(producer.shutdown(), "停机收心跳线程");

    let stored = runtime
        .sessions()
        .load(&session_id)
        .await
        .expect("load")
        .expect("session row");
    assert_eq!(
        stored.quota.consumed_tokens, 15,
        "关怀回合照常计费: usage 10+5 记进会话配额账本 (与普通回合同一本账)"
    );
    assert_eq!(provider.call_count(), 1, "poll 零模型调用: 只有回合叫模型");
}

/// 自述关怀节扩心跳态: 下次 poll 时刻 / 最近心跳命中取心跳生产者真值;
/// 未接线 (无心跳态探测口) = 两列显式 null (不冒充在跑)。
#[tokio::test]
async fn self_status_care_section_reports_the_heartbeat_state_honestly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ticks = Arc::new(AtomicUsize::new(0));
    let turns = Arc::new(AtomicUsize::new(0));
    let handle = Arc::new(Mutex::new(
        CareHeartbeatProducer::start(
            &test_config(),
            || T0,
            QuietPoll {
                ticks: Arc::clone(&ticks),
            },
            CountingDispatch {
                turns: Arc::clone(&turns),
            },
        )
        .expect("开关开 = 注册即起"),
    ));
    wait_for("首拍", Duration::from_secs(10), || {
        handle
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .status()
            .polls
            >= 1
    })
    .await;

    let probe_dir = dir.path().to_path_buf();
    let heartbeat_probe: Arc<dyn Fn() -> CareHeartbeatStatus + Send + Sync> = {
        let handle = Arc::clone(&handle);
        Arc::new(move || handle.lock().unwrap_or_else(|p| p.into_inner()).status())
    };
    let backends = ProductionBackends {
        self_status_care: Some(care_status_probe_with_heartbeat(
            probe_dir.clone(),
            Arc::new(|| T0),
            Arc::new(|| None),
            Some(heartbeat_probe),
        )),
        ..ProductionBackends::default()
    };
    let modules = ProductionModules::build(base_config(), backends, frozen_clock()).expect("build");
    let report = invoke_self_status(&modules).await;
    let section = &report["care"];
    assert_eq!(
        section["heartbeat_next_poll_at_ms"],
        T0 + i64::try_from(TEST_POLL_INTERVAL.as_millis()).expect("interval fits i64"),
        "下次 poll 时刻 = 时钟 + 间隔 (真值)"
    );
    assert_eq!(
        section["heartbeat_last_hit_at_ms"],
        Value::Null,
        "尚无心跳命中 = 显式 null (不编)"
    );

    // 未接心跳态探测口 = 两列显式 null (0 装诚实)。
    let backends = ProductionBackends {
        self_status_care: Some(care_status_probe(
            probe_dir,
            Arc::new(|| T0),
            Arc::new(|| None),
        )),
        ..ProductionBackends::default()
    };
    let modules = ProductionModules::build(base_config(), backends, frozen_clock()).expect("build");
    let report = invoke_self_status(&modules).await;
    assert_eq!(report["care"]["heartbeat_next_poll_at_ms"], Value::Null);
    assert_eq!(report["care"]["heartbeat_last_hit_at_ms"], Value::Null);

    handle.lock().unwrap_or_else(|p| p.into_inner()).shutdown();
}
