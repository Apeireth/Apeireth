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

use apeireth_orchestration::task_queue::{
    TaskEvent, TaskQueueError, TaskQueueResult, TaskQueueState, TaskRecord, TaskStore, TaskWake,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

/// 任务表 (一行一任务; `data` = 任务记录 JSON, `state` = 分态计数列)。
const TASKS_TABLE: &str = "task_queue_tasks";
/// 队列元表 (写派号 / 唤醒队列 / 事件账)。
const META_TABLE: &str = "task_queue_meta";

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
        let mut connection = self.lock()?;
        // 一次 update = 一次原子读改写 (IMMEDIATE 事务): 并发多手互不覆盖。
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| TaskQueueError::Store {
                operation: "begin",
                reason: error.to_string(),
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
