//! Apeireth repository tooling facade.
//!
//! Scanning and quality analysis remain separate modules because their public
//! contracts intentionally contain similarly named types such as `CacheEntry`
//! and `ReportGenerator`.

#![warn(missing_docs)]

/// Filesystem, git-state, sensitive-content, and report scanning.
pub mod scan;
/// Technical-debt, complexity, dependency, and security analysis.
pub mod analyzer;