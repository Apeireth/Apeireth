//! `apeireth trait report --json` 的 JSON 出口面 (D9: 与 `exposure report --json` /
//! `egress ledger --json` 对称) + 错误文案准确性 (多余参数 / 未知旗标指名拒)。
//!
//! 覆盖四件事:
//! ① `trait report --json` ⇒ 退出码 0 且 stdout 是**可解析的 JSON** (字段名与
//!    `tool.trait_report` 工具帧同构: 名字 / 锁定态 / 各项指标一项不少);
//! ② `trait report` (不带旗标) ⇒ 仍是人读形态, 退出 0;
//! ③ 多余参数 ⇒ 报「多余参数」且**非零**退出 (不再冒充 "unknown action");
//! ④ 未知旗标 ⇒ 拒绝即帧、**指名**旗标、非零退出。
//!
//! 数据一律用**测试内构造的固定账** (`tuning-log.jsonl` 逐记录信封),
//! 不依赖真实用户档案: 二进制面用 `APEIRETH_SESSION_DB` 把成长账钉到临时目录。

use std::path::{Path, PathBuf};
use std::process::Command;

use apeireth_cli::trait_command::run_trait_at;
use apeireth_orchestration::self_tuning::{
    TunableParam, TuningRecord, DAY_MS, TRAIT_LOCKED_SKIP_PREFIX,
};
use apeireth_runtime_assembly::{
    now_epoch_ms, TraitGrowthPaths, TUNING_LOG_DOC_NAME, TUNING_LOG_DOC_VERSION, TUNING_LOG_FILE,
};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-trait-json-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// 追加一行成长账 (逐记录信封, 与写端同格式)。
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

/// 写一份固定成长账: 两条实际移动 + 一条锁定跳过留痕 (窗外一条不计)。
fn seed_ledger(dir: &Path) {
    let now = now_epoch_ms();
    let log_path = dir.join(TUNING_LOG_FILE);
    append_row(
        &log_path,
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
        &log_path,
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
        &log_path,
        &TuningRecord {
            seq: 3,
            param: TunableParam::ToneSaturation,
            previous: 0.8,
            next: 0.8,
            reason: format!("{TRAIT_LOCKED_SKIP_PREFIX} (审计留痕): 语气反馈: 过于平淡"),
            at_epoch_ms: now - DAY_MS,
        },
    );
    append_row(
        &log_path,
        &TuningRecord {
            seq: 4,
            param: TunableParam::MemoryFade,
            previous: 1.25,
            next: 5.0,
            reason: "窗外的沉积 (8 天前)".into(),
            at_epoch_ms: now - 8 * DAY_MS,
        },
    );
}

/// 跑一次真二进制 (`apeireth trait …`): 成长账落位钉到临时目录。
fn run_cli(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_apeireth"))
        .arg("trait")
        .args(args)
        .env("APEIRETH_SESSION_DB", dir.join("sessions.sqlite3"))
        .output()
        .expect("run apeireth")
}

/// ① `report --json` ⇒ 退出 0 + stdout 可解析的 JSON (含名字 / 锁定态 / 各项指标)。
#[test]
fn report_json_flag_emits_parseable_json_and_exits_zero() {
    let dir = temp_dir("json");
    seed_ledger(&dir);
    // 锁一根杆: 锁定态必须如实进 JSON (同一份锁定档)。
    let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
    run_trait_at(&paths, &["lock".into(), "tone_saturation".into()]).expect("lock tone_saturation");

    let output = run_cli(&dir, &["report", "--json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "退出码 0 (实际 {:?}); stderr={stderr}",
        output.status.code()
    );

    let frame: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout 必须是可解析的 JSON");
    assert_eq!(frame["write_face"], "read_only");
    assert_eq!(frame["window_days"], 7);
    assert_eq!(
        frame["records_in_window"], 3,
        "窗内 3 条 (8 天前那条不进窗): {frame}"
    );
    assert_eq!(frame["locked_skipped"], 1, "锁定跳过留痕单独计数: {frame}");

    let sliders = frame["sliders"].as_array().expect("sliders 数组");
    assert_eq!(sliders.len(), 4, "四杆一行不多不少: {frame}");
    let fade = &sliders[0];
    assert_eq!(fade["param"], "memory_fade");
    assert_eq!(fade["label"], "遗忘衰减强度", "名字在 JSON 里也在场");
    assert_eq!(fade["locked"], false);
    assert_eq!(fade["move_count"], 1);
    let net = fade["net_delta"].as_f64().expect("net_delta 是数字");
    assert!((net - 0.25).abs() < 1e-9, "净移动量与账一致: {net}");
    assert!(fade["value"].is_number(), "当前生效值在场");
    assert_eq!(
        fade["last_move"]["reason"],
        "检索命中 4/未命中 0: 召回供给充裕, 遗忘衰减上调一档"
    );

    let tone = &sliders[2];
    assert_eq!(tone["param"], "tone_saturation");
    assert_eq!(tone["locked"], true, "锁定态如实进 JSON: {tone}");
    assert_eq!(tone["locked_skipped"], 1);

    let reasons = frame["top_reasons"].as_array().expect("top_reasons 数组");
    let texts: Vec<&str> = reasons
        .iter()
        .map(|row| row["reason"].as_str().expect("reason 字符串"))
        .collect();
    assert!(
        texts.contains(&"检索命中 4/未命中 0: 召回供给充裕, 遗忘衰减上调一档"),
        "移动原因随账真值: {texts:?}"
    );
    assert!(
        !texts
            .iter()
            .any(|text| text.starts_with(TRAIT_LOCKED_SKIP_PREFIX)),
        "锁定跳过留痕不算移动原因: {texts:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// ①b 库面同语义 (路径钉死): `run_trait_at` 的 `--json` 同样出可解析 JSON。
#[test]
fn report_json_flag_is_available_on_the_pinned_path_kernel() {
    let dir = temp_dir("kernel");
    seed_ledger(&dir);
    let paths = TraitGrowthPaths::new(dir.join(TUNING_LOG_FILE));
    let out = run_trait_at(&paths, &["report".into(), "--json".into()]).expect("report --json");
    let frame: serde_json::Value = serde_json::from_str(&out).expect("可解析 JSON");
    assert_eq!(frame["window_days"], 7);
    assert_eq!(frame["sliders"].as_array().expect("sliders").len(), 4);
    assert_eq!(frame["records_in_window"], 3);
    let _ = std::fs::remove_dir_all(&dir);
}

/// ② `report` (不带旗标) ⇒ 仍是人读形态, 退出 0。
#[test]
fn report_without_flag_stays_human_readable_and_exits_zero() {
    let dir = temp_dir("human");
    seed_ledger(&dir);

    let output = run_cli(&dir, &["report"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "退出码 0 (实际 {:?})",
        output.status.code()
    );
    assert!(stdout.contains("性格周报 · 近 7 天"), "{stdout}");
    assert!(stdout.contains("遗忘衰减强度 (memory_fade)"), "{stdout}");
    assert!(stdout.contains("移动 +0.250"), "{stdout}");
    assert!(stdout.contains("移动原因 top:"), "{stdout}");
    assert!(
        !stdout.trim_start().starts_with('{'),
        "不带旗标时不得出 JSON: {stdout}"
    );
    assert!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).is_err(),
        "人读形态不是 JSON: {stdout}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// ③ 多余参数 ⇒ 报「多余参数」且非零退出 (动作已知时不再说 "unknown action")。
#[test]
fn extra_arguments_are_named_as_extra_and_exit_non_zero() {
    let dir = temp_dir("extra");

    let output = run_cli(&dir, &["report", "surplus"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "多余参数必须非零退出 (实际 {:?})",
        output.status.code()
    );
    assert!(stderr.contains("多余参数"), "{stderr}");
    assert!(stderr.contains("surplus"), "指名多余的那一个: {stderr}");
    assert!(
        !stderr.contains("unknown action"),
        "动作已知时报「多余参数」而不是「unknown action」: {stderr}"
    );

    let output = run_cli(&dir, &["lock", "memory_fade", "tone_saturation"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "多余参数必须非零退出 (实际 {:?})",
        output.status.code()
    );
    assert!(stderr.contains("多余参数"), "{stderr}");
    assert!(
        !stderr.contains("unknown action"),
        "动作已知时报「多余参数」而不是「unknown action」: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// ④ 未知旗标 ⇒ 拒绝即帧: 指名旗标 + 非零退出 (不静默忽略)。
#[test]
fn unknown_flags_are_refused_by_name_and_exit_non_zero() {
    let dir = temp_dir("flag");

    let output = run_cli(&dir, &["report", "--nope"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "未知旗标必须非零退出 (实际 {:?})",
        output.status.code()
    );
    assert!(stderr.contains("未知旗标"), "{stderr}");
    assert!(stderr.contains("--nope"), "指名旗标: {stderr}");

    // 悬空/畸形旗标 (`--json` 带值) 同样指名拒。
    let output = run_cli(&dir, &["report", "--json=1"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "畸形旗标必须非零退出 (实际 {:?})",
        output.status.code()
    );
    assert!(stderr.contains("未知旗标"), "{stderr}");
    assert!(stderr.contains("--json=1"), "指名旗标: {stderr}");

    // lock / unlock 没有旗标 ⇒ `--json` 在这两个动作上同样被指名拒。
    let output = run_cli(&dir, &["lock", "--json", "memory_fade"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "lock 不收旗标 (实际 {:?})",
        output.status.code()
    );
    assert!(stderr.contains("未知旗标"), "{stderr}");
    assert!(stderr.contains("--json"), "指名旗标: {stderr}");

    let _ = std::fs::remove_dir_all(&dir);
}
