//! Production composition for [`apeireth_runtime`].
//!
//! Desktop composition is the default feature. The portable host builds without
//! desktop capabilities; memory and durable profiles can be selected separately.

#![deny(unsafe_code)]

pub mod mesh;
pub mod portable;
// 问责闭环第 ③ 步 · 自动回填面: 工具失败 ⇒ 对应已声明影响回填 `failed`
// （归属由调用方**显式登记**；`confirmed` 仍只走人工/外部）。
//
// 2026-10-31 合并注记: main 把本 crate 的桌面组装面整体收进 `desktop` 模块;
// 本模块 main 侧**不存在**，且 `apeireth-cli` 与 e2e 测试都按
// **根级路径**引用（`apeireth_runtime_assembly::consequence_backfill::…`）
// ⇒ 只能留在根上。
pub mod consequence_backfill;
// 数据根路径解析 (同理由根级公开): `apeireth_runtime_assembly::paths::default_data_root()`
// 被 CLI 与 `foundation/core` 的文档交叉引用。
pub mod paths;
#[cfg(feature = "memory")]
pub mod profile_memory;
pub mod system_one;
pub use portable::{AssemblyError, PortableHost};

// 桌面组装面 (main 的结构): 全部重模块挂在 `desktop` 下 ⇒ **同一文件只被编译一次**。
// ⚠️ 曾经把 `pub mod canonical;` 也留在根上 ⇒ `council_ballot_log` 等模块被编译两份
// ⇒ `expected CouncilBallotRecord, found a different CouncilBallotRecord` (同名不同类型)。
// 根级路径 (`crate::canonical::…` / `apeireth_runtime_assembly::canonical::…`) 由下面的
// glob 再导出维持不变。
#[cfg(feature = "desktop")]
mod desktop;
#[cfg(feature = "desktop")]
pub use desktop::*;

#[cfg(all(feature = "memory", not(feature = "desktop")))]
#[path = "canonical/memory_module.rs"]
pub mod memory_module;
#[cfg(all(feature = "memory", feature = "desktop"))]
pub use canonical::memory_module;

#[cfg(all(feature = "profile-store", not(feature = "desktop")))]
pub mod profile_store;
