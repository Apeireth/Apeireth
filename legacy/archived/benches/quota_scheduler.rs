//! 认知配额调度基准: 优先级队列派发 + 抢占/PIP 上下文切换周期。
//!
//! 被测对象: `CognitiveQuotaScheduler` (同步 TCB 注册表 + 五级优先级运行队列)。
//! 口径 (每个样本 = batch 个周期的平均):
//! - `dispatch-roundtrip` 派发往返 = 构造 TCB + submit_task + schedule_next +
//!   complete_task (单任务稳态, 含 TCB 构造开销 = 调用方真实成本);
//! - `preempt-pip-cycle` 抢占 + PIP 周期 = 提交 P3 后台任务(持资源记账) + 提交
//!   P1 用户任务 → schedule_next 派发 P1 → preempt_active 重入队 → PIP 提升后台
//!   任务有效优先级 → schedule_next 派发被提升者 → release_lock + 两次 complete_task。

use std::hint::black_box;

use apeireth_orchestration::{
    CognitiveContextFrame, CognitiveInterrupt, CognitivePriority, CognitiveQuota,
    CognitiveQuotaScheduler, CognitiveTaskControlBlock,
};

use crate::support::{measure_per_op, BenchConfig, Outcome, Target};

/// 构造一个最小 TCB (字段与调度器测试同形)。
fn tcb(task_id: &str, priority: CognitivePriority) -> CognitiveTaskControlBlock {
    CognitiveTaskControlBlock {
        task_id: task_id.to_string(),
        session_id: "bench-session".to_string(),
        base_priority: priority,
        effective_priority: priority,
        quota: CognitiveQuota::new(1000, 10, 1000),
        context_frame: CognitiveContextFrame {
            task_id: task_id.to_string(),
            session_id: "bench-session".to_string(),
            step_index: 0,
            call_stack: Vec::new(),
            local_transcript_snapshot: Vec::new(),
            world_state_hash: "bench-hash".to_string(),
        },
        is_preempted: false,
    }
}

/// 运行认知配额调度基准, 返回 [派发往返, 抢占+PIP 周期] 两个结果项。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    // ---- 派发往返: submit + schedule_next + complete ----
    let dispatch_samples = {
        let scheduler = CognitiveQuotaScheduler::new();
        measure_per_op(cfg.samples(3), cfg.samples(80), 256, || {
            scheduler
                .submit_task(tcb("bench-task", CognitivePriority::InteractiveUser))
                .expect("提交任务");
            let dispatched = scheduler.schedule_next().expect("派发任务");
            black_box(dispatched.task_id == "bench-task");
            scheduler.complete_task("bench-task");
        })
    };

    // ---- 抢占 + PIP 上下文切换周期 ----
    let preempt_samples = {
        let scheduler = CognitiveQuotaScheduler::new();
        measure_per_op(cfg.samples(3), cfg.samples(80), 256, || {
            scheduler
                .submit_task(tcb("bg-task", CognitivePriority::BackgroundDreaming))
                .expect("提交后台任务");
            scheduler
                .submit_task(tcb("user-task", CognitivePriority::InteractiveUser))
                .expect("提交用户任务");
            scheduler.register_lock("memory-graph-lock", "bg-task");

            let first = scheduler.schedule_next().expect("派发用户任务");
            let preempted = scheduler
                .preempt_active(CognitiveInterrupt::EmergencyPreempt {
                    reason: "bench".to_string(),
                })
                .expect("抢占");
            scheduler
                .boost_priority_for_lock("memory-graph-lock", CognitivePriority::InteractiveUser);
            let pip = scheduler.schedule_next().expect("派发 PIP 提升者");
            scheduler.release_lock("memory-graph-lock");
            scheduler.complete_task("user-task");
            scheduler.complete_task("bg-task");
            black_box((first.task_id, preempted, pip.task_id));
        })
    };

    vec![
        Outcome::from_ns(
            "quota-dispatch",
            "认知配额调度-优先级队列派发",
            "构造 TCB + submit_task + schedule_next + complete_task 单任务稳态往返",
            "us",
            256,
            cfg.samples(3),
            dispatch_samples,
            Some(Target {
                label: "< 50.0 us".to_string(),
                value: 50.0,
            }),
            "每样本 = 256 次往返的平均; 含 TCB 构造 (字符串分配) 的调用方真实成本; 单线程, 调度器实例常驻复用",
        ),
        Outcome::from_ns(
            "quota-preempt-pip",
            "认知配额调度-抢占+PIP 上下文切换",
            "提交双优先级任务 + schedule_next + preempt_active 重入队 + PIP 提升 + 再派发 + 释放/回收 全周期",
            "us",
            256,
            cfg.samples(3),
            preempt_samples,
            Some(Target {
                label: "< 50.0 us".to_string(),
                value: 50.0,
            }),
            "每样本 = 256 个全周期的平均; 与 README「优先级队列派发 + PIP context switch」行同口径; 单线程进程内调度",
        ),
    ]
}
