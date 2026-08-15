//! Apeireth repository tooling facade.
//!
//! Scanning and quality analysis remain separate modules because their public
//! contracts intentionally contain similarly named types such as `CacheEntry`
//! and `ReportGenerator`.

#![warn(missing_docs)]

/// Filesystem, git-state, sensitive-content, and report scanning.
pub mod scan;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
/// Technical-debt, complexity, dependency, and security analysis.
pub mod analyzer;