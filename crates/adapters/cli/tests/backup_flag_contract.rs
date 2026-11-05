//! G5 备份恢复 CLI 面的**旗标层契约** (crate 级共享判据 `apeireth_cli::flags`
//! 的第六处收口面: `backup.rs`)。
//!
//! 本文件钉的是**旗标行为** (不是备份本体语义): 判据一律走 crate 级共享模块,
//! 本面只留自己的措辞 (英文档: 用法帧是英文的) 与用法帧。逐条对应共享模块
//! 声明的五条口径 + 本面的三条自有纪律:
//!
//! 1. **未知旗标** —— 拒绝即帧: 点名旗标 + `--` 分隔提示 + 用法帧;
//! 2. **缺值** —— `--to` 缺值 / 值本身像旗标 ⇒ 报「缺值」(不是「未知参数」);
//! 3. **`--` 之后** —— 一律位置参数 (不再做旗标解析);
//! 4. **负数是值** —— `-1` 不是旗标 (不被当旗标拒);
//! 5. **布尔旗标** —— 本面**不收** ⇒ `--json` 指名拒, 且不被当路径吞掉;
//! 6. **重复 `--to`** —— 拒绝即帧 (不静默取末位: 搬家恢复落点选错 = 恢复落错根);
//! 7. **语言档** —— 错误帧与本面用法帧同语言 (说 "unknown flag", 不说中文)。

use apeireth_cli::backup::{run_backup_at, BackupCliPaths, USAGE};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// 临时数据根 (本文件只验旗标层, 不碰真实数据面)。
fn temp_paths(tag: &str) -> (tempfile::TempDir, BackupCliPaths) {
    let dir = tempfile::Builder::new()
        .prefix(&format!("apeireth-backup-flags-{tag}-"))
        .tempdir()
        .expect("temp dir");
    let root = dir.path();
    let paths = BackupCliPaths::new(
        root,
        root.join("sessions.sqlite3"),
        root.join("cognitive.sqlite3"),
    );
    (dir, paths)
}

/// 旗标层错误 (未知旗标 / 缺值) 的共同特征: 点名 + `--` 分隔提示 + 用法帧。
fn assert_flag_frame(error: &str, named: &str) {
    assert!(error.contains("unknown flag"), "说 unknown flag: {error}");
    assert!(error.contains(named), "点名被拒的旗标: {error}");
    assert!(error.contains("`--`"), "给出 `--` 分隔提示: {error}");
    assert!(error.contains(USAGE), "带用法帧: {error}");
}

/// 旗标层就拒 ⇒ 不得落到引擎 (旗标不得被静默当成路径去落盘/去读)。
fn assert_stopped_at_flag_layer(error: &str) {
    assert!(
        !error.contains("backup manifest is corrupt")
            && !error.contains("backup operation refused")
            && !error.contains("backup io failed"),
        "旗标层就拒, 不得落到引擎 (不得静默把旗标当路径): {error}"
    );
}

// ---------------------------------------------------------------------------
// ① 未知旗标 —— 拒绝即帧 (三个动词同口径)。
// ---------------------------------------------------------------------------

#[test]
fn unknown_flags_are_refused_and_named_in_every_verb() {
    let (_dir, paths) = temp_paths("unknown");
    for (verb, tail) in [
        ("export", vec!["--nope"]),
        ("verify", vec!["--nope"]),
        ("restore", vec!["bundle", "--nope"]),
    ] {
        let mut argv = vec![verb];
        argv.extend(tail.clone());
        let error = run_backup_at(&paths, &args(&argv)).expect_err("未知旗标必须拒");
        assert_flag_frame(&error, "--nope");
        assert_stopped_at_flag_layer(&error);
    }
}

/// 本面**不收**布尔旗标 ⇒ `--json` 指名拒 (不被当路径吞掉, 也不被静默忽略)。
#[test]
fn boolean_flags_are_not_accepted_by_the_backup_faces() {
    let (_dir, paths) = temp_paths("boolean");
    for argv in [
        vec!["export", "--json"],
        vec!["restore", "bundle", "--json"],
    ] {
        let error = run_backup_at(&paths, &args(&argv)).expect_err("布尔旗标必须拒");
        assert_flag_frame(&error, "--json");
    }
}

// ---------------------------------------------------------------------------
// ② 缺值 —— 缺失 / 值像旗标 一律「缺值」 (不是「未知参数」也不是「未知旗标」)。
// ---------------------------------------------------------------------------

#[test]
fn missing_value_is_reported_as_missing_value() {
    let (_dir, paths) = temp_paths("missing");
    let error =
        run_backup_at(&paths, &args(&["restore", "bundle", "--to"])).expect_err("缺值必须拒");
    assert!(error.contains("requires a value"), "说缺值: {error}");
    assert!(error.contains("--to"), "点名旗标: {error}");
    assert!(error.contains(USAGE), "带用法帧: {error}");
}

/// 值本身像旗标 ⇒ 同样是「缺值」, 且点名撞上的那一项 (不把 `--json` 当落点根)。
#[test]
fn flag_shaped_value_is_reported_as_missing_value() {
    let (_dir, paths) = temp_paths("flagged-value");
    let error = run_backup_at(&paths, &args(&["restore", "bundle", "--to", "--nope"]))
        .expect_err("缺值必须拒");
    assert!(error.contains("requires a value"), "说缺值: {error}");
    assert!(error.contains("--nope"), "点名撞上的那一项: {error}");
    assert!(!error.contains("unknown flag"), "缺值不是未知旗标: {error}");
}

/// 值层 (面自己的纪律): 落点根不得是空串/纯空白。
#[test]
fn empty_relocation_root_is_refused() {
    let (_dir, paths) = temp_paths("empty-root");
    let error = run_backup_at(&paths, &args(&["restore", "bundle", "--to", "   "]))
        .expect_err("空根必须拒");
    assert!(error.contains("non-empty root"), "说空根: {error}");
    assert!(error.contains(USAGE), "带用法帧: {error}");
}

// ---------------------------------------------------------------------------
// ③ `--` 之后 —— 一律位置参数 (不再做旗标解析)。
// ---------------------------------------------------------------------------

#[test]
fn double_dash_ends_flag_parsing_for_the_backup_faces() {
    let (_dir, paths) = temp_paths("double-dash");
    // `--` 之后的 `--nope` 是位置参数 (bundle), 不是未知旗标 ⇒ 不报 unknown flag。
    let error =
        run_backup_at(&paths, &args(&["restore", "--", "--nope"])).expect_err("bundle 不存在");
    assert!(
        !error.contains("unknown flag"),
        "`--` 之后不当旗标: {error}"
    );
    assert!(
        error.contains("--nope/manifest.json"),
        "`--` 之后的词当位置参数送进引擎 (当 bundle 路径): {error}"
    );
}

// ---------------------------------------------------------------------------
// ④ 负数是值 —— 不是旗标 (不被当旗标拒, 也不报缺值)。
// ---------------------------------------------------------------------------

#[test]
fn negative_numbers_are_values_not_flags() {
    let (_dir, paths) = temp_paths("negative");
    // `-1` 当 bundle 路径 (值), 不是旗标。
    let error = run_backup_at(&paths, &args(&["restore", "-1"])).expect_err("bundle 不存在");
    assert!(!error.contains("unknown flag"), "负数不是旗标: {error}");
    assert!(!error.contains("requires a value"), "负数不是缺值: {error}");
    assert!(
        error.contains("-1/manifest.json"),
        "负数原样当路径送进引擎: {error}"
    );
    // `--to -1`: 负数是值 ⇒ 当落点根 (不是缺值), 旗标层放行到引擎。
    let error = run_backup_at(&paths, &args(&["restore", "bundle", "--to", "-1"]))
        .expect_err("bundle 不存在");
    assert!(!error.contains("requires a value"), "负数当值: {error}");
    assert!(!error.contains("unknown flag"), "负数不是旗标: {error}");
    assert!(
        error.contains("bundle/manifest.json"),
        "旗标层放行 (负数当落点根): {error}"
    );
}

// ---------------------------------------------------------------------------
// ⑤ 重复 `--to` —— 拒绝即帧 (搬家恢复落点选错 = 恢复落错根)。
// ---------------------------------------------------------------------------

#[test]
fn repeated_relocation_root_is_refused() {
    let (_dir, paths) = temp_paths("repeat");
    let error = run_backup_at(
        &paths,
        &args(&["restore", "bundle", "--to", "/a", "--to", "/b"]),
    )
    .expect_err("重复旗标必须拒");
    assert!(error.contains("given twice"), "说重复: {error}");
    assert!(error.contains(USAGE), "带用法帧: {error}");
}

// ---------------------------------------------------------------------------
// ⑥ 位置参数计数 —— 本面自有纪律 (只收一个 <BUNDLE> / <DIR>)。
// ---------------------------------------------------------------------------

#[test]
fn positional_counts_are_enforced_per_verb() {
    let (_dir, paths) = temp_paths("counts");
    let cases: [(&[&str], &str); 6] = [
        (&["export"], "backup export requires exactly one <DIR>"),
        (
            &["export", "a", "b"],
            "backup export requires exactly one <DIR>",
        ),
        (&["verify"], "backup verify requires exactly one <BUNDLE>"),
        (
            &["verify", "a", "b"],
            "backup verify requires exactly one <BUNDLE>",
        ),
        (&["restore"], "backup restore requires a <BUNDLE>"),
        (
            &["restore", "a", "b"],
            "backup restore takes exactly one <BUNDLE>",
        ),
    ];
    for (argv, expected) in cases {
        let error = run_backup_at(&paths, &args(argv)).expect_err("参数个数错必须拒");
        assert!(error.contains(expected), "说清缺什么: {error}");
        assert!(error.contains(USAGE), "带用法帧: {error}");
    }
    // 空串按缺 (与 export/verify 同口径)。
    let error = run_backup_at(&paths, &args(&["restore", ""])).expect_err("空 bundle 必须拒");
    assert!(error.contains(USAGE), "带用法帧: {error}");
}

// ---------------------------------------------------------------------------
// ⑦ 语言档 —— 错误帧与本面用法帧同语言 (英文)。
// ---------------------------------------------------------------------------

#[test]
fn error_frames_follow_the_english_style_of_this_face() {
    let (_dir, paths) = temp_paths("style");
    let error = run_backup_at(&paths, &args(&["export", "--nope"])).expect_err("未知旗标必须拒");
    assert!(error.contains("unknown flag"), "英文档: {error}");
    assert!(!error.contains("未知旗标"), "本面用法帧是英文的: {error}");
    // 未知动词仍是用法帧 (不编造未知动词分类)。
    let error = run_backup_at(&paths, &args(&["nope"])).expect_err("未知动词必须拒");
    assert!(error.contains(USAGE), "未知动词给用法帧: {error}");
}
