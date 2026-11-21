//! SQLite SessionStore adapter for the production assembly.

use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_runtime::canonical::{RuntimeError, RuntimeResult, Session, SessionStore};
use apeireth_storage::{run_migrations, SqliteConnectionPool, StorageError};
use async_trait::async_trait;

use crate::canonical::session_seal::SessionSealer;
use crate::startup::{StartupPhase, StartupTimeline};

const SEAL_PROBE_ROW: &str = "session-seal-state:singleton";
const SEAL_PROBE_TEXT: &[u8] = b"apeireth-session-seal-key-check/v1";
// `APSR2:` 必须与 `session_seal::ROW_SEAL_PREFIX` 同步；触发器让迁移前已打开的
// 无 sealer 句柄也无法在探针建立后重新写入明文。
const SEALED_WRITE_GUARDS_SQL: &str = r#"
CREATE TRIGGER IF NOT EXISTS apeireth_sessions_sealed_insert
BEFORE INSERT ON sessions
WHEN NEW.data IS NULL OR substr(NEW.data, 1, 6) <> 'APSR2:'
BEGIN SELECT RAISE(ABORT, 'protected sessions require APSR2'); END;
CREATE TRIGGER IF NOT EXISTS apeireth_sessions_sealed_update
BEFORE UPDATE OF data ON sessions
WHEN NEW.data IS NULL OR substr(NEW.data, 1, 6) <> 'APSR2:'
BEGIN SELECT RAISE(ABORT, 'protected sessions require APSR2'); END;
CREATE TRIGGER IF NOT EXISTS apeireth_evidence_sealed_insert
BEFORE INSERT ON module_operation_evidence
WHEN NEW.data IS NULL OR substr(NEW.data, 1, 6) <> 'APSR2:'
BEGIN SELECT RAISE(ABORT, 'protected evidence requires APSR2'); END;
"#;

/// Durable SQLite implementation of the kernel's SessionStore port.
pub struct SqliteSessionStore {
    /// 持 Arc 而非裸句柄: 共享池注册表按 Arc 存活判复用 (句柄组随本 Arc 保活)。
    pool: Arc<SqliteConnectionPool>,
    /// B6-2a 试点: `sessions.data` 列封缄器。`None` = 旧路径逐字节不变
    /// (开关 `APEIRETH_ENABLE_SESSION_SEAL` 默认关)。
    sealer: Option<Arc<SessionSealer>>,
}

impl SqliteSessionStore {
    /// Open a file-backed store and apply storage migrations.
    ///
    /// 打开走**进程内共享池** (同一路径重复打开复用同一组底层句柄, 见
    /// [`SqliteConnectionPool::open_shared`]); 迁移带版本检查短路 (已迁移库
    /// 不再跑迁移, 见 `apeireth_storage::run_migrations_report`)。
    pub async fn open(path: impl AsRef<std::path::Path>) -> RuntimeResult<Self> {
        Self::open_with_timeline(path, None).await
    }

    /// [`Self::open`] 的分段计时形态，打开语义相同。
    pub async fn open_with_timeline(
        path: impl AsRef<std::path::Path>,
        timeline: Option<&Arc<StartupTimeline>>,
    ) -> RuntimeResult<Self> {
        // 先解出密钥再触碰数据库：显式保护却缺钥时，不创建/迁移任何库文件。
        let sealer = crate::canonical::session_seal::sealer_from_env()
            .map_err(RuntimeError::session_store_open)?
            .map(Arc::new);
        Self::open_resolved(path.as_ref(), timeline, sealer).await
    }

    /// 以显式密钥打开并完成受保护启动迁移（嵌入方与合成迁移测试入口）。
    pub async fn open_protected(
        path: impl AsRef<std::path::Path>,
        sealer: SessionSealer,
    ) -> RuntimeResult<Self> {
        Self::open_resolved(path.as_ref(), None, Some(Arc::new(sealer))).await
    }

    async fn open_resolved(
        path: &std::path::Path,
        timeline: Option<&Arc<StartupTimeline>>,
        sealer: Option<Arc<SessionSealer>>,
    ) -> RuntimeResult<Self> {
        let pool = match timeline {
            Some(timeline) => {
                timeline
                    .measure_async(
                        StartupPhase::SessionPoolOpen,
                        SqliteConnectionPool::open_shared(path),
                    )
                    .await
            }
            None => SqliteConnectionPool::open_shared(path).await,
        }
        .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        let migrations = Self::prepare_pool(pool.as_ref(), sealer.clone());
        match timeline {
            Some(timeline) => {
                timeline
                    .measure_async(StartupPhase::SessionMigrations, migrations)
                    .await
            }
            None => migrations.await,
        }
        .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        Ok(Self { pool, sealer })
    }

    async fn prepare_pool(
        pool: &SqliteConnectionPool,
        sealer: Option<Arc<SessionSealer>>,
    ) -> Result<(), StorageError> {
        if sealer.is_none() {
            pool.read(|connection| {
                let protected: i64 = connection.query_row(
                    "SELECT EXISTS(
                         SELECT 1 FROM sqlite_master
                         WHERE type='table' AND name='apeireth_session_seal_state'
                     )",
                    [],
                    |row| row.get(0),
                )?;
                if protected != 0 {
                    return Err(StorageError::Serialization(
                        "会话库已建立封缄密钥探针，未提供密钥不得降级明文".to_string(),
                    ));
                }
                Ok(())
            })?;
        }
        pool.write(|connection| run_migrations(connection)).await?;
        if let Some(sealer) = sealer {
            pool.write(move |connection| migrate_protected_store(connection, &sealer))
                .await?;
        }
        Ok(())
    }

    /// 仅供空/已迁移测试 store 后置注入；嵌入方须用 [`Self::open_protected`]。
    #[must_use]
    pub fn with_sealer(mut self, sealer: SessionSealer) -> Self {
        self.sealer = Some(Arc::new(sealer));
        self
    }

    /// 当前是否启用封缄 (自述/测试读值)。
    pub fn sealing_enabled(&self) -> bool {
        self.sealer.is_some()
    }

    /// Open a shared in-memory SQLite store.
    pub async fn in_memory() -> RuntimeResult<Self> {
        let sealer = crate::canonical::session_seal::sealer_from_env()
            .map_err(RuntimeError::session_store_open)?
            .map(Arc::new);
        let pool = SqliteConnectionPool::in_memory()
            .await
            .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        Self::prepare_pool(&pool, sealer.clone())
            .await
            .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        Ok(Self {
            pool: Arc::new(pool),
            sealer,
        })
    }

    fn storage_error(
        session: SessionId,
        operation: &'static str,
        error: StorageError,
    ) -> RuntimeError {
        match operation {
            "load" => RuntimeError::session_load(session, error.to_string()),
            "delete" => RuntimeError::Session {
                session,
                operation: "deleted",
                reason: error.to_string(),
            },
            _ => RuntimeError::session_save(session, error.to_string()),
        }
    }
}

fn session_row_ref(id: &str) -> String {
    format!("sessions|{}:{id}", id.len())
}

fn evidence_row_ref(session_id: &str, operation_id: &str) -> String {
    format!(
        "module_operation_evidence|{}:{session_id}|{}:{operation_id}",
        session_id.len(),
        operation_id.len()
    )
}

fn open_session_row(
    sealer: &SessionSealer,
    id: &str,
    stored: &str,
) -> Result<Vec<u8>, StorageError> {
    let plaintext = sealer
        .open_row_text(&session_row_ref(id), stored)
        .map_err(StorageError::Serialization)?;
    let json = std::str::from_utf8(&plaintext)
        .map_err(|error| StorageError::Serialization(format!("session UTF-8 非法: {error}")))?;
    let session: Session = serde_json::from_str(json)
        .map_err(|error| StorageError::Serialization(format!("session JSON 非法: {error}")))?;
    if session.id.to_string() != id {
        return Err(StorageError::Serialization(
            "session 行 id 与载荷身份不符".to_string(),
        ));
    }
    Ok(plaintext)
}

fn open_evidence_row(
    sealer: &SessionSealer,
    session_id: &str,
    operation_id: &str,
    stored: &str,
) -> Result<Vec<u8>, StorageError> {
    let plaintext = sealer
        .open_row_text(&evidence_row_ref(session_id, operation_id), stored)
        .map_err(StorageError::Serialization)?;
    let json = std::str::from_utf8(&plaintext).map_err(|error| {
        StorageError::Serialization(format!("operation evidence UTF-8 非法: {error}"))
    })?;
    let value: apeireth_runtime::CommittedOperationIntent =
        serde_json::from_str(json).map_err(|error| {
            StorageError::Serialization(format!("operation evidence JSON 非法: {error}"))
        })?;
    let payload_hash =
        apeireth_runtime::canonical::committed_evidence::operation_payload_hash(&value.payload)
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
    if value.input.session.to_string() != session_id
        || value.operation_id != operation_id
        || value.payload_hash != payload_hash
    {
        return Err(StorageError::Serialization(
            "operation evidence 行身份或载荷摘要不符".to_string(),
        ));
    }
    Ok(plaintext)
}

fn reseal_legacy_row(
    sealer: &SessionSealer,
    row_ref: &str,
    stored: &str,
    plaintext: &[u8],
) -> Result<Option<String>, StorageError> {
    if SessionSealer::is_row_sealed(stored) {
        return Ok(None);
    }
    let replacement = sealer
        .seal_row_to_text(row_ref, plaintext)
        .map_err(StorageError::Serialization)?;
    let opened = sealer
        .open_row_text(row_ref, &replacement)
        .map_err(StorageError::Serialization)?;
    if opened != plaintext {
        return Err(StorageError::Serialization(
            "会话封缄迁移写前验证失败".to_string(),
        ));
    }
    Ok(Some(replacement))
}

fn verify_probe(sealer: &SessionSealer, stored: &str) -> Result<(), StorageError> {
    let opened = sealer
        .open_row_text(SEAL_PROBE_ROW, stored)
        .map_err(StorageError::Serialization)?;
    if opened != SEAL_PROBE_TEXT {
        return Err(StorageError::Serialization(
            "会话封缄密钥探针不符".to_string(),
        ));
    }
    Ok(())
}

/// IMMEDIATE 事务升级明文/v1；不承诺抹除 WAL、旧页或外部备份。
fn migrate_protected_store(
    connection: &mut rusqlite::Connection,
    sealer: &SessionSealer,
) -> Result<(), StorageError> {
    use rusqlite::OptionalExtension as _;

    let transaction =
        connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS apeireth_session_seal_state (
             singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
             probe TEXT NOT NULL
         );",
    )?;
    let probe: Option<String> = transaction
        .query_row(
            "SELECT probe FROM apeireth_session_seal_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(stored) = probe.as_deref() {
        verify_probe(sealer, stored)?;
    }

    let session_rows = {
        let mut statement = transaction.prepare("SELECT id, data FROM sessions ORDER BY id")?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    for (id, stored) in session_rows {
        if probe.is_some() && !SessionSealer::is_row_sealed(&stored) {
            return Err(StorageError::Serialization(
                "已建立密钥探针的会话库出现非 v2 行，拒绝静默再迁移".to_string(),
            ));
        }
        let plaintext = open_session_row(sealer, &id, &stored)?;
        if let Some(replacement) =
            reseal_legacy_row(sealer, &session_row_ref(&id), &stored, &plaintext)?
        {
            let affected = transaction.execute(
                "UPDATE sessions SET data = ?1 WHERE id = ?2 AND data = ?3",
                rusqlite::params![replacement, id, stored],
            )?;
            if affected != 1 {
                return Err(StorageError::Serialization(
                    "session 迁移期间行发生变化".to_string(),
                ));
            }
            let written: String =
                transaction.query_row("SELECT data FROM sessions WHERE id = ?1", [&id], |row| {
                    row.get(0)
                })?;
            if written != replacement {
                return Err(StorageError::Serialization(
                    "session 迁移后读回不一致".to_string(),
                ));
            }
            open_session_row(sealer, &id, &written)?;
        }
    }

    let evidence_rows = {
        let mut statement = transaction.prepare(
            "SELECT seq, session_id, operation_id, data
             FROM module_operation_evidence ORDER BY seq",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    let migrates_evidence = evidence_rows
        .iter()
        .any(|(_, _, _, stored)| !SessionSealer::is_row_sealed(stored));
    if probe.is_some() && migrates_evidence {
        return Err(StorageError::Serialization(
            "已建立密钥探针的证据账出现非 v2 行，拒绝静默再迁移".to_string(),
        ));
    }
    if migrates_evidence {
        transaction.execute_batch("DROP TRIGGER IF EXISTS module_operation_evidence_no_update;")?;
    }
    for (sequence, session_id, operation_id, stored) in evidence_rows {
        let plaintext = open_evidence_row(sealer, &session_id, &operation_id, &stored)?;
        if let Some(replacement) = reseal_legacy_row(
            sealer,
            &evidence_row_ref(&session_id, &operation_id),
            &stored,
            &plaintext,
        )? {
            let affected = transaction.execute(
                "UPDATE module_operation_evidence SET data = ?1 WHERE seq = ?2 AND data = ?3",
                rusqlite::params![replacement, sequence, stored],
            )?;
            if affected != 1 {
                return Err(StorageError::Serialization(
                    "operation evidence 迁移期间行发生变化".to_string(),
                ));
            }
            let written: String = transaction.query_row(
                "SELECT data FROM module_operation_evidence WHERE seq = ?1",
                [sequence],
                |row| row.get(0),
            )?;
            if written != replacement {
                return Err(StorageError::Serialization(
                    "operation evidence 迁移后读回不一致".to_string(),
                ));
            }
            open_evidence_row(sealer, &session_id, &operation_id, &written)?;
        }
    }
    if migrates_evidence {
        transaction.execute_batch(
            "CREATE TRIGGER module_operation_evidence_no_update
             BEFORE UPDATE ON module_operation_evidence
             BEGIN SELECT RAISE(ABORT, 'operation evidence is append-only'); END;",
        )?;
    }

    transaction.execute_batch(SEALED_WRITE_GUARDS_SQL)?;
    if probe.is_none() {
        let stored = sealer
            .seal_row_to_text(SEAL_PROBE_ROW, SEAL_PROBE_TEXT)
            .map_err(StorageError::Serialization)?;
        transaction.execute(
            "INSERT INTO apeireth_session_seal_state(singleton, probe) VALUES (1, ?1)",
            [stored],
        )?;
    }
    let stored: String = transaction.query_row(
        "SELECT probe FROM apeireth_session_seal_state WHERE singleton = 1",
        [],
        |row| row.get(0),
    )?;
    verify_probe(sealer, &stored)?;
    transaction.commit()?;
    Ok(())
}

#[async_trait]
impl SessionStore for SqliteSessionStore {
    fn supports_operation_evidence(&self) -> bool {
        true
    }

    async fn put_operation_evidence(
        &self,
        intent: &apeireth_runtime::CommittedOperationIntent,
    ) -> RuntimeResult<apeireth_runtime::CommittedOperationIntent> {
        if apeireth_runtime::canonical::committed_evidence::operation_payload_hash(&intent.payload)
            .map_err(|error| RuntimeError::session_save(intent.input.session, error.to_string()))?
            != intent.payload_hash
        {
            return Err(RuntimeError::session_save(
                intent.input.session,
                "operation evidence integrity mismatch",
            ));
        }
        let intent = intent.clone();
        let id = intent.input.session;
        let sealer = self.sealer.clone();
        self.pool.write(move |connection| {
            use rusqlite::OptionalExtension;
            let session_id = id.to_string();
            let row_ref = evidence_row_ref(&session_id, &intent.operation_id);
            let transaction = rusqlite::Transaction::new_unchecked(connection, rusqlite::TransactionBehavior::Immediate)?;
            let existing: Option<(u64,String)> = transaction.query_row(
                "SELECT seq,data FROM module_operation_evidence WHERE session_id=?1 AND operation_id=?2",
                rusqlite::params![session_id,intent.operation_id], |row|Ok((row.get(0)?,row.get(1)?))).optional()?;
            if let Some((sequence,data)) = existing {
                let json = match &sealer { Some(sealer)=>String::from_utf8(sealer.open_current_row_text(&row_ref,&data).map_err(StorageError::Serialization)?).map_err(|error|StorageError::Serialization(error.to_string()))?,None=>data };
                let mut old: apeireth_runtime::CommittedOperationIntent = serde_json::from_str(&json).map_err(|error|StorageError::Serialization(error.to_string()))?;
                if old.operation_id!=intent.operation_id || old.module_id!=intent.module_id || old.input!=intent.input || old.payload_hash!=intent.payload_hash || old.payload!=intent.payload {
                    return Err(StorageError::Serialization("operation evidence identity conflict".into()));
                }
                old.sequence=sequence; transaction.commit()?; return Ok(old);
            }
            let json=serde_json::to_string(&intent).map_err(|error|StorageError::Serialization(error.to_string()))?;
            let data=match &sealer {Some(sealer)=>sealer.seal_row_to_text(&row_ref,json.as_bytes()).map_err(StorageError::Serialization)?,None=>json};
            transaction.execute("INSERT INTO module_operation_evidence(session_id,operation_id,data) VALUES (?1,?2,?3)",rusqlite::params![session_id,intent.operation_id,data])?;
            let mut result=intent;result.sequence=transaction.last_insert_rowid() as u64;transaction.commit()?;Ok(result)
        }).await.map_err(|error|Self::storage_error(id,"save",error))
    }

    async fn operation_evidence(
        &self,
        id: &SessionId,
    ) -> RuntimeResult<Vec<apeireth_runtime::CommittedOperationIntent>> {
        let id = *id;
        let sealer = self.sealer.clone();
        self.pool.read(move |connection| {
            let session_id=id.to_string();
            let mut statement=connection.prepare("SELECT seq,operation_id,data FROM module_operation_evidence WHERE session_id=?1 ORDER BY seq")?;
            let rows=statement.query_map([&session_id],|row|Ok((row.get::<_,u64>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?)))?;
            let mut out=Vec::new();
            for row in rows {let(sequence,operation_id,data)=row?;
                let row_ref=evidence_row_ref(&session_id,&operation_id);
                let json=match &sealer {Some(sealer)=>String::from_utf8(sealer.open_current_row_text(&row_ref,&data).map_err(StorageError::Serialization)?).map_err(|error|StorageError::Serialization(error.to_string()))?,None=>data};
                let mut value:apeireth_runtime::CommittedOperationIntent=serde_json::from_str(&json).map_err(|error|StorageError::Serialization(error.to_string()))?;
                if value.input.session!=id || value.operation_id!=operation_id || apeireth_runtime::canonical::committed_evidence::operation_payload_hash(&value.payload).map_err(|error|StorageError::Serialization(error.to_string()))?!=value.payload_hash {
                    return Err(StorageError::Serialization("operation evidence integrity mismatch".into()));
                }
                value.sequence=sequence;out.push(value);
            } Ok(out)
        }).map_err(|error|Self::storage_error(id,"load",error))
    }
    async fn load(&self, id: &SessionId) -> RuntimeResult<Option<Session>> {
        use rusqlite::OptionalExtension;

        let id = *id;
        let sealer = self.sealer.clone();
        self.pool
            .read(move |connection| {
                let data: Option<String> = connection
                    .prepare("SELECT data FROM sessions WHERE id = ?1")?
                    .query_row([id.to_string()], |row| row.get(0))
                    .optional()?;
                let Some(stored) = data else { return Ok(None) };
                let json = match sealer.as_deref() {
                    Some(sealer) => sealer
                        .open_current_row_text(&session_row_ref(&id.to_string()), &stored)
                        .and_then(|bytes| {
                            String::from_utf8(bytes)
                                .map_err(|error| format!("session UTF-8 非法: {error}"))
                        })
                        .map_err(StorageError::Serialization)?,
                    None => stored,
                };
                let session: Session = serde_json::from_str(&json)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                if session.id != id {
                    return Err(StorageError::Serialization(
                        "session 行 id 与载荷身份不符".to_string(),
                    ));
                }
                Ok(Some(session))
            })
            .map_err(|error| Self::storage_error(id, "load", error))
    }

    async fn save(&self, session: &Session) -> RuntimeResult<()> {
        let id = session.id;
        let json = serde_json::to_string(session)
            .map_err(|error| RuntimeError::session_save(id, error.to_string()))?;
        // 封缄开写行绑定密文；关则保持原明文 JSON 路径。
        let data = match self.sealer.as_deref() {
            Some(sealer) => sealer
                .seal_row_to_text(&session_row_ref(&id.to_string()), json.as_bytes())
                .map_err(|error| RuntimeError::session_save(id, error))?,
            None => json,
        };
        self.pool
            .write(move |connection| {
                connection.execute(
                    "INSERT INTO sessions (id, data) VALUES (?1, ?2)
                     ON CONFLICT(id) DO UPDATE SET data = excluded.data",
                    rusqlite::params![id.to_string(), data],
                )?;
                Ok(())
            })
            .await
            .map_err(|error| Self::storage_error(id, "save", error))
    }

    async fn list(&self) -> RuntimeResult<Vec<Session>> {
        let sealer = self.sealer.clone();
        let mut sessions = self
            .pool
            .read(|connection| {
                let mut statement = connection.prepare("SELECT id, data FROM sessions")?;
                let rows = statement.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                rows.map(|row| {
                    let (id, stored) =
                        row.map_err(|error| StorageError::Serialization(error.to_string()))?;
                    let json = match sealer.as_deref() {
                        Some(sealer) => sealer
                            .open_current_row_text(&session_row_ref(&id), &stored)
                            .and_then(|bytes| {
                                String::from_utf8(bytes)
                                    .map_err(|error| format!("session UTF-8 非法: {error}"))
                            })
                            .map_err(StorageError::Serialization)?,
                        None => stored,
                    };
                    let session: Session = serde_json::from_str(&json)
                        .map_err(|error| StorageError::Serialization(error.to_string()))?;
                    if session.id.to_string() != id {
                        return Err(StorageError::Serialization(
                            "session 行 id 与载荷身份不符".to_string(),
                        ));
                    }
                    Ok(session)
                })
                .collect::<Result<Vec<_>, StorageError>>()
            })
            .map_err(|error| RuntimeError::session_store_open(format!("list: {error}")))?;
        sessions.sort_by(|left, right| {
            right
                .updated_at
                .epoch_millis()
                .cmp(&left.updated_at.epoch_millis())
        });
        Ok(sessions)
    }

    async fn delete(&self, id: &SessionId) -> RuntimeResult<bool> {
        let id = *id;
        self.pool
            .write(move |connection| {
                let transaction = rusqlite::Transaction::new_unchecked(
                    connection,
                    rusqlite::TransactionBehavior::Immediate,
                )?;
                transaction.execute(
                    "DELETE FROM module_operation_evidence WHERE session_id=?1",
                    [id.to_string()],
                )?;
                let affected = transaction.execute(
                    "DELETE FROM sessions WHERE id = ?1",
                    rusqlite::params![id.to_string()],
                )?;
                transaction.commit()?;
                Ok(affected > 0)
            })
            .await
            .map_err(|error| Self::storage_error(id, "delete", error))
    }

    /// 在册探测 (W3-③ 域事件过滤面): 只查存在性, 不载入会话 JSON。
    async fn contains(&self, id: &SessionId) -> RuntimeResult<bool> {
        use rusqlite::OptionalExtension;

        let id = *id;
        self.pool
            .read(move |connection| {
                let hit: Option<i64> = connection
                    .prepare("SELECT 1 FROM sessions WHERE id = ?1")?
                    .query_row([id.to_string()], |row| row.get(0))
                    .optional()?;
                Ok(hit.is_some())
            })
            .map_err(|error| Self::storage_error(id, "load", error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{system_clock, RequestId, Timestamp, TraceId};
    use apeireth_protocol::canonical::{ContentPart, NormalizedMessage};
    use apeireth_runtime::canonical::{PermissionPreset, Session, SessionSettings};
    use apeireth_runtime::{CommittedInputOrigin, CommittedOperationIntent, CommittedUserInput};
    use apeireth_storage::SqliteConnectionPool;

    /// 读 `sessions.data` 原始落盘值（测试专用：绕过 store 的解封口）。
    async fn raw_data(path: &std::path::Path, id: SessionId) -> String {
        let pool = SqliteConnectionPool::open(path).await.unwrap();
        pool.read(move |connection| {
            Ok(connection
                .prepare("SELECT data FROM sessions WHERE id = ?1")?
                .query_row([id.to_string()], |row| row.get::<_, String>(0))?)
        })
        .unwrap()
    }

    fn test_sealer() -> SessionSealer {
        sealer_with_byte(3)
    }

    fn sealer_with_byte(byte: u8) -> SessionSealer {
        use apeireth_storage::crypto::{PartitionId, PartitionKey};
        SessionSealer::new(PartitionKey::from_bytes(
            PartitionId::new("sessions").expect("partition"),
            [byte; 32],
        ))
    }

    fn seeded_session() -> Session {
        let clock = system_clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        session.append(NormalizedMessage::user("secret-hello"), clock.as_ref());
        session
    }

    fn seeded_intent(session: SessionId) -> CommittedOperationIntent {
        let payload = serde_json::json!({"secret":"legacy-operation-canary"});
        CommittedOperationIntent {
            input: CommittedUserInput {
                session,
                request: RequestId::new(),
                trace: TraceId::new(),
                principal: None,
                authorization_epoch: None,
                at: Timestamp::from_epoch_millis(42).unwrap(),
                messages: vec![NormalizedMessage::user("accepted input")],
                origin: CommittedInputOrigin::DirectUser,
            },
            operation_id: "legacy-operation".into(),
            module_id: "module.test".into(),
            payload_hash: apeireth_runtime::canonical::committed_evidence::operation_payload_hash(
                &payload,
            )
            .unwrap(),
            payload,
            prepared_at: Timestamp::from_epoch_millis(43).unwrap(),
            sequence: 0,
        }
    }

    #[tokio::test]
    async fn sealing_off_writes_plaintext_exactly_like_before() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plain.sqlite3");
        let store = SqliteSessionStore::open(&path).await.unwrap();
        assert!(!store.sealing_enabled(), "默认关");

        let session = seeded_session();
        let id = session.id;
        store.save(&session).await.unwrap();

        let raw = raw_data(&path, id).await;
        assert!(raw.starts_with('{'), "默认关必须是明文 JSON: {raw}");
        assert!(raw.contains("secret-hello"), "默认关明文里看得到内容");
        assert!(!SessionSealer::is_sealed(&raw), "默认关不得出现密文形态");
    }

    #[tokio::test]
    async fn sealing_on_round_trips_and_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sealed.sqlite3");

        let session = seeded_session();
        let id = session.id;
        {
            let store = SqliteSessionStore::open_protected(&path, test_sealer())
                .await
                .unwrap();
            assert!(store.sealing_enabled());
            store.save(&session).await.unwrap();

            let raw = raw_data(&path, id).await;
            assert!(SessionSealer::is_sealed(&raw), "开开关必须落密文: {raw}");
            assert!(!raw.contains("secret-hello"), "密文不得含明文子串");

            let loaded = store.load(&id).await.unwrap().expect("session");
            assert_eq!(loaded.id, id);
            let listed = store.list().await.unwrap();
            assert_eq!(listed.len(), 1);
        }

        let reopened = SqliteSessionStore::open_protected(&path, test_sealer())
            .await
            .unwrap();
        let loaded = reopened
            .load(&id)
            .await
            .unwrap()
            .expect("session after reopen");
        assert_eq!(loaded.id, id);
    }

    /// 显式保护启动在返回前迁移两张内容表，并恢复证据账 append-only 约束。
    #[tokio::test]
    async fn protected_startup_migrates_plaintext_session_and_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy-sealed.sqlite3");
        let session = seeded_session();
        let id = session.id;
        let plain = SqliteSessionStore::open(&path).await.unwrap();
        plain.save(&session).await.unwrap();
        let saved = plain
            .put_operation_evidence(&seeded_intent(id))
            .await
            .unwrap();
        drop(plain);

        let protected = SqliteSessionStore::open_protected(&path, test_sealer())
            .await
            .unwrap();
        let raw = raw_data(&path, id).await;
        assert!(SessionSealer::is_row_sealed(&raw));
        assert!(!raw.contains("secret-hello"));
        assert_eq!(protected.load(&id).await.unwrap().unwrap().id, id);
        assert_eq!(
            protected.operation_evidence(&id).await.unwrap(),
            vec![saved]
        );
        let evidence: String = protected
            .pool
            .read(|connection| {
                Ok(connection.query_row(
                    "SELECT data FROM module_operation_evidence",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        assert!(SessionSealer::is_row_sealed(&evidence));
        assert!(!evidence.contains("legacy-operation-canary"));
        assert!(protected
            .pool
            .write(|connection| {
                connection.execute("UPDATE module_operation_evidence SET data='changed'", [])?;
                Ok(())
            })
            .await
            .is_err());
    }

    #[tokio::test]
    async fn preexisting_plain_handle_cannot_downgrade_after_protection_commits() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dual-handle.sqlite3");
        let old_plain = SqliteSessionStore::open(&path).await.unwrap();
        let first = seeded_session();
        old_plain.save(&first).await.unwrap();

        let protected = SqliteSessionStore::open_protected(&path, test_sealer())
            .await
            .unwrap();
        let second = seeded_session();
        assert!(
            old_plain.save(&second).await.is_err(),
            "迁移前已打开的无密钥句柄不得再插入明文会话"
        );
        let intent = seeded_intent(first.id);
        assert!(
            old_plain.put_operation_evidence(&intent).await.is_err(),
            "迁移前已打开的无密钥句柄不得再追加明文证据"
        );
        assert!(!old_plain.contains(&second.id).await.unwrap());

        protected.save(&second).await.unwrap();
        protected.put_operation_evidence(&intent).await.unwrap();
        assert!(protected.contains(&second.id).await.unwrap());
        assert_eq!(
            protected.operation_evidence(&first.id).await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn wrong_key_rolls_back_mixed_legacy_migration_and_safe_retry_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed-legacy.sqlite3");
        let first = seeded_session();
        let second = seeded_session();
        let first_id = first.id;
        let second_id = second.id;
        let plain = SqliteSessionStore::open(&path).await.unwrap();
        plain.save(&first).await.unwrap();
        plain.save(&second).await.unwrap();
        let first_plain = raw_data(&path, first_id).await;
        let first_v1 = test_sealer()
            .seal_to_text(first_plain.as_bytes())
            .expect("legacy v1");
        plain
            .pool
            .write({
                let first_v1 = first_v1.clone();
                move |connection| {
                    connection.execute(
                        "UPDATE sessions SET data = ?1 WHERE id = ?2",
                        rusqlite::params![first_v1, first_id.to_string()],
                    )?;
                    Ok(())
                }
            })
            .await
            .unwrap();
        let second_plain = raw_data(&path, second_id).await;
        drop(plain);

        let error = SqliteSessionStore::open_protected(&path, sealer_with_byte(4))
            .await
            .err()
            .expect("wrong key must reject legacy ciphertext");
        assert!(error.to_string().contains("解封失败"), "{error}");
        assert_eq!(raw_data(&path, first_id).await, first_v1);
        assert_eq!(raw_data(&path, second_id).await, second_plain);
        let connection = rusqlite::Connection::open(&path).unwrap();
        let state_tables: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master
                 WHERE type='table' AND name='apeireth_session_seal_state'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state_tables, 0, "失败事务不得留下密钥探针表");
        drop(connection);

        let recovered = SqliteSessionStore::open_protected(&path, test_sealer())
            .await
            .expect("same store can retry with the correct key");
        assert!(recovered.load(&first_id).await.unwrap().is_some());
        assert!(recovered.load(&second_id).await.unwrap().is_some());
        assert!(SessionSealer::is_row_sealed(
            &raw_data(&path, first_id).await
        ));
        assert!(SessionSealer::is_row_sealed(
            &raw_data(&path, second_id).await
        ));
    }

    #[tokio::test]
    async fn protected_startup_rejects_identity_mismatch_without_partial_rewrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity-mismatch.sqlite3");
        let first = seeded_session();
        let second = seeded_session();
        let first_id = first.id;
        let second_id = second.id;
        let plain = SqliteSessionStore::open(&path).await.unwrap();
        plain.save(&first).await.unwrap();
        plain.save(&second).await.unwrap();
        let first_raw = raw_data(&path, first_id).await;
        let second_raw = raw_data(&path, second_id).await;
        plain
            .pool
            .write({
                let first_raw = first_raw.clone();
                move |connection| {
                    connection.execute(
                        "UPDATE sessions SET data = ?1 WHERE id = ?2",
                        rusqlite::params![first_raw, second_id.to_string()],
                    )?;
                    Ok(())
                }
            })
            .await
            .unwrap();
        drop(plain);

        let error = SqliteSessionStore::open_protected(&path, test_sealer())
            .await
            .err()
            .expect("payload identity mismatch must abort startup");
        assert!(error.to_string().contains("身份不符"), "{error}");
        assert_eq!(raw_data(&path, first_id).await, first_raw);
        assert_eq!(raw_data(&path, second_id).await, first_raw);
        assert_ne!(first_raw, second_raw, "fixtures should be distinct");
    }

    #[tokio::test]
    async fn persisted_probe_rejects_missing_or_wrong_key_for_an_empty_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty-probe.sqlite3");
        SqliteSessionStore::open_protected(&path, test_sealer())
            .await
            .unwrap();
        let missing = SqliteSessionStore::open_resolved(&path, None, None)
            .await
            .err()
            .expect("protected store must not silently reopen in plaintext mode");
        assert!(missing.to_string().contains("不得降级明文"), "{missing}");
        let error = SqliteSessionStore::open_protected(&path, sealer_with_byte(5))
            .await
            .err()
            .expect("probe must pin the established key");
        assert!(error.to_string().contains("解封失败"), "{error}");
        SqliteSessionStore::open_protected(&path, test_sealer())
            .await
            .expect("correct key remains usable after rejection");
    }

    #[tokio::test]
    async fn established_probe_refuses_a_later_plaintext_row_instead_of_hiding_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("late-plaintext.sqlite3");
        let session = seeded_session();
        let id = session.id;
        let protected = SqliteSessionStore::open_protected(&path, test_sealer())
            .await
            .unwrap();
        protected.save(&session).await.unwrap();
        let plaintext = serde_json::to_string(&session).unwrap();
        protected
            .pool
            .write(move |connection| {
                connection
                    .execute_batch("DROP TRIGGER IF EXISTS apeireth_sessions_sealed_update;")?;
                connection.execute(
                    "UPDATE sessions SET data = ?1 WHERE id = ?2",
                    rusqlite::params![plaintext, id.to_string()],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        assert!(protected.load(&id).await.is_err(), "在线读也必须拒绝降级行");
        drop(protected);

        let error = SqliteSessionStore::open_protected(&path, test_sealer())
            .await
            .err()
            .expect("established protection must expose the downgrade");
        assert!(error.to_string().contains("非 v2 行"), "{error}");
        let raw = raw_data(&path, id).await;
        assert!(raw.starts_with('{'), "拒绝时不得顺手重写证据: {raw}");
    }

    #[tokio::test]
    async fn row_bound_ciphertext_rejects_swap_and_tamper_inside_sqlite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("row-swap.sqlite3");
        let first = seeded_session();
        let second = seeded_session();
        let first_id = first.id;
        let second_id = second.id;
        let store = SqliteSessionStore::open_protected(&path, test_sealer())
            .await
            .unwrap();
        store.save(&first).await.unwrap();
        store.save(&second).await.unwrap();
        let first_raw = raw_data(&path, first_id).await;
        let second_raw = raw_data(&path, second_id).await;
        store
            .pool
            .write({
                let first_raw = first_raw.clone();
                let second_raw = second_raw.clone();
                move |connection| {
                    connection
                        .execute_batch("DROP TRIGGER IF EXISTS apeireth_sessions_sealed_update;")?;
                    connection.execute(
                        "UPDATE sessions SET data = CASE id
                         WHEN ?1 THEN ?2 WHEN ?3 THEN ?4 ELSE data END
                     WHERE id IN (?1, ?3)",
                        rusqlite::params![
                            first_id.to_string(),
                            second_raw,
                            second_id.to_string(),
                            first_raw
                        ],
                    )?;
                    Ok(())
                }
            })
            .await
            .unwrap();
        assert!(store.load(&first_id).await.is_err());
        assert!(store.load(&second_id).await.is_err());
        let mut tampered = first_raw;
        let last = tampered.pop().expect("non-empty");
        tampered.push(if last == '0' { '1' } else { '0' });
        store
            .pool
            .write(move |connection| {
                connection.execute(
                    "UPDATE sessions SET data = CASE id
                         WHEN ?1 THEN ?2 WHEN ?3 THEN ?4 ELSE data END
                     WHERE id IN (?1, ?3)",
                    rusqlite::params![
                        first_id.to_string(),
                        tampered,
                        second_id.to_string(),
                        second_raw
                    ],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let error = store.load(&first_id).await.expect_err("篡改必须报错");
        assert!(
            error.to_string().contains("解封失败"),
            "错误必须指名解封失败: {error}"
        );
        assert!(store.load(&second_id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn legacy_session_json_without_settings_opens_with_defaults_and_no_data_loss() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.sqlite3");

        // Simulate a pre-settings database: create only the `sessions` table
        // (as migration v1 did) and store a session JSON with no `settings`
        // key — the on-disk shape this feature must migrate from.
        let pool = SqliteConnectionPool::open(&path).await.unwrap();
        let clock = system_clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        session.append(NormalizedMessage::user("hello"), clock.as_ref());
        let session_id = session.id;
        let mut legacy = serde_json::to_value(&session).unwrap();
        legacy
            .as_object_mut()
            .expect("session serializes as object")
            .remove("settings");
        let data = serde_json::to_string(&legacy).unwrap();

        pool.write(move |conn| {
            conn.execute_batch("CREATE TABLE sessions (id TEXT PRIMARY KEY, data TEXT);")?;
            conn.execute(
                "INSERT INTO sessions (id, data) VALUES (?1, ?2)",
                rusqlite::params![session_id.to_string(), data],
            )?;
            Ok(())
        })
        .await
        .unwrap();
        drop(pool);

        // Reopen through the production store. `run_migrations` is idempotent
        // against the existing table and must not drop or rewrite the row.
        let store = SqliteSessionStore::open(&path).await.unwrap();
        let loaded = store.load(&session_id).await.unwrap().unwrap();

        assert_eq!(loaded.settings, SessionSettings::default());
        assert_eq!(loaded.settings.model, None);
        assert_eq!(
            loaded.settings.permission_preset,
            PermissionPreset::Standard
        );
        assert_eq!(
            loaded.messages.len(),
            1,
            "transcript must survive migration"
        );
        assert_eq!(ContentPart::join_text(&loaded.messages[0].content), "hello");
    }

    #[tokio::test]
    async fn delete_removes_the_row_and_reports_whether_it_existed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.sqlite3");
        let store = SqliteSessionStore::open(&path).await.unwrap();
        let clock = system_clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        session.append(NormalizedMessage::user("hello"), clock.as_ref());
        let session_id = session.id;
        store.save(&session).await.unwrap();

        assert!(store.delete(&session_id).await.unwrap());
        assert!(store.load(&session_id).await.unwrap().is_none());
        assert!(store.list().await.unwrap().is_empty());
        assert!(
            !store.delete(&session_id).await.unwrap(),
            "a second delete must report no removal"
        );
    }

    #[tokio::test]
    async fn session_settings_roundtrip_preserves_each_preset() {
        // Select a tier → save → load: every tier (and each switch between
        // tiers) must read back as itself, with no field distorted.
        let store = SqliteSessionStore::in_memory().await.unwrap();
        let clock = system_clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        session.append(NormalizedMessage::user("hello"), clock.as_ref());
        session.settings.model = Some("some/model".into());
        let session_id = session.id;

        for (preset, remember) in [
            (PermissionPreset::ReadOnly, true),
            (PermissionPreset::Standard, false),
            (PermissionPreset::Full, true),
        ] {
            session.settings.permission_preset = preset;
            session.settings.approval_remember = remember;
            store.save(&session).await.unwrap();

            let loaded = store.load(&session_id).await.unwrap().unwrap();
            assert_eq!(loaded.settings.permission_preset, preset);
            assert_eq!(loaded.settings.approval_remember, remember);
            assert_eq!(loaded.settings.model.as_deref(), Some("some/model"));
            session = loaded;
        }
    }

    #[tokio::test]
    async fn session_settings_survive_store_reopen_across_preset_changes() {
        // Reopening the file store stands in for a process restart: the
        // persisted tier must be restored as itself, and a later tier switch
        // must survive another reopen the same way.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sessions.sqlite3");
        let clock = system_clock();
        let session_id = SessionId::new();

        let store = SqliteSessionStore::open(&path).await.unwrap();
        let mut session = Session::new(session_id, clock.as_ref());
        session.settings.permission_preset = PermissionPreset::ReadOnly;
        session.settings.approval_remember = true;
        store.save(&session).await.unwrap();
        drop(store);

        let reopened = SqliteSessionStore::open(&path).await.unwrap();
        let mut loaded = reopened.load(&session_id).await.unwrap().unwrap();
        assert_eq!(
            loaded.settings.permission_preset,
            PermissionPreset::ReadOnly
        );
        assert!(loaded.settings.approval_remember);

        loaded.settings.permission_preset = PermissionPreset::Full;
        loaded.settings.approval_remember = false;
        reopened.save(&loaded).await.unwrap();
        drop(reopened);

        let reopened_again = SqliteSessionStore::open(&path).await.unwrap();
        let loaded = reopened_again.load(&session_id).await.unwrap().unwrap();
        assert_eq!(loaded.settings.permission_preset, PermissionPreset::Full);
        assert!(
            !loaded.settings.approval_remember,
            "the switched tier must win over both the default and the previous tier"
        );
    }
}
