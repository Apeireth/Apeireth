//! `apeireth memory export` 的测试 —— **只读导出** + **当场自校验**。
//!
//! **核心纪律**：
//! * **只读**（导出不动库）；
//! * **自校验通过才产出归档物**（不产出「导出了但坏了」的归档物）；
//! * **未纳管**记录一并导出并**如实计数**；
//! * **参数错 / 库不存在 / 自校验失败** ⇒ 三种退出码**分开**。

use apeireth_cli::memory_export_command::{
    export_to_path, parse_options, render, run_export, ExportFailure,
};
use std::path::PathBuf;
use tempfile::tempdir;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn parses_the_options() {
    let options = parse_options(&args(&[
        "--db",
        "/tmp/x.sqlite3",
        "--out",
        "/tmp/a.json",
        "--subject",
        "alice",
        "--limit",
        "5",
    ]))
    .expect("应能解析");
    assert_eq!(options.db, Some(PathBuf::from("/tmp/x.sqlite3")));
    assert_eq!(options.out, PathBuf::from("/tmp/a.json"));
    assert_eq!(options.subject, Some("alice".to_string()));
    assert_eq!(options.limit, 5);
}

#[test]
fn rejects_missing_out() {
    // ⇒ **--out 必填**（⇒ 忘了就报错，不是「导到默认路径」）。
    assert!(parse_options(&args(&["--db", "/tmp/x.sqlite3"])).is_err());
}

#[test]
fn rejects_unknown_flags() {
    assert!(parse_options(&args(&["--out", "/tmp/a.json", "--wat"])).is_err());
}

#[test]
fn rejects_a_non_numeric_limit() {
    assert!(parse_options(&args(&["--out", "/tmp/a.json", "--limit", "abc"])).is_err());
}

#[test]
fn a_missing_database_is_unavailable_not_a_pass() {
    // ⇒ **库不存在 ⇒ 退出码 2**（**不是**「导出成功但空的」）。
    let dir = tempdir().expect("临时目录");
    let missing = dir.path().join("missing.sqlite3");
    let out = dir.path().join("archive.json");
    let error = run_export(&args(&[
        "--db",
        missing.to_str().expect("路径"),
        "--out",
        out.to_str().expect("路径"),
    ]))
    .expect_err("库不存在应失败");
    assert!(
        matches!(error, ExportFailure::Unavailable(_)),
        "实际 {error:?}"
    );
}

#[test]
fn usage_error_is_its_own_kind() {
    let error = run_export(&args(&["--db"])).expect_err("缺参数应失败");
    assert_eq!(error, ExportFailure::Usage);
}

#[test]
fn exports_and_self_verifies() {
    // ⇒ 🎯 **导出 + 自校验通过** ⇒ ⇒ 产出物存在且**自洽**。
    let dir = tempdir().expect("临时目录");
    let conn = rusqlite::Connection::open_in_memory().expect("内存库");
    // ⇒ 建最小表结构（与离线归档读取器一致）。
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS episodes (
            id TEXT PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            session_id TEXT,
            chain_hash TEXT,
            chain_prev_hash TEXT,
            continuity_id TEXT
        );
        CREATE TABLE IF NOT EXISTS chain_heads (
            subject_id TEXT PRIMARY KEY,
            entry_count INTEGER NOT NULL,
            head_hash TEXT
        );",
    )
    .expect("建表");
    conn.execute(
        "INSERT INTO episodes (id, timestamp, role, content, session_id)
         VALUES ('e1', 1000, 'user', '第一段记忆', 's1')",
        [],
    )
    .expect("插入");
    conn.execute(
        "INSERT INTO episodes (id, timestamp, role, content, session_id)
         VALUES ('e2', 2000, 'user', '第二段记忆', 's1')",
        [],
    )
    .expect("插入");

    let out = dir.path().join("archive.json");
    let report = export_to_path(&conn, &out, None, 0).expect("导出应成功");
    assert_eq!(report.entries, 2, "应导出两条");
    assert!(!report.content_digest.is_empty(), "应给出内容哈希");
    assert!(out.exists(), "归档物文件应存在");

    let text = std::fs::read_to_string(&out).expect("读归档物");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("应是合法 JSON");
    assert!(parsed.get("manifest").is_some(), "应有清单：{parsed}");
    assert_eq!(
        parsed["episodes"].as_array().map(|items| items.len()),
        Some(2)
    );
}

#[test]
fn the_report_states_the_key_numbers() {
    let dir = tempdir().expect("临时目录");
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
        "INSERT INTO episodes (id, timestamp, role, content, session_id)
         VALUES ('e1', 1000, 'user', '未纳管的记忆', 's1')",
        [],
    )
    .expect("插入");
    let out = dir.path().join("archive.json");
    let report = export_to_path(&conn, &out, None, 0).expect("导出应成功");
    let text = render(&report);
    for needle in [
        "记忆导出完成",
        "导出条数",
        "未纳管条数",
        "内容哈希",
        "自校验通过",
    ] {
        assert!(text.contains(needle), "报告里应含 {needle}：\n{text}");
    }
    assert_eq!(report.unmanaged_entries, 1, "未纳管条数应如实计数");
}
