//! crate 级旗标判据的**逐条机械断言**（`apeireth_cli::flags` 契约面）。
//!
//! 本文件守的是**共享判据本身**（不是某一面的措辞）：已收口的旗标扫描面
//! (`main.rs` 入口面 / `exposure.rs` / `lessons_command.rs` / `egress_command.rs` /
//! `trait_command.rs` / `backup.rs` / `crypto_keys.rs` / `user_admin.rs`) 都从这里取判据，判据漂移 ⇒ 本文件红。**收口面增减时同步本清单。** / `mcp_command.rs`
//!
//! 逐条对应共享模块声明的五条口径：
//! 1. **旗标形态** —— 负数**不是**旗标（是值）；
//! 2. **未知旗标** —— 拒绝即帧，错误帧点名旗标 + `--` 分隔提示；
//! 3. **带值旗标取值** —— 值缺失 / 值本身像旗标 ⇒ 报「缺值」（不是「未知参数」）；
//! 4. **布尔旗标** —— 不带值、**不吞**后面的词；
//! 5. **`--` 之后** —— 一律位置参数（不再做旗标解析）。
//!
//! 措辞档另测：中文档（仓内多数派）与英文档（用法文本是英文的面）各出一帧，
//! 两档都要点名被拒的旗标，并给出 `--` 分隔提示。

use apeireth_cli::flags::{self, FlagError, FlagSpec, FrameContext, Scan, Style, UnknownNoun};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

/// 带值旗标 + 布尔旗标各一的样例白名单。
fn spec() -> FlagSpec {
    FlagSpec {
        value: &["--path", "--limit"],
        boolean: &["--json"],
        meta_as_positional: false,
    }
}

// ---------------------------------------------------------------------------
// ① 旗标形态：负数是**值**，不是旗标。
// ---------------------------------------------------------------------------

#[test]
fn negative_numbers_are_values_not_flags() {
    // 负数不是旗标形态 —— `--cooldown -1` 里的 `-1` 是值。
    assert!(!flags::is_flag_like("-1"), "-1 是值");
    assert!(!flags::is_flag_like("-1.5"), "-1.5 是值");
    assert!(!flags::is_flag_like("-1e9"), "-1e9 是值");
    assert!(!flags::is_flag_like("-0"), "-0 是值");
    // 负数形态判定本身。
    assert!(flags::is_negative_number("-1"));
    assert!(flags::is_negative_number("-1.5"));
    assert!(flags::is_negative_number("-1e9"));
    assert!(!flags::is_negative_number("--1"), "--1 是旗标形态");
    assert!(!flags::is_negative_number("-x"), "-x 不是负数");
    assert!(!flags::is_negative_number("-"), "单个 '-' 不是负数");
    assert!(!flags::is_negative_number("1"), "无符号不是负数");
}

#[test]
fn flag_shapes_are_flags() {
    assert!(flags::is_flag_like("--path"), "长旗标");
    assert!(flags::is_flag_like("-x"), "短旗标");
    assert!(flags::is_flag_like("--"), "`--` 是分隔符（旗标形态）");
    assert!(!flags::is_flag_like("-"), "单个 '-' 不是旗标形态");
    assert!(!flags::is_flag_like("path"), "词类不是旗标");
    assert!(!flags::is_flag_like(""), "空串不是旗标");
}

/// 严格档「以 `-` 开头」：负数与单个 `-` 也算（落档类闸用）。
#[test]
fn strict_dash_prefix_covers_negatives_and_the_bare_dash() {
    assert!(flags::starts_with_dash("-1"), "负数也以 '-' 开头");
    assert!(flags::starts_with_dash("-"), "单个 '-'");
    assert!(flags::starts_with_dash("--json"));
    assert!(!flags::starts_with_dash("json"));
    assert!(!flags::starts_with_dash(""));
}

/// 元旗标（只在首参数位生效；出现在正文之后 = 正文）。
#[test]
fn meta_flags_are_recognized() {
    for token in ["--help", "-h", "--version", "-V"] {
        assert!(flags::is_meta_flag(token), "{token} 是元旗标");
    }
    for token in ["--nope", "-x", "--", "help"] {
        assert!(!flags::is_meta_flag(token), "{token} 不是元旗标");
    }
}

// ---------------------------------------------------------------------------
// ② + ③ 取值：缺失 / 值像旗标 ⇒ 报「缺值」（不是「未知参数」）。
// ---------------------------------------------------------------------------

#[test]
fn missing_value_is_reported_as_missing_value() {
    let list = args(&["--path"]);
    let mut index = 0;
    let error = flags::take_flag_value(&list, &mut index, "--path").expect_err("悬空旗标必须拒");
    match error {
        FlagError::MissingValue { flag, next } => {
            assert_eq!(flag, "--path", "点名旗标");
            assert!(next.is_none(), "没有下一项 ⇒ 值缺失");
        }
        other => panic!("必须报缺值, 实际: {other:?}"),
    }
}

#[test]
fn flag_shaped_value_is_reported_as_missing_value() {
    let list = args(&["--path", "--json"]);
    let mut index = 0;
    let error = flags::take_flag_value(&list, &mut index, "--path").expect_err("值像旗标必须拒");
    match error {
        FlagError::MissingValue { flag, next } => {
            assert_eq!(flag, "--path", "点名旗标");
            assert_eq!(next.as_deref(), Some("--json"), "点名撞上的那一项");
        }
        other => panic!("必须报缺值, 实际: {other:?}"),
    }
}

#[test]
fn negative_value_is_taken_as_a_value() {
    let list = args(&["--cooldown", "-1"]);
    let mut index = 0;
    let value = flags::take_flag_value(&list, &mut index, "--cooldown").expect("负数是值");
    assert_eq!(value, "-1", "负数原样取值（报「值不合法」而不是「缺值」）");
}

// ---------------------------------------------------------------------------
// ④ 布尔旗标不吞词；带值旗标吃一个词。
// ---------------------------------------------------------------------------

#[test]
fn boolean_flag_does_not_eat_the_next_word() {
    let scanned: Scan =
        flags::scan(&args(&["--json", "act-1", "confirmed"]), &spec()).expect("扫描应成功");
    assert!(scanned.has("--json"), "--json 命中");
    assert_eq!(
        scanned.positionals,
        vec!["act-1".to_string(), "confirmed".to_string()],
        "布尔旗标不吞后面的词"
    );
    assert!(scanned.value("--path").is_none(), "布尔旗标不带值");
}

#[test]
fn value_flag_eats_exactly_one_word() {
    let scanned: Scan =
        flags::scan(&args(&["--path", "/tmp/a.jsonl", "extra"]), &spec()).expect("扫描应成功");
    assert_eq!(
        scanned.value("--path"),
        Some("/tmp/a.jsonl"),
        "带值旗标吃一个词"
    );
    assert_eq!(
        scanned.positionals,
        vec!["extra".to_string()],
        "值之后的词仍是位置参数"
    );
}

/// 同名带值旗标出现多次 ⇒ 末位生效（调用方可用 `count` 自己判重复）。
#[test]
fn repeated_value_flag_takes_the_last_occurrence() {
    let scanned: Scan =
        flags::scan(&args(&["--path", "/tmp/a", "--path", "/tmp/b"]), &spec()).expect("扫描应成功");
    assert_eq!(scanned.value("--path"), Some("/tmp/b"), "末位生效");
    assert_eq!(scanned.count("--path"), 2, "出现次数可数（重复即拒的面用）");
}

// ---------------------------------------------------------------------------
// ⑤ `--` 之后一律位置参数。
// ---------------------------------------------------------------------------

#[test]
fn double_dash_ends_flag_parsing() {
    let scanned: Scan =
        flags::scan(&args(&["--path", "/tmp/a", "--", "--json"]), &spec()).expect("扫描应成功");
    assert_eq!(
        scanned.value("--path"),
        Some("/tmp/a"),
        "`--` 之前的旗标照旧"
    );
    assert!(!scanned.has("--json"), "`--` 之后不再是旗标");
    assert_eq!(
        scanned.positionals,
        vec!["--json".to_string()],
        "`--` 之后的词一律位置参数"
    );
}

/// `--` 之后的词不再触发未知旗标拒绝。
#[test]
fn double_dash_makes_unknown_flags_positional() {
    let scanned: Scan = flags::scan(&args(&["--", "--nope"]), &spec()).expect("扫描应成功");
    assert_eq!(
        scanned.positionals,
        vec!["--nope".to_string()],
        "`--` 之后未知旗标也是位置参数"
    );
}

// ---------------------------------------------------------------------------
// ② 未知旗标：拒绝即帧（分类 + 措辞）。
// ---------------------------------------------------------------------------

#[test]
fn unknown_flag_is_refused_and_named() {
    let error = flags::scan(&args(&["--typo", "x"]), &spec()).expect_err("未知旗标必须拒");
    match error {
        FlagError::UnknownFlag { flag } => assert_eq!(flag, "--typo", "点名旗标"),
        other => panic!("必须是未知旗标, 实际: {other:?}"),
    }
}

/// 元旗标当正文（`chat how --help` 的正文不被吞）vs 当未知旗标。
#[test]
fn meta_flag_handling_follows_the_spec() {
    let body_spec = FlagSpec {
        value: &[],
        boolean: &[],
        meta_as_positional: true,
    };
    let scanned = flags::scan(&args(&["how", "--help"]), &body_spec).expect("元旗标当正文");
    assert_eq!(
        scanned.positionals,
        vec!["how".to_string(), "--help".to_string()],
        "元旗标落在正文里（顺序不变）"
    );
    let error = flags::scan(&args(&["--help"]), &spec()).expect_err("元旗标当未知旗标时必须拒");
    assert!(matches!(error, FlagError::UnknownFlag { .. }), "{error:?}");
}

/// 面不收位置参数时: 第一个多余的位置参数 ⇒ 未知参数（不是未知旗标）。
#[test]
fn extra_positional_is_an_unknown_argument() {
    let scanned = flags::scan(&args(&["extra"]), &spec()).expect("扫描应成功");
    match scanned.extra_positional().expect("多余位置参数") {
        FlagError::UnknownArgument { token } => assert_eq!(token, "extra", "点名多余的那一个"),
        other => panic!("必须是未知参数, 实际: {other:?}"),
    }
    assert!(
        flags::scan(&args(&[]), &spec())
            .expect("扫描应成功")
            .extra_positional()
            .is_none(),
        "没有位置参数时不报错"
    );
}

// ---------------------------------------------------------------------------
// 措辞档：中文档（仓内多数派）/ 英文档（用法文本是英文的面）。
// ---------------------------------------------------------------------------

#[test]
fn zh_frame_names_the_flag_and_hints_the_separator() {
    let error = FlagError::UnknownFlag {
        flag: "--nope".to_string(),
    };
    let frame = error.frame(
        Style::Zh,
        &FrameContext::new("lessons", UnknownNoun::Flag).with_usage("用法帧"),
    );
    assert!(frame.contains("--nope"), "点名旗标: {frame}");
    assert!(frame.contains("`--`"), "给出 `--` 分隔提示: {frame}");
    assert!(frame.contains("用法帧"), "带用法帧: {frame}");
}

#[test]
fn en_frame_names_the_flag_and_hints_the_separator() {
    let error = FlagError::UnknownFlag {
        flag: "--nope".to_string(),
    };
    let frame = error.frame(Style::En, &FrameContext::new("chat", UnknownNoun::Flag));
    assert!(frame.contains("--nope"), "点名旗标: {frame}");
    assert!(frame.contains("`--`"), "给出 `--` 分隔提示: {frame}");
    assert!(
        frame.contains("unknown flag"),
        "英文档说 unknown flag: {frame}"
    );
}

/// 未知项的措辞逐面选（既有测试钉死的措辞照旧）。
#[test]
fn unknown_noun_selects_the_wording() {
    let error = FlagError::UnknownFlag {
        flag: "--nope".to_string(),
    };
    let flag_noun = error.frame(
        Style::Zh,
        &FrameContext::new("egress ledger", UnknownNoun::Flag),
    );
    assert!(flag_noun.contains("未知旗标"), "{flag_noun}");
    let argument_noun = error.frame(
        Style::Zh,
        &FrameContext::new("lessons", UnknownNoun::Argument),
    );
    assert!(argument_noun.contains("未知参数"), "{argument_noun}");
}

/// 缺值帧: 说「缺值 / requires a value」，**不说**「未知参数」。
#[test]
fn missing_value_frame_says_missing_value_not_unknown_argument() {
    let error = FlagError::MissingValue {
        flag: "--path".to_string(),
        next: None,
    };
    let zh = error.frame(Style::Zh, &FrameContext::new("lessons", UnknownNoun::Flag));
    assert!(zh.contains("缺值"), "中档案说缺值: {zh}");
    assert!(zh.contains("--path"), "点名旗标: {zh}");
    assert!(!zh.contains("未知参数"), "缺值不是未知参数: {zh}");
    let en = error.frame(Style::En, &FrameContext::new("chat", UnknownNoun::Flag));
    assert!(
        en.contains("requires a value"),
        "英文档说 requires a value: {en}"
    );
    assert!(en.contains("--path"), "点名旗标: {en}");
}

/// 值撞上旗标时: 缺值帧要点名撞上的那一项。
#[test]
fn missing_value_frame_names_the_offending_next_token() {
    let error = FlagError::MissingValue {
        flag: "--path".to_string(),
        next: Some("--json".to_string()),
    };
    let zh = error.frame(
        Style::Zh,
        &FrameContext::new("egress ledger", UnknownNoun::Flag),
    );
    assert!(zh.contains("缺值"), "{zh}");
    assert!(zh.contains("--json"), "点名撞上的那一项: {zh}");
    assert!(!zh.contains("未知旗标"), "缺值不是未知旗标: {zh}");
    let en = error.frame(
        Style::En,
        &FrameContext::new("exposure report", UnknownNoun::Flag),
    );
    assert!(en.contains("--json"), "点名撞上的那一项: {en}");
    assert!(en.contains("'-'"), "说清为什么拒: {en}");
}

/// 白名单提示（只认哪些旗标）进错误帧。
#[test]
fn known_flag_list_is_part_of_the_frame() {
    let known = spec().known();
    assert!(known.contains("--path"), "{known}");
    assert!(known.contains("--limit"), "{known}");
    assert!(known.contains("--json"), "{known}");
    let frame = FlagError::UnknownFlag {
        flag: "--nope".to_string(),
    }
    .frame(
        Style::Zh,
        &FrameContext::new("egress ledger", UnknownNoun::Flag).with_known(&known),
    );
    assert!(
        frame.contains("--path"),
        "错误帧列出本命令只认的旗标: {frame}"
    );
}
