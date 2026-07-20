//! SQLite Note adapter

use async_trait::async_trait;
use apeireth_core::Note;
use apeireth_ports::{NoteRepository, PortError};
use rusqlite::{Connection, params};

pub struct SqliteNoteRepository {
    conn: std::sync::Arc<std::sync::Mutex<Connection>>,
}

impl SqliteNoteRepository {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, PortError> {
        let conn = Connection::open(path).map_err(|e| PortError::Io(e.to_string()))?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS notes (
                nid TEXT PRIMARY KEY,
                topic TEXT NOT NULL,
                claim TEXT NOT NULL,
                evidence TEXT DEFAULT '[]',
                confidence REAL DEFAULT 0.5,
                importance INTEGER DEFAULT 5,
                created_at TEXT NOT NULL,
                last_consolidated TEXT NOT NULL,
                supersedes TEXT DEFAULT '[]',
                tier TEXT DEFAULT 'stm',
                salience REAL DEFAULT 1.0
            );
            CREATE INDEX IF NOT EXISTS idx_notes_tier ON notes(tier);
        ").map_err(|e| PortError::Backend(e.to_string()))?;
        Ok(Self { conn: std::sync::Arc::new(std::sync::Mutex::new(conn)) })
    }
}

#[async_trait]
impl NoteRepository for SqliteNoteRepository {
    async fn upsert(&self, note: &Note) -> Result<bool, PortError> {
        let conn = self.conn.lock().map_err(|e| PortError::Backend(e.to_string()))?;
        let evidence = serde_json::to_string(&note.evidence).map_err(|e| PortError::Serde(e.to_string()))?;
        let supersedes = serde_json::to_string(&note.supersedes).map_err(|e| PortError::Serde(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO notes(nid, topic, claim, evidence, confidence, importance, created_at, last_consolidated, supersedes, tier, salience) VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            params![
                note.nid,
                note.topic,
                note.claim,
                evidence,
                note.confidence,
                note.importance,
                note.created_at.to_rfc3339(),
                note.last_consolidated.to_rfc3339(),
                supersedes,
                note.tier,
                note.salience,
            ],
        ).map_err(|e| PortError::Backend(e.to_string()))?;
        Ok(true)
    }

    async fn get(&self, nid: &str) -> Result<Option<Note>, PortError> {
        let conn = self.conn.lock().map_err(|e| PortError::Backend(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT nid, topic, claim, evidence, confidence, importance, created_at, last_consolidated, supersedes, tier, salience FROM notes WHERE nid = ?"
        ).map_err(|e| PortError::Backend(e.to_string()))?;
        let mut rows = stmt.query(params![nid]).map_err(|e| PortError::Backend(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| PortError::Backend(e.to_string()))? {
            let evidence: String = row.get(3).map_err(|e| PortError::Backend(e.to_string()))?;
            let supersedes: String = row.get(8).map_err(|e| PortError::Backend(e.to_string()))?;
            let created_str: String = row.get(6).map_err(|e| PortError::Backend(e.to_string()))?;
            let consolidated_str: String = row.get(7).map_err(|e| PortError::Backend(e.to_string()))?;
            Ok(Some(Note {
                nid: row.get(0).map_err(|e| PortError::Backend(e.to_string()))?,
                topic: row.get(1).map_err(|e| PortError::Backend(e.to_string()))?,
                claim: row.get(2).map_err(|e| PortError::Backend(e.to_string()))?,
                evidence: serde_json::from_str(&evidence).unwrap_or_default(),
                confidence: row.get(4).map_err(|e| PortError::Backend(e.to_string()))?,
                importance: row.get(5).map_err(|e| PortError::Backend(e.to_string()))?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_str).map_err(|e| PortError::Serde(e.to_string()))?.with_timezone(&chrono::Utc),
                last_consolidated: chrono::DateTime::parse_from_rfc3339(&consolidated_str).map_err(|e| PortError::Serde(e.to_string()))?.with_timezone(&chrono::Utc),
                supersedes: serde_json::from_str(&supersedes).unwrap_or_default(),
                tier: row.get(9).map_err(|e| PortError::Backend(e.to_string()))?,
                salience: row.get(10).map_err(|e| PortError::Backend(e.to_string()))?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list(&self, limit: usize) -> Result<Vec<Note>, PortError> {
        let conn = self.conn.lock().map_err(|e| PortError::Backend(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT nid, topic, claim, evidence, confidence, importance, created_at, last_consolidated, supersedes, tier, salience FROM notes ORDER BY last_consolidated DESC LIMIT ?"
        ).map_err(|e| PortError::Backend(e.to_string()))?;
        let mut notes = Vec::new();
        let mut rows = stmt.query(params![limit as i64]).map_err(|e| PortError::Backend(e.to_string()))?;
        while let Some(row) = rows.next().map_err(|e| PortError::Backend(e.to_string()))? {
            let evidence: String = row.get(3).map_err(|e| PortError::Backend(e.to_string()))?;
            let supersedes: String = row.get(8).map_err(|e| PortError::Backend(e.to_string()))?;
            let created_str: String = row.get(6).map_err(|e| PortError::Backend(e.to_string()))?;
            let consolidated_str: String = row.get(7).map_err(|e| PortError::Backend(e.to_string()))?;
            notes.push(Note {
                nid: row.get(0).map_err(|e| PortError::Backend(e.to_string()))?,
                topic: row.get(1).map_err(|e| PortError::Backend(e.to_string()))?,
                claim: row.get(2).map_err(|e| PortError::Backend(e.to_string()))?,
                evidence: serde_json::from_str(&evidence).unwrap_or_default(),
                confidence: row.get(4).map_err(|e| PortError::Backend(e.to_string()))?,
                importance: row.get(5).map_err(|e| PortError::Backend(e.to_string()))?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_str).map_err(|e| PortError::Serde(e.to_string()))?.with_timezone(&chrono::Utc),
                last_consolidated: chrono::DateTime::parse_from_rfc3339(&consolidated_str).map_err(|e| PortError::Serde(e.to_string()))?.with_timezone(&chrono::Utc),
                supersedes: serde_json::from_str(&supersedes).unwrap_or_default(),
                tier: row.get(9).map_err(|e| PortError::Backend(e.to_string()))?,
                salience: row.get(10).map_err(|e| PortError::Backend(e.to_string()))?,
            });
        }
        Ok(notes)
    }

    async fn forget(&self, nid: &str) -> Result<(), PortError> {
        let conn = self.conn.lock().map_err(|e| PortError::Backend(e.to_string()))?;
        conn.execute("DELETE FROM notes WHERE nid = ?", params![nid])
            .map_err(|e| PortError::Backend(e.to_string()))?;
        Ok(())
    }
}