//! Persistent task store (SQLite-backed).
//!
//! Stores TaskRecord rows so TaskIds survive daemon restarts. Complements
//! `apeireth-tools::long_task::TaskManager` (which is in-memory).
//!
//! **Honest** (per O-5 不假装):
//! - Uses rusqlite with bundled sqlite (workspace dep).
//! - Real schema migration on first run; idempotent.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex; // N17: rusqlite::Connection 是 Send + !Sync, 装进 Mutex 让 PersistentTaskStore: Sync (§10 铁边界 ② Tool: Send+Sync)
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PersistError {
    #[error("sqlite: `{0}`")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct TaskRecord {
    pub task_id: String,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

pub struct PersistentTaskStore {
    conn: Mutex<Connection>,
}

impl PersistentTaskStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, PersistError> {
        let conn = Connection::open(path)?;
        // Schema (idempotent CREATE)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                task_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                duration_ms INTEGER,
                error TEXT
            );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn open_in_memory() -> Result<Self, PersistError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                task_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                duration_ms INTEGER,
                error TEXT
            );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert(&self, name: &str) -> Result<TaskRecord, PersistError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let conn = self
            .conn
            .lock()
            .expect("PersistentTaskStore mutex poisoned");
        conn.execute(
            "INSERT INTO tasks (task_id, name, status, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, name, "Running", now.to_rfc3339()],
        )?;
        Ok(TaskRecord {
            task_id: id,
            name: name.to_string(),
            status: "Running".to_string(),
            created_at: now,
            duration_ms: None,
            error: None,
        })
    }

    pub fn complete(&self, task_id: &str, duration: Duration) -> Result<(), PersistError> {
        let conn = self
            .conn
            .lock()
            .expect("PersistentTaskStore mutex poisoned");
        conn.execute(
            "UPDATE tasks SET status = ?1, duration_ms = ?2 WHERE task_id = ?3",
            params!["Completed", duration.as_millis() as u64, task_id],
        )?;
        Ok(())
    }

    pub fn fail(&self, task_id: &str, error: &str) -> Result<(), PersistError> {
        let conn = self
            .conn
            .lock()
            .expect("PersistentTaskStore mutex poisoned");
        conn.execute(
            "UPDATE tasks SET status = ?1, error = ?2 WHERE task_id = ?3",
            params!["Failed", error, task_id],
        )?;
        Ok(())
    }

    pub fn count(&self) -> Result<i64, PersistError> {
        let conn = self
            .conn
            .lock()
            .expect("PersistentTaskStore mutex poisoned");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn insert_and_count() {
        let store = PersistentTaskStore::open_in_memory().unwrap();
        assert_eq!(store.count().unwrap(), 0);
        let rec = store.insert("test_task").unwrap();
        assert_eq!(store.count().unwrap(), 1);
        assert_eq!(rec.status, "Running");
        store
            .complete(&rec.task_id, Duration::from_millis(123))
            .unwrap();
    }

    #[test]
    fn fail_record() {
        let store = PersistentTaskStore::open_in_memory().unwrap();
        let rec = store.insert("failing").unwrap();
        store.fail(&rec.task_id, "boom").unwrap();
    }
}
