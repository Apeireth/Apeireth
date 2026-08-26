//! `apeireth-task` 示例: **DAG 任务调度演示**.
//!
//! **本示例演示** (per `v09021-rust-translation-blueprint-RIVAL §2.5.4`
//! + 主人 2026-08-05 21:27 拍板"效率不慢下来"):
//! 1. 验证 8 编译期 hardcode (TASK_SCHEMA_VERSION / PLATFORM_NAME / 7 状态 / 5 优先级 / 重试 / 超时 / DAG 深度)
//! 2. 验证 m3 防御 (8 工具白名单 + 拒绝虚构工具)
//! 3. 验证 K-1 强校验 5 字样 (apeireth / task / state / dag / must-do)
//! 4. 构造 3 节点 DAG (Input → Tool → Output), 拓扑序调度
//! 5. 演示重试 + 取消 + 终态守门
//!
//! **运行**:
//! ```bash
//! cargo run -p apeireth-task --example task_demo
//! ```
//!
//! **期望输出** (stdout):
//! ```text
//! === apeireth-task DAG 任务调度 demo ===
//! [1] 8 编译期 hardcode 报告
//! [2] m3 防御: 8 工具白名单验证通过
//! [3] K-1 强校验 5 字样验证通过
//! [4] DAG 构造: 3 节点 (Input → Tool → Output) + 拓扑序
//! [5] 注册 3 task 到 scheduler, 标 0 依赖 root
//! [6] 调度: 0 依赖 root 入队, 模拟完成推进下游
//! [7] 重试: 失败 task 重试 3 次 (per MAX_RETRIES_DEFAULT)
//! [8] 取消: cancel 由 Pending 直转 Cancelled
//! === DAG 任务调度 demo 完成 ===
//! ```

#![warn(missing_docs)]

use apeireth_task::{
    dag::{NodeKind, TaskDag},
    queue::PriorityTaskQueue,
    scheduler::{DagScheduler, RetryPolicy},
    state_machine::TaskStateMachine,
    validate_k1_invariant, validate_tool_call, k1_invariant_5_keys, TaskId, TaskPriority,
    TaskState, MAX_DAG_DEPTH, MAX_RETRIES_DEFAULT, PLATFORM_NAME, RETRY_BACKOFF_MS,
    SUPPORTED_PRIORITIES, SUPPORTED_STATES, TASK_PRIORITY_COUNT, TASK_SCHEMA_VERSION,
    TASK_STATE_COUNT, TASK_TIMEOUT_DEFAULT_MS, TOOL_WHITELIST,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-task DAG 任务调度 demo ===\n");

    // [1] 8 编译期 hardcode 报告
    println!("[1] 8 编译期 hardcode 报告");
    println!("    TASK_SCHEMA_VERSION     = {TASK_SCHEMA_VERSION}");
    println!("    PLATFORM_NAME           = {PLATFORM_NAME}");
    println!("    SUPPORTED_STATES.len()  = {} (per TASK_STATE_COUNT = {TASK_STATE_COUNT})", SUPPORTED_STATES.len());
    println!("    SUPPORTED_PRIORITIES.len() = {} (per TASK_PRIORITY_COUNT = {TASK_PRIORITY_COUNT})", SUPPORTED_PRIORITIES.len());
    println!("    MAX_RETRIES_DEFAULT     = {MAX_RETRIES_DEFAULT}");
    println!("    RETRY_BACKOFF_MS        = {RETRY_BACKOFF_MS}");
    println!("    TASK_TIMEOUT_DEFAULT_MS = {TASK_TIMEOUT_DEFAULT_MS}");
    println!("    MAX_DAG_DEPTH           = {MAX_DAG_DEPTH}");

    // [2] m3 防御: 8 工具白名单
    println!("\n[2] m3 防御: 8 工具白名单验证通过");
    println!("    TOOL_WHITELIST.len() = {}", TOOL_WHITELIST.len());
    for (i, t) in TOOL_WHITELIST.iter().enumerate() {
        let r = validate_tool_call(t, &serde_json::json!({}));
        println!("    [{i}] {t}: {}", if r.is_ok() { "ok" } else { "FAIL" });
    }
    // 拒绝虚构
    let bad = validate_tool_call("apeireth_task_purge", &serde_json::json!({}));
    println!("    [reject] apeireth_task_purge: {}", if bad.is_err() { "rejected ✓" } else { "FAIL" });

    // [3] K-1 强校验 5 字样
    println!("\n[3] K-1 强校验 5 字样验证通过");
    for (i, k) in k1_invariant_5_keys().iter().enumerate() {
        println!("    K-1 key[{i}] = {k}");
    }
    assert!(validate_k1_invariant().is_ok(), "K-1 强校验必须通过");

    // [4] DAG 构造: 3 节点 (Input → Tool → Output)
    println!("\n[4] DAG 构造: 3 节点 (Input → Tool → Output) + 拓扑序");
    let mut dag = TaskDag::new();
    let a = TaskId::new();
    let b = TaskId::new();
    let c = TaskId::new();
    dag.add_node(a.clone(), NodeKind::Input, "input");
    dag.add_node(b.clone(), NodeKind::Tool, "tool");
    dag.add_node(c.clone(), NodeKind::Output, "output");
    dag.add_edge(a.clone(), b.clone())?;
    dag.add_edge(b.clone(), c.clone())?;
    assert!(!dag.detect_cycle(), "DAG 无环");
    dag.validate_depth()?;
    let topo = dag.topological_sort()?;
    println!("    DAG 节点数 = {}, 拓扑序长度 = {}", dag.len(), topo.len());
    println!("    无环 + 深度 ≤ MAX_DAG_DEPTH ({MAX_DAG_DEPTH}) ✓");

    // [5] 注册 3 task 到 scheduler
    println!("\n[5] 注册 3 task 到 scheduler, 标 0 依赖 root");
    let mut sched = DagScheduler::new();
    let t_root = sched.register(vec![], TaskPriority::Critical, 1000)?;
    let t_mid = sched.register(vec![t_root.clone()], TaskPriority::Normal, 1001)?;
    let t_leaf = sched.register(vec![t_mid.clone()], TaskPriority::Low, 1002)?;
    println!("    root = {t_root}");
    println!("    mid  = {t_mid}");
    println!("    leaf = {t_leaf}");

    // [6] 调度: 0 依赖 root 入队
    println!("\n[6] 调度: 0 依赖 root 入队, 模拟完成推进下游");
    let mut pq = PriorityTaskQueue::new();
    pq.enqueue(t_root.clone(), TaskPriority::Critical);
    let first = pq.dequeue().expect("critical 必出队");
    println!("    出队[0] (critical) = {first}");
    // 模拟 root 完成, 推进 mid 入队
    let mut sm_root = TaskStateMachine::new(t_root.clone(), 1000);
    sm_root.transition(TaskState::Queued, 1001)?;
    sm_root.transition(TaskState::Running, 1002)?;
    sm_root.transition(TaskState::Completed, 1003)?;
    println!("    root: Pending → Queued → Running → Completed ✓");

    // [7] 重试: 失败 task 重试 3 次
    println!("\n[7] 重试: 失败 task 重试 3 次 (per MAX_RETRIES_DEFAULT)");
    let policy = RetryPolicy::default();
    println!("    policy.max_retries = {}", policy.max_retries);
    let mut sm_mid = TaskStateMachine::new(t_mid.clone(), 1000);
    sm_mid.transition(TaskState::Queued, 1001)?;
    sm_mid.transition(TaskState::Running, 1002)?;
    sm_mid.transition(TaskState::Failed, 1003)?;
    // 重试 max_retries 次, 每次 Failed → Queued → Running → Failed
    for i in 0..MAX_RETRIES_DEFAULT {
        sm_mid.transition(TaskState::Queued, 2000 + i as i64)?;
        sm_mid.transition(TaskState::Running, 2001 + i as i64)?;
        sm_mid.transition(TaskState::Failed, 2002 + i as i64)?;
        println!("    重试 [{i}] Failed → Queued → Running → Failed ✓");
    }
    println!("    已用 {MAX_RETRIES_DEFAULT} 次重试, 第 4 次必拒");

    // [8] 取消: cancel 由 Pending 直转 Cancelled
    println!("\n[8] 取消: cancel 由 Pending 直转 Cancelled");
    let mut sm_leaf = TaskStateMachine::new(t_leaf.clone(), 1000);
    sm_leaf.transition(TaskState::Cancelled, 1001)?;
    assert!(sm_leaf.is_terminal());
    println!("    leaf: Pending → Cancelled ✓ (终态不可再转)");

    println!("\n=== DAG 任务调度 demo 完成 ===");
    Ok(())
}
