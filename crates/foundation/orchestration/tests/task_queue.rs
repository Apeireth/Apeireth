//! 持久任务队列 (V6-a) 行为钉死: 状态机 / 幂等唤醒 / 崩溃恢复 / failed 人判 /
//! 环路熔断 / 结算接账 / 并发原子。
//!
//! 测试钉**行为**不钉字面: 每条断言对应设计承诺 (见 `task_queue` 模块头),
//! 持久层真落盘的重启证据在装配层的 sqlite 存储测试 (`task_queue_store.rs`)。

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use apeireth_orchestration::task_queue::{
    EnqueueOutcome, InMemoryTaskStore, SettlementOutcome, TaskCaller, TaskClockMs, TaskEvent,
    TaskQueue, TaskQueueConfig, TaskQueueError, TaskQueueResult, TaskRecord, TaskSettlement,
    TaskSpec, TaskState, TaskStore,
};
use apeireth_orchestration::work_consumption::{fold_consumed_work, WorkEvent, WorkKind};

/// 定值时钟 (确定性测试: 不吃真实时间)。
fn fixed_clock(start_ms: i64) -> TaskClockMs {
    let tick = Arc::new(AtomicI64::new(start_ms));
    Arc::new(move || tick.fetch_add(1, Ordering::SeqCst))
}

fn store() -> Arc<dyn TaskStore> {
    Arc::new(InMemoryTaskStore::new())
}

fn open(store: Arc<dyn TaskStore>) -> TaskQueue {
    TaskQueue::open_with_clock(
        store,
        TaskQueueConfig::default(),
        fixed_clock(1_700_000_000_000),
    )
    .expect("queue opens")
}

fn reopened(store: Arc<dyn TaskStore>) -> TaskQueue {
    // 「重启」= 丢弃旧句柄, 恢复点在 open。
    open(store)
}

fn worker() -> TaskCaller {
    TaskCaller::agent("worker-a")
}

fn human() -> TaskCaller {
    TaskCaller::human("owner")
}

// ============================================================================
// 1. 状态机: 全部转移显式, 非法转移拒绝即帧
// ============================================================================

#[test]
fn state_machine_walks_explicit_transitions_and_refuses_the_rest() {
    let store = store();
    let queue = open(Arc::clone(&store));

    let EnqueueOutcome::Enqueued { task, wake } = queue
        .enqueue(TaskSpec::new("写状态机测试", serde_json::json!({"n": 1})))
        .expect("enqueue")
    else {
        panic!("首次入队必须是新任务");
    };
    assert_eq!(task.state, TaskState::Pending);
    assert!(wake.is_none(), "无初始执行者即无唤醒");
    let id = task.id.clone();

    // Pending → Assigned (assign 即唤醒)。
    let assigned = queue.assign(&id, "worker-a", &worker()).expect("assign");
    assert_eq!(assigned.state, TaskState::Assigned);
    assert_eq!(assigned.assignee.as_deref(), Some("worker-a"));

    // Assigned → Running (attempts 计执行段)。
    let running = queue.begin_execution(&id, &worker()).expect("begin");
    assert_eq!(running.state, TaskState::Running);
    assert_eq!(running.attempts, 1);

    // Running → Done。
    let done = queue.complete(&id, &worker()).expect("complete");
    assert_eq!(done.state, TaskState::Done);

    // 终态之后: 指派/完成/人判 fail 全部拒绝即帧 (状态机不许隐式回退)。
    assert!(matches!(
        queue.assign(&id, "worker-b", &human()),
        Err(TaskQueueError::InvalidTransition { .. })
    ));
    assert!(matches!(
        queue.complete(&id, &worker()),
        Err(TaskQueueError::InvalidTransition { .. })
    ));
    assert!(matches!(
        queue.human_judge_fail(&id, &human(), "no"),
        Err(TaskQueueError::InvalidTransition { .. })
    ));

    // Pending 上的非法转移同样拒绝即帧。
    let EnqueueOutcome::Enqueued { task: second, .. } = queue
        .enqueue(TaskSpec::new("第二条", serde_json::Value::Null))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    assert!(matches!(
        queue.begin_execution(&second.id, &worker()),
        Err(TaskQueueError::InvalidTransition { .. })
    ));
    assert!(matches!(
        queue.complete(&second.id, &worker()),
        Err(TaskQueueError::InvalidTransition { .. })
    ));
}

// ============================================================================
// 2. 队列持久化: 重启恢复 running/assigned → pending + attempt 计数
// ============================================================================

#[test]
fn reopening_over_the_store_recovers_running_to_pending_with_attempt_accounting() {
    let store = store();
    let queue = open(Arc::clone(&store));
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("执行段会崩", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let id = task.id.clone();
    // 唤醒先取走 (= 正常投递过一次), 隔离「唤醒重投」计数。
    let wakes = queue.drain_wakes().expect("drain");
    assert_eq!(wakes.len(), 1);
    let running = queue.begin_execution(&id, &worker()).expect("begin");
    assert_eq!(running.state, TaskState::Running);
    assert_eq!(running.attempts, 1);

    // 「崩溃」: 旧句柄丢弃, 重启 = open 恢复点。
    drop(queue);
    let queue = reopened(Arc::clone(&store));
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
    assert_eq!(
        recovered.assignee.as_deref(),
        Some("worker-a"),
        "执行者不丢 (任务不丢主)"
    );
    assert!(
        queue
            .journal()
            .expect("journal")
            .iter()
            .any(|event| matches!(event, TaskEvent::RecoveredToPending { task_id, attempts: 2, .. } if task_id == &id)),
        "恢复要留审计"
    );
}

// ============================================================================
// 3. 幂等去重: 同 key 双投递 = 单执行
// ============================================================================

#[test]
fn same_idempotency_key_delivered_twice_executes_once() {
    let store = store();
    let queue = open(Arc::clone(&store));

    let first = queue
        .enqueue(TaskSpec::new("只干一次", serde_json::Value::Null).with_idempotency_key("job-42"))
        .expect("first delivery");
    let EnqueueOutcome::Enqueued { task, .. } = &first else {
        panic!("首次投递必须是新任务");
    };
    let id = task.id.clone();

    // 重复投递: 不产生第二个任务, 只计重投递。
    let second = queue
        .enqueue(
            TaskSpec::new("只干一次 (重投)", serde_json::Value::Null)
                .with_idempotency_key("job-42"),
        )
        .expect("second delivery");
    assert_eq!(
        second,
        EnqueueOutcome::Deduplicated {
            task_id: id.clone(),
            redelivered: 1,
        },
        "同 key 重复投递 = 不重复执行"
    );
    assert_eq!(queue.list().expect("list").len(), 1, "队列里只有一条任务");
    assert_eq!(queue.get(&id).expect("get").redelivered, 1);

    // 执行段照常只有一段 (单执行)。
    queue.assign(&id, "worker-a", &worker()).expect("assign");
    queue.begin_execution(&id, &worker()).expect("begin");
    queue.complete(&id, &worker()).expect("complete");
    assert_eq!(queue.get(&id).expect("get").attempts, 1, "双投递只执行一段");
    assert!(
        queue
            .journal()
            .expect("journal")
            .iter()
            .any(|event| matches!(event, TaskEvent::DeliveryDeduplicated { key, redelivered: 1, .. } if key == "job-42")),
        "幂等去重要留审计"
    );
}

// ============================================================================
// 4. failed 只能人判: 失败不自动重试, 重试是显式动作
// ============================================================================

#[test]
fn failed_tasks_never_retry_automatically_and_only_human_judgment_moves_them() {
    let store = store();
    let queue = open(Arc::clone(&store));
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("会失败的活", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let id = task.id.clone();
    queue.drain_wakes().expect("drain"); // 入队唤醒先正常投递一次
    queue.begin_execution(&id, &worker()).expect("begin");

    // agent 碰不到人判面 (failed 只能人判)。
    assert!(matches!(
        queue.human_judge_fail(&id, &worker(), "我想判它失败"),
        Err(TaskQueueError::HumanOnly { .. })
    ));
    assert!(matches!(
        queue.human_judge_retry(&id, &worker(), "我想重试"),
        Err(TaskQueueError::HumanOnly { .. })
    ));

    // 人判失败 = 唯一失败入口。
    let failed = queue
        .human_judge_fail(&id, &human(), "验收不过")
        .expect("human fail");
    assert_eq!(failed.state, TaskState::Failed);

    // 失败不自动重试: 重启恢复不触碰 Failed, 队列不自动再派。
    drop(queue);
    let queue = reopened(Arc::clone(&store));
    assert_eq!(
        queue.get(&id).expect("get").state,
        TaskState::Failed,
        "崩溃恢复绝不把失败当未处理自动重试"
    );
    assert_eq!(
        queue.get(&id).expect("get").attempts,
        1,
        "失败之后零自动重派"
    );

    // 重试 = 显式人判动作 (回队待派, 不自动唤醒)。
    let retried = queue
        .human_judge_retry(&id, &human(), "修好了再试")
        .expect("human retry");
    assert_eq!(retried.state, TaskState::Pending);
    assert!(
        queue.drain_wakes().expect("drain").is_empty(),
        "重试不自动唤醒"
    );
    assert!(
        queue
            .journal()
            .expect("journal")
            .iter()
            .any(|event| matches!(event, TaskEvent::RetryJudged { judge, .. } if judge == "owner")),
        "人判重试要留审计"
    );
}

// ============================================================================
// 5. 环路熔断: 同任务链深度上限, 超限拒绝即帧 + 审计
// ============================================================================

#[test]
fn spawn_chain_depth_fuse_refuses_deep_chains_and_audits() {
    let store = store();
    let queue = TaskQueue::open_with_clock(
        Arc::clone(&store),
        TaskQueueConfig {
            max_attempts: 3,
            max_chain_depth: 2,
        },
        fixed_clock(1_700_000_000_000),
    )
    .expect("queue opens");

    // 链: 根(0) → 子(1) → 孙(2); 再深一层熔断。
    // 跨线补位 (机械空白, 语义 0 改): clippy `manual_let_else` 门禁断点按 clippy
    // 建议的等价 let...else 改写; 任务簿语义归口任务线。
    let EnqueueOutcome::Enqueued {
        task: mut parent, ..
    } = queue
        .enqueue(TaskSpec::new("根", serde_json::Value::Null))
        .expect("root")
    else {
        panic!("根任务必须是新任务")
    };
    for depth in 1..=2 {
        parent = match queue
            .enqueue(
                TaskSpec::new(format!("第 {depth} 层"), serde_json::Value::Null)
                    .with_parent(parent.id.clone()),
            )
            .expect("child within fuse")
        {
            EnqueueOutcome::Enqueued { task, .. } => {
                assert_eq!(task.chain_depth, depth);
                task
            }
            _ => panic!("熔断深度内的子任务必须入队"),
        };
    }

    // 第 3 层 = 超上限: 拒绝即帧 + 审计照样落盘。
    let refused = queue
        .enqueue(TaskSpec::new("第 3 层", serde_json::Value::Null).with_parent(parent.id.clone()));
    let Err(TaskQueueError::ChainDepthExceeded {
        depth, max_depth, ..
    }) = refused
    else {
        panic!("超深链必须拒绝即帧: {refused:?}");
    };
    assert_eq!((depth, max_depth), (3, 2));
    assert!(
        queue
            .journal()
            .expect("journal")
            .iter()
            .any(|event| matches!(
                event,
                TaskEvent::ChainRefused {
                    depth: 3,
                    max_depth: 2,
                    ..
                }
            )),
        "环路熔断必须留审计"
    );
    assert_eq!(queue.list().expect("list").len(), 3, "被熔断的不落队");
}

/// 默认熔断深度 = 5 层 (设计裁定 R1⑳): 默认配置 5 层链放行、第 6 层拒绝即帧;
/// 显式配置照常覆盖 (旧行为兼容: 旧 4 层口径可显式表达)。
#[test]
fn default_chain_depth_fuse_is_five_layers_and_explicit_config_still_wins() {
    assert_eq!(
        TaskQueueConfig::default().max_chain_depth,
        5,
        "环路熔断默认深度 = 5 (设计裁定 R1⑳)"
    );
    let store = store();
    let queue = TaskQueue::open_with_clock(
        Arc::clone(&store),
        TaskQueueConfig::default(),
        fixed_clock(1_700_000_000_000),
    )
    .expect("queue opens");

    let EnqueueOutcome::Enqueued {
        task: mut parent, ..
    } = queue
        .enqueue(TaskSpec::new("根", serde_json::Value::Null))
        .expect("root")
    else {
        panic!("根任务必须是新任务")
    };
    for depth in 1..=5 {
        parent = match queue
            .enqueue(
                TaskSpec::new(format!("第 {depth} 层"), serde_json::Value::Null)
                    .with_parent(parent.id.clone()),
            )
            .expect("child within the default fuse")
        {
            EnqueueOutcome::Enqueued { task, .. } => {
                assert_eq!(task.chain_depth, depth);
                task
            }
            _ => panic!("默认熔断深度内的子任务必须入队"),
        };
    }
    let refused = queue
        .enqueue(TaskSpec::new("第 6 层", serde_json::Value::Null).with_parent(parent.id.clone()));
    assert!(
        matches!(
            refused,
            Err(TaskQueueError::ChainDepthExceeded {
                depth: 6,
                max_depth: 5,
                ..
            })
        ),
        "默认 5 层熔断: 第 6 层必须拒绝即帧: {refused:?}"
    );

    // 显式配置覆盖 (旧行为兼容: 显式 1 层照常熔断, 默认值不锁死)。
    let explicit = TaskQueue::open_with_clock(
        store,
        TaskQueueConfig {
            max_attempts: 3,
            max_chain_depth: 1,
        },
        fixed_clock(1_700_000_000_000),
    )
    .expect("queue opens");
    let EnqueueOutcome::Enqueued { task: root, .. } = explicit
        .enqueue(TaskSpec::new("显式根", serde_json::Value::Null))
        .expect("root")
    else {
        panic!("根任务必须是新任务")
    };
    let EnqueueOutcome::Enqueued { task: child, .. } = explicit
        .enqueue(TaskSpec::new("显式子", serde_json::Value::Null).with_parent(root.id.clone()))
        .expect("显式 1 层内的子任务入队")
    else {
        panic!("显式熔断深度内的子任务必须是新任务")
    };
    let refused = explicit
        .enqueue(TaskSpec::new("显式孙", serde_json::Value::Null).with_parent(child.id.clone()));
    assert!(
        matches!(
            refused,
            Err(TaskQueueError::ChainDepthExceeded {
                depth: 2,
                max_depth: 1,
                ..
            })
        ),
        "显式配置照常覆盖默认深度: {refused:?}"
    );
}

// ============================================================================
// 6. 结算挂账: 一回合一段, 无重复记账 (金额沿三维账本既有口径)
// ============================================================================

#[test]
fn settlement_linking_is_idempotent_and_carries_no_cost() {
    let store = store();
    let queue = open(Arc::clone(&store));
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("回合执行段", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let id = task.id.clone();
    queue.begin_execution(&id, &worker()).expect("begin");

    // 三维账本既有口径: 回合消耗在事件日志里 (本模块只接账)。
    let mut ledger = apeireth_orchestration::work_consumption::WorkLog::new();
    ledger.append(WorkEvent::TurnStarted {
        turn: 7,
        at_ms: 1_700_000_000_100,
    });
    ledger.append(WorkEvent::Consumed {
        turn: 7,
        at_ms: 1_700_000_000_200,
        kind: WorkKind::ToolCall,
        units: 5,
    });
    ledger.append(WorkEvent::Finished {
        turn: 7,
        at_ms: 1_700_000_000_300,
    });

    let link = TaskSettlement {
        turn: 7,
        ledger_ref: "work_log".to_string(),
        linked_at_ms: 1_700_000_000_400,
    };
    let outcome = queue.link_settlement(&id, link.clone()).expect("link");
    assert_eq!(
        outcome,
        SettlementOutcome::Linked {
            task_id: id.clone(),
            turn: 7
        }
    );

    // 同一回合重复挂账 = 不重复记账。
    let again = queue.link_settlement(&id, link).expect("link again");
    assert_eq!(
        again,
        SettlementOutcome::AlreadyLinked {
            task_id: id.clone(),
            turn: 7
        }
    );
    let record = queue.get(&id).expect("get");
    assert_eq!(record.settlements.len(), 1, "一回合一段, 无重复记账");

    // 只接账不重复记: 任务账没有金额字段, 花不花钱由三维账本折叠回答
    // (取消/中断同答案 —— 挂账不改账本)。
    let consumed = fold_consumed_work(7, ledger.events());
    assert_eq!(consumed.units, 5);
    assert_eq!(consumed.tool_calls, 1);
    let json = serde_json::to_value(&record.settlements[0]).expect("settlement serializes");
    assert!(json.get("units").is_none() && json.get("cost").is_none());
    assert!(
        queue
            .journal()
            .expect("journal")
            .iter()
            .any(|event| matches!(event, TaskEvent::SettlementDuplicateIgnored { turn: 7, .. })),
        "无重复记账要留审计"
    );
}

// ============================================================================
// 7. 事件唤醒: assign 即唤醒, 唤醒信号与状态事件同汇
// ============================================================================

#[test]
fn assign_emits_the_wake_through_the_same_event_stream() {
    let store = store();
    let queue = open(Arc::clone(&store));
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("等唤醒的活", serde_json::Value::Null))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let id = task.id.clone();

    let assigned = queue.assign(&id, "worker-a", &worker()).expect("assign");
    assert_eq!(assigned.state, TaskState::Assigned);

    // 唤醒体在持久唤醒队列, 信号在事件汇 (同一本账)。
    let wakes = queue.drain_wakes().expect("drain");
    assert_eq!(wakes.len(), 1);
    assert_eq!(wakes[0].task_id, id);
    assert_eq!(wakes[0].assignee, "worker-a");
    let journal = queue.journal().expect("journal");
    assert!(journal
        .iter()
        .any(|event| matches!(event, TaskEvent::Assigned { task_id, .. } if task_id == &id)));
    assert!(
        journal
            .iter()
            .any(|event| matches!(event, TaskEvent::WakeEmitted { task_id, .. } if task_id == &id)),
        "唤醒信号与状态事件必须同汇"
    );

    // 取走即已投递, 不重复唤醒。
    assert!(queue.drain_wakes().expect("drain").is_empty());
}

#[test]
fn undelivered_wakes_are_redelivered_after_restart() {
    let store = store();
    let queue = open(Arc::clone(&store));
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("没人收的唤醒", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let id = task.id.clone();
    // 不取唤醒就「崩溃」。
    drop(queue);

    let queue = reopened(Arc::clone(&store));
    assert_eq!(
        queue.get(&id).expect("get").redelivered,
        1,
        "未投递唤醒重启后重投, 计 redelivered"
    );
    let wakes = queue.drain_wakes().expect("drain");
    assert_eq!(wakes.len(), 1, "唤醒不丢");
    assert!(
        queue.journal().expect("journal").iter().any(
            |event| matches!(event, TaskEvent::WakeRedelivered { task_id, .. } if task_id == &id)
        ),
        "重投递要留审计"
    );
}

// ============================================================================
// 8. 并发入队原子: 多手同店互不覆盖
// ============================================================================

#[test]
fn concurrent_enqueues_are_atomic_and_lose_no_tasks() {
    let store: Arc<dyn TaskStore> = Arc::new(InMemoryTaskStore::new());
    let handles: Vec<Arc<TaskQueue>> = (0..8)
        .map(|_| {
            Arc::new(
                TaskQueue::open_with_clock(
                    Arc::clone(&store),
                    TaskQueueConfig::default(),
                    fixed_clock(1_700_000_000_000),
                )
                .expect("queue opens"),
            )
        })
        .collect();

    let threads: Vec<_> = handles
        .iter()
        .enumerate()
        .map(|(index, queue)| {
            let queue = Arc::clone(queue);
            std::thread::spawn(move || {
                for round in 0..8 {
                    queue
                        .enqueue(
                            TaskSpec::new(
                                format!("并发入队 {index}-{round}"),
                                serde_json::Value::Null,
                            )
                            .with_idempotency_key(format!("key-{index}-{round}")),
                        )
                        .expect("enqueue");
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("thread finishes");
    }

    // 8 手 × 8 条 = 64 条, 一条不丢 (原子读改写)。
    let tasks = handles[0].list().expect("list");
    assert_eq!(tasks.len(), 64, "并发入队不得丢任务");
    assert_eq!(
        handles[0].write_seq().expect("write_seq"),
        64,
        "写派号单调不跳号"
    );
}

// ============================================================================
// 9. 认领授权: 自报必须是本人
// ============================================================================

#[test]
fn self_reports_must_come_from_the_assignee() {
    let store = store();
    let queue = open(Arc::clone(&store));
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("有主的活", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let id = task.id.clone();
    let intruder = TaskCaller::agent("worker-b");

    // 委托者/旁人不能伪造工人状态。
    assert!(matches!(
        queue.begin_execution(&id, &intruder),
        Err(TaskQueueError::NotAssignee { .. })
    ));
    assert!(matches!(
        queue.complete(&id, &intruder),
        Err(TaskQueueError::NotAssignee { .. })
    ));
    assert!(matches!(
        queue.block(&id, &intruder, "伪造阻塞"),
        Err(TaskQueueError::NotAssignee { .. })
    ));

    // 本人可以; 人可以代推进 (终审权)。
    assert_eq!(
        queue.begin_execution(&id, &worker()).expect("begin").state,
        TaskState::Running
    );
    assert_eq!(
        queue.block(&id, &worker(), "等外部").expect("block").state,
        TaskState::Blocked
    );
    assert_eq!(
        queue
            .assign(&id, "worker-a", &human())
            .expect("assign")
            .state,
        TaskState::Assigned
    );
    assert_eq!(
        queue.complete(&id, &human()).expect("human complete").state,
        TaskState::Done
    );
}

// ============================================================================
// 10. 自述节真值: 分态计数
// ============================================================================

#[test]
fn counts_report_each_state_truthfully() {
    let store = store();
    let queue = open(Arc::clone(&store));

    // pending / assigned / running / done / failed / blocked 各一条。
    queue
        .enqueue(TaskSpec::new("待派", serde_json::Value::Null))
        .expect("enqueue");
    let EnqueueOutcome::Enqueued { task: assigned, .. } = queue
        .enqueue(TaskSpec::new("已派", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let EnqueueOutcome::Enqueued { task: running, .. } = queue
        .enqueue(TaskSpec::new("在跑", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    queue
        .begin_execution(&running.id, &worker())
        .expect("begin");
    let EnqueueOutcome::Enqueued { task: done, .. } = queue
        .enqueue(TaskSpec::new("干完", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    queue.complete(&done.id, &worker()).expect("complete");
    let EnqueueOutcome::Enqueued { task: failed, .. } = queue
        .enqueue(TaskSpec::new("人判失败", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    queue
        .human_judge_fail(&failed.id, &human(), "验收不过")
        .expect("fail");
    let EnqueueOutcome::Enqueued { task: blocked, .. } = queue
        .enqueue(TaskSpec::new("外部等待", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    queue
        .block(&blocked.id, &worker(), "等外部")
        .expect("block");

    let counts = queue.counts().expect("counts");
    assert_eq!(counts.pending, 1);
    assert_eq!(counts.assigned, 1, "已派未开始的在 assigned");
    assert_eq!(counts.running, 1);
    assert_eq!(counts.done, 1);
    assert_eq!(counts.failed, 1);
    assert_eq!(counts.blocked, 1);
    let _ = assigned;
}

// ============================================================================
// 11. 派发熔断: 崩溃死循环有上限 (attempt 计数防死循环)
// ============================================================================

#[test]
fn attempt_fuse_blocks_endless_crash_recovery_loops() {
    let store = store();
    let config = TaskQueueConfig {
        max_attempts: 2,
        max_chain_depth: 4,
    };
    let queue =
        TaskQueue::open_with_clock(Arc::clone(&store), config, fixed_clock(1_700_000_000_000))
            .expect("queue opens");
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("反复崩", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let id = task.id.clone();
    queue.drain_wakes().expect("drain");
    queue.begin_execution(&id, &worker()).expect("begin");

    // 第一次恢复: 回 pending 重派 (attempts 1 → 2, 未超上限 2)。
    drop(queue);
    let queue =
        TaskQueue::open_with_clock(Arc::clone(&store), config, fixed_clock(1_700_000_000_000))
            .expect("queue opens");
    let recovered = queue.get(&id).expect("get");
    assert_eq!(recovered.state, TaskState::Pending);
    assert_eq!(recovered.attempts, 2);

    // 再派一次又崩: 第二次恢复超上限 (attempts 2 → 3 > 2) → 熔断为 Blocked。
    queue.assign(&id, "worker-a", &worker()).expect("assign");
    drop(queue);
    let queue =
        TaskQueue::open_with_clock(Arc::clone(&store), config, fixed_clock(1_700_000_000_000))
            .expect("queue opens");
    let fused = queue.get(&id).expect("get");
    assert_eq!(fused.state, TaskState::Blocked, "防崩溃死循环: 超上限熔断");
    assert_eq!(fused.attempts, 3);
    assert!(
        queue.journal().expect("journal").iter().any(
            |event| matches!(event, TaskEvent::AttemptFused { task_id, .. } if task_id == &id)
        ),
        "熔断要留审计"
    );
}

// ============================================================================
// 12. 载荷与幂等键的输入校验 (错误即帧)
// ============================================================================

#[test]
fn invalid_specs_are_rejected_as_frames_not_silently_accepted() {
    let queue = open(store());
    let result: TaskQueueResult<EnqueueOutcome> =
        queue.enqueue(TaskSpec::new("   ", serde_json::Value::Null));
    assert!(matches!(result, Err(TaskQueueError::InvalidInput { .. })));
    let result =
        queue.enqueue(TaskSpec::new("空键", serde_json::Value::Null).with_idempotency_key("  "));
    assert!(matches!(result, Err(TaskQueueError::InvalidInput { .. })));
    // 不存在的任务 = NotFound (不静默建)。
    let missing: TaskQueueResult<TaskRecord> = queue.complete("task-999", &human());
    assert!(matches!(missing, Err(TaskQueueError::NotFound { .. })));
}

// ============================================================================
// 13. 面板一次快照读: 任务表 + 计数 + 事件账同一刻 (与 list/counts 同口径)
// ============================================================================

#[test]
fn snapshot_reads_tasks_counts_and_journal_in_one_moment() {
    let store = store();
    let queue = open(Arc::clone(&store));
    let EnqueueOutcome::Enqueued { task: first, .. } = queue
        .enqueue(TaskSpec::new("快照甲", serde_json::json!({"step": 1})))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    let EnqueueOutcome::Enqueued { task: second, .. } = queue
        .enqueue(TaskSpec::new("快照乙", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    queue
        .human_judge_fail(&first.id, &human(), "验收不过")
        .expect("fail");

    let snapshot = queue.snapshot().expect("snapshot");
    // 排序与 list 同口径 (入队时刻, 同刻按 id)。
    let listed: Vec<String> = queue
        .list()
        .expect("list")
        .iter()
        .map(|task| task.id.clone())
        .collect();
    let snapshotted: Vec<String> = snapshot.tasks.iter().map(|task| task.id.clone()).collect();
    assert_eq!(snapshotted, listed);
    // 计数与 counts 同口径 (一次读里同出, 不混两刻)。
    assert_eq!(snapshot.counts, queue.counts().expect("counts"));
    assert_eq!(snapshot.counts.failed, 1);
    assert_eq!(snapshot.counts.assigned, 1);
    // 事件账随同一刻读出: 人判失败审计在列。
    assert!(snapshot.journal.iter().any(
        |event| matches!(event, TaskEvent::FailureJudged { task_id, .. } if task_id == &first.id)
    ));
    assert!(snapshot.journal.iter().any(
        |event| matches!(event, TaskEvent::Enqueued { task_id, .. } if task_id == &second.id)
    ));
}

// ============================================================================
// 14. 附从句柄: 同店第二把句柄不重复崩溃恢复 (不假记审计 / 不重派)
// ============================================================================

#[test]
fn attached_handles_never_rerun_crash_recovery() {
    let store = store();
    let queue = open(Arc::clone(&store));
    let EnqueueOutcome::Enqueued { task, .. } = queue
        .enqueue(TaskSpec::new("在跑任务", serde_json::Value::Null).with_assignee("worker-a"))
        .expect("enqueue")
    else {
        panic!("必须是新任务");
    };
    queue.drain_wakes().expect("drain");
    queue.begin_execution(&task.id, &worker()).expect("begin");
    let before = queue.journal().expect("journal").len();

    // 附从句柄 (旁路面板): Running 原样在跑, 事件账不增, redelivered 不涨。
    let attached = TaskQueue::attach_with_clock(
        Arc::clone(&store),
        TaskQueueConfig::default(),
        fixed_clock(1_700_000_000_000),
    );
    let live = attached.get(&task.id).expect("get");
    assert_eq!(live.state, TaskState::Running, "附从句柄不重派在跑任务");
    assert_eq!(live.redelivered, 0);
    assert_eq!(
        attached.journal().expect("journal").len(),
        before,
        "附从句柄不产生任何恢复审计"
    );
}
