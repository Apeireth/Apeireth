//! SQLite connection pool and configuration.
//!
//! The pool uses a single serialized writer plus a reader pool. The writer is
//! a dedicated connection owned by a background thread; all mutations must go
//! through [`SqliteConnectionPool::write`]. Readers are short-lived closures
//! over the `r2d2` pool and should be used for queries only.
//!
//! **写线程同步收口**: 池被弃用 (最后一个引用释放) 时, 写线程**同步**退出并
//! 关闭写连接 ⇒「池已弃」=「写连接已关」。旧实现只丢 mpsc `Sender`, 写线程
//! **异步**醒来才关连接 ⇒「池已弃」与「连接已关」之间留了一个窗口 ⇒ 同路径
//! reopen 撞上 WAL 索引重建的排他锁 ⇒ `database is locked` (SQLITE_BUSY)。
//! 收口是**有界**的 (见 [`WRITER_SHUTDOWN_GRACE`]), 不会让 `Drop` 无限期阻塞。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex as StdMutex, OnceLock, Weak};
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
/// The defaults mirror the baseline implementation: WAL, synchronous NORMAL,
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

type WriteTaskBody = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

/// 单个写任务 + panic 标志.
///
/// M8: `body` panic 时, 其捕获的回复端随 unwind 一起被丢弃, 调用方只能看到
/// "writer task did not return a result" — 无法区分根因是 panic 还是其它.
/// `panicked` 由 body 内的 [`ReplyGuard`] 在 panic 路径置位 (unwind 时 guard
/// 先于回复端被 drop, 保证调用方读到标志时它一定已置位), 调用方据此把根因
/// 报成 `WriteQueue("writer task panicked")`.
struct WriteTask {
    body: WriteTaskBody,
    panicked: Arc<AtomicBool>,
}

/// body 完成守卫: 只有 panic 路径会留下 `completed == false`.
struct ReplyGuard {
    completed: bool,
    panicked: Arc<AtomicBool>,
}

impl Drop for ReplyGuard {
    fn drop(&mut self) {
        if !self.completed {
            self.panicked.store(true, Ordering::SeqCst);
        }
    }
}

/// 写线程收口等待的**额外**余量 (叠加在配置的 `busy_timeout` 之上)。
///
/// 收口等待必须**有界**: 写线程有可能正卡在一次长事务 / IO 上, `Drop`
/// 不得无限期等一个可能在等 IO 的线程。超时 ⇒ 放弃 join (分离写线程), 它
/// 退出时写连接照常关闭。
const WRITER_SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

/// SQLITE_BUSY 退避重试: 首次等待时长。
const BUSY_RETRY_INITIAL_DELAY: Duration = Duration::from_millis(5);
/// SQLITE_BUSY 退避重试: 单次等待上限 (指数退避封顶)。
const BUSY_RETRY_MAX_DELAY: Duration = Duration::from_millis(250);
/// SQLITE_BUSY 退避重试: 最多尝试次数 (含首次)。
const BUSY_RETRY_ATTEMPTS: u32 = 8;

/// 写线程句柄: 池弃用时**同步收口**写线程 ⇒「池已弃」=「写连接已关」。
///
/// 取案 (2026-11-06, 三树同窗取证): 旧实现 `drop(pool)` 只丢 mpsc `Sender`
/// ⇒ 写线程**异步**醒来才退出并关写连接, `drop` **立即返回** ⇒「池已弃」与
/// 「连接已关」之间存在窗口 ⇒ 同路径 reopen 时 WAL 索引重建要取**排他锁**
/// (SQLite 的 WAL 恢复路径直接取排他锁, **不走** busy handler ⇒ `busy_timeout`
/// 兜不住) ⇒ 与尚未退净的旧连接抢锁 ⇒ SQLITE_BUSY (`database is locked`)。
///
/// 本句柄把「写线程退出」变成 `Drop` 里的**有界同步等待**: 退出信号在写连接
/// **关闭之后**才发 ⇒ 收到信号即「连接已关」⇒ 窗口被**消除**, 而不是靠重试
/// 把 BUSY 拖过去。
struct WriterHandle {
    /// 写线程退出信号: **写连接已关之后**才发 (收到 ⇒ 连接确已关闭)。
    exited: StdMutex<std_mpsc::Receiver<()>>,
    /// 写线程是否已退出 (= 写连接已关)。写线程在**写连接关闭之后**置位。
    closed: Arc<AtomicBool>,
    /// 写线程句柄: 收口成功后 join; 超时则分离 (不 join, 避免无限期阻塞)。
    join: StdMutex<Option<std::thread::JoinHandle<()>>>,
    /// 写线程自身的 thread id: 写线程内释放最后一个池引用时不得 join 自己。
    thread_id: std::thread::ThreadId,
    /// 收口等待上限 (有界 ⇒ `Drop` 不会无限期阻塞)。
    grace: Duration,
}

impl WriterHandle {
    /// 有界等待写线程退出。返回 `true` 表示写连接已确认关闭。
    fn await_exit(&self) -> bool {
        let receiver = self
            .exited
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match receiver.recv_timeout(self.grace) {
            Ok(()) => true,
            // 写线程已死 (panic / 提前退出) ⇒ 写连接随线程栈一起 drop ⇒ 已关。
            Err(std_mpsc::RecvTimeoutError::Disconnected) => true,
            Err(std_mpsc::RecvTimeoutError::Timeout) => false,
        }
    }

    /// 写线程是否已退出 (= 写连接已关)。诊断快照。
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

impl Drop for WriterHandle {
    fn drop(&mut self) {
        // 写线程自身释放最后一个池引用 (在 write body 里 drop 池) ⇒ 不能 join
        // 自己 (join 自己 = 死锁)。此时写线程本来就要退出, 写连接照样会关。
        if self.thread_id == std::thread::current().id() {
            return;
        }
        // 有界等待写线程退出 (= 写连接已关)。超时 ⇒ **不 join** (join 会无限期
        // 阻塞在一次长写 / 长事务上) ⇒ 分离写线程, 它退出时连接照常关闭。
        if !self.await_exit() {
            return;
        }
        let mut slot = self
            .join
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(handle) = slot.take() {
            let _ = handle.join();
        }
    }
}

/// 带 SQLITE_BUSY 退避重试的连接管理器。
///
/// 包装 [`SqliteConnectionManager`]: 建连接 (含初始化 PRAGMA) 撞上
/// `SQLITE_BUSY` 时**有界**退避重试, 而不是一次即失败。
///
/// 论证 (为什么这是**加固**而不是掩盖根因):
/// - 根因是「池已弃但写连接还活着」这个窗口, 由 [`WriterHandle`] 的同步收口
///   **消除**; 本包装不参与、也不依赖那个窗口。
/// - 本包装只覆盖**别的**活连接造成的瞬时争用 (同路径另一套池句柄 / 另一进程 /
///   另一连接正在跑 WAL 索引恢复) —— 这类争用本来就该等, 不该一次即失败。
/// - 预算有界 (≤ `BUSY_RETRY_ATTEMPTS` 次、累计 < 1s), 不会把「真被锁死」
///   变成无限等待。
struct BusyRetryManager {
    inner: SqliteConnectionManager,
}

impl BusyRetryManager {
    fn new(inner: SqliteConnectionManager) -> Self {
        Self { inner }
    }
}

impl std::fmt::Debug for BusyRetryManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BusyRetryManager")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl ManageConnection for BusyRetryManager {
    type Connection = Connection;
    type Error = rusqlite::Error;

    fn connect(&self) -> Result<Connection, Self::Error> {
        connect_with_busy_retry(&self.inner)
    }

    fn is_valid(&self, conn: &mut Connection) -> Result<(), Self::Error> {
        self.inner.is_valid(conn)
    }

    fn has_broken(&self, conn: &mut Connection) -> bool {
        self.inner.has_broken(conn)
    }
}

/// 建连接撞上 SQLITE_BUSY 时**有界**退避重试 (见 [`BusyRetryManager`]）。
fn connect_with_busy_retry(
    manager: &SqliteConnectionManager,
) -> Result<Connection, rusqlite::Error> {
    let mut delay = BUSY_RETRY_INITIAL_DELAY;
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match manager.connect() {
            Ok(conn) => return Ok(conn),
            Err(error) => {
                if !is_busy(&error) || attempt >= BUSY_RETRY_ATTEMPTS {
                    return Err(error);
                }
                std::thread::sleep(delay);
                delay = delay.saturating_mul(2).min(BUSY_RETRY_MAX_DELAY);
            }
        }
    }
}

/// 判断一个 rusqlite 错误是否是 SQLITE_BUSY / SQLITE_LOCKED。
fn is_busy(error: &rusqlite::Error) -> bool {
    match error {
        rusqlite::Error::SqliteFailure(inner, _) => matches!(
            inner.code,
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
        ),
        _ => false,
    }
}

/// SQLite connection pool with one serialized writer and a reader pool.
#[derive(Clone)]
pub struct SqliteConnectionPool {
    /// 读池 (短命读连接; `min_idle = 0` ⇒ 读用到才建)。
    pool: r2d2::Pool<BusyRetryManager>,
    /// 写任务队列的发送端: 最后一个发送端被丢 ⇒ 写线程退出循环。
    write_tx: mpsc::Sender<WriteTask>,
    /// 写线程句柄: 最后一个池引用释放时**同步收口**写线程 (关闭写连接)。
    ///
    /// **字段顺序即 drop 顺序** (Rust 按声明顺序 drop 字段): 读池先关 (读连接
    /// 释放共享锁) → 再丢 `write_tx` (通知写线程退出) → 最后 drop 本句柄
    /// (等写线程真正退出 = 写连接已关) ⇒ 池弃用后同路径 reopen 不再撞锁。
    writer: Arc<WriterHandle>,
}

impl std::fmt::Debug for SqliteConnectionPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteConnectionPool")
            .field("pool_state", &self.pool.state())
            // 写线程是否已收口（`true` = 写线程已退出 ⇒ 写连接已关闭）。
            .field("writer_closed", &self.writer.as_ref().is_closed())
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
        let manager = BusyRetryManager::new(
            SqliteConnectionManager::file(&path)
                .with_init(move |conn| conn.execute_batch(&init_sql)),
        );

        let writer = manager.connect().map_err(|e| StorageError::Open {
            path: path.clone(),
            message: e.to_string(),
        })?;

        // 启动期 0 条空闲读连接, 读用到才建 (r2d2 默认在 build 时建满 max_size
        // 条 —— 每条都是一次文件打开 + PRAGMA 初始化, 是启动期重复 I/O)。
        // max_size 上限与并发语义不变; 只读路径首次查询补 1 条并按需补满,
        // 纯写路径 (如会话存储启动期迁移) 一条读连接都不建。
        let pool = r2d2::Pool::builder()
            .max_size(config.max_connections)
            .min_idle(Some(0))
            .build(manager)
            .map_err(|e| StorageError::Open {
                path: path.clone(),
                message: e.to_string(),
            })?;

        Ok(Self::from_pool_and_writer(
            pool,
            writer,
            shutdown_grace(&config),
        ))
    }

    /// Opens a shared in-memory pool.
    ///
    /// SQLite `:memory:` is per connection. This constructor uses a shared
    /// memory URI (through `r2d2_sqlite::SqliteConnectionManager::memory`) so
    /// the writer and every reader connection see the same database.
    pub async fn in_memory_with_config(config: SqliteConfig) -> Result<Self, StorageError> {
        config.validate()?;

        let init_sql = config.init_sql();
        let manager = BusyRetryManager::new(
            SqliteConnectionManager::memory().with_init(move |conn| conn.execute_batch(&init_sql)),
        );

        let writer = manager.connect().map_err(|e| StorageError::Open {
            path: PathBuf::from(":memory:"),
            message: e.to_string(),
        })?;

        // 同文件池: 启动期 0 条空闲读连接, 读用到才建 (见 open_with_config)。
        let pool = r2d2::Pool::builder()
            .max_size(config.max_connections)
            .min_idle(Some(0))
            .build(manager)
            .map_err(|e| StorageError::Open {
                path: PathBuf::from(":memory:"),
                message: e.to_string(),
            })?;

        Ok(Self::from_pool_and_writer(
            pool,
            writer,
            shutdown_grace(&config),
        ))
    }

    /// Opens a shared in-memory pool with default configuration.
    pub async fn in_memory() -> Result<Self, StorageError> {
        Self::in_memory_with_config(SqliteConfig::default()).await
    }

    /// Opens a file-backed pool that is **reused per path inside the process**.
    ///
    /// 池复用 (B-P1): 同一路径的重复 open 返回同一组底层句柄 (同一条写线程 +
    /// 同一个 r2d2 读池), 不再每客体各建一套连接。注册表持弱引用 —— 最后一个
    /// 使用者放手后池整体释放 (句柄与写线程随 Drop 回收), 注册表不留尸体。
    /// 并发语义不变: `max_size` 上限、单写者串行都作用在同一组句柄上。
    pub async fn open_shared<P: AsRef<Path>>(path: P) -> Result<Arc<Self>, StorageError> {
        let key = shared_registry_key(path.as_ref());
        let mut registry = shared_registry().lock().await;
        if let Some(existing) = registry.get(&key).and_then(Weak::upgrade) {
            return Ok(existing);
        }
        let pool = Arc::new(Self::open(path.as_ref()).await?);
        registry.insert(key, Arc::downgrade(&pool));
        Ok(pool)
    }

    /// 当前仍存活的共享池条目数 (句柄复用断言口; 死条目顺手清)。
    pub async fn shared_live_pools() -> usize {
        let mut registry = shared_registry().lock().await;
        registry.retain(|_, weak| weak.strong_count() > 0);
        registry.len()
    }

    /// 底层读池当前持有的连接句柄数 (不含写线程连接)。
    pub fn reader_connection_count(&self) -> u32 {
        self.pool.state().connections
    }

    /// Runs a short-lived read closure over a pooled reader connection.
    pub fn read<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let conn = self.pool.get()?;
        f(&conn)
    }

    /// Runs a mutation on the single serialized writer from synchronous code.
    ///
    /// This is the bridge for synchronous public traits. It submits to the same
    /// writer queue as [`Self::write`] and waits on a standard-library channel;
    /// it never enters or blocks on a Tokio runtime.
    pub fn write_sync<F, R>(&self, f: F) -> Result<R, StorageError>
    where
        F: FnOnce(&mut Connection) -> Result<R, StorageError> + Send + 'static,
        R: Send + 'static,
    {
        if tokio::runtime::Handle::try_current().is_ok() {
            std::thread::scope(|scope| {
                scope
                    .spawn(|| self.write_sync_blocking(f))
                    .join()
                    .map_err(|_| {
                        StorageError::WriteQueue("synchronous writer thread panicked".to_string())
                    })?
            })
        } else {
            self.write_sync_blocking(f)
        }
    }

    fn write_sync_blocking<F, R>(&self, f: F) -> Result<R, StorageError>
    where
        F: FnOnce(&mut Connection) -> Result<R, StorageError> + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = std_mpsc::sync_channel(0);
        let panicked = Arc::new(AtomicBool::new(false));
        let panicked_for_guard = Arc::clone(&panicked);
        let panicked_for_task = Arc::clone(&panicked);
        let task = WriteTask {
            body: Box::new(move |conn: &mut Connection| {
                let mut guard = ReplyGuard {
                    completed: false,
                    panicked: panicked_for_guard,
                };
                let result = f(conn);
                guard.completed = true;
                let _ = tx.send(result);
            }),
            panicked: panicked_for_task,
        };
        self.write_tx
            .blocking_send(task)
            .map_err(|_| StorageError::WriteQueue("writer channel is closed".to_string()))?;

        rx.recv().map_err(|_| writer_task_error(&panicked))?
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
        let panicked = Arc::new(AtomicBool::new(false));
        let panicked_for_guard = Arc::clone(&panicked);
        let panicked_for_task = Arc::clone(&panicked);
        let task = WriteTask {
            body: Box::new(move |conn: &mut Connection| {
                let mut guard = ReplyGuard {
                    completed: false,
                    panicked: panicked_for_guard,
                };
                let result = f(conn);
                guard.completed = true;
                let _ = tx.send(result);
            }),
            panicked: panicked_for_task,
        };

        self.write_tx
            .send(task)
            .await
            .map_err(|_| StorageError::WriteQueue("writer channel is closed".to_string()))?;

        rx.await.map_err(|_| writer_task_error(&panicked))?
    }

    fn from_pool_and_writer(
        pool: r2d2::Pool<BusyRetryManager>,
        writer: Connection,
        grace: Duration,
    ) -> Self {
        let (write_tx, mut write_rx) = mpsc::channel::<WriteTask>(1000);
        let (exit_tx, exit_rx) = std_mpsc::channel::<()>();

        // M8: writer 线程不得因任一调用方闭包 panic 而死亡. 旧实现
        // `task(&mut writer)` 直接调 — 任一 panic → 线程 unwind 死亡 →
        // write_tx 接收端消失 → 之后所有 write/write_sync 永远返回
        // "writer channel is closed", 且不自动重启. 现在 catch_unwind
        // 捕获 panic (panic = "abort" 构建下仍会终止进程, 这是 Rust 的
        // 固有限制), 向该调用方回 "writer task panicked" 后继续循环.
        // `closed` 标志：写线程**退出后**置位（诊断快照：写连接是否已关）。
        let closed = Arc::new(AtomicBool::new(false));
        let closed_for_thread = Arc::clone(&closed);
        let handle = std::thread::spawn(move || {
            {
                // 内层作用域: 写连接在作用域结束时关闭。
                let mut writer = writer;
                while let Some(task) = write_rx.blocking_recv() {
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        (task.body)(&mut writer);
                    }));
                    if outcome.is_err() {
                        task.panicked.store(true, Ordering::SeqCst);
                    }
                }
                // ← 作用域结束 ⇒ 写连接在此**关闭** (SQLite 侧的文件锁随之释放)。
            }
            // 退出信号在**写连接已关之后**才发 ⇒ 收到信号即「连接已关」。
            // 池弃用方 (`WriterHandle::drop`) 等这个信号 ⇒ 同路径 reopen 时
            // 旧连接已退净 ⇒ 不再撞 WAL 索引重建的排他锁。
            closed_for_thread.store(true, Ordering::SeqCst);
            let _ = exit_tx.send(());
        });
        let thread_id = handle.thread().id();

        let writer = Arc::new(WriterHandle {
            exited: StdMutex::new(exit_rx),
            closed,
            join: StdMutex::new(Some(handle)),
            thread_id,
            grace,
        });

        Self {
            pool,
            write_tx,
            writer,
        }
    }
}

/// 写线程收口等待上限 = 配置的 `busy_timeout` + [`WRITER_SHUTDOWN_GRACE`]。
///
/// 收口最多要等「写线程做完手上那次写」—— 一次写最长卡在 SQLite 的
/// `busy_timeout` 上 ⇒ 上限必须 **>** `busy_timeout`, 否则会在写线程刚好
/// 要收尾时放弃 join (窗口就又回来了)。上限之上再加固定余量吸收调度延迟。
fn shutdown_grace(config: &SqliteConfig) -> Duration {
    config.busy_timeout.saturating_add(WRITER_SHUTDOWN_GRACE)
}

/// 写任务回复端断开时的根因判定 (M8).
///
/// `body` 正常完成时回复一定已通过 channel 送达; 回复端断开只剩两种原因:
/// writer 线程整体死亡 (channel closed, 由 send 侧报错) 或 `body` panic.
/// `panicked` 标志位把后者与其它异常区分开, 报出真实根因.
fn writer_task_error(panicked: &AtomicBool) -> StorageError {
    if panicked.load(Ordering::SeqCst) {
        StorageError::WriteQueue("writer task panicked".to_string())
    } else {
        StorageError::WriteQueue("writer task did not return a result".to_string())
    }
}

/// 进程内共享池注册表 (B-P1 池复用): 键 = 归一化绝对路径, 值 = 弱引用。
/// 打开与查复用在同一把异步锁下串行, 并发 open 同一路径不会建出两套池。
fn shared_registry() -> &'static tokio::sync::Mutex<HashMap<PathBuf, Weak<SqliteConnectionPool>>> {
    static REGISTRY: OnceLock<tokio::sync::Mutex<HashMap<PathBuf, Weak<SqliteConnectionPool>>>> =
        OnceLock::new();
    REGISTRY.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

/// 注册表键归一: 绝对路径 (+ Windows 大小写折叠)。**不用** `canonicalize` ——
/// 首次 open 时目标文件尚不存在, canonicalize 成败前后键会不一致, 反而拆掉复用。
fn shared_registry_key(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    if cfg!(windows) {
        PathBuf::from(absolute.to_string_lossy().to_lowercase())
    } else {
        absolute
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 共享注册表是进程级状态: 三例断言活条目数, 彼此串行防互踩。
    static TEST_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

    async fn serialize() -> tokio::sync::MutexGuard<'static, ()> {
        TEST_LOCK
            .get_or_init(|| tokio::sync::Mutex::new(()))
            .lock()
            .await
    }

    fn temp_db(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "apeireth-pool-reuse-{}-{name}.sqlite3",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn shared_opens_reuse_one_pool_and_one_handle_set() {
        let _guard = serialize().await;
        let path = temp_db("reuse");
        let first = SqliteConnectionPool::open_shared(&path).await.unwrap();
        let second = SqliteConnectionPool::open_shared(&path).await.unwrap();
        let third = SqliteConnectionPool::open_shared(&path).await.unwrap();

        // 复用断言 1: 三次 open 是同一组底层句柄 (同一池 Arc)。
        assert!(Arc::ptr_eq(&first, &second));
        assert!(Arc::ptr_eq(&second, &third));
        // 复用断言 2: 注册表恰 1 条活条目 (句柄数口径)。
        assert_eq!(SqliteConnectionPool::shared_live_pools().await, 1);

        // 复用断言 3: 读路径只补出 1 条连接句柄, 三次 open 不叠加。
        first
            .read(|conn| Ok(conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))?))
            .unwrap();
        second
            .read(|conn| Ok(conn.query_row("SELECT 1", [], |row| row.get::<_, i64>(0))?))
            .unwrap();
        assert!(first.reader_connection_count() <= 1);
        assert_eq!(
            first.reader_connection_count(),
            second.reader_connection_count()
        );

        drop(first);
        drop(second);
        drop(third);
        // 最后一个使用者放手后注册表条目回收 (弱引用, 不留尸体)。
        assert_eq!(SqliteConnectionPool::shared_live_pools().await, 0);
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn shared_opens_on_distinct_paths_stay_distinct_pools() {
        let _guard = serialize().await;
        let left = temp_db("left");
        let right = temp_db("right");
        let a = SqliteConnectionPool::open_shared(&left).await.unwrap();
        let b = SqliteConnectionPool::open_shared(&right).await.unwrap();
        let a2 = SqliteConnectionPool::open_shared(&left).await.unwrap();

        assert!(!Arc::ptr_eq(&a, &b));
        assert!(Arc::ptr_eq(&a, &a2));
        assert_eq!(SqliteConnectionPool::shared_live_pools().await, 2);

        drop(a);
        drop(a2);
        drop(b);
        assert_eq!(SqliteConnectionPool::shared_live_pools().await, 0);
        let _ = std::fs::remove_file(&left);
        let _ = std::fs::remove_file(&right);
    }

    #[tokio::test]
    async fn shared_pool_writes_and_reads_behave_like_a_plain_pool() {
        let _guard = serialize().await;
        let path = temp_db("rw");
        let pool = SqliteConnectionPool::open_shared(&path).await.unwrap();
        pool.write(|conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY, v TEXT);
                 INSERT INTO t (id, v) VALUES (1, 'x');",
            )?;
            Ok(())
        })
        .await
        .unwrap();
        let value: String = pool
            .read(|conn| Ok(conn.query_row("SELECT v FROM t WHERE id = 1", [], |row| row.get(0))?))
            .unwrap();
        assert_eq!(value, "x");
        drop(pool);
        let _ = std::fs::remove_file(&path);
    }

    /// **根因钉子测试（2026-11-06）**：池被弃时若**写事务仍在进行**，
    /// 写连接此刻**持有 WAL 写锁** ⇒ 随后对**同一路径** reopen 要重建 `-shm`
    /// ⇒ 需要排他锁 ⇒ 旧连接未关 ⇒ `SQLITE_BUSY`（`database is locked`）。
    ///
    /// 修复前：`drop(pool)` 只丢 `Sender`、**立即返回** ⇒ 写事务仍在跑、旧连接仍开着
    /// ⇒ reopen 撞锁。修复后：`drop` 等写线程收口 ⇒ **连接已关** ⇒ reopen 必成。
    ///
    /// ⚠️ 复现要点：写任务必须**真的在跑**（持锁）时 drop ⇒ 用一个带闸门的写任务，
    /// 由测试侧在事务中途放行 ⇒ 避免依赖时间窗口。
    #[tokio::test]
    async fn dropping_the_last_pool_while_a_write_is_in_flight_lets_the_same_path_reopen() {
        let _guard = serialize().await;
        let path = temp_db("reopen-after-drop");
        let _ = std::fs::remove_file(&path);

        let pool = SqliteConnectionPool::open_shared(&path).await.unwrap();
        pool.write(|conn| {
            conn.execute_batch("CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY, v TEXT);")?;
            Ok(())
        })
        .await
        .unwrap();

        // 一笔**长写事务**：写线程持写锁时等待测试侧放行。
        // 闸门用 `Arc<(Mutex<bool>, Condvar)>`（可跨线程共享）⇒ 不依赖时间窗口。
        let gate = Arc::new((StdMutex::new(false), std::sync::Condvar::new()));
        let writer_pool = pool.clone();
        let write_gate = Arc::clone(&gate);
        let write_task = tokio::task::spawn(async move {
            writer_pool
                .write(move |conn| {
                    conn.execute_batch(
                        "INSERT INTO t (id, v) VALUES (2, 'y');
                         BEGIN IMMEDIATE;
                         INSERT INTO t (id, v) VALUES (3, 'z');",
                    )?;
                    // 持着写锁等测试侧放行。
                    let (lock, cvar) = &*write_gate;
                    let mut released = lock.lock().unwrap_or_else(|p| p.into_inner());
                    while !*released {
                        released = cvar.wait(released).unwrap_or_else(|p| p.into_inner());
                    }
                    conn.execute_batch("COMMIT;")?;
                    Ok(())
                })
                .await
        });

        // 等写事务真正进入（写线程已取到任务）。
        tokio::time::sleep(Duration::from_millis(200)).await;

        // **关键动作**：弃掉最后一个池引用（写线程本身不持有池 ⇒ 不会自 join）。
        drop(pool);

        // 放行写事务（若 drop 已同步收口，此刻写线程已在等这个信号）。
        {
            let (lock, cvar) = &*gate;
            let mut released = lock.lock().unwrap_or_else(|p| p.into_inner());
            *released = true;
            cvar.notify_all();
        }
        let _ = write_task.await;

        // 同路径**立即** reopen：修复前此处会撞 `database is locked`。
        let reopened = SqliteConnectionPool::open_shared(&path).await;
        assert!(
            reopened.is_ok(),
            "同路径 reopen 必须成功（写连接已随池收口而关闭）: {:?}",
            reopened.err()
        );
        if let Ok(reopened) = reopened {
            let value: i64 = reopened
                .read(|conn| {
                    Ok(conn
                        .query_row("SELECT COUNT(*) FROM t WHERE id = 3", [], |row| row.get(0))?)
                })
                .unwrap();
            assert_eq!(value, 1, "被封的那笔写事务必须已落库（不能丢写）");
            drop(reopened);
        }
        let _ = std::fs::remove_file(&path);
    }
}
