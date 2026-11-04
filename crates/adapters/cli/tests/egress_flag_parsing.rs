//! `egress` 三动词的**旗标解析契约**（D4 / D5 / D6 / D7）。
//!
//! 四条口径：
//! 1. **拒绝即帧** —— 带值旗标缺值 ⇒ 报「缺值」（指名旗标），不静默回落默认账本；
//! 2. **未知旗标指名** —— 拼错的 `--xxx` 一律拒绝，错误帧要指名旗标；
//! 3. **布尔旗标不吞位置参数** —— `--json` 不带值，`act-1 confirmed` 必须原样解析；
//! 4. **用法帧列全三个动词**（`ledger` / `impact` / `outcome`）与它们的旗标。
//!
//! 口径补充：`--` 之后的词一律当**位置参数**（不再做旗标解析）；
//! 正常路径（没写错旗标时）的输出格式与账本语义**不变** —— 最后一条测试钉住它。

use std::path::{Path, PathBuf};
use std::process::Command;

use apeireth_cli::egress_command::{run_egress_at, USAGE};
use apeireth_runtime_assembly::accountability::Outcome;
use apeireth_runtime_assembly::consequence::ConsequenceTracker;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("apeireth-egress-flag-{tag}-{}", std::process::id()));
    let _ = fs_err::remove_dir_all(&dir);
    fs_err::create_dir_all(&dir).expect("temp dir");
    dir
}

/// 跑真二进制 —— 验「非零退出」这条口径（库面 Err ⇒ 进程退出码非零）。
fn run_binary(tag: &str, cli_args: &[&str]) -> std::process::Output {
    let dir = temp_dir(tag);
    Command::new(env!("CARGO_BIN_EXE_apeireth"))
        .args(cli_args)
        .env("APEIRETH_DATA_DIR", &dir)
        .output()
        .expect("应能跑二进制")
}

// ---------------------------------------------------------------------------
// ① 悬空带值旗标（`--path` 后面没有值）⇒ 拒绝即帧，指名「缺值」。
// ---------------------------------------------------------------------------

#[test]
fn dangling_value_flag_is_refused() {
    let err = run_egress_at(Path::new("/nonexistent-root"), &args(&["ledger", "--path"]))
        .expect_err("悬空 --path 必须拒绝");
    assert!(err.contains("--path"), "错误帧要指名旗标: {err}");
    assert!(err.contains("缺值"), "错误帧要说缺值: {err}");

    // 缺值**不是**「未知参数」—— 报的是「缺值」。
    assert!(!err.contains("未知旗标"), "缺值不该报成未知旗标: {err}");

    // 真二进制：非零退出 + stderr 指名旗标。
    let output = run_binary("dangling", &["egress", "ledger", "--path"]);
    assert!(!output.status.success(), "退出码应非零");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--path"), "stderr 要指名旗标: {stderr}");
    assert!(stderr.contains("缺值"), "stderr 要说缺值: {stderr}");
}

/// 带值旗标的「值」位置又撞上另一个旗标 ⇒ 同样报缺值（不把 `--json` 当路径）。
#[test]
fn value_flag_followed_by_another_flag_is_refused() {
    let err = run_egress_at(
        Path::new("/nonexistent-root"),
        &args(&["ledger", "--path", "--json"]),
    )
    .expect_err("缺值必须拒绝");
    assert!(err.contains("--path"), "错误帧要指名旗标: {err}");
    assert!(err.contains("缺值"), "错误帧要说缺值: {err}");
}

/// 值**不合法**时报值的错（不是「缺值」）：`--limit -1`。
#[test]
fn invalid_value_reports_value_error_not_missing_value() {
    let err = run_egress_at(
        Path::new("/nonexistent-root"),
        &args(&["ledger", "--limit", "-1"]),
    )
    .expect_err("负数条数必须拒绝");
    assert!(err.contains("--limit"), "错误帧要指名旗标: {err}");
    assert!(err.contains("非负整数"), "错误帧要说值不合法: {err}");
    assert!(!err.contains("缺值"), "值不合法不该报缺值: {err}");
}

// ---------------------------------------------------------------------------
// ② 未知旗标 ⇒ 拒绝即帧，指名旗标。
// ---------------------------------------------------------------------------

#[test]
fn unknown_flag_is_refused() {
    let err = run_egress_at(
        Path::new("/nonexistent-root"),
        &args(&["ledger", "--typo", "xyz"]),
    )
    .expect_err("未知旗标必须拒绝");
    assert!(err.contains("--typo"), "错误帧要指名旗标: {err}");

    // 真二进制：非零退出 + stderr 指名旗标。
    let output = run_binary("unknown", &["egress", "ledger", "--typo", "xyz"]);
    assert!(!output.status.success(), "退出码应非零");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--typo"), "stderr 要指名旗标: {stderr}");
}

/// `impact` 动词同样拒未知旗标（白名单不是只给 `ledger` 用的）。
#[test]
fn unknown_flag_is_refused_on_impact_too() {
    let err = run_egress_at(
        Path::new("/nonexistent-root"),
        &args(&["impact", "--nope", "1"]),
    )
    .expect_err("未知旗标必须拒绝");
    assert!(err.contains("--nope"), "错误帧要指名旗标: {err}");
}

// ---------------------------------------------------------------------------
// ③ 布尔旗标不吞位置参数：`--json act-1 confirmed` 两个位置参数都要解析出来。
// ---------------------------------------------------------------------------

#[test]
fn outcome_boolean_flag_does_not_eat_positional() {
    let dir = temp_dir("outcome-json");
    let ledger = dir.join("consequence-ledger.jsonl");
    let cli = args(&[
        "outcome",
        "--json",
        "act-1",
        "confirmed",
        "--path",
        &ledger.display().to_string(),
    ]);
    let out = run_egress_at(&dir, &cli).expect("应成功");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("--json 应输出合法 JSON");
    assert_eq!(
        parsed["action_id"], "act-1",
        "第一个位置参数应解析出来: {out}"
    );
    assert_eq!(
        parsed["outcome"], "confirmed",
        "第二个位置参数应解析出来: {out}"
    );

    // 落盘可查：回填真的写进了账本（写面语义不变）。
    let tracker = ConsequenceTracker::with_ledger(ledger);
    assert_eq!(
        tracker.current_outcome("act-1"),
        Outcome::Confirmed,
        "后果应已回填进账本"
    );
}

/// 布尔旗标放在**最后**也不该改变位置参数的解析（前后等价）。
#[test]
fn outcome_boolean_flag_trailing_still_parses_positionals() {
    let dir = temp_dir("outcome-json-trailing");
    let ledger = dir.join("consequence-ledger.jsonl");
    let cli = args(&[
        "outcome",
        "act-2",
        "confirmed",
        "--path",
        &ledger.display().to_string(),
        "--json",
    ]);
    let out = run_egress_at(&dir, &cli).expect("应成功");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("--json 应输出合法 JSON");
    assert_eq!(parsed["action_id"], "act-2");
    assert_eq!(parsed["outcome"], "confirmed");
}

// ---------------------------------------------------------------------------
// ④ 用法帧列全三个动词 + 三个动词的旗标。
// ---------------------------------------------------------------------------

#[test]
fn usage_lists_all_three_verbs_and_flags() {
    assert!(USAGE.contains("egress ledger"), "用法帧要列 ledger");
    assert!(USAGE.contains("egress impact"), "用法帧要列 impact");
    assert!(
        USAGE.contains("egress outcome"),
        "用法帧要列 outcome（D7）: {USAGE}"
    );
    for flag in ["--path", "--limit", "--json"] {
        assert!(USAGE.contains(flag), "用法帧要列旗标 {flag}: {USAGE}");
    }
}

/// 出错帧里的用法帧同样列全（出错的人靠它查）。
#[test]
fn error_frame_usage_lists_outcome_verb() {
    let err = run_egress_at(Path::new("/nonexistent-root"), &args(&["bogus"]))
        .expect_err("未知动词必须拒绝");
    assert!(err.contains("unknown egress command"), "{err}");
    assert!(
        err.contains("egress outcome"),
        "错误帧的用法帧要含 outcome 动词（D7）: {err}"
    );
}

// ---------------------------------------------------------------------------
// ⑤ `--` 之后的内容不被当旗标。
// ---------------------------------------------------------------------------

#[test]
fn double_dash_ends_flag_parsing() {
    let dir = temp_dir("double-dash");
    let ledger = dir.join("consequence-ledger.jsonl");
    let path = ledger.display().to_string();

    // `--` 之后的两个词是位置参数（旗标要写在 `--` 之前）。
    let out = run_egress_at(
        &dir,
        &args(&["outcome", "--path", &path, "--", "act-1", "confirmed"]),
    )
    .expect("应成功");
    assert!(out.contains("已回填"), "回填帧应照常: {out}");
    assert!(out.contains("act-1"), "位置参数应解析出来: {out}");

    // `--` 之后的 `--json` 是**位置参数**，不是布尔旗标 ⇒ 位置参数变成三个 ⇒ 拒绝。
    let err = run_egress_at(
        &dir,
        &args(&[
            "outcome",
            "--path",
            &path,
            "--",
            "--json",
            "act-1",
            "confirmed",
        ]),
    )
    .expect_err("`--` 之后不该再把 --json 当旗标");
    assert!(
        err.contains("需要两个位置参数"),
        "错误帧应说明位置参数个数不对: {err}"
    );

    // 同理：`ledger -- --json` 不是「开 JSON 输出」，而是多了一个位置参数。
    let err = run_egress_at(
        Path::new("/nonexistent-root"),
        &args(&["ledger", "--", "--json"]),
    )
    .expect_err("`--` 之后不该再把 --json 当旗标");
    assert!(
        err.contains("不接受位置参数"),
        "错误帧应说明不接受位置参数: {err}"
    );
}

// ---------------------------------------------------------------------------
// 正常路径不变：输出格式与账本语义一字不改。
// ---------------------------------------------------------------------------

#[test]
fn normal_paths_keep_output_shape() {
    let dir = temp_dir("normal");
    let ledger = dir.join("consequence-ledger.jsonl");
    let path = ledger.display().to_string();

    // `outcome`（唯一写面）：人读帧不变。
    let out = run_egress_at(
        &dir,
        &args(&["outcome", "act-1", "confirmed", "--path", &path]),
    )
    .expect("应成功");
    assert!(
        out.contains("已回填：行动 act-1 的后果 = confirmed"),
        "{out}"
    );
    assert!(out.contains("来源：用户确认"), "{out}");

    // `ledger`：空账本的人读帧不变（不谎称「没外流过」）。
    let out = run_egress_at(
        &dir,
        &args(&[
            "ledger",
            "--path",
            &dir.join("empty-ledger.jsonl").display().to_string(),
        ]),
    )
    .expect("应成功");
    assert!(out.contains("账本为空"), "{out}");
    assert!(out.contains("只等于「没有记录」"), "{out}");

    // `ledger --json`：JSON 数组不变。
    let out = run_egress_at(
        &dir,
        &args(&[
            "ledger",
            "--path",
            &dir.join("empty-ledger.jsonl").display().to_string(),
            "--limit",
            "3",
            "--json",
        ]),
    )
    .expect("应成功");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("应为合法 JSON");
    assert!(parsed.is_array(), "--json 应输出数组: {out}");
}
