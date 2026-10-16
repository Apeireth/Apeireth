//! 矿脉数据库面 —— 只读 SQL 查询（批次推进）。
//!
//! 契约（I-starport-builtins.md「矿脉」）：
//!   1. 只读口径双保险 —— 前端 classifySql 与本件 ensure_readonly_sql 各自
//!      独立强制（纵深：一层被绕过另一层仍拒），写类/DDL/多语句一律拒绝；
//!   2. 真引擎真文件 —— rusqlite（bundled，版本对齐主 workspace 硬锁 0.32）
//!      以 READ_ONLY 标志打开真实 SQLite 文件，天然拒绝写入；
//!   3. 有界结果 —— 行上限 [`SQL_ROW_LIMIT`]，超出截断并如实标注 truncated；
//!      blob 不外泄原文，以 `<blob N bytes>` 占位（0 装：不猜内容）；
//!   4. 外部数据库驱动（MySQL/PostgreSQL 等）为扩展接线缝。
//!
//! 测试：`cargo test`（纯函数 + 真临时库往返）。

use std::path::Path;
use std::time::{Duration, Instant};

use rusqlite::types::Value as SqlValue;
use rusqlite::OpenFlags;

/// 结果行上限（超出截断，truncated 如实标注）。
pub const SQL_ROW_LIMIT: usize = 1000;

/// 只读语句前导词（小写比对）。
const READ_PREFIXES: [&str; 4] = ["select", "with", "pragma", "explain"];

#[derive(Debug, serde::Serialize)]
pub struct SqlQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub truncated: bool,
    pub duration_ms: u64,
}

/// 剥前导注释（`--` 行注释与 `/* */` 块注释），返回剩余文本。
fn strip_leading_comments(sql: &str) -> &str {
    let mut text = sql.trim();
    loop {
        if let Some(rest) = text.strip_prefix("--") {
            text = match rest.find('\n') {
                Some(nl) => rest[nl + 1..].trim(),
                None => "",
            };
        } else if let Some(rest) = text.strip_prefix("/*") {
            text = match rest.find("*/") {
                Some(end) => rest[end + 2..].trim(),
                None => "",
            };
        } else {
            return text;
        }
    }
}

/// 只读口径校验（原生侧第二道闸）。多语句/写类/空语句一律拒绝。
pub fn ensure_readonly_sql(sql: &str) -> Result<&str, String> {
    let text = strip_leading_comments(sql);
    if text.is_empty() {
        return Err("SQL 为空".to_string());
    }
    // 多语句拒绝：分号之后仅允许空白/注释。
    if let Some(semi) = text.find(';') {
        let tail = strip_leading_comments(text[semi + 1..].trim());
        if !tail.is_empty() {
            return Err("多语句拒绝（一次只读一条）".to_string());
        }
    }
    let head_end = match text.find(';') {
        Some(semi) => semi,
        None => text.len(),
    };
    let head = text[..head_end].trim().to_ascii_lowercase();
    if READ_PREFIXES.iter().any(|prefix| head.starts_with(prefix)) {
        Ok(text)
    } else {
        Err("只读口径：仅放行 SELECT/WITH/PRAGMA/EXPLAIN".to_string())
    }
}

/// SQLite 值 → JSON（blob 不外泄原文）。
fn value_to_json(value: &SqlValue) -> serde_json::Value {
    match value {
        SqlValue::Null => serde_json::Value::Null,
        SqlValue::Integer(i) => serde_json::Value::from(*i),
        SqlValue::Real(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        SqlValue::Text(s) => serde_json::Value::String(s.clone()),
        SqlValue::Blob(b) => serde_json::Value::String(format!("<blob {} bytes>", b.len())),
    }
}

/// 只读查询一个真实 SQLite 文件（有界结果、截断如实标注）。
#[tauri::command]
pub fn query_sqlite_file(path: String, sql: String) -> Result<SqlQueryResult, String> {
    let checked = ensure_readonly_sql(&sql)?;
    let file = Path::new(path.trim());
    if path.trim().is_empty() || !file.is_file() {
        return Err(format!("数据库文件不存在或非文件：{}", path.trim()));
    }
    let started = Instant::now();
    let conn = rusqlite::Connection::open_with_flags(file, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("打开失败（如实报）：{error}"))?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|error| format!("设置忙等超时失败：{error}"))?;

    let mut stmt = conn
        .prepare(checked)
        .map_err(|error| format!("语句准备失败（如实报）：{error}"))?;
    let columns: Vec<String> = stmt
        .column_names()
        .iter()
        .map(|name| name.to_string())
        .collect();

    let mut rows: Vec<Vec<serde_json::Value>> = Vec::new();
    let mut truncated = false;
    {
        let mut fetched = stmt
            .query([])
            .map_err(|error| format!("查询执行失败（如实报）：{error}"))?;
        while let Some(row) = fetched
            .next()
            .map_err(|error| format!("读取结果行失败（如实报）：{error}"))?
        {
            if rows.len() >= SQL_ROW_LIMIT {
                truncated = true;
                break;
            }
            let mut record = Vec::with_capacity(columns.len());
            for index in 0..columns.len() {
                let value = row
                    .get::<_, SqlValue>(index)
                    .map_err(|error| format!("取列值失败（如实报）：{error}"))?;
                record.push(value_to_json(&value));
            }
            rows.push(record);
        }
    }

    Ok(SqlQueryResult {
        columns,
        row_count: rows.len(),
        truncated,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readonly_gate_allows_read_forms() {
        for sql in [
            "SELECT 1",
            "  select * from t",
            "-- lead\nSELECT 1",
            "/* block */ SELECT 1",
            "WITH x AS (SELECT 1) SELECT * FROM x",
            "PRAGMA table_info(t)",
            "EXPLAIN QUERY PLAN SELECT 1",
            "SELECT 1;",
            "SELECT 1; -- trailing comment",
        ] {
            assert!(ensure_readonly_sql(sql).is_ok(), "应放行：{sql}");
        }
    }

    #[test]
    fn readonly_gate_rejects_writes_and_multi() {
        for sql in [
            "",
            "   ",
            "-- only comment",
            "INSERT INTO t VALUES (1)",
            "UPDATE t SET a = 1",
            "DELETE FROM t",
            "DROP TABLE t",
            "CREATE TABLE t (a)",
            "ATTACH DATABASE 'x' AS y",
            "VACUUM",
            "SELECT 1; DROP TABLE t",
            "SELECT 1; SELECT 2",
        ] {
            assert!(ensure_readonly_sql(sql).is_err(), "应拒绝：{sql}");
        }
    }

    #[test]
    fn value_mapping_hides_blob_payload() {
        assert_eq!(value_to_json(&SqlValue::Null), serde_json::Value::Null);
        assert_eq!(value_to_json(&SqlValue::Integer(7)), serde_json::json!(7));
        assert_eq!(
            value_to_json(&SqlValue::Text("x".into())),
            serde_json::json!("x")
        );
        assert_eq!(
            value_to_json(&SqlValue::Blob(vec![1, 2, 3])),
            serde_json::json!("<blob 3 bytes>")
        );
    }

    #[test]
    fn real_file_roundtrip_and_truncation() {
        let dir = std::env::temp_dir().join(format!("starport-db-test-{}", std::process::id()));
        // Workspace policy (clippy.toml): fs_err over std::fs（含测试），错误带路径。
        fs_err::create_dir_all(&dir).unwrap();
        let db_path = dir.join("roundtrip.db");
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute("CREATE TABLE t (a INTEGER, b TEXT)", [])
                .unwrap();
            for i in 0..(SQL_ROW_LIMIT + 5) {
                conn.execute(
                    "INSERT INTO t (a, b) VALUES (?1, ?2)",
                    rusqlite::params![i as i64, format!("row-{i}")],
                )
                .unwrap();
            }
        }

        let result = query_sqlite_file(
            db_path.to_string_lossy().into_owned(),
            "SELECT a, b FROM t ORDER BY a".to_string(),
        )
        .unwrap();
        assert_eq!(result.columns, vec!["a", "b"]);
        assert_eq!(result.row_count, SQL_ROW_LIMIT);
        assert!(result.truncated, "超上限必须如实标注截断");

        let small = query_sqlite_file(
            db_path.to_string_lossy().into_owned(),
            "SELECT COUNT(*) AS n FROM t".to_string(),
        )
        .unwrap();
        assert_eq!(small.rows.len(), 1);
        assert!(!small.truncated);

        assert!(
            query_sqlite_file(
                db_path.to_string_lossy().into_owned(),
                "DELETE FROM t".to_string()
            )
            .is_err(),
            "写类必须拒绝"
        );
        assert!(
            query_sqlite_file("".to_string(), "SELECT 1".to_string()).is_err(),
            "空路径必须拒绝"
        );
        let _ = fs_err::remove_dir_all(&dir);
    }
}
