//! `apeireth memory rollback-drill` 的测试 —— **迁移回滚可演练**的生产出口。
//!
//! # 为什么要有这个命令
//!
//! `docs/status/spec-batch-status.md` 的 B4 行写着「✅ 完成（补了 CLI 出口）」，
//! 但 `apeireth memory` 子命令里**只有** `verify` / `migrate-subject` / `export` /
//! `handover` ⇒ ⇒ 「回滚**可演练**」这件事在生产面**没有出口**
//! ⇒ ⇒ 本命令就是那个出口（`apeireth_memory::migrate_rollback::drill_rollback`）。
//!
//! # 本文件钉死的纪律
//!
//! * **接线前不可用 / 接线后可用**：接线前 `memory rollback-drill` 落到分发块的
//!   `_` 兜底臂 ⇒ 打印别的命令的用法 ⇒ **退出码非 0**；
//!   接线后 ⇒ 真的跑演练 ⇒ **退出码 0** 且报告里出现「回到原点」的机器可证结论；
//! * **演练不许改原记录**（P1 底线）⇒ 演练后库里**一条 `mig-` 副本都不许留**；
//! * **`--db` 必填** ⇒ 不给「手一抖把生产库当演练库」的口子；
//! * **源锚点没有记录 ⇒ 演练判失败**（退出码 2）⇒ 不许「没演练也报通过」。

use std::path::Path;
use std::process::Command;

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::{EpisodeStore, SqliteMemoryStore};

/// 源锚点（**会话锚点** ⇒ 与 `migrate_subject` 的取源口径同源）。
const FROM: &str = "drill-anchor-a";

/// 目标锚点（演练用的新主体）。
const TO: &str = "drill-anchor-b";

/// 真正被演练的**主体**（迁移不动它 ⇒ 回滚后必须原样还在）。
const SUBJECT: &str = "drill-subject-x";

/// 真实 CLI 二进制路径（与同目录其它 CLI 测试同一口径）。
fn bin_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("current test binary path");
    path.pop(); // 退出测试可执行文件
    if path.ends_with("deps") {
        path.pop(); // 退出 deps 目录 → target/debug
    }
    path.push(if cfg!(windows) {
        "apeireth.exe"
    } else {
        "apeireth"
    });
    path
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

/// 造一个**文件库**（演练要同库第二个连接 ⇒ 必须是文件库），并写入 3 条带链原记录。
fn seed_database(db_path: &Path) {
    let store = SqliteMemoryStore::open(db_path).expect("开库");
    for (id, ts, content) in [
        ("ep-a", 100_i64, "事实一"),
        ("ep-b", 101_i64, "事实二"),
        ("ep-c", 102_i64, "事实三"),
    ] {
        store
            .put_episode_for_subject(
                &Episode {
                    id: id.to_string(),
                    timestamp: ts,
                    role: "user".to_string(),
                    content: content.to_string(),
                    session_id: FROM.to_string(),
                },
                SUBJECT,
            )
            .expect("写入原记录");
    }
}

/// 库里的记录 id（**有序**）⇒ 用来证明「演练后没有留下副本」。
fn episode_ids(db_path: &Path) -> Vec<String> {
    let store = SqliteMemoryStore::open(db_path).expect("开库");
    let conn = store.conn().expect("拿连接");
    let mut stmt = conn
        .prepare("SELECT id FROM episodes ORDER BY id ASC")
        .expect("准备查询");
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("执行查询");
    let mut out = Vec::new();
    for row in rows {
        out.push(row.expect("读一行"));
    }
    out
}

/// ⭐ **接线前不可用 / 接线后可用**：子命令**真的被分发**（走真实二进制）。
#[test]
fn rollback_drill_subcommand_is_dispatched_and_proves_return_to_origin() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("cognitive.sqlite3");
    seed_database(&db_path);

    let bin = bin_path();
    assert!(bin.exists(), "CLI 二进制应存在：{}", bin.display());

    let output = Command::new(&bin)
        .args(args(&[
            "memory",
            "rollback-drill",
            "--from",
            FROM,
            "--to",
            TO,
            "--db",
            &db_path.display().to_string(),
        ]))
        .output()
        .expect("执行 CLI");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "演练应通过（退出码 0）\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("回到原点"),
        "报告必须给出「回到原点」的机器可证结论：\n{stdout}"
    );
    assert!(
        stdout.contains("一条未动"),
        "报告必须如实说清原记录一条未动（P1）：\n{stdout}"
    );

    // ⚠️ **原记录一条不少 + 副本一条不留** ⇒ 演练真的回到了原点。
    let ids = episode_ids(&db_path);
    assert_eq!(
        ids,
        vec!["ep-a".to_string(), "ep-b".to_string(), "ep-c".to_string()],
        "演练后库里只剩原记录"
    );
    assert!(
        !ids.iter().any(|id| id.starts_with("mig-")),
        "演练后不许留下任何 `mig-` 副本：{ids:?}"
    );
}

/// `--db` **必填** ⇒ 缺了就报错（**不是**「默认打生产库」）。
#[test]
fn rollback_drill_requires_an_explicit_database() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("cognitive.sqlite3");
    seed_database(&db_path);

    let bin = bin_path();
    assert!(bin.exists(), "CLI 二进制应存在：{}", bin.display());

    let output = Command::new(&bin)
        .args(args(&[
            "memory",
            "rollback-drill",
            "--from",
            FROM,
            "--to",
            TO,
        ]))
        .output()
        .expect("执行 CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "缺 --db ⇒ 退出码 2\nstdout:\n{}\nstderr:\n{stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        stderr.contains("--db"),
        "报错必须点明 `--db` 必填：\n{stderr}"
    );
}

/// 源锚点下没有记录 ⇒ **没有可演练之物** ⇒ 判失败（退出码 2），不许「没演练也报通过」。
#[test]
fn rollback_drill_fails_when_the_source_anchor_has_no_records() {
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("cognitive.sqlite3");
    // 只建库、不写记录。
    let _ = SqliteMemoryStore::open(&db_path).expect("开库");

    let bin = bin_path();
    assert!(bin.exists(), "CLI 二进制应存在：{}", bin.display());

    let output = Command::new(&bin)
        .args(args(&[
            "memory",
            "rollback-drill",
            "--from",
            FROM,
            "--to",
            TO,
            "--db",
            &db_path.display().to_string(),
        ]))
        .output()
        .expect("执行 CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "源锚点没记录 ⇒ 演练判失败（退出码 2）\nstderr:\n{stderr}"
    );
}

/// 未知子命令 ⇒ 落到 `_` 兜底臂 ⇒ **退出码非 0**（这就是「接线前 `rollback-drill` 的行为」）。
#[test]
fn unknown_memory_subcommand_is_rejected() {
    let bin = bin_path();
    assert!(bin.exists(), "CLI 二进制应存在：{}", bin.display());

    let output = Command::new(&bin)
        .args(args(&["memory", "rollback-drill-not-a-command"]))
        .output()
        .expect("执行 CLI");
    assert_ne!(output.status.code(), Some(0), "未知子命令不许退出码 0");
}
