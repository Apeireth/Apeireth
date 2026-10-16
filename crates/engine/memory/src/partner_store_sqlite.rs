//! SQLite-backed persistence for partner records.
//!
//! Companion to [`crate::partner::InMemoryPartnerStore`]: same [`PartnerStore`]
//! contract, but records survive restarts. Rows store the serde-JSON form of
//! [`Partner`] keyed by [`PartnerId`]; the schema is intentionally local and
//! idempotent so the store can be used before central migrations own it.

use std::sync::Arc;

use apeireth_storage::SqliteConnectionPool;
use rusqlite::{params, OptionalExtension};

use crate::partner::{Partner, PartnerId, PartnerStore};
use crate::MemoryError;

/// Durable partner store over the storage crate's connection pool.
///
/// Writes go through the serialized writer (`write_sync`); reads use pooled
/// reader connections. The trait surface is synchronous by contract.
#[derive(Clone)]
pub struct SqlitePartnerStore {
    pool: Arc<SqliteConnectionPool>,
}

impl SqlitePartnerStore {
    /// Creates a store from an owned connection pool.
    pub fn new(pool: SqliteConnectionPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Creates a store sharing an existing connection pool.
    pub fn from_arc(pool: Arc<SqliteConnectionPool>) -> Self {
        Self { pool }
    }

    /// Returns the underlying pool for diagnostics and wiring code.
    pub fn pool(&self) -> &SqliteConnectionPool {
        self.pool.as_ref()
    }

    /// Creates the partner table when central migrations have not run yet.
    /// Idempotent and harmless after bootstrap owns the schema.
    pub fn ensure_schema(&self) -> Result<(), MemoryError> {
        self.pool
            .write_sync(|conn| {
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS partner_records (
                        partner_id  TEXT PRIMARY KEY NOT NULL,
                        data        TEXT NOT NULL,
                        updated_at_ms INTEGER NOT NULL
                    );
                    "#,
                )?;
                Ok(())
            })
            .map_err(|err| MemoryError::Other(err.to_string()))
    }
}

impl PartnerStore for SqlitePartnerStore {
    fn save_partner(&self, partner: &Partner) -> Result<(), MemoryError> {
        let data = serde_json::to_string(partner)?;
        let partner_id = partner.id.0.clone();
        self.pool
            .write_sync(move |conn| {
                conn.execute(
                    r#"
                    INSERT INTO partner_records (partner_id, data, updated_at_ms)
                    VALUES (?1, ?2, ?3)
                    ON CONFLICT(partner_id) DO UPDATE SET
                        data = excluded.data,
                        updated_at_ms = excluded.updated_at_ms
                    "#,
                    params![partner_id, data, now_ms()],
                )?;
                Ok(())
            })
            .map_err(|err| MemoryError::Other(err.to_string()))?;
        Ok(())
    }

    fn get_partner(&self, id: &PartnerId) -> Result<Option<Partner>, MemoryError> {
        let partner_id = id.0.clone();
        let row: Option<String> = self
            .pool
            .read(move |conn| {
                Ok(conn
                    .query_row(
                        "SELECT data FROM partner_records WHERE partner_id = ?1",
                        params![partner_id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()?)
            })
            .map_err(|err| MemoryError::Other(err.to_string()))?;
        match row {
            Some(data) => Ok(Some(serde_json::from_str(&data)?)),
            None => Ok(None),
        }
    }

    fn list_partners(&self) -> Result<Vec<Partner>, MemoryError> {
        let rows: Vec<String> = self
            .pool
            .read(|conn| {
                let mut stmt =
                    conn.prepare("SELECT data FROM partner_records ORDER BY partner_id ASC")?;
                let mapped = stmt
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(mapped)
            })
            .map_err(|err| MemoryError::Other(err.to_string()))?;
        rows.into_iter()
            .map(|data| Ok(serde_json::from_str(&data)?))
            .collect()
    }
}

/// Wall-clock timestamp for bookkeeping rows only (not domain semantics).
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partner::PartnerPreferences;

    fn sample_partner(id: &str, display_name: &str) -> Partner {
        Partner::new(
            PartnerId(id.to_string()),
            display_name,
            PartnerPreferences::default(),
            1_700_000_000_000,
        )
    }

    async fn fresh() -> SqlitePartnerStore {
        let pool = SqliteConnectionPool::in_memory()
            .await
            .expect("in-memory pool");
        let store = SqlitePartnerStore::new(pool);
        store.ensure_schema().expect("ensure_schema");
        store
    }

    #[tokio::test]
    async fn save_get_roundtrip_survives_reopen_semantics() {
        let store = fresh().await;
        let partner = sample_partner("p-1", "小林");
        store.save_partner(&partner).expect("save");
        let loaded = store
            .get_partner(&PartnerId("p-1".to_string()))
            .expect("get")
            .expect("present");
        assert_eq!(loaded, partner);
    }

    #[tokio::test]
    async fn upsert_overwrites_and_list_orders_by_id() {
        let store = fresh().await;
        store
            .save_partner(&sample_partner("p-2", "旧名"))
            .expect("save");
        store
            .save_partner(&sample_partner("p-2", "新名"))
            .expect("upsert");
        store
            .save_partner(&sample_partner("p-1", "甲"))
            .expect("save");
        let all = store.list_partners().expect("list");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id.0, "p-1");
        assert_eq!(all[1].display_name, "新名");
    }

    #[tokio::test]
    async fn missing_partner_is_none_not_error() {
        let store = fresh().await;
        let got = store
            .get_partner(&PartnerId("absent".to_string()))
            .expect("get");
        assert!(got.is_none());
        assert!(store.list_partners().expect("list").is_empty());
    }
}
