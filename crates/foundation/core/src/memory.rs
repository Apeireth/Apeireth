//! P-arch (2026-08-27): core drain 第二阶段 (O-6 重构批次 Refactor-5).
//!
//! **位置**: 本模块是**legacy compat shim**, 不再定义域类型.
//! 域类型 (Episode / Note / Session / IdentityCard / Migration) 已搬到
//! `crate::kernel::memory` (canonical source of truth).
//!
//! v1 compat: `use apeireth_core::Episode;` 仍可用 (lib.rs `pub use memory::*`),
//! 新代码推荐: `use apeireth_core::kernel::Episode;` 表达 v2 canonical 意图.
//!
//! v2.0.0-rc 阶段: 给本模块 re-export 加 `#[deprecated]` + 12 consumer 批量迁 kernel
//! (那时删 `pub use memory::*` 完成真 drain).

pub use crate::kernel::memory::{Episode, IdentityCard, Migration, Note, Session};
