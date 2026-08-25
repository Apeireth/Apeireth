#![allow(unexpected_cfgs)]

//! v1 era apeireth-tool-browser transcription (10 files).
//!
//! Source: crates/_archived/v1.0-legacy/apeireth-tool-browser/src
//! Files transcribed verbatim:
//!   - accessibility.rs
//!   - browser.rs
//!   - cdp.rs
//!   - cli.rs
//!   - compat.rs
//!   - enhanced.rs
//!   - fetch.rs
//!   - mcp.rs
//!   - organ_kani_proofs.rs (kept verbatim)
//!   - register.rs

// v1 lib.rs (transcribed + reconstructed to wire the modules below)
pub mod lib;
pub mod accessibility;
pub mod browser;
pub mod cli;
pub mod compat;
pub mod enhanced;
pub mod fetch;
pub mod mcp;
pub mod register;
#[cfg(feature = "cdp")]
pub mod cdp;
mod organ_kani_proofs;
