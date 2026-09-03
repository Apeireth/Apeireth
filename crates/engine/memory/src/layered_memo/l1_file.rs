//! L1: File persistence layer (rusqlite-backed).

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use rusqlite::{params, Connection};
use std::path::Path;
use thiserror::Error;

use chrono::{DateTime, Utc};

#[derive(Debug, Error)]
pub enum L1Error {
    #[error("sqlite: `{0}`")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub id: String,
    pub path: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

pub struct L1FileStore {
    conn: Connection,
}

impl L1FileStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, L1Error> {
        let conn = Connection::open(path)?;
        Self::migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self, L1Error> {
        let conn = Connection::open_in_memory()?;
        Self::migrate(&conn)?;
        Ok(Self { conn })
    }

    fn migrate(conn: &Connection) -> Result<(), L1Error> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    pub fn insert(&self, entry: &FileEntry) -> Result<(), L1Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO files (id, path, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                entry.id,
                entry.path,
                entry.content,
                entry.created_at.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<FileEntry, L1Error> {
        let entry = self.conn.query_row(
            "SELECT id, path, content, created_at FROM files WHERE id = ?1",
            params![id],
            |r| {
                let id: String = r.get(0)?;
                let path: String = r.get(1)?;
                let content: String = r.get(2)?;
                let created_at: String = r.get(3)?;
                Ok(FileEntry {
                    id,
                    path,
                    content,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            },
        )?;
        Ok(entry)
    }

    pub fn count(&self) -> Result<i64, L1Error> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_and_count() {
        let s = L1FileStore::open_in_memory().unwrap();
        assert_eq!(s.count().unwrap(), 0);
    }

    #[test]
    fn insert_and_get() {
        let s = L1FileStore::open_in_memory().unwrap();
        let entry = FileEntry {
            id: "f1".to_string(),
            path: "/tmp/test.md".to_string(),
            content: "hello world".to_string(),
            created_at: Utc::now(),
        };
        s.insert(&entry).unwrap();
        let got = s.get("f1").unwrap();
        assert_eq!(got.content, "hello world");
    }

    #[test]
    fn get_missing_returns_error() {
        let s = L1FileStore::open_in_memory().unwrap();
        let r = s.get("nonexistent");
        assert!(r.is_err());
    }
}
