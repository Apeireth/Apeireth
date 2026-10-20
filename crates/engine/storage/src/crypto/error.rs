//! Crypto-face errors (加密面错误帧).
//!
//! 错误消息**不带密钥材料/明文/口令** (脱敏红线); 解密失败不区分
//! 「密钥错」与「密文被篡改」(AEAD 单一拒解口径, 不给攻击者错误预言机)。

/// Errors produced by the storage crypto face.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// Sealed bytes failed to open: wrong key, partition AAD mismatch
    /// (跨分区挪用), or tampered ciphertext — refused as one case.
    #[error("sealed field refused to open (wrong key / cross-partition AAD / tampered ciphertext): {reason}")]
    OpenRefused {
        /// Context of the refusal (never key material).
        reason: &'static str,
    },

    /// Sealing failed (AEAD construction error; practically unreachable).
    #[error("sealed field encryption failed: {0}")]
    SealFailed(&'static str),

    /// Sealed field encoding is malformed (magic/length/hex).
    #[error("sealed field encoding error: {0}")]
    Encoding(String),

    /// A key wrap operation was refused.
    #[error("key wrap refused: {0}")]
    WrapRefused(String),

    /// Key unwrap refused: wrong passphrase / wrong recovery key /
    /// tampered wrap slot (one refusal, no oracle).
    #[error(
        "key unwrap refused (wrong passphrase / wrong recovery key / tampered wrap slot): {reason}"
    )]
    UnwrapRefused {
        /// Context of the refusal (never key material).
        reason: &'static str,
    },

    /// Key material is gone: no usable wrap slot and no unlock material.
    /// **不可恢复** — 无后门、无找回流程 (见 `KEY_LOSS_BOUNDARY`)。
    #[error("key material lost (unrecoverable by design): {0}")]
    KeyMaterialLost(String),

    /// KDF parameters are invalid (unknown kind / zero iterations / bad salt).
    #[error("invalid KDF parameters: {0}")]
    InvalidKdf(String),

    /// The sealed backup bundle format is broken (manifest/entry mismatch).
    #[error("sealed backup format error: {0}")]
    BackupFormat(String),

    /// A bundle or restore path failed the path-safety check.
    #[error("unsafe path refused: {0}")]
    PathSafety(String),

    /// Restore refused before touching the disk (宁拒不损).
    #[error("sealed restore refused (nothing written): {0}")]
    RestoreRefused(String),

    /// Filesystem I/O failure.
    #[error("crypto io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization failure for wrap files and manifests.
    #[error("crypto json error: {0}")]
    Json(#[from] serde_json::Error),
}
