//! Persistent code index (rusqlite schema).
//!
//! Schema:
//! - files (path, hash, indexed_at)
//! - symbols (file_id, name, kind, line, column, signature)
//! - imports (file_id, target)
//!
//! Honest scope (per O-5 不假装):
//! - Schema migration on first run (idempotent CREATE)
//! - CRUD operations for files / symbols / imports
//! - FTS5 virtual table NOT enabled (would need rusqlite feature flag);
//!   full-text search uses in-memory Aho-Corasick on indexed files instead.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use rusqlite::{params, Connection};
use std::path::Path;
use thiserror::Error;

use crate::symbols::Symbol;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("sqlite: `{0}`")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub id: i64,
    pub path: String,
}

pub struct CodeIndex {
    conn: Connection,
}

impl CodeIndex {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, IndexError> {
        let conn = Connection::open(path)?;
        Self::migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self, IndexError> {
        let conn = Connection::open_in_memory()?;
        Self::migrate(&conn)?;
        Ok(Self { conn })
    }

    fn migrate(conn: &Connection) -> Result<(), IndexError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                indexed_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS symbols (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                line INTEGER NOT NULL,
                column INTEGER NOT NULL,
                signature TEXT,
                FOREIGN KEY(file_id) REFERENCES files(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS imports (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id INTEGER NOT NULL,
                target TEXT NOT NULL,
                FOREIGN KEY(file_id) REFERENCES files(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind);
            CREATE INDEX IF NOT EXISTS idx_imports_target ON imports(target);",
        )?;
        Ok(())
    }

    pub fn upsert_file(&self, path: &str) -> Result<i64, IndexError> {
        let now = chrono::Utc::now().to_rfc3339();
        // Try insert; if exists, update timestamp; then read id.
        self.conn.execute(
            "INSERT OR IGNORE INTO files (path, indexed_at) VALUES (?1, ?2)",
            params![path, now],
        )?;
        self.conn.execute(
            "UPDATE files SET indexed_at = ?1 WHERE path = ?2",
            params![now, path],
        )?;
        let id: i64 =
            self.conn
                .query_row("SELECT id FROM files WHERE path = ?1", params![path], |r| {
                    r.get(0)
                })?;
        Ok(id)
    }

    pub fn insert_symbol(&self, file_id: i64, sym: &Symbol) -> Result<(), IndexError> {
        self.conn.execute(
            "INSERT INTO symbols (file_id, name, kind, line, column, signature)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file_id,
                sym.name,
                sym.kind.as_str(),
                sym.line as i64,
                sym.column as i64,
                sym.signature,
            ],
        )?;
        Ok(())
    }

    pub fn insert_import(&self, file_id: i64, target: &str) -> Result<(), IndexError> {
        self.conn.execute(
            "INSERT INTO imports (file_id, target) VALUES (?1, ?2)",
            params![file_id, target],
        )?;
        Ok(())
    }

    pub fn symbol_count(&self) -> Result<i64, IndexError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))?;
        Ok(n)
    }

    pub fn file_count(&self) -> Result<i64, IndexError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        Ok(n)
    }

    /// Lookup symbols by name (case-sensitive exact match).
    pub fn lookup_symbols_by_name(&self, name: &str) -> Result<Vec<IndexEntry>, IndexError> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.path FROM files f
             JOIN symbols s ON s.file_id = f.id
             WHERE s.name = ?1",
        )?;
        let rows = stmt.query_map(params![name], |r| {
            Ok(IndexEntry {
                id: r.get(0)?,
                path: r.get(1)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_and_migrate() {
        let idx = CodeIndex::open_in_memory().unwrap();
        assert_eq!(idx.file_count().unwrap(), 0);
        assert_eq!(idx.symbol_count().unwrap(), 0);
    }

    #[test]
    fn upsert_file_idempotent() {
        let idx = CodeIndex::open_in_memory().unwrap();
        let id1 = idx.upsert_file("src/main.rs").unwrap();
        let id2 = idx.upsert_file("src/main.rs").unwrap();
        assert_eq!(id1, id2, "upsert should return same id");
        assert_eq!(idx.file_count().unwrap(), 1);
    }

    #[test]
    fn insert_symbol_and_lookup() {
        let idx = CodeIndex::open_in_memory().unwrap();
        let file_id = idx.upsert_file("src/lib.rs").unwrap();
        let sym = Symbol {
            name: "hello".to_string(),
            kind: crate::symbols::SymbolKind::Function,
            line: 1,
            column: 1,
            language: "rust".to_string(),
            signature: "fn hello() {}".to_string(),
        };
        idx.insert_symbol(file_id, &sym).unwrap();
        let found = idx.lookup_symbols_by_name("hello").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, "src/lib.rs");
    }

    #[test]
    fn import_insert() {
        let idx = CodeIndex::open_in_memory().unwrap();
        let file_id = idx.upsert_file("src/main.rs").unwrap();
        idx.insert_import(file_id, "crate::lib").unwrap();
        let _ = idx.symbol_count().unwrap();
    }

    #[test]
    fn lookup_nonexistent_returns_empty() {
        let idx = CodeIndex::open_in_memory().unwrap();
        let r = idx.lookup_symbols_by_name("nothing").unwrap();
        assert!(r.is_empty());
    }
}
