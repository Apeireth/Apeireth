//! Apeireth repository tooling facade.
//!
//! Scanning and quality analysis remain separate modules because their public
//! contracts intentionally contain similarly named types such as `CacheEntry`
//! and `ReportGenerator`.

#![warn(missing_docs)]

/// Filesystem, git-state, sensitive-content, and report scanning.
pub mod scan;
// R177: organ invariants (5 tests + 2 Kani)
/// Technical-debt, complexity, dependency, and security analysis.
pub mod analyzer;
mod organ_kani_proofs;
/// N17/TP2: 装配统一注册件 (§10 铁边界: Tool + ToolRegistry.register)
pub mod register;
