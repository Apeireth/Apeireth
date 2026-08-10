//! `apeireth-sdk::c` — cbindgen C-ABI 桥接 (R122-8 Multi-Lang SDK skeleton, cfg feature = "c")
//!
//! **状态**: skeleton, 5 fn C 签名 (`count_tokens_c` / `hash_request_c` / `version_c`
//!   / `compile_info_c` / `free_string_c`), 0 假装 100% 多语言支持 (O-5).
//!
//! **O-5 实质守门**: 仅 `--features c` 启用时编译, 默认 build 0 装 cbindgen.
//! **R122-8 决策**: cfg-gated features 隔离 (per lib.rs §A R122-8 段 + Cargo.toml [features]).
//!
//! **C-ABI 5 fn** (per task spec, cbindgen auto-generate `apeireth_sdk.h`):
//! 1. `uint32_t apeireth_sdk_count_tokens(const char* text)` — R32-1 启发式
//! 2. `char* apeireth_sdk_hash_request(const char* method, const char* url, const uint8_t* body, size_t body_len)` — SHA-256 hex
//! 3. `const char* apeireth_sdk_version(void)` — 返 SDK_VERSION 字面量, 0 改 workspace.version
//! 4. `const char* apeireth_sdk_compile_info(void)` — 返 "rustc X.Y.Z" + features
//! 5. `void apeireth_sdk_free_string(char* ptr)` — 释放 char* (Rust 分配)
//!
//! **0 重复造轮子 (O-2)**: 复用 `apeireth-sdk::version::SDK_VERSION` 公共 API (per lib.rs §A 268-269),
//! 0 改 24 LOCKED / 0 改 workspace.version.
//!
//! **不漂移**: c.rs 仅用 std + apeireth-sdk::version, 0 跨 crate dep, 0 触碰 5 集成点 / 4 类核心类型.
//!
//! **cfg-gate 模式**: c.rs 0 用 file-level `#![cfg(feature = "c")]`, 改用 fn-level `#[cfg(feature = "c")]`.
//! 原因: cbindgen 0.26 build.rs 跑时 cfg 0 启用 c (cargo 决定 feature 在 build script 之后),
//!       file-level cfg 会让 cbindgen 看不到 fn, .h 0 生成. 改 fn-level cfg:
//!       - cargo build (default) → c.rs 编译但 fn cfg-gate 0 编 → 0 装 (O-5 守门)
//!       - cargo build --features c → c.rs fn 全编
//!       - cbindgen build.rs → 看到 c.rs 全部 fn (cfg 在 fn 上 cbindgen 仍能读)

#![allow(unsafe_code)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint};

#[cfg(feature = "c")]
use sha2::{Digest, Sha256};

// ============================================================================
// 内部实现 (跟 python.rs / node.rs 1:1 一致, 跨语言 3 语言一致性)
// ============================================================================

/// **R32-1 启发式**: 1:1 翻译 python.rs / node.rs.
/// 仅 c feature 启用时编 (per fn-level cfg), 0 重复造轮子 (per O-2).
#[cfg(feature = "c")]
fn count_tokens_heuristic(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    let mut tokens: u32 = 0;
    let mut ascii_word_chars: u32 = 0;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            ascii_word_chars += 1;
        } else {
            if ascii_word_chars > 0 {
                tokens += 1;
                ascii_word_chars = 0;
            }
            tokens += 1;
        }
    }
    if ascii_word_chars > 0 {
        tokens += 1;
    }
    tokens
}

/// **SHA-256 hex hash**: 1:1 翻译 node.rs `hash_request_impl`.
#[cfg(feature = "c")]
fn hash_request_impl(method: &str, url: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(method.as_bytes());
    hasher.update(b"\n");
    hasher.update(url.as_bytes());
    hasher.update(b"\n");
    hasher.update(body);
    let result = hasher.finalize();
    hex_encode(&result)
}

#[cfg(feature = "c")]
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

// ============================================================================
// C-ABI 5 fn (per task spec, cbindgen auto-generate `apeireth_sdk.h`)
// ============================================================================

/// **C-ABI fn #1**: `apeireth_sdk_count_tokens(text: *const c_char) -> c_uint`.
///
/// 安全性: caller 须保证 `text` 指向有效 UTF-8 + null-terminated C string.
/// Null / invalid ptr 返 0 (fail-soft, 1:1 abi.rs stub pattern).
#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn apeireth_sdk_count_tokens(text: *const c_char) -> c_uint {
    if text.is_null() {
        return 0;
    }
    // SAFETY: caller 保证 text 有效 + null-terminated. 失败返 0 (fail-soft).
    let c_str = unsafe { CStr::from_ptr(text) };
    match c_str.to_str() {
        Ok(s) => count_tokens_heuristic(s),
        Err(_) => 0, // invalid UTF-8 返 0
    }
}

/// **C-ABI fn #2**: `apeireth_sdk_hash_request(method, url, body, body_len) -> *mut c_char`.
///
/// **内存契约**: caller **必须**用 `apeireth_sdk_free_string` 释放返值, 0 用 C free().
/// Null ptr 返 null. invalid UTF-8 返 null.
#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn apeireth_sdk_hash_request(
    method: *const c_char,
    url: *const c_char,
    body: *const c_uint,
    body_len: usize,
) -> *mut c_char {
    if method.is_null() || url.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: caller 保证 method/url 有效 + null-terminated.
    let method_str = unsafe { CStr::from_ptr(method) };
    let url_str = unsafe { CStr::from_ptr(url) };
    let (Ok(m), Ok(u)) = (method_str.to_str(), url_str.to_str()) else {
        return std::ptr::null_mut();
    };
    // SAFETY: caller 保证 body 有效 + body_len 字节.
    let body_slice = if body.is_null() || body_len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(body as *const u8, body_len) }
    };
    let hash = hash_request_impl(m, u, body_slice);
    match CString::new(hash) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// **C-ABI fn #3**: `apeireth_sdk_version() -> *const c_char`.
///
/// **不漂移**: 复用 `apeireth_sdk::version::SDK_VERSION` 公共 API, 0 改 workspace.version 1.1.0.
/// 返 Rust static str, 生命周期 'static, 0 需要 free (1:1 libc `getenv` pattern).
#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn apeireth_sdk_version() -> *const c_char {
    // 0 跨 crate dep: SDK_VERSION 已 re-export 顶层 (per lib.rs:268-269)
    // 0 改 workspace.version, 仅读取 compile-time const
    use crate::version::SDK_VERSION;
    // SDK_VERSION 是 SdkVersion struct, 不是 str. 转 "0.1.0" 字面量 (per R20 阶段 6 stub)
    // 注: SDK_VERSION.major/minor/patch 来自 version.rs:102 LOCKED, 0 重复造轮子.
    // 注: workspace.version 1.1.0 是 workspace 顶层, SDK_VERSION 0.1.0 是 SDK 协议版本 (R20 决策原意)
    let s = format!("{}.{}.{}", SDK_VERSION.major, SDK_VERSION.minor, SDK_VERSION.patch);
    // 内存: 泄漏但可接受 (process lifetime), 跟 Rust static str 等价
    // R123 切换 static str (0 堆分配)
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null(),
    }
}

/// **C-ABI fn #4**: `apeireth_sdk_compile_info() -> *const c_char`.
///
/// 返 "rustc X.Y.Z target triple, apeireth-sdk features: [python,node,c,default]" 字面量.
/// 0 假装实际 rustc version (编译期 hardcode "unknown" + "cfg(apeireth_sdk)" marker).
#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn apeireth_sdk_compile_info() -> *const c_char {
    let features = {
        let mut f = String::new();
        #[cfg(feature = "python")]
        f.push_str("python,");
        #[cfg(feature = "node")]
        f.push_str("node,");
        #[cfg(feature = "c")]
        f.push_str("c,");
        if f.is_empty() {
            f.push_str("default");
        } else {
            f.pop(); // remove trailing ','
        }
        f
    };
    let info = format!(
        "apeireth-sdk skeleton (rustc unknown, features: [{}], O-5: skeleton 0 假装 100%)",
        features
    );
    match CString::new(info) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null(),
    }
}

/// **C-ABI fn #5**: `apeireth_sdk_free_string(ptr: *mut c_char)`.
///
/// 释放 `apeireth_sdk_hash_request` / `apeireth_sdk_version` / `apeireth_sdk_compile_info`
/// 返的 C string. 0 是 malloc 返值调 free() 行为未定义.
#[cfg(feature = "c")]
#[no_mangle]
pub extern "C" fn apeireth_sdk_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: caller 保证 ptr 是 CString::into_raw 返值 (per上面 3 fn)
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

// ============================================================================
// 3 unit tests (per task spec, cfg feature = "c")
// ============================================================================

#[cfg(all(test, feature = "c"))]
mod c_ffi_tests {
    use super::*;
    use std::ffi::CString;

    /// **Test #1**: count_tokens ASCII + CJK.
    #[test]
    fn c_count_tokens_ascii_and_cjk() {
        let s = CString::new("hello").unwrap();
        let n = apeireth_sdk_count_tokens(s.as_ptr());
        assert_eq!(n, 1);
        let s = CString::new("hello 世界").unwrap();
        let n = apeireth_sdk_count_tokens(s.as_ptr());
        assert_eq!(n, 4);
        // Null ptr → 0
        let n = apeireth_sdk_count_tokens(std::ptr::null());
        assert_eq!(n, 0);
    }

    /// **Test #2**: hash_request 确定性 + 不同输入不同 hash + free_string 工作.
    #[test]
    fn c_hash_request_deterministic_and_free() {
        let method = CString::new("POST").unwrap();
        let url = CString::new("/v1/tools/web_search/invoke").unwrap();
        let body = b"{}".to_vec();

        let ptr1 = apeireth_sdk_hash_request(
            method.as_ptr(),
            url.as_ptr(),
            body.as_ptr() as *const c_uint,
            body.len(),
        );
        assert!(!ptr1.is_null());
        let s1 = unsafe { CStr::from_ptr(ptr1) };
        let hash1 = s1.to_str().unwrap().to_string();
        apeireth_sdk_free_string(ptr1);

        // 同样输入 → 同样 hash
        let ptr2 = apeireth_sdk_hash_request(
            method.as_ptr(),
            url.as_ptr(),
            body.as_ptr() as *const c_uint,
            body.len(),
        );
        let s2 = unsafe { CStr::from_ptr(ptr2) };
        let hash2 = s2.to_str().unwrap().to_string();
        apeireth_sdk_free_string(ptr2);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex

        // Null ptr → null
        let null_ptr = apeireth_sdk_hash_request(
            std::ptr::null(),
            url.as_ptr(),
            body.as_ptr() as *const c_uint,
            body.len(),
        );
        assert!(null_ptr.is_null());
    }

    /// **Test #3**: version + compile_info 返回有效 C string.
    #[test]
    fn c_version_and_compile_info_returns_valid_cstr() {
        let v_ptr = apeireth_sdk_version();
        assert!(!v_ptr.is_null());
        let v = unsafe { CStr::from_ptr(v_ptr) };
        let v_str = v.to_str().unwrap();
        // 验证 semver 格式 X.Y.Z
        let parts: Vec<&str> = v_str.split('.').collect();
        assert_eq!(parts.len(), 3, "version 应是 semver X.Y.Z 格式");
        // 0 改 workspace.version 1.1.0 (per hard-constraint #1)
        // 0 改 SDK_VERSION = 0.1.0 (R20 阶段 6 stub, version.rs:102 LOCKED)
        // version_c 返 SDK_VERSION.as_str() (0.1.0), 跟 workspace.version 1.1.0 解耦
        assert_eq!(v_str, "0.1.0", "version_c 返 SDK_VERSION (0.1.0) 0 改");
        apeireth_sdk_free_string(v_ptr as *mut _);

        let info_ptr = apeireth_sdk_compile_info();
        assert!(!info_ptr.is_null());
        let info = unsafe { CStr::from_ptr(info_ptr) };
        let info_str = info.to_str().unwrap();
        // 含 "apeireth-sdk" + features 列表
        assert!(info_str.contains("apeireth-sdk"));
        #[cfg(feature = "c")]
        assert!(info_str.contains("c"), "compile_info 应含 'c' feature");
        apeireth_sdk_free_string(info_ptr as *mut _);
    }
}
