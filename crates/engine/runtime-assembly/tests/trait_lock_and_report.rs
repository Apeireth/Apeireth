//! 性格锁定 + 性格周报 (裁定 ④⑤ 落地) 的装配级证明。
//!
//! 钉住: ①锁定后成长沉积**跳过**该杆且留审计痕 (杆值不变, 留痕落成长计
//! 既有账); ②解锁 = 显式动作, 解锁后沉积恢复生效; ③周报数值正确 (7 天窗 /
//! 移动量 / 移动原因 top / 锁定态) 且走 `tool.trait_report` 生产面; ④只读档
//! (非写/执行能力, `read_only` 预设放行, 零审批档); ⑤self_status 性格节
//! 带锁定态 + 最近移动 (与周报同账); ⑥注册即闸 + 名册如实。

use std::sync::{Arc, Mutex};

use apeireth_orchestration::self_tuning::{
    is_trait_locked_skip, TunableParam, TuningRecord, TuningValues, DAY_MS,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::ToolCall;
use apeireth_runtime_assembly::{
    is_write_or_execute_capability, now_epoch_ms, read_growth_records, trait_status_probe,
    ProductionBackends, ProductionModules, ProductionModulesConfig, ProductionSelfStatusSource,
    SelfTuningWire, TraitGrowthPaths, TRAIT_LOCKS_FILE, TRAIT_REPORT_CAPABILITY_ID,
    TUNING_LOG_FILE,
};
use serde_json::{json, Value};

/// 生效值 override 是进程级全局槽: 触碰它的用例必须串行 (与
/// `self_tuning_wire` 内嵌测试同口径)。
static GLOBAL_GUARD: Mutex<()> = Mutex::new(());

fn lock_global() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL_GUARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-trait-r2-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn call(name: &str) -> ToolCall {
    ToolCall {
        id: format!("call_{name}"),
        name: name.into(),
        arguments: json!({}),
    }
}

fn envelope(row: &TuningRecord) -> String {
    serde_json::to_string(&apeireth_core::stored_doc::StoredDoc::new(
        apeireth_runtime_assembly::TUNING_LOG_DOC_NAME,
        apeireth_runtime_assembly::TUNING_LOG_DOC_VERSION,
        vec![apeireth_runtime_assembly::TUNING_LOG_DOC_VERSION],
        row,
    ))
    .expect("serialize")
}

fn append_row(log_path: &std::path::Path, row: &TuningRecord) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .expect("open ledger");
    writeln!(file, "{}", envelope(row)).expect("append row");
}

/// ①② 锁定后沉积跳过 + 审计痕落账 (杆值不变); 显式解锁恢复移动。
#[tokio::test]
async fn locked_slider_skips_the_deposit_leaves_an_audit_row_and_unlock_resumes() {
    let _global = lock_global();
    let dir = temp_dir("wire");
    let log_path = dir.join(TUNING_LOG_FILE);
    let wire = SelfTuningWire::new(log_path.clone(), TuningValues::baseline());

    // 显式锁定一根滑杆 → 持久档落位。
    let locks = wire.lock_param(TunableParam::MemoryFade).expect("lock");
    assert!(locks.memory_fade);
    assert_eq!(wire.locks_path(), dir.join(TRAIT_LOCKS_FILE));

    // 一轮全命中批 (强证据, 正常会调): 锁定 = 跳过沉积, 杆值不变, 留痕落账。
    let base = TunableParam::MemoryFade.baseline();
    for index in 0..4_i64 {
        wire.record_retrieval_round(true, 1_000 + index);
    }
    assert_eq!(
        wire.effective_values().memory_fade,
        base,
        "锁定杆的成长沉积不得移动杆值"
    );
    assert_eq!(wire.log_failures(), 0);
    assert_eq!(wire.lock_failures(), 0);
    let (records, _) = read_growth_records(&log_path).expect("ledger readable");
    assert_eq!(records.len(), 1, "恰好一条审计留痕");
    let audit = &records[0];
    assert!(is_trait_locked_skip(audit), "留痕可识别: {}", audit.reason);
    assert_eq!(audit.previous, audit.next, "跳过沉积 = 零移动");
    assert!(
        audit.reason.contains("命中"),
        "留痕带原沉积意图原因: {}",
        audit.reason
    );

    // 显式解锁 → 同类沉积恢复生效 (解锁 = 显式动作)。
    let locks = wire.unlock_param(TunableParam::MemoryFade).expect("unlock");
    assert!(!locks.any_locked());
    for index in 0..4_i64 {
        wire.record_retrieval_round(true, 2_000 + index);
    }
    assert!(
        wire.effective_values().memory_fade > base,
        "解锁后成长沉积恢复移动"
    );
    let (records, _) = read_growth_records(&log_path).expect("ledger readable");
    assert_eq!(records.len(), 2, "解锁后的一条真实沉积也落同一本账");
    assert!(!is_trait_locked_skip(&records[1]));

    // 坏档 fail-closed: 锁定态读不出来 = 全锁定 (绝不把坏档读成全解锁放行),
    // 失败计数不静默。
    std::fs::write(wire.locks_path(), "{ not json").expect("corrupt locks");
    let before = wire.effective_values().memory_fade;
    for index in 0..4_i64 {
        wire.record_retrieval_round(true, 3_000 + index);
    }
    assert_eq!(
        wire.effective_values().memory_fade,
        before,
        "坏档 fail-closed: 读不出锁定态就不动任何杆"
    );
    assert!(wire.lock_failures() > 0, "坏档必须计数不静默");

    apeireth_orchestration::self_tuning::clear_effective_overrides();
    let _ = std::fs::remove_dir_all(&dir);
}

/// ③ 周报数值正确 (走 `tool.trait_report` 生产面): 7 天窗 / 移动量 / 原因 top / 锁定态。
#[tokio::test]
async fn trait_report_numbers_are_correct_on_the_production_face() {
    let _global = lock_global();
    let dir = temp_dir("report");
    let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
    let now = now_epoch_ms();
    // 窗内: MemoryFade 移动 +0.25 (一次) + 锁定跳过留痕 (一次)。
    append_row(
        &paths.log_path,
        &TuningRecord {
            seq: 1,
            param: TunableParam::MemoryFade,
            previous: 1.0,
            next: 1.25,
            reason: "检索命中 4/未命中 0: 召回供给充裕, 遗忘衰减上调一档".into(),
            at_epoch_ms: now - DAY_MS,
        },
    );
    append_row(
        &paths.log_path,
        &TuningRecord {
            seq: 2,
            param: TunableParam::MemoryFade,
            previous: 1.25,
            next: 1.25,
            reason: "已锁定, 跳过沉积 (审计留痕): 测试意图".into(),
            at_epoch_ms: now - DAY_MS,
        },
    );
    // 窗外 (8 天前): 不得进周报。
    append_row(
        &paths.log_path,
        &TuningRecord {
            seq: 3,
            param: TunableParam::ToneSaturation,
            previous: 1.0,
            next: 0.5,
            reason: "窗外的沉积".into(),
            at_epoch_ms: now - 8 * DAY_MS,
        },
    );
    apeireth_runtime_assembly::set_trait_lock(
        &paths.locks_path(),
        TunableParam::ToneSaturation,
        true,
    )
    .expect("lock tone");

    let mut config = ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        trait_report: Some(paths.clone()),
        ..ProductionModulesConfig::default()
    };
    let frozen = || -> Arc<dyn apeireth_core::kernel::Clock> {
        Arc::new(apeireth_core::kernel::SystemClock)
    };
    let modules = ProductionModules::build(config.clone(), ProductionBackends::default(), frozen())
        .expect("build");
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == TRAIT_REPORT_CAPABILITY_ID)
        .cloned()
        .expect("tool.trait_report must register");
    let result = tool.invoke(&call("trait_report")).await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).expect("report json");

    assert_eq!(report["write_face"], "read_only");
    assert_eq!(report["window_days"], 7);
    let window_from = report["window_from_ms"].as_i64().expect("window_from_ms");
    assert!(
        (window_from - (now - 7 * DAY_MS)).abs() < 60_000,
        "窗口 = 近 7 天: {window_from} vs {}",
        now - 7 * DAY_MS
    );
    assert_eq!(report["records_in_window"], 2, "窗外记录不进窗");
    assert_eq!(report["locked_skipped"], 1);

    let sliders = report["sliders"].as_array().expect("sliders");
    assert_eq!(sliders.len(), 4, "四杆一行不多不少");
    let fade = sliders
        .iter()
        .find(|row| row["param"] == "memory_fade")
        .expect("memory_fade row");
    let fade_net = fade["net_delta"].as_f64().expect("net_delta");
    assert!(
        (fade_net - 0.25).abs() < 1e-9,
        "窗内净移动量 = +0.25: {fade_net}"
    );
    assert_eq!(fade["move_count"], 1);
    assert_eq!(fade["locked_skipped"], 1);
    assert_eq!(fade["locked"], false);
    assert_eq!(fade["last_move"]["delta"], 0.25);
    let tone = sliders
        .iter()
        .find(|row| row["param"] == "tone_saturation")
        .expect("tone row");
    assert_eq!(tone["locked"], true, "锁定态如实");
    assert_eq!(tone["move_count"], 0, "窗外移动不计");
    assert_eq!(tone["last_move"], Value::Null);

    let reasons = report["top_reasons"].as_array().expect("top reasons");
    assert_eq!(reasons.len(), 1, "窗内实际移动只有一种原因");
    assert!(
        reasons[0]["reason"]
            .as_str()
            .is_some_and(|text| text.contains("遗忘衰减上调一档")),
        "{reasons:?}"
    );
    assert_eq!(reasons[0]["count"], 1);

    // 名册如实 (开): 装配出的自述照实报 `trait_report: true` (注册即闸同源)。
    let self_status = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .cloned()
        .expect("tool.self_status");
    let status_report: Value =
        serde_json::from_str(&self_status.invoke(&call("self_status")).await.render())
            .expect("self report json");
    assert_eq!(
        status_report["capabilities"]["trait_report"], true,
        "名册开"
    );

    // 关 (未注入成长账落位) = 不注册 + 名册照实 false。
    config.trait_report = None;
    let off = ProductionModules::build(config.clone(), ProductionBackends::default(), frozen())
        .expect("off");
    assert!(off
        .capabilities()
        .iter()
        .all(|capability| capability.id().as_str() != TRAIT_REPORT_CAPABILITY_ID));
    let source = ProductionSelfStatusSource::new(
        apeireth_runtime_assembly::roster_from_config(&config, false, false, false, &[]),
        false,
        None,
    );
    let self_status = apeireth_tools_canonical::SelfStatusTool::new(Arc::new(source));
    let roster_report: Value =
        serde_json::from_str(&self_status.invoke(&call("self_status")).await.render())
            .expect("self report");
    assert_eq!(
        roster_report["capabilities"]["trait_report"], false,
        "名册照实"
    );

    apeireth_orchestration::self_tuning::clear_effective_overrides();
    let _ = std::fs::remove_dir_all(&dir);
}

/// ④ 只读档: 非写/执行能力 (`read_only` 预设放行) + 零审批档的授权面在 CLI 同档证明。
#[test]
fn trait_report_is_a_read_only_capability() {
    let capability = apeireth_core::kernel::CapabilityId::new(TRAIT_REPORT_CAPABILITY_ID)
        .expect("capability id");
    assert!(
        !is_write_or_execute_capability(&capability),
        "tool.trait_report 必须是只读能力 (read_only 预设下照常可用)"
    );
}

/// ⑤ self_status 性格节: 锁定态 + 最近移动 (与周报同一成长计既有账)。
#[tokio::test]
async fn self_status_tuning_section_carries_lock_state_and_recent_moves() {
    let _global = lock_global();
    let dir = temp_dir("status");
    let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
    let now = now_epoch_ms();
    append_row(
        &paths.log_path,
        &TuningRecord {
            seq: 1,
            param: TunableParam::MemoryFade,
            previous: 1.0,
            next: 1.25,
            reason: "最近的一次移动".into(),
            at_epoch_ms: now - DAY_MS,
        },
    );
    apeireth_runtime_assembly::set_trait_lock(&paths.locks_path(), TunableParam::MemoryFade, true)
        .expect("lock");

    let source = ProductionSelfStatusSource::new(Vec::new(), false, None)
        .with_trait_growth_probe(trait_status_probe(paths.clone()));
    let self_status = apeireth_tools_canonical::SelfStatusTool::new(Arc::new(source));
    let result = self_status.invoke(&call("self_status")).await;
    assert!(result.is_ok(), "{}", result.render());
    let report: Value = serde_json::from_str(&result.render()).expect("self report json");
    assert_eq!(report["tuning"]["locked"]["memory_fade"], true);
    assert_eq!(report["tuning"]["locked"]["tone_saturation"], false);
    let moves = report["tuning"]["recent_moves"]
        .as_array()
        .expect("recent moves");
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0]["param"], "memory_fade");
    assert_eq!(moves[0]["label"], "遗忘衰减强度");
    assert_eq!(moves[0]["reason"], "最近的一次移动");
    assert_eq!(report["tuning"]["reason"], Value::Null, "齐全 = 无缺因");

    // 性格节来源缺席 = 显式 null + 原因 (0 装诚实, 不编锁态)。
    let bare = ProductionSelfStatusSource::new(Vec::new(), false, None);
    let self_status = apeireth_tools_canonical::SelfStatusTool::new(Arc::new(bare));
    let report: Value =
        serde_json::from_str(&self_status.invoke(&call("self_status")).await.render())
            .expect("self report json");
    assert_eq!(report["tuning"]["locked"], Value::Null);
    assert_eq!(report["tuning"]["recent_moves"], Value::Null);
    assert_eq!(
        report["tuning"]["reason"],
        "status source not wired in this composition"
    );

    apeireth_orchestration::self_tuning::clear_effective_overrides();
    let _ = std::fs::remove_dir_all(&dir);
}
