//! SQLite Episode adapter (借鉴 Phase 2.5 Python v0.2)

use async_trait::async_trait;
use apeireth_core::Episode;
use apeireth_ports::{EpisodeRepository, PortError};
use rusqlite::{Connection, params};

pub struct SqliteEpisodeRepository {
    conn: std::sync::Arc<std::sync::Mutex<Connection>>,
}

impl SqliteEpisodeRepository {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, PortError> {
        let conn = Connection::open(path).map_err(|e| PortError::Io(e.to_string()))?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS episodes (
                eid TEXT PRIMARY KEY,
                actor TEXT NOT NULL,
                content TEXT NOT NULL,
                context TEXT DEFAULT '',
                kind TEXT DEFAULT 'utterance',
                ts TEXT NOT NULL,
                linked_identity_hash TEXT DEFAULT '',
                fingerprint TEXT NOT NULL,
                tier TEXT DEFAULT 'stm'
            );
            CREATE INDEX IF NOT EXISTS idx_episodes_ts ON episodes(ts);
            CREATE INDEX IF NOT EXISTS idx_episodes_tier ON episodes(tier);
            CREATE INDEX IF NOT EXISTS idx_episodes_fingerprint ON episodes(fingerprint);
        ").map_err(|e| PortError::Backend(e.to_string()))?;
        Ok(Self { conn: std::sync::Arc::new(std::sync::Mutex::new(conn)) })
    }
}

#[async_trait]
impl EpisodeRepository for SqliteEpisodeRepository {
    async fn append(&self, episode: &Episode) -> Result<bool, PortError> {
        let conn = self.conn.lock().map_err(|e| PortError::Backend(e.to_string()))?;
        // De-dup check
        let exists: bool = conn.query_row(
            "SELECT 1 FROM episodes WHERE fingerprint = ? LIMIT 1",
            params![episode.fingerprint],
            |_| Ok(true),
        ).unwrap_or(false);
        if exists {
            return Ok(false);
        }
        let actor_str = format!("{:?}", episode.actor);
        let kind_str = format!("{:?}", episode.kind);
        conn.execute(
            "INSERT INTO episodes(eid, actor, content, context, kind, ts, linked_identity_hash, fingerprint, tier) VALUES (?,?,?,?,?,?,?,?,?)",
            params![
                episode.eid,
                actor_str,
                episode.content,
                episode.context,
                kind_str,
                episode.ts.to_rfc3339(),
                episode.linked_identity_hash,
                episode.fingerprint,
                episode.tier,
            ],
        ).map_err(|e| PortError::Backend(e.to_string()))?;
        Ok(true)
    }

    async fn get(&self, eid: &str) -> Result<Option<Episode>, PortError> {
        let conn = self.conn.lock().map_err(|e| PortError::Backend(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT eid, actor, content, context, kind, ts, linked_identity_hash, fingerprint, tier FROM episodes WHERE eid = ?"
        ).map_err(|e| PortError::Backend(e.to_string()))?;
        let mut rows = stmt.query(params![eid]).map_err(|e| PortError::Backend(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| PortError::Backend(e.to_string()))? {
            let actor_str: String = row.get(1).map_err(|e| PortError::Backend(e.to_string()))?;
            let kind_str: String = row.get(4).map_err(|e| PortError::Backend(e.to_string()))?;
            let ts_str: String = row.get(5).map_err(|e| PortError::Backend(e.to_string()))?;
            let actor = match actor_str.as_str() {
                "Master" => apeireth_core::episode::Actor::Master,
                "Apeireth" => apeireth_core::episode::Actor::Apeireth,
                "Tool" => apeireth_core::episode::Actor::Tool,
                _ => apeireth_core::episode::Actor::System,
            };
            let kind = match kind_str.as_str() {
                "Utterance" => apeireth_core::episode::EpisodeKind::Utterance,
                "ToolCall" => apeireth_core::episode::EpisodeKind::ToolCall,
                "Observation" => apeireth_core::episode::EpisodeKind::Observation,
                "Kickoff" => apeireth_core::episode::EpisodeKind::Kickoff,
                "Reflection" => apeireth_core::episode::EpisodeKind::Reflection,
                "Consolidation" => apeireth_core::episode::EpisodeKind::Consolidation,
                _ => apeireth_core::episode::EpisodeKind::Utterance,
            };
            let ts = chrono::DateTime::parse_from_rfc3339(&ts_str)
                .map_err(|e| PortError::Serde(e.to_string()))?
                .with_timezone(&chrono::Utc);
            Ok(Some(Episode {
                eid: row.get(0).map_err(|e| PortError::Backend(e.to_string()))?,
                actor,
                content: row.get(2).map_err(|e| PortError::Backend(e.to_string()))?,
                context: row.get(3).map_err(|e| PortError::Backend(e.to_string()))?,
                kind,
                ts,
                linked_identity_hash: row.get(6).map_err(|e| PortError::Backend(e.to_string()))?,
                fingerprint: row.get(7).map_err(|e| PortError::Backend(e.to_string()))?,
                tier: row.get(8).map_err(|e| PortError::Backend(e.to_string()))?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_by_tier(&self, tier: &str, limit: usize) -> Result<Vec<Episode>, PortError> {
        let conn = self.conn.lock().map_err(|e| PortError::Backend(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT eid, actor, content, context, kind, ts, linked_identity_hash, fingerprint, tier FROM episodes WHERE tier = ? ORDER BY ts DESC LIMIT ?"
        ).map_err(|e| PortError::Backend(e.to_string()))?;
        let rows: Result<Vec<_>, _> = stmt.query_map(params![tier, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        }).map_err(|e| PortError::Backend(e.to_string()))?
        .collect();
        let rows = rows.map_err(|e| PortError::Backend(e.to_string()))?;
        let mut eps = Vec::with_capacity(rows.len());
        for r in rows {
            let actor = match r.1.as_str() {
                "Master" => apeireth_core::episode::Actor::Master,
                "Apeireth" => apeireth_core::episode::Actor::Apeireth,
                "Tool" => apeireth_core::episode::Actor::Tool,
                _ => apeireth_core::episode::Actor::System,
            };
            let kind = match r.4.as_str() {
                "Utterance" => apeireth_core::episode::EpisodeKind::Utterance,
                "ToolCall" => apeireth_core::episode::EpisodeKind::ToolCall,
                "Observation" => apeireth_core::episode::EpisodeKind::Observation,
                "Kickoff" => apeireth_core::episode::EpisodeKind::Kickoff,
                "Reflection" => apeireth_core::episode::EpisodeKind::Reflection,
                "Consolidation" => apeireth_core::episode::EpisodeKind::Consolidation,
                _ => apeireth_core::episode::EpisodeKind::Utterance,
            };
            let ts = chrono::DateTime::parse_from_rfc3339(&r.5)
                .map_err(|e| PortError::Serde(e.to_string()))?
                .with_timezone(&chrono::Utc);
            eps.push(Episode {
                eid: r.0,
                actor,
                content: r.2,
                context: r.3,
                kind,
                ts,
                linked_identity_hash: r.6,
                fingerprint: r.7,
                tier: r.8,
            });
        }
        Ok(eps)
    }

    async fn count(&self) -> Result<u64, PortError> {
        let conn = self.conn.lock().map_err(|e| PortError::Backend(e.to_string()))?;
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM episodes", [], |r| r.get(0))
            .map_err(|e| PortError::Backend(e.to_string()))?;
        Ok(n as u64)
    }
}