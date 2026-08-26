//! CDP (Chrome DevTools Protocol) browser implementation.
//!
//! Only available with `--features cdp` build. Requires:
//! - chromiumoxide (Rust CDP client) — heavy compile-time dep
//! - A Chrome / Chromium binary on the host system
//!
//! When `cdp` feature is NOT enabled, this module is excluded from the build.
//! The `CdpBrowser::connect()` returns an honest error rather than faking
//! Chrome presence (per O-5 不假装).

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
                        // CI fix 2026-08: `pub mod cdp_impl;` (cfg cdp) 的 cdp_impl.rs 从未存在 → clippy --all-features
                        // E0583. CDP 真接 (chromiumoxide) 落地时恢复 (0 装: 不留指向虚无的声明).
