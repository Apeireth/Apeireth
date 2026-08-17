//! SQLite SymbolStore — 单表 `symbols` (per task 验收).
//!
//! 字段: symbol (PK) / name / sector / industry / exchange / country / currency /
//!       market_cap (REAL NULL) / ipo_year (INTEGER NULL) / provenance / last_updated_ms
//! 索引: sector / industry / exchange 各 1 索引 (per task #4 加速过滤).

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;
use tracing::warn;

use crate::catalog::SymbolCatalog;
use crate::symbol::{Provenance, SymbolMeta};

#[derive(Debug, Error)]
pub enum SymbolStoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("csv: {0}")]
    Csv(String),
    #[error("lock poisoned")]
    LockPoisoned,
}

/// SQLite 标的存储 (单连接 + Mutex 串行化, 与 SqliteMemoryStore 一致风格).
#[derive(Clone)]
pub struct SymbolStore {
    inner: Arc<Mutex<Connection>>,
}

impl SymbolStore {
    /// 打开/创建数据库文件.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, SymbolStoreError> {
        let conn = Connection::open(path)?;
        let s = Self { inner: Arc::new(Mutex::new(conn)) };
        s.migrate()?;
        Ok(s)
    }

    /// 内存数据库 (测试用).
    pub fn open_in_memory() -> Result<Self, SymbolStoreError> {
        let conn = Connection::open_in_memory()?;
        let s = Self { inner: Arc::new(Mutex::new(conn)) };
        s.migrate()?;
        Ok(s)
    }

    /// V5 migration: 建 symbols 表 + 3 索引.
    fn migrate(&self) -> Result<(), SymbolStoreError> {
        let conn = self.conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS symbols (
                symbol TEXT NOT NULL PRIMARY KEY,
                name TEXT NOT NULL DEFAULT '',
                sector TEXT NOT NULL DEFAULT '',
                industry TEXT NOT NULL DEFAULT '',
                exchange TEXT NOT NULL DEFAULT '',
                country TEXT NOT NULL DEFAULT '',
                currency TEXT NOT NULL DEFAULT '',
                market_cap REAL,
                ipo_year INTEGER,
                provenance TEXT NOT NULL DEFAULT 'manual',
                last_updated_ms INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_symbols_sector ON symbols(sector);
            CREATE INDEX IF NOT EXISTS idx_symbols_industry ON symbols(industry);
            CREATE INDEX IF NOT EXISTS idx_symbols_exchange ON symbols(exchange);",
        )?;
        Ok(())
    }

    fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, SymbolStoreError> {
        self.inner.lock().map_err(|_| SymbolStoreError::LockPoisoned)
    }

    /// 单条 upsert (INSERT OR REPLACE).
    pub fn upsert(&self, m: &SymbolMeta) -> Result<(), SymbolStoreError> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO symbols
             (symbol, name, sector, industry, exchange, country, currency,
              market_cap, ipo_year, provenance, last_updated_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                m.symbol,
                m.name,
                m.sector,
                m.industry,
                m.exchange,
                m.country,
                m.currency,
                m.market_cap,
                m.ipo_year,
                m.provenance.as_str(),
                m.last_updated_ms,
            ],
        )?;
        Ok(())
    }

    /// 批量 upsert (单事务包裹, N=1000 批量, per csv.rs 调用约定).
    pub fn insert_batch(&self, batch: &[SymbolMeta]) -> Result<(), SymbolStoreError> {
        if batch.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO symbols
                 (symbol, name, sector, industry, exchange, country, currency,
                  market_cap, ipo_year, provenance, last_updated_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for m in batch {
                stmt.execute(params![
                    m.symbol,
                    m.name,
                    m.sector,
                    m.industry,
                    m.exchange,
                    m.country,
                    m.currency,
                    m.market_cap,
                    m.ipo_year,
                    m.provenance.as_str(),
                    m.last_updated_ms,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// 按主键查询.
    pub fn get(&self, symbol: &str) -> Option<SymbolMeta> {
        let conn = self.conn().ok()?;
        conn.query_row(
            "SELECT symbol, name, sector, industry, exchange, country, currency,
                    market_cap, ipo_year, provenance, last_updated_ms
             FROM symbols WHERE symbol = ?1",
            params![symbol],
            row_to_meta,
        )
        .optional()
        .ok()
        .flatten()
    }

    /// 标的总数.
    pub fn count(&self) -> usize {
        let conn = match self.conn() {
            Ok(c) => c,
            Err(_) => return 0,
        };
        conn.query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get::<_, i64>(0))
            .map(|n| n as usize)
            .unwrap_or_else(|e| {
                warn!("count 失败: {}", e);
                0
            })
    }

    /// 多字段过滤 (任一过滤条件为 None 即不参与, limit 截断).
    ///
    /// 排序: 按 market_cap DESC 优先 (有市值标的优先), 然后 symbol ASC 稳定排序.
    pub fn search(
        &self,
        sector: Option<&str>,
        industry: Option<&str>,
        exchange: Option<&str>,
        limit: usize,
    ) -> Vec<SymbolMeta> {
        // 动态拼 SQL (3 个字段可选), 用 enum 表查询参数绑定
        let mut sql = String::from(
            "SELECT symbol, name, sector, industry, exchange, country, currency,
                    market_cap, ipo_year, provenance, last_updated_ms
             FROM symbols WHERE 1=1",
        );
        let mut binds: Vec<String> = Vec::new();
        if let Some(s) = sector {
            sql.push_str(" AND sector = ?");
            sql.push_str(&(binds.len() + 1).to_string());
            binds.push(s.to_string());
        }
        if let Some(s) = industry {
            sql.push_str(" AND industry = ?");
            sql.push_str(&(binds.len() + 1).to_string());
            binds.push(s.to_string());
        }
        if let Some(s) = exchange {
            sql.push_str(" AND exchange = ?");
            sql.push_str(&(binds.len() + 1).to_string());
            binds.push(s.to_string());
        }
        sql.push_str(" ORDER BY market_cap DESC NULLS LAST, symbol ASC LIMIT ?");
        sql.push_str(&(binds.len() + 1).to_string());
        binds.push(limit.to_string());

        let conn = match self.conn() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let params_dyn: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(params_dyn.as_slice(), row_to_meta).ok();
        match rows {
            Some(it) => it.filter_map(|r| r.ok()).collect(),
            None => Vec::new(),
        }
    }

    /// 删除单条 (供测试 + 对账用).
    pub fn delete(&self, symbol: &str) -> Result<(), SymbolStoreError> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM symbols WHERE symbol = ?1", params![symbol])?;
        Ok(())
    }
}

impl SymbolCatalog for SymbolStore {
    fn get(&self, symbol: &str) -> Option<SymbolMeta> {
        SymbolStore::get(self, symbol)
    }

    fn search(
        &self,
        sector: Option<&str>,
        industry: Option<&str>,
        exchange: Option<&str>,
        limit: usize,
    ) -> Vec<SymbolMeta> {
        SymbolStore::search(self, sector, industry, exchange, limit)
    }

    fn count(&self) -> usize {
        SymbolStore::count(self)
    }
}

fn row_to_meta(row: &rusqlite::Row<'_>) -> rusqlite::Result<SymbolMeta> {
    let prov_s: String = row.get(9)?;
    Ok(SymbolMeta {
        symbol: row.get(0)?,
        name: row.get(1)?,
        sector: row.get(2)?,
        industry: row.get(3)?,
        exchange: row.get(4)?,
        country: row.get(5)?,
        currency: row.get(6)?,
        market_cap: row.get(7)?,
        ipo_year: row.get(8)?,
        provenance: Provenance::from_db(&prov_s),
        last_updated_ms: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SymbolStore {
        SymbolStore::open_in_memory().unwrap()
    }

    fn sample(symbol: &str, sector: &str, market_cap: Option<f64>) -> SymbolMeta {
        SymbolMeta {
            symbol: symbol.into(),
            name: format!("{symbol} Inc."),
            sector: sector.into(),
            industry: "Test".into(),
            exchange: "NYSE".into(),
            country: "US".into(),
            currency: "USD".into(),
            market_cap,
            ipo_year: Some(2000),
            provenance: Provenance::FinanceDatabase,
            last_updated_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn upsert_and_get() {
        let s = store();
        let m = sample("AAPL", "Technology", Some(2.9e12));
        s.upsert(&m).unwrap();
        let back = s.get("AAPL").unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn upsert_overwrite() {
        let s = store();
        let m1 = sample("AAPL", "Technology", Some(1.0));
        s.upsert(&m1).unwrap();
        let m2 = sample("AAPL", "Tech-Updated", Some(2.0));
        s.upsert(&m2).unwrap();
        let back = s.get("AAPL").unwrap();
        assert_eq!(back.sector, "Tech-Updated");
        assert_eq!(back.market_cap, Some(2.0));
    }

    #[test]
    fn insert_batch_in_single_transaction() {
        let s = store();
        let batch: Vec<SymbolMeta> = (0..100)
            .map(|i| sample(&format!("SYM{:03}", i), "Tech", Some(i as f64)))
            .collect();
        s.insert_batch(&batch).unwrap();
        assert_eq!(s.count(), 100);
    }

    #[test]
    fn insert_batch_empty_noop() {
        let s = store();
        s.insert_batch(&[]).unwrap();
        assert_eq!(s.count(), 0);
    }

    #[test]
    fn get_missing_returns_none() {
        let s = store();
        assert!(s.get("NONEXIST").is_none());
    }

    #[test]
    fn count_after_inserts() {
        let s = store();
        for i in 0..5 {
            s.upsert(&sample(&format!("S{}", i), "Tech", None)).unwrap();
        }
        assert_eq!(s.count(), 5);
    }

    #[test]
    fn search_filters_sector() {
        let s = store();
        s.upsert(&sample("A", "Tech", Some(1.0))).unwrap();
        s.upsert(&sample("B", "Finance", Some(2.0))).unwrap();
        s.upsert(&sample("C", "Tech", Some(3.0))).unwrap();
        let r = s.search(Some("Tech"), None, None, 10);
        assert_eq!(r.len(), 2);
        assert!(r.iter().any(|m| m.symbol == "A"));
        assert!(r.iter().any(|m| m.symbol == "C"));
    }

    #[test]
    fn search_filters_industry() {
        let s = store();
        s.upsert(&sample("A", "Tech", Some(1.0))).unwrap();
        s.upsert(&sample("B", "Tech", Some(2.0))).unwrap();
        // 改 industry
        let mut m = s.get("B").unwrap();
        m.industry = "Hardware".into();
        s.upsert(&m).unwrap();
        let r = s.search(None, Some("Hardware"), None, 10);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].symbol, "B");
    }

    #[test]
    fn search_filters_exchange() {
        let s = store();
        s.upsert(&sample("A", "Tech", None)).unwrap();
        let mut m = s.get("A").unwrap();
        m.exchange = "NASDAQ".into();
        s.upsert(&m).unwrap();
        let r = s.search(None, None, Some("NASDAQ"), 10);
        assert_eq!(r.len(), 1);
        let r_none = s.search(None, None, Some("NYSE"), 10);
        assert_eq!(r_none.len(), 0);
    }

    #[test]
    fn search_limit_truncates() {
        let s = store();
        for i in 0..20 {
            s.upsert(&sample(&format!("S{:02}", i), "Tech", Some(i as f64))).unwrap();
        }
        let r = s.search(None, None, None, 5);
        assert_eq!(r.len(), 5);
    }

    #[test]
    fn search_orders_by_market_cap_desc() {
        let s = store();
        s.upsert(&sample("LOW", "Tech", Some(1.0))).unwrap();
        s.upsert(&sample("HIGH", "Tech", Some(100.0))).unwrap();
        s.upsert(&sample("MID", "Tech", Some(50.0))).unwrap();
        let r = s.search(None, None, None, 10);
        assert_eq!(r[0].symbol, "HIGH");
        assert_eq!(r[1].symbol, "MID");
        assert_eq!(r[2].symbol, "LOW");
    }

    #[test]
    fn search_handles_null_market_cap() {
        let s = store();
        s.upsert(&sample("NULL_MC", "Tech", None)).unwrap();
        s.upsert(&sample("HAS_MC", "Tech", Some(10.0))).unwrap();
        let r = s.search(None, None, None, 10);
        assert_eq!(r.len(), 2);
        // HAS_MC 应排在 NULL_MC 前 (NULLS LAST)
        assert_eq!(r[0].symbol, "HAS_MC");
        assert_eq!(r[1].symbol, "NULL_MC");
    }

    #[test]
    fn delete_removes_row() {
        let s = store();
        s.upsert(&sample("A", "Tech", None)).unwrap();
        assert!(s.get("A").is_some());
        s.delete("A").unwrap();
        assert!(s.get("A").is_none());
    }

    #[test]
    fn open_creates_table_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.db");
        let s = SymbolStore::open(&p).unwrap();
        s.upsert(&sample("A", "Tech", None)).unwrap();
        drop(s);
        // 重新打开, 数据持久化
        let s2 = SymbolStore::open(&p).unwrap();
        assert!(s2.get("A").is_some());
    }

    #[test]
    fn migration_idempotent() {
        // 二次 open 不报错 (V5 IF NOT EXISTS)
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("test.db");
        let _ = SymbolStore::open(&p).unwrap();
        let _ = SymbolStore::open(&p).unwrap();
    }

    #[test]
    fn row_to_meta_provenance_serialization() {
        let s = store();
        let mut m = sample("A", "Tech", None);
        m.provenance = Provenance::FinanceDatabase;
        s.upsert(&m).unwrap();
        let back = s.get("A").unwrap();
        assert_eq!(back.provenance, Provenance::FinanceDatabase);
    }

    #[test]
    fn row_to_meta_unknown_provenance_falls_back_manual() {
        // 模拟老数据或 DB corruption: 手动注入非法 provenance
        let s = store();
        let conn = s.conn().unwrap();
        conn.execute(
            "INSERT INTO symbols (symbol, name, provenance, last_updated_ms)
             VALUES ('X', 'X Co', 'unknown_value', 0)",
            [],
        ).unwrap();
        drop(conn);
        let back = s.get("X").unwrap();
        assert_eq!(back.provenance, Provenance::Manual);
    }
}