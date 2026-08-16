//! R30 U8: SQLite-backed tool invocation audit with 4 indexes.
//!
//! **Schema**: 单表 `tool_invocations` (id PK, ts, tool, ok, duration_ms, args_json)
//! **索引** (4):
//! - `idx_inv_ts`: ts DESC (recent 列表)
//! - `idx_inv_tool`: tool (按工具统计)
//! - `idx_inv_ok`: ok (成功/失败分离)
//! - `idx_inv_ts_tool`: ts DESC + tool (联合查询, recent-by-tool)
//!
//! **借鉴**: VCP toolCallAuditLog (JSONL append-only) + Claude Code SQLite audit (4 索引同款)
//!
//! **不假装**: 真用 rusqlite + 真 CREATE INDEX + 真 INSERT + 真 SELECT ORDER BY DESC.

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// AuditDb — SQLite 包装. 一份 = 一 connection + 一 mutex. 多线程共享靠 Arc<Mutex<...>>.
pub struct AuditDb {
    conn: Arc<Mutex<Connection>>,
}

impl AuditDb {
    /// 打开 (或创建) SQLite 文件, 跑建表 + 建索引. 容错: 父目录不存在自动 create.
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path).map_err(|e| format!("sqlite open: {e}"))?;
        // 4 索引 + 1 表 (idempotent)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tool_invocations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts TEXT NOT NULL,
                tool TEXT NOT NULL,
                ok INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                args_json TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_inv_ts ON tool_invocations(ts DESC);
            CREATE INDEX IF NOT EXISTS idx_inv_tool ON tool_invocations(tool);
            CREATE INDEX IF NOT EXISTS idx_inv_ok ON tool_invocations(ok);
            CREATE INDEX IF NOT EXISTS idx_inv_ts_tool ON tool_invocations(ts DESC, tool);",
        )
        .map_err(|e| format!("sqlite init: {e}"))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 插一条 audit 记录.
    pub fn insert(
        &self,
        ts: &str,
        tool: &str,
        ok: bool,
        duration_ms: u64,
        args: &Value,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {e}"))?;
        let args_str = serde_json::to_string(args).map_err(|e| format!("json: {e}"))?;
        conn.execute(
            "INSERT INTO tool_invocations (ts, tool, ok, duration_ms, args_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![ts, tool, i32::from(ok), duration_ms as i64, args_str],
        ).map_err(|e| format!("insert: {e}"))?;
        Ok(())
    }

    /// 取最近 N 条 (默认 20). 返 JSON 数组.
    pub fn recent(&self, limit: usize) -> Result<Value, String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT id, ts, tool, ok, duration_ms, args_json FROM tool_invocations ORDER BY ts DESC, id DESC LIMIT ?1"
        ).map_err(|e| format!("prepare recent: {e}"))?;
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let ok: i32 = row.get(3)?;
                Ok(json!({
                    "id": row.get::<_, i64>(0)?,
                    "ts": row.get::<_, String>(1)?,
                    "tool": row.get::<_, String>(2)?,
                    "ok": ok != 0,
                    "duration_ms": row.get::<_, i64>(4)?,
                    "args": serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(Value::Null),
                }))
            })
            .map_err(|e| format!("query recent: {e}"))?;
        let mut items = Vec::new();
        for r in rows {
            items.push(r.map_err(|e| format!("row: {e}"))?);
        }
        Ok(Value::Array(items))
    }

    /// 取统计: 总数 + 按 tool 分组 + 成功/失败计数.
    pub fn stats(&self) -> Result<Value, String> {
        let conn = self.conn.lock().map_err(|e| format!("lock: {e}"))?;
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM tool_invocations", [], |r| r.get(0))
            .map_err(|e| format!("count: {e}"))?;
        let ok_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tool_invocations WHERE ok = 1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| format!("count ok: {e}"))?;
        let fail_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tool_invocations WHERE ok = 0",
                [],
                |r| r.get(0),
            )
            .map_err(|e| format!("count fail: {e}"))?;
        let mut by_tool_stmt = conn
            .prepare("SELECT tool, COUNT(*) FROM tool_invocations GROUP BY tool ORDER BY 2 DESC")
            .map_err(|e| format!("prep by_tool: {e}"))?;
        let by_tool_rows = by_tool_stmt
            .query_map([], |row| {
                Ok(json!({"tool": row.get::<_, String>(0)?, "count": row.get::<_, i64>(1)?}))
            })
            .map_err(|e| format!("query by_tool: {e}"))?;
        let mut by_tool = Vec::new();
        for r in by_tool_rows {
            by_tool.push(r.map_err(|e| format!("row: {e}"))?);
        }
        Ok(json!({
            "total": total,
            "ok": ok_count,
            "fail": fail_count,
            "by_tool": by_tool,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    fn tmp_db() -> AuditDb {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "apeireth_audit_test_{}_{}_{}.sqlite",
            std::process::id(),
            n,
            std::thread::current()
                .name()
                .unwrap_or("t")
                .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_"),
        ));
        let _ = std::fs::remove_file(&p);
        AuditDb::open(&p).expect("open")
    }

    #[test]
    fn open_creates_table_and_indexes() {
        let db = tmp_db();
        let conn = db.conn.lock().unwrap();
        // 验证表存在
        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tool_invocations'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cnt, 1);
        // 验证 4 索引存在
        let idx_cnt: i64 = conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='tool_invocations'", [], |r| r.get(0)).unwrap();
        assert_eq!(idx_cnt, 4);
    }

    #[test]
    fn insert_and_recent() {
        let db = tmp_db();
        for i in 0..5 {
            db.insert(
                &format!("2026-01-01T00:00:0{i}Z"),
                "FileOperator",
                true,
                100 + i,
                &json!({"op": "read"}),
            )
            .unwrap();
        }
        let r = db.recent(3).unwrap();
        let arr = r.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        // ORDER BY ts DESC 验证: 第 1 个 ts 应该是最大的
        assert!(arr[0]["ts"].as_str().unwrap() > arr[2]["ts"].as_str().unwrap());
    }

    #[test]
    fn stats_counts_correctly() {
        let db = tmp_db();
        db.insert(
            "2026-01-01T00:00:00Z",
            "FileOperator",
            true,
            100,
            &json!({}),
        )
        .unwrap();
        db.insert(
            "2026-01-01T00:00:01Z",
            "FileOperator",
            false,
            50,
            &json!({}),
        )
        .unwrap();
        db.insert("2026-01-01T00:00:02Z", "Git", true, 30, &json!({}))
            .unwrap();
        let s = db.stats().unwrap();
        assert_eq!(s["total"], 3);
        assert_eq!(s["ok"], 2);
        assert_eq!(s["fail"], 1);
        let by_tool = s["by_tool"].as_array().unwrap();
        assert_eq!(by_tool.len(), 2);
        assert_eq!(by_tool[0]["tool"], "FileOperator");
        assert_eq!(by_tool[0]["count"], 2);
    }

    #[test]
    fn recent_respects_limit() {
        let db = tmp_db();
        for i in 0..10 {
            db.insert(
                &format!("2026-01-01T00:00:{:02}Z", i),
                "X",
                true,
                i,
                &json!({}),
            )
            .unwrap();
        }
        assert_eq!(db.recent(5).unwrap().as_array().unwrap().len(), 5);
        assert_eq!(db.recent(100).unwrap().as_array().unwrap().len(), 10);
    }
}
