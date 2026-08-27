//! P-arch (2026-08-27) + v2.0.0-rc.1 RC-2: Experience trait 真 SQLite impl
//!
//! impl 在 apeireth-memory (engine), trait 在 apeireth-plugin (foundation)

use std::sync::Arc;
use apeireth_plugin::experience::{
    AssociationEdge, AssociationStore, GraphFact, GraphLink, KnowledgeGraphStore, WikiEntry,
    WikiEntryStore,
};
use apeireth_plugin::memory_backend::CapabilityResult;
use apeireth_storage::SqliteConnectionPool;

pub struct SQLiteExperienceStore {
    pool: Arc<SqliteConnectionPool>,
}

impl SQLiteExperienceStore {
    pub fn new(pool: SqliteConnectionPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub fn from_arc(pool: Arc<SqliteConnectionPool>) -> Self {
        Self { pool }
    }

    pub async fn ensure_schema(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let pool = self.pool.clone();
        pool.write(|conn| -> Result<(), apeireth_storage::StorageError> {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS wiki_entries (
                    id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL,
                    source_episode_id TEXT NOT NULL,
                    topic TEXT NOT NULL,
                    summary TEXT NOT NULL,
                    body TEXT NOT NULL,
                    confidence REAL NOT NULL,
                    tags TEXT NOT NULL,
                    extracted_at INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_wiki_session_extracted
                    ON wiki_entries(session_id, extracted_at);
                CREATE TABLE IF NOT EXISTS kg_facts (
                    id TEXT PRIMARY KEY,
                    subject_id TEXT NOT NULL,
                    subject_kind TEXT NOT NULL,
                    predicate TEXT NOT NULL,
                    object_id TEXT NOT NULL,
                    object_kind TEXT NOT NULL,
                    valid_from_ms INTEGER NOT NULL,
                    valid_until_ms INTEGER,
                    source_episode_id TEXT NOT NULL,
                    confidence REAL NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_kg_facts_subject
                    ON kg_facts(subject_id);
                CREATE TABLE IF NOT EXISTS kg_links (
                    from_id TEXT NOT NULL,
                    to_id TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    weight REAL NOT NULL,
                    source_episode_id TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    PRIMARY KEY (from_id, to_id, kind)
                );
                CREATE TABLE IF NOT EXISTS association_nodes (
                    entity_id TEXT PRIMARY KEY,
                    co_occurrence_count INTEGER NOT NULL,
                    last_seen_at INTEGER NOT NULL
                );
                CREATE TABLE IF NOT EXISTS association_edges (
                    from_entity TEXT NOT NULL,
                    to_entity TEXT NOT NULL,
                    co_occurrence_count INTEGER NOT NULL,
                    last_seen_episode_id TEXT,
                    last_seen_at INTEGER NOT NULL,
                    PRIMARY KEY (from_entity, to_entity)
                );",
            )
            .map_err(apeireth_storage::StorageError::from)
        })
        .await
        .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

impl WikiEntryStore for SQLiteExperienceStore {
    fn put_wiki(&self, entry: &WikiEntry) -> CapabilityResult<()> {
        let tags_json = serde_json::to_string(&entry.tags)
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        self.pool
            .read(|conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO wiki_entries \
                     (id, session_id, source_episode_id, topic, summary, body, confidence, tags, extracted_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        entry.id,
                        entry.session_id,
                        entry.source_episode_id,
                        entry.topic,
                        entry.summary,
                        entry.body,
                        entry.confidence,
                        tags_json,
                        entry.extracted_at,
                    ],
                )?;
                Ok(())
            })
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn list_wiki(
        &self,
        session_id: &str,
        topic: &str,
        limit: u32,
    ) -> CapabilityResult<Vec<WikiEntry>> {
        self.pool
            .read(|conn| -> Result<Vec<WikiEntry>, apeireth_storage::StorageError> {
                let mut stmt = conn.prepare_cached(
                    "SELECT id, session_id, source_episode_id, topic, summary, body, confidence, tags, extracted_at \
                     FROM wiki_entries \
                     WHERE session_id = ?1 \
                       AND (?2 = '' OR topic LIKE '%' || ?2 || '%') \
                     ORDER BY extracted_at DESC \
                     LIMIT ?3",
                )?;
                let rows = stmt.query_map(
                    rusqlite::params![session_id, topic, i64::from(limit)],
                    |row| {
                        let id: String = row.get(0)?;
                        let session_id: String = row.get(1)?;
                        let source_episode_id: String = row.get(2)?;
                        let topic: String = row.get(3)?;
                        let summary: String = row.get(4)?;
                        let body: String = row.get(5)?;
                        let confidence: f64 = row.get(6)?;
                        let tags_str: String = row.get(7)?;
                        let extracted_at: i64 = row.get(8)?;
                        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                        Ok(WikiEntry {
                            id,
                            session_id,
                            source_episode_id,
                            topic,
                            summary,
                            body,
                            confidence,
                            tags,
                            extracted_at,
                        })
                    },
                )?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                Ok(out)
            })
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn wiki_for_episode(&self, episode_id: &str) -> CapabilityResult<Vec<WikiEntry>> {
        self.pool
            .read(|conn| -> Result<Vec<WikiEntry>, apeireth_storage::StorageError> {
                let mut stmt = conn.prepare_cached(
                    "SELECT id, session_id, source_episode_id, topic, summary, body, confidence, tags, extracted_at \
                     FROM wiki_entries \
                     WHERE source_episode_id = ?1 \
                     ORDER BY extracted_at DESC",
                )?;
                let rows = stmt.query_map(rusqlite::params![episode_id], |row| {
                    let id: String = row.get(0)?;
                    let session_id: String = row.get(1)?;
                    let source_episode_id: String = row.get(2)?;
                    let topic: String = row.get(3)?;
                    let summary: String = row.get(4)?;
                    let body: String = row.get(5)?;
                    let confidence: f64 = row.get(6)?;
                    let tags_str: String = row.get(7)?;
                    let extracted_at: i64 = row.get(8)?;
                    let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                    Ok(WikiEntry {
                        id,
                        session_id,
                        source_episode_id,
                        topic,
                        summary,
                        body,
                        confidence,
                        tags,
                        extracted_at,
                    })
                })?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                Ok(out)
            })
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

impl KnowledgeGraphStore for SQLiteExperienceStore {
    fn put_fact(&self, fact: &GraphFact) -> CapabilityResult<()> {
        self.pool
            .read(|conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO kg_facts \
                     (id, subject_id, subject_kind, predicate, object_id, object_kind, \
                      valid_from_ms, valid_until_ms, source_episode_id, confidence) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        fact.id,
                        fact.subject_id,
                        fact.subject_kind,
                        fact.predicate,
                        fact.object_id,
                        fact.object_kind,
                        fact.valid_from,
                        fact.valid_until,
                        fact.source_episode_id,
                        fact.confidence,
                    ],
                )?;
                Ok(())
            })
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn put_link(&self, link: &GraphLink) -> CapabilityResult<()> {
        self.pool
            .read(|conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO kg_links \
                     (from_id, to_id, kind, weight, source_episode_id, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        link.from_id,
                        link.to_id,
                        link.kind,
                        link.weight,
                        link.source_episode_id,
                        link.created_at,
                    ],
                )?;
                Ok(())
            })
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn facts_from(
        &self,
        subject_id: &str,
        limit: u32,
    ) -> CapabilityResult<Vec<GraphFact>> {
        self.pool
            .read(|conn| -> Result<Vec<GraphFact>, apeireth_storage::StorageError> {
                let mut stmt = conn.prepare_cached(
                    "SELECT id, subject_id, subject_kind, predicate, object_id, object_kind, \
                            valid_from_ms, valid_until_ms, source_episode_id, confidence \
                     FROM kg_facts \
                     WHERE subject_id = ?1 \
                       AND (valid_until_ms IS NULL OR valid_until_ms > ?2) \
                     ORDER BY confidence DESC \
                     LIMIT ?3",
                )?;
                let now_ms = chrono::Utc::now().timestamp_millis();
                let rows = stmt.query_map(
                    rusqlite::params![subject_id, now_ms, i64::from(limit)],
                    |row| {
                        Ok(GraphFact {
                            id: row.get(0)?,
                            subject_id: row.get(1)?,
                            subject_kind: row.get(2)?,
                            predicate: row.get(3)?,
                            object_id: row.get(4)?,
                            object_kind: row.get(5)?,
                            valid_from: row.get(6)?,
                            valid_until: row.get(7)?,
                            source_episode_id: row.get(8)?,
                            confidence: row.get(9)?,
                        })
                    },
                )?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                Ok(out)
            })
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn links_from(
        &self,
        from_id: &str,
        limit: u32,
    ) -> CapabilityResult<Vec<GraphLink>> {
        self.pool
            .read(|conn| -> Result<Vec<GraphLink>, apeireth_storage::StorageError> {
                let mut stmt = conn.prepare_cached(
                    "SELECT from_id, to_id, kind, weight, source_episode_id, created_at \
                     FROM kg_links WHERE from_id = ?1 ORDER BY weight DESC LIMIT ?2",
                )?;
                let rows = stmt.query_map(rusqlite::params![from_id, i64::from(limit)], |row| {
                    Ok(GraphLink {
                        from_id: row.get(0)?,
                        to_id: row.get(1)?,
                        kind: row.get(2)?,
                        weight: row.get(3)?,
                        source_episode_id: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                })?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                Ok(out)
            })
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn forget_subject(&self, subject_id: &str) -> CapabilityResult<()> {
        self.pool
            .read(|conn| {
                conn.execute(
                    "DELETE FROM kg_facts WHERE subject_id = ?1",
                    rusqlite::params![subject_id],
                )?;
                conn.execute(
                    "DELETE FROM kg_links WHERE from_id = ?1 OR to_id = ?1",
                    rusqlite::params![subject_id],
                )?;
                Ok(())
            })
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

impl AssociationStore for SQLiteExperienceStore {
    fn record_cooccurrence(
        &self,
        from: &str,
        to: &str,
        episode_id: &str,
    ) -> CapabilityResult<()> {
        let now = chrono::Utc::now().timestamp();
        self.pool.read(|conn| {
            conn.execute(
                "INSERT INTO association_edges (from_entity, to_entity, co_occurrence_count, last_seen_episode_id, last_seen_at) \
                 VALUES (?1, ?2, 1, ?3, ?4) \
                 ON CONFLICT (from_entity, to_entity) DO UPDATE SET \
                    co_occurrence_count = co_occurrence_count + 1, \
                    last_seen_episode_id = excluded.last_seen_episode_id, \
                    last_seen_at = excluded.last_seen_at",
                rusqlite::params![from, to, episode_id, now],
            )?;
            for entity in [from, to] {
                conn.execute(
                    "INSERT INTO association_nodes (entity_id, co_occurrence_count, last_seen_at) \
                     VALUES (?1, 1, ?2) \
                     ON CONFLICT (entity_id) DO UPDATE SET \
                        co_occurrence_count = co_occurrence_count + 1, \
                        last_seen_at = excluded.last_seen_at",
                    rusqlite::params![entity, now],
                )?;
            }
            Ok(())
        })
        .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn top_associations(
        &self,
        entity: &str,
        limit: u32,
    ) -> CapabilityResult<Vec<AssociationEdge>> {
        self.pool
            .read(|conn: &rusqlite::Connection| -> Result<Vec<AssociationEdge>, apeireth_storage::StorageError> {
                let mut stmt = conn.prepare_cached(
                    "SELECT from_entity, to_entity, co_occurrence_count, last_seen_episode_id, last_seen_at \
                     FROM association_edges \
                     WHERE from_entity = ?1 OR to_entity = ?1 \
                     ORDER BY co_occurrence_count DESC \
                     LIMIT ?2",
                )?;
                let rows = stmt.query_map(rusqlite::params![entity, i64::from(limit)], |row| {
                    Ok(AssociationEdge {
                        from_entity: row.get(0)?,
                        to_entity: row.get(1)?,
                        co_occurrence_count: row.get(2)?,
                        last_seen_episode_id: row.get(3)?,
                        last_seen_at: row.get(4)?,
                    })
                })?;
                let mut out = Vec::new();
                for r in rows {
                    out.push(r?);
                }
                Ok(out)
            })
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;

    async fn fresh() -> SQLiteExperienceStore {
        let pool = SqliteConnectionPool::in_memory()
            .await
            .expect("in-memory pool");
        let store = SQLiteExperienceStore::new(pool);
        store.ensure_schema().await.expect("ensure_schema");
        store
    }

    /// RC-2 验收: WikiEntryStore roundtrip
    #[tokio::test]
    async fn wiki_store_roundtrip() {
        let store = fresh().await;
        let sid = SessionId::new();
        let entry = WikiEntry {
            id: "wiki-1".into(),
            session_id: sid.to_string(),
            source_episode_id: "ep-1".into(),
            topic: "Rust language".into(),
            summary: "Rust is fast".into(),
            body: "Rust is fast and safe".into(),
            confidence: 0.85,
            tags: vec!["rust".into()],
            extracted_at: 1_700_000_000,
        };
        WikiEntryStore::put_wiki(&store, &entry).expect("put");
        let list = WikiEntryStore::list_wiki(&store, &sid.to_string(), "Rust", 10)
            .expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "wiki-1");
    }

    /// RC-2 验收: KnowledgeGraphStore fact + link roundtrip
    #[tokio::test]
    async fn kg_fact_and_link_roundtrip() {
        let store = fresh().await;
        let fact = GraphFact {
            id: "fact-1".into(),
            subject_id: "rust".into(),
            subject_kind: "language".into(),
            predicate: "is".into(),
            object_id: "fast".into(),
            object_kind: "property".into(),
            valid_from: 1_700_000_000,
            valid_until: None,
            source_episode_id: "ep-1".into(),
            confidence: 0.9,
        };
        KnowledgeGraphStore::put_fact(&store, &fact).expect("put_fact");
        let facts = KnowledgeGraphStore::facts_from(&store, "rust", 10)
            .expect("facts_from");
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].id, "fact-1");

        let link = GraphLink {
            from_id: "rust".into(),
            to_id: "fast".into(),
            kind: "is".into(),
            weight: 0.9,
            source_episode_id: "ep-1".into(),
            created_at: 1_700_000_000,
        };
        KnowledgeGraphStore::put_link(&store, &link).expect("put_link");
        let links = KnowledgeGraphStore::links_from(&store, "rust", 10)
            .expect("links_from");
        assert_eq!(links.len(), 1);
    }

    /// RC-2 验收: forget_subject 真删
    #[tokio::test]
    async fn kg_forget_subject_deletes() {
        let store = fresh().await;
        let fact = GraphFact {
            id: "f-1".into(),
            subject_id: "ephemeral".into(),
            subject_kind: "thing".into(),
            predicate: "is".into(),
            object_id: "gone".into(),
            object_kind: "state".into(),
            valid_from: 1_700_000_000,
            valid_until: None,
            source_episode_id: "ep-1".into(),
            confidence: 0.5,
        };
        KnowledgeGraphStore::put_fact(&store, &fact).expect("put");
        assert_eq!(
            KnowledgeGraphStore::facts_from(&store, "ephemeral", 10).unwrap().len(),
            1
        );
        KnowledgeGraphStore::forget_subject(&store, "ephemeral").expect("forget");
        assert_eq!(
            KnowledgeGraphStore::facts_from(&store, "ephemeral", 10).unwrap().len(),
            0,
            "forget 真删"
        );
    }

    /// RC-2 验收: AssociationStore record_cooccurrence UPSERT
    #[tokio::test]
    async fn association_record_and_top() {
        let store = fresh().await;
        AssociationStore::record_cooccurrence(&store, "rust", "fast", "ep-1")
            .expect("rec 1");
        AssociationStore::record_cooccurrence(&store, "rust", "fast", "ep-2")
            .expect("rec 2");
        let top = AssociationStore::top_associations(&store, "rust", 10).expect("top");
        assert!(!top.is_empty());
        assert!(
            top[0].co_occurrence_count >= 2,
            "co_occurrence_count 累加, not 0 装 reset"
        );
    }

    /// RC-2 验收: Send + Sync 边界
    #[test]
    fn experience_store_is_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<SQLiteExperienceStore>();
    }

    /// RC-2 验收: ensure_schema 幂等
    #[tokio::test]
    async fn ensure_schema_idempotent() {
        let pool = SqliteConnectionPool::in_memory().await.expect("pool");
        let store = SQLiteExperienceStore::new(pool);
        store.ensure_schema().await.expect("first");
        store.ensure_schema().await.expect("second idempotent");
    }
}
