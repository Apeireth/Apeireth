//! R129-10 Stage 5.2 形式化扩展 — 10 模块 mod 索引
//!
//! # 背景 (per 决策 #33 §2.3 + 决策 #55 §1 + 决策 #61 §3.1 R129-10)
//!
//! P8-2 retry Stage 5.1 形式化 (per `agent-p8-2-retry-...-final-2026-08-10.md`) 在 `crates/apeireth-library-governance/src/formal_proof.rs` 39.3KB.
//! R129-5 Stage 5 治理 (per `agent-r129-5-...-final-2026-08-11.md`) 在 `crates/apeireth-pybridge/src/` 124KB.
//! R129-10 Stage 5.2 形式化扩展 (本目录) 在 `crates/apeireth-formal/src/stage5_2/` 10 模块 ~70KB.
//!
//! # 10 模块 (F1-F10) Stage 5.2 形式化扩展
//!
//! 1. `six_gates_v7_formal` — F1 6 重守门 v7 形式化 (B4 严守)
//! 2. `eight_anchors_formal` — F2 8 哲学锚形式化 (B5 严守)
//! 3. `v05_30dim_formal` — F3 V0.5 30 维形式化 (B3 严守)
//! 4. `verdict_cache_13keys_formal` — F4 13 键 verdict cache 形式化 (A3 严守)
//! 5. `r11_baseline_formal` — F5 R11 baseline 3 值形式化 (A1 严守)
//! 6. `locked_24_entry_formal` — F6 24 LOCKED 入口签名形式化 (B1 严守)
//! 7. `borrow_8_id_formal` — F7 8 借鉴 ID 真实施形式化 (C2 严守)
//! 8. `integration_4_commit_formal` — F8 整合 #4 commit 严守形式化 (C1 严守)
//! 9. `cross_module_proof` — F9 跨模块证明 (F1-F8 跨模块集成)
//! 10. `integration_proof` — F10 集成证明 (F1-F9 完整集成)
//!
//! # 借鉴 ID
//!
//! `R129-10-STAGE5.2-BORROW-kani-4502-Invariant-trait-2026-08-11`
//! - 0 装 PASS 严守: ✅ 0 引 kani 依赖, 0 装"已 Kani 形式化"
//! - 0 越界 8 硬墙: B1/B2/A1/B3/B4/B5/A3/C1/C2/C3 0 越界
//!
//! # 0 触碰 (per 决策 #33 §2.3 + 决策 #61 §6)
//!
//! - B1 24 LOCKED 入口签名 0 改
//! - C1 0 主动 commit: Mavis 整合 #5.1 commit 拍板
//! - 0 主动 push: 等 1.0 release 配 GitHub remote

pub mod six_gates_v7_formal;
pub mod eight_anchors_formal;
pub mod v05_30dim_formal;
pub mod verdict_cache_13keys_formal;
pub mod r11_baseline_formal;
pub mod locked_24_entry_formal;
pub mod borrow_8_id_formal;
pub mod integration_4_commit_formal;
pub mod cross_module_proof;
pub mod integration_proof;

/// Stage 5.2 形式化扩展 10 模块 (F1-F10) 严守
pub const STAGE5_2_MODULE_COUNT: usize = 10;

/// Stage 5.2 形式化扩展 10 模块 ID 索引
pub const STAGE5_2_MODULE_IDS: [&str; STAGE5_2_MODULE_COUNT] = [
    "F1_six_gates_v7_formal",
    "F2_eight_anchors_formal",
    "F3_v05_30dim_formal",
    "F4_verdict_cache_13keys_formal",
    "F5_r11_baseline_formal",
    "F6_locked_24_entry_formal",
    "F7_borrow_8_id_formal",
    "F8_integration_4_commit_formal",
    "F9_cross_module_proof",
    "F10_integration_proof",
];

/// 跑全部 10 模块 runtime sanity (供 `super::run_all` 调用)
pub fn run_all() -> bool {
    six_gates_v7_formal::sanity_check()
        && eight_anchors_formal::sanity_check()
        && v05_30dim_formal::sanity_check()
        && verdict_cache_13keys_formal::sanity_check()
        && r11_baseline_formal::sanity_check()
        && locked_24_entry_formal::sanity_check()
        && borrow_8_id_formal::sanity_check()
        && integration_4_commit_formal::sanity_check()
        && cross_module_proof::sanity_check()
        && integration_proof::sanity_check()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage5_2_module_count_is_10() {
        assert_eq!(STAGE5_2_MODULE_COUNT, 10);
        assert_eq!(STAGE5_2_MODULE_IDS.len(), 10);
    }

    #[test]
    fn stage5_2_all_modules_sanity_check_passes() {
        assert!(run_all(), "Stage 5.2 形式化扩展 10 模块 sanity 应全 pass");
    }
}
