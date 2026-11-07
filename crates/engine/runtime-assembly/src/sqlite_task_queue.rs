//! SQLite 任务队列存储 (V6-a): 与会话账本**同库** (`sessions.sqlite3`) 的任务表。
//!
//! [`apeireth_orchestration::task_queue`] 的持久化端口落地: 任务逐行落
//! `task_queue_tasks` 表 (id/state/data), 写派号/唤醒队列/事件账落 `task_queue_meta`
//! 表 —— 一次 [`TaskStore::update`] = 一次 `BEGIN IMMEDIATE` 事务里的原子读改写,
//! 并发入队 (多手/多进程同库) 互不覆盖。
//!
//! **与会话账本同库族**: 本存储开在会话账本同一个库文件上 (同一个 sqlite 库),
//! 任务交接与会话账本同寿命、同备份面; 表是增量建表 (`CREATE TABLE IF NOT
//! EXISTS`), 不动账本既有表。

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use apeireth_orchestration::task_queue::{
    TaskEvent, TaskQueueError, TaskQueueResult, TaskQueueState, TaskRecord, TaskStore, TaskWake,
};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

/// 任务表 (一行一任务; `data` = 任务记录 JSON, `state` = 分态计数列)。
const TASKS_TABLE: &str = "task_queue_tasks";
/// 队列元表 (写派号 / 唤醒队列 / 事件账)。
const META_TABLE: &str = "task_queue_meta";

// ============================================================================
// SQLITE_BUSY 有界退避重试 (只包住「能不能开始事务」这一层)
// ============================================================================
//
// 形态照抄仓内既有先例 (`crates/engine/storage/src/pool.rs` 的
// `BusyRetryManager` / `connect_with_busy_retry` / `is_busy`, `:279`-`:354`):
// 指数退避 + 有界预算 + 只认 SQLITE_BUSY / SQLITE_LOCKED。
//
// 为什么**必须**在生产侧 (而不是让调用方重试整个入队):
// - 根因是「并发写者撞车」: `TaskStore::update` 用 `BEGIN IMMEDIATE` 起事务,
//   赢方在事务里做**整档读改写** (`:153`-`:212`) ⇒ 写锁一直持有到 `commit()`;
//   输方全部预算 = 连接的 `busy_timeout` (5s, `:41`) ⇒ 赢方耗时 > 预算时输方拿到
//   SQLITE_BUSY ⇒ 生产路径**直接失败**。
// - 端口层的 `TaskQueueError::Store { reason }` 只有**人读 reason**, 没有结构化的
//   sqlite 错误码 ⇒ 上一批只能在**测试侧**按文案猜「是不是 busy」(弱判定)。
//   本批在**生产侧**直接用 `rusqlite::Error` 的 `ErrorCode` 结构化判定
//   (`DatabaseBusy` / `DatabaseLocked`) ⇒ 那套文案匹配不再需要。
//
// 为什么在 BEGIN 处重试是**安全**的:
// - `BEGIN IMMEDIATE` 失败 ⇒ SQLite **没有**开启事务 ⇒ 连接回到 autocommit 态
//   ⇒ 重试只是「再要一次写锁」, 事务体 (整档读改写) 一次都没跑过
//   ⇒ 无重复写、无重复吃 `write_seq`。
// - 事务体一旦开始 (拿到锁) ⇒ 后面无论成功还是回滚, 都只执行一次。
//
// 预算取值 (生产侧比测试侧保守 —— 上一批测试侧用 20s; 生产是**运行期**写路径,
// 不能为一把锁挂 20s):
// - 第一次尝试**完全沿用**连接既有的 `busy_timeout` (5s) ⇒ 常见路径 (对方 < 5s
//   就提交) 的行为**一字未改**。
// - 第一次失败后**额外**再给 ≤ `BUSY_RETRY_EXTRA_BUDGET` (3s) 的退避重试预算
//   ⇒ 总等待 ≲ 5s + 3s = 8s。
// - 额外轮次把 busy 窗口**收窄**到 `BUSY_RETRY_WINDOW` (250ms) ⇒ 每轮等待有界
//   ⇒ 总等待有界 (不会「预算还剩 10ms 却再等 5s」)。
const BUSY_RETRY_INITIAL_DELAY: Duration = Duration::from_millis(5);
/// SQLITE_BUSY 退避重试: 单次等待上限 (指数退避封顶).
const BUSY_RETRY_MAX_DELAY: Duration = Duration::from_millis(250);
/// 退避重试的**额外**总预算 (叠加在连接既有的 `busy_timeout` 之上).
const BUSY_RETRY_EXTRA_BUDGET: Duration = Duration::from_secs(3);
/// 退避重试**轮次**的 busy 窗口 (收窄 ⇒ 每轮等待有界 ⇒ 总等待有界).
const BUSY_RETRY_WINDOW: Duration = Duration::from_millis(250);

/// 判定一个 sqlite 错误是不是 SQLITE_BUSY / SQLITE_LOCKED。
///
/// 结构化判定 (读 `rusqlite::ErrorCode`), **不**做文案匹配 ⇒ 端口层
/// `TaskQueueError::Store { reason }` 没有结构化错误码这件事不再影响生产判定。
fn is_busy_error(error: &rusqlite::Error) -> bool {
    match error {
        rusqlite::Error::SqliteFailure(inner, _) => matches!(
            inner.code,
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
        ),
        _ => false,
    }
}

/// 读连接当前的 `busy_timeout` (毫秒)。
fn read_busy_timeout_ms(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("PRAGMA busy_timeout", [], |row| row.get(0))
}

/// 把 `PRAGMA busy_timeout` 的毫秒值还原成 [`Duration`] (负值按 0 处理).
fn busy_timeout_duration(millis: i64) -> Duration {
    Duration::from_millis(u64::try_from(millis).unwrap_or(0))
}

/// 预算耗尽仍是 busy ⇒ **保留原错误码** (结构化判定不变: 仍是
/// `DatabaseBusy` / `DatabaseLocked`), 只在**消息尾部**追加「等过、仍被锁」的说明
/// (原消息前缀保留) ⇒ 既不吞错、也不把 busy 改写成别的错。
fn busy_error_after_wait(
    error: rusqlite::Error,
    attempts: u32,
    elapsed: Duration,
) -> rusqlite::Error {
    match error {
        rusqlite::Error::SqliteFailure(inner, message) => {
            let base = message.unwrap_or_else(|| inner.to_string());
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: inner.code,
                    extended_code: inner.extended_code,
                },
                Some(format!(
                    "{base}; write lock still held after waiting: {attempts} extra attempt(s), \
                     {} ms total (extra budget {} ms)",
                    elapsed.as_millis(),
                    BUSY_RETRY_EXTRA_BUDGET.as_millis(),
                )),
            )
        }
        other => other,
    }
}

/// 起一个 `BEGIN IMMEDIATE` 事务: 撞上 SQLITE_BUSY / SQLITE_LOCKED 时
/// **有界**退避重试。
///
/// - ① 只对 busy 类错误重试; 其它错误**原样上抛**。
/// - ② 预算有界: 第一次尝试沿用连接既有的 `busy_timeout`, 失败后**额外**再给
///   `BUSY_RETRY_EXTRA_BUDGET` (3s) 的指数退避重试 ⇒ 总等待 ≲ 8s。
/// - ③ 重试发生在 `BEGIN` 处 ⇒ **事务体一次都不重复执行** (整档读改写不重复,
///   `write_seq` 不会被重复吃)。
/// - ④ 预算耗尽仍是 busy ⇒ 返回原错误 (错误码不变) + 「等过、仍被锁」的说明。
///
/// ⚠️ **为什么用 [`Transaction::new_unchecked`] 而不是
/// [`Connection::transaction_with_behavior`]**: 后者要求 `&mut Connection`,
/// 而「拿到写锁」这个借用必须**活到函数返回** (返回的 `Transaction` 挂在它上面)
/// ⇒ 函数体内无法再碰连接 (收窄 / 恢复 `busy_timeout` 都要碰) ⇒ 借用检查直接拒绝。
/// `Transaction::new` 在 rusqlite 里**就是** `Transaction::new_unchecked`
/// (只有 `&mut` / `&` 的**编译期**嵌套防护不同, 运行时发出的 SQL 完全一样:
/// `BEGIN IMMEDIATE`) ⇒ 换过去**运行时语义一字未改**。
///
/// **嵌套防护仍在**: 调用方传进来的是 `&mut Connection` (这里按共享引用重借用)
/// ⇒ 事务存活期间调用方拿不回那个 `&mut` ⇒ 同一个连接上开嵌套事务仍然编译期被禁。
fn begin_immediate_with_busy_retry<'conn>(
    conn: &'conn Connection,
) -> Result<Transaction<'conn>, rusqlite::Error> {
    // 连接既有的 busy_timeout (毫秒): 收窄 / 恢复都要用 ⇒ 读不出来就**不收窄**
    // (退化成「每次尝试都吃满既有 busy_timeout」, 仍然有界, 只是上界更松)。
    let original_busy_timeout: Option<i64> = read_busy_timeout_ms(conn).ok();
    let started = Instant::now();
    let mut delay = BUSY_RETRY_INITIAL_DELAY;
    let mut extra_attempts: u32 = 0;
    // 额外预算的截止时刻: **第一次失败之后**才起算 (第一次尝试花掉的是连接既有
    // 的 busy_timeout, 那部分不计入「额外预算」)。
    let mut deadline: Option<Instant> = None;
    let mut narrowed = false;

    loop {
        match Transaction::new_unchecked(conn, TransactionBehavior::Immediate) {
            Ok(tx) => {
                if narrowed {
                    if let Some(original) = original_busy_timeout {
                        // 恢复连接既有的 busy_timeout (净效果: 连接配置不变)。
                        tx.busy_timeout(busy_timeout_duration(original))?;
                    }
                }
                return Ok(tx);
            }
            Err(error) => {
                // ① 其它错误一律原样上抛 (不重试、不改写)。
                if !is_busy_error(&error) {
                    if narrowed {
                        if let Some(original) = original_busy_timeout {
                            conn.busy_timeout(busy_timeout_duration(original))?;
                        }
                    }
                    return Err(error);
                }
                // ② 第一次失败 ⇒ 起算额外预算, 并收窄 busy 窗口 (保证总等待有界)。
                let deadline =
                    *deadline.get_or_insert_with(|| Instant::now() + BUSY_RETRY_EXTRA_BUDGET);
                if !narrowed {
                    narrowed = true;
                    // 读不出原值就不收窄 (收窄了却恢复不了 = 连接配置被改坏)。
                    if original_busy_timeout.is_some() {
                        conn.busy_timeout(BUSY_RETRY_WINDOW)?;
                    }
                }
                let now = Instant::now();
                if now >= deadline {
                    if let Some(original) = original_busy_timeout {
                        conn.busy_timeout(busy_timeout_duration(original))?;
                    }
                    // ④ 预算耗尽: 保留原错误码, 追加「等过、仍被锁」的说明。
                    return Err(busy_error_after_wait(
                        error,
                        extra_attempts,
                        started.elapsed(),
                    ));
                }
                std::thread::sleep(delay.min(deadline - now));
                delay = delay.saturating_mul(2).min(BUSY_RETRY_MAX_DELAY);
                extra_attempts += 1;
            }
        }
    }
}

/// sqlite 持久化的任务队列存储 (同会话账本库文件)。
pub struct SqliteTaskStore {
    connection: Mutex<Connection>,
}

impl SqliteTaskStore {
    /// 打开 (或初始化) 指定库文件上的任务表: 与会话账本同库, 增量建表。
    pub fn open(path: impl AsRef<Path>) -> TaskQueueResult<Self> {
        let connection =
            Connection::open(path.as_ref()).map_err(|error| TaskQueueError::Store {
                operation: "open",
                reason: error.to_string(),
            })?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|error| TaskQueueError::Store {
                operation: "busy_timeout",
                reason: error.to_string(),
            })?;
        ensure_schema(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// 共享内存库 (测试/无持久面的组合用; 与 [`Self::open`] 同一 schema)。
    pub fn in_memory() -> TaskQueueResult<Self> {
        let connection = Connection::open_in_memory().map_err(|error| TaskQueueError::Store {
            operation: "open",
            reason: error.to_string(),
        })?;
        ensure_schema(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(&self) -> TaskQueueResult<std::sync::MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|error| TaskQueueError::Store {
                operation: "lock",
                reason: error.to_string(),
            })
    }
}

/// 增量建表 (幂等; 不动账本既有表)。
fn ensure_schema(connection: &Connection) -> TaskQueueResult<()> {
    connection
        .execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {TASKS_TABLE} (
                id TEXT PRIMARY KEY,
                state TEXT NOT NULL,
                data TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS {META_TABLE} (
                key TEXT PRIMARY KEY,
                data TEXT NOT NULL
            );"
        ))
        .map_err(|error| TaskQueueError::Store {
            operation: "schema",
            reason: error.to_string(),
        })
}

/// 把一列 meta 行读成值 (缺行 = None)。
fn read_meta<T: serde::de::DeserializeOwned>(
    connection: &Connection,
    key: &str,
) -> TaskQueueResult<Option<T>> {
    let data: Option<String> = connection
        .query_row(
            &format!("SELECT data FROM {META_TABLE} WHERE key = ?1"),
            rusqlite::params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| TaskQueueError::Store {
            operation: "read_meta",
            reason: error.to_string(),
        })?;
    data.map(|json| {
        serde_json::from_str(&json).map_err(|error| TaskQueueError::Store {
            operation: "parse_meta",
            reason: format!("{key}: {error}"),
        })
    })
    .transpose()
}

/// 整档读出 (任务行 + 元行)。
fn load_locked(connection: &Connection) -> TaskQueueResult<TaskQueueState> {
    let mut statement = connection
        .prepare(&format!("SELECT data FROM {TASKS_TABLE}"))
        .map_err(|error| TaskQueueError::Store {
            operation: "read",
            reason: error.to_string(),
        })?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| TaskQueueError::Store {
            operation: "read",
            reason: error.to_string(),
        })?;
    let mut state = TaskQueueState::default();
    for row in rows {
        let data = row.map_err(|error| TaskQueueError::Store {
            operation: "read",
            reason: error.to_string(),
        })?;
        let record: TaskRecord =
            serde_json::from_str(&data).map_err(|error| TaskQueueError::Store {
                operation: "parse_task",
                reason: error.to_string(),
            })?;
        state.tasks.insert(record.id.clone(), record);
    }
    state.write_seq = read_meta::<u64>(connection, "write_seq")?.unwrap_or(0);
    state.wakes = read_meta::<Vec<TaskWake>>(connection, "wakes")?.unwrap_or_default();
    state.journal = read_meta::<Vec<TaskEvent>>(connection, "journal")?.unwrap_or_default();
    Ok(state)
}

/// 整档写回 (事务内全量对齐: 任务行 upsert/删除 + 元行 upsert)。
fn save_locked(connection: &Connection, state: &TaskQueueState) -> TaskQueueResult<()> {
    connection
        .execute(&format!("DELETE FROM {TASKS_TABLE}"), [])
        .map_err(|error| TaskQueueError::Store {
            operation: "write",
            reason: error.to_string(),
        })?;
    for (id, record) in &state.tasks {
        let data = serde_json::to_string(record).map_err(|error| TaskQueueError::Store {
            operation: "serialize_task",
            reason: error.to_string(),
        })?;
        connection
            .execute(
                &format!("INSERT INTO {TASKS_TABLE} (id, state, data) VALUES (?1, ?2, ?3)"),
                rusqlite::params![id, record.state.to_string(), data],
            )
            .map_err(|error| TaskQueueError::Store {
                operation: "write",
                reason: error.to_string(),
            })?;
    }
    let meta: [(&str, String); 3] = [
        (
            "write_seq",
            serde_json::to_string(&state.write_seq).map_err(|error| TaskQueueError::Store {
                operation: "serialize_meta",
                reason: error.to_string(),
            })?,
        ),
        (
            "wakes",
            serde_json::to_string(&state.wakes).map_err(|error| TaskQueueError::Store {
                operation: "serialize_meta",
                reason: error.to_string(),
            })?,
        ),
        (
            "journal",
            serde_json::to_string(&state.journal).map_err(|error| TaskQueueError::Store {
                operation: "serialize_meta",
                reason: error.to_string(),
            })?,
        ),
    ];
    for (key, data) in meta {
        connection
            .execute(
                &format!(
                    "INSERT INTO {META_TABLE} (key, data) VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET data = excluded.data"
                ),
                rusqlite::params![key, data],
            )
            .map_err(|error| TaskQueueError::Store {
                operation: "write",
                reason: error.to_string(),
            })?;
    }
    Ok(())
}

impl TaskStore for SqliteTaskStore {
    fn read(&self) -> TaskQueueResult<TaskQueueState> {
        let connection = self.lock()?;
        load_locked(&connection)
    }

    fn update(&self, update: &mut dyn FnMut(&mut TaskQueueState)) -> TaskQueueResult<()> {
        let connection = self.lock()?;
        // 一次 update = 一次原子读改写 (IMMEDIATE 事务): 并发多手互不覆盖。
        //
        // B-P2 (SQLITE_BUSY 下沉到生产侧): `BEGIN IMMEDIATE` 撞上对方未提交的写
        // 事务时走**有界退避重试** (见 `begin_immediate_with_busy_retry`) —— 重试
        // 只发生在「能不能开始事务」这一层 ⇒ 整档读改写一次都不重复执行
        // (无重复写、无重复吃 `write_seq`)。
        let transaction = begin_immediate_with_busy_retry(&connection).map_err(|error| {
            TaskQueueError::Store {
                operation: "begin",
                reason: error.to_string(),
            }
        })?;
        let mut state = load_locked(&transaction)?;
        update(&mut state);
        save_locked(&transaction, &state)?;
        transaction.commit().map_err(|error| TaskQueueError::Store {
            operation: "commit",
            reason: error.to_string(),
        })
    }
}
