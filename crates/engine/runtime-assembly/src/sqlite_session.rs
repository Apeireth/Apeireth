//! SQLite SessionStore adapter for the production assembly.

use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_runtime::canonical::{RuntimeError, RuntimeResult, Session, SessionStore};
use apeireth_storage::{run_migrations, SqliteConnectionPool, StorageError};
use async_trait::async_trait;

use crate::startup::{StartupPhase, StartupTimeline};

/// Durable SQLite implementation of the kernel's SessionStore port.
pub struct SqliteSessionStore {
    /// 持 Arc 而非裸句柄: 共享池注册表按 Arc 存活判复用 (句柄组随本 Arc 保活)。
    pool: Arc<SqliteConnectionPool>,
    /// B6-2a 试点: `sessions.data` 列封缄器。`None` = 旧路径逐字节不变
    /// (开关 `APEIRETH_ENABLE_SESSION_SEAL` 默认关)。
    sealer: Option<Arc<crate::canonical::session_seal::SessionSealer>>,
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

    /// [`Self::open`] 的分段计时形态 (B-P1): 注入 [`StartupTimeline`] 后
    /// 逐段落账 `SessionPoolOpen` (连接池建立) 与 `SessionMigrations` (迁移),
    /// 打开语义与 [`Self::open`] 完全一致。
    pub async fn open_with_timeline(
        path: impl AsRef<std::path::Path>,
        timeline: Option<&Arc<StartupTimeline>>,
    ) -> RuntimeResult<Self> {
        let path = path.as_ref();
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
        let migrations = pool.write(|connection| run_migrations(connection));
        match timeline {
            Some(timeline) => {
                timeline
                    .measure_async(StartupPhase::SessionMigrations, migrations)
                    .await
            }
            None => migrations.await,
        }
        .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        // B6-2a: 开关关 = 不读密钥文件 (零变化); 开关开 + 密钥文件缺失/坏 =
        // 启动失败 (fail-closed, 绝不降级明文)。
        let sealer = crate::canonical::session_seal::sealer_from_env()
            .map_err(RuntimeError::session_store_open)?
            .map(Arc::new);
        Ok(Self { pool, sealer })
    }

    /// 显式注入封缄器 (测试与嵌入方用; 生产走 [`Self::open`] 的 env 装配)。
    #[must_use]
    pub fn with_sealer(mut self, sealer: crate::canonical::session_seal::SessionSealer) -> Self {
        self.sealer = Some(Arc::new(sealer));
        self
    }

    /// 当前是否启用封缄 (自述/测试读值)。
    pub fn sealing_enabled(&self) -> bool {
        self.sealer.is_some()
    }

    /// Open a shared in-memory SQLite store.
    pub async fn in_memory() -> RuntimeResult<Self> {
        let pool = SqliteConnectionPool::in_memory()
            .await
            .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        pool.write(|connection| run_migrations(connection))
            .await
            .map_err(|error| RuntimeError::session_store_open(error.to_string()))?;
        let sealer = crate::canonical::session_seal::sealer_from_env()
            .map_err(RuntimeError::session_store_open)?
            .map(Arc::new);
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

#[async_trait]
impl SessionStore for SqliteSessionStore {
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
                // 封缄开: 密文行解封, 明文旧行原样读 (过渡兼容); 解封失败 = 硬错误。
                let json = match sealer.as_deref() {
                    Some(sealer) => sealer
                        .open_text(&stored)
                        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                        .map_err(StorageError::Serialization)?,
                    None => stored,
                };
                serde_json::from_str::<Session>(&json)
                    .map(Some)
                    .map_err(|error| StorageError::Serialization(error.to_string()))
            })
            .map_err(|error| Self::storage_error(id, "load", error))
    }

    async fn save(&self, session: &Session) -> RuntimeResult<()> {
        let id = session.id;
        let json = serde_json::to_string(session)
            .map_err(|error| RuntimeError::session_save(id, error.to_string()))?;
        // 封缄开: 落盘 = hex 密文 (与 storage `SqliteFieldSink` 产物同格式);
        // 关: 原样明文 JSON, 逐字节同旧路径。
        let data = match self.sealer.as_deref() {
            Some(sealer) => sealer
                .seal_to_text(json.as_bytes())
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
                let mut statement = connection.prepare("SELECT data FROM sessions")?;
                let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
                rows.map(|row| {
                    let stored =
                        row.map_err(|error| StorageError::Serialization(error.to_string()))?;
                    let json = match sealer.as_deref() {
                        Some(sealer) => sealer
                            .open_text(&stored)
                            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                            .map_err(StorageError::Serialization)?,
                        None => stored,
                    };
                    serde_json::from_str::<Session>(&json)
                        .map_err(|error| StorageError::Serialization(error.to_string()))
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
                let affected = connection.execute(
                    "DELETE FROM sessions WHERE id = ?1",
                    rusqlite::params![id.to_string()],
                )?;
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
    use apeireth_core::kernel::system_clock;
    use apeireth_protocol::canonical::{ContentPart, NormalizedMessage};
    use apeireth_runtime::canonical::{PermissionPreset, Session, SessionSettings};
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

    fn test_sealer() -> crate::canonical::session_seal::SessionSealer {
        use apeireth_storage::crypto::{PartitionId, PartitionKey};
        crate::canonical::session_seal::SessionSealer::new(PartitionKey::from_bytes(
            PartitionId::new("sessions").expect("partition"),
            [3u8; 32],
        ))
    }

    fn seeded_session() -> Session {
        let clock = system_clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        session.append(NormalizedMessage::user("secret-hello"), clock.as_ref());
        session
    }

    /// 开关关 = 落盘逐字节同旧路径（明文 JSON，无密文魔数）。
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
        assert!(
            !crate::canonical::session_seal::SessionSealer::is_sealed(&raw),
            "默认关不得出现密文形态"
        );
    }

    /// 开关开 = 落盘密文；往返、重启后读、列表读都通。
    #[tokio::test]
    async fn sealing_on_round_trips_and_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sealed.sqlite3");

        let session = seeded_session();
        let id = session.id;
        {
            let store = SqliteSessionStore::open(&path)
                .await
                .unwrap()
                .with_sealer(test_sealer());
            assert!(store.sealing_enabled());
            store.save(&session).await.unwrap();

            let raw = raw_data(&path, id).await;
            assert!(
                crate::canonical::session_seal::SessionSealer::is_sealed(&raw),
                "开开关必须落密文: {raw}"
            );
            assert!(!raw.contains("secret-hello"), "密文不得含明文子串");

            let loaded = store.load(&id).await.unwrap().expect("session");
            assert_eq!(loaded.id, id);
            let listed = store.list().await.unwrap();
            assert_eq!(listed.len(), 1);
        }

        // 重启（新 store + 同密钥）仍读得出。
        let reopened = SqliteSessionStore::open(&path)
            .await
            .unwrap()
            .with_sealer(test_sealer());
        let loaded = reopened
            .load(&id)
            .await
            .unwrap()
            .expect("session after reopen");
        assert_eq!(loaded.id, id);
    }

    /// 过渡兼容：B6-2a 之前落盘的明文行，开开关后照读（不猜编码、不丢数据）。
    #[tokio::test]
    async fn plaintext_legacy_row_stays_readable_with_sealing_on() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy-sealed.sqlite3");

        let session = seeded_session();
        let id = session.id;
        {
            let plain = SqliteSessionStore::open(&path).await.unwrap();
            plain.save(&session).await.unwrap();
        }
        let sealed = SqliteSessionStore::open(&path)
            .await
            .unwrap()
            .with_sealer(test_sealer());
        let loaded = sealed
            .load(&id)
            .await
            .unwrap()
            .expect("legacy row readable");
        assert_eq!(loaded.id, id);

        // 再写一次 = 该行升级为密文（惰性迁移）。
        sealed.save(&loaded).await.unwrap();
        let raw = raw_data(&path, id).await;
        assert!(crate::canonical::session_seal::SessionSealer::is_sealed(
            &raw
        ));
    }

    /// 篡改密文 = 显式错误，绝不静默退回明文或空会话。
    #[tokio::test]
    async fn tampered_ciphertext_is_an_explicit_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tampered.sqlite3");

        let session = seeded_session();
        let id = session.id;
        let store = SqliteSessionStore::open(&path)
            .await
            .unwrap()
            .with_sealer(test_sealer());
        store.save(&session).await.unwrap();

        // 把密文尾部一位翻掉（形态仍合法，AEAD 校验必失败）。
        let mut raw = raw_data(&path, id).await;
        let last = raw.pop().expect("non-empty");
        raw.push(if last == '0' { '1' } else { '0' });
        store
            .pool
            .write(move |connection| {
                connection
                    .execute(
                        "UPDATE sessions SET data = ?1 WHERE id = ?2",
                        rusqlite::params![raw, id.to_string()],
                    )
                    .map(|_| ())
                    .map_err(|error| StorageError::Serialization(error.to_string()))
            })
            .await
            .unwrap();

        let error = store.load(&id).await.expect_err("篡改必须报错");
        assert!(
            error.to_string().contains("解封失败"),
            "错误必须指名解封失败: {error}"
        );
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
