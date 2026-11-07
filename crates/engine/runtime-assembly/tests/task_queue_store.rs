//! V6-a 任务队列的 sqlite 持久面证明: 与会话账本同库的任务表。
//!
//! 这里钉**真落盘**的崩溃安全: 文件库 + 重开 = 进程重启, `running` 态任务回
//! `pending` 并派 attempt 计数; 多手同库并发入队原子; 未投递唤醒跨重启重投。

use std::sync::Arc;

use apeireth_orchestration::task_queue::{
    EnqueueOutcome, TaskCaller, TaskClockMs, TaskEvent, TaskQueue, TaskQueueConfig, TaskQueueError,
    TaskQueueResult, TaskSpec, TaskState, TaskStore,
};
use apeireth_runtime_assembly::SqliteTaskStore;

/// SQLITE_BUSY 退避重试: 首次等待时长。
const BUSY_RETRY_INITIAL_DELAY: std::time::Duration = std::time::Duration::from_millis(5);
/// SQLITE_BUSY 退避重试: 单次等待上限 (指数退避封顶)。
const BUSY_RETRY_MAX_DELAY: std::time::Duration = std::time::Duration::from_millis(250);
/// 输方等赢方事务提交的总预算 (含每次尝试里 SQLite busy handler 在锁上的等待)。
const BUSY_WAIT_BUDGET: std::time::Duration = std::time::Duration::from_secs(20);

/// 判定一个存储错误是不是「对方还占着写锁」。
///
/// `TaskQueueError::Store` 只带**人读 reason** (存储端口层没有结构化的 sqlite
/// 错误码) ⇒ 这里按 SQLite 的 busy 文案判定; 非 busy 的存储错误 (序列化失败 /
/// 约束冲突 / 读失败 …) 一律**不重试**, 原样上抛。
fn is_busy_store_error(error: &TaskQueueError) -> bool {
    match error {
        TaskQueueError::Store { reason, .. } => {
            reason.contains("database is locked")
                || reason.contains("database table is locked")
                || reason.contains("The database file is locked")
                || reason.contains("SQLITE_BUSY")
        }
        _ => false,
    }
}

/// 并发入队时, 输方在 `SQLITE_BUSY` 上做**有界退避重试**。
///
/// 机制 (读码取证, 行号指 `crates/engine/runtime-assembly/src/sqlite_task_queue.rs`):
/// - `TaskStore::update` 用 `BEGIN IMMEDIATE` 起事务 (`:225`); 两个 store 句柄
///   = 两个**裸** `rusqlite::Connection` (`:28`-`:30`) ⇒ 只能一个拿到写锁。
/// - 赢方在事务里做**整档读改写** (删表 + 逐行插回 + 3 行 meta upsert,
///   `:153`-`:212`) ⇒ 写锁一直持有到 `commit()` (`:233`)。
/// - 输方拿不到锁时只有 `busy_timeout` (5s, `:41`) 的预算 ⇒ 赢方事务还没跑完时
///   输方拿到 `SQLITE_BUSY` ⇒ 旧写法 `expect("enqueue")` 直接 panic。
/// - 受控复现 (施工期的临时脚本, 未入库): 外部写者持写锁 6s ⇒ 旧写法
///   **5013ms 后** 报 `task queue store begin: database is locked`; 本退避重试版
///   **6051ms 后** 跑通 (等对方提交完再继续)。
///
/// 为什么这是「等对方跑完」而不是「把失败藏起来」:
/// - `SQLITE_BUSY` 只表示「对方那个写事务还没提交」; 输方该做的是等。
/// - 重试是**整次入队**重试, 但失败的 attempt **没留下任何写入**:
///   `update` 在 `BEGIN IMMEDIATE` 失败时事务根本没起、闭包没跑;
///   事务内失败则整笔回滚 ⇒ 重试不会重复入队、不会重复吃 `write_seq`
///   ⇒ 本用例要钉的语义 (并发入队一条不丢 / `write_seq` 恰好 20) 一条没松。
/// - 预算**有界** (总预算 + 退避封顶) ⇒ 真死锁不会变成无限挂起;
///   预算耗尽仍是 `SQLITE_BUSY` ⇒ 照旧 panic (错误不吞、不改写)。
fn enqueue_waiting_for_writer(
    queue: &TaskQueue,
    spec: TaskSpec,
) -> TaskQueueResult<EnqueueOutcome> {
    let deadline = std::time::Instant::now() + BUSY_WAIT_BUDGET;
    let mut delay = BUSY_RETRY_INITIAL_DELAY;
    loop {
        match queue.enqueue(spec.clone()) {
            Ok(outcome) => return Ok(outcome),
            Err(error) => {
                if !is_busy_store_error(&error) {
                    return Err(error);
                }
                let now = std::time::Instant::now();
                if now >= deadline {
                    return Err(error);
                }
                std::thread::sleep(delay.min(deadline - now));
                delay = delay.saturating_mul(2).min(BUSY_RETRY_MAX_DELAY);
            }
        }
    }
}

/// 定值时钟 (确定性测试)。
fn fixed_clock() -> TaskClockMs {
    Arc::new(|| 1_700_000_000_000)
}

fn worker() -> TaskCaller {
    TaskCaller::agent("worker-a")
}

/// 队列持久化: 重开 (真文件库) 后 running → pending + attempt 计数, 任务不丢。
#[test]
fn reopening_the_sqlite_store_recovers_running_to_pending_with_attempt_accounting() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("sessions.sqlite3");

    // 与会话账本同库: 同一个库文件上的任务表 (增量建表, 不动账本表)。
    let store: Arc<dyn TaskStore> = Arc::new(SqliteTaskStore::open(&db).expect("store"));
    let queue = TaskQueue::open_with_clock(
        Arc::clone(&store),
        TaskQueueConfig::default(),
        fixed_clock(),
    )
    .expect("queue opens");
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("会崩的执行段", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let id = task.id.clone();
    queue.drain_wakes().expect("drain");
    queue.begin_execution(&id, &worker()).expect("begin");
    drop(queue);
    drop(store);

    // 「进程重启」: 全部句柄丢弃, 从同一个库文件重开。
    let store: Arc<dyn TaskStore> = Arc::new(SqliteTaskStore::open(&db).expect("store"));
    let queue = TaskQueue::open_with_clock(store, TaskQueueConfig::default(), fixed_clock())
        .expect("queue opens");
    let recovered = queue.get(&id).expect("task survives restart");
    assert_eq!(
        recovered.state,
        TaskState::Pending,
        "running 必须回 pending"
    );
    assert_eq!(
        recovered.attempts, 2,
        "恢复即重派一次 (attempt 计数防死循环)"
    );
    assert_eq!(recovered.assignee.as_deref(), Some("worker-a"));
    assert!(
        queue
            .journal()
            .expect("journal")
            .iter()
            .any(|event| matches!(event, TaskEvent::RecoveredToPending { task_id, attempts: 2, .. } if task_id == &id)),
        "恢复要留审计"
    );

    // 表在会话账本同一库文件里 (与会话账本同库族)。
    assert!(db.is_file(), "任务表与会话账本同库落盘");
}

/// 并发入队原子: 两只手 (两个 store 句柄) 同库互不覆盖, 一条不丢。
#[test]
fn concurrent_enqueues_across_two_sqlite_handles_are_atomic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("sessions.sqlite3");
    let left = Arc::new(SqliteTaskStore::open(&db).expect("store"));
    let right = Arc::new(SqliteTaskStore::open(&db).expect("store"));
    let left_queue = TaskQueue::open_with_clock(
        left.clone() as Arc<dyn TaskStore>,
        TaskQueueConfig::default(),
        fixed_clock(),
    )
    .expect("queue opens");
    let right_queue = TaskQueue::open_with_clock(
        right.clone() as Arc<dyn TaskStore>,
        TaskQueueConfig::default(),
        fixed_clock(),
    )
    .expect("queue opens");

    let threads = [
        std::thread::spawn(move || {
            for round in 0..10 {
                // 输方处置 = **有界退避重试** (不是一次即失败, 也不是吞错):
                // 等赢方把写事务提交掉, 再重试这次入队。
                enqueue_waiting_for_writer(
                    &left_queue,
                    TaskSpec::new(format!("左手 {round}"), serde_json::Value::Null)
                        .with_idempotency_key(format!("left-{round}")),
                )
                .expect("enqueue");
            }
        }),
        std::thread::spawn(move || {
            for round in 0..10 {
                enqueue_waiting_for_writer(
                    &right_queue,
                    TaskSpec::new(format!("右手 {round}"), serde_json::Value::Null)
                        .with_idempotency_key(format!("right-{round}")),
                )
                .expect("enqueue");
            }
        }),
    ];
    for thread in threads {
        thread.join().expect("thread finishes");
    }

    let store: Arc<dyn TaskStore> = Arc::new(SqliteTaskStore::open(&db).expect("store"));
    let queue = TaskQueue::open_with_clock(store, TaskQueueConfig::default(), fixed_clock())
        .expect("queue opens");
    assert_eq!(
        queue.list().expect("list").len(),
        20,
        "并发入队不得丢任务 (一次 update = 一次原子事务)"
    );
    assert_eq!(queue.write_seq().expect("write_seq"), 20);
}

/// 未投递唤醒跨重启重投 (redelivered 计数), 唤醒不丢。
#[test]
fn undelivered_wakes_survive_restart_on_the_sqlite_store() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("sessions.sqlite3");
    let store: Arc<dyn TaskStore> = Arc::new(SqliteTaskStore::open(&db).expect("store"));
    let queue = TaskQueue::open_with_clock(store, TaskQueueConfig::default(), fixed_clock())
        .expect("queue opens");
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("没人收的唤醒", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let id = task.id.clone();
    drop(queue);

    let store: Arc<dyn TaskStore> = Arc::new(SqliteTaskStore::open(&db).expect("store"));
    let queue = TaskQueue::open_with_clock(store, TaskQueueConfig::default(), fixed_clock())
        .expect("queue opens");
    assert_eq!(
        queue.get(&id).expect("get").redelivered,
        1,
        "未投递唤醒重启后重投, 计 redelivered"
    );
    let wakes = queue.drain_wakes().expect("drain");
    assert_eq!(wakes.len(), 1, "唤醒不丢");
    assert_eq!(wakes[0].task_id, id);
}

/// 结算挂账的持久面: 一回合一段跨重启不重复。
#[test]
fn settlements_survive_restart_without_duplicating_the_segment() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("sessions.sqlite3");
    let store: Arc<dyn TaskStore> = Arc::new(SqliteTaskStore::open(&db).expect("store"));
    let queue = TaskQueue::open_with_clock(store, TaskQueueConfig::default(), fixed_clock())
        .expect("queue opens");
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("带结算的活", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let id = task.id.clone();
    let settlement = apeireth_orchestration::task_queue::TaskSettlement {
        turn: 12,
        ledger_ref: "work_log".to_string(),
        linked_at_ms: 1_700_000_000_500,
    };
    queue
        .link_settlement(&id, settlement.clone())
        .expect("link");
    drop(queue);

    let store: Arc<dyn TaskStore> = Arc::new(SqliteTaskStore::open(&db).expect("store"));
    let queue = TaskQueue::open_with_clock(store, TaskQueueConfig::default(), fixed_clock())
        .expect("queue opens");
    let outcome = queue.link_settlement(&id, settlement).expect("link again");
    assert!(
        matches!(
            outcome,
            apeireth_orchestration::task_queue::SettlementOutcome::AlreadyLinked { turn: 12, .. }
        ),
        "跨重启重复挂账仍不重复记账: {outcome:?}"
    );
    assert_eq!(queue.get(&id).expect("get").settlements.len(), 1);
}

/// 输入校验在持久面上同样拒绝即帧 (不落脏行)。
#[test]
fn invalid_specs_never_reach_the_sqlite_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store: Arc<dyn TaskStore> =
        Arc::new(SqliteTaskStore::open(dir.path().join("sessions.sqlite3")).expect("store"));
    let queue = TaskQueue::open_with_clock(store, TaskQueueConfig::default(), fixed_clock())
        .expect("queue opens");
    let result: TaskQueueResult<EnqueueOutcome> =
        queue.enqueue(TaskSpec::new("  ", serde_json::Value::Null));
    assert!(result.is_err());
    assert_eq!(queue.list().expect("list").len(), 0);
}
