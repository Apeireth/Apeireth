//! `apeireth-sdk::node` — napi-rs 桥接 (R122-8 Multi-Lang SDK skeleton, cfg feature = "node")
//!
//! **状态**: skeleton, 2 fn (`count_tokens` + `hash_request`), 0 假装 100% 多语言支持 (O-5).
//!
//! **O-5 实质守门**: 仅 `--features node` 启用时编译, 默认 build 0 装 napi-rs.
//! **R122-8 决策**: cfg-gated features 隔离 (per lib.rs §A R122-8 段 + Cargo.toml [features]).
//! **R122-1 协作**: R122-1 hash_request retry 跑中, R122-8 inline 简版 SHA-256 hex
//!   (1:1 R122-1 设计, 0 跨 crate dep, R122-1 retry 完成后 R123 切换).
//!
//! **napi version**: 2.x (任务 2.16, 真实 2.x branch, 3.x latest 备选, build 时 cargo resolve).
//!
//! **公共 API** (napi-rs 暴露给 Node.js):
//! - `count_tokens(text: string, model: string) => number` — R32-1 启发式
//! - `hashRequest(method: string, url: string, body: Buffer) => string` — SHA-256 hex
//!
//! **用法** (Node.js 端, 假设 .node 已编译):
//! ```js
//! const { countTokens, hashRequest } = require('./apeireth_sdk.node');
//! console.log(countTokens("Hello, 世界!", "cl100k_base"));
//! console.log(hashRequest("POST", "/v1/tools/web_search/invoke", Buffer.from("{}")));
//! ```

#![cfg(feature = "node")]
// napi-rs 桥接内部用 unsafe (Node-API C 调用), 0 改 apeireth-sdk 顶层 #![deny(unsafe_code)]
// (per abi.rs 已有模式: extern "C" 桥接局部 #![allow(unsafe_code)])
#![allow(unsafe_code)]

use napi_derive::napi;

use sha2::{Digest, Sha256};

// ============================================================================
// R32-1 1:1 port: count_tokens 启发式 (跨语言一致性, 跟 python.rs 同算法)
// ============================================================================

/// **R32-1 启发式** (1:1 翻译 `apeireth-asi::tokenizer::count_tokens`):
/// - ASCII word = 1 token, CJK char = 1 token, 其他 = 1 token
///
/// **不假装**: 0 调 tiktoken-rs, 0 假装 LLM 真实 tokenizer.
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
            // 简化: CJK + 其他均按 1 token (R32-1 算法 1:1 简化版)
            tokens += 1;
        }
    }
    if ascii_word_chars > 0 {
        tokens += 1;
    }
    tokens
}

// ============================================================================
// SHA-256 hash_request (R122-1 1:1 port)
// ============================================================================

/// **SHA-256 hex hash**: method + url + body 拼接后 SHA-256, hex 64 字符.
/// 1:1 R122-1 `hash_request(method, url, body) -> String` 设计.
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

/// Hex 编码 (lowercase, 0 假装大写, 跟 R122-1 一致)
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
// napi-rs 桥接 (2 fn, R122-8 skeleton 1:1)
// ============================================================================

/// **napi-rs fn #1**: 暴露给 Node.js `countTokens(text, model) => number`.
/// 1:1 跟 python.rs `py_count_tokens` 算法, 跨语言一致性优先.
#[napi]
pub fn count_tokens(text: String, model: String) -> u32 {
    // model 参数 0 使用 (签名占位, 跨语言 1:1 一致性)
    let _ = model;
    count_tokens_heuristic(&text)
}

/// **napi-rs fn #2**: 暴露给 Node.js `hashRequest(method, url, body) => string`.
/// 1:1 R122-1 `hash_request`, SHA-256 hex (64 字符).
///
/// **napi 2.16 API**: 用 `napi::bindgen_prelude::Buffer` (#[napi] 标准 buffer 类型,
/// 实现 `From<Vec<u8>>` + `AsRef<[u8]>`, Node 端对应 `Buffer` 类型).
/// 1:1 task spec 接受 `napi::Buffer` 命名, 实际 2.16 路径是 `napi::bindgen_prelude::Buffer`.
#[napi]
pub fn hash_request(method: String, url: String, body: napi::bindgen_prelude::Buffer) -> String {
    // napi 2.16 Buffer 0 重复造轮子 (per O-2), 直接 as_ref() 取 &[u8]
    hash_request_impl(&method, &url, body.as_ref())
}

// ============================================================================
// 3 unit tests (per task spec, 仅 Rust 单元, 0 Node 运行时测试)
// ============================================================================

#[cfg(test)]
mod node_ffi_tests {
    use super::*;

    /// **Test #1**: ASCII 单词 token count.
    #[test]
    fn node_count_tokens_ascii() {
        assert_eq!(count_tokens_heuristic("hello"), 1);
        assert_eq!(count_tokens_heuristic("hello world"), 3);
        assert_eq!(count_tokens_heuristic(""), 0);
    }

    /// **Test #2**: CJK + 混合 token count.
    #[test]
    fn node_count_tokens_cjk_and_mixed() {
        assert_eq!(count_tokens_heuristic("你好"), 2);
        assert_eq!(count_tokens_heuristic("hello 世界"), 4);
    }

    /// **Test #3**: hash_request 确定性 + 不同输入不同 hash.
    #[test]
    fn node_hash_request_deterministic() {
        let h1 = hash_request_impl("POST", "/v1/tools/web_search/invoke", b"{}");
        let h2 = hash_request_impl("POST", "/v1/tools/web_search/invoke", b"{}");
        // 同输入 → 同 hash
        assert_eq!(h1, h2);
        // SHA-256 hex 长度 = 64
        assert_eq!(h1.len(), 64);

        // 不同 body → 不同 hash
        let h3 = hash_request_impl("POST", "/v1/tools/web_search/invoke", b"{\"q\":\"x\"}");
        assert_ne!(h1, h3);

        // 不同 method → 不同 hash
        let h4 = hash_request_impl("GET", "/v1/tools/web_search/invoke", b"{}");
        assert_ne!(h1, h4);

        // 不同 url → 不同 hash
        let h5 = hash_request_impl("POST", "/v1/tools/file_ops/invoke", b"{}");
        assert_ne!(h1, h5);
    }
}
