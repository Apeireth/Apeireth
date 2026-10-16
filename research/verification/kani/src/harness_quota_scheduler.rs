//! 性质族 4 · 配额与调度安全 harness (cognitive_quota_scheduler)。
//!
//! 对应两条宣称:
//!   - "配额扣减后任何维度不得为负": 消费记账为无符号 saturating 累加,
//!     非负性由构造保证; 非平凡面是 扣减只增不减 (不回绕) + 耗尽状态粘滞
//!     + consume_step 返回值与 is_exhausted 一致。
//!   - "高优先级 (含 PIP 继承) 不被低优先级无限阻塞 (有界情形)": 出队必选
//!     最高紧急度层 (枚举序最小), 平级 FIFO; PIP 提升后持锁者先于请求者出队。
//!
//! 每个 harness 的注释一句话写明"证明什么、边界是什么"。

use super::cognitive_quota_scheduler::{
    CognitiveContextFrame, CognitivePriority, CognitiveQuota, CognitiveQuotaScheduler,
    CognitiveTaskControlBlock,
};

fn prio(choice: u8) -> CognitivePriority {
    match choice % 5 {
        0 => CognitivePriority::SystemEmergency,
        1 => CognitivePriority::InteractiveUser,
        2 => CognitivePriority::ActiveSubAgent,
        3 => CognitivePriority::BackgroundDreaming,
        _ => CognitivePriority::IdleMaintenance,
    }
}

fn tcb(id: &str, priority: CognitivePriority) -> CognitiveTaskControlBlock {
    CognitiveTaskControlBlock {
        task_id: id.to_string(),
        session_id: "s".to_string(),
        base_priority: priority,
        effective_priority: priority,
        quota: CognitiveQuota::new(1, 1, 1),
        context_frame: CognitiveContextFrame {
            task_id: id.to_string(),
            session_id: "s".to_string(),
            step_index: 0,
            call_stack: Vec::new(),
            local_transcript_snapshot: Vec::new(),
            world_state_hash: "h".to_string(),
        },
        is_preempted: false,
    }
}

/// 证明: 配额消费记账单调不减 (任意 tokens/cost 扣减后各维度不为负、不回绕),
/// 耗尽状态一旦成立不再复活, consume_step 返回值 == !is_exhausted()。
/// 边界: 记账维度为 usize/u64 无符号 saturating 累加 (非负性由构造保证);
/// 扣减次数 2 (有界), 各数值为任意 u64/usize; unwind 32。
#[kani::proof]
#[kani::unwind(32)]
fn kani_quota_ledger_monotone_non_negative() {
    let mut quota = CognitiveQuota::new(
        kani::any::<usize>(),
        kani::any::<usize>(),
        kani::any::<u64>(),
    );
    for _ in 0..2 {
        let (prev_tokens, prev_steps, prev_cost) = (
            quota.consumed_tokens,
            quota.consumed_tool_steps,
            quota.consumed_cost_micros,
        );
        let was_exhausted = quota.is_exhausted();

        let not_exhausted = quota.consume_step(kani::any::<usize>(), kani::any::<u64>());

        assert!(
            quota.consumed_tokens >= prev_tokens,
            "tokens 维度扣减后不为负、不回绕 (只增不减)"
        );
        assert!(
            quota.consumed_tool_steps >= prev_steps,
            "steps 维度扣减后不为负、不回绕 (只增不减)"
        );
        assert!(
            quota.consumed_cost_micros >= prev_cost,
            "cost 维度扣减后不为负、不回绕 (只增不减)"
        );
        assert_eq!(
            not_exhausted,
            !quota.is_exhausted(),
            "consume_step 返回值与耗尽判定一致"
        );
        if was_exhausted {
            assert!(quota.is_exhausted(), "耗尽状态单调 (不复活)");
        }
    }
}

/// 证明: 调度不饿死高优先级 —— 任意 3 个任务 (优先级任意 5 层, 入队顺序
/// 固定) 下, 首次出队必是最高紧急度层中最早提交者 (紧急度优先 + 平级 FIFO)。
/// 边界: 3 个任务 (有界情形; 无限任务流的公平性是活性质, 不在安全不变量域),
/// 具体短 id ("a"/"b"/"c", 避免符号键入 SipHash), unwind 48。
#[kani::proof]
#[kani::unwind(48)]
fn kani_quota_highest_priority_dispatched_first() {
    let scheduler = CognitiveQuotaScheduler::new();
    let ids = ["a", "b", "c"];
    let priorities = [
        prio(kani::any::<u8>()),
        prio(kani::any::<u8>()),
        prio(kani::any::<u8>()),
    ];
    for i in 0..3 {
        scheduler.submit_task(tcb(ids[i], priorities[i])).unwrap();
    }

    // 期望: 枚举序最小 (最紧急) 的层里最早提交的任务。
    let (mut expect_prio, mut expect_id) = (priorities[0], ids[0]);
    for i in 1..3 {
        if priorities[i] < expect_prio {
            expect_prio = priorities[i];
            expect_id = ids[i];
        }
    }

    let first = scheduler.schedule_next().expect("队列非空必有出队");
    assert_eq!(first.effective_priority, expect_prio, "最紧急层先出队");
    assert_eq!(first.task_id, expect_id, "平级 FIFO: 最早提交者先出队");
}

/// 证明: PIP 防优先级反转 —— 后台持锁者经 boost_priority_for_lock 继承
/// 请求者优先级后, 必先于请求者本人出队 (有界情形: 2 任务)。
/// 边界: 请求者优先级任意取 P0..P2 (必严于持锁者 P3, 提升条件恒满足),
/// 提升口径 = 请求者优先级 (真实 PIP 语义), 具体短 id/资源名, unwind 48。
#[kani::proof]
#[kani::unwind(48)]
fn kani_quota_pip_boost_prevents_inversion() {
    let scheduler = CognitiveQuotaScheduler::new();
    scheduler
        .submit_task(tcb("h", CognitivePriority::BackgroundDreaming))
        .unwrap();
    let requester_prio = prio(kani::any::<u8>() % 3); // P0..P2
    scheduler.submit_task(tcb("r", requester_prio)).unwrap();

    scheduler.register_lock("res", "h");
    scheduler.boost_priority_for_lock("res", requester_prio);

    let first = scheduler.schedule_next().expect("必有出队");
    assert_eq!(first.task_id, "h", "PIP: 持锁者先于请求者出队 (防反转)");
    assert_eq!(
        first.effective_priority, requester_prio,
        "PIP: 持锁者继承请求者优先级"
    );

    let second = scheduler.schedule_next().expect("必有出队");
    assert_eq!(second.task_id, "r", "请求者随后出队");
}
