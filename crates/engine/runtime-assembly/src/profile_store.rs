//! Host-only durable profile configuration; no model-facing raw configuration API.
use apeireth_core::kernel::ProfileId;
use apeireth_plugin::agents::{AgentError, AgentErrorCode, AgentResult};
use apeireth_runtime::canonical::profiles::{ProfileConfig, ProfileRecord, ProfileStore};
use apeireth_storage::SqliteConnectionPool;
use async_trait::async_trait;
use rusqlite::OptionalExtension;
use std::sync::Arc;

pub struct SqliteProfileStore {
    pool: Arc<SqliteConnectionPool>,
}
fn failure(error: impl ToString) -> AgentError {
    AgentError::new(AgentErrorCode::InvalidConfig, error.to_string())
}
impl SqliteProfileStore {
    pub async fn open(path: impl AsRef<std::path::Path>) -> AgentResult<Self> {
        let pool = SqliteConnectionPool::open_shared(path)
            .await
            .map_err(failure)?;
        Self::from_pool(pool).await
    }
    pub async fn from_pool(pool: Arc<SqliteConnectionPool>) -> AgentResult<Self> {
        pool.write(|connection| {connection.execute_batch("CREATE TABLE IF NOT EXISTS agent_profiles_v1 (id TEXT PRIMARY KEY, revision INTEGER NOT NULL CHECK(revision>0), data TEXT NOT NULL)")?; Ok(())}).await.map_err(failure)?;
        Ok(Self { pool })
    }
}
#[async_trait]
impl ProfileStore for SqliteProfileStore {
    async fn load(&self, id: ProfileId) -> AgentResult<Option<ProfileRecord>> {
        let id = id.to_string();
        let pool = self.pool.clone();
        let data: Option<String> = tokio::task::spawn_blocking(move || {
            pool.read(move |c| {
                Ok(c.query_row(
                    "SELECT data FROM agent_profiles_v1 WHERE id=?1",
                    [id],
                    |row| row.get(0),
                )
                .optional()?)
            })
        })
        .await
        .map_err(failure)?
        .map_err(failure)?;
        data.map(|text| serde_json::from_str(&text).map_err(failure))
            .transpose()
    }
    async fn save(
        &self,
        config: ProfileConfig,
        expected: Option<u64>,
    ) -> AgentResult<ProfileRecord> {
        config.agent_config()?;
        let revision = expected
            .unwrap_or(0)
            .checked_add(1)
            .filter(|r| *r <= i64::MAX as u64)
            .ok_or_else(|| failure("profile revision overflow"))?;
        let record = ProfileRecord { config, revision };
        let data = serde_json::to_string(&record).map_err(failure)?;
        let id = record.config.id.to_string();
        let changed=self.pool.write(move|c|{
            Ok(if let Some(expected)=expected {c.execute("UPDATE agent_profiles_v1 SET revision=?1,data=?2 WHERE id=?3 AND revision=?4",rusqlite::params![revision as i64,data,id,expected as i64])?} else {c.execute("INSERT OR IGNORE INTO agent_profiles_v1(id,revision,data) VALUES(?1,?2,?3)",rusqlite::params![id,revision as i64,data])?})
        }).await.map_err(failure)?;
        if changed != 1 {
            return Err(failure("profile revision conflict"));
        }
        Ok(record)
    }
    async fn list(&self) -> AgentResult<Vec<ProfileRecord>> {
        let pool = self.pool.clone();
        let data: Vec<String> = tokio::task::spawn_blocking(move || {
            pool.read(|c| {
                let mut stmt = c.prepare("SELECT data FROM agent_profiles_v1 ORDER BY id")?;
                let rows = stmt
                    .query_map([], |r| r.get(0))?
                    .collect::<Result<Vec<String>, _>>()?;
                Ok(rows)
            })
        })
        .await
        .map_err(failure)?
        .map_err(failure)?;
        data.into_iter()
            .map(|text| serde_json::from_str(&text).map_err(failure))
            .collect()
    }
}
