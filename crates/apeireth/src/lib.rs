// Copyright (c) 2026 The Apeireth Authors.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! # Apeireth 引擎门面（facade）
//!
//! **本 crate 只做 re-export：零业务逻辑、零新 trait、零新注册表、零新能力。**
//!
//! 它的价值是**入口**：下游只需一行依赖
//!
//! ```toml
//! apeireth = "2.0.0-rc.4"
//! ```
//!
//! 即可拿到引擎核心模块；需要更多模块时按 feature 打开，不必逐个 crate 手写依赖。
//!
//! ## 默认导出（3 个）
//!
//! | 门面路径 | 实际 crate |
//! |---|---|
//! | [`apeireth::core`] | `apeireth-core` |
//! | [`apeireth::protocol`] | `apeireth-protocol` |
//! | [`apeireth::runtime`] | `apeireth-runtime` |
//!
//! **默认依赖树实测 92 个包**（= 91 个依赖 + 门面本体 1 个；ADR-0020 瘦身：原先把 `tools` 放在默认面，
//! 实测传递依赖带进 160 个包 ⇒ 总体 252 → **92**，减 63.5%）。
//!
//! ## 可选模块（feature 门控，默认关闭）
//!
//! | feature | 门面路径 | 实际 crate |
//! |---|---|---|
//! | `memory` | [`apeireth::memory`] | `apeireth-memory` |
//! | `guard` | [`apeireth::guard`] | `apeireth-guard` |
//! | `orchestration` | [`apeireth::orchestration`] | `apeireth-orchestration` |
//! | `tools` | [`apeireth::tools`] | `apeireth-tools-canonical`（**传递依赖重：+160 包**） |
//!
//! **不在门面内**：`apeireth-organ` / `apeireth-image-core` —— 需要时直接依赖对应 crate
//! （图像家族含 10 个 vendored crate，依赖树较重，且会挤占生产边预算）。
//!
//! ```toml
//! apeireth = { version = "2.0.0-rc.4", features = ["tools", "memory"] }
//! ```
//!
//! ## 纪律（0 装）
//!
//! - **门面不新增能力**：本 crate 里出现任何非 `pub use` 的实质逻辑，即为违反本 ADR。
//! - **re-export 的模块集合只增不减**：删模块即门面 breaking（下游会跟着断）。
//! - **能力四级口径**（IMPLEMENTED / PRODUCTION WIRED / DEFAULT ENABLED /
//!   HARDWARE VALIDATED）不在此处声明，单一真源见
//!   `docs/03-reference/capabilities-matrix.md`；测试数字单一真源见
//!   `docs/03-reference/test-baseline.md`（本文不抄写）。
//!
//! ## 许可
//!
//! Apache-2.0 OR MIT。第三方归属见仓库根 `NOTICE` / `OSS_NOTICE.md` /
//! `THIRD-PARTY-NOTICES.md`。

#![forbid(unsafe_code)]

// --- 默认导出：四个核心模块 ---
pub use apeireth_core as core;
pub use apeireth_protocol as protocol;
pub use apeireth_runtime as runtime;

// --- 可选模块（feature 门控） ---
#[cfg(feature = "guard")]
pub use apeireth_guard as guard;
#[cfg(feature = "memory")]
pub use apeireth_memory as memory;
#[cfg(feature = "orchestration")]
pub use apeireth_orchestration as orchestration;
#[cfg(feature = "tools")]
pub use apeireth_tools_canonical as tools;

/// 门面 crate 的版本号（跟随 workspace 单轴）。
///
/// 便于下游在日志/诊断里打印"我用的是哪一版门面"，无需额外依赖。
pub const FACADE_VERSION: &str = env!("CARGO_PKG_VERSION");
