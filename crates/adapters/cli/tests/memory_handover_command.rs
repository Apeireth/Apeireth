//! `apeireth memory handover` 的测试 —— **只读打包** + **当场跑演练**。
//!
//! **核心纪律**：
//! * **只读**（打包不动库）；
//! * **演练通过才报成功**（不产出「打出来但读不懂」的交接包）；
//! * **已知问题**如实带进包里（**不隐瞒坏消息**）；
//! * 参数错 / 库不存在 ⇒ **退出码分开**。

use apeireth_cli::memory_handover_command::{
    handover_package_from_conn, parse_options, render, run_handover, HandoverFailure,
};
use apeireth_memory::handover::NextStep;
use std::path::PathBuf;
use tempfile::tempdir;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn memory_conn() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().expect("内存库");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS episodes (
            id TEXT PRIMARY KEY, timestamp INTEGER NOT NULL, role TEXT NOT NULL,
            content TEXT NOT NULL, session_id TEXT, chain_hash TEXT,
            chain_prev_hash TEXT, continuity_id TEXT
        );
        CREATE TABLE IF NOT EXISTS chain_heads (
            subject_id TEXT PRIMARY KEY, entry_count INTEGER NOT NULL, head_hash TEXT
        );",
    )
    .expect("建表");
    conn.execute(
        "INSERT INTO episodes (id, timestamp, role, content, session_id, continuity_id)
         VALUES ('e1', 1000, 'user', '第一条记忆', 's1', 'sub-1')",
        [],
    )
    .expect("插入");
    conn.execute(
        "INSERT INTO episodes (id, timestamp, role, content, session_id, continuity_id)
         VALUES ('e2', 2000, 'user', '第二条记忆', 's1', 'sub-1')",
        [],
    )
    .expect("插入");
    conn
}

#[test]
fn parses_options_including_issues_and_next_steps() {
    let options = parse_options(&args(&[
        "--db",
        "/tmp/x.sqlite3",
        "--out",
        "/tmp/p.json",
        "--issue",
        "坏消息也要写",
        "--next",
        "做什么 | 为什么 | 怎么验收 | 2",
    ]))
    .expect("应能解析");
    assert_eq!(options.db, Some(PathBuf::from("/tmp/x.sqlite3")));
    assert_eq!(options.out, PathBuf::from("/tmp/p.json"));
    assert_eq!(options.issues, vec!["坏消息也要写".to_string()]);
    assert_eq!(options.next_steps.len(), 1);
    assert_eq!(options.next_steps[0].cost, 2);
    assert_eq!(options.next_steps[0].what, "做什么");
}

#[test]
fn rejects_missing_out() {
    assert!(parse_options(&args(&["--db", "/tmp/x.sqlite3"])).is_err());
}

#[test]
fn rejects_a_malformed_next_step() {
    // ⇒ `--next` 的格式是**四段**（做什么 | 为什么 | 怎么验收 | 工作量）。
    assert!(parse_options(&args(&["--out", "/tmp/p.json", "--next", "只有一段"])).is_err());
    assert!(parse_options(&args(&["--out", "/tmp/p.json", "--next", "a | b | c | 9"])).is_err());
    assert!(parse_options(&args(&["--out", "/tmp/p.json", "--next", "a | b | c | x"])).is_err());
}

#[test]
fn a_missing_database_is_unavailable() {
    let dir = tempdir().expect("临时目录");
    let error = run_handover(&args(&[
        "--db",
        dir.path().join("missing.sqlite3").to_str().expect("路径"),
        "--out",
        dir.path().join("p.json").to_str().expect("路径"),
    ]))
    .expect_err("库不存在应失败");
    assert!(
        matches!(error, HandoverFailure::Unavailable(_)),
        "实际 {error:?}"
    );
}

#[test]
fn usage_error_is_its_own_kind() {
    assert_eq!(
        run_handover(&args(&["--db"])).expect_err("缺参数应失败"),
        HandoverFailure::Usage
    );
}

#[test]
fn builds_a_package_and_passes_the_drill() {
    // ⇒ 🎯 **打包 + 演练通过**。
    let dir = tempdir().expect("临时目录");
    let conn = memory_conn();
    let out = dir.path().join("package.json");
    let report = handover_package_from_conn(
        &conn,
        &out,
        &PathBuf::from("/tmp/fake.sqlite3"),
        vec!["recall@10 = 0.2983（坏消息也要写）".to_string()],
        vec![NextStep {
            what: "做什么".to_string(),
            why: "为什么".to_string(),
            acceptance: "怎么验收".to_string(),
            cost: 1,
        }],
    )
    .expect("打包应成功");
    assert_eq!(report.entries, 2);
    assert!(
        report.drill_passed,
        "演练应通过，问题清单：{:?}",
        report.drill_problems
    );
    assert!(out.exists(), "交接包文件应存在");

    let text = std::fs::read_to_string(&out).expect("读包");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("应是合法 JSON");
    for key in [
        "version",
        "data",
        "state",
        "next_steps",
        "red_lines",
        "archive",
    ] {
        assert!(parsed.get(key).is_some(), "包里应有 {key}：{parsed}");
    }
}

#[test]
fn known_issues_are_carried_into_the_package() {
    // ⇒ ⚠️ **不隐瞒坏消息** ⇒ ⇒ 已知问题必须进包。
    let dir = tempdir().expect("临时目录");
    let conn = memory_conn();
    let out = dir.path().join("package.json");
    handover_package_from_conn(
        &conn,
        &out,
        &PathBuf::from("/tmp/fake.sqlite3"),
        vec!["recall@10 = 0.2983（坏消息也要写）".to_string()],
        Vec::new(),
    )
    .expect("打包应成功");
    let text = std::fs::read_to_string(&out).expect("读包");
    assert!(
        text.contains("坏消息也要写"),
        "包里的「已知问题」应包含命令行给的原文"
    );
}

#[test]
fn the_report_states_the_numbers_and_the_drill_outcome() {
    let dir = tempdir().expect("临时目录");
    let conn = memory_conn();
    let out = dir.path().join("package.json");
    let report = handover_package_from_conn(
        &conn,
        &out,
        &PathBuf::from("/tmp/fake.sqlite3"),
        vec!["坏消息也要写".to_string()],
        vec![NextStep {
            what: "做什么".to_string(),
            why: "为什么".to_string(),
            acceptance: "怎么验收".to_string(),
            cost: 1,
        }],
    )
    .expect("打包应成功");
    let text = render(&report);
    for needle in [
        "交接包生成完成",
        "记录条数",
        "未纳管条数",
        "红线",
        "演练通过",
    ] {
        assert!(text.contains(needle), "报告里应含 {needle}：\n{text}");
    }
}

#[test]
fn a_package_without_next_steps_fails_the_drill() {
    // ⇒ ⚠️ **演练在工作**：没有下一步 ⇒ ⇒ 「接手者不知道该干什么」
    // ⇒ ⇒ **演练失败** ⇒ ⇒ 退出码 **1** ⇒ ⇒ **不产出「打出来但读不懂」的交接包**。
    let dir = tempdir().expect("临时目录");
    let conn = memory_conn();
    let out = dir.path().join("package.json");
    let report = handover_package_from_conn(
        &conn,
        &out,
        &PathBuf::from("/tmp/fake.sqlite3"),
        Vec::new(),
        Vec::new(),
    )
    .expect("打包本身应成功");
    assert!(
        !report.drill_passed,
        "没有下一步 ⇒ 演练**应失败**（问题清单：{:?}）",
        report.drill_problems
    );
    assert!(
        report
            .drill_problems
            .iter()
            .any(|problem| problem.contains("下一步")),
        "问题清单里应指出「下一步」缺失（实际 {:?}）",
        report.drill_problems
    );
    let text = render(&report);
    assert!(text.contains("演练失败"), "报告里应写明失败：\n{text}");
}

// ═══════════════════════════════════════════════════════════════════════════
// 能力接线审计（2026-11-07）：交接包报告必须**自带能力真值**
// ═══════════════════════════════════════════════════════════════════════════
//
// 纪律：**测不到就如实报「测不到」** —— 内存库没有可重开的本库路径 ⇒
// 依赖 store 的探针必须报 `UNAVAILABLE` + 真因，**不许**静默当 0。

/// 内存库没有可重开路径 ⇒ 依赖 store 的探针必须**如实**报不可测。
#[test]
fn the_report_carries_the_capability_audit_and_never_hides_an_unmeasurable_probe() {
    let dir = tempdir().expect("临时目录");
    let conn = memory_conn();
    let out = dir.path().join("package.json");
    let report = handover_package_from_conn(
        &conn,
        &out,
        &PathBuf::from("/tmp/fake.sqlite3"),
        Vec::new(),
        Vec::new(),
    )
    .expect("打包应成功");
    let text = render(&report);
    assert!(
        text.contains("能力接线审计"),
        "报告里必须有能力审计段：\n{text}"
    );
    for needle in ["ARMED", "IDLE", "UNAVAILABLE"] {
        assert!(text.contains(needle), "审计段应含计数标签 {needle}：\n{text}");
    }
    assert!(
        text.contains("没有可重开的本库文件路径"),
        "内存库上依赖 store 的探针必须**如实报真因**（不许当 0 条）：\n{text}"
    );
}

/// 文件库（真实 schema）⇒ 写侧/读侧能力必须真的翻成 `ARMED`。
#[test]
fn a_file_backed_database_arms_the_real_capabilities() {
    let dir = tempdir().expect("临时目录");
    let db_path = dir.path().join("cognitive.sqlite3");
    let store = apeireth_memory::SqliteMemoryStore::open(&db_path).expect("建库");
    let conn = store.conn().expect("连接");
    let episode = apeireth_core::kernel::memory::Episode {
        id: "ep-cli-audit".to_string(),
        timestamp: 1_760_000_000_000,
        role: "user".to_string(),
        content: "cli 侧审计用的一段真实正文".to_string(),
        session_id: "sess-cli-audit".to_string(),
    };
    store
        .put_episode_for_subject(&episode, "cid-cli-audit")
        .expect("写记录");
    let out = dir.path().join("package.json");
    let report = handover_package_from_conn(
        &conn,
        &out,
        &db_path,
        Vec::new(),
        Vec::new(),
    )
    .expect("打包应成功");
    let text = render(&report);
    assert!(
        text.contains("ARMED"),
        "文件库上有真记录 ⇒ 审计必须出现 ARMED：\n{text}"
    );
    assert!(
        text.contains("memory_view") && text.contains("offline_embed"),
        "审计段必须列出被审计的能力（memory_view / offline_embed）：\n{text}"
    );
}
