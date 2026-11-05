//! `apeireth crypto` 面的**旗标层契约** (crate 级共享判据 `apeireth_cli::flags`
//! 的第七处收口面: `crypto_keys.rs`)。
//!
//! 本文件钉的是**旗标行为** (不是密钥材料语义): 判据一律走 crate 级共享模块,
//! 本面只留自己的措辞 (英文档: 本面用法帧是英文的) 与用法帧。逐条对应共享模块
//! 声明的五条口径 + 本面的三条自有纪律:
//!
//! 1. **未知项** —— 拒绝即帧: 点名 + `--` 分隔提示 + 白名单提示 + 用法帧;
//! 2. **缺值** —— 值缺失 / 值本身像旗标 ⇒ 报「requires a value」(不是「未知参数」);
//! 3. **`--` 之后** —— 一律位置参数 (不再做旗标解析);
//! 4. **负数是值** —— `-1` 不是旗标 (不被当旗标拒, 也不报缺值);
//! 5. **布尔旗标** —— 本面**不收** ⇒ `--json` 指名拒, 且不被当路径吞掉;
//! 6. **重复即拒** —— 同一旗标给两次 ⇒ 拒绝即帧 (不静默取末位: 密钥落点选错 =
//!    密钥写错文件);
//! 7. **非空值** —— 值不得是空串/纯空白 (空路径 = 落到当前目录);
//! 8. **位置参数计数** —— 本面**不收**任何位置参数 (词类参数指名拒);
//! 9. **语言档** —— 错误帧与本面用法帧同语言 (说 "unknown argument", 不说中文)。
//!
//! ⚠️ 本面是**密钥材料面**: 共享层比本面旧行为更宽松的两处 (允许空值 /
//! 允许重复) 必须由本面自有纪律补回 —— 本文件第 ⑥ ⑦ 两节逐条钉死。

use apeireth_cli::crypto_keys::{run_crypto, USAGE};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// 旗标层错误 (未知项 / 缺值) 的共同特征: 点名 + `--` 分隔提示 + 用法帧。
fn assert_flag_frame(error: &str, named: &str) {
    assert!(
        error.contains("unknown argument"),
        "说 unknown argument (本面把旗标与位置词类同称参数): {error}"
    );
    assert!(error.contains(named), "点名被拒的那一项: {error}");
    assert!(error.contains("`--`"), "给出 `--` 分隔提示: {error}");
    assert!(error.contains(USAGE), "带用法帧: {error}");
}

/// 旗标层就拒 ⇒ 不得落到密钥本体 (旗标不得被静默当成路径去落盘/去读)。
fn assert_stopped_at_flag_layer(error: &str) {
    assert!(
        !error.contains("refused"),
        "旗标层就拒, 不得落到密钥本体 (不得静默把旗标当路径): {error}"
    );
}

// ---------------------------------------------------------------------------
// ① 未知项 —— 拒绝即帧 (三个动词同口径)。
// ---------------------------------------------------------------------------

#[test]
fn unknown_flags_are_refused_and_named_in_every_verb() {
    for (verb, tail) in [
        ("wrap", vec!["--nope"]),
        ("recovery-export", vec!["--nope"]),
        ("unlock", vec!["--nope"]),
    ] {
        let mut argv = vec![verb];
        argv.extend(tail);
        let error = run_crypto(&args(&argv)).expect_err("未知旗标必须拒");
        assert_flag_frame(&error, "--nope");
        assert_stopped_at_flag_layer(&error);
    }
}

/// 未知项的帧带**白名单提示** (说清本动词只认哪些旗标)。
#[test]
fn unknown_flag_frame_lists_the_accepted_flags() {
    let error = run_crypto(&args(&["wrap", "--nope"])).expect_err("未知旗标必须拒");
    assert!(
        error.contains("(crypto wrap accepts --partition / --keys / --recovery-out)"),
        "列出本动词只认的旗标: {error}"
    );
    // 白名单按动词走 (recovery-export 只认 --keys / --out)。
    let error = run_crypto(&args(&["recovery-export", "--nope"])).expect_err("未知旗标必须拒");
    assert!(
        error.contains("(crypto recovery-export accepts --keys / --out)"),
        "白名单按动词分: {error}"
    );
}

/// 本面**不收**布尔旗标 ⇒ `--json` 指名拒 (不被当路径吞掉, 也不被静默忽略)。
#[test]
fn boolean_flags_are_not_accepted_by_the_crypto_faces() {
    for argv in [
        vec!["wrap", "--json"],
        vec!["recovery-export", "--json"],
        vec!["unlock", "--json"],
    ] {
        let error = run_crypto(&args(&argv)).expect_err("布尔旗标必须拒");
        assert_flag_frame(&error, "--json");
        assert_stopped_at_flag_layer(&error);
    }
}

/// 元旗标 (`--help` / `-h`) 在本面不是位置参数 ⇒ 指名拒 (不静默变成路径)。
#[test]
fn meta_flags_are_refused_by_the_crypto_faces() {
    for argv in [vec!["wrap", "--help"], vec!["unlock", "-h"]] {
        let error = run_crypto(&args(&argv)).expect_err("元旗标必须拒");
        assert_flag_frame(&error, if argv[1] == "-h" { "-h" } else { "--help" });
    }
}

// ---------------------------------------------------------------------------
// ② 缺值 —— 缺失 / 值像旗标 一律「requires a value」。
// ---------------------------------------------------------------------------

#[test]
fn missing_value_is_reported_as_missing_value() {
    let error = run_crypto(&args(&["wrap", "--partition"])).expect_err("缺值必须拒");
    assert!(error.contains("requires a value"), "说缺值: {error}");
    assert!(error.contains("--partition"), "点名旗标: {error}");
    assert!(error.contains(USAGE), "带用法帧: {error}");
    assert!(
        !error.contains("unknown argument"),
        "缺值不是未知参数: {error}"
    );
}

/// 值本身像旗标 ⇒ 同样是「缺值」, 且点名撞上的那一项 (不把 `--keys` 当分区 id)。
#[test]
fn flag_shaped_value_is_reported_as_missing_value() {
    let error =
        run_crypto(&args(&["wrap", "--partition", "--keys", "keys.json"])).expect_err("缺值必须拒");
    assert!(error.contains("requires a value"), "说缺值: {error}");
    assert!(error.contains("--keys"), "点名撞上的那一项: {error}");
    assert!(
        !error.contains("unknown argument"),
        "缺值不是未知参数: {error}"
    );
    assert_stopped_at_flag_layer(&error);
}

/// 三个动词的缺值同口径 (白名单按动词走)。
#[test]
fn missing_value_is_refused_in_every_verb() {
    for (verb, flag) in [
        ("wrap", "--partition"),
        ("recovery-export", "--keys"),
        ("unlock", "--keys"),
    ] {
        let error = run_crypto(&args(&[verb, flag])).expect_err("缺值必须拒");
        assert!(error.contains("requires a value"), "说缺值: {error}");
        assert!(error.contains(flag), "点名旗标: {error}");
    }
}

// ---------------------------------------------------------------------------
// ③ `--` 之后 —— 一律位置参数 (不再做旗标解析)。
// ---------------------------------------------------------------------------

#[test]
fn double_dash_ends_flag_parsing_for_the_crypto_faces() {
    // `--` 之后的 `--nope` 是位置参数 (本面不收位置参数 ⇒ 指名拒为未知参数),
    // 不是未知旗标 ⇒ 错误帧**不说** unknown argument 之外的旗标分类差。
    let error = run_crypto(&args(&["wrap", "--", "--nope"])).expect_err("位置参数必须拒");
    assert!(
        error.contains("unknown argument: --nope"),
        "`--` 之后的词当位置参数 (指名拒): {error}"
    );
    assert!(error.contains(USAGE), "带用法帧: {error}");
    assert!(
        !error.contains("requires a value"),
        "`--` 之后不做旗标解析 (不报缺值): {error}"
    );
}

// ---------------------------------------------------------------------------
// ④ 负数是值 —— 不是旗标 (不被当旗标拒, 也不报缺值)。
// ---------------------------------------------------------------------------

#[test]
fn negative_numbers_are_values_not_flags() {
    // `--partition -1`: 负数是值 ⇒ 旗标层放行 (报的是后面的必填旗标缺失,
    // 不是缺值 / 不是未知项)。
    let error = run_crypto(&args(&["wrap", "--partition", "-1"])).expect_err("缺必填旗标");
    assert!(
        !error.contains("requires a value"),
        "负数是值 (不报缺值): {error}"
    );
    assert!(!error.contains("unknown argument"), "负数不是旗标: {error}");
    assert!(
        error.contains("--keys"),
        "负数当取值放行, 报的是必填旗标缺失: {error}"
    );
}

// ---------------------------------------------------------------------------
// ⑤ 位置参数计数 —— 本面自有纪律 (不收任何位置参数)。
// ---------------------------------------------------------------------------

#[test]
fn positional_arguments_are_refused_by_the_crypto_faces() {
    for argv in [
        vec!["wrap", "extra"],
        vec!["recovery-export", "extra"],
        vec!["unlock", "extra"],
    ] {
        let error = run_crypto(&args(&argv)).expect_err("位置参数必须拒");
        assert!(
            error.contains("unknown argument: extra"),
            "词类参数指名拒: {error}"
        );
        assert!(error.contains(USAGE), "带用法帧: {error}");
        assert_stopped_at_flag_layer(&error);
    }
}

/// 必填旗标缺失 = 用法帧 (不静默用空值继续)。
#[test]
fn missing_required_flags_are_refused_with_a_usage_frame() {
    let error = run_crypto(&args(&["wrap", "--partition", "user-a"])).expect_err("缺必填旗标");
    assert!(
        error.contains("missing flag: --keys"),
        "点名缺哪个旗标: {error}"
    );
    assert!(error.contains(USAGE), "带用法帧: {error}");
    assert_stopped_at_flag_layer(&error);
}

// ---------------------------------------------------------------------------
// ⑥ 重复即拒 —— 本面自有纪律 (不静默取末位: 密钥落点选错 = 密钥写错文件)。
// ---------------------------------------------------------------------------

#[test]
fn repeated_flags_are_refused() {
    for (argv, flag) in [
        (
            vec!["wrap", "--keys", "a.json", "--keys", "b.json"],
            "--keys",
        ),
        (
            vec!["recovery-export", "--out", "a.txt", "--out", "b.txt"],
            "--out",
        ),
        (
            vec!["unlock", "--recovery", "a.hex", "--recovery", "b.hex"],
            "--recovery",
        ),
    ] {
        let error = run_crypto(&args(&argv)).expect_err("重复旗标必须拒");
        assert!(error.contains("given twice"), "说重复: {error}");
        assert!(error.contains(flag), "点名重复的旗标: {error}");
        assert!(error.contains(USAGE), "带用法帧: {error}");
        assert_stopped_at_flag_layer(&error);
    }
}

/// 重复 + 空值同时出现时仍是**拒绝即帧** (不放宽任何一条拒绝路径)。
#[test]
fn repeated_flag_with_an_empty_value_is_still_refused() {
    let error =
        run_crypto(&args(&["wrap", "--keys", "", "--keys", "b.json"])).expect_err("重复旗标必须拒");
    assert!(error.contains(USAGE), "带用法帧: {error}");
    assert_stopped_at_flag_layer(&error);
    assert!(
        error.contains("given twice") || error.contains("requires a non-empty value"),
        "重复 / 空值两条纪律各自可辨: {error}"
    );
}

// ---------------------------------------------------------------------------
// ⑦ 非空值 —— 本面自有纪律 (空路径 = 落到当前目录)。
// ---------------------------------------------------------------------------

#[test]
fn empty_values_are_refused() {
    for argv in [
        vec!["wrap", "--keys", ""],
        vec!["wrap", "--keys", "   "],
        vec!["recovery-export", "--out", ""],
        vec!["unlock", "--keys", "\t"],
    ] {
        let error = run_crypto(&args(&argv)).expect_err("空值必须拒");
        assert!(
            error.contains("requires a non-empty value"),
            "说非空值: {error}"
        );
        assert!(error.contains(USAGE), "带用法帧: {error}");
        assert_stopped_at_flag_layer(&error);
    }
}

/// 空值与缺值是**两条**纪律 (措辞各自可辨: 空值说 non-empty, 缺值说 requires a value)。
#[test]
fn empty_value_and_missing_value_have_distinct_wordings() {
    let empty = run_crypto(&args(&["wrap", "--keys", ""])).expect_err("空值必须拒");
    assert!(
        empty.contains("requires a non-empty value"),
        "空值: {empty}"
    );
    assert!(
        !empty.contains("requires a value (next"),
        "空值不是缺值: {empty}"
    );
    let missing = run_crypto(&args(&["wrap", "--keys"])).expect_err("缺值必须拒");
    assert!(missing.contains("requires a value"), "缺值: {missing}");
    assert!(
        !missing.contains("non-empty"),
        "缺值不是空值 (两条纪律不混): {missing}"
    );
}

// ---------------------------------------------------------------------------
// ⑧ 语言档 —— 错误帧与本面用法帧同语言 (英文)。
// ---------------------------------------------------------------------------

#[test]
fn error_frames_follow_the_english_style_of_this_face() {
    let error = run_crypto(&args(&["wrap", "--nope"])).expect_err("未知旗标必须拒");
    assert!(error.contains("unknown argument"), "英文档: {error}");
    assert!(!error.contains("未知旗标"), "本面用法帧是英文的: {error}");
    assert!(!error.contains("未知参数"), "本面用法帧是英文的: {error}");
    // 缺值帧同样英文。
    let error = run_crypto(&args(&["wrap", "--partition"])).expect_err("缺值必须拒");
    assert!(error.contains("requires a value"), "英文档: {error}");
    assert!(!error.contains("缺值"), "英文档: {error}");
}

// ---------------------------------------------------------------------------
// ⑨ 真实二进制 —— CLI 可观测面 (退出码 + 错误帧)。
// ---------------------------------------------------------------------------

#[test]
fn real_binary_refuses_an_unknown_crypto_flag_with_an_error_frame() {
    let home = tempfile::tempdir().expect("temp dir");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_apeireth"))
        .args(["crypto", "wrap", "--nope"])
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .output()
        .expect("failed to execute cli binary");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "旗标错 = 非零退出: {stderr}");
    assert!(stderr.contains("--nope"), "错误帧须指出旗标名: {stderr}");
    assert!(
        stderr.contains("`--`"),
        "错误帧须提示 -- 分隔用法: {stderr}"
    );
    assert!(stderr.contains("Usage:"), "错误帧须带用法帧: {stderr}");
}
