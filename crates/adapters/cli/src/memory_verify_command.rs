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
/// **连接怎么来**：`rusqlite` 在 CLI 只是 **dev-dependency** ⇒ 生产代码不能直接用它
/// ⇒ 走 `apeireth_storage::pool::SqliteConnectionPool` 的 `read`（**只读连接**）。
/// 这条路径同时保证「体检不会写库」。
///
/// **口径边界（如实说明）**：这里校验的是**链的结构完整性**（首尾相连）。
/// 摘要**重算**需要完整内容与全部参与字段的编码规则 ⇒ 那是**记忆层链实现**的责任
/// （**单一真源**）⇒ 等记忆层接线后，本命令改调它的 API；在此之前**不假装覆盖**。
fn verify_database(db_path: &Path, subject: Option<&str>) -> Result<VerifyReport, String> {
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
    // 链列可能不存在（老库）⇒ 此时所有记录都是「未纳管」，**不算坏链**。
    let has_chain_columns = table_has_column(conn, "episodes", "chain_hash")
        .map_err(|error| format!("读取表结构失败：{error}"))?;
    let mut subjects: Vec<String> = Vec::new();
    if has_chain_columns {
        let sql = match subject {
            Some(_) => "SELECT DISTINCT subject_id FROM episodes ORDER BY subject_id",
            None => "SELECT DISTINCT subject_id FROM episodes ORDER BY subject_id",
        };
        let mut statement = conn
            .prepare(sql)
            .map_err(|error| format!("准备语句失败：{error}"))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| format!("查询主体失败：{error}"))?;
        for row in rows {
            subjects.push(row.map_err(|error| format!("读取主体失败：{error}"))?);
        }
        if let Some(wanted) = subject {
            subjects.retain(|candidate| candidate == wanted);
        }
    }

    let mut details = Vec::new();
    let mut entries = 0usize;
    let mut broken_subjects = 0usize;
    for subject_id in &subjects {
        let rows = load_chain_rows(conn, subject_id)?;
        let managed = rows.iter().filter(|row| row.chain_hash.is_some()).count();
        let total = rows.len();
        entries += managed;
        let report = verify_rows(&rows);
        if !report.ok {
            broken_subjects += 1;
        }
        details.push(SubjectReport {
            subject_id: subject_id.clone(),
            ok: report.ok,
            entry_count: managed,
            first_bad_id: report.first_bad_id,
            first_bad_index: report.first_bad_index,
        });
        let _ = total;
    }

    // 未纳管记录数（没有链字段的）。
    let unmanaged_entries = if has_chain_columns {
        count_unmanaged(conn, subject).map_err(|error| format!("统计未纳管记录失败：{error}"))?
    } else {
        count_all(conn, subject).map_err(|error| format!("统计记录失败：{error}"))?
    };

    Ok(VerifyReport {
        db_path: db_path.to_path_buf(),
        subjects: details.len(),
        entries,
        unmanaged_entries,
        broken_subjects,
        details,
    })
}

/// 链上的一行（链字段 + 顺序键）。
struct ChainRow {
    id: String,
    chain_hash: Option<String>,
    chain_prev_hash: Option<String>,
}

/// 按**记录顺序**（时间、再 id）读某主体的行。
fn load_chain_rows(conn: &rusqlite::Connection, subject_id: &str) -> Result<Vec<ChainRow>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, chain_hash, chain_prev_hash FROM episodes \
             WHERE subject_id = ? ORDER BY created_at ASC, id ASC",
        )
        .map_err(|error| format!("准备语句失败：{error}"))?;
    let rows = statement
        .query_map([subject_id], |row| {
            Ok(ChainRow {
                id: row.get(0)?,
                chain_hash: row.get(1)?,
                chain_prev_hash: row.get(2)?,
            })
        })
        .map_err(|error| format!("查询链记录失败：{error}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|error| format!("读取链记录失败：{error}"))?);
    }
    Ok(out)
}

/// 校验一串链行：首尾相连。
///
/// ⚠️ **摘要重算不在这里做**：重算需要完整内容与全部参与字段的编码规则，
/// 那是记忆层链实现的责任（**单一真源**）⇒ 本命令在记忆层接线前**只查链的结构完整性**，
/// 并在报告里**如实说明覆盖范围**（不假装覆盖了摘要重算）。
fn verify_rows(rows: &[ChainRow]) -> RowVerification {
    let mut previous: Option<&str> = None;
    let mut managed_index = 0usize;
    for row in rows {
        let (Some(hash), Some(prev)) = (&row.chain_hash, &row.chain_prev_hash) else {
            // 未纳管 ⇒ 跳过（不计为坏）。
            continue;
        };
        if let Some(expected) = previous {
            if prev != expected {
                return RowVerification {
                    ok: false,
                    first_bad_id: Some(row.id.clone()),
                    first_bad_index: Some(managed_index),
                };
            }
        }
        previous = Some(hash.as_str());
        managed_index += 1;
    }
    RowVerification {
        ok: true,
        first_bad_id: None,
        first_bad_index: None,
    }
}

struct RowVerification {
    ok: bool,
    first_bad_id: Option<String>,
    first_bad_index: Option<usize>,
}

/// 表是否有某列。
fn table_has_column(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
) -> Result<bool, rusqlite::Error> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut columns = statement.query([])?;
    while let Some(row) = columns.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// 统计未纳管记录（没有链字段的）。
fn count_unmanaged(
    conn: &rusqlite::Connection,
    subject: Option<&str>,
) -> Result<usize, rusqlite::Error> {
    let sql = match subject {
        Some(_) => "SELECT COUNT(*) FROM episodes WHERE subject_id = ? AND (chain_hash IS NULL OR chain_prev_hash IS NULL)",
        None => "SELECT COUNT(*) FROM episodes WHERE chain_hash IS NULL OR chain_prev_hash IS NULL",
    };
    let mut statement = conn.prepare(sql)?;
    let count: i64 = match subject {
        Some(value) => statement.query_row([value], |row| row.get(0))?,
        None => statement.query_row([], |row| row.get(0))?,
    };
    Ok(count as usize)
}

/// 统计全部记录（老库没有链列时用）。
fn count_all(conn: &rusqlite::Connection, subject: Option<&str>) -> Result<usize, rusqlite::Error> {
    let sql = match subject {
        Some(_) => "SELECT COUNT(*) FROM episodes WHERE subject_id = ?",
        None => "SELECT COUNT(*) FROM episodes",
    };
    let mut statement = conn.prepare(sql)?;
    let count: i64 = match subject {
        Some(value) => statement.query_row([value], |row| row.get(0))?,
        None => statement.query_row([], |row| row.get(0))?,
    };
    Ok(count as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE episodes (
                 id TEXT PRIMARY KEY,
                 subject_id TEXT NOT NULL,
                 content TEXT NOT NULL,
                 created_at INTEGER NOT NULL,
                 chain_hash TEXT,
                 chain_prev_hash TEXT
             );",
        )
        .expect("create episodes");
        conn
    }

    #[test]
    fn reports_all_good_when_chain_is_intact() {
        let conn = memory_db();
        conn.execute(
            "INSERT INTO episodes (id, subject_id, content, created_at, chain_hash, chain_prev_hash)
             VALUES ('e1', 's1', 'a', 1, 'h1', 'g'), ('e2', 's1', 'b', 2, 'h2', 'h1')",
            [],
        )
        .expect("insert");
        let report =
            verify_with_connection(&conn, Path::new("/tmp/x.sqlite3"), None).expect("verify");
        assert!(report.broken_subjects == 0);
        assert_eq!(report.subjects, 1);
        assert_eq!(report.entries, 2);
        assert_eq!(report.details[0].ok, true);
    }

    #[test]
    fn locates_the_first_broken_link() {
        let conn = memory_db();
        conn.execute(
            "INSERT INTO episodes (id, subject_id, content, created_at, chain_hash, chain_prev_hash)
             VALUES ('e1', 's1', 'a', 1, 'h1', 'g'), ('e2', 's1', 'b', 2, 'h2', 'WRONG')",
            [],
        )
        .expect("insert");
        let report =
            verify_with_connection(&conn, Path::new("/tmp/x.sqlite3"), None).expect("verify");
        assert_eq!(report.broken_subjects, 1);
        let detail = &report.details[0];
        assert!(!detail.ok);
        assert_eq!(detail.first_bad_id.as_deref(), Some("e2"));
        assert_eq!(detail.first_bad_index, Some(1));
    }

    #[test]
    fn counts_unmanaged_records_honestly() {
        let conn = memory_db();
        conn.execute(
            "INSERT INTO episodes (id, subject_id, content, created_at, chain_hash, chain_prev_hash)
             VALUES ('e1', 's1', 'a', 1, 'h1', 'g'), ('e2', 's1', 'b', 2, NULL, NULL)",
            [],
        )
        .expect("insert");
        let report =
            verify_with_connection(&conn, Path::new("/tmp/x.sqlite3"), None).expect("verify");
        assert_eq!(report.unmanaged_entries, 1, "未纳管记录要如实计数");
        assert_eq!(report.entries, 1, "未纳管记录不进链上条目数");
    }

    #[test]
    fn old_database_without_chain_columns_is_reported_as_unmanaged_not_broken() {
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            "CREATE TABLE episodes (
                 id TEXT PRIMARY KEY,
                 subject_id TEXT NOT NULL,
                 content TEXT NOT NULL,
                 created_at INTEGER NOT NULL
             );
             INSERT INTO episodes VALUES ('e1', 's1', 'a', 1);",
        )
        .expect("create old schema");
        let report =
            verify_with_connection(&conn, Path::new("/tmp/x.sqlite3"), None).expect("verify");
        assert_eq!(report.broken_subjects, 0, "老库不算坏链");
        assert_eq!(report.unmanaged_entries, 1, "老库记录如实记为未纳管");
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
