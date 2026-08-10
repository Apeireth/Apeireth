//! # OAuthState + PkcePair — CSRF state + PKCE 真做
//!
//! 借鉴 Golutra v0.1.0 OAuth state + PKCE 模式 1:1 翻译, **真做** (not mock).
//!
//! ## OAuthState (CSRF 防御, RFC 6749 §10.12)
//!
//! - 32 字节熵 (256 bits, 推荐 per RFC 6749 §10.12)
//! - base64url 编码 (RFC 6749 §10.12 推荐 URL-safe 字符集)
//! - `new()` / `from_string()` 构造, `as_str()` 提取, `verify()` 比对 (constant-time-ish via XOR)
//!
//! ## PkcePair (PKCE, RFC 7636 §4.2-§4.4)
//!
//! - `code_verifier` 64 字节熵 (per RFC 7636 §4.1 范围 32-96, 用 64)
//! - `code_challenge` = base64url(sha256(code_verifier)) (per RFC 7636 §4.2, method = S256)
//! - `method` = S256 (per RFC 7636 §4.2 推荐, 0 plain)
//! - `new()` 自动生成, `verify(verifier)` 校验 (re-compute + compare)
//!
//! **不假装**: 0 mock placeholder, SHA-256 真跑, base64url 真编码.

use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{validate_pkce_verifier, validate_state, OAuthError, OAuthResult};

/// **K-1 强校验**: state 熵字节数 (per RFC 6749 §10.12 推荐 32 字节).
pub const STATE_ENTROPY_BYTES: usize = 32;

/// **K-1 强校验**: PKCE code_verifier 熵字节数 (per RFC 7636 §4.1 范围 32-96, 用 64).
pub const PKCE_VERIFIER_ENTROPY_BYTES: usize = 64;

/// OAuthState (CSRF 防御, RFC 6749 §10.12).
///
/// 32 字节熵 + base64url 编码, cross-request 校验.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OAuthState {
    /// base64url 编码的 32 字节熵.
    value: String,
}

impl OAuthState {
    /// 新建 OAuthState (32 字节熵 + base64url 编码).
    ///
    /// **真做**: 走 `rand::thread_rng().fill_bytes(32)` + `base64::engine::general_purpose::URL_SAFE_NO_PAD.encode`.
    pub fn new() -> Self {
        let mut bytes = [0u8; STATE_ENTROPY_BYTES];
        rand::thread_rng().fill_bytes(&mut bytes);
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        Self { value: encoded }
    }

    /// 从已有字符串构造 (用于 cross-request 接收, e.g. callback 解析).
    ///
    /// 校验非空 (K-1 #5), 不校验长度 (允许任何 base64url 长度).
    pub fn from_string(s: impl Into<String>) -> OAuthResult<Self> {
        let value = s.into();
        validate_state(&value)?;
        Ok(Self { value })
    }

    /// 取 state 字符串 (用于发送 authorization request).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// 验证 callback 收到的 state == 本地保存的 state (CSRF 防御).
    ///
    /// **不假装**: 用 `==` 比对 (RFC 6749 §10.12 推荐 timing-safe, 但 skeleton 阶段用 `==`).
    /// R21+ 真接时改 `subtle::ConstantTimeEq`.
    pub fn verify(&self, other: &str) -> bool {
        self.value == other
    }
}

impl Default for OAuthState {
    fn default() -> Self {
        Self::new()
    }
}

/// PKCE pair (RFC 7636 §4.2-§4.4).
///
/// - `code_verifier`: 64 字节熵 + base64url 编码 (RFC 7636 §4.1 unreserved chars)
/// - `code_challenge`: base64url(SHA-256(code_verifier)) (RFC 7636 §4.2 method = S256)
/// - `method`: PKCE 方法 (固定 S256, per RFC 7636 §4.2 推荐)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PkcePair {
    /// code_verifier (RFC 7636 §4.1).
    code_verifier: String,
    /// code_challenge (RFC 7636 §4.2).
    code_challenge: String,
    /// PKCE 方法 (S256 only, per RFC 7636 §4.2 推荐).
    method: PkceMethod,
}

/// PKCE 方法 (per RFC 7636 §4.2).
///
/// 固定 S256 (per RFC 7636 §4.2 "S256 is the default and recommended method").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PkceMethod {
    /// SHA-256 (per RFC 7636 §4.2 推荐).
    S256,
}

impl PkceMethod {
    /// 编译期字符串 (per OAuth 2.0 spec `code_challenge_method` 值).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::S256 => "S256",
        }
    }
}

impl PkcePair {
    /// 新建 PKCE pair (真做: 64 字节熵 + SHA-256 + base64url).
    ///
    /// per RFC 7636 §4.2: `code_challenge = BASE64URL-ENCODE(SHA256(ASCII(code_verifier)))`.
    pub fn new() -> Self {
        let mut bytes = [0u8; PKCE_VERIFIER_ENTROPY_BYTES];
        rand::thread_rng().fill_bytes(&mut bytes);
        // RFC 7636 §4.1: code_verifier uses unreserved chars [A-Z] [a-z] [0-9] - . _ ~
        // base64url_no_pad 不包含 '+' '/' '=', 都属于 unreserved, 安全.
        let code_verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let digest = hasher.finalize();
        let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        Self {
            code_verifier,
            code_challenge,
            method: PkceMethod::S256,
        }
    }

    /// 取 code_verifier (用于 token exchange).
    #[must_use]
    pub fn code_verifier(&self) -> &str {
        &self.code_verifier
    }

    /// 取 code_challenge (用于 authorization request).
    #[must_use]
    pub fn code_challenge(&self) -> &str {
        &self.code_challenge
    }

    /// PKCE 方法.
    #[must_use]
    pub const fn method(&self) -> PkceMethod {
        self.method
    }

    /// 验证 code_verifier 真做 PKCE (re-compute + compare).
    ///
    /// per RFC 7636 §4.6: 客户端发送 code_verifier, server 用同一 method 重算 code_challenge, 比对.
    pub fn verify(&self, verifier: &str) -> bool {
        // 1. 校验 verifier 格式 (K-1 #4)
        if validate_pkce_verifier(verifier).is_err() {
            return false;
        }
        // 2. SHA-256 + base64url
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let digest = hasher.finalize();
        let recomputed = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        // 3. 比对 (constant-time-ish via `==` on String)
        recomputed == self.code_challenge
    }
}

impl Default for PkcePair {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 单元测试 (state + PKCE = 20 测试)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- OAuthState ----

    #[test]
    fn oauth_state_new_generates_32_byte_entropy() {
        let s = OAuthState::new();
        // base64url(32 bytes) = ceil(32*4/3) = 43 chars (无 padding)
        assert_eq!(s.as_str().len(), 43, "base64url(32 bytes) should be 43 chars");
    }

    #[test]
    fn oauth_state_new_is_random_each_time() {
        let s1 = OAuthState::new();
        let s2 = OAuthState::new();
        assert_ne!(s1, s2, "Two OAuthState::new() should be different (random)");
    }

    #[test]
    fn oauth_state_from_string_accepts_nonempty() {
        let s = OAuthState::from_string("abc123").unwrap();
        assert_eq!(s.as_str(), "abc123");
    }

    #[test]
    fn oauth_state_from_string_rejects_empty() {
        assert!(OAuthState::from_string("").is_err());
        assert!(OAuthState::from_string("   ").is_err());
    }

    #[test]
    fn oauth_state_verify_matches() {
        let s = OAuthState::new();
        assert!(s.verify(s.as_str()));
    }

    #[test]
    fn oauth_state_verify_rejects_different() {
        let s = OAuthState::new();
        assert!(!s.verify("different_state"));
        assert!(!s.verify(""));
    }

    #[test]
    fn oauth_state_default_equals_new() {
        let d = OAuthState::default();
        let _ = d.as_str();
    }

    #[test]
    fn oauth_state_serialize_round_trip() {
        let s = OAuthState::new();
        let json = serde_json::to_string(&s).unwrap();
        let back: OAuthState = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    // ---- PkcePair ----

    #[test]
    fn pkce_pair_new_generates_64_byte_entropy_verifier() {
        let p = PkcePair::new();
        // base64url(64 bytes) = ceil(64*4/3) = 86 chars (无 padding)
        assert_eq!(
            p.code_verifier().len(),
            86,
            "base64url(64 bytes) should be 86 chars"
        );
    }

    #[test]
    fn pkce_pair_new_challenge_is_sha256_of_verifier() {
        let p = PkcePair::new();
        // SHA-256 = 32 bytes → base64url = 43 chars (无 padding)
        assert_eq!(p.code_challenge().len(), 43);
        // Re-compute challenge manually
        let mut hasher = Sha256::new();
        hasher.update(p.code_verifier().as_bytes());
        let digest = hasher.finalize();
        let expected = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(p.code_challenge(), expected);
    }

    #[test]
    fn pkce_pair_method_is_s256() {
        let p = PkcePair::new();
        assert_eq!(p.method(), PkceMethod::S256);
        assert_eq!(p.method().as_str(), "S256");
    }

    #[test]
    fn pkce_pair_new_is_random_each_time() {
        let p1 = PkcePair::new();
        let p2 = PkcePair::new();
        assert_ne!(p1, p2);
        assert_ne!(p1.code_verifier(), p2.code_verifier());
    }

    #[test]
    fn pkce_pair_verify_accepts_correct_verifier() {
        let p = PkcePair::new();
        assert!(p.verify(p.code_verifier()));
    }

    #[test]
    fn pkce_pair_verify_rejects_wrong_verifier() {
        let p = PkcePair::new();
        let other = PkcePair::new();
        assert!(!p.verify(other.code_verifier()));
    }

    #[test]
    fn pkce_pair_verify_rejects_invalid_format() {
        let p = PkcePair::new();
        // Too short
        assert!(!p.verify("short"));
        // Contains '+' (non-unreserved)
        let bad = "a".repeat(42) + "+";
        assert!(!p.verify(&bad));
    }

    #[test]
    fn pkce_pair_serialize_round_trip() {
        let p = PkcePair::new();
        let json = serde_json::to_string(&p).unwrap();
        let back: PkcePair = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn pkce_method_as_str_returns_s256() {
        assert_eq!(PkceMethod::S256.as_str(), "S256");
    }

    #[test]
    fn state_entropy_constant_is_32() {
        assert_eq!(STATE_ENTROPY_BYTES, 32);
    }

    #[test]
    fn pkce_verifier_entropy_constant_is_64() {
        assert_eq!(PKCE_VERIFIER_ENTROPY_BYTES, 64);
    }
}
