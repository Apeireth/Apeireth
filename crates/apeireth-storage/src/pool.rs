//! SQLite connection pool and configuration.
//!
//! The pool uses a single serialized writer plus a reader pool. The writer is
//! a dedicated connection owned by a background thread; all mutations must go
//! through [`SqliteConnectionPool::write`]. Readers are short-lived closures
//! over the `r2d2` pool and should be used for queries only.

use std::path::{Path, PathBuf};
use std::time::Duration;

use r2d2::ManageConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot};

use crate::StorageError;

/// SQLite journal mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalMode {
    /// WAL mode: readers do not block a writer.
    Wal,
    /// Traditional rollback journal.
    Delete,
    /// Rollback journal retained and truncated.
    Truncate,
    /// Rollback journal retained but not truncated.
    Persist,
    /// In-memory journal.
    Memory,
    /// No journal.
    Off,
}

impl JournalMode {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Wal => "WAL",
            Self::Delete => "DELETE",
            Self::Truncate => "TRUNCATE",
            Self::Persist => "PERSIST",
            Self::Memory => "MEMORY",
            Self::Off => "OFF",
        }
    }
}

/// SQLite synchronous setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynchronousMode {
    /// No fsync; fastest, least safe.
    Off,
    /// Normal durability; good balance with WAL.
    Normal,
    /// Full durability.
    Full,
    /// Extra durability.
    Extra,
}

impl SynchronousMode {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Normal => "NORMAL",
            Self::Full => "FULL",
            Self::Extra => "EXTRA",
        }
    }
}

/// SQLite connection/pool configuration.
///
/// The defaults mirror the donor implementation: WAL, synchronous NORMAL,
/// foreign keys ON, a 5-second busy timeout, and a read pool of 10.
#[derive(Debug, Clone)]
pub struct SqliteConfig {
    /// Maximum number of connections in the reader pool.
    pub max_connections: u32,
    /// SQLite journal mode.
    pub journal_mode: JournalMode,
    /// SQLite synchronous mode.
    pub synchronous: SynchronousMode,
    /// Whether `PRAGMA foreign_keys = ON` is set on every connection.
    pub foreign_keys: bool,
    /// SQLite busy timeout.
    pub busy_timeout: Duration,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            journal_mode: JournalMode::Wal,
            synchronous: SynchronousMode::Normal,
            foreign_keys: true,
            busy_timeout: Duration::from_millis(5000),
        }
    }
}

impl SqliteConfig {
    /// Creates the default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum reader pool size.
    pub fn with_max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = max_connections;
        self
    }

    /// Sets the journal mode.
    pub fn with_journal_mode(mut self, journal_mode: JournalMode) -> Self {
        self.journal_mode = journal_mode;
        self
    }

    /// Sets the synchronous mode.
    pub fn with_synchronous(mut self, synchronous: SynchronousMode) -> Self {
        self.synchronous = synchronous;
        self
    }

    /// Enables or disables `PRAGMA foreign_keys`.
    pub fn with_foreign_keys(mut self, foreign_keys: bool) -> Self {
        self.foreign_keys = foreign_keys;
        self
    }

    /// Sets the SQLite busy timeout.
    pub fn with_busy_timeout(mut self, busy_timeout: Duration) -> Self {
        self.busy_timeout = busy_timeout;
        self
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.max_connections == 0 {
            return Err(StorageError::InvalidConfiguration(
                "max_connections must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }

    fn init_sql(&self) -> String {
        format!(
            "PRAGMA journal_mode = {};\nPRAGMA synchronous = {};\nPRAGMA foreign_keys = {};\nPRAGMA busy_timeout = {};",
            self.journal_mode.as_sql(),
            self.synchronous.as_sql(),
            if self.foreign_keys { "ON" } else { "OFF" },
            self.busy_timeout.as_millis()
        )
    }
}

type WriteTask = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

/// SQLite connection pool with one serialized writer and a reader pool.
#[derive(Clone)]
pub struct SqliteConnectionPool {
    pool: r2d2::Pool<SqliteConnectionManager>,
    write_tx: mpsc::Sender<WriteTask>,
}

impl std::fmt::Debug for SqliteConnectionPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteConnectionPool")
            .field("pool_state", &self.pool.state())
            .finish_non_exhaustive()
    }
}

impl SqliteConnectionPool {
    /// Opens a file-backed pool with default configuration.
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        Self::open_with_config(path, SqliteConfig::default()).await
    }

    /// Opens a file-backed pool with explicit configuration.
    pub async fn open_with_config<P: AsRef<Path>>(
        path: P,
        config: SqliteConfig,
    ) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        config.validate()?;

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| StorageError::Open {
                    path: path.clone(),
                    message: format!("failed to create parent directory: {e}"),
                })?;
            }
        }

        let init_sql = config.init_sql();
        let manager = SqliteConnectionManager::file(&path)
            .with_init(move |conn| conn.execute_batch(&init_sql));

        let writer = manager.connect().map_err(|e| StorageError::Open {
            path: path.clone(),
            message: e.to_string(),
        })?;

        let pool = r2d2::Pool::builder()
            .max_size(config.max_connections)
            .build(manager)
            .map_err(|e| StorageError::Open {
                path: path.clone(),
                message: e.to_string(),
            })?;

        Ok(Self::from_pool_and_writer(pool, writer))
    }

    /// Opens a shared in-memory pool.
    ///
    /// SQLite `:memory:` is per connection. This constructor uses a shared
    /// memory URI (through `r2d2_sqlite::SqliteConnectionManager::memory`) so
    /// the writer and every reader connection see the same database.
    pub async fn in_memory_with_config(config: SqliteConfig) -> Result<Self, StorageError> {
        config.validate()?;

        let init_sql = config.init_sql();
        let manager =
            SqliteConnectionManager::memory().with_init(move |conn| conn.execute_batch(&init_sql));

        let writer = manager.connect().map_err(|e| StorageError::Open {
            path: PathBuf::from(":memory:"),
            message: e.to_string(),
        })?;

        let pool = r2d2::Pool::builder()
            .max_size(config.max_connections)
            .build(manager)
            .map_err(|e| StorageError::Open {
                path: PathBuf::from(":memory:"),
                message: e.to_string(),
            })?;

        Ok(Self::from_pool_and_writer(pool, writer))
    }

    /// Opens a shared in-memory pool with default configuration.
    pub async fn in_memory() -> Result<Self, StorageError> {
        Self::in_memory_with_config(SqliteConfig::default()).await
    }

    /// Runs a short-lived read closure over a pooled reader connection.
    pub fn read<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let conn = self.pool.get()?;
        f(&conn)
    }

    /// Runs a mutation on the single serialized writer and returns its result.
    ///
    /// Writes are ordered by channel arrival and executed by exactly one
    /// background writer thread. A failure in `f` is returned to the caller
    /// and does not mark the writer connection broken for later writes.
    pub async fn write<F, R>(&self, f: F) -> Result<R, StorageError>
    where
        F: FnOnce(&mut Connection) -> Result<R, StorageError> + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let task: WriteTask = Box::new(move |conn: &mut Connection| {
            let result = f(conn);
            let _ = tx.send(result);
        });

        self.write_tx
            .send(task)
            .await
            .map_err(|_| StorageError::WriteQueue("writer channel is closed".to_string()))?;

        rx.await.map_err(|_| {
            StorageError::WriteQueue("writer task did not return a result".to_string())
        })?
    }

    fn from_pool_and_writer(
        pool: r2d2::Pool<SqliteConnectionManager>,
        mut writer: Connection,
    ) -> Self {
        let (write_tx, mut write_rx) = mpsc::channel::<WriteTask>(1000);

        std::thread::spawn(move || {
            while let Some(task) = write_rx.blocking_recv() {
                task(&mut writer);
            }
        });

        Self { pool, write_tx }
    }
}
