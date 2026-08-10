//! Fixture 5: in-process Task Service 验证 (per RIVAL 蓝图 §3.7 缺口 5)
//!
//! 测 3 类事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. **m3 防御**: `TOOL_WHITELIST` 编译期 hardcode 包含 8 task 工具
//! 2. **K-1 强校验**: 5 字样 (apeireth / task / state / dag / must-do) 编译期 hardcode
//! 3. **8 编译期 hardcode**: schema / platform / 7 状态 / 5 优先级 / 重试 / 超时 / DAG 深度
//!
//! 5 P0 + 9 skeleton crate 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).

use apeireth_task::{
    dag::{NodeKind, TaskDag},
    queue::PriorityTaskQueue,
    scheduler::{DagScheduler, RetryPolicy},
    state_machine::TaskStateMachine,
    validate_k1_invariant, validate_tool_call, k1_invariant_5_keys, TaskError, TaskId,
    TaskPriority, TaskState, MAX_DAG_DEPTH, MAX_RETRIES_DEFAULT, PLATFORM_NAME,
    RETRY_BACKOFF_MS, SUPPORTED_PRIORITIES, SUPPORTED_STATES, TASK_PRIORITY_COUNT,
    TASK_SCHEMA_VERSION, TASK_STATE_COUNT, TASK_TIMEOUT_DEFAULT_MS, TOOL_COUNT, TOOL_WHITELIST,
};

// ====================================================================
// 类别 1: m3 防御 (TOOL_WHITELIST 8 工具)
// ====================================================================

#[test]
fn test_whitelist_contains_eight_task_tools() {
    assert_eq!(TOOL_WHITELIST.len(), 8, "8 工具");
    assert_eq!(TOOL_WHITELIST.len(), TOOL_COUNT, "TOOL_COUNT = 8");
    for tool in [
        "apeireth_task_create",
        "apeireth_task_submit",
        "apeireth_task_cancel",
        "apeireth_task_list",
        "apeireth_task_get",
        "apeireth_task_dag_validate",
        "apeireth_task_schedule",
        "apeireth_task_retry",
    ] {
        assert!(
            TOOL_WHITELIST.contains(&tool),
            "TOOL_WHITELIST 缺: {tool}"
        );
    }
}

#[test]
fn test_validate_tool_call_accepts_whitelisted() {
    let args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        let result = validate_tool_call(tool, &args);
        assert!(result.is_ok(), "白名单工具 {tool} 应通过: {result:?}");
    }
}

#[test]
fn test_validate_tool_call_rejects_unknown() {
    // m3 hallucination 防御: 不在白名单的工具必须拒绝
    let args = serde_json::json!({});
    let result = validate_tool_call("apeireth_task_purge", &args);
    assert!(result.is_err(), "白名单外工具必须拒绝");
    match result.unwrap_err() {
        TaskError::ToolNotWhitelisted(t) => {
            assert_eq!(t, "apeireth_task_purge");
        }
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}

// ====================================================================
// 类别 2: K-1 强校验 5 字样 (per supervisor-prompt-818 §5.3 模式)
// ====================================================================

#[test]
fn k1_invariant_5_keys_present() {
    let keys = k1_invariant_5_keys();
    assert_eq!(keys.len(), 5, "K-1 必须 5 字样");
    assert!(keys.contains(&"apeireth"), "K-1 缺 'apeireth' (平台名)");
    assert!(keys.contains(&"task"), "K-1 缺 'task' (模块名)");
    assert!(keys.contains(&"state"), "K-1 缺 'state' (核心 API)");
    assert!(keys.contains(&"dag"), "K-1 缺 'dag' (核心 API)");
    assert!(keys.contains(&"must-do"), "K-1 缺 'must-do' (翻译 invariant)");
}

#[test]
fn k1_invariant_validate_function_passes() {
    assert!(
        validate_k1_invariant().is_ok(),
        "validate_k1_invariant 必须 Ok"
    );
}

// ====================================================================
// 类别 3: 8 编译期 hardcode 守门
// ====================================================================

#[test]
fn test_eight_compile_time_hardcodes() {
    // #1 schema
    assert_eq!(TASK_SCHEMA_VERSION, "1", "TASK_SCHEMA_VERSION = 1");
    // #2 platform
    assert_eq!(PLATFORM_NAME, "apeireth", "PLATFORM_NAME = apeireth");
    // #3 7 状态
    assert_eq!(SUPPORTED_STATES.len(), 7, "7 TaskState");
    assert_eq!(TASK_STATE_COUNT, 7, "TASK_STATE_COUNT = 7");
    for s in [
        TaskState::Pending,
        TaskState::Queued,
        TaskState::Running,
        TaskState::Completed,
        TaskState::Failed,
        TaskState::Cancelled,
        TaskState::Timeout,
    ] {
        assert!(SUPPORTED_STATES.contains(&s), "7 状态缺: {s:?}");
    }
    // #4 5 优先级
    assert_eq!(SUPPORTED_PRIORITIES.len(), 5, "5 TaskPriority");
    assert_eq!(TASK_PRIORITY_COUNT, 5, "TASK_PRIORITY_COUNT = 5");
    // #5 max retries
    assert_eq!(MAX_RETRIES_DEFAULT, 3, "MAX_RETRIES_DEFAULT = 3");
    // #6 backoff
    assert_eq!(RETRY_BACKOFF_MS, 1000, "RETRY_BACKOFF_MS = 1000");
    // #7 timeout
    assert_eq!(TASK_TIMEOUT_DEFAULT_MS, 30_000, "TASK_TIMEOUT_DEFAULT_MS = 30s");
    // #8 dag depth
    assert_eq!(MAX_DAG_DEPTH, 32, "MAX_DAG_DEPTH = 32");
}

#[test]
fn test_retry_policy_default_matches_hardcodes() {
    let p = RetryPolicy::default();
    assert_eq!(p.max_retries, MAX_RETRIES_DEFAULT);
    assert_eq!(p.backoff_ms, RETRY_BACKOFF_MS);
    assert_eq!(p.timeout_ms, TASK_TIMEOUT_DEFAULT_MS);
}

// ====================================================================
// 类别 4: 核心 Fixture: t_dag_task_in_process
// 构造 3 节点 DAG, 端到端验证 8 编译期 hardcode + 8 工具 + K-1 5 字样
// ====================================================================

#[test]
fn t_dag_task_in_process() {
    println!("=== t_dag_task_in_process ===");
    println!("构造 3 节点 DAG (Input → Tool → Output), 验证 8 hardcode + K-1 + m3");

    // 场景 A: DAG 构造
    let mut dag = TaskDag::new();
    let a = TaskId::new();
    let b = TaskId::new();
    let c = TaskId::new();
    dag.add_node(a.clone(), NodeKind::Input, "input");
    dag.add_node(b.clone(), NodeKind::Tool, "tool");
    dag.add_node(c.clone(), NodeKind::Output, "output");
    dag.add_edge(a.clone(), b.clone()).unwrap();
    dag.add_edge(b.clone(), c.clone()).unwrap();
    assert!(!dag.detect_cycle());
    let topo = dag.topological_sort().expect("topo sort");
    assert_eq!(topo.len(), 3);
    println!("[A] 3 节点 DAG 拓扑序 ✓ len={}", topo.len());

    // 场景 B: DAG 深度守门
    dag.validate_depth().expect("depth ≤ 32");
    println!("[B] DAG 深度守门 ≤ MAX_DAG_DEPTH ({MAX_DAG_DEPTH}) ✓");

    // 场景 C: Priority queue 5 优先级
    let mut pq = PriorityTaskQueue::new();
    let bg = TaskId::new();
    let cr = TaskId::new();
    let nm = TaskId::new();
    pq.enqueue(bg.clone(), TaskPriority::Background);
    pq.enqueue(nm.clone(), TaskPriority::Normal);
    pq.enqueue(cr.clone(), TaskPriority::Critical);
    assert_eq!(pq.dequeue(), Some(cr.clone()));
    assert_eq!(pq.dequeue(), Some(nm.clone()));
    assert_eq!(pq.dequeue(), Some(bg.clone()));
    println!("[C] 5 优先级 strict order (Critical > Normal > Background) ✓");

    // 场景 D: State machine 7 状态
    let mut sm = TaskStateMachine::new(a.clone(), 1000);
    sm.transition(TaskState::Queued, 1001).unwrap();
    sm.transition(TaskState::Running, 1002).unwrap();
    sm.transition(TaskState::Completed, 1003).unwrap();
    assert!(sm.is_terminal());
    println!("[D] State machine: Pending → Queued → Running → Completed ✓");

    // 场景 E: Scheduler 注册 + 重试守门
    let mut sched = DagScheduler::new();
    let id = sched.register(vec![], TaskPriority::Normal, 1000).unwrap();
    // 手动转 Failed 后重试 max_retries 次
    {
        let sm = sched.state_machines.get_mut(&id).unwrap();
        sm.transition(TaskState::Queued, 1001).unwrap();
        sm.transition(TaskState::Running, 1002).unwrap();
        sm.transition(TaskState::Failed, 1003).unwrap();
    }
    for i in 0..MAX_RETRIES_DEFAULT {
        assert!(sched.retry(&id, 2000 + i as i64).is_ok(), "重试 {i} 必 Ok");
    }
    // 第 4 次必拒
    let r = sched.retry(&id, 3000);
    assert!(matches!(r, Err(TaskError::MaxRetriesExceeded { .. })));
    println!("[E] Scheduler 重试守门: {MAX_RETRIES_DEFAULT} 次后拒 ✓");

    // 场景 F: m3 防御拒绝虚构
    let bad = validate_tool_call("apeireth_task_purge", &serde_json::json!({}));
    assert!(bad.is_err());
    println!("[F] m3 防御: 拒绝虚构工具 ✓");

    // 场景 G: K-1 5 字样
    let keys = k1_invariant_5_keys();
    assert_eq!(keys.len(), 5);
    assert!(validate_k1_invariant().is_ok());
    println!("[G] K-1 5 字样 (apeireth/task/state/dag/must-do) ✓");

    // 场景 H: 8 编译期 hardcode 报告
    assert_eq!(TASK_SCHEMA_VERSION, "1");
    assert_eq!(PLATFORM_NAME, "apeireth");
    assert_eq!(SUPPORTED_STATES.len(), 7);
    assert_eq!(SUPPORTED_PRIORITIES.len(), 5);
    assert_eq!(MAX_RETRIES_DEFAULT, 3);
    assert_eq!(RETRY_BACKOFF_MS, 1000);
    assert_eq!(TASK_TIMEOUT_DEFAULT_MS, 30_000);
    assert_eq!(MAX_DAG_DEPTH, 32);
    println!("[H] 8 编译期 hardcode 报告 ✓");

    println!("=== t_dag_task_in_process 全部通过 ===");
}
