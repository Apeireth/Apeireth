//! Kani 验证 mirror crate (零复制)。
//!
//! 通过 `#[path]` 直接包含 canonical 源码 —— 与生产同源同文件,
//! 不存在拷贝漂移问题:
//!   crates/engine/runtime/src/canonical/research_approval_sm.rs
//!
//! 为什么存在: workspace 声明 rustc 1.97, Kani 0.67 (crates.io 最新) 自带
//! nightly 1.93, `cargo kani -p apeireth-runtime` 被 cargo rust-version 检查拒绝。
//! 本 crate 独立于 workspace (root Cargo.toml 已 exclude research/),
//! 只编译状态机一个文件 + serde, 1.93 可编译。
//!
//! 运行 (CI: .github/workflows/kani.yml):
//!   cargo kani --manifest-path research/verification/kani/Cargo.toml
//! 验证目标 = 文件内 `#[cfg(kani)] mod kani_proofs` 的 3 个 `#[kani::proof]`:
//!   kani_terminal_lock_no_outgoing_transitions
//!   kani_executed_monotonic_once
//!   kani_crash_recovery_invariant_c

#[path = "../../../../crates/engine/runtime/src/canonical/research_approval_sm.rs"]
pub mod research_approval_sm;
