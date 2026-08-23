use crate::StorageError;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tokio::sync::{mpsc, oneshot};

type WriteTask = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

#[derive(Clone)]
pub struct SqliteConnectionPool {
    pool: r2d2::Pool<SqliteConnectionManager>,
    write_tx: mpsc::Sender<WriteTask>,
}

impl SqliteConnectionPool {
    /// Resolves standard platform-specific application data directory for Apeireth:
    /// - Windows: %APPDATA%\apeireth\memory.sqlite
    /// - macOS: ~/Library/Application Support/apeireth/memory.sqlite
    /// - Linux: ~/.local/share/apeireth/memory.sqlite
    pub fn default_platform_db_path() -> PathBuf {
        let base_dir = if cfg!(target_os = "windows") {
            std::env::var("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
        } else if cfg!(target_os = "macos") {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
                .unwrap_or_else(|_| PathBuf::from("."))
        } else {
            // Linux & other Unix
            std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
                .unwrap_or_else(|_| PathBuf::from("."))
        };

        base_dir.join("apeireth").join("memory.sqlite")
    }

    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let manager = SqliteConnectionManager::file(p)
            .with_init(|c| {
                c.execute_batch(
                    "PRAGMA journal_mode = WAL;
                     PRAGMA synchronous = NORMAL;
                     PRAGMA foreign_keys = ON;
                     PRAGMA busy_timeout = 5000;"
                )
            });

        let pool = r2d2::Pool::builder().max_size(10).build(manager)?;
        
        let (write_tx, mut write_rx) = mpsc::channel::<WriteTask>(1000);
        
        let mut write_conn = pool.get()?;
        std::thread::spawn(move || {
            while let Some(task) = write_rx.blocking_recv() {
                task(&mut write_conn);
            }
        });
        
        Ok(Self { pool, write_tx })
    }

    pub fn get_reader(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>, StorageError> {
        Ok(self.pool.get()?)
    }

    pub async fn write<F, R>(&self, f: F) -> Result<R, StorageError>
    where
        F: FnOnce(&mut Connection) -> Result<R, StorageError> + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let task = Box::new(move |conn: &mut Connection| {
            let res = f(conn);
            let _ = tx.send(res);
        });
        
        self.write_tx.send(task).await.map_err(|_| StorageError::WriteQueue)?;
        rx.await.map_err(|_| StorageError::WriteQueue)?
    }
}
