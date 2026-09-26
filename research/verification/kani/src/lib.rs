//! Kani 验证 mirror crate (零复制)。
//!
//! 通过 `#[path]` 直接包含 canonical 源码 —— 与生产同源同文件,
//! 不存在拷贝漂移问题:
//!   - crates/engine/runtime/src/canonical/research_approval_sm.rs
//!   - crates/engine/runtime-assembly/src/canonical/causal_world_model.rs
//!   - crates/foundation/orchestration/src/context_fold/fold_block.rs
//!   - crates/foundation/orchestration/src/async_context.rs
//!   - crates/foundation/orchestration/src/cognitive_quota_scheduler.rs
//!   - crates/engine/memory/src/{residual_pyramid,semantic_axis,river_topology,bitemporal_graph}.rs
//!   - crates/adapters/gateway/src/file_fetcher.rs
//!   - crates/capabilities/tools/src/sensitive_path.rs
//!   - crates/foundation/governance/src/{risk,intent}.rs
//!
//! 为什么存在: workspace 声明 rustc 1.97, Kani 0.67 (crates.io 最新) 自带
//! nightly 1.93, `cargo kani -p apeireth-runtime` 被 cargo rust-version 检查拒绝。
//! 本 crate 独立于 workspace (root Cargo.toml 已 exclude research/),
//! 只编译经 #[path] 引入的 canonical 文件 + 少量依赖, 1.93 可编译。
//!
//! 验证目标 = 两处 `#[cfg(kani)]` 门控 harness:
//!   1. research_approval_sm.rs 内 `mod kani_proofs` (3 个 #[kani::proof], 既有);
//!   2. 本 crate `src/harness_*.rs` 六个性质族 harness (assertion 即命题):
//!      - harness_panic_freedom.rs       性质族 1  panic-freedom (零未捕获异常)
//!      - harness_memory_conservation.rs 性质族 2  记忆守恒 (protect/forget 语义纯模型)
//!      - harness_governance.rs          性质族 3  治理单调性 (默认拒绝 / fail-closed)
//!      - harness_quota_scheduler.rs     性质族 4  配额非负与调度安全 (含 PIP)
//!      - harness_path_sandbox.rs        性质族 5  路径沙箱不逃逸
//!      - harness_saga_rollback.rs       性质族 6  SAGA/CoW 回滚精确性
//!   另: file_fetcher.rs 内 `mod kani_base64_proofs` 覆盖模块私有 base64_decode。
//!
//! harness 输入全部由 `kani::any()` 生成并显式有界; 前置形状约束写作早退守卫
//! (与 `kani::assume` 语义等价: 命题为 cond ⇒ P), 每个 harness 的注释写明
//! "证明什么、边界是什么"。生产构建 cfg(kani) 关闭, 验证代码零参与。
//!
//! 运行 (CI: .github/workflows/kani.yml):
//!   cargo kani --manifest-path research/verification/kani/Cargo.toml --harness <name>
//! 本地 Windows 无 Kani; 本地可跑的类型检查/冒烟桩在 ../kani-typecheck/。

// 非 kani 构建下 harness 被 cfg 掉, canonical 条目呈 dead_code —— 静音;
// unused_imports 同 workspace lints 口径 (canonical 文件按整 crate 编译时
// 部分导入仅测试路径使用)。
#![allow(dead_code, unused_imports)]

#[path = "../../../../crates/engine/runtime/src/canonical/research_approval_sm.rs"]
pub mod research_approval_sm;

#[path = "../../../../crates/engine/runtime-assembly/src/canonical/causal_world_model.rs"]
pub mod causal_world_model;

#[path = "../../../../crates/foundation/orchestration/src/context_fold/fold_block.rs"]
pub mod fold_block;

#[path = "../../../../crates/foundation/orchestration/src/async_context.rs"]
pub mod async_context;

#[path = "../../../../crates/foundation/orchestration/src/cognitive_quota_scheduler.rs"]
pub mod cognitive_quota_scheduler;

#[path = "../../../../crates/engine/memory/src/residual_pyramid.rs"]
pub mod residual_pyramid;

#[path = "../../../../crates/engine/memory/src/semantic_axis.rs"]
pub mod semantic_axis;

#[path = "../../../../crates/engine/memory/src/river_topology.rs"]
pub mod river_topology;

#[path = "../../../../crates/engine/memory/src/bitemporal_graph.rs"]
pub mod bitemporal_graph;

#[path = "../../../../crates/adapters/gateway/src/file_fetcher.rs"]
pub mod file_fetcher;

#[path = "../../../../crates/capabilities/tools/src/sensitive_path.rs"]
pub mod sensitive_path;

#[path = "../../../../crates/foundation/governance/src/risk.rs"]
pub mod risk;

#[path = "../../../../crates/foundation/governance/src/intent.rs"]
pub mod intent;

// ===== 性质族 harness (cfg(kani) 门控, 生产零参与) =====

#[cfg(kani)]
#[path = "harness_panic_freedom.rs"]
mod harness_panic_freedom;

#[cfg(kani)]
#[path = "harness_memory_conservation.rs"]
mod harness_memory_conservation;

#[cfg(kani)]
#[path = "harness_governance.rs"]
mod harness_governance;

#[cfg(kani)]
#[path = "harness_quota_scheduler.rs"]
mod harness_quota_scheduler;

#[cfg(kani)]
#[path = "harness_path_sandbox.rs"]
mod harness_path_sandbox;

#[cfg(kani)]
#[path = "harness_saga_rollback.rs"]
mod harness_saga_rollback;
