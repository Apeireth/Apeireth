//! C-ABI top-level API (skeleton placeholder for V2 D1 build)
//!
//! `unsafe` 局部豁免在此模块（用 `#![allow(unsafe_code)]`），确保 lib root 的
//! `#![deny(unsafe_code)]` 不被误伤。完整 ABI 设计在 V2 D2 R-Cycle 6 战役
//! (docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md 衔接 R-Cycle 7) 才实装。
//!
//! Tech-Review 2026-08-05: 当前 stub 不调用任何 unsafe 块，`extern "C"` 声明本身
//! 是 ABI 约定非 unsafe 块。V2 D2 实装时必须为真实 FFI 调用逐条加 `// SAFETY:` 注释。

#![allow(unsafe_code)]

use crate::error::SdkError;

/// Stub for the negotiation entry point — full negotiation in V2 D2.
#[no_mangle]
pub extern "C" fn apeireth_sdk_init() -> i32 {
    0
}

/// Stub for error-message retrieval — last-error buffer wired in V2 D2.
#[no_mangle]
pub extern "C" fn apeireth_sdk_last_error(_buf: *mut u8, _len: usize) -> i32 {
    -1
}

/// Stub that just exercises the error type to keep the module non-empty.
#[allow(dead_code)]
fn _ensure_error_linked() -> Result<(), SdkError> {
    Ok(())
}
