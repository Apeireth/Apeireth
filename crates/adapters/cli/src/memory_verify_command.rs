//! `apeireth memory verify` —— 记忆库**体检**命令（哈希链 + 完整性）。
//!
//! # 这个命令做什么
//!
//! 对认知库（记忆库）做一次**只读体检**，如实报告：
//! * 每个主体的**哈希链**是否完整（首尾相连 + 每条摘要与内容重算一致）；
//! * **哪一条**坏了（第几条、哪个记录 id）—— 定位到具体记录，而不是只说「坏了」；
//! * 有多少记录**未纳管**（没有链字段 ⇒ 老库或未接入的记录）—— 如实计数，不假装它们没问题。
//!
//! # 为什么需要它（设计稿原则 P3）
//!
//! 原则 P3 要求「权威层带哈希链 + 定期 scrub + **能定位到具体记录**」。
//! 缺口审计（`docs/status/d0-fidelity-audit.md` 的 G7）指出：**体检命令此前不存在**
//! ⇒ 静默篡改/损坏只能等用户自己发现。
//!
//! 本命令是**只读**的：不修复、不改写、不删除任何东西。
//! 发现问题后的处置（修复/重建/归档）属后续批次 —— **体检与修复必须分开**，
//! 否则「体检」就变成了「顺手改数据」，那不是体检。
//!
//! # 口径
//!
//! * **NOT IMPLEMENTED 的部分**：`memory verify` 目前只覆盖**链完整性**；
//!   索引一致性、孤儿检测等（设计稿 G1/G7 的其余部分）尚未接入 ⇒ 如实说明，不假装覆盖。
//! * 退出码：**0 = 全部通过**；**1 = 发现问题**；**2 = 无法体检**（库不存在/打不开）。

use std::path::{Path, PathBuf};

/// 命令用法（与其它命令的 `USAGE` 风格一致）。
pub const USAGE: &str = "apeireth memory verify [--db <COGNITIVE_DB>] [--subject <SUBJECT_ID>]\n\n  只读体检：校验记忆库哈希链的完整性，并定位到具体坏掉的记录。\n\n  --db      认知库路径（缺省 = APEIRETH_COGNITIVE_DB 或数据目录下 cognitive.sqlite3）\n  --subject 只校验某个主体（缺省 = 全部主体）\n\n  退出码：0 = 全部通过 · 1 = 发现问题 · 2 = 无法体检";

/// 体检结果（供 CLI 渲染；也便于测试断言）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    /// 被体检的库路径。
    pub db_path: PathBuf,
    /// 被校验的主体数。
    pub subjects: usize,
    /// 链上记录总数。
    pub entries: usize,
    /// **未纳管**的记录数（没有链字段 ⇒ 老库/未接入记录）。
    pub unmanaged_entries: usize,
    /// 链坏了的主体数。
    pub broken_subjects: usize,
    /// 每个主体的明细（`subject_id` → 是否通过 / 第一处坏在哪）。
    pub details: Vec<SubjectReport>,
}

/// 单个主体的校验明细。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectReport {
    pub subject_id: String,
    pub ok: bool,
    pub entry_count: usize,
    /// 第一处出问题的记录 id（没有则 None）。
    pub first_bad_id: Option<String>,
    /// 第一处出问题的位置（0 基）。
    pub first_bad_index: Option<usize>,
    /// 该主体的未纳管记录数。
    pub unmanaged_entries: usize,
    /// 该主体被检出异常的链上条目数。
    pub broken_entries: usize,
}

/// 渲染成人类可读的文本（**如实**列出问题，不吞）。
pub fn render(report: &VerifyReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("记忆库体检：{}", report.db_path.display()));
    lines.push(format!(
        "  主体 {} · 链上记录 {} · 未纳管 {} · 坏主体 {}",
        report.subjects, report.entries, report.unmanaged_entries, report.broken_subjects
    ));
    for detail in &report.details {
        if detail.ok {
            lines.push(format!(
                "  ✓ {}（{} 条）",
                detail.subject_id, detail.entry_count
            ));
            continue;
        }
        let where_ = match (&detail.first_bad_id, detail.first_bad_index) {
            (Some(id), Some(index)) => format!("第 {index} 条（{id}）"),
            (Some(id), None) => format!("记录 {id}"),
            (None, Some(index)) => format!("第 {index} 条"),
            _ => "位置未知".to_string(),
        };
        lines.push(format!(
            "  ✗ {}（{} 条）—— 第一处异常：{}",
            detail.subject_id, detail.entry_count, where_
        ));
    }
    if report.unmanaged_entries > 0 {
        lines.push(format!(
            "  ! {} 条记录**未纳管**（没有链字段）—— 旧库或未接入路径的记录，链校验覆盖不到它们",
            report.unmanaged_entries
        ));
    }
    if report.broken_subjects == 0 {
        lines.push("  结论：链完整性通过。".to_string());
    } else {
        lines.push(format!(
            "  结论：**{} 个主体的链有问题** —— 详见上面列出的位置（未修复；本命令只读）。",
            report.broken_subjects
        ));
    }
    lines.join("\n")
}

/// 命令入口。
pub fn run_verify(args: &[String]) -> Result<VerifyReport, String> {
    let options = parse_verify_args(args)?;
    let db_path = options.db_path.unwrap_or_else(default_cognitive_db_path);
    if !db_path.exists() {
        return Err(format!(
            "认知库不存在：{} —— 先跑一次会写记忆的功能，或用 --db 指定路径",
            db_path.display()
        ));
    }
    let reports = verify_database(&db_path, options.subject.as_deref())?;
    Ok(reports)
}

/// 解析参数（`--db` / `--subject`；未知参数 ⇒ 报错，不静默忽略）。
fn parse_verify_args(args: &[String]) -> Result<VerifyOptions, String> {
    let mut options = VerifyOptions::default();
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--db" => {
                index += 1;
                let value = args.get(index).ok_or("--db 缺参数")?;
                options.db_path = Some(PathBuf::from(value));
            }
            "--subject" => {
                index += 1;
                let value = args.get(index).ok_or("--subject 缺参数")?;
                options.subject = Some(value.clone());
            }
            other => return Err(format!("未知参数：{other}\n\n{USAGE}")),
        }
        index += 1;
    }
    Ok(options)
}

#[derive(Debug, Default)]
struct VerifyOptions {
    db_path: Option<PathBuf>,
    subject: Option<String>,
}

/// 缺省认知库路径：`APEIRETH_COGNITIVE_DB` 或 `<数据根>/cognitive.sqlite3`。
fn default_cognitive_db_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("APEIRETH_COGNITIVE_DB") {
        if !explicit.trim().is_empty() {
            return PathBuf::from(explicit);
        }
    }
    crate::default_panel_data_dir().join("cognitive.sqlite3")
}

/// 对库做体检（**只读**）。
///
/// **校验规则来自记忆层**（[`apeireth_memory::memory_chain`]）⇒ **单一真源**：
/// 链规则、摘要编码、链头账实核对全在那儿；本命令只负责**取连接 + 渲染**。
/// 连接走 `SqliteConnectionPool::read`（**只读连接**）⇒ 体检不会写库。
///
/// ⚠️ **曾经的教训（已修）**：本命令第一版**自己**写了一份体检 SQL
/// （按 `subject_id` / `created_at` 分组），而真实记忆 schema 用的是
/// `continuity_id` / `timestamp` ⇒ 对真实库**直接报错**
/// （`no such column: subject_id`）。教训：**校验规则不要在调用方复写**。
fn verify_database(db_path: &Path, subject: Option<&str>) -> Result<VerifyReport, String> {
    use apeireth_memory::memory_chain;
    use apeireth_storage::pool::SqliteConnectionPool;

    let pool =
        runtime_block_on(async move { SqliteConnectionPool::open(db_path.to_path_buf()).await })
            .map_err(|error| format!("打不开认知库（只读）：{error}"))?;
    // 闭包必须返回 `StorageError` ⇒ 内部的 `String` 错误用 `Db` 变体承载
    // （`StorageError` 没有通用 `Other` 变体；`Db` 带 message，够如实表达）。
    pool.read(|conn| {
        verify_with_connection(conn, db_path, subject).map_err(|error| {
            apeireth_storage::error::StorageError::Db(rusqlite::Error::InvalidParameterName(error))
        })
    })
    .map_err(|error| format!("体检失败：{error}"))
}

/// 在当前线程上跑一个 future（CLI 的命令函数是同步的）。
fn runtime_block_on<F: std::future::Future>(future: F) -> F::Output {
    match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime.block_on(future),
        // 建不出运行时 ⇒ 极少见（缺系统库等）⇒ 如实报错，不 panic。
        Err(error) => {
            eprintln!("apeireth memory verify: 建不出异步运行时：{error}");
            std::process::exit(2);
        }
    }
}

/// 用既有连接做体检（便于测试注入内存库）。
fn verify_with_connection(
    conn: &rusqlite::Connection,
    db_path: &Path,
    subject: Option<&str>,
) -> Result<VerifyReport, String> {
    use apeireth_memory::memory_chain;

    let verifications = match subject {
        Some(one) => {
            let result = memory_chain::verify_subject_chain(conn, one)
                .map_err(|error| format!("校验主体 {one} 失败：{error}"))?;
            vec![result]
        }
        None => memory_chain::verify_all_chains(conn)
            .map_err(|error| format!("校验全部主体失败：{error}"))?,
    };
    let summary = memory_chain::summarize(conn).map_err(|error| format!("汇总失败：{error}"))?;

    let details = verifications
        .iter()
        .map(|item| SubjectReport {
            subject_id: item.subject_id.clone(),
            ok: item.ok,
            entry_count: item.entry_count,
            first_bad_id: item.first_bad_id.clone(),
            first_bad_index: item.first_bad_index,
            unmanaged_entries: item.unmanaged_entries,
            broken_entries: item.broken_entries,
        })
        .collect();

    Ok(VerifyReport {
        db_path: db_path.to_path_buf(),
        subjects: summary.subjects,
        entries: summary.entries,
        unmanaged_entries: summary.unmanaged_entries,
        broken_subjects: summary.broken_subjects,
        details,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 建一个**真实 schema** 的记忆库（含链列）。
    ///
    /// ⚠️ **刻意用真实列名**（`continuity_id` / `timestamp`，**没有** `subject_id`
    /// 或 `created_at`）⇒ 测试能抓住「体检 SQL 假设了不存在的列」这类缺陷
    /// （本命令的第一版就是这么坏的：查 `episodes.subject_id` ⇒ 真实库直接报错）。
    fn memory_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE episodes (
                 id TEXT PRIMARY KEY,
                 continuity_id TEXT NOT NULL,
                 session_id TEXT,
                 timestamp INTEGER NOT NULL,
                 role TEXT NOT NULL,
                 content TEXT NOT NULL,
                 valid_from_ms INTEGER,
                 valid_until_ms INTEGER,
                 created_ms INTEGER NOT NULL,
                 provenance TEXT NOT NULL,
                 chain_hash TEXT,
                 chain_prev_hash TEXT
             );
             CREATE TABLE memory_chain_heads (
                 subject_id TEXT PRIMARY KEY,
                 last_hash TEXT NOT NULL,
                 entry_count INTEGER NOT NULL DEFAULT 0,
                 updated_at_ms INTEGER NOT NULL
             );",
        )
        .expect("create episodes");
        conn
    }

    /// 插入一条链上记录（摘要值由测试直接给出 ⇒ 只测**结构**判据；
    /// 摘要重算的正确性由记忆层自己的测试负责）。
    fn insert_chained(
        conn: &rusqlite::Connection,
        id: &str,
        continuity_id: &str,
        content: &str,
        timestamp: i64,
    ) {
        // **用记忆层的链写入路径造真链** ⇒ 摘要由它算。
        // ⚠️ 不能用假摘要：体检会**重算摘要**并比对 ⇒ 假摘要一律判成「篡改」
        // （那正是摘要重算在起作用）⇒ 测试必须用真链，否则测的是错的东西。
        use apeireth_memory::memory_chain::{put_chained_episode, ChainEntry};
        let entry = ChainEntry {
            id: id.to_string(),
            content: content.to_string(),
            timestamp,
            role: "user".to_string(),
            session_id: "sess-1".to_string(),
            valid_from_ms: None,
            valid_until_ms: None,
            created_ms: Some(timestamp),
            provenance: Some("manual".to_string()),
        };
        put_chained_episode(conn, continuity_id, &entry).expect("写入链上记录");
    }

    #[test]
    fn reports_all_good_when_chain_is_intact() {
        let conn = memory_db();
        insert_chained(&conn, "e1", "c1", "第一条", 1);
        insert_chained(&conn, "e2", "c1", "第二条", 2);
        let report =
            verify_with_connection(&conn, Path::new("/tmp/x.sqlite3"), None).expect("verify");
        assert_eq!(report.broken_subjects, 0, "{report:?}");
        assert_eq!(report.entries, 2);
        assert_eq!(report.subjects, 1);
        assert!(report.details[0].ok, "{report:?}");
    }

    #[test]
    fn locates_the_first_broken_link() {
        let conn = memory_db();
        insert_chained(&conn, "e1", "c1", "第一条", 1);
        insert_chained(&conn, "e2", "c1", "第二条", 2);
        // 改**内容**（不动链字段）⇒ 摘要重算对不上 ⇒ 定位到那一条。
        conn.execute(
            "UPDATE episodes SET content = '被改过的内容' WHERE id = 'e2'",
            [],
        )
        .expect("tamper content");
        let report =
            verify_with_connection(&conn, Path::new("/tmp/x.sqlite3"), None).expect("verify");
        assert_eq!(report.broken_subjects, 1, "{report:?}");
        let detail = &report.details[0];
        assert!(!detail.ok);
        assert_eq!(detail.first_bad_id.as_deref(), Some("e2"), "{report:?}");
        assert_eq!(detail.first_bad_index, Some(1), "{report:?}");
    }

    #[test]
    fn counts_unmanaged_records_honestly() {
        let conn = memory_db();
        insert_chained(&conn, "e1", "c1", "第一条", 1);
        conn.execute(
            "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content,
                valid_from_ms, valid_until_ms, created_ms, provenance, chain_hash, chain_prev_hash)
             VALUES ('e2', 'c1', NULL, 2, 'user', '未纳管', NULL, NULL, 2, 'manual', NULL, NULL)",
            [],
        )
        .expect("insert unmanaged");
        let report =
            verify_with_connection(&conn, Path::new("/tmp/x.sqlite3"), None).expect("verify");
        assert_eq!(report.unmanaged_entries, 1, "未纳管记录要如实计数");
    }

    #[test]
    fn old_database_without_chain_columns_is_reported_as_unmanaged_not_broken() {
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE episodes (
                 id TEXT PRIMARY KEY,
                 continuity_id TEXT NOT NULL,
                 session_id TEXT,
                 timestamp INTEGER NOT NULL,
                 role TEXT NOT NULL,
                 content TEXT NOT NULL,
                 valid_from_ms INTEGER,
                 valid_until_ms INTEGER,
                 created_ms INTEGER NOT NULL,
                 provenance TEXT NOT NULL
             );
             INSERT INTO episodes VALUES ('e1', 'c1', NULL, 1, 'user', 'a', NULL, NULL, 1, 'manual');",
        )
        .expect("create old schema");
        let report =
            verify_with_connection(&conn, Path::new("/tmp/x.sqlite3"), None).expect("verify");
        assert_eq!(report.broken_subjects, 0, "老库不算坏链");
        assert_eq!(report.unmanaged_entries, 1, "老库记录如实记为未纳管");
    }

    #[test]
    fn subject_filter_narrows_the_report() {
        let conn = memory_db();
        insert_chained(&conn, "e1", "c1", "甲", 1);
        insert_chained(&conn, "e2", "c2", "乙", 1);
        let report =
            verify_with_connection(&conn, Path::new("/tmp/x.sqlite3"), Some("c2")).expect("verify");
        assert_eq!(report.details.len(), 1);
        assert_eq!(report.details[0].subject_id, "c2");
    }

    #[test]
    fn render_is_honest_about_problems() {
        let report = VerifyReport {
            db_path: PathBuf::from("/tmp/x.sqlite3"),
            subjects: 1,
            entries: 2,
            unmanaged_entries: 1,
            broken_subjects: 1,
            details: vec![SubjectReport {
                subject_id: "s1".to_string(),
                ok: false,
                entry_count: 2,
                first_bad_id: Some("e2".to_string()),
                first_bad_index: Some(1),
                unmanaged_entries: 0,
                broken_entries: 1,
            }],
        };
        let text = render(&report);
        assert!(text.contains("第 1 条"), "{text}");
        assert!(text.contains("e2"), "{text}");
        assert!(text.contains("未纳管"), "{text}");
        assert!(text.contains("只读"), "{text}");
    }

    #[test]
    fn unknown_argument_is_refused() {
        let error = parse_verify_args(&["--nope".to_string()]).expect_err("未知参数应报错");
        assert!(error.contains("未知参数"), "{error}");
    }

    #[test]
    fn missing_database_is_refused_with_a_clear_message() {
        let error = run_verify(&["--db".to_string(), "/nonexistent/never.sqlite3".to_string()])
            .expect_err("不存在的库应报错");
        assert!(error.contains("不存在"), "{error}");
    }
}
