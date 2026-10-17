//! 裁定 ④⑤ 性格锁定 + 性格周报的 CLI 面测试。
//!
//! 覆盖: ①`apeireth trait lock|unlock` 显式动作持久 (锁定档 `trait-locks.json`,
//! 解锁 = 显式动作); ②`apeireth trait report` 周报数值 (7 天窗 / 移动量 /
//! 移动原因 / 锁定态) 从成长计既有账真值打印; ③`tool.trait_report` 只读档
//! (默认 grant + 零审批, 与 `tool.self_status` 同档) + 生产面真调用;
//! ④文档随行 (user-manual 写明工具面与命令面)。

use std::path::{Path, PathBuf};

use apeireth_cli::trait_command::{run_trait_at, USAGE};
use apeireth_orchestration::self_tuning::{TunableParam, TuningRecord, DAY_MS};
use apeireth_runtime_assembly::{
    load_trait_locks, now_epoch_ms, trait_locks_path_for, TraitGrowthPaths, TRAIT_LOCKS_FILE,
    TUNING_LOG_DOC_NAME, TUNING_LOG_DOC_VERSION, TUNING_LOG_FILE,
};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("apeireth-trait-cli-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn append_row(log_path: &Path, row: &TuningRecord) {
    use std::io::Write;
    let doc = apeireth_core::stored_doc::StoredDoc::new(
        TUNING_LOG_DOC_NAME,
        TUNING_LOG_DOC_VERSION,
        vec![TUNING_LOG_DOC_VERSION],
        row,
    );
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .expect("open ledger");
    writeln!(file, "{}", serde_json::to_string(&doc).expect("serialize")).expect("append");
}

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

/// ① 锁定/解锁 = 显式动作且持久; 解锁无自动路径 (只有显式动作能解)。
#[test]
fn trait_lock_and_unlock_are_explicit_and_persisted() {
    let dir = temp_dir("lock");
    let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
    assert_eq!(
        paths.locks_path(),
        dir.join(TRAIT_LOCKS_FILE),
        "锁定档与成长账同目录"
    );

    // 锁一根 → 持久 → 输出报锁定态。
    let out = run_trait_at(&paths, &args(&["lock", "memory_fade"])).expect("lock");
    assert!(out.contains("已锁定"), "{out}");
    assert!(out.contains("memory_fade"), "{out}");
    assert!(out.contains("跳过该杆"), "{out}");
    let locks = load_trait_locks(&paths.locks_path()).expect("locks persisted");
    assert!(locks.memory_fade, "锁定必须持久");
    assert!(!locks.tone_saturation, "只锁一根不误伤别杆");

    // 显式解锁 → 持久恢复。
    let out = run_trait_at(&paths, &args(&["unlock", "memory_fade"])).expect("unlock");
    assert!(out.contains("已解锁"), "{out}");
    assert!(out.contains("显式动作"), "{out}");
    let locks = load_trait_locks(&paths.locks_path()).expect("locks persisted");
    assert!(!locks.any_locked(), "解锁恢复 (且只能显式解)");

    // 坏滑杆名 / 坏动作: 拒绝即帧 + 用法。
    let error = run_trait_at(&paths, &args(&["lock", "nope"])).unwrap_err();
    assert!(error.contains("unknown slider"), "{error}");
    assert!(error.contains("sliders:"), "{error}");
    let error = run_trait_at(&paths, &args(&["unlock"])).unwrap_err();
    assert!(error.contains("unknown action"), "{error}");
    assert_eq!(USAGE.lines().next(), Some("apeireth trait report"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// ② 周报数值从成长计既有账真值打印: 7 天窗 / 移动量 / 原因 / 锁定态。
#[test]
fn trait_report_prints_the_seven_day_window_from_the_growth_ledger() {
    let dir = temp_dir("report");
    let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
    let now = now_epoch_ms();
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
            param: TunableParam::ToneSaturation,
            previous: 1.0,
            next: 0.8,
            reason: "语气反馈: 情绪过饱和, 饱和度下调".into(),
            at_epoch_ms: now - 2 * DAY_MS,
        },
    );
    append_row(
        &paths.log_path,
        &TuningRecord {
            seq: 3,
            param: TunableParam::MemoryFade,
            previous: 1.25,
            next: 5.0,
            reason: "窗外的沉积 (8 天前)".into(),
            at_epoch_ms: now - 8 * DAY_MS,
        },
    );
    append_row(
        &paths.log_path,
        &TuningRecord {
            seq: 4,
            param: TunableParam::ToneSaturation,
            previous: 0.8,
            next: 0.8,
            reason: "已锁定, 跳过沉积 (审计留痕): 语气反馈: 过于平淡".into(),
            at_epoch_ms: now - DAY_MS,
        },
    );
    run_trait_at(&paths, &args(&["lock", "tone_saturation"])).expect("lock tone");

    let out = run_trait_at(&paths, &args(&["report"])).expect("report");
    assert!(out.contains("性格周报 · 近 7 天"), "{out}");
    assert!(out.contains("遗忘衰减强度 (memory_fade)"), "{out}");
    assert!(out.contains("移动 +0.250"), "{out}");
    assert!(out.contains("+0.250 @ "), "最近移动带时刻");
    assert!(out.contains("遗忘衰减上调一档"), "移动原因随账真值");
    assert!(out.contains("过饱和, 饱和度下调"), "移动原因随账真值");
    assert!(out.contains("最近移动: 无"), "没动过的杆如实报无");
    assert!(out.contains("锁定跳过 1 条"), "锁定跳过留痕单独计数");
    assert!(out.contains("[已锁定] 语气情绪饱和度"), "锁定态如实");
    assert!(
        out.contains("账面: 窗内记录 3 条"),
        "窗内记录数 (8 天前的不算)"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// ③ 只读档: `tool.trait_report` 默认 grant 且零审批 (与 `tool.self_status`
/// 同档), 生产面真调用出周报帧。
#[tokio::test]
async fn trait_report_is_granted_by_default_with_zero_approval() {
    use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
    use apeireth_governance::{Action, Decision, GovernanceRequest};
    use apeireth_plugin::ToolCapability;
    use apeireth_protocol::canonical::ToolCall;

    let dir = temp_dir("grant");
    std::env::set_var("APEIRETH_SESSION_DB", dir.join("sessions.sqlite3"));
    std::env::set_var("APEIRETH_COGNITIVE_DB", dir.join("cognitive.sqlite3"));

    let runtime: apeireth_runtime::Runtime = apeireth_cli::build_canonical_runtime_from_env()
        .await
        .expect("keyless production runtime builds without network");

    let capability = CapabilityId::new("tool.trait_report").expect("capability id");
    let arguments = serde_json::json!({});
    let request = GovernanceRequest::new(
        Action::CapabilityDispatch {
            capability: &capability,
            arguments: &arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    );
    let verdict = runtime.governance().evaluate_verbose(&request).await;
    assert!(
        verdict.decision.is_allowed(),
        "只读性格周报必须默认可用且零审批: {}",
        verdict.decision
    );
    assert!(
        !matches!(verdict.decision, Decision::RequireApproval { .. }),
        "tool.trait_report 不得要求审批: {}",
        verdict.decision
    );

    // 生产面真调用: 注册 + 周报帧 (write_face = read_only)。
    let tool = runtime
        .capability_registry()
        .entries()
        .into_iter()
        .map(|(_owner, capability)| capability)
        .find(|capability| capability.id().as_str() == "tool.trait_report")
        .expect("tool.trait_report must be registered");
    let call = ToolCall {
        id: "call_trait_report".into(),
        name: "trait_report".into(),
        arguments: serde_json::json!({}),
    };
    let result = tool.invoke(&call).await;
    assert!(result.is_ok(), "{}", result.render());
    let frame: serde_json::Value = serde_json::from_str(&result.render()).expect("frame json");
    assert_eq!(frame["write_face"], "read_only");
    assert_eq!(frame["window_days"], 7);
    assert_eq!(frame["sliders"].as_array().expect("sliders").len(), 4);

    let _ = std::fs::remove_dir_all(&dir);
}

/// ④ 文档随行: 用户手册写明工具面与命令面 (接口可查)。
#[test]
fn user_manual_documents_the_trait_lock_and_report_faces() {
    let manual = include_str!("../../../../docs/02-guides/user-manual.md");
    assert!(
        manual.contains("tool.trait_report"),
        "user-manual 必须写明性格周报工具面"
    );
    assert!(
        manual.contains("apeireth trait"),
        "user-manual 必须写明 trait 命令面"
    );
    assert!(
        manual.contains("trait-locks.json"),
        "user-manual 必须写明锁定态持久档"
    );
    assert!(
        manual.contains("只读") && manual.contains("零审批"),
        "user-manual 必须写明只读零审批档"
    );
}

/// 锁定档路径派生口径: 与成长账同目录 (默认落位跟随 session db 目录)。
#[test]
fn trait_locks_path_sits_beside_the_growth_ledger() {
    assert_eq!(
        trait_locks_path_for(Path::new("data/tuning-log.jsonl")),
        Path::new("data").join(TRAIT_LOCKS_FILE)
    );
}
