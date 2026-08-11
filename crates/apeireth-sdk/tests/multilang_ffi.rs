//! `apeireth-sdk::multilang_ffi` — 5 集成测试 (R122-8, cfg-gated per feature)
//!
//! **0 范围扩散**: 仅测试 cfg-gated 桥接的 5 fn (1 per language + 2 C 共享 + 1 compile_info),
//! 0 测 11 agent 公共 API 签名 / 0 测 apeireth-api 集成 (那是 R122-1 retry 范围).
//!
//! **5 集成 test** (per task spec):
//! 1. `sdk_python_ffi_count_tokens_works` (cfg feature python)
//! 2. `sdk_node_ffi_count_tokens_works` (cfg feature node)
//! 3. `sdk_c_ffi_hash_request_returns_same_value_as_rust` (cfg feature c)
//! 4. `sdk_c_ffi_version_returns_semver` (cfg feature c)
//! 5. `sdk_compile_info_includes_features` (cfg feature c)
//!
//! **跨语言一致性**: multilang_ffi.rs 内部 inline 同样的 R32-1 启发式 + R122-1 SHA-256 hex,
//! 验证 C-ABI 跟"参考 Rust 实现"输出 1:1 一致. 0 依赖 c.rs 内部 fn (internal 不 export).

// ============================================================================
// 内部参考实现 (R32-1 + R122-1 1:1 port, 跨语言 1:1 一致性验证用)
// ============================================================================

fn ref_count_tokens_heuristic(text: &str) -> u32 {
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

fn ref_hash_request_sha256(_method: &str, _url: &str, _body: &[u8]) -> String {
    // 跨 features 工作: 优先用 cfg c 的 sha2 (已 optional dep), 否则 Python 1:1 内联 hex
    #[cfg(any(feature = "c", feature = "node"))]
    {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(method.as_bytes());
        hasher.update(b"\n");
        hasher.update(url.as_bytes());
        hasher.update(b"\n");
        hasher.update(body);
        let result = hasher.finalize();
        let mut s = String::with_capacity(result.len() * 2);
        for &b in result.iter() {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
    #[cfg(not(any(feature = "c", feature = "node")))]
    {
        // 0 启用 c/node feature 时, 不需要 sha2, 0 实现
        // (仅 python feature 时 multilang_ffi 0 跑 hash_request test, 因 0 暴露 napi/c 桥接)
        String::new()
    }
}

// ============================================================================
// Test #1: Python 桥接 (cfg feature python)
// ============================================================================

#[cfg(feature = "python")]
#[test]
fn sdk_python_ffi_count_tokens_works() {
    use apeireth_sdk::python::py_count_tokens;
    // 1 fn per language, 1:1 R32-1 算法
    assert_eq!(py_count_tokens("hello", "cl100k_base").expect("py_count_tokens ok"), 1);
    assert_eq!(py_count_tokens("hello world", "cl100k_base").expect("py_count_tokens ok"), 3);
    assert_eq!(py_count_tokens("你好", "cl100k_base").expect("py_count_tokens ok"), 2);
    assert_eq!(py_count_tokens("", "cl100k_base").expect("py_count_tokens ok"), 0);
    // 1:1 跟 ref_count_tokens_heuristic 行为 (R32-1 1:1 port)
    assert_eq!(py_count_tokens("hello 世界", "cl100k_base").expect("ok"), ref_count_tokens_heuristic("hello 世界"));
}

// ============================================================================
// Test #2: Node 桥接 (cfg feature node)
// ============================================================================

#[cfg(feature = "node")]
#[test]
fn sdk_node_ffi_count_tokens_works() {
    use apeireth_sdk::node::count_tokens;
    // 1:1 跟 python.rs py_count_tokens
    assert_eq!(count_tokens("hello".to_string(), "cl100k_base".to_string()), 1);
    assert_eq!(count_tokens("hello world".to_string(), "cl100k_base".to_string()), 3);
    assert_eq!(count_tokens("你好".to_string(), "cl100k_base".to_string()), 2);
    assert_eq!(count_tokens("hello 世界".to_string(), "cl100k_base".to_string()),
               ref_count_tokens_heuristic("hello 世界"));

    // hash_request 确定性测试 (1:1 跨语言)
    use apeireth_sdk::node::hash_request;
    // napi 2.16 Buffer (bindgen_prelude) 实现 From<Vec<u8>> + AsRef<[u8]>
    use napi::bindgen_prelude::Buffer;
    // 创建 2 个独立 Buffer (0 clone, 每调用 1 次)
    let body1 = Buffer::from(b"{}".to_vec());
    let h1 = hash_request("POST".to_string(), "/v1/tools/web_search/invoke".to_string(), body1);

    let body2 = Buffer::from(b"{}".to_vec());
    let h2 = hash_request("POST".to_string(), "/v1/tools/web_search/invoke".to_string(), body2);

    assert_eq!(h1, h2, "同输入 → 同 hash");
    assert_eq!(h1.len(), 64, "SHA-256 hex 长度 = 64");
    assert_eq!(h1, ref_hash_request_sha256("POST", "/v1/tools/web_search/invoke", b"{}"));
}

// ============================================================================
// Test #3: C 桥接 hash_request (cfg feature c) — C-ABI 跟 ref 1:1 一致
// ============================================================================

#[cfg(feature = "c")]
#[test]
fn sdk_c_ffi_hash_request_returns_same_value_as_rust() {
    use apeireth_sdk::c::{apeireth_sdk_free_string, apeireth_sdk_hash_request};
    use std::ffi::CString;
    use std::os::raw::c_uint;

    let method = CString::new("POST").unwrap();
    let url = CString::new("/v1/tools/web_search/invoke").unwrap();
    let body: Vec<u8> = b"{}".to_vec();

    let ptr1 = apeireth_sdk_hash_request(
        method.as_ptr(),
        url.as_ptr(),
        body.as_ptr() as *const c_uint,
        body.len(),
    );
    assert!(!ptr1.is_null());
    let hash_c = unsafe {
        let s = std::ffi::CStr::from_ptr(ptr1);
        s.to_str().unwrap().to_string()
    };
    apeireth_sdk_free_string(ptr1);

    // 跟 ref_hash_request_sha256 直接调用结果一致 (跨 2 语言 1:1 一致性)
    let hash_ref = ref_hash_request_sha256("POST", "/v1/tools/web_search/invoke", b"{}");
    assert_eq!(hash_c, hash_ref, "C-ABI hash 跟 ref SHA-256 一致");
    assert_eq!(hash_c.len(), 64, "SHA-256 hex 长度 = 64");
}

// ============================================================================
// Test #4: C 桥接 version (cfg feature c) — semver X.Y.Z + 0 改 workspace.version
// ============================================================================

#[cfg(feature = "c")]
#[test]
fn sdk_c_ffi_version_returns_semver() {
    use apeireth_sdk::c::apeireth_sdk_version;
    let ptr = apeireth_sdk_version();
    assert!(!ptr.is_null());
    let v_str = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str().unwrap();
    // semver X.Y.Z 格式校验
    let parts: Vec<&str> = v_str.split('.').collect();
    assert_eq!(parts.len(), 3, "version 应是 semver X.Y.Z 格式");
    // 0 改 workspace.version 1.1.0 (per hard-constraint #1)
    // 0 改 SDK_VERSION = 0.1.0 (R20 阶段 6 stub, version.rs:102 LOCKED)
    // version_c 返 SDK_VERSION.as_str() (0.1.0), 跟 workspace.version 1.1.0 解耦
    let sdk_ver = format!("{}.{}.{}",
        apeireth_sdk::SDK_VERSION.major,
        apeireth_sdk::SDK_VERSION.minor,
        apeireth_sdk::SDK_VERSION.patch);
    assert_eq!(v_str, sdk_ver, "version_c 返 SDK_VERSION 0 改");
    apeireth_sdk::c::apeireth_sdk_free_string(ptr as *mut _);
}

// ============================================================================
// Test #5: compile_info 含 features (cfg feature c) — 含 apeireth-sdk + 当前 features
// ============================================================================

#[cfg(feature = "c")]
#[test]
fn sdk_compile_info_includes_features() {
    use apeireth_sdk::c::apeireth_sdk_compile_info;
    let ptr = apeireth_sdk_compile_info();
    assert!(!ptr.is_null());
    let info_str = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str().unwrap();
    // 含 "apeireth-sdk" 标识
    assert!(info_str.contains("apeireth-sdk"), "compile_info 应含 'apeireth-sdk' 标识");
    // 含当前启用 features (per cfg 编译期守门)
    #[cfg(feature = "c")]
    assert!(info_str.contains("c"), "compile_info 应含 'c' feature");
    // O-5 标识: skeleton 0 假装 100%
    assert!(info_str.contains("O-5") || info_str.contains("skeleton"),
            "compile_info 应含 O-5 / skeleton 标识");
    apeireth_sdk::c::apeireth_sdk_free_string(ptr as *mut _);
}

// ============================================================================
// Test #6 (Bonus): default build 0 装 pyo3/napi/cbindgen, 公共 API 0 改 (per O-5 实质)
// ============================================================================

#[cfg(not(any(feature = "python", feature = "node", feature = "c")))]
#[test]
fn sdk_default_build_no_bridge_compiles() {
    // default build 0 装 pyo3/napi/cbindgen (O-5 实质守门)
    // 公共 API 顶层 re-export 0 改 (per lib.rs §A 268-279)
    // SDK_VERSION = 0.1.0 (R20 阶段 6 stub, version.rs:102 LOCKED, 0 改)
    use apeireth_sdk::{SDK_VERSION, STUB_MODE, PLATFORM_NAME};
    assert_eq!(SDK_VERSION.major, 0);
    assert_eq!(SDK_VERSION.minor, 1);
    assert_eq!(SDK_VERSION.patch, 0);
    let _ = STUB_MODE;
    assert_eq!(PLATFORM_NAME, "apeireth", "K-1 #1: 平台名 = 'apeireth'");
    // R122-8 0 改 11 agent 公共 API 签名 (per hard-constraint #6)
}
