//! W3.2 生物机制族「自述面只读聚合」的装配级证明。
//!
//! 钉住：
//! ① 21 节冻结合同不变 —— `bio_mechanisms` 只挂在既有 `capabilities` 节内，
//!   **不是**新的顶层节；
//! ② 未挂观测面 = 子对象缺省（旧帧形状逐键不变，兼容旧消费者）；
//! ③ 默认关：开关只认 `APEIRETH_ENABLE_BIO_SELF_STATUS="1"`，关时整节
//!   `mechanisms: null` + 原因「未启用」，一个 probe 都不调用；
//! ④ 开关开且注册 probe 才采值；未注册 = `wired:false` + 原因「未接线」；
//!   probe 读失败 = 该格 `null` + 真因，不整帧失败；
//! ⑤ 至少一条 **已有运行时数据源** 的只读值接上：承载力（SG-26）读运行时
//!   同一个任务队列的分态计数真值（与 `tool.task_board` 同队同账）；
//! ⑥ 确定性：注入时钟后同输入同输出。
//!
//! W3.5 观测扩展（同文件扩面）：
//! ⑦ 免疫记忆（SG-15）/ 审议门（SG-24）/ 关怀心跳（SG-19）/ 练习账（SG-21）/
//!   痛觉（SG-16）/ 再生（SG-17）六条 probe 的真值、脱敏与失败边界；
//! ⑧ 便捷注册口存在 ≠ 启用：关时零 probe 调用；未注册的格仍 `wired:false`；
//! ⑨ 21 节合同与帧形状不变，扩展后的 bio 子对象 serde 往返稳定、采集确定性。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::{Clock, Timestamp};
use apeireth_guard::hook::SharedImmuneMemory;
use apeireth_guard::immune_memory::{ImmuneMemory, IncidentKind, SeverityTier};
use apeireth_orchestration::nociception::{FailureClass, Habituation, Recommendation};
use apeireth_orchestration::quorum::{QuorumDecision, QuorumEscalation};
use apeireth_orchestration::task_queue::{
    EnqueueOutcome, TaskCaller, TaskQueue, TaskQueueConfig, TaskSpec, TaskStore,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime::canonical::{
    BioPainObservation, BioPainSnapshot, BioRecoveryPhaseCounts, BioRecoverySnapshot,
};
use apeireth_runtime_assembly::canonical::orchestrator::QuorumTrace;
use apeireth_runtime_assembly::canonical::self_status_source::{
    capacity_bio_probe, care_heartbeat_probe, immune_quorum_probe, pain_probe, play_probe,
    quorum_trace_probe, recovery_probe, BioMechanismRegistry, PainSnapshotReader,
    ProductionSelfStatusSource,
};
use apeireth_runtime_assembly::canonical::{
    bio_play_wiring_from_env_value, CareHeartbeatStatus, PLAY_ABORT_CAPABILITY_ID,
    PLAY_ABORT_TOOL_NAME, PLAY_FINISH_CAPABILITY_ID, PLAY_FINISH_TOOL_NAME,
    PLAY_PROPOSE_CAPABILITY_ID, PLAY_PROPOSE_TOOL_NAME, PLAY_START_CAPABILITY_ID,
    PLAY_START_TOOL_NAME,
};
use apeireth_runtime_assembly::{task_board_status_probe, SqliteTaskStore};
use apeireth_tools_canonical::self_status::{
    bio_self_status_enabled_from_env_value, BioMechanismKind, BioMechanismProbe,
    BioMechanismReading, BioMechanismsStatus, BIO_SELF_STATUS_ENABLE_ENV, BIO_STAGE_NOT_WIRED,
    BIO_STAGE_WIRED, CAPABILITIES_BIO_MECHANISMS, SELF_STATUS_SECTIONS,
};
use apeireth_tools_canonical::SelfStatusTool;
use serde_json::{json, Value};

fn call() -> ToolCall {
    ToolCall {
        id: "call_bio_self_status".into(),
        name: "self_status".into(),
        arguments: json!({}),
    }
}

/// 走真实工具入口（`spawn_blocking` 采集 + 结构化帧）取一份自述 JSON。
async fn report(source: ProductionSelfStatusSource) -> Value {
    let tool = SelfStatusTool::new(Arc::new(source));
    let result = tool.invoke(&call()).await;
    assert!(result.is_ok(), "整帧必须成功: {}", result.render());
    serde_json::from_str(&result.render()).expect("structured self-report json")
}

fn fixed_clock() -> Arc<dyn Fn() -> i64 + Send + Sync> {
    Arc::new(|| 1_728_000_000_123)
}

/// 与生产装配根同款的最小来源：一条名册行（`organs` 照实 false）。
fn production_source() -> ProductionSelfStatusSource {
    ProductionSelfStatusSource::new(
        vec![apeireth_tools_canonical::CapabilitySwitch::new(
            "organs", false,
        )],
        false,
        None,
    )
}

/// 真任务队列（内存 sqlite 存储）：与 `tool.task_board` 装配级测试同一条装配路径。
fn queue() -> Arc<TaskQueue> {
    let store: Arc<dyn TaskStore> =
        Arc::new(SqliteTaskStore::in_memory().expect("in-memory task store"));
    Arc::new(TaskQueue::open(store, TaskQueueConfig::default()).expect("queue opens"))
}

struct FixedProbe;

impl BioMechanismProbe for FixedProbe {
    fn snapshot(&self, now_ms: i64) -> Result<BioMechanismReading, String> {
        Ok(BioMechanismReading::wired(
            Some(json!({ "now": now_ms })),
            Some(6.0),
            Some("最近一次仪式候选".to_string()),
        ))
    }
}

struct BrokenProbe;

impl BioMechanismProbe for BrokenProbe {
    fn snapshot(&self, _now_ms: i64) -> Result<BioMechanismReading, String> {
        Err("task queue read failed: boom".to_string())
    }
}

/// ① 21 节冻结合同不变：顶层键集 == 冻结节表，生物聚合只在 `capabilities` 内，
/// 且帧仍过 `OutputSchema`。
#[tokio::test]
async fn the_frozen_twenty_one_sections_survive_the_bio_aggregate() {
    assert_eq!(SELF_STATUS_SECTIONS.len(), 21, "冻结合同仍是二十一节");
    let report = report(production_source().with_bio_self_status(true)).await;
    let top = report.as_object().expect("object frame");
    let rendered: std::collections::BTreeSet<&str> = top.keys().map(String::as_str).collect();
    let frozen: std::collections::BTreeSet<&str> = SELF_STATUS_SECTIONS.iter().copied().collect();
    assert_eq!(rendered, frozen, "生物聚合不得新增/删除顶层节");
    assert!(
        top.get(CAPABILITIES_BIO_MECHANISMS).is_none(),
        "bio_mechanisms 不是顶层节"
    );
    assert!(
        report["capabilities"][CAPABILITIES_BIO_MECHANISMS].is_object(),
        "子对象必须挂在 capabilities 节内: {report}"
    );
    assert!(SelfStatusTool::output_schema().validate(&report).is_ok());
    // 既有名册行语义不变：老消费者按名取布尔仍照旧。
    assert_eq!(report["capabilities"]["organs"], false);
    assert_eq!(report["identity"]["product_name"], "apeireth");
}

/// ② 未挂观测面 = 旧帧形状不变：capabilities 逐键仍是布尔，一个键不多
/// （既有 e2e「名册全为布尔」断言不被本批破）。
#[tokio::test]
async fn a_source_without_the_bio_face_keeps_the_legacy_capabilities_shape() {
    let report = report(ProductionSelfStatusSource::new(Vec::new(), false, None)).await;
    let capabilities = report["capabilities"].as_object().expect("roster object");
    assert!(
        capabilities.get(CAPABILITIES_BIO_MECHANISMS).is_none(),
        "未挂观测面时不得出现子对象"
    );
    for (name, value) in capabilities {
        assert!(value.is_boolean(), "{name} 必须仍是布尔: {value}");
    }
    assert!(SelfStatusTool::output_schema().validate(&report).is_ok());
}

/// ③ 默认关：观测面已挂 + 已注册 probe，但开关关 = `mechanisms: null` +
/// 原因「未启用」，一个 probe 都不调用（0 装诚实：没采就是没采，不写 0）。
#[tokio::test]
async fn switched_off_keeps_mechanisms_null_with_the_not_enabled_reason() {
    struct CountingProbe(Arc<AtomicUsize>);

    impl BioMechanismProbe for CountingProbe {
        fn snapshot(&self, _now_ms: i64) -> Result<BioMechanismReading, String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(BioMechanismReading::wired(Some(json!(1)), None, None))
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let mut source =
        ProductionSelfStatusSource::new(Vec::new(), false, None).with_bio_self_status(false);
    source.register_bio_probe(
        BioMechanismKind::Capacity,
        Arc::new(CountingProbe(Arc::clone(&calls))),
    );
    let report = report(source).await;
    let bio = &report["capabilities"][CAPABILITIES_BIO_MECHANISMS];
    assert_eq!(bio["enabled"], false);
    assert_eq!(bio["mechanisms"], Value::Null, "关时一格也不采");
    let reason = bio["reason"].as_str().expect("reason");
    assert!(reason.contains("未启用"), "{reason}");
    assert!(reason.contains(BIO_SELF_STATUS_ENABLE_ENV), "{reason}");
    assert_eq!(calls.load(Ordering::SeqCst), 0, "关时不许调用 probe");
    assert!(SelfStatusTool::output_schema().validate(&report).is_ok());
}

/// ③b env 开关惯例：只认 "1"（纯函数，不碰进程 env）。
#[test]
fn the_bio_switch_only_accepts_explicit_one() {
    assert!(!bio_self_status_enabled_from_env_value(None));
    assert!(bio_self_status_enabled_from_env_value(Some("1")));
    for raw in ["0", "true", "on", "yes", "", "01"] {
        assert!(
            !bio_self_status_enabled_from_env_value(Some(raw)),
            "{raw:?} 必须为关"
        );
    }
}

/// ④ 开关开但未注册 probe：14 条机制全部 `wired:false` + 原因「未接线」，
/// 值/阈值/最近动作全 null（不许编数值）。
#[tokio::test]
async fn enabled_without_probes_reports_fourteen_not_wired_rows() {
    let report =
        report(ProductionSelfStatusSource::new(Vec::new(), false, None).with_bio_self_status(true))
            .await;
    let bio = &report["capabilities"][CAPABILITIES_BIO_MECHANISMS];
    assert_eq!(bio["enabled"], true);
    assert!(bio["reason"].is_null());
    let rows = bio["mechanisms"].as_object().expect("mechanisms object");
    assert_eq!(rows.len(), BioMechanismKind::ALL.len(), "14 格必须逐格在场");
    for kind in BioMechanismKind::ALL {
        let row = &rows[kind.as_str()];
        assert_eq!(row["stage"], BIO_STAGE_NOT_WIRED, "{kind:?}");
        assert_eq!(row["wired"], false, "{kind:?}");
        assert_eq!(row["value"], Value::Null, "{kind:?}");
        assert_eq!(row["threshold"], Value::Null, "{kind:?}");
        assert_eq!(row["last_action"], Value::Null, "{kind:?}");
        assert!(
            row["reason"].as_str().unwrap().contains("未接线"),
            "{kind:?}: {row}"
        );
    }
}

/// ④b 注册 probe + 开关开：该格出现真值/阈值/最近动作，其余 13 格如实未接线。
#[tokio::test]
async fn a_registered_probe_feeds_value_threshold_and_last_action() {
    let mut source = ProductionSelfStatusSource::new(Vec::new(), false, None)
        .with_bio_mechanisms(BioMechanismRegistry::new(true).with_now_ms(fixed_clock()));
    source.register_bio_probe(BioMechanismKind::Ritual, Arc::new(FixedProbe));
    let report = report(source).await;
    let rows = report["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"]
        .as_object()
        .expect("mechanisms object");
    let ritual = &rows["ritual"];
    assert_eq!(ritual["stage"], BIO_STAGE_WIRED);
    assert_eq!(ritual["wired"], true);
    assert_eq!(ritual["value"]["now"], 1_728_000_000_123i64);
    assert_eq!(ritual["threshold"], 6.0);
    assert_eq!(ritual["last_action"], "最近一次仪式候选");
    assert!(ritual["reason"].is_null());
    assert_eq!(rows["capacity"]["wired"], false, "未注册的格绝不放值");
}

/// ④c probe 读失败 = 该格 null + 真因，整帧仍成功、其余节不受牵连。
#[tokio::test]
async fn a_failing_probe_is_null_with_a_reason_not_a_failed_frame() {
    let mut source =
        ProductionSelfStatusSource::new(Vec::new(), false, None).with_bio_self_status(true);
    source.register_bio_probe(BioMechanismKind::Nociception, Arc::new(BrokenProbe));
    let report = report(source).await;
    let row = &report["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"]["nociception"];
    assert_eq!(row["value"], Value::Null);
    assert_eq!(row["threshold"], Value::Null);
    assert_eq!(row["last_action"], Value::Null);
    assert_eq!(row["wired"], true, "已注册 = 接线事实成立");
    assert!(
        row["reason"]
            .as_str()
            .unwrap()
            .contains("task queue read failed: boom"),
        "{row}"
    );
    // 不整帧失败：其余节照常。
    assert_eq!(report["identity"]["product_name"], "apeireth");
    assert!(report["memory_ledger"].is_object());
    assert!(SelfStatusTool::output_schema().validate(&report).is_ok());
}

/// ⑥ 确定性：注入固定时钟 → probe 收到同一时刻，同输入两次渲染逐字节一致。
#[tokio::test]
async fn the_injected_clock_reaches_the_probe_and_render_is_deterministic() {
    let registry = BioMechanismRegistry::new(true).with_now_ms(fixed_clock());
    let mut source =
        ProductionSelfStatusSource::new(Vec::new(), false, None).with_bio_mechanisms(registry);
    source.register_bio_probe(BioMechanismKind::Ritual, Arc::new(FixedProbe));
    let first = report(source).await;
    let mut source = ProductionSelfStatusSource::new(Vec::new(), false, None)
        .with_bio_mechanisms(BioMechanismRegistry::new(true).with_now_ms(fixed_clock()));
    source.register_bio_probe(BioMechanismKind::Ritual, Arc::new(FixedProbe));
    let second = report(source).await;
    assert_eq!(
        first["capabilities"][CAPABILITIES_BIO_MECHANISMS],
        second["capabilities"][CAPABILITIES_BIO_MECHANISMS],
        "同输入必须同输出"
    );
    assert_eq!(
        first["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"]["ritual"]["value"]["now"],
        1_728_000_000_123i64
    );
}

/// ⑤ 已接 1 个**已有运行时数据源**的只读值：承载力（SG-26）读运行时同一个
/// 任务队列的分态计数真值（与 `tool.task_board` 同队同账），经自述帧上报。
#[tokio::test]
async fn the_capacity_probe_reads_the_live_task_queue_through_the_frame() {
    let queue = queue();
    let assigned = match queue
        .enqueue(TaskSpec::new("在跑的一件", json!({})).with_assignee("worker-a"))
        .expect("enqueue")
    {
        EnqueueOutcome::Enqueued { task, .. } => task,
        other => panic!("unexpected enqueue outcome: {other:?}"),
    };
    queue
        .begin_execution(&assigned.id, &TaskCaller::agent("worker-a"))
        .expect("begin execution");
    queue
        .enqueue(TaskSpec::new("待派的一件", json!({})))
        .expect("enqueue");

    let mut source = ProductionSelfStatusSource::new(Vec::new(), false, None)
        .with_bio_mechanisms(BioMechanismRegistry::new(true).with_now_ms(fixed_clock()))
        // 任务节与承载力格同一队列同源：两个探测口绑同一个 `TaskQueue` 实例。
        .with_task_board_probe(task_board_status_probe(Arc::clone(&queue)));
    source.register_bio_probe(
        BioMechanismKind::Capacity,
        capacity_bio_probe(Arc::clone(&queue), Some(8)),
    );
    let report = report(source).await;
    let row = &report["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"]["capacity"];
    assert_eq!(row["stage"], BIO_STAGE_WIRED);
    assert_eq!(row["wired"], true);
    assert_eq!(row["threshold"], 8.0, "注入的并发上限进阈值");
    assert_eq!(row["value"]["active_tasks"], 1, "assigned 0 + running 1");
    assert_eq!(row["value"]["running"], 1);
    assert_eq!(row["value"]["pending"], 1);
    assert_eq!(row["value"]["observed_at_ms"], 1_728_000_000_123i64);
    assert!(row["reason"].is_null());
    // 同一次装配的任务节与承载力格同源：任务节也报同一队列真值。
    assert_eq!(report["task_board"]["pending"], 1);
    assert_eq!(report["task_board"]["running"], 1);
    assert_eq!(report["task_board"]["reason"], Value::Null);
}

/// ⑤b 队列读失败（探测口级）：承载力格 null + 真因，不整帧失败。
#[tokio::test]
async fn a_failing_capacity_queue_read_stays_honest() {
    // 用已关闭的 sqlite 连接没法简单造坏队列；改由 probe 层失败路径证明
    // （注册表统一把 Err 折成 null + 原因），此处用同一条 BrokenProbe 覆盖
    // 「已有数据源读失败」的帧级行为。
    let mut source =
        ProductionSelfStatusSource::new(Vec::new(), false, None).with_bio_self_status(true);
    source.register_bio_probe(BioMechanismKind::Capacity, Arc::new(BrokenProbe));
    let report = report(source).await;
    let row = &report["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"]["capacity"];
    assert_eq!(row["value"], Value::Null);
    assert_eq!(row["wired"], true);
    assert!(row["reason"].as_str().unwrap().contains("boom"));
}

// ============================================
// W3.5 观测扩展: 六条真实运行时数据源 probe 的装配级证明
// ============================================

fn fixed_virtual_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_123)
            .expect("valid fixed timestamp")
            .as_datetime(),
    ))
}

fn play_tool_call(name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: format!("call_{name}"),
        name: name.to_string(),
        arguments,
    }
}

async fn invoke_play_tool(tool: &Arc<dyn ToolCapability>, name: &str, arguments: Value) -> Value {
    let result = tool.invoke(&play_tool_call(name, arguments)).await;
    assert!(result.is_ok(), "{name} 必须成功: {}", result.render());
    serde_json::from_str(&result.render()).expect("structured play frame")
}

fn find_play_capability<'a>(
    capabilities: &'a [Arc<dyn ToolCapability>],
    id: &str,
) -> &'a Arc<dyn ToolCapability> {
    capabilities
        .iter()
        .find(|capability| capability.id().as_str() == id)
        .expect("play capability must be registered")
}

fn play_candidates() -> Value {
    json!({
        "candidates": [{
            "topic": "预演一次捕猎路线",
            "skills": ["拆解", "复盘"],
            "risk": "sandboxed"
        }]
    })
}

/// 一份有真实记录的共享免疫账: 一条高严重度新签名 (= 升级记录) + 一条
/// 已形成适应记忆的同类事件 (观察 >= 2 次)。含路径样原文, 供脱敏断言。
fn immune_memory_with_records() -> SharedImmuneMemory {
    let memory: SharedImmuneMemory =
        Arc::new(parking_lot::Mutex::new(ImmuneMemory::with_capacity(8)));
    {
        let mut guard = memory.lock();
        guard.observe(
            IncidentKind::CredentialLeak,
            "C:\\Users\\owner\\token.txt",
            SeverityTier::Tier2Guard,
            false,
            1_000,
        );
        guard.observe(
            IncidentKind::ToolPoison,
            "tool descriptor",
            SeverityTier::Tier1Note,
            false,
            2_000,
        );
        guard.observe(
            IncidentKind::ToolPoison,
            "tool descriptor",
            SeverityTier::Tier1Note,
            false,
            3_000,
        );
    }
    memory
}

/// 一份带痛觉观测与再生案例计数的运行时快照 (纯数据, 由迟绑定读取口返回)。
fn pain_snapshot() -> BioPainSnapshot {
    BioPainSnapshot {
        enabled: true,
        assessed: 3,
        repaired: 1,
        observations: vec![BioPainObservation {
            capability: "tool.web_fetch".to_string(),
            cause_key: "transient:0123456789abcdef".to_string(),
            class: FailureClass::Transient,
            at_ms: 700,
            habituation: Habituation::Sensitized,
            chronic: true,
            recommended: Recommendation::Diagnose,
            rationale: "chronic same cause".to_string(),
        }],
        groups: Vec::new(),
        recovery: BioRecoverySnapshot {
            active_cases: 3,
            closed_cases: 2,
            rejected_transitions: 1,
            phases: BioRecoveryPhaseCounts {
                hemostasis: 1,
                inflammation: 1,
                proliferation: 1,
                remodeling: 0,
                resolved: 0,
            },
            cases: Vec::new(),
        },
    }
}

fn pain_reader(snapshot: BioPainSnapshot) -> PainSnapshotReader {
    Arc::new(move || Ok(snapshot.clone()))
}

/// ⑦a 免疫记忆 probe: 读同一份共享账的真值 (总数/未收口/适应模式/升级数/
/// 类别分布/首末时刻), 空账全零不冒充未接线。
#[test]
fn the_immune_probe_reports_live_shared_memory_counts() {
    let memory = immune_memory_with_records();
    let reading = immune_quorum_probe(Arc::clone(&memory))
        .snapshot(9_999)
        .expect("live immune ledger");
    assert!(reading.wired);
    let value = reading.value.expect("value");
    assert_eq!(value["records"], 2);
    assert_eq!(value["open"], 2);
    assert_eq!(value["resolved"], 0);
    assert_eq!(value["known_patterns"], 1, "同签名观察 >= 2 = 适应记忆");
    assert_eq!(value["escalations"], 1, "高严重度新签名 = 升级记录");
    assert_eq!(value["kinds"]["credential_leak"], 1);
    assert_eq!(value["kinds"]["tool_poison"], 1);
    assert_eq!(value["first_seen_ms"], 1_000);
    assert_eq!(value["last_seen_ms"], 3_000);
    assert_eq!(value["observed_at_ms"], 9_999);

    // 空账: 全零计数 = 实况 (不是未接线, 也不是编造)。
    let empty: SharedImmuneMemory =
        Arc::new(parking_lot::Mutex::new(ImmuneMemory::with_capacity(8)));
    let reading = immune_quorum_probe(empty)
        .snapshot(1)
        .expect("empty immune ledger");
    let value = reading.value.expect("value");
    assert_eq!(value["records"], 0);
    assert_eq!(value["first_seen_ms"], Value::Null);
    assert_eq!(value["last_seen_ms"], Value::Null);
    assert_eq!(value["escalations"], 0);
}

/// ⑦b 免疫记忆 probe 脱敏: 值里只有计数/类别名/时刻, 零签名/记录键/路径/原文。
#[test]
fn the_immune_probe_never_leaks_record_keys_paths_or_raw_text() {
    let memory = immune_memory_with_records();
    // 记录键形如 `credential_leak:<16 hex>`; 从账本里取出真键做泄漏对照。
    let record_key = {
        let guard = memory.lock();
        let records = apeireth_guard::immune_memory::ImmuneMemoryStore::list(guard.store());
        records
            .first()
            .map(|(signature, _)| signature.record_key())
            .expect("record")
    };
    assert!(record_key.contains(":"), "记录键形态: {record_key}");

    let reading = immune_quorum_probe(memory).snapshot(1).expect("reading");
    let rendered = serde_json::to_string(&reading.value).expect("render");
    assert!(!rendered.contains(&record_key), "不得回记录键: {rendered}");
    assert!(!rendered.contains("token.txt"), "不得回路径: {rendered}");
    assert!(!rendered.contains("Users"), "不得回路径: {rendered}");
    assert!(
        !rendered.contains("tool descriptor"),
        "不得回原文: {rendered}"
    );
    let keys: std::collections::BTreeSet<&str> = reading
        .value
        .as_ref()
        .and_then(Value::as_object)
        .expect("object value")
        .keys()
        .map(String::as_str)
        .collect();
    let expected: std::collections::BTreeSet<&str> = [
        "records",
        "open",
        "resolved",
        "known_patterns",
        "escalations",
        "kinds",
        "first_seen_ms",
        "last_seen_ms",
        "observed_at_ms",
    ]
    .into_iter()
    .collect();
    assert_eq!(keys, expected, "免疫格值字段闭合 (不夹带私有字段)");
}

/// ⑦c 审议门 probe: `Hold` 拒绝带分数/差额/类型, 但不回门账理由原文。
#[test]
fn the_quorum_probe_reports_a_hold_denial_without_reason_text() {
    let trace = QuorumTrace {
        at_ms: 4_200,
        convened: false,
        escalation: None,
        denial: Some(QuorumDecision::Hold {
            score: 0.25,
            need: 0.75,
        }),
    };
    let reading = quorum_trace_probe(Arc::new(move || Some(trace.clone())))
        .snapshot(5_000)
        .expect("trace reading");
    let value = reading.value.expect("value");
    assert_eq!(value["traces"], 1);
    assert_eq!(value["convened"], false);
    assert_eq!(value["at_ms"], 4_200);
    assert_eq!(value["denial"], "hold");
    assert_eq!(value["denial_score"], 0.25);
    assert_eq!(value["denial_need"], 0.75);
    assert_eq!(value["cooldown_until_ms"], Value::Null);
    assert_eq!(value["escalation_score"], Value::Null);
    assert_eq!(value["reason_count"], 0);
    assert_eq!(value["observed_at_ms"], 5_000);
}

/// ⑦d 审议门 probe: 升议只回浓度与理由**条数**, 理由文本 (含自由来源/用户
/// 原文) 一律不出帧; 无门账 = `traces: 0` (默认关/从未 gated 的实况)。
#[test]
fn the_quorum_probe_reports_escalation_counts_and_no_trace_honestly() {
    let escalation = QuorumEscalation {
        score: 1.5,
        reasons: vec![
            "conflict weight=1.000 source=session-secret-42 at_ms=10".to_string(),
            "anomaly weight=0.500 source=guard.immune at_ms=11".to_string(),
        ],
        triggered_at_ms: 12,
    };
    let trace = QuorumTrace {
        at_ms: 12,
        convened: true,
        escalation: Some(escalation),
        denial: None,
    };
    let reading = quorum_trace_probe(Arc::new(move || Some(trace.clone())))
        .snapshot(20)
        .expect("trace reading");
    let rendered = serde_json::to_string(&reading.value).expect("render");
    assert!(!rendered.contains("session-secret-42"), "{rendered}");
    assert!(!rendered.contains("weight="), "{rendered}");
    assert!(!rendered.contains("guard.immune"), "{rendered}");
    let value = reading.value.expect("value");
    assert_eq!(value["traces"], 1);
    assert_eq!(value["convened"], true);
    assert_eq!(value["escalation_score"], 1.5);
    assert_eq!(value["reason_count"], 2, "只回条数不回原文");
    assert_eq!(value["denial"], Value::Null);

    // 门默认关 / 从未 gated: 无门账 = traces 0 + 各结果字段显式 null。
    let reading = quorum_trace_probe(Arc::new(|| None))
        .snapshot(30)
        .expect("no trace reading");
    let value = reading.value.expect("value");
    assert_eq!(value["traces"], 0);
    assert_eq!(value["convened"], Value::Null);
    assert_eq!(value["at_ms"], Value::Null);
    assert_eq!(value["escalation_score"], Value::Null);
    assert_eq!(value["reason_count"], 0);
}

/// ⑦e 关怀心跳 probe: 数字真值透传; 未运行 = `next_poll_at_ms: null` +
/// 计数实况 (不是编 0, 也不是未接线)。
#[test]
fn the_care_heartbeat_probe_reports_polls_hits_and_null_when_not_running() {
    let running = CareHeartbeatStatus {
        next_poll_at_ms: Some(9_000),
        last_hit_at_ms: Some(8_000),
        polls: 7,
        hits: 2,
    };
    let reading = care_heartbeat_probe(Arc::new(move || running))
        .snapshot(10_000)
        .expect("heartbeat reading");
    let value = reading.value.expect("value");
    assert_eq!(value["polls"], 7);
    assert_eq!(value["hits"], 2);
    assert_eq!(value["next_poll_at_ms"], 9_000);
    assert_eq!(value["last_hit_at_ms"], 8_000);
    assert_eq!(value["observed_at_ms"], 10_000);

    let idle = care_heartbeat_probe(Arc::new(|| CareHeartbeatStatus::default()))
        .snapshot(1)
        .expect("idle reading");
    let value = idle.value.expect("value");
    assert_eq!(value["polls"], 0);
    assert_eq!(value["hits"], 0);
    assert_eq!(value["next_poll_at_ms"], Value::Null);
    assert_eq!(value["last_hit_at_ms"], Value::Null);
}

/// ⑦f 练习账 probe: 读四件套同一本内存账 — 空账全零 → 开场 running →
/// 结束 finished → 第二场 abort; 只回计数不回 session 主题/路径。
#[tokio::test]
async fn the_play_probe_counts_the_live_ledger_through_the_tool_face() {
    let root = std::env::temp_dir().join(format!(
        "apeireth-bio-self-status-play-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("测试工作区");
    let wiring =
        bio_play_wiring_from_env_value(Some("1"), Some(root.as_path()), fixed_virtual_clock())
            .expect("四件套必须构造");
    let state = Arc::clone(wiring.state());
    let capabilities: Vec<Arc<dyn ToolCapability>> = wiring
        .modules()
        .iter()
        .flat_map(|module| module.capabilities())
        .collect();
    let propose = Arc::clone(find_play_capability(
        &capabilities,
        PLAY_PROPOSE_CAPABILITY_ID,
    ));
    let start = Arc::clone(find_play_capability(
        &capabilities,
        PLAY_START_CAPABILITY_ID,
    ));
    let finish = Arc::clone(find_play_capability(
        &capabilities,
        PLAY_FINISH_CAPABILITY_ID,
    ));
    let abort = Arc::clone(find_play_capability(
        &capabilities,
        PLAY_ABORT_CAPABILITY_ID,
    ));
    let probe = play_probe(Arc::clone(&state));

    // 空账: 全零 (不是编造)。
    let value = probe
        .snapshot(1)
        .expect("empty ledger")
        .value
        .expect("value");
    assert_eq!(value["sessions"], 0);
    assert_eq!(value["pending"], 0);
    assert_eq!(value["finished"], 0);
    assert_eq!(value["aborted"], 0);

    // 开场 = running/pending 各 1。
    let frame = invoke_play_tool(&propose, PLAY_PROPOSE_TOOL_NAME, play_candidates()).await;
    let proposal = frame["proposal"].clone();
    let started = invoke_play_tool(
        &start,
        PLAY_START_TOOL_NAME,
        json!({
            "session_id": "s-bio-1",
            "sandbox_dir": "sandbox/play-1",
            "proposal": proposal,
            "user_opt_in": true,
        }),
    )
    .await;
    assert_eq!(started["session"]["status"], "running");
    let value = probe
        .snapshot(2)
        .expect("running ledger")
        .value
        .expect("value");
    assert_eq!(value["sessions"], 1);
    assert_eq!(value["pending"], 1);
    assert_eq!(value["running"], 1);
    assert_eq!(value["planned"], 0);
    assert_eq!(value["finished"], 0);
    assert_eq!(value["aborted"], 0);

    // 结束 = finished 1 / pending 0。
    invoke_play_tool(
        &finish,
        PLAY_FINISH_TOOL_NAME,
        json!({
            "session_id": "s-bio-1",
            "learned": ["拆解"],
            "artifacts": ["sandbox/play-1/demo.txt"],
            "budget_used": 1_234,
        }),
    )
    .await;
    let value = probe
        .snapshot(3)
        .expect("finished ledger")
        .value
        .expect("value");
    assert_eq!(value["sessions"], 1);
    assert_eq!(value["pending"], 0);
    assert_eq!(value["finished"], 1);
    assert_eq!(value["running"], 0);

    // 第二场 abort = aborted 1, 总账 2。
    let frame = invoke_play_tool(&propose, PLAY_PROPOSE_TOOL_NAME, play_candidates()).await;
    let proposal = frame["proposal"].clone();
    invoke_play_tool(
        &start,
        PLAY_START_TOOL_NAME,
        json!({
            "session_id": "s-bio-2",
            "sandbox_dir": "sandbox/play-2",
            "proposal": proposal,
            "user_opt_in": true,
        }),
    )
    .await;
    invoke_play_tool(
        &abort,
        PLAY_ABORT_TOOL_NAME,
        json!({"session_id": "s-bio-2"}),
    )
    .await;
    let reading = probe.snapshot(4).expect("aborted ledger");
    let value = reading.value.expect("value");
    assert_eq!(value["sessions"], 2);
    assert_eq!(value["pending"], 0);
    assert_eq!(value["finished"], 1);
    assert_eq!(value["aborted"], 1);
    // 脱敏: 值里不出现 session id / 主题 / 沙箱路径。
    let rendered = serde_json::to_string(&value).expect("render");
    assert!(!rendered.contains("s-bio"), "{rendered}");
    assert!(!rendered.contains("预演一次捕猎路线"), "{rendered}");
    assert!(!rendered.contains("sandbox/play"), "{rendered}");

    let _ = std::fs::remove_dir_all(&root);
}

/// ⑦g 痛觉 probe: 读运行时快照的只读计数 (开关/评估/修复/分组/观测/慢性),
/// 零能力名 / 原因键 / 理由原文。
#[test]
fn the_pain_probe_reports_the_runtime_snapshot_counts() {
    let reading = pain_probe(pain_reader(pain_snapshot()))
        .snapshot(50)
        .expect("pain reading");
    let value = reading.value.expect("value");
    assert_eq!(value["enabled"], true);
    assert_eq!(value["assessed"], 3);
    assert_eq!(value["repaired"], 1);
    assert_eq!(value["pain_groups"], 0);
    assert_eq!(value["observations"], 1);
    assert_eq!(value["chronic_observations"], 1);
    assert_eq!(value["observed_at_ms"], 50);
    let rendered = serde_json::to_string(&value).expect("render");
    assert!(!rendered.contains("tool.web_fetch"), "{rendered}");
    assert!(!rendered.contains("transient:"), "{rendered}");
    assert!(!rendered.contains("chronic same cause"), "{rendered}");
}

/// ⑦h 再生 probe: 与痛觉共用同一读取口, 报四期分布/活跃/收尾/拒绝迁移计数。
#[test]
fn the_recovery_probe_reports_the_recovery_case_counts() {
    let reading = recovery_probe(pain_reader(pain_snapshot()))
        .snapshot(60)
        .expect("recovery reading");
    let value = reading.value.expect("value");
    assert_eq!(value["enabled"], true);
    assert_eq!(value["active_cases"], 3);
    assert_eq!(value["closed_cases"], 2);
    assert_eq!(value["rejected_transitions"], 1);
    assert_eq!(value["hemostasis"], 1);
    assert_eq!(value["inflammation"], 1);
    assert_eq!(value["proliferation"], 1);
    assert_eq!(value["remodeling"], 0);
    assert_eq!(value["resolved"], 0);
    assert_eq!(value["observed_at_ms"], 60);
    let keys: std::collections::BTreeSet<&str> = value
        .as_object()
        .expect("object value")
        .keys()
        .map(String::as_str)
        .collect();
    let expected: std::collections::BTreeSet<&str> = [
        "enabled",
        "active_cases",
        "closed_cases",
        "rejected_transitions",
        "hemostasis",
        "inflammation",
        "proliferation",
        "remodeling",
        "resolved",
        "observed_at_ms",
    ]
    .into_iter()
    .collect();
    assert_eq!(keys, expected);
}

/// ⑧a 迟绑定读取口失败 = 单格 null + 真因, 其余格照常 wired (不整帧失败)。
#[tokio::test]
async fn a_failing_late_bound_reader_is_one_null_cell_not_a_failed_frame() {
    let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
        .with_bio_self_status(true)
        .with_immune_probe(immune_memory_with_records())
        .with_pain_probe(Arc::new(|| Err("runtime not attached yet".to_string())));
    // 再生格不注册: 仍然如实未接线。
    let report = report(source).await;
    let rows = report["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"]
        .as_object()
        .expect("mechanisms object");
    let pain = &rows["nociception"];
    assert_eq!(pain["wired"], true, "已注册 = 接线事实成立");
    assert_eq!(pain["value"], Value::Null);
    assert!(
        pain["reason"]
            .as_str()
            .unwrap()
            .contains("runtime not attached yet"),
        "{pain}"
    );
    // 其余已注册格不受牵连: 免疫格照常真值。
    let immune = &rows["immune_memory"];
    assert_eq!(immune["wired"], true);
    assert_eq!(immune["value"]["records"], 2);
    assert!(immune["reason"].is_null());
    // 未注册的再生格仍然 wired:false + 「未接线」。
    assert_eq!(rows["recovery"]["wired"], false);
    assert_eq!(rows["recovery"]["stage"], BIO_STAGE_NOT_WIRED);
    // 整帧仍成功且 21 节不变。
    let top: std::collections::BTreeSet<&str> = report
        .as_object()
        .expect("object frame")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        top,
        SELF_STATUS_SECTIONS
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
    );
    assert!(SelfStatusTool::output_schema().validate(&report).is_ok());
}

/// ⑧b 便捷注册口 = 注册 ≠ 启用: 显式关时整节 mechanisms null, 一格 reader
/// 都不调用; 显式开且未注册的格仍 wired:false。
#[tokio::test]
async fn convenience_registration_entry_points_are_off_by_default() {
    let calls = Arc::new(AtomicUsize::new(0));
    let care_calls = Arc::clone(&calls);
    let pain_calls = Arc::clone(&calls);
    let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
        .with_bio_self_status(false)
        .with_immune_probe(immune_memory_with_records())
        .with_care_heartbeat_probe(Arc::new(move || {
            care_calls.fetch_add(1, Ordering::SeqCst);
            CareHeartbeatStatus::default()
        }))
        .with_pain_probe(Arc::new(move || {
            pain_calls.fetch_add(1, Ordering::SeqCst);
            Ok(pain_snapshot())
        }))
        .with_play_probe(Arc::new(
            apeireth_runtime_assembly::canonical::BioPlayState::new(
                "C:\\work",
                fixed_virtual_clock(),
            ),
        ));
    let off_report = report(source).await;
    let bio = &off_report["capabilities"][CAPABILITIES_BIO_MECHANISMS];
    assert_eq!(bio["enabled"], false);
    assert_eq!(bio["mechanisms"], Value::Null, "关时一格也不采");
    assert_eq!(calls.load(Ordering::SeqCst), 0, "关时不许调用任何 reader");

    // 显式开: 已注册的 4 格 wired, 未注册的格如实未接线。
    let report = report(
        ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_bio_self_status(true)
            .with_immune_probe(immune_memory_with_records())
            .with_care_heartbeat_probe(Arc::new(|| CareHeartbeatStatus::default())),
    )
    .await;
    let rows = report["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"]
        .as_object()
        .expect("mechanisms object");
    assert_eq!(rows["immune_memory"]["wired"], true);
    assert_eq!(rows["attachment"]["wired"], true);
    for (name, row) in rows {
        if name == "immune_memory" || name == "attachment" {
            continue;
        }
        assert_eq!(row["wired"], false, "{name} 未注册必须 wired:false");
        assert_eq!(row["stage"], BIO_STAGE_NOT_WIRED, "{name}");
    }
}

/// ⑨a 六条新 probe 全注册的帧: 21 节合同不变, 全部只挂在 capabilities 子对象
/// 内; serde 往返稳定 (同一份读数序列化-反序列化-再序列化逐字段一致)。
#[tokio::test]
async fn six_new_probes_keep_the_twenty_one_sections_and_round_trip_through_serde() {
    let memory = immune_memory_with_records();
    let quorum = QuorumTrace {
        at_ms: 12,
        convened: true,
        escalation: Some(QuorumEscalation {
            score: 1.5,
            reasons: Vec::new(),
            triggered_at_ms: 12,
        }),
        denial: None,
    };
    let pain = pain_snapshot();
    let mut source = ProductionSelfStatusSource::new(Vec::new(), false, None)
        .with_bio_mechanisms(BioMechanismRegistry::new(true).with_now_ms(fixed_clock()))
        .with_immune_probe(memory)
        .with_care_heartbeat_probe(Arc::new(|| CareHeartbeatStatus {
            next_poll_at_ms: Some(9_000),
            last_hit_at_ms: Some(8_000),
            polls: 7,
            hits: 2,
        }))
        .with_play_probe(Arc::new(
            apeireth_runtime_assembly::canonical::BioPlayState::new(
                "C:\\work",
                fixed_virtual_clock(),
            ),
        ))
        .with_pain_probe(pain_reader(pain.clone()))
        .with_recovery_probe(pain_reader(pain));
    source.register_bio_probe(
        BioMechanismKind::Quorum,
        quorum_trace_probe(Arc::new(move || Some(quorum.clone()))),
    );

    let report = report(source).await;
    let top: std::collections::BTreeSet<&str> = report
        .as_object()
        .expect("object frame")
        .keys()
        .map(String::as_str)
        .collect();
    let frozen: std::collections::BTreeSet<&str> = SELF_STATUS_SECTIONS.iter().copied().collect();
    assert_eq!(top, frozen, "扩展 probe 不得新增/删除顶层节");
    assert!(
        report.get(CAPABILITIES_BIO_MECHANISMS).is_none(),
        "不是顶层节"
    );

    let bio = report["capabilities"][CAPABILITIES_BIO_MECHANISMS].clone();
    let mechanisms = bio["mechanisms"].as_object().expect("mechanisms object");
    assert_eq!(mechanisms.len(), BioMechanismKind::ALL.len());
    for kind in [
        BioMechanismKind::ImmuneMemory,
        BioMechanismKind::Quorum,
        BioMechanismKind::Attachment,
        BioMechanismKind::Play,
        BioMechanismKind::Nociception,
        BioMechanismKind::Recovery,
    ] {
        assert_eq!(mechanisms[kind.as_str()]["wired"], true, "{kind:?}");
        assert_eq!(
            mechanisms[kind.as_str()]["stage"],
            BIO_STAGE_WIRED,
            "{kind:?}"
        );
    }
    assert_eq!(mechanisms["immune_memory"]["value"]["records"], 2);
    assert_eq!(mechanisms["attachment"]["value"]["polls"], 7);
    assert_eq!(mechanisms["play"]["value"]["sessions"], 0);
    assert_eq!(mechanisms["nociception"]["value"]["assessed"], 3);
    assert_eq!(mechanisms["recovery"]["value"]["active_cases"], 3);
    assert_eq!(mechanisms["capacity"]["wired"], false, "未注册的格绝不冒充");

    // serde 往返: 解析回类型再序列化, 与帧内子对象逐字段一致 (字段合同稳定)。
    let parsed: BioMechanismsStatus = serde_json::from_value(bio.clone()).expect("round trip");
    assert_eq!(parsed.enabled, true);
    assert_eq!(parsed.reason, None);
    assert_eq!(parsed.mechanisms.as_ref().expect("rows").len(), 14);
    let re_encoded = serde_json::to_value(&parsed).expect("re-encode");
    assert_eq!(re_encoded, bio, "serde 往返必须逐字段稳定");
    assert!(SelfStatusTool::output_schema().validate(&report).is_ok());
}

/// ⑨b 确定性: 同一时钟 + 同一批只读句柄 → 两次采集的 bio 子对象逐字节一致
/// (免疫账/练习账只读, 采集本身不写账)。
#[tokio::test]
async fn the_extended_bio_face_is_deterministic_under_an_injected_clock() {
    let memory = immune_memory_with_records();
    let pain = pain_snapshot();
    let play_state = Arc::new(apeireth_runtime_assembly::canonical::BioPlayState::new(
        "C:\\work",
        fixed_virtual_clock(),
    ));
    let build = || {
        let mut source = ProductionSelfStatusSource::new(Vec::new(), false, None)
            .with_bio_mechanisms(BioMechanismRegistry::new(true).with_now_ms(fixed_clock()))
            .with_immune_probe(Arc::clone(&memory))
            .with_care_heartbeat_probe(Arc::new(|| CareHeartbeatStatus {
                next_poll_at_ms: Some(9_000),
                last_hit_at_ms: Some(8_000),
                polls: 7,
                hits: 2,
            }))
            .with_play_probe(Arc::clone(&play_state))
            .with_pain_probe(pain_reader(pain.clone()))
            .with_recovery_probe(pain_reader(pain.clone()));
        source.register_bio_probe(
            BioMechanismKind::Quorum,
            quorum_trace_probe(Arc::new(|| {
                Some(QuorumTrace {
                    at_ms: 12,
                    convened: false,
                    escalation: None,
                    denial: Some(QuorumDecision::Cooldown {
                        until_ms: 99,
                        score: 0.5,
                    }),
                })
            })),
        );
        source
    };
    let first = report(build()).await;
    let second = report(build()).await;
    assert_eq!(
        first["capabilities"][CAPABILITIES_BIO_MECHANISMS],
        second["capabilities"][CAPABILITIES_BIO_MECHANISMS],
        "同输入必须同输出"
    );
    assert_eq!(
        first["capabilities"][CAPABILITIES_BIO_MECHANISMS]["mechanisms"]["quorum"]["value"]
            ["cooldown_until_ms"],
        99
    );
}
