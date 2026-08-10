//! 不变量模块: 每个不变量一个文件, 每个文件暴露 1 个 Kani harness + 1 个 sanity test.
//!
//! Ponytail: 第 1 个不变量 (double_onion_sample) 已上, 后续不变量按相同模板追加。

pub mod double_onion_sample;
pub mod e_layer_isolation;
pub mod permission_grant_l0;
pub mod mid_task_atomicity;
pub mod seven_advisor_voting;

/// 运行所有不变量的 runtime sanity check (供 `run_all` 调用).
///
/// Kani 形式化证明与 runtime sanity 是两条独立的验证通道:
/// - Kani 符号执行覆盖**所有**输入 (完备)
/// - runtime sanity 仅覆盖少量具体输入 (快速 smoke test)
///
/// runtime 全部通过 ≠ 形式化成立; 但 runtime 失败 = 一定有问题。
pub fn run_all() -> bool {
    double_onion_sample::sanity_check() && e_layer_isolation::sanity_check() && permission_grant_l0::sanity_check() && mid_task_atomicity::sanity_check() && seven_advisor_voting::sanity_check()
}

