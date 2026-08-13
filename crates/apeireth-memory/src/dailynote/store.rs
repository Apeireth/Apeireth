//! SQLite-backed daily note store.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use thiserror::Error;

use super::note::{DailyNote, NoteId};

#[derive(Debug, Error)]
pub enum DailyNoteError {
    #[error("sqlite: `{0}`")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("note not found: `{0}`")]
    NotFound(String),
}

pub struct DailyNoteStore {
    conn: Connection,
}

impl DailyNoteStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, DailyNoteError> {
        let conn = Connection::open(path)?;
        Self::migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self, DailyNoteError> {
        let conn = Connection::open_in_memory()?;
        Self::migrate(&conn)?;
        Ok(Self { conn })
    }

    fn migrate(conn: &Connection) -> Result<(), DailyNoteError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                date TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_notes_date ON notes(date);
            CREATE TABLE IF NOT EXISTS note_tags (
                note_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (note_id, tag),
                FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_note_tags_tag ON note_tags(tag);",
        )?;
        Ok(())
    }

    pub fn insert(&self, note: &DailyNote) -> Result<(), DailyNoteError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO notes (id, date, title, content, tags, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                note.id.0,
                note.date.to_rfc3339(),
                note.title,
                note.content,
                note.tags.join(","),
                note.created_at.to_rfc3339(),
                note.updated_at.to_rfc3339(),
            ],
        )?;
        // Refresh tag index
        self.conn.execute("DELETE FROM note_tags WHERE note_id = ?1", params![note.id.0])?;
        for tag in &note.tags {
            self.conn.execute(
                "INSERT OR IGNORE INTO note_tags (note_id, tag) VALUES (?1, ?2)",
                params![note.id.0, tag],
            )?;
        }
        Ok(())
    }

    pub fn get(&self, id: &NoteId) -> Result<DailyNote, DailyNoteError> {
        let mut stmt = self.conn.prepare("SELECT id, date, title, content, tags, created_at, updated_at FROM notes WHERE id = ?1")?;
        let note = stmt.query_row(params![id.0], |r| {
            let id: String = r.get(0)?;
            let date: String = r.get(1)?;
            let title: String = r.get(2)?;
            let content: String = r.get(3)?;
            let tags: String = r.get(4)?;
            let created_at: String = r.get(5)?;
            let updated_at: String = r.get(6)?;
            let tags_vec: Vec<String> = if tags.is_empty() { Vec::new() } else { tags.split(',').map(String::from).collect() };
            Ok(DailyNote {
                id: NoteId(id),
                date: DateTime::parse_from_rfc3339(&date).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                title,
                content,
                tags: tags_vec,
                created_at: DateTime::parse_from_rfc3339(&created_at).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&updated_at).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now()),
            })
        })?;
        Ok(note)
    }

    pub fn delete(&self, id: &NoteId) -> Result<(), DailyNoteError> {
        let n = self.conn.execute("DELETE FROM notes WHERE id = ?1", params![id.0])?;
        if n == 0 {
            return Err(DailyNoteError::NotFound(id.0.clone()));
        }
        Ok(())
    }

    pub fn count(&self) -> Result<i64, DailyNoteError> {
        let n: i64 = self.conn.query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))?;
        Ok(n)
    }

    /// List notes with a given tag.
    pub fn list_by_tag(&self, tag: &str) -> Result<Vec<NoteId>, DailyNoteError> {
        let mut stmt = self.conn.prepare("SELECT note_id FROM note_tags WHERE tag = ?1 ORDER BY note_id")?;
        let rows = stmt.query_map(params![tag], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(NoteId(row?));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_and_migrate() {
        let s = DailyNoteStore::open_in_memory().unwrap();
        assert_eq!(s.count().unwrap(), 0);
    }

    #[test]
    fn insert_and_get() {
        let s = DailyNoteStore::open_in_memory().unwrap();
        let n = DailyNote::new("Title", "Body").with_tags(vec!["a".into(), "b".into()]);
        s.insert(&n).unwrap();
        let got = s.get(&n.id).unwrap();
        assert_eq!(got.title, "Title");
        assert_eq!(got.tags, vec!["a", "b"]);
    }

    #[test]
    fn delete_existing() {
        let s = DailyNoteStore::open_in_memory().unwrap();
        let n = DailyNote::new("T", "C");
        s.insert(&n).unwrap();
        s.delete(&n.id).unwrap();
        assert_eq!(s.count().unwrap(), 0);
    }

    #[test]
    fn delete_missing_errors() {
        let s = DailyNoteStore::open_in_memory().unwrap();
        let r = s.delete(&NoteId::new());
        assert!(matches!(r, Err(DailyNoteError::NotFound(_))));
    }

    #[test]
    fn list_by_tag() {
        let s = DailyNoteStore::open_in_memory().unwrap();
        let n1 = DailyNote::new("A", "1").with_tags(vec!["work".into()]);
        let n2 = DailyNote::new("B", "2").with_tags(vec!["work".into(), "idea".into()]);
        s.insert(&n1).unwrap();
        s.insert(&n2).unwrap();
        let work_notes = s.list_by_tag("work").unwrap();
        assert_eq!(work_notes.len(), 2);
        let idea_notes = s.list_by_tag("idea").unwrap();
        assert_eq!(idea_notes.len(), 1);
    }
}