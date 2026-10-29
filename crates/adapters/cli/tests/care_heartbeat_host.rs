//! 心跳关怀常驻宿主接线的装配级证明 (care-heartbeat-producer.md §7 接线清单 (b))。
//!
//! 走真宿主接线入口 [`CareHeartbeatSource::start`] (与常驻 gateway serve 同一
//! 注入点): 闸经 [`apeireth_organ::care::FnInitiativeGate`] 桥 presence
//! `try_take_initiative` 装配级读取口注入 [`apeireth_organ::care::CareEngine`],
//! 发起回调接 [`apeireth_runtime::canonical::Runtime::execute`] 标准回合
//! (命中 → 会话流普通转写, 照常计费), 心跳真值随 `care_status_probe_with_heartbeat`
//! 进自述关怀节 (两列 null → 真值)。
//!
//! 口径: **双闸齐开** (`APEIRETH_ENABLE_CARE` + `APEIRETH_ENABLE_CARE_HEARTBEAT`)
//! 才起 (注册即闸 fail-closed, 生效值走纯函数 `from_env_value` 不吃全局 env);
//! 关 = 零动作零线程零会话, 自述两列显式 null (不冒充在跑)。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_cli::{
    build_canonical_runtime_with_sessions_and_voice_emotion_from_env, CareHeartbeatSource,
};
use apeireth_core::kernel::{
    CapabilityId, Clock, ModelId, PluginId, SessionId, Timestamp, VirtualClock,
};
use apeireth_gateway::{EventBus, PresenceService};
use apeireth_governance::AllowAll;
use apeireth_organ::care::{CareDiscipline, CareRule, CareStore, CareTrigger};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ProviderCapability,
    ProviderError, ToolCapability,
};
use apeireth_protocol::canonical::{
    ContentPart, ModelDescriptor, ModelFeature, NormalizedRequest, NormalizedResponse,
    NormalizedUsage, ToolCall,
};
use apeireth_runtime::canonical::Runtime;
use apeireth_runtime_assembly::CareHeartbeatConfig;
use serde_json::Value;

const MODEL: &str = "fake-model-care-heartbeat-host";

/// 心跳拍测试口径: 1 秒一拍 (生效值解析的最小正整数秒档)。
fn one_second() -> Duration {
    Duration::from_secs(1)
}

fn now_ms() -> i64 {
    Timestamp::now().epoch_millis()
}

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
            id: "resp_care_hb_host".to_string(),
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

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_786_838_400_000)
            .unwrap()
            .as_datetime(),
    ))
}

async fn fake_runtime(provider: Arc<FakeProvider>) -> Arc<Runtime> {
    Arc::new(
        Runtime::builder()
            .with_clock(frozen_clock())
            .with_governance(Arc::new(AllowAll))
            .with_plugin(ProviderPlugin::new(provider))
            .with_default_model(MODEL)
            .build_agent_runtime(apeireth_runtime::AgentHostPolicy::default())
            .await
            .expect("managed host builds")
            .into_main_runtime(apeireth_plugin::agents::AgentConfig::default())
            .await
            .expect("managed main builds"),
    )
}

/// 到点关怀规则面: 间隔 1 小时、创建于 2 小时前 (真时时钟下立即到点) +
/// 显式关勿扰窗口 (跑测时刻不漂移)。
fn due_care_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = CareStore::new(dir.path());
    store
        .add_rule(CareRule {
            id: "check-in".to_string(),
            trigger: CareTrigger::Interval { hours: 1 },
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: now_ms() - 2 * 3_600_000,
        })
        .expect("add rule");
    let mut body = store.list().expect("list");
    body.discipline = CareDiscipline {
        quiet_start_minutes: None,
        quiet_end_minutes: None,
        ..CareDiscipline::default()
    };
    store.save(&body).expect("save discipline");
    dir
}

fn wait_for(what: &str, deadline: Duration, mut cond: impl FnMut() -> bool) {
    let until = std::time::Instant::now() + deadline;
    while std::time::Instant::now() < until {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(cond(), "超时未等到: {what}");
}

/// 真调 `tool.self_status` 取关怀节 (与生产同一条投影路径)。
async fn care_section_of(runtime: &Runtime) -> Value {
    let self_status = runtime
        .capability_registry()
        .entries()
        .into_iter()
        .map(|(_owner, capability)| capability)
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .expect("tool.self_status");
    let call = ToolCall {
        id: "call_status".into(),
        name: "self_status".into(),
        arguments: serde_json::json!({}),
    };
    let result: apeireth_protocol::canonical::ToolResult = self_status.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).expect("self report json");
    report["care"].clone()
}

/// 宿主接线两态 · 开: 双闸齐开 = 心跳真自动跑, 命中 → 标准回合进会话流
/// (最小 E2E, 真 Runtime 真 presence 真关怀内核) + poll 零模型调用 + 照常计费。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_resident_host_runs_the_heartbeat_when_both_gates_are_open() {
    let dir = due_care_dir();
    let provider = FakeProvider::new("最近挺好, 别担心~");
    let runtime = fake_runtime(Arc::clone(&provider)).await;
    let presence = PresenceService::new(EventBus::new(8));
    let source = CareHeartbeatSource::new();
    let session = SessionId::new();

    // 双闸齐开 (CARE + CARE_HEARTBEAT) 的生效值 (纯函数解析, 不吃全局 env)。
    let config = CareHeartbeatConfig::from_env_value(Some("1"), Some("1"), Some("1"))
        .expect("双闸齐开 + 1 秒档合法");
    let started = source
        .start(
            &config,
            Arc::clone(&runtime),
            &presence,
            dir.path().to_path_buf(),
            session,
        )
        .expect("宿主接线成功");
    assert!(started, "双闸齐开 = 心跳真起 (注册即起)");
    assert!(source.is_running(), "常驻心跳线程在跑");

    // 命中 → 标准回合进会话流 (关怀回合输入 + 生成的关怀消息都进普通转写)。
    let until = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        let row = runtime
            .sessions()
            .load_or_create(session)
            .await
            .expect("session");
        let texts: Vec<String> = row
            .messages
            .iter()
            .map(|message| ContentPart::join_text(&message.content))
            .collect();
        if texts.iter().any(|text| text.contains("[care:check-in]"))
            && texts.iter().any(|text| text.contains("最近挺好, 别担心~"))
        {
            break;
        }
        assert!(
            std::time::Instant::now() < until,
            "超时未等到关怀回合进会话流: {texts:?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // 计费诚实: poll 零模型调用 —— 模型调用数 = 标准回合数 = 1; 关怀回合
    // 照常计费 (usage 10+5 落会话配额账本, 无免费通道)。
    assert_eq!(provider.call_count(), 1, "poll 零模型调用: 只有回合叫模型");
    let stored = runtime
        .sessions()
        .load(&session)
        .await
        .expect("load")
        .expect("session row");
    assert_eq!(stored.quota.consumed_tokens, 15, "关怀回合照常计费");
    let main = runtime.managed_main().expect("producer uses managed main");
    assert_eq!(runtime.tool_dispatcher().agent(session), main.id);
    assert_eq!(main.usage.requests, 1);
    assert!(runtime
        .agent_runtime()
        .unwrap()
        .requests(main.id)
        .unwrap()
        .iter()
        .all(|fact| fact.agent == main.id && fact.owner == main.owner));
    assert!(
        source.dispatch_errors().is_empty(),
        "关怀回合无错: {:?}",
        source.dispatch_errors()
    );

    // 自述两列真值口: 命中时刻真值 + 下次 poll 时刻真值 (时钟 + 间隔)。
    let status = source.status();
    assert_eq!(status.hits, 1, "恰发起一次 (间隔规则回锚)");
    assert!(status.last_hit_at_ms.is_some(), "最近心跳命中真值");
    assert!(status.next_poll_at_ms.is_some(), "下次 poll 时刻真值");

    // 已起再起 = 拒绝即帧 (不双开)。
    assert!(source
        .start(
            &config,
            Arc::clone(&runtime),
            &presence,
            dir.path().to_path_buf(),
            session,
        )
        .is_err());

    // 优雅停机: join 收线程 (不僵尸), 停机后零 poll (无幽灵 tick), 幂等。
    assert!(source.shutdown(), "第一次停机真收掉线程");
    assert!(!source.is_running(), "停机后不僵尸");
    assert!(source.thread_reaped(), "线程句柄无泄漏 (join 已回收)");
    let polls_frozen_at = source.status().polls;
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        source.status().polls,
        polls_frozen_at,
        "停机后零 poll (无幽灵 tick)"
    );
    assert!(!source.shutdown(), "停机幂等");
}

/// 宿主接线两态 · 关: 双闸未齐开 = 零动作 (不建线程不 poll 零回合零会话),
/// 自述两列显式 null (未起不冒充在跑)。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_resident_host_stays_quiet_when_the_gates_are_not_both_open() {
    let dir = due_care_dir();
    let provider = FakeProvider::new("不该出现");
    let runtime = fake_runtime(Arc::clone(&provider)).await;
    let presence = PresenceService::new(EventBus::new(8));
    let source = CareHeartbeatSource::new();
    let session = SessionId::new();

    for (care, heartbeat) in [
        (None, None),
        (Some("1"), None),
        (None, Some("1")),
        (Some("1"), Some("0")),
        (Some("1"), Some("true")),
    ] {
        let config = CareHeartbeatConfig::from_env_value(care, heartbeat, None).expect("合法解析");
        assert!(!config.enabled, "双闸未齐开 = 关 (fail-closed)");
        let started = source
            .start(
                &config,
                Arc::clone(&runtime),
                &presence,
                dir.path().to_path_buf(),
                session,
            )
            .expect("接线");
        assert!(!started, "开关关 = 不注册 (零动作)");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!source.is_running(), "零动作: 无心跳线程");
    assert!(source.thread_reaped(), "未起 = 无线程 = 无泄漏");
    assert_eq!(source.status().polls, 0, "零动作: 零 poll");
    assert_eq!(source.status().hits, 0, "零动作: 零命中");
    assert_eq!(provider.call_count(), 0, "零动作零模型调用零计费");
    assert!(
        !dir.path().join("ritual_state.json").exists(),
        "零动作: 仪式状态存储零读写 (不建文件)"
    );
    assert!(
        !source.has_ritual_state_store(),
        "零动作: 仪式状态存储未绑定 (注入口 fail-closed)"
    );
    assert!(
        runtime
            .sessions()
            .load(&session)
            .await
            .expect("load")
            .is_none(),
        "零动作 = 零会话零账本消耗"
    );

    // 自述关怀节心跳两列显式 null (未起 = 不冒充在跑)。
    let state = (source.status_probe())();
    assert_eq!(state.next_poll_at_ms, None, "未起 = 下次 poll 显式 null");
    assert_eq!(state.last_hit_at_ms, None, "未起 = 命中时刻显式 null");
    assert!(!source.shutdown(), "未起 = 无可收线程");
}

/// 自述两列真值 (生产路径): 生产装配的关怀节探针绑同一心跳真值口 ——
/// 未起 = 两列显式 null; 起后 = 下次 poll / 最近心跳命中转真值; 停机后
/// 「下次 poll」转回 null (未运行不冒充在跑), 命中历史真值不抹。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_self_status_care_section_reports_the_live_heartbeat_columns() {
    // std::env 是进程全局 (本二进制内串锁), 数据面钉临时目录。
    static ENV_LOCK: Mutex<()> = Mutex::new(());
    let _lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let data_dir = tempfile::tempdir().expect("data dir");
    let session_db = tempfile::tempdir().expect("session db dir");
    let cognitive_db = tempfile::tempdir().expect("cognitive db dir");
    std::env::set_var("APEIRETH_DATA_DIR", data_dir.path());
    std::env::set_var(
        "APEIRETH_SESSION_DB",
        session_db.path().join("sessions.sqlite3"),
    );
    std::env::set_var(
        "APEIRETH_COGNITIVE_DB",
        cognitive_db.path().join("cognitive.sqlite3"),
    );

    let (runtime, _sessions, _memory, _policy, _guard, _voice, care_heartbeat, _cost_router) =
        build_canonical_runtime_with_sessions_and_voice_emotion_from_env()
            .await
            .expect("keyless 生产装配");
    let runtime = Arc::new(runtime);

    // 未起 = 两列显式 null (生产装配不冒充在跑)。
    let section = care_section_of(&runtime).await;
    assert_eq!(
        section["heartbeat_next_poll_at_ms"],
        Value::Null,
        "未起 = 下次 poll 显式 null"
    );
    assert_eq!(
        section["heartbeat_last_hit_at_ms"],
        Value::Null,
        "未起 = 命中时刻显式 null"
    );

    // 到点规则 + 双闸齐开起心跳 (真宿主接线入口)。
    let store = CareStore::new(data_dir.path());
    store
        .add_rule(CareRule {
            id: "check-in".to_string(),
            trigger: CareTrigger::Interval { hours: 1 },
            material: "问问他最近怎么样".to_string(),
            enabled: true,
            created_at_ms: now_ms() - 2 * 3_600_000,
        })
        .expect("add rule");
    let mut body = store.list().expect("list");
    body.discipline = CareDiscipline {
        quiet_start_minutes: None,
        quiet_end_minutes: None,
        ..CareDiscipline::default()
    };
    store.save(&body).expect("save discipline");

    let presence = PresenceService::new(EventBus::new(8));
    let config = CareHeartbeatConfig {
        enabled: true,
        poll_interval: one_second(),
    };
    let started = care_heartbeat
        .start(
            &config,
            Arc::clone(&runtime),
            &presence,
            data_dir.path().to_path_buf(),
            SessionId::new(),
        )
        .expect("宿主接线成功");
    assert!(started, "双闸齐开 = 心跳真起");

    wait_for("心跳命中留痕", Duration::from_secs(15), || {
        care_heartbeat.status().last_hit_at_ms.is_some()
    });

    // 自述两列 = 生产者真值 (同源投影, 不编数)。
    let section = care_section_of(&runtime).await;
    let state = (care_heartbeat.status_probe())();
    assert_eq!(
        section["heartbeat_next_poll_at_ms"],
        Value::from(state.next_poll_at_ms),
        "下次 poll 时刻 = 心跳真值口同一真值"
    );
    assert_eq!(
        section["heartbeat_last_hit_at_ms"],
        Value::from(state.last_hit_at_ms),
        "最近心跳命中 = 心跳真值口同一真值"
    );
    assert!(
        section["heartbeat_next_poll_at_ms"].as_i64().is_some(),
        "下次 poll 时刻转真值: {section}"
    );
    assert!(
        section["heartbeat_last_hit_at_ms"].as_i64().is_some(),
        "最近心跳命中转真值: {section}"
    );
    assert_eq!(
        section["heartbeat_last_hit_at_ms"],
        Value::from(care_heartbeat.status().last_hit_at_ms),
        "自述两列与诊断口同一真值"
    );

    // 停机后: 「下次 poll」转回 null (未运行不冒充在跑), 命中历史真值不抹。
    care_heartbeat.shutdown();
    let section = care_section_of(&runtime).await;
    assert_eq!(
        section["heartbeat_next_poll_at_ms"],
        Value::Null,
        "未运行 = 下次 poll 显式 null (不冒充在跑)"
    );
    assert!(
        section["heartbeat_last_hit_at_ms"].as_i64().is_some(),
        "命中历史真值不抹: {section}"
    );
}

/// 闸同源: 宿主注入 CareEngine 的发起闸就是 presence `try_take_initiative`
/// 装配级读取口 (同一份防唠叨预算) —— 预算耗尽 = Held 拦下不丢候选零回合。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_care_initiative_gate_on_the_heartbeat_path_is_the_presence_gate() {
    let dir = due_care_dir();
    let provider = FakeProvider::new("不该出现");
    let runtime = fake_runtime(Arc::clone(&provider)).await;
    let presence = PresenceService::new(EventBus::new(8));
    let source = CareHeartbeatSource::new();
    let session = SessionId::new();

    // 把 presence 防唠叨预算花光 (同源闸: 与其他 initiative 生产者同一本账)。
    let now = now_ms();
    let mut admitted = 0;
    while presence.try_take_initiative(now) {
        admitted += 1;
        assert!(admitted <= 8, "预算上限必须有限");
    }
    assert!(admitted > 0, "预算先放行过若干次 (限量是真实存在的)");

    let config = CareHeartbeatConfig {
        enabled: true,
        poll_interval: Duration::from_millis(200),
    };
    let started = source
        .start(
            &config,
            Arc::clone(&runtime),
            &presence,
            dir.path().to_path_buf(),
            session,
        )
        .expect("宿主接线成功");
    assert!(started);

    wait_for("若干拍 poll", Duration::from_secs(15), || {
        source.status().polls >= 3
    });
    let status = source.status();
    source.shutdown();

    // 闸拒绝 = Held 拦下: 零命中零回合零模型调用, 候选不丢 (不写台账不耗锚点)。
    assert_eq!(status.hits, 0, "闸拒绝 = 不发起 (候选不丢重试)");
    assert_eq!(provider.call_count(), 0, "闸拒绝 = 零回合零模型调用零计费");
    let body = CareStore::new(dir.path()).list().expect("list");
    assert!(
        body.history.is_empty(),
        "闸拦下不写台账: {:?}",
        body.history
    );
    assert!(body.fired_at.is_empty(), "候选不丢: {:?}", body.fired_at);
    assert!(source.dispatch_errors().is_empty(), "零回合 = 发起缝零错误");
}
