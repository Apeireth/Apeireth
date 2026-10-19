//! 矿脉数据库面 —— 只读 SQL 查询（批次推进）。
//!
//! 契约（I-starport-builtins.md「矿脉」）：
//!   1. 只读口径双保险 —— 前端 classifySql 与本件 ensure_readonly_sql 各自
//!      独立强制（纵深：一层被绕过另一层仍拒），写类/DDL/多语句一律拒绝；
//!      WITH/EXPLAIN 做保守结构核对（CTE 内嵌 DML、WITH 引导 DML、EXPLAIN
//!      ANALYZE 执行写类均拒），远程面另有只读事务包裹兜底；
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

/// 写类词面（小写；裸词命中即拒）。
///
/// 覆盖 CTE 内嵌 DML（`WITH x AS (UPDATE …)`）、`EXPLAIN ANALYZE` 执行写类、
/// `SELECT … INTO` 建表/落盘、`SELECT … FOR UPDATE` 行锁等写入/副作用面；
/// `into` 同时兜住 LOAD/INSERT/MERGE 的表目标子句。词面按**词元全等**比对
/// （`updates`、`update_time` 等标识符不误伤），字符串字面量/引号标识符不参与。
const WRITE_MARKERS: [&str; 5] = ["insert", "update", "delete", "merge", "into"];

/// EXPLAIN 前导修饰词（跳过层）：SQLite 的 QUERY PLAN、常见 ANALYZE/EXTENDED/
/// PARTITIONS 变体；`FORMAT=…` 形式另有三元跳过。未列修饰保守拒绝。
const EXPLAIN_SKIP_WORDS: [&str; 5] = ["query", "plan", "analyze", "extended", "partitions"];

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

/// 词法词元（保守启发式用；注释已剥）。
#[derive(Debug, PartialEq, Eq)]
enum Token<'a> {
    /// 裸标识符/关键字词面（原文切片，比对时不区分大小写）。
    Word(&'a str),
    /// 不透明引用：字符串字面量 / 引号标识符（内容不参与判断）。
    Quoted,
    /// 单字符标点（含 `(` `)` `,` `.` 等）。
    Punct(char),
}

/// 词法化（保守；拿不准即 Err）：剥注释、字符串字面量与引号标识符收成不透明
/// 词元，其余按裸词/标点切分。字符串只认 `''` 转义——反斜杠转义在不同 SQL 方言
/// 下口径分歧，取「不认」这一侧：本扫描结束位置只会**早于或等于**服务端，
/// 词面只会暴露更多（fail-closed），不会把服务端眼里的代码藏进字面量。
/// 注释/字面量/引号未终结一律 Err。
fn tokenize(text: &str) -> Result<Vec<Token<'_>>, String> {
    let mut tokens = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let head = rest.chars().next().expect("rest 非空必有首字符");
        if head.is_whitespace() {
            rest = &rest[head.len_utf8()..];
        } else if let Some(tail) = rest.strip_prefix("--") {
            rest = match tail.find('\n') {
                Some(nl) => &tail[nl + 1..],
                None => "",
            };
        } else if let Some(tail) = rest.strip_prefix("/*") {
            let end = tail.find("*/").ok_or("块注释未终结（保守拒绝）")?;
            rest = &tail[end + 2..];
        } else if head == '\'' {
            let body = &rest[1..];
            let mut cursor = 0usize;
            loop {
                let quote = body[cursor..]
                    .find('\'')
                    .ok_or("字符串字面量未终结（保守拒绝）")?;
                cursor += quote + 1;
                if body[cursor..].starts_with('\'') {
                    cursor += 1; // '' 转义，继续找闭合
                } else {
                    break;
                }
            }
            tokens.push(Token::Quoted);
            rest = &body[cursor..];
        } else if head == '"' || head == '`' {
            let end = rest[1..].find(head).ok_or("引号标识符未终结（保守拒绝）")?;
            tokens.push(Token::Quoted);
            rest = &rest[1 + end + 1..];
        } else if head.is_alphanumeric() || head == '_' || head == '$' {
            let end = rest
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
                .unwrap_or(rest.len());
            tokens.push(Token::Word(&rest[..end]));
            rest = &rest[end..];
        } else {
            tokens.push(Token::Punct(head));
            rest = &rest[head.len_utf8()..];
        }
    }
    Ok(tokens)
}

fn is_word(token: Option<&Token<'_>>, word: &str) -> bool {
    matches!(token, Some(Token::Word(face)) if face.eq_ignore_ascii_case(word))
}

fn is_punct(token: Option<&Token<'_>>, punct: char) -> bool {
    matches!(token, Some(Token::Punct(mark)) if *mark == punct)
}

/// 从 `open`（必为 `(`）跳过配平括号组，返回闭合 `)` 之后的下标；不配平即 Err。
fn skip_paren_group(tokens: &[Token<'_>], open: usize) -> Result<usize, String> {
    if !is_punct(tokens.get(open), '(') {
        return Err("内部错误：括号组起点非法".to_string());
    }
    let mut depth = 0usize;
    for (offset, token) in tokens[open..].iter().enumerate() {
        match token {
            Token::Punct('(') => depth += 1,
            Token::Punct(')') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(open + offset + 1);
                }
            }
            _ => {}
        }
    }
    Err("括号未配平（保守拒绝）".to_string())
}

/// CTE 主体（`AS ( … )` 括号内）引导词核对：只认 SELECT，或再嵌 WITH（递归，
/// 其主体同样必须收束到 SELECT）。VALUES/TABLE/括号包查询等拿不准形态一律拒。
fn check_cte_body(body: &[Token<'_>]) -> Result<(), String> {
    if is_word(body.first(), "select") {
        Ok(())
    } else if is_word(body.first(), "with") {
        check_with_clause(body)
    } else {
        Err("CTE 主体引导词须为 SELECT（或 WITH…SELECT），保守拒绝".to_string())
    }
}

/// WITH 语句结构核对（保守启发式）：`WITH [RECURSIVE] 名 [(列清单)] AS
/// [[NOT] MATERIALIZED] (体) [, …] 主体`。逐个 CTE 核对形状；CTE 体过
/// [`check_cte_body`]；列表结束后的主体引导词必须是 SELECT；SEARCH/CYCLE 修饰
/// 不做解析、保守拒绝。任何不认识的形状即 Err（fail-closed）。
fn check_with_clause(tokens: &[Token<'_>]) -> Result<(), String> {
    let mut cursor = 1usize; // tokens[0] == with
    if is_word(tokens.get(cursor), "recursive") {
        cursor += 1;
    }
    loop {
        match tokens.get(cursor) {
            Some(Token::Word(_)) | Some(Token::Quoted) => cursor += 1,
            _ => return Err("WITH 结构无法解析（CTE 名缺失），保守拒绝".to_string()),
        }
        if is_punct(tokens.get(cursor), '(') {
            cursor = skip_paren_group(tokens, cursor)?; // 可选列名清单
        }
        if !is_word(tokens.get(cursor), "as") {
            return Err("WITH 结构无法解析（CTE 缺 AS），保守拒绝".to_string());
        }
        cursor += 1;
        if is_word(tokens.get(cursor), "not") {
            cursor += 1;
            if !is_word(tokens.get(cursor), "materialized") {
                return Err("WITH 结构无法解析（NOT 修饰非法），保守拒绝".to_string());
            }
            cursor += 1;
        } else if is_word(tokens.get(cursor), "materialized") {
            cursor += 1;
        }
        if !is_punct(tokens.get(cursor), '(') {
            return Err("WITH 结构无法解析（CTE 缺主体括号），保守拒绝".to_string());
        }
        let body_start = cursor + 1;
        let after_body = skip_paren_group(tokens, cursor)?;
        check_cte_body(&tokens[body_start..after_body - 1])?;
        cursor = after_body;
        if is_word(tokens.get(cursor), "search") || is_word(tokens.get(cursor), "cycle") {
            return Err("CTE 的 SEARCH/CYCLE 修饰不解析（保守拒绝）".to_string());
        }
        if is_punct(tokens.get(cursor), ',') {
            cursor += 1;
            continue;
        }
        if is_word(tokens.get(cursor), "select") {
            return Ok(());
        }
        return Err("WITH 主体引导词须为 SELECT（保守拒绝）".to_string());
    }
}

/// EXPLAIN 尾部核对：跳过括号选项组与已知前导修饰（QUERY PLAN / ANALYZE /
/// EXTENDED / PARTITIONS / FORMAT=…），内层语句引导词必须仍过只读闸
/// （SELECT，或 WITH 递归核对）。未支持变体保守拒绝。
fn check_explain_tail(tokens: &[Token<'_>]) -> Result<(), String> {
    let mut cursor = 0usize;
    loop {
        if is_punct(tokens.get(cursor), '(') {
            cursor = skip_paren_group(tokens, cursor)?;
        } else if tokens.get(cursor).is_some_and(|token| match token {
            Token::Word(word) => EXPLAIN_SKIP_WORDS
                .iter()
                .any(|skip| word.eq_ignore_ascii_case(skip)),
            _ => false,
        }) {
            cursor += 1;
        } else if is_word(tokens.get(cursor), "format") {
            cursor += 1;
            if !is_punct(tokens.get(cursor), '=') {
                return Err("EXPLAIN FORMAT 修饰无法解析（保守拒绝）".to_string());
            }
            cursor += 2; // '=' 与格式值
        } else {
            break;
        }
    }
    if is_word(tokens.get(cursor), "select") {
        Ok(())
    } else if is_word(tokens.get(cursor), "with") {
        check_with_clause(&tokens[cursor..])
    } else {
        Err("EXPLAIN 内层语句非只读（保守拒绝）".to_string())
    }
}

/// 语句分类核对：真实引导关键字（词元级）+ 写类词面全局扫描（含括号内/CTE 体内
/// /EXPLAIN 内层）+ WITH/EXPLAIN 结构核对。
fn check_readonly_tokens(tokens: &[Token<'_>]) -> Result<(), String> {
    let head = tokens
        .first()
        .ok_or_else(|| "SQL 为空".to_string())
        .and_then(|token| match token {
            Token::Word(word) => Ok(*word),
            _ => Err("只读口径：仅放行 SELECT/WITH/PRAGMA/EXPLAIN".to_string()),
        })?;
    for token in tokens {
        if let Token::Word(word) = token {
            if WRITE_MARKERS
                .iter()
                .any(|marker| word.eq_ignore_ascii_case(marker))
            {
                return Err(format!("只读口径：语句含写类词面 {word:?}（保守拒绝）"));
            }
        }
    }
    if head.eq_ignore_ascii_case("select") || head.eq_ignore_ascii_case("pragma") {
        Ok(())
    } else if head.eq_ignore_ascii_case("with") {
        check_with_clause(tokens)
    } else if head.eq_ignore_ascii_case("explain") {
        check_explain_tail(&tokens[1..])
    } else {
        Err("只读口径：仅放行 SELECT/WITH/PRAGMA/EXPLAIN".to_string())
    }
}

/// 只读口径校验（原生侧第二道闸）。多语句/写类/空语句一律拒绝。
///
/// **保守启发式，不是完整 SQL 方言解析器（0 装口径）**：只做词法级扫描
/// （剥 `--` / `/* */` 注释；字符串字面量与引号标识符收成不透明词元，内容不参与
/// 判断）与最小结构核对——真实引导关键字取词元而非字符串前缀（`WITH x AS (…)`
/// 引导 DML、`EXPLAIN ANALYZE <写类>` 等换装写法不再能靠前缀混入）；WITH 引导
/// 时逐个 CTE 核对 `AS (体)` 结构、CTE 体与主体引导词必须是 SELECT（CTE 体内可
/// 再嵌 WITH，递归核对）；写类词面（[`WRITE_MARKERS`]）对**全词元流**（含括号
/// 内）扫描，子查询里藏的 DML CTE 同样命中。拿不准即拒（fail-closed）：括号不
/// 配平、注释/字面量未终结、无法解析的 CTE 结构、未支持的修饰（如 SEARCH/
/// CYCLE）一律 Err，宁可误拒不放行。远程线协议执行面另有只读事务包裹兜底，
/// 本闸是纵深第一层。
pub fn ensure_readonly_sql(sql: &str) -> Result<&str, String> {
    let text = strip_leading_comments(sql);
    if text.is_empty() {
        return Err("SQL 为空".to_string());
    }
    // 多语句拒绝：分号之后仅允许空白/注释（含字面量内分号——宁误拒不放行）。
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
    let tokens = tokenize(text[..head_end].trim())?;
    check_readonly_tokens(&tokens)?;
    Ok(text)
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
            "WITH RECURSIVE x(n) AS (SELECT 1) SELECT * FROM x",
            "PRAGMA table_info(t)",
            "EXPLAIN QUERY PLAN SELECT 1",
            "SELECT 1;",
            "SELECT 1; -- trailing comment",
            "SELECT 'update' FROM t",
            "SELECT update_time FROM t",
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
            "WITH x AS (UPDATE t SET a=1 RETURNING *) SELECT * FROM x",
            "WITH x AS (SELECT 1) DELETE FROM t",
            "WITH x AS (SELECT 1) INSERT INTO t SELECT 1",
            "EXPLAIN ANALYZE UPDATE t SET a=1",
            "SELECT * INTO new_t FROM t",
            "WITH x AS (SELECT 1) SELECT * FROM x FOR UPDATE",
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
