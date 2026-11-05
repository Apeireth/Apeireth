//! `apeireth user` 面的**旗标层契约** (crate 级共享判据 `apeireth_cli::flags`
//! 的收口面之一: `user_admin.rs`)。
//!
//! 本文件钉的是**旗标行为** (不是用户表语义): 判据一律走 crate 级共享模块,
//! 本面只留自己的措辞 (英文档: 本面用法帧是英文的) 与两条自有纪律
//! (重复即拒 / 不收位置参数)。逐条对应共享模块声明的五条口径 + 本面纪律:
//!
//! 1. **动词先行** —— 动词 (add / list / role / remove) 先分派, 动词**不是**
//!    位置参数; 动词决定本命令的旗标白名单 (逐动词一个 `FlagSpec`);
//! 2. **未知项** —— 拒绝即帧: 点名 + `--` 分隔提示 + 白名单提示 + 用法帧;
//! 3. **缺值** —— 值缺失 / 值本身像旗标 ⇒ 报「requires a value」(不是「未知参数」)
//!    —— ⚠️ 含**短旗标** (`-x`) 与**长旗标** (`--role`) 两档: 旧行为会把旗标当值
//!    吞下 (`--user-id --role owner` ⇒ `--role` 被当成用户 id);
//! 4. **`--` 之后** —— 一律位置参数 (不再做旗标解析);
//! 5. **负数是值** —— `-1` 不是旗标 (不被当旗标拒, 也不报缺值);
//! 6. **布尔旗标** —— 本面**不收** ⇒ `--json` 指名拒, 且不被当取值吞掉;
//! 7. **重复即拒** —— 同一旗标给两次 ⇒ 拒绝即帧 (不静默取末位: 管理动作的
//!    参数选错 = 改错人);
//! 8. **不收位置参数** —— 词类参数指名拒;
//! 9. **语言档** —— 错误帧与本面用法帧同语言 (说 "unknown argument", 不说中文)。
//!
//! ⚠️ 本面旧行为**无**空值判据 ⇒ 本批**不新增** (新增即收口外的收紧, 且
//! `--clients ""` 是合法的空客户端集) —— 用例 `empty_values_keep_the_old_behavior`
//! 把这条口径钉死。

use std::path::Path;

use apeireth_cli::user_admin::run_user_admin_at;
use apeireth_core::stored_doc::StoredDoc;
use apeireth_gateway::{
    ClientTokenConfig, ClientTokenSpec, CLIENT_TOKENS_DOC_NAME, CLIENT_TOKENS_DOC_VERSION,
    CLIENT_TOKENS_FILE,
};

const OWNER_TOKEN: &str = "tok-owner-0123456789abcdef";

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

/// 临时数据目录 (本面的一切写盘都在这里; 旗标层的拒发生在触盘之前)。
fn temp_dir(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("apeireth-user-flag-{tag}-"))
        .tempdir()
        .expect("temp dir")
}

/// 写一份客户端令牌表 (`client-tokens.json`, StoredDoc 信封)。
fn write_tokens(dir: &Path, entries: &[(&str, &str, Option<&str>)]) {
    let clients = entries
        .iter()
        .map(|(name, token, user_id)| ClientTokenSpec {
            name: (*name).to_string(),
            token: (*token).to_string(),
            user_id: user_id.map(str::to_string),
        })
        .collect();
    let doc = StoredDoc::new(
        CLIENT_TOKENS_DOC_NAME,
        CLIENT_TOKENS_DOC_VERSION,
        vec![CLIENT_TOKENS_DOC_VERSION],
        ClientTokenConfig::new(clients),
    );
    fs_err::write(
        dir.join(CLIENT_TOKENS_FILE),
        serde_json::to_vec_pretty(&doc).unwrap(),
    )
    .unwrap();
}

/// 旗标层错误 (未知项 / 缺值) 的共同特征: 点名 + `--` 分隔提示 + 用法帧。
fn assert_flag_frame(error: &str, named: &str) {
    assert!(
        error.contains("unknown argument"),
        "说 unknown argument (本面把旗标与位置词类同称参数): {error}"
    );
    assert!(error.contains(named), "点名被拒的那一项: {error}");
    assert!(error.contains("`--`"), "给出 `--` 分隔提示: {error}");
    assert!(error.contains("Usage:"), "带用法帧: {error}");
}

/// 旗标层就拒 ⇒ 不得落到用户表本体 (不得静默把旗标当取值去建户/改户)。
fn assert_stopped_at_flag_layer(error: &str) {
    assert!(
        !error.contains("refusing to add user"),
        "旗标层就拒, 不得落到用户表本体: {error}"
    );
    assert!(
        !error.contains("owner credential"),
        "旗标层就拒, 不得落到 owner 角色门: {error}"
    );
}

// ---------------------------------------------------------------------------
// ① 动词先行 —— 动词不是位置参数; 动词决定旗标白名单。
// ---------------------------------------------------------------------------

#[test]
fn the_verb_is_parsed_first_and_never_treated_as_a_positional() {
    let dir = temp_dir("verb");
    // 动词被分派到 `add` (不是位置参数 ⇒ 不报「未知参数」)。
    let summary = run_user_admin_at(
        dir.path(),
        &args(&["add", "--user-id", "u-1", "--role", "owner"]),
    )
    .expect("动词分派到 add");
    assert!(summary.contains("u-1"), "{summary}");

    // 缺动词 ⇒ 用法帧 (动词本身也走拒绝即帧)。
    let error = run_user_admin_at(dir.path(), &args(&[])).expect_err("缺动词必须拒");
    assert!(error.contains("user requires a subcommand"), "{error}");
    assert!(error.contains("Usage:"), "带用法帧: {error}");

    // 未知动词 ⇒ 指名拒 (不是「未知参数」: 动词位不是旗标位)。
    let error = run_user_admin_at(dir.path(), &args(&["nope"])).expect_err("未知动词必须拒");
    assert!(error.contains("unknown user command: nope"), "{error}");
    assert!(error.contains("Usage:"), "带用法帧: {error}");
}

/// 动词级白名单: 各动词只认自己的旗标 (共享层按动词分 `FlagSpec` 表达)。
#[test]
fn each_verb_accepts_only_its_own_flags() {
    let dir = temp_dir("verb-scope");
    // `list` 不认 `--user-id` (那是 add / role / remove 的旗标)。
    let error = run_user_admin_at(dir.path(), &args(&["list", "--user-id", "u-1"]))
        .expect_err("动词级白名单必须拒");
    assert!(error.contains("unknown argument: --user-id"), "{error}");
    assert!(
        error.contains("(user list accepts --token)"),
        "白名单按动词分: {error}"
    );
    // `role` 不认 `--clients` (那是 add 的旗标)。
    let error = run_user_admin_at(dir.path(), &args(&["role", "--clients", "c"]))
        .expect_err("动词级白名单必须拒");
    assert!(error.contains("unknown argument: --clients"), "{error}");
    // `remove` 不认 `--role`。
    let error = run_user_admin_at(dir.path(), &args(&["remove", "--role", "owner"]))
        .expect_err("动词级白名单必须拒");
    assert!(error.contains("unknown argument: --role"), "{error}");
}

/// `add` 的三个可选旗标 (`--clients` / `--domain` / `--token`) 仍被收下。
#[test]
fn the_add_verb_still_accepts_its_optional_flags() {
    let dir = temp_dir("add-optional");
    write_tokens(dir.path(), &[("tui", OWNER_TOKEN, Some("owner-1"))]);
    run_user_admin_at(
        dir.path(),
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("bootstrap owner");
    let summary = run_user_admin_at(
        dir.path(),
        &args(&[
            "add",
            "--user-id",
            "u-2",
            "--role",
            "member",
            "--clients",
            "desk",
            "--domain",
            "home",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("add 带三个可选旗标");
    assert!(summary.contains("u-2"), "{summary}");
}

// ---------------------------------------------------------------------------
// ② 未知项 —— 拒绝即帧 (四个动词同口径)。
// ---------------------------------------------------------------------------

#[test]
fn unknown_flags_are_refused_and_named_in_every_verb() {
    let dir = temp_dir("unknown");
    for verb in ["add", "list", "role", "remove"] {
        let error =
            run_user_admin_at(dir.path(), &args(&[verb, "--nope"])).expect_err("未知旗标必须拒");
        assert_flag_frame(&error, "--nope");
        assert_stopped_at_flag_layer(&error);
    }
}

/// 未知项的帧带**白名单提示** (说清本动词只认哪些旗标)。
#[test]
fn unknown_flag_frame_lists_the_accepted_flags() {
    let dir = temp_dir("known-list");
    let error =
        run_user_admin_at(dir.path(), &args(&["add", "--nope"])).expect_err("未知旗标必须拒");
    assert!(
        error.contains("(user add accepts --user-id / --role / --clients / --domain / --token)"),
        "列出本动词只认的旗标: {error}"
    );
    // 白名单按动词走 (remove 只认 --user-id / --token)。
    let error =
        run_user_admin_at(dir.path(), &args(&["remove", "--nope"])).expect_err("未知旗标必须拒");
    assert!(
        error.contains("(user remove accepts --user-id / --token)"),
        "白名单按动词分: {error}"
    );
}

/// 本面**不收**布尔旗标 ⇒ `--json` 指名拒 (不被当取值吞掉, 也不被静默忽略)。
#[test]
fn boolean_flags_are_not_accepted_by_the_user_faces() {
    let dir = temp_dir("boolean");
    for verb in ["add", "list", "role", "remove"] {
        let error =
            run_user_admin_at(dir.path(), &args(&[verb, "--json"])).expect_err("布尔旗标必须拒");
        assert_flag_frame(&error, "--json");
        assert_stopped_at_flag_layer(&error);
    }
}

/// 元旗标 (`--help` / `-h`) 在本面不是位置参数 ⇒ 指名拒 (不静默变成取值)。
#[test]
fn meta_flags_are_refused_by_the_user_faces() {
    let dir = temp_dir("meta");
    for (verb, meta) in [("add", "--help"), ("list", "-h")] {
        let error = run_user_admin_at(dir.path(), &args(&[verb, meta])).expect_err("元旗标必须拒");
        assert_flag_frame(&error, meta);
        assert_stopped_at_flag_layer(&error);
    }
}

// ---------------------------------------------------------------------------
// ③ 缺值 —— 值缺失 / 值本身像旗标 一律「requires a value」。
// ---------------------------------------------------------------------------

#[test]
fn missing_value_is_reported_as_missing_value() {
    let dir = temp_dir("missing");
    for (verb, flag) in [
        ("add", "--user-id"),
        ("list", "--token"),
        ("role", "--role"),
        ("remove", "--user-id"),
    ] {
        let error = run_user_admin_at(dir.path(), &args(&[verb, flag])).expect_err("缺值必须拒");
        assert!(error.contains("requires a value"), "说缺值: {error}");
        assert!(error.contains(flag), "点名旗标: {error}");
        assert!(error.contains("Usage:"), "带用法帧: {error}");
        assert!(
            !error.contains("unknown argument"),
            "缺值不是未知参数: {error}"
        );
        assert_stopped_at_flag_layer(&error);
    }
}

/// ⚠️ **值像旗标** ⇒ 同样是「缺值」, 且点名撞上的那一项
/// (旧行为把 `--role` 当**用户 id 吞下** —— 那是本批最该修的一条隐患)。
#[test]
fn flag_shaped_value_is_reported_as_missing_value() {
    let dir = temp_dir("flag-shaped");
    let error = run_user_admin_at(dir.path(), &args(&["add", "--user-id", "--role", "owner"]))
        .expect_err("值像旗标必须报缺值");
    assert!(error.contains("requires a value"), "说缺值: {error}");
    assert!(error.contains("--user-id"), "点名缺值的旗标: {error}");
    assert!(
        error.contains("--role"),
        "点名撞上的那一项 (不得被静默吞成用户 id): {error}"
    );
    assert!(
        !error.contains("unknown argument"),
        "缺值不是未知参数: {error}"
    );
    assert_stopped_at_flag_layer(&error);
}

/// ⚠️ **短旗标** (`-x`) 也是旗标形态 ⇒ 不得被当取值吞下。
#[test]
fn short_flags_are_not_swallowed_as_values() {
    let dir = temp_dir("short-flag");
    // 值位上的短旗标 ⇒ 缺值 (旧行为把 `-x` 当用户 id 吞下)。
    let error = run_user_admin_at(dir.path(), &args(&["add", "--user-id", "-x"]))
        .expect_err("短旗标不得被当取值");
    assert!(error.contains("requires a value"), "说缺值: {error}");
    assert!(error.contains("-x"), "点名撞上的那一项: {error}");
    assert_stopped_at_flag_layer(&error);
    // 旗标位上的短旗标 ⇒ 未知项 (指名拒)。
    let error = run_user_admin_at(dir.path(), &args(&["add", "-x"])).expect_err("短旗标必须拒");
    assert!(error.contains("unknown argument: -x"), "{error}");
    assert_stopped_at_flag_layer(&error);
}

// ---------------------------------------------------------------------------
// ④ 重复即拒 —— 本面自有纪律 (共享层 `Scan::value` 是末位生效 ⇒ 更宽松)。
// ---------------------------------------------------------------------------

#[test]
fn repeated_flags_are_refused() {
    let dir = temp_dir("repeat");
    for (verb, flag) in [
        ("add", "--user-id"),
        ("list", "--token"),
        ("role", "--role"),
        ("remove", "--user-id"),
    ] {
        let error = run_user_admin_at(dir.path(), &args(&[verb, flag, "a", flag, "b"]))
            .expect_err("重复旗标必须拒");
        assert!(error.contains("was given twice"), "说重复: {error}");
        assert!(error.contains(flag), "点名重复的旗标: {error}");
        assert!(error.contains("Usage:"), "带用法帧: {error}");
        assert_stopped_at_flag_layer(&error);
    }
}

// ---------------------------------------------------------------------------
// ⑤ `--` 之后 —— 一律位置参数 (不再做旗标解析)。
// ---------------------------------------------------------------------------

#[test]
fn double_dash_ends_flag_parsing_for_the_user_faces() {
    let dir = temp_dir("double-dash");
    // `--` 之后的 `--nope` 是位置参数 (本面不收位置参数 ⇒ 指名拒为未知参数),
    // 不是未知旗标 ⇒ 不报缺值。
    let error =
        run_user_admin_at(dir.path(), &args(&["add", "--", "--nope"])).expect_err("位置参数必须拒");
    assert!(
        error.contains("unknown argument: --nope"),
        "`--` 之后的词当位置参数 (指名拒): {error}"
    );
    assert!(error.contains("Usage:"), "带用法帧: {error}");
    assert!(
        !error.contains("requires a value"),
        "`--` 之后不做旗标解析 (不报缺值): {error}"
    );
    // `--` 本身不再是「未知参数」(它是分隔符) —— 被拒的是它后面的词。
    assert!(
        !error.contains("unknown argument: --\n"),
        "`--` 是分隔符不是未知项: {error}"
    );
}

// ---------------------------------------------------------------------------
// ⑥ 负数是值 —— 不是旗标 (不被当旗标拒, 也不报缺值)。
// ---------------------------------------------------------------------------

#[test]
fn negative_numbers_are_values_not_flags() {
    let dir = temp_dir("negative");
    // `--user-id -1`: 负数是值 ⇒ 旗标层放行 (后面报的是用户表本体的词面校验,
    // 不是缺值 / 不是未知项)。
    let error = run_user_admin_at(
        dir.path(),
        &args(&["add", "--user-id", "-1", "--role", "owner"]),
    )
    .expect_err("负数当取值放行");
    assert!(
        !error.contains("requires a value"),
        "负数是值 (不报缺值): {error}"
    );
    assert!(!error.contains("unknown argument"), "负数不是旗标: {error}");
    assert!(
        error.contains("must start with a lowercase letter or digit"),
        "负数被当取值送进用户表本体: {error}"
    );
}

// ---------------------------------------------------------------------------
// ⑦ 不收位置参数 —— 本面自有纪律 (词类参数指名拒)。
// ---------------------------------------------------------------------------

#[test]
fn positional_arguments_are_refused_by_the_user_faces() {
    let dir = temp_dir("positional");
    for verb in ["add", "list", "role", "remove"] {
        let error =
            run_user_admin_at(dir.path(), &args(&[verb, "extra"])).expect_err("位置参数必须拒");
        assert!(
            error.contains("unknown argument: extra"),
            "词类参数指名拒: {error}"
        );
        assert!(error.contains("Usage:"), "带用法帧: {error}");
        assert_stopped_at_flag_layer(&error);
    }
}

// ---------------------------------------------------------------------------
// ⑧ 空值 —— 本面旧行为**无**此判据 ⇒ 本批不新增 (共享层不提供, 面内也不补)。
// ---------------------------------------------------------------------------

#[test]
fn empty_values_keep_the_old_behavior() {
    let dir = temp_dir("empty");
    // `--clients ""` = 空的客户端集 (合法取值) ⇒ 旗标层放行, 用户表本体也收下。
    let summary = run_user_admin_at(
        dir.path(),
        &args(&[
            "add",
            "--user-id",
            "u-1",
            "--role",
            "owner",
            "--clients",
            "",
        ]),
    )
    .expect("空客户端集不被旗标层拒");
    assert!(summary.contains("added user u-1"), "{summary}");
}

// ---------------------------------------------------------------------------
// ⑨ 语言档 —— 错误帧与本面用法帧同语言 (英文)。
// ---------------------------------------------------------------------------

#[test]
fn error_frames_follow_the_english_style_of_this_face() {
    let dir = temp_dir("language");
    let error =
        run_user_admin_at(dir.path(), &args(&["add", "--nope"])).expect_err("未知旗标必须拒");
    assert!(error.contains("unknown argument"), "英文档: {error}");
    assert!(!error.contains("未知旗标"), "本面用法帧是英文的: {error}");
    assert!(!error.contains("未知参数"), "本面用法帧是英文的: {error}");
    // 缺值帧同样英文。
    let error =
        run_user_admin_at(dir.path(), &args(&["add", "--user-id"])).expect_err("缺值必须拒");
    assert!(error.contains("requires a value"), "英文档: {error}");
    assert!(!error.contains("缺值"), "英文档: {error}");
}

// ---------------------------------------------------------------------------
// ⑩ 动词分派仍工作 —— 收口后四个动词各自的动作链照旧 (旗标层不误伤)。
// ---------------------------------------------------------------------------

#[test]
fn the_verb_dispatch_still_reaches_every_action() {
    let dir = temp_dir("dispatch");
    write_tokens(dir.path(), &[("tui", OWNER_TOKEN, Some("owner-1"))]);

    // add: 引导入表 (首用户必须是 owner; 客户端绑定与令牌表一致)。
    let summary = run_user_admin_at(
        dir.path(),
        &args(&[
            "add",
            "--user-id",
            "owner-1",
            "--role",
            "owner",
            "--clients",
            "tui",
        ]),
    )
    .expect("add");
    assert!(summary.contains("added user owner-1"), "{summary}");

    // add: 再加一个 member (带 owner 令牌; 该客户端没有令牌条目 ⇒ 不牵绑)。
    let summary = run_user_admin_at(
        dir.path(),
        &args(&[
            "add",
            "--user-id",
            "member-1",
            "--role",
            "member",
            "--clients",
            "desk",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("add member");
    assert!(summary.contains("added user member-1"), "{summary}");

    // list: owner 令牌可读全表。
    let listing =
        run_user_admin_at(dir.path(), &args(&["list", "--token", OWNER_TOKEN])).expect("list");
    assert!(listing.contains("owner-1\towner"), "{listing}");
    assert!(listing.contains("member-1\tmember"), "{listing}");

    // role: 改角色。
    let summary = run_user_admin_at(
        dir.path(),
        &args(&[
            "role",
            "--user-id",
            "member-1",
            "--role",
            "guest",
            "--token",
            OWNER_TOKEN,
        ]),
    )
    .expect("role");
    assert!(summary.contains("member-1 is now guest"), "{summary}");

    // remove: 删户。
    let summary = run_user_admin_at(
        dir.path(),
        &args(&["remove", "--user-id", "member-1", "--token", OWNER_TOKEN]),
    )
    .expect("remove");
    assert!(summary.contains("removed user member-1"), "{summary}");
}

// ---------------------------------------------------------------------------
// ⑪ 真实二进制 —— CLI 可观测面 (退出码 + 错误帧)。
// ---------------------------------------------------------------------------

#[test]
fn real_binary_refuses_an_unknown_user_flag_with_an_error_frame() {
    let home = tempfile::tempdir().expect("temp dir");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_apeireth"))
        .args(["user", "add", "--nope"])
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

/// 真实二进制: 值像旗标 ⇒ 缺值 (不吞下 `--role` 当用户 id)。
#[test]
fn real_binary_reports_a_flag_shaped_value_as_a_missing_value() {
    let home = tempfile::tempdir().expect("temp dir");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_apeireth"))
        .args(["user", "add", "--user-id", "--role", "owner"])
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .output()
        .expect("failed to execute cli binary");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "缺值 = 非零退出: {stderr}");
    assert!(
        stderr.contains("requires a value"),
        "值像旗标 = 缺值: {stderr}"
    );
    assert!(
        !stderr.contains("unknown argument"),
        "缺值不是未知参数: {stderr}"
    );
}
