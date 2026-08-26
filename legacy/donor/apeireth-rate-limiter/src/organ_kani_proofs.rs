//! R177 rate-limiter organ Kani proofs (W8)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_rl_01_config_default() {
    let c = RateLimiterConfig::default();
    let _: String = format!("{:?}", c);
}

#[test]
fn r177_rl_02_bucket_config_default() {
    let b = BucketConfig::default();
    assert!(b.rate_per_second > 0.0);
    assert!(b.burst > 0);
}

#[test]
fn r177_rl_03_strategy_kind_default() {
    let s = StrategyConfig::default();
    assert_eq!(s.kind, StrategyKind::TokenBucket);
}

#[test]
fn r177_rl_04_storage_kind_default() {
    let s = StorageConfig::default();
    assert_eq!(s.kind, StorageKind::InMemory);
}

#[test]
fn r177_rl_05_observability_default() {
    let o = ObservabilityConfig::default();
    assert!(o.track_stats);
}

#[cfg(kani)]
#[kani::proof]
fn r177_rl_kani_01_bucket_rate_positive() {
    let b = BucketConfig::default();
    assert!(b.rate_per_second > 0.0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_rl_kani_02_bucket_burst_positive() {
    let b = BucketConfig::default();
    assert!(b.burst >= 1);
}
