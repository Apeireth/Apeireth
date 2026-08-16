//! R177 api organ Kani proofs (W7)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_api_01_platform_version() {
    assert!(!PLATFORM_VERSION.is_empty());
}

#[test]
fn r177_api_02_timeout_ms() {
    assert_eq!(DEFAULT_TIMEOUT_MS, 60_000);
}

#[test]
fn r177_api_03_max_retries() {
    assert_eq!(DEFAULT_MAX_RETRIES, 3);
}

#[test]
fn r177_api_04_retry_backoff() {
    assert_eq!(DEFAULT_RETRY_BACKOFF_BASE_MS, 500);
}

#[test]
fn r177_api_05_max_concurrent() {
    assert_eq!(DEFAULT_MAX_CONCURRENT, 32);
}

#[cfg(kani)]
#[kani::proof]
fn r177_api_kani_01_timeout_positive() {
    assert!(DEFAULT_TIMEOUT_MS > 0);
}

#[cfg(kani)]
#[kani::proof]
fn r177_api_kani_02_retries_positive() {
    assert!(DEFAULT_MAX_RETRIES > 0);
}
