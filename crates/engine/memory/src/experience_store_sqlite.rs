//! P-arch (2026-08-27) + v2.0.0-rc.1 RC-2: Experience trait 真 SQLite impl
//!
//! impl 在 apeireth-memory (engine), trait 在 apeireth-plugin (foundation)

use apeireth_plugin::experience::{
    AssociationEdge, AssociationStore, GraphFact, GraphLink, KnowledgeGraphStore, WikiEntry,
    WikiEntryStore,
};
use apeireth_plugin::memory_backend::CapabilityResult;
use apeireth_storage::SqliteConnectionPool;
use std::sync::Arc;

// A Host-supplied external source without an episode remains compatible; an
// explicit governance withdrawal or journal suppression always takes priority.
fn source_filter(
    conn: &rusqlite::Connection,
    source: &str,
) -> Result<String, apeireth_storage::StorageError> {
    let table = |name| {
        crate::writeback_recovery::has_table(conn, name)
            .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))
    };
    let mut predicates = vec!["1".to_owned()];
    let governed = table("episode_governance")?;
    if governed {
        predicates.push(format!("NOT EXISTS(SELECT 1 FROM episode_governance source_g WHERE source_g.episode_id={source} AND (source_g.status='forgotten' OR source_g.content_override IS NOT NULL))"));
    }
    if table("memory_writeback_jobs")? {
        predicates.push(format!("NOT EXISTS(SELECT 1 FROM memory_writeback_dependencies source_d JOIN memory_writeback_jobs source_j ON source_j.id=source_d.job_id WHERE source_d.episode_id={source} AND source_j.state='suppressed')"));
        if governed {
            predicates.push(format!("NOT EXISTS(SELECT 1 FROM memory_writeback_dependencies source_d JOIN memory_writeback_dependencies target_d ON target_d.job_id=source_d.job_id JOIN episode_governance source_g ON source_g.episode_id=target_d.episode_id WHERE source_d.episode_id={source} AND source_g.status='forgotten')"));
        }
    }
    Ok(predicates.join(" AND "))
}

fn source_visible(
    conn: &rusqlite::Connection,
    id: &str,
) -> Result<bool, apeireth_storage::StorageError> {
    if crate::writeback_recovery::has_table(conn, "episode_governance")
        .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))?
    {
        let revised:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM episode_governance WHERE episode_id=?1 AND content_override IS NOT NULL)",[id],|r|r.get(0))?;
        if revised {
            return Ok(false);
        }
    }
    match crate::writeback_recovery::assert_episode_source_allowed(conn, id) {
        Ok(()) => Ok(true),
        Err(crate::MemoryError::Invalid(_)) => Ok(false),
        Err(e) => Err(apeireth_storage::StorageError::Serialization(e.to_string())),
    }
}

fn governed_association(
    conn: &rusqlite::Connection,
    mut edge: AssociationEdge,
) -> Result<Option<AssociationEdge>, apeireth_storage::StorageError> {
    let mut statement=conn.prepare("SELECT source_episode_id FROM association_observations WHERE from_entity=?1 AND to_entity=?2")?;
    let observed = statement
        .query_map(rusqlite::params![edge.from_entity, edge.to_entity], |r| {
            r.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if observed.is_empty() {
        // Legacy aggregates have no per-source ledger. Preserve the recorded
        // aggregate only while its recorded latest source is still visible;
        // never manufacture observations for its historical count.
        return Ok(source_visible(conn, &edge.last_seen_episode_id)?.then_some(edge));
    }
    let mut active = Vec::new();
    for id in observed {
        if source_visible(conn, &id)? {
            let at = crate::writeback_recovery::episode_source_timestamp(conn, &id)
                .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))?;
            active.push((id, at));
        }
    }
    if active.is_empty() {
        return Ok(None);
    }
    edge.co_occurrence_count = u32::try_from(active.len()).map_err(|_| {
        apeireth_storage::StorageError::Serialization("association count exceeds u32".into())
    })?;
    if !active
        .iter()
        .any(|(id, _)| id == &edge.last_seen_episode_id)
    {
        // Older external observations carry no timestamp. Once the latest is
        // withdrawn the public nonoptional time cannot honestly represent them.
        if active.iter().any(|(_, at)| at.is_none()) {
            return Ok(None);
        }
        active.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        edge.last_seen_episode_id = active[0].0.clone();
        edge.last_seen_at = active[0].1.expect("checked timestamp");
    }
    Ok(Some(edge))
}

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
                );
                CREATE TABLE IF NOT EXISTS association_observations (
                    from_entity TEXT NOT NULL,
                    to_entity TEXT NOT NULL,
                    source_episode_id TEXT NOT NULL,
                    PRIMARY KEY (from_entity, to_entity, source_episode_id)
                );",
            )
            .map_err(apeireth_storage::StorageError::from)
        })
        .await
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

impl WikiEntryStore for SQLiteExperienceStore {
    fn put_wiki(&self, entry: &WikiEntry) -> CapabilityResult<()> {
        let tags_json = serde_json::to_string(&entry.tags)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let id = entry.id.clone();
        let session_id = entry.session_id.clone();
        let source_episode_id = entry.source_episode_id.clone();
        let topic = entry.topic.clone();
        let summary = entry.summary.clone();
        let body = entry.body.clone();
        let confidence = entry.confidence;
        let extracted_at = entry.extracted_at;
        self.pool
            .write_sync(move |conn| {
                let tx=conn.transaction()?;
                crate::writeback_recovery::assert_episode_source_allowed(&tx,&source_episode_id)
                    .map_err(|e|apeireth_storage::StorageError::Serialization(e.to_string()))?;
                tx.execute(
                    "INSERT OR REPLACE INTO wiki_entries \
                     (id, session_id, source_episode_id, topic, summary, body, confidence, tags, extracted_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        id, session_id, source_episode_id, topic, summary, body,
                        confidence, tags_json, extracted_at,
                    ],
                )?;
                tx.commit()?;
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
                let visible=source_filter(conn,"wiki_entries.source_episode_id")?;
                let mut stmt = conn.prepare_cached(
                    &format!("SELECT id, session_id, source_episode_id, topic, summary, body, confidence, tags, extracted_at \
                     FROM wiki_entries \
                     WHERE session_id = ?1 \
                       AND (?2 = '' OR topic LIKE '%' || ?2 || '%') \
                       AND {visible} \
                     ORDER BY extracted_at DESC \
                     LIMIT ?3"),
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
                let visible=source_filter(conn,"wiki_entries.source_episode_id")?;
                let mut stmt = conn.prepare_cached(
                    &format!("SELECT id, session_id, source_episode_id, topic, summary, body, confidence, tags, extracted_at \
                     FROM wiki_entries \
                     WHERE source_episode_id = ?1 AND {visible} \
                     ORDER BY extracted_at DESC"),
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
        let id = fact.id.clone();
        let subject_id = fact.subject_id.clone();
        let subject_kind = fact.subject_kind.clone();
        let predicate = fact.predicate.clone();
        let object_id = fact.object_id.clone();
        let object_kind = fact.object_kind.clone();
        let valid_from = fact.valid_from;
        let valid_until = fact.valid_until;
        let source_episode_id = fact.source_episode_id.clone();
        let confidence = fact.confidence;
        self.pool
            .write_sync(move |conn| {
                let tx = conn.transaction()?;
                crate::writeback_recovery::assert_episode_source_allowed(&tx, &source_episode_id)
                    .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))?;
                tx.execute(
                    "INSERT OR REPLACE INTO kg_facts \
                     (id, subject_id, subject_kind, predicate, object_id, object_kind, \
                      valid_from_ms, valid_until_ms, source_episode_id, confidence) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        id,
                        subject_id,
                        subject_kind,
                        predicate,
                        object_id,
                        object_kind,
                        valid_from,
                        valid_until,
                        source_episode_id,
                        confidence,
                    ],
                )?;
                tx.commit()?;
                Ok(())
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn put_link(&self, link: &GraphLink) -> CapabilityResult<()> {
        let from_id = link.from_id.clone();
        let to_id = link.to_id.clone();
        let kind = link.kind.clone();
        let weight = link.weight;
        let source_episode_id = link.source_episode_id.clone();
        let created_at = link.created_at;
        self.pool
            .write_sync(move |conn| {
                let tx = conn.transaction()?;
                crate::writeback_recovery::assert_episode_source_allowed(&tx, &source_episode_id)
                    .map_err(|e| apeireth_storage::StorageError::Serialization(e.to_string()))?;
                tx.execute(
                    "INSERT OR REPLACE INTO kg_links \
                     (from_id, to_id, kind, weight, source_episode_id, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![from_id, to_id, kind, weight, source_episode_id, created_at,],
                )?;
                tx.commit()?;
                Ok(())
            })
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    /// ⭐ **按 as-of 时间取事实**（2026-11-05 覆写 trait 默认实现）。
    ///
    /// **为什么必须覆写**：默认实现是在 `facts_from`（=「现在有效」）的结果上过滤
    /// ⇒ ⇒ **拿不回「当时有效、现已作废」的事实** ⇒ ⇒ 时间旅行查询会漏。
    /// ⇒ ⇒ 这里**走真 SQL**：valid-time 区间直接落在 WHERE 上。
    ///
    /// ⚠️ **与 `facts_from` 的唯一差别** = 多一个 `valid_from_ms <= ?2`
    /// ⇒ ⇒ **未来才生效的事实不再出现在「现在」的查询里**（原先会漏出来 ⇒ ⇒ 那是 bug）。
    fn facts_from_as_of(
        &self,
        subject_id: &str,
        limit: u32,
        as_of_ms: i64,
    ) -> CapabilityResult<Vec<GraphFact>> {
        self.pool
            .read(
                |conn: &rusqlite::Connection| -> Result<Vec<GraphFact>, apeireth_storage::StorageError> {
                    let visible=source_filter(conn,"kg_facts.source_episode_id")?;
                    let mut stmt = conn.prepare_cached(
                        &format!("SELECT id, subject_id, subject_kind, predicate, object_id, object_kind, \
                            valid_from_ms, valid_until_ms, source_episode_id, confidence \
                         FROM kg_facts \
                         WHERE subject_id = ?1 \
                           AND valid_from_ms <= ?2 \
                           AND (valid_until_ms IS NULL OR valid_until_ms > ?2) \
                           AND {visible} \
                         ORDER BY confidence DESC \
                         LIMIT ?3"),
                    )?;
                    let rows = stmt.query_map(
                        rusqlite::params![subject_id, as_of_ms, i64::from(limit)],
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
                },
            )
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn facts_from(&self, subject_id: &str, limit: u32) -> CapabilityResult<Vec<GraphFact>> {
        self.pool
            .read(
                |conn| -> Result<Vec<GraphFact>, apeireth_storage::StorageError> {
                    let visible = source_filter(conn, "kg_facts.source_episode_id")?;
                    let mut stmt = conn.prepare_cached(&format!(
                        "SELECT id, subject_id, subject_kind, predicate, object_id, object_kind, \
                            valid_from_ms, valid_until_ms, source_episode_id, confidence \
                     FROM kg_facts \
                     WHERE subject_id = ?1 \
                       AND (valid_until_ms IS NULL OR valid_until_ms > ?2) \
                       AND {visible} \
                     ORDER BY confidence DESC \
                     LIMIT ?3"
                    ))?;
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
                },
            )
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    /// 全量 fact 列举 (W3 community 消费, 2026-10-10): 最近 limit 条,
    /// 有效窗口过滤 (与 facts_from 同口径), 无 subject 前缀。
    fn all_facts(&self, limit: u32) -> CapabilityResult<Vec<GraphFact>> {
        self.pool
            .read(
                |conn| -> Result<Vec<GraphFact>, apeireth_storage::StorageError> {
                    let visible = source_filter(conn, "kg_facts.source_episode_id")?;
                    let mut stmt = conn.prepare_cached(&format!(
                        "SELECT id, subject_id, subject_kind, predicate, object_id, object_kind, \
                            valid_from_ms, valid_until_ms, source_episode_id, confidence \
                         FROM kg_facts \
                         WHERE (valid_until_ms IS NULL OR valid_until_ms > ?1) \
                         AND {visible} \
                         ORDER BY valid_from_ms DESC \
                         LIMIT ?2"
                    ))?;
                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let rows =
                        stmt.query_map(rusqlite::params![now_ms, i64::from(limit)], |row| {
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
                        })?;
                    let mut out = Vec::new();
                    for r in rows {
                        out.push(r?);
                    }
                    Ok(out)
                },
            )
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn links_from(&self, from_id: &str, limit: u32) -> CapabilityResult<Vec<GraphLink>> {
        self.pool
            .read(
                |conn| -> Result<Vec<GraphLink>, apeireth_storage::StorageError> {
                    let visible = source_filter(conn, "kg_links.source_episode_id")?;
                    let mut stmt = conn.prepare_cached(&format!(
                        "SELECT from_id, to_id, kind, weight, source_episode_id, created_at \
                     FROM kg_links WHERE from_id = ?1 AND {visible} ORDER BY weight DESC LIMIT ?2"
                    ))?;
                    let rows =
                        stmt.query_map(rusqlite::params![from_id, i64::from(limit)], |row| {
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
                },
            )
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn forget_subject(&self, subject_id: &str) -> CapabilityResult<()> {
        let subject_id = subject_id.to_string();
        self.pool
            .write_sync(move |conn| {
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
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

impl AssociationStore for SQLiteExperienceStore {
    fn record_cooccurrence(&self, from: &str, to: &str, episode_id: &str) -> CapabilityResult<()> {
        let now = chrono::Utc::now().timestamp();
        let from = from.to_string();
        let to = to.to_string();
        let episode_id = episode_id.to_string();
        self.pool
            .write_sync(move |conn| {
                let tx = conn.transaction()?;
                crate::writeback_recovery::assert_episode_source_allowed(&tx,&episode_id)
                    .map_err(|e|apeireth_storage::StorageError::Serialization(e.to_string()))?;
                let now=crate::writeback_recovery::episode_source_timestamp(&tx,&episode_id)
                    .map_err(|e|apeireth_storage::StorageError::Serialization(e.to_string()))?.unwrap_or(now);
                let inserted = tx.execute(
                    "INSERT OR IGNORE INTO association_observations \
                     (from_entity, to_entity, source_episode_id) VALUES (?1, ?2, ?3)",
                    rusqlite::params![from, to, episode_id],
                )?;
                if inserted == 0 {
                    tx.commit()?;
                    return Ok(());
                }
                tx.execute(
                    "INSERT INTO association_edges (from_entity, to_entity, co_occurrence_count, last_seen_episode_id, last_seen_at) \
                     VALUES (?1, ?2, 1, ?3, ?4) \
                     ON CONFLICT (from_entity, to_entity) DO UPDATE SET \
                        co_occurrence_count = co_occurrence_count + 1, \
                        last_seen_episode_id = excluded.last_seen_episode_id, \
                        last_seen_at = excluded.last_seen_at",
                    rusqlite::params![from, to, episode_id, now],
                )?;
                for entity in [from, to] {
                    tx.execute(
                        "INSERT INTO association_nodes (entity_id, co_occurrence_count, last_seen_at) \
                         VALUES (?1, 1, ?2) \
                         ON CONFLICT (entity_id) DO UPDATE SET \
                            co_occurrence_count = co_occurrence_count + 1, \
                            last_seen_at = excluded.last_seen_at",
                        rusqlite::params![entity, now],
                    )?;
                }
                tx.commit()?;
                Ok(())
            })
            .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))
    }

    fn top_associations(&self, entity: &str, limit: u32) -> CapabilityResult<Vec<AssociationEdge>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        self.pool
            .read(|conn: &rusqlite::Connection| -> Result<Vec<AssociationEdge>, apeireth_storage::StorageError> {
                let mut stmt = conn.prepare_cached(
                    "SELECT from_entity, to_entity, co_occurrence_count, last_seen_episode_id, last_seen_at \
                     FROM association_edges \
                     WHERE from_entity = ?1 OR to_entity = ?1 \
                     ORDER BY co_occurrence_count DESC",
                )?;
                let rows = stmt.query_map(rusqlite::params![entity], |row| {
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
                    if let Some(edge)=governed_association(conn,r?)? {out.push(edge)}
                }
                out.sort_by(|a,b|b.co_occurrence_count.cmp(&a.co_occurrence_count).then_with(||a.from_entity.cmp(&b.from_entity)).then_with(||a.to_entity.cmp(&b.to_entity)));
                out.truncate(limit as usize);
                Ok(out)
            })
            .map_err(|e| -> Box::<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;
    use apeireth_core::Episode;
    use apeireth_plugin::experience::extract_experience;

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
        let list = WikiEntryStore::list_wiki(&store, &sid.to_string(), "Rust", 10).expect("list");
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
        let facts = KnowledgeGraphStore::facts_from(&store, "rust", 10).expect("facts_from");
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
        let links = KnowledgeGraphStore::links_from(&store, "rust", 10).expect("links_from");
        assert_eq!(links.len(), 1);
    }

    /// W3 community 消费 (2026-10-10): all_facts 全量列举 roundtrip + limit 封顶。
    #[tokio::test]
    async fn kg_all_facts_roundtrip_and_limit() {
        let store = fresh().await;
        for index in 0..3 {
            let fact = GraphFact {
                id: format!("f-{index}"),
                subject_id: format!("s{index}"),
                subject_kind: "thing".into(),
                predicate: "rel".into(),
                object_id: format!("o{index}"),
                object_kind: "thing".into(),
                valid_from: 1_700_000_000 + index,
                valid_until: None,
                source_episode_id: "ep-1".into(),
                confidence: 0.8,
            };
            KnowledgeGraphStore::put_fact(&store, &fact).expect("put_fact");
        }
        let all = KnowledgeGraphStore::all_facts(&store, 10).expect("all_facts");
        assert_eq!(all.len(), 3, "全量列举必须看到全部 subject 的 fact");
        let capped = KnowledgeGraphStore::all_facts(&store, 2).expect("all_facts");
        assert_eq!(capped.len(), 2, "limit 封顶");
        // 无 subject 前缀: facts_from 只能按 subject 看一跳, all_facts 是全集。
        let from = KnowledgeGraphStore::facts_from(&store, "s0", 10).expect("facts_from");
        assert_eq!(from.len(), 1);
    }

    /// 默认 trait 实现: 未覆写 all_facts 的 store = 空 (契约默认, 不炸)。
    #[test]
    fn default_all_facts_is_empty_for_non_sqlite_stores() {
        struct NoopStore;
        impl apeireth_plugin::experience::KnowledgeGraphStore for NoopStore {
            fn put_fact(
                &self,
                _fact: &GraphFact,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                Ok(())
            }
            fn put_link(
                &self,
                _link: &GraphLink,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                Ok(())
            }
            fn facts_from(
                &self,
                _subject_id: &str,
                _limit: u32,
            ) -> apeireth_plugin::memory_backend::CapabilityResult<Vec<GraphFact>> {
                Ok(Vec::new())
            }
            fn links_from(
                &self,
                _from_id: &str,
                _limit: u32,
            ) -> apeireth_plugin::memory_backend::CapabilityResult<Vec<GraphLink>> {
                Ok(Vec::new())
            }
            fn forget_subject(
                &self,
                _subject_id: &str,
            ) -> apeireth_plugin::memory_backend::CapabilityResult<()> {
                Ok(())
            }
        }
        assert!(NoopStore.all_facts(10).expect("default").is_empty());
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
            KnowledgeGraphStore::facts_from(&store, "ephemeral", 10)
                .unwrap()
                .len(),
            1
        );
        KnowledgeGraphStore::forget_subject(&store, "ephemeral").expect("forget");
        assert_eq!(
            KnowledgeGraphStore::facts_from(&store, "ephemeral", 10)
                .unwrap()
                .len(),
            0,
            "forget 真删"
        );
    }

    /// RC-2 验收: AssociationStore record_cooccurrence UPSERT
    #[tokio::test]
    async fn association_record_and_top() {
        let store = fresh().await;
        AssociationStore::record_cooccurrence(&store, "rust", "fast", "ep-1").expect("rec 1");
        AssociationStore::record_cooccurrence(&store, "rust", "fast", "ep-1")
            .expect("duplicate rec");
        AssociationStore::record_cooccurrence(&store, "rust", "fast", "ep-2").expect("rec 2");
        let top = AssociationStore::top_associations(&store, "rust", 10).expect("top");
        assert!(!top.is_empty());
        assert!(
            top[0].co_occurrence_count == 2,
            "same episode must not be counted twice"
        );
    }

    #[tokio::test]
    async fn extracted_experience_persists_with_episode_evidence_and_deduplicates() {
        let store = fresh().await;
        let episode = Episode {
            id: "ep-extracted".into(),
            timestamp: 1_700_000_000,
            role: "assistant".into(),
            content: "Rust is fast.\nfact: rust | property | fast\nlink: rust | fast | supports\nassociate: rust | cargo".into(),
            session_id: "session-1".into(),
        };
        let artifacts = extract_experience(&episode).expect("extract");
        for entry in &artifacts.wiki_entries {
            WikiEntryStore::put_wiki(&store, entry).expect("wiki");
        }
        for fact in &artifacts.facts {
            KnowledgeGraphStore::put_fact(&store, fact).expect("fact");
        }
        for link in &artifacts.links {
            KnowledgeGraphStore::put_link(&store, link).expect("link");
        }
        for association in &artifacts.associations {
            AssociationStore::record_cooccurrence(
                &store,
                &association.from_entity,
                &association.to_entity,
                &association.source_episode_id,
            )
            .expect("association");
        }

        // A retry/replay of the same durable episode is idempotent at the
        // existing stores' boundaries.
        for association in &artifacts.associations {
            AssociationStore::record_cooccurrence(
                &store,
                &association.from_entity,
                &association.to_entity,
                &association.source_episode_id,
            )
            .expect("duplicate association");
        }
        assert_eq!(
            WikiEntryStore::wiki_for_episode(&store, &episode.id)
                .expect("wiki evidence")
                .len(),
            1
        );
        assert_eq!(
            KnowledgeGraphStore::facts_from(&store, "rust", 10)
                .expect("fact evidence")
                .len(),
            1
        );
        assert_eq!(
            KnowledgeGraphStore::links_from(&store, "rust", 10)
                .expect("link evidence")
                .len(),
            1
        );
        assert_eq!(
            AssociationStore::top_associations(&store, "rust", 10).expect("association evidence")
                [0]
            .co_occurrence_count,
            1
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
