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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apeireth_orchestration::task_queue::{
    EnqueueOutcome, TaskCaller, TaskQueue, TaskQueueConfig, TaskSpec, TaskStore,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::canonical::self_status_source::{
    capacity_bio_probe, BioMechanismRegistry, ProductionSelfStatusSource,
};
use apeireth_runtime_assembly::{task_board_status_probe, SqliteTaskStore};
use apeireth_tools_canonical::self_status::{
    bio_self_status_enabled_from_env_value, BioMechanismKind, BioMechanismProbe,
    BioMechanismReading, BIO_SELF_STATUS_ENABLE_ENV, BIO_STAGE_NOT_WIRED, BIO_STAGE_WIRED,
    CAPABILITIES_BIO_MECHANISMS, SELF_STATUS_SECTIONS,
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
