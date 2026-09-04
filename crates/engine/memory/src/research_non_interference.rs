//! B8 · Phase 6 原型二: 模块非干扰性 (Research 前缀, 不进默认路径)。
//!
//! # 学术账本 (铁律 3)
//! - **问题定义**: 记忆模块组合 (diary/wiki/chronicle/…) 时, 任一模块的操作
//!   不得改变其他模块的可观测结果; 并发交错必须等价于某种顺序执行
//!   (交换性/非干扰信息流)。
//! - **假设**: 对"独立状态空间"的模块, 其操作满足强非干扰 —
//!   模块 A 的结果流与模块 B 的调度无关; 确定性交错 harness 可穷举验证。
//! - **状态**: 原型已实现 (抽象 trait + 确定性交错枚举, 无真实线程 → 无 flaky)。
//! - **引用**: Goguen & Meseguer 1982 (non-interference); 项目纪律锚定
//!   "实验与产品代码分离" (铁律 4) 与模块边界审查。
//! - **baseline**: `research/baselines/baseline-2026-09-phase0.md`。
//! - **已知局限**: ① 本原型验证的是**抽象状态机**的非干扰, 不是真实
//!   `SqliteMemoryStore` 的跨表隔离 (真实 SQLite 单写者语义另测); ② 共享
//!   子资源 (同一 FTS 索引/同一文件) 的模块不在"独立空间"假设内, 需显式
//!   声明共享并单独验证; ③ 不涉及信息流安全 (涉密分级), 那是另一个方向。

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

/// 抽象模块: 状态 S + 操作 Op; 操作必须确定性 (同状态同 op 同结果)。
pub trait ResearchModule {
    type State: Clone + PartialEq + Debug;
    type Op: Clone + PartialEq + Debug;

    fn init() -> Self::State;
    /// 应用操作, 返回可观测结果 (用于结果流对比)。
    fn apply(state: &mut Self::State, op: &Self::Op) -> String;
}

/// 非干扰验证结果。
#[derive(Debug, Clone, PartialEq)]
pub struct ResearchNonInterferenceReport {
    pub interleavings_checked: u64,
    pub violations: u64,
    /// 违规样例 (模块名, 交错, 期望, 实际)。
    pub details: Vec<String>,
}

/// 对两个独立模块 A/B: 枚举同一 op 多重集的所有交错 (确定性, 单线程),
/// 验证:
/// 1. **强非干扰**: A 的最终状态/结果流与 B 的调度无关;
/// 2. **交换性**: 任意交错与顺序执行 (先 A 后 B / 先 B 后 A) 结果一致。
pub fn research_check_non_interference<A, B>(
    ops_a: Vec<A::Op>,
    ops_b: Vec<B::Op>,
) -> ResearchNonInterferenceReport
where
    A: ResearchModule,
    B: ResearchModule,
    A::State: Default,
    B::State: Default,
{
    let mut report = ResearchNonInterferenceReport {
        interleavings_checked: 0,
        violations: 0,
        details: Vec::new(),
    };
    // 顺序参照: A 全量后 B 全量。
    let mut ref_a = A::init();
    let mut ref_b = B::init();
    let ref_results_a: Vec<String> = ops_a.iter().map(|op| A::apply(&mut ref_a, op)).collect();
    let ref_results_b: Vec<String> = ops_b.iter().map(|op| B::apply(&mut ref_b, op)).collect();

    // 交错枚举: 所有把 A 的 n 个 op 与 B 的 m 个 op 交错排列 (位掩码: 0=A, 1=B)。
    let total = ops_a.len() + ops_b.len();
    let mut interleavings: Vec<Vec<bool>> = Vec::new();
    let mut stack: Vec<(Vec<bool>, usize, usize)> = vec![(Vec::new(), 0, 0)];
    while let Some((prefix, ia, ib)) = stack.pop() {
        if prefix.len() == total {
            interleavings.push(prefix);
            continue;
        }
        if ia < ops_a.len() {
            let mut p = prefix.clone();
            p.push(false);
            stack.push((p, ia + 1, ib));
        }
        if ib < ops_b.len() {
            let mut p = prefix.clone();
            p.push(true);
            stack.push((p, ia, ib + 1));
        }
    }

    for order in interleavings {
        report.interleavings_checked += 1;
        let mut sa = A::init();
        let mut sb = B::init();
        let mut ia = 0usize;
        let mut ib = 0usize;
        let mut results_a: Vec<String> = Vec::new();
        let mut results_b: Vec<String> = Vec::new();
        for is_b in &order {
            if *is_b {
                results_b.push(B::apply(&mut sb, &ops_b[ib]));
                ib += 1;
            } else {
                results_a.push(A::apply(&mut sa, &ops_a[ia]));
                ia += 1;
            }
        }
        if sa != ref_a || results_a != ref_results_a {
            report.violations += 1;
            report.details.push(format!(
                "module A interference: order={order:?} state={sa:?} != ref={ref_a:?}"
            ));
        }
        if sb != ref_b || results_b != ref_results_b {
            report.violations += 1;
            report.details.push(format!(
                "module B interference: order={order:?} state={sb:?} != ref={ref_b:?}"
            ));
        }
    }
    report
}

/// 计数模块 (原型样例): 单调递增计数。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResearchCounterModule;
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResearchCounterOp {
    Add(u64),
    Sub(u64),
}
impl ResearchModule for ResearchCounterModule {
    type State = u64;
    type Op = ResearchCounterOp;
    fn init() -> Self::State {
        0
    }
    fn apply(state: &mut Self::State, op: &Self::Op) -> String {
        match op {
            Self::Op::Add(n) => {
                *state = state.saturating_add(*n);
                format!("+{n}")
            }
            Self::Op::Sub(n) => {
                *state = state.saturating_sub(*n);
                format!("-{n}")
            }
        }
    }
}

/// 集合模块 (原型样例): 幂等标签集合。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResearchSetModule;
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResearchSetOp {
    Insert(String),
    Remove(String),
}
impl ResearchModule for ResearchSetModule {
    type State = BTreeMap<String, ()>;
    type Op = ResearchSetOp;
    fn init() -> Self::State {
        BTreeMap::new()
    }
    fn apply(state: &mut Self::State, op: &Self::Op) -> String {
        match op {
            Self::Op::Insert(k) => {
                state.insert(k.clone(), ());
                format!("+{k}")
            }
            Self::Op::Remove(k) => {
                state.remove(k);
                format!("-{k}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 计数器 × 集合: 全交错零干扰 (强非干扰 + 交换性)。
    #[test]
    fn counter_and_set_fully_non_interfering() {
        use ResearchCounterOp::{Add, Sub};
        use ResearchSetOp::{Insert, Remove};
        let ops_a = vec![Add(3), Sub(1), Add(10)];
        let ops_b = vec![Insert("x".into()), Insert("y".into()), Remove("x".into())];
        let report = research_check_non_interference::<ResearchCounterModule, ResearchSetModule>(
            ops_a, ops_b,
        );
        assert_eq!(report.violations, 0, "详情: {:?}", report.details);
        // C(4,3) = 20 种交错全部枚举
        assert_eq!(report.interleavings_checked, 20);
    }

    /// 同模块不同实例 (两个计数器) 同样互不干扰。
    #[test]
    fn two_instances_of_same_module_non_interfering() {
        use ResearchCounterOp::{Add, Sub};
        let report = research_check_non_interference::<ResearchCounterModule, ResearchCounterModule>(
            vec![Add(1), Add(2)],
            vec![Sub(5), Add(7)],
        );
        assert_eq!(report.violations, 0, "详情: {:?}", report.details);
    }

    /// 更重的组合: 3 个 A op × 3 个 B op (20 交错) + 4×2 (15 交错) 都零违例。
    #[test]
    fn heavier_op_sets_zero_violations() {
        use ResearchCounterOp::Add;
        use ResearchSetOp::Insert;
        for (na, nb) in [(3usize, 3usize), (4, 2), (2, 4), (5, 1)] {
            let ops_a: Vec<_> = (0..na).map(|i| Add(i as u64 + 1)).collect();
            let ops_b: Vec<_> = (0..nb)
                .map(|i| Insert(format!("tag{i}")))
                .collect();
            let report = research_check_non_interference::<ResearchCounterModule, ResearchSetModule>(
                ops_a, ops_b,
            );
            assert_eq!(report.violations, 0, "({na},{nb}): {:?}", report.details);
        }
    }
}
