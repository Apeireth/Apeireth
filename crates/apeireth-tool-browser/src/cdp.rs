//! CDP (Chrome DevTools Protocol) browser implementation.
//!
//! Only available with `--features cdp` build. Requires:
//! - chromiumoxide (Rust CDP client) — heavy compile-time dep
//! - A Chrome / Chromium binary on the host system
//!
//! When `cdp` feature is NOT enabled, this module is excluded from the build.
//! The `CdpBrowser::connect()` returns an honest error rather than faking
//! Chrome presence (per O-5 不假装).

#[cfg(feature = "cdp")]
pub mod cdp_impl;