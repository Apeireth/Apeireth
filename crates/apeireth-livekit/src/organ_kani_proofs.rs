//! R177 livekit organ Kani proofs (W12)

#![allow(missing_docs)]

use crate::*;

#[test]
fn r177_lk_01_schema_version() {
    assert_eq!(LIVEKIT_SCHEMA_VERSION, "1");
}

#[test]
fn r177_lk_02_twirp_prefix() {
    assert_eq!(LIVEKIT_TWIRP_PREFIX, "/twirp");
}

#[test]
fn r177_lk_03_stub_mode() {
    assert_eq!(STUB_MODE, true);
}

#[test]
fn r177_lk_04_endpoint_count() {
    assert_eq!(LIVEKIT_ENDPOINT_COUNT, 6);
}

#[test]
fn r177_lk_05_tool_whitelist() {
    assert_eq!(TOOL_WHITELIST_COUNT, 7);
}

#[cfg(kani)]
#[kani::proof]
fn r177_lk_kani_01_endpoint_count() {
    assert_eq!(LIVEKIT_ENDPOINT_COUNT, 6);
}

#[cfg(kani)]
#[kani::proof]
fn r177_lk_kani_02_max_token_ttl() {
    assert!(MAX_TOKEN_TTL_SECONDS >= 1);
}

