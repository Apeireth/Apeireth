//! R129-20 Stage 5.3 跨模块证明 — 10 模块 mod 索引
//!
//! # 背景 (per 决策 #33 §2.3 + 决策 #55 §1 + 决策 #61 §3.1 R129-20)
//!
//! P8-2 retry Stage 5.1 形式化 (per `agent-p8-2-retry-...-final-2026-08-10.md`) 在 `crates/apeireth-library-governance/src/formal_proof.rs` 39.3KB.
//! R129-5 Stage 5 治理 (per `agent-r129-5-...-final-2026-08-11.md`) 在 `crates/apeireth-pybridge/src/` 124KB.
//! R129-10 Stage 5.2 形式化扩展 (per stage5_2/) 在 `crates/apeireth-formal/src/stage5_2/` 11 文件 ~75KB.
//! R129-20 Stage 5.3 跨模块证明 (本目录) 在 `crates/apeireth-formal/src/stage5_3/` 11 文件 ~85KB.
//!
//! # 10 模块 (F11-F20) Stage 5.3 跨模块证明
//!
//! 1. `cross_crate_integration_proof` — F11 跨 crate 集成证明 (24 LOCKED + 2 NEW, B1 严守)
//! 2. `cross_borrow_integration_proof` — F12 跨借鉴集成证明 (8 借鉴 ID 8×8=64 边, C2 严守)
//! 3. `cross_stage_integration_proof` — F13 跨 stage 集成证明 (Stage 1-7 ASI Python, 决策 #57-#61 严守)
//! 4. `cross_decision_integration_proof` — F14 跨决策集成证明 (13 关键决策 #22-#66, 决策 #22-#66 严守)
//! 5. `cross_commit_integration_proof` — F15 跨 commit 集成证明 (5 整合 #1-#5, 决策 #48+#62 严守)
//! 6. `cross_locked_integration_proof` — F16 跨 LOCKED 集成证明 (24 LOCKED 入口签名 跨 crate, B1 严守)
//! 7. `cross_anchor_integration_proof` — F17 跨 anchor 集成证明 (8 哲学锚 跨 crate, B5 严守)
//! 8. `cross_gate_integration_proof` — F18 跨 gate 集成证明 (6 重守门 v7 跨 crate, B4 严守)
//! 9. `cross_version_integration_proof` — F19 跨 version 集成证明 (26 crate workspace.version 1.2.0, B2 严守)
//! 10. `cross_push_integration_proof` — F20 跨 push 集成证明 (13 关键决策 全 0 主动 push, 决策 #33 §2.3 + 决策 #61 §6 严守)
//!
//! # 借鉴 ID
//!
//! `R129-20-STAGE5.3-BORROW-kani-4502-Invariant-trait-2026-08-11`
//! - 0 装 PASS 严守: ✅ 0 引 kani 依赖, 0 装"已 Kani 形式化"
//! - 0 越界 8 硬墙: B1/B2/A1/B3/B4/B5/A3/C1/C2/C3 0 越界
//!
//! # 0 触碰 (per 决策 #33 §2.3 + 决策 #61 §6)
//!
//! - B1 24 LOCKED 入口签名 0 改
//! - C1 0 主动 commit: Mavis 整合 #5.1 commit 拍板
//! - 0 主动 push: 等 1.0 release 配 GitHub remote

pub mod cross_crate_integration_proof;
pub mod cross_borrow_integration_proof;
pub mod cross_stage_integration_proof;
pub mod cross_decision_integration_proof;
pub mod cross_commit_integration_proof;
pub mod cross_locked_integration_proof;
pub mod cross_anchor_integration_proof;
pub mod cross_gate_integration_proof;
pub mod cross_version_integration_proof;
pub mod cross_push_integration_proof;

/// Stage 5.3 跨模块证明 10 模块 (F11-F20) 严守
pub const STAGE5_3_MODULE_COUNT: usize = 10;

/// Stage 5.3 跨模块证明 10 模块 ID 索引
pub const STAGE5_3_MODULE_IDS: [&str; STAGE5_3_MODULE_COUNT] = [
    "F11_cross_crate_integration_proof",
    "F12_cross_borrow_integration_proof",
    "F13_cross_stage_integration_proof",
    "F14_cross_decision_integration_proof",
    "F15_cross_commit_integration_proof",
    "F16_cross_locked_integration_proof",
    "F17_cross_anchor_integration_proof",
    "F18_cross_gate_integration_proof",
    "F19_cross_version_integration_proof",
    "F20_cross_push_integration_proof",
];

/// 跑全部 10 模块 runtime sanity (供 `super::run_all` 调用)
pub fn run_all() -> bool {
    cross_crate_integration_proof::sanity_check()
        && cross_borrow_integration_proof::sanity_check()
        && cross_stage_integration_proof::sanity_check()
        && cross_decision_integration_proof::sanity_check()
        && cross_commit_integration_proof::sanity_check()
        && cross_locked_integration_proof::sanity_check()
        && cross_anchor_integration_proof::sanity_check()
        && cross_gate_integration_proof::sanity_check()
        && cross_version_integration_proof::sanity_check()
        && cross_push_integration_proof::sanity_check()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage5_3_module_count_is_10() {
        assert_eq!(STAGE5_3_MODULE_COUNT, 10);
        assert_eq!(STAGE5_3_MODULE_IDS.len(), 10);
    }

    #[test]
    fn stage5_3_all_modules_sanity_check_passes() {
        assert!(run_all(), "Stage 5.3 跨模块证明 10 模块 sanity 应全 pass");
    }
}
