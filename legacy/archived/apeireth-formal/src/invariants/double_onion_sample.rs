//! 不变量: 权限洋葱 L0 必须要求 HA (双洋葱护栏).
//!
//! 这是 docs/v2-strategy/03 §4A "形式化验证" 的第 1 个 sample 不变量.
//! 物理含义: L0 是 HA 核心, 失去 HA = 失去最后一道门, 架构层不允许.
//!
//! # 验证通道
//! - **Kani 形式化**: `cargo kani --harness double_onion_sample`
//!   对任意 `u8 × bool` 符号执行, 完备覆盖 (无界模型检查).
//! - **runtime sanity**: `cargo test -p apeireth-formal`
//!   仅跑 11 个具体 case (1 个 L0+true happy path + 5 层 × 2 requires_ha flag), 快速 smoke test.
//!
//! ponytail: 不变量断言体 1 行, harness 总 LOC < 30. 后续不变量照此模板追加.

use crate::{l0_requires_ha_invariant, PermissionLayerConfig};

/// Kani proof harness — 命名必须与 CI `--harness double_onion_sample` 对齐.
///
/// `#[cfg_attr(kani, kani::proof)]` 让 stable Rust 也能编译本文件
/// (cargo-kani 离线时, proof 属性被剥离, 函数退化为普通 fn).
#[cfg_attr(kani, kani::proof)]
pub fn double_onion_sample() {
    let cfg = nondet_config();
    assert!(l0_requires_ha_invariant(cfg));
}

/// 非确定性输入生成 — Kani 模式下返回 `kani::any()`, 其它模式返回具体值.
///
/// 把 cfg 隔离开, 让 harness 函数体始终 1 行 (YAGNI).
#[cfg(kani)]
fn nondet_config() -> PermissionLayerConfig {
    kani::any()
}

#[cfg(not(kani))]
fn nondet_config() -> PermissionLayerConfig {
    // cargo test 兜底: 选一个 L0+true 的 happy path, 不会触发 assert!
    PermissionLayerConfig::new(0, true)
}

/// Runtime sanity: 对满足不变量前提的输入跑具体 case, 应全部通过.
///
/// 注意: 故意 **排除** `kind=0 && requires_ha=false` (那是不变量应该抓出的负例,
/// 见 `negative_l0_without_ha_must_violate` 测试).
pub fn sanity_check() -> bool {
    // case 1: L0 + true (不变量前提: L0 永远要求 HA)
    if !l0_requires_ha_invariant(PermissionLayerConfig::new(0, true)) {
        return false;
    }
    // case 2..7: 1..=5 层无论 flag 均应满足不变量
    for kind in 1u8..=5 {
        for ha in [true, false] {
            if !l0_requires_ha_invariant(PermissionLayerConfig::new(kind, ha)) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_function_is_publicly_visible() {
        // 文档化保证: harness 函数名 = `double_onion_sample`
        // CI 用 `cargo kani --harness double_onion_sample` 跑, 改名会立刻断 CI.
        let _: fn() = double_onion_sample;
    }

    #[test]
    fn sanity_check_passes_all_precondition_inputs() {
        assert!(
            sanity_check(),
            "L0 requires_ha invariant violated on valid input"
        );
    }

    #[test]
    fn negative_l0_without_ha_must_violate() {
        // 反例: L0 + requires_ha=false 必须被不变量抓出
        let bad = PermissionLayerConfig::new(0, false);
        assert!(
            !l0_requires_ha_invariant(bad),
            "L0 without HA must violate invariant"
        );
    }

    #[test]
    fn positive_all_non_l0_layers_hold_regardless_of_ha() {
        // 1..=5 任何 flag 都应通过 (不变量是"非 L0 总是安全")
        for kind in 1u8..=5 {
            assert!(l0_requires_ha_invariant(PermissionLayerConfig::new(
                kind, false
            )));
            assert!(l0_requires_ha_invariant(PermissionLayerConfig::new(
                kind, true
            )));
        }
    }
}
