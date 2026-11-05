//! CLI 参数层旗标口径回归 (main.rs 单点修复批)。
//!
//! 覆盖五处缺陷：
//! - D1: `chat how --help` —— 首参数位之外的 `--help` 不再劫持命令 (正文不被吞)；
//! - D1b: `chat -- -h` —— `--` 之后的词一律当正文 (不解析成旗标)；
//! - D8: `chat --modle ...` —— 未知旗标拒绝即帧 (非零退出 + 指出旗标名 + `--` 提示)；
//! - D12: `council --foo bar` —— 议题位同样拒绝未知旗标；
//! - D6-附: 帮助文本列全顶层动词 (补齐 dream / council / subagent / nightwatch /
//!   lessons / egress 六个子命令)。
//!
//! 口径真源在 crate 级共享模块 `apeireth_cli::flags`（`src/main.rs` 只留本面措辞
//! 档）；解析层的逐字语义由该文件的 `#[cfg(test)]` 单测守住，本文件守 CLI 可观测面 (退出码 / 错误帧 / 帮助文本)。

use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_apeireth")
}

/// 隔离环境跑一次 CLI：无凭据 ⇒ 聊天路径必然在取凭据处失败 (不触网)，
/// 于是「进了聊天层」与「旗标层就拒了」两种结果可稳定区分。
fn run_isolated(home: &std::path::Path, args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("APEIRETH_DATA_DIR", home.join("data"))
        .env("APEIRETH_COGNITIVE_DB", home.join("data/cognitive.sqlite3"))
        .env_remove("APEIRETH_API_KEY")
        .output()
        .expect("failed to execute cli binary")
}

#[test]
fn chat_help_flag_after_the_prompt_does_not_swallow_the_prompt() {
    // D1: `--help` 不是子命令首参数 ⇒ 不打全局帮助, 正文照发模型。
    let home = tempfile::tempdir().expect("temp dir");
    let output = run_isolated(home.path(), &["chat", "how", "--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("Usage:"),
        "首参数位之外的 --help 不得打印帮助: {stdout}"
    );
    assert!(
        stderr.contains("canonical chat failed"),
        "正文应进聊天层 (无凭据故失败), 而不是被旗标解析吞掉: {stderr}"
    );
    assert!(
        !stderr.contains("unknown flag"),
        "错位的 --help 不是未知旗标: {stderr}"
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn chat_double_dash_turns_dash_words_into_positional_text() {
    // D1b: `--` 之后的词一律当正文 (不解析成旗标 ⇒ 不打帮助)。
    let home = tempfile::tempdir().expect("temp dir");
    let output = run_isolated(home.path(), &["chat", "--", "-h"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("Usage:"),
        "`--` 之后的词不得解析成帮助旗标: {stdout}"
    );
    assert!(
        stderr.contains("canonical chat failed"),
        "`-h` 应作为正文进聊天层: {stderr}"
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn chat_unknown_flag_is_refused_with_an_error_frame() {
    // D8: 拼错的旗标拒绝即帧 (非零退出 + 指出旗标名 + `--` 分隔提示)。
    let home = tempfile::tempdir().expect("temp dir");
    let output = run_isolated(home.path(), &["chat", "--modle", "gpt-4", "hello"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("--modle"), "错误帧须指出旗标名: {stderr}");
    assert!(
        stderr.contains("`--`"),
        "错误帧须提示 -- 分隔用法: {stderr}"
    );
    assert!(
        !stderr.contains("canonical chat failed"),
        "被拒的旗标绝不进模型: {stderr}"
    );
}

#[test]
fn council_unknown_flag_is_refused_with_an_error_frame() {
    // D12: 议题位同样拒绝未知旗标 (不静默吞成议题)。
    let home = tempfile::tempdir().expect("temp dir");
    let output = run_isolated(home.path(), &["council", "--foo", "bar"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("--foo"), "错误帧须指出旗标名: {stderr}");
    assert!(
        stderr.contains("`--`"),
        "错误帧须提示 -- 分隔用法: {stderr}"
    );
}

#[test]
fn help_text_lists_every_top_level_verb() {
    // D6-附: 帮助文本须列全 18 个顶层动词。
    let output = Command::new(bin())
        .arg("--help")
        .output()
        .expect("failed to execute cli binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for verb in [
        "apeireth session",
        "apeireth chat",
        "apeireth onboard",
        "apeireth approve",
        "apeireth reject",
        "apeireth cancel",
        "apeireth dream",
        "apeireth council",
        "apeireth subagent",
        "apeireth nightwatch",
        "apeireth lessons",
        "apeireth egress",
        "apeireth gateway serve",
        "apeireth user",
        "apeireth backup",
        "apeireth crypto",
        "apeireth exposure",
        "apeireth trait",
    ] {
        assert!(stdout.contains(verb), "帮助文本须列出 {verb}");
    }
}

#[test]
fn verb_level_help_and_version_still_work() {
    // 回归护栏: 动词首参数位的 `--help` / `--version` 仍生效 (退出 0)。
    let help = Command::new(bin())
        .args(["chat", "--help"])
        .output()
        .expect("failed to execute cli binary");
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage:"));

    let version = Command::new(bin())
        .args(["chat", "--version"])
        .output()
        .expect("failed to execute cli binary");
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("apeireth "));
}
