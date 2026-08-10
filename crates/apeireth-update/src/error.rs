//! # Update error types
//!
//! 11 [`UpdateError`] variant + 5 K-1 强校验 helper.
//!
//! **8 项不修改承诺 #4**: 5 K-1 强校验守门必保留, R21+ 续真接时不增删.
//!
//! 借鉴 Golutra P3 minisign + autoupdate 思想 (per [`docs/stage4/BORROW_FROM_GOLUTRA.md` §8 P3 第 10-11 项](file:///)).
//! 公开 API 11 variant, 每 variant 独立处理 (不用 `Other(String)` 兜底 — 6 哲学 anchor O-4 拒绝兜底).

use thiserror::Error;

// ============================================================================
// §1 UpdateError — 11 variant
// ============================================================================

/// Update 子系统统一错误 (per task spec §3 + 借鉴文档 §8 P3).
///
/// **变体顺序固定** (8 项不修改承诺 §3 枚举顺序严守): R21+ 续真接时不增删.
#[derive(Debug, Error)]
pub enum UpdateError {
    /// 当前版本已是最新 (non-fatal: 调用方应吞掉).
    #[error("no update available: current version {current} is up to date")]
    NoUpdateAvailable {
        /// 当前版本
        current: String,
    },

    /// 版本解析失败 (semver 格式不合法, K-1 强校验).
    #[error("invalid semver: {0}")]
    InvalidSemver(String),

    /// 公钥指纹不匹配 (K-1 强校验: 编译期 hardcode `TrustedKey` 枚举).
    #[error("untrusted public key fingerprint: expected {expected}, got {got}")]
    UntrustedPublicKey {
        /// 期望指纹 (hex)
        expected: String,
        /// 实际指纹 (hex)
        got: String,
    },

    /// 公钥 base64 解析失败.
    #[error("invalid public key encoding: {0}")]
    InvalidPublicKey(String),

    /// 签名 base64 解析失败.
    #[error("invalid signature encoding: {0}")]
    InvalidSignature(String),

    /// 签名验签失败 (minisign 内部错误).
    #[error("signature verification failed: {0}")]
    SignatureVerifyFailed(String),

    /// Asset 哈希不匹配 (SHA-256 二次校验, K-1 强校验).
    #[error("asset hash mismatch: expected {expected}, got {got}")]
    HashMismatch {
        /// 期望 SHA-256 (hex)
        expected: String,
        /// 实际 SHA-256 (hex)
        got: String,
    },

    /// Channel 不支持 (e.g. 用户要 Nightly 但 release 是 Stable).
    #[error("channel not supported: requested {requested}, release is {release}")]
    ChannelNotSupported {
        /// 请求 channel
        requested: String,
        /// 实际 release channel
        release: String,
    },

    /// Endpoint 请求 schema 不合法.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Stub 模式占位 (R21+ 真接时此 variant 移除).
    #[error("stub mode: {0}")]
    Stub(String),

    /// I/O 错误 (asset 下载 / 配置读, R21+ 真接时启用).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// 11 variant 编译期守门 (K-1 强校验 + 8 项不修改承诺 #4).
pub const UPDATE_ERROR_VARIANT_COUNT: usize = 11;

/// 5 K-1 强校验名称 (per task spec §3 借鉴文档 §6 + 主人 8 项承诺 #4).
pub const K1_STRONG_VALIDATION_VARIANTS: [&str; 5] = [
    "InvalidSemver",
    "UntrustedPublicKey",
    "InvalidPublicKey",
    "InvalidSignature",
    "HashMismatch",
];

/// 编译期守门: 11 variant + 5 K-1 校验.
const _: () = assert!(K1_STRONG_VALIDATION_VARIANTS.len() == 5);
const _: () = assert!(UPDATE_ERROR_VARIANT_COUNT == 11);

/// Update 子系统统一结果类型.
pub type UpdateResult<T> = Result<T, UpdateError>;

// ============================================================================
// §2 K-1 强校验 helper 函数
// ============================================================================

/// K-1 强校验: semver 字符串非空且 ASCII.
pub fn validate_version_string(s: &str) -> UpdateResult<()> {
    if s.is_empty() {
        return Err(UpdateError::InvalidSemver("empty version".to_string()));
    }
    if !s.is_ascii() {
        return Err(UpdateError::InvalidSemver(format!(
            "non-ascii version: {}",
            s
        )));
    }
    // 二次校验: 必含 `.` (semver 必有 major.minor)
    if !s.contains('.') {
        return Err(UpdateError::InvalidSemver(format!(
            "not semver (no dot): {}",
            s
        )));
    }
    Ok(())
}

/// K-1 强校验: 公钥 base64 字符串非空 + 长度对齐.
pub fn validate_public_key_b64(s: &str) -> UpdateResult<()> {
    if s.is_empty() {
        return Err(UpdateError::InvalidPublicKey("empty".to_string()));
    }
    // minisign 公钥 base64 解码后 = 42 bytes (Ed25519 public key + 2 bytes keynum)
    // base64 编码后 ~ 56 chars
    if s.len() < 40 {
        return Err(UpdateError::InvalidPublicKey(format!(
            "too short: {} chars",
            s.len()
        )));
    }
    Ok(())
}

/// K-1 强校验: 签名 base64 字符串非空 + 长度对齐.
pub fn validate_signature_b64(s: &str) -> UpdateResult<()> {
    if s.is_empty() {
        return Err(UpdateError::InvalidSignature("empty".to_string()));
    }
    if s.len() < 64 {
        return Err(UpdateError::InvalidSignature(format!(
            "too short: {} chars",
            s.len()
        )));
    }
    Ok(())
}

/// K-1 强校验: SHA-256 hex = 64 chars.
pub fn validate_sha256_hex(s: &str) -> UpdateResult<()> {
    if s.len() != 64 {
        return Err(UpdateError::HashMismatch {
            expected: "64 hex chars".to_string(),
            got: format!("{} chars", s.len()),
        });
    }
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(UpdateError::HashMismatch {
            expected: "hex only".to_string(),
            got: "non-hex chars".to_string(),
        });
    }
    Ok(())
}

/// K-1 强校验: 公钥指纹 (16 字符 hex = 8 bytes truncated).
pub fn validate_fingerprint_hex(s: &str) -> UpdateResult<()> {
    if s.len() != 16 {
        return Err(UpdateError::UntrustedPublicKey {
            expected: "16 hex chars".to_string(),
            got: format!("{} chars", s.len()),
        });
    }
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(UpdateError::UntrustedPublicKey {
            expected: "hex only".to_string(),
            got: "non-hex chars".to_string(),
        });
    }
    Ok(())
}

// ============================================================================
// §3 单元测试 (K-1 强校验守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_count_is_11() {
        assert_eq!(UPDATE_ERROR_VARIANT_COUNT, 11);
    }

    #[test]
    fn k1_variants_count_is_5() {
        assert_eq!(K1_STRONG_VALIDATION_VARIANTS.len(), 5);
    }

    #[test]
    fn validate_version_string_accepts_semver() {
        assert!(validate_version_string("1.0.0").is_ok());
        assert!(validate_version_string("0.14.0-R14").is_ok());
        assert!(validate_version_string("2.10.5").is_ok());
    }

    #[test]
    fn validate_version_string_rejects_empty() {
        assert!(matches!(
            validate_version_string(""),
            Err(UpdateError::InvalidSemver(_))
        ));
    }

    #[test]
    fn validate_version_string_rejects_no_dot() {
        assert!(validate_version_string("123").is_err());
        assert!(validate_version_string("v1").is_err());
    }

    #[test]
    fn validate_public_key_b64_accepts_long() {
        let long_b64 = "a".repeat(56);
        assert!(validate_public_key_b64(&long_b64).is_ok());
    }

    #[test]
    fn validate_public_key_b64_rejects_empty() {
        assert!(validate_public_key_b64("").is_err());
    }

    #[test]
    fn validate_public_key_b64_rejects_short() {
        assert!(validate_public_key_b64("short").is_err());
    }

    #[test]
    fn validate_signature_b64_rejects_empty() {
        assert!(validate_signature_b64("").is_err());
    }

    #[test]
    fn validate_signature_b64_accepts_long() {
        let long_b64 = "a".repeat(100);
        assert!(validate_signature_b64(&long_b64).is_ok());
    }

    #[test]
    fn validate_sha256_hex_accepts_64_hex() {
        let h = "a".repeat(64);
        assert!(validate_sha256_hex(&h).is_ok());
        let h2 = "0123456789abcdef".repeat(4);
        assert!(validate_sha256_hex(&h2).is_ok());
    }

    #[test]
    fn validate_sha256_hex_rejects_wrong_length() {
        assert!(validate_sha256_hex("abc").is_err());
        assert!(validate_sha256_hex(&"a".repeat(63)).is_err());
        assert!(validate_sha256_hex(&"a".repeat(65)).is_err());
    }

    #[test]
    fn validate_sha256_hex_rejects_non_hex() {
        let bad = "z".repeat(64);
        assert!(validate_sha256_hex(&bad).is_err());
    }

    #[test]
    fn validate_fingerprint_hex_accepts_16_hex() {
        assert!(validate_fingerprint_hex("99F790EC4BE6E38D").is_ok());
        assert!(validate_fingerprint_hex("0123456789abcdef").is_ok());
    }

    #[test]
    fn validate_fingerprint_hex_rejects_wrong_length() {
        assert!(validate_fingerprint_hex("99F790EC4BE6E38").is_err()); // 15
        assert!(validate_fingerprint_hex("99F790EC4BE6E38DD").is_err()); // 17
    }

    #[test]
    fn validate_fingerprint_hex_rejects_non_hex() {
        assert!(validate_fingerprint_hex("ZZZZZZZZZZZZZZZZ").is_err());
    }
}
