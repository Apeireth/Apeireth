//! V6-a 任务队列的 sqlite 持久面证明: 与会话账本同库的任务表。
//!
//! 这里钉**真落盘**的崩溃安全: 文件库 + 重开 = 进程重启, `running` 态任务回
//! `pending` 并派 attempt 计数; 多手同库并发入队原子; 未投递唤醒跨重启重投。

use std::sync::Arc;

use apeireth_orchestration::task_queue::{
    EnqueueOutcome, TaskCaller, TaskClockMs, TaskEvent, TaskQueue, TaskQueueConfig,
    TaskQueueResult, TaskSpec, TaskState, TaskStore,
};
use apeireth_runtime_assembly::SqliteTaskStore;

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
                left_queue
                    .enqueue(
                        TaskSpec::new(format!("左手 {round}"), serde_json::Value::Null)
                            .with_idempotency_key(format!("left-{round}")),
                    )
                    .expect("enqueue");
            }
        }),
        std::thread::spawn(move || {
            for round in 0..10 {
                right_queue
                    .enqueue(
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
