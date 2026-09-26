//! SAGA 补偿回退基准: 逆序 LIFO 补偿栈执行。
//!
//! 口径 (每个操作):
//! 1. `fork_branch` fork 一个假设分支;
//! 2. `push_saga_compensation` × 100 压入补偿动作;
//! 3. `rollback_branch` 回退: 逆序 (LIFO) 弹出全部补偿并返回执行序;
//! 4. 校验返回序恰为压入序的逆序 (顺序正确性)。
//!
//! 被测对象为进程内纯内存实现 (补偿动作本体的外部执行不在本基准内,
//! 本基准测的是补偿栈的编排/回退开销)。

use std::hint::black_box;

use apeireth_runtime_assembly::canonical::causal_world_model::{
    CausalWorldModel, SagaCompensatingAction,
};

use crate::support::{measure_per_op, BenchConfig, Outcome, Target};

/// 每操作补偿动作数。
const COMPENSATIONS: usize = 100;
/// 每样本操作数。
const BATCH: usize = 10;

/// 运行 SAGA 补偿回退基准。
pub fn run(cfg: &BenchConfig) -> Vec<Outcome> {
    let mut model = CausalWorldModel::new("snap-root");
    let counter = std::cell::Cell::new(0u64);

    let samples_ns = measure_per_op(cfg.samples(3), cfg.samples(40), BATCH, || {
        let n = counter.get();
        counter.set(n + 1);
        let branch = format!("saga-{n}");
        model.fork_branch(&branch).expect("fork 分支");
        for i in 0..COMPENSATIONS {
            model
                .push_saga_compensation(
                    &branch,
                    SagaCompensatingAction {
                        action_id: format!("a{i:03}"),
                        forward_action_name: format!("forward-{i:03}"),
                        compensation_action_name: format!("compensate-{i:03}"),
                        payload: std::collections::HashMap::new(),
                    },
                )
                .expect("压入补偿");
        }
        let executed = model.rollback_branch(&branch).expect("回退分支");
        assert_eq!(executed.len(), COMPENSATIONS, "补偿全部逆序弹出");
        assert_eq!(
            executed.first().map(|a| a.action_id.as_str()),
            Some("a099"),
            "LIFO: 最后压入者最先补偿"
        );
        black_box(executed);
    });

    vec![Outcome::from_ns(
        "saga-rollback",
        "SAGA 补偿回退 (逆序 LIFO 补偿执行)",
        "fork_branch + 100 次 push_saga_compensation + rollback_branch 逆序弹出 单次",
        "us",
        BATCH,
        cfg.samples(3),
        samples_ns,
        Some(Target {
            label: "< 1.0 ms".to_string(),
            value: 1000.0,
        }),
        "纯内存补偿栈编排 (不含补偿动作的外部副作用执行); 每样本 = 10 个操作的平均; 回退分支按被测语义保留为已剪枝",
    )]
}
