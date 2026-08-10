//! Sovereignty 数字签名 trait 抽象 (R20 阶段 6 估补)
//!
//! **职责** (本模块 flesh out 估补, lib.rs LOCKED 不重 export):
//! - **3 算法 trait 抽象** (纯 trait + mock impl, **不引 ed25519-dalek / rsa / p256 等新依赖**):
//!   1. `Ed25519` (EdDSA over Curve25519)
//!   2. `Rsa2048` (RSA-2048, RSASSA-PKCS1-v1_5)
//!   3. `EcdsaP256` (ECDSA over NIST P-256 / secp256r1)
//! - **3 K-1 强校验** (任何 sign/verify 必须满足, 否则 `Err(SignatureError::K1Violation)`):
//!   1. **K-1.a** — `payload` 非空
//!   2. **K-1.b** — `key_id` 非空 (签名者身份)
//!   3. **K-1.c** — `signature` 非空 (签名结果非空)
//!
//! **6 哲学锚穿透**:
//! - **主 22:33 ASI 北极星** — 数字签名让治理动作可验真, 服务"事后可还原"
//! - **主 17:43 实事求是** — 3 算法是真实业界标准映射, 非装饰
//! - **主 17:58 不假装** — Mock impl 明确标 "mock", 不假装是密码学实现
//! - **主 19:33 走在前人肩上** — **不引 ed25519-dalek / rsa / p256**, 因 sovereignty 不需要真密码学;
//!   真生产应在 `apeireth-crypto` (未来 crate) 实装, 本模块只抽象 trait
//! - **主 23:44 干到底** — 3 K-1 强校验在 `verify` 一处集中执行
//! - **主 00:56 任何人都能接手** — 3 算法枚举化, mock impl 简单直白
//!
//! **8 项不修改承诺**:
//! - ✅ 编译期 hardcode: 算法数 = 3, K-1 强校验数 = 3
//! - ✅ 0 触碰 LOCKED
//! - ✅ 0 依赖 NewAPI
//! - ✅ **0 重复造轮子**: ❌ **不**引 `ed25519-dalek` / `rsa` / `p256` 等真密码学 crate
//!   (R17 决策: 复用现有依赖, 不引新 crypto 库); 真生产应建 `apeireth-crypto` 隔离
//! - ✅ 诚实标缺: ❌ **不**是真密码学实现, 是 mock; 标 "mock" + "TEST ONLY"
//!
//! **诚实登记**:
//! - ❌ **不**是真密码学实现 — mock impl 用 `format!` 拼字符串, **不可用于生产**
//! - ❌ **不**抗碰撞 / 抗篡改 — 真生产应在独立 `apeireth-crypto` crate 中实装
//! - ❌ **不**做密钥管理 — `key_id` 只是字符串 ID, 不持有真实密钥
//!
//! **用法**:
//! ```ignore
//! use signature::{Ed25519Signer, SignatureAlgorithm, Signer, VerificationResult};
//!
//! let signer = Ed25519Signer::new("alice-key-1".into());
//! let sig = signer.sign(b"hello world")?;
//! let result = signer.verify(b"hello world", &sig)?;
//! assert!(matches!(result, VerificationResult::Valid { .. }));
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================
// 编译时 hardcode: 3 算法 / 3 K-1 强校验
// ============================================================

/// 算法数 (编译时硬编码: Ed25519 / Rsa2048 / EcdsaP256 = 3)
pub const SIGNATURE_ALGORITHM_COUNT_HARDCODE: usize = 3;

/// K-1 强校验数 (编译时硬编码: payload 非空 / key_id 非空 / signature 非空 = 3)
pub const K1_STRICT_CHECK_COUNT_HARDCODE: usize = 3;

/// 3 签名算法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    /// EdDSA over Curve25519 (32 字节公钥, 64 字节签名) — 业界主流
    Ed25519,
    /// RSA-2048 + RSASSA-PKCS1-v1_5 (256 字节签名) — 传统兼容
    Rsa2048,
    /// ECDSA over NIST P-256 / secp256r1 (64 字节签名) — 政府和银行常用
    EcdsaP256,
}

impl SignatureAlgorithm {
    /// 字符串 ID
    pub fn as_str(self) -> &'static str {
        match self {
            SignatureAlgorithm::Ed25519 => "ed25519",
            SignatureAlgorithm::Rsa2048 => "rsa2048",
            SignatureAlgorithm::EcdsaP256 => "ecdsa_p256",
        }
    }
}

/// 签名结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// 算法
    pub algorithm: SignatureAlgorithm,
    /// 签名者 key ID — K-1.b
    pub key_id: String,
    /// 签名 base64 (mock impl 用 hex 字符串) — K-1.c
    pub signature_bytes: String,
    /// 时间戳 (epoch ms)
    pub timestamp_ms: i64,
}

impl Signature {
    /// 3 K-1 强校验
    ///
    /// - **K-1.a**: payload 引用由调用方校验 (此函数无 payload 字段)
    /// - **K-1.b**: key_id 非空
    /// - **K-1.c**: signature_bytes 非空
    pub fn validate_k1(&self) -> Result<(), SignatureError> {
        if self.key_id.trim().is_empty() {
            return Err(SignatureError::K1KeyIdEmpty);
        }
        if self.signature_bytes.trim().is_empty() {
            return Err(SignatureError::K1SignatureEmpty);
        }
        Ok(())
    }
}

/// 验签结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    /// 验签通过
    Valid {
        /// 算法
        algorithm: SignatureAlgorithm,
        /// key_id
        key_id: String,
    },
    /// 验签失败 (签名不匹配 payload 或 key_id 错误)
    Invalid {
        /// 算法
        algorithm: SignatureAlgorithm,
        /// 失败原因
        reason: String,
    },
}

/// 签名错误
#[derive(Debug, Error, PartialEq)]
pub enum SignatureError {
    /// K-1.a 强校验失败 — payload 非空
    #[error("K-1.a 强校验失败: payload 为空")]
    K1PayloadEmpty,
    /// K-1.b 强校验失败 — key_id 非空
    #[error("K-1.b 强校验失败: key_id 为空 (签名必须记录签名者)")]
    K1KeyIdEmpty,
    /// K-1.c 强校验失败 — signature 非空
    #[error("K-1.c 强校验失败: signature_bytes 为空 (签名结果不能为空)")]
    K1SignatureEmpty,
    /// Mock impl 错误 (不模拟, 直接 fail)
    #[error("签名错误: {0}")]
    SignFailed(String),
}

/// 签名者 trait — 抽象签名 + 验签
pub trait Signer: Send + Sync {
    /// 算法
    fn algorithm(&self) -> SignatureAlgorithm;
    /// key_id
    fn key_id(&self) -> &str;
    /// 签 payload → Signature (K-1 强校验在内部)
    fn sign(&self, payload: &[u8]) -> Result<Signature, SignatureError>;
    /// 验签 payload + signature → VerificationResult (K-1 强校验在内部)
    fn verify(&self, payload: &[u8], sig: &Signature) -> Result<VerificationResult, SignatureError>;
}

// ============================================================
// 3 Mock 实现 — TEST ONLY, 不可用于生产
//
// 算法:
//   - Ed25519: sig = "ed25519:" + key_id + ":" + hex(payload)
//   - Rsa2048: sig = "rsa2048:" + key_id + ":" + hex(payload)
//   - EcdsaP256: sig = "ecdsa_p256:" + key_id + ":" + hex(payload)
//
// 验签: 重新计算 sig 字符串, 与 sig.signature_bytes 比较; 一致 → Valid, 否则 Invalid
// ============================================================

/// Mock Ed25519 签名者 (TEST ONLY)
#[derive(Debug, Clone)]
pub struct Ed25519Signer {
    key_id: String,
}

impl Ed25519Signer {
    /// 构造 Ed25519 mock signer
    pub fn new(key_id: String) -> Self {
        Self { key_id }
    }
}

impl Signer for Ed25519Signer {
    fn algorithm(&self) -> SignatureAlgorithm {
        SignatureAlgorithm::Ed25519
    }

    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn sign(&self, payload: &[u8]) -> Result<Signature, SignatureError> {
        if payload.is_empty() {
            return Err(SignatureError::K1PayloadEmpty);
        }
        let hex_payload = payload
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        Ok(Signature {
            algorithm: SignatureAlgorithm::Ed25519,
            key_id: self.key_id.clone(),
            signature_bytes: format!("ed25519:{}:{}", self.key_id, hex_payload),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        })
    }

    fn verify(&self, payload: &[u8], sig: &Signature) -> Result<VerificationResult, SignatureError> {
        sig.validate_k1()?;
        if payload.is_empty() {
            return Err(SignatureError::K1PayloadEmpty);
        }
        if sig.key_id != self.key_id {
            return Ok(VerificationResult::Invalid {
                algorithm: sig.algorithm,
                reason: format!("key_id 不匹配: 期望 {}, 实际 {}", self.key_id, sig.key_id),
            });
        }
        if sig.algorithm != SignatureAlgorithm::Ed25519 {
            return Ok(VerificationResult::Invalid {
                algorithm: sig.algorithm,
                reason: format!(
                    "算法不匹配: 期望 Ed25519, 实际 {:?}",
                    sig.algorithm
                ),
            });
        }
        let expected = self.sign(payload)?;
        if expected.signature_bytes == sig.signature_bytes {
            Ok(VerificationResult::Valid {
                algorithm: sig.algorithm,
                key_id: sig.key_id.clone(),
            })
        } else {
            Ok(VerificationResult::Invalid {
                algorithm: sig.algorithm,
                reason: "签名与 payload 不匹配".into(),
            })
        }
    }
}

/// Mock RSA-2048 签名者 (TEST ONLY)
#[derive(Debug, Clone)]
pub struct Rsa2048Signer {
    key_id: String,
}

impl Rsa2048Signer {
    /// 构造 RSA-2048 mock signer
    pub fn new(key_id: String) -> Self {
        Self { key_id }
    }
}

impl Signer for Rsa2048Signer {
    fn algorithm(&self) -> SignatureAlgorithm {
        SignatureAlgorithm::Rsa2048
    }

    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn sign(&self, payload: &[u8]) -> Result<Signature, SignatureError> {
        if payload.is_empty() {
            return Err(SignatureError::K1PayloadEmpty);
        }
        let hex_payload = payload
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        Ok(Signature {
            algorithm: SignatureAlgorithm::Rsa2048,
            key_id: self.key_id.clone(),
            signature_bytes: format!("rsa2048:{}:{}", self.key_id, hex_payload),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        })
    }

    fn verify(&self, payload: &[u8], sig: &Signature) -> Result<VerificationResult, SignatureError> {
        sig.validate_k1()?;
        if payload.is_empty() {
            return Err(SignatureError::K1PayloadEmpty);
        }
        if sig.algorithm != SignatureAlgorithm::Rsa2048 {
            return Ok(VerificationResult::Invalid {
                algorithm: sig.algorithm,
                reason: format!(
                    "算法不匹配: 期望 Rsa2048, 实际 {:?}",
                    sig.algorithm
                ),
            });
        }
        let expected = self.sign(payload)?;
        if expected.signature_bytes == sig.signature_bytes {
            Ok(VerificationResult::Valid {
                algorithm: sig.algorithm,
                key_id: sig.key_id.clone(),
            })
        } else {
            Ok(VerificationResult::Invalid {
                algorithm: sig.algorithm,
                reason: "签名与 payload 不匹配".into(),
            })
        }
    }
}

/// Mock ECDSA-P256 签名者 (TEST ONLY)
#[derive(Debug, Clone)]
pub struct EcdsaP256Signer {
    key_id: String,
}

impl EcdsaP256Signer {
    /// 构造 ECDSA-P256 mock signer
    pub fn new(key_id: String) -> Self {
        Self { key_id }
    }
}

impl Signer for EcdsaP256Signer {
    fn algorithm(&self) -> SignatureAlgorithm {
        SignatureAlgorithm::EcdsaP256
    }

    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn sign(&self, payload: &[u8]) -> Result<Signature, SignatureError> {
        if payload.is_empty() {
            return Err(SignatureError::K1PayloadEmpty);
        }
        let hex_payload = payload
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        Ok(Signature {
            algorithm: SignatureAlgorithm::EcdsaP256,
            key_id: self.key_id.clone(),
            signature_bytes: format!("ecdsa_p256:{}:{}", self.key_id, hex_payload),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        })
    }

    fn verify(&self, payload: &[u8], sig: &Signature) -> Result<VerificationResult, SignatureError> {
        sig.validate_k1()?;
        if payload.is_empty() {
            return Err(SignatureError::K1PayloadEmpty);
        }
        if sig.algorithm != SignatureAlgorithm::EcdsaP256 {
            return Ok(VerificationResult::Invalid {
                algorithm: sig.algorithm,
                reason: format!(
                    "算法不匹配: 期望 EcdsaP256, 实际 {:?}",
                    sig.algorithm
                ),
            });
        }
        let expected = self.sign(payload)?;
        if expected.signature_bytes == sig.signature_bytes {
            Ok(VerificationResult::Valid {
                algorithm: sig.algorithm,
                key_id: sig.key_id.clone(),
            })
        } else {
            Ok(VerificationResult::Invalid {
                algorithm: sig.algorithm,
                reason: "签名与 payload 不匹配".into(),
            })
        }
    }
}

const _: () = {
    assert!(SIGNATURE_ALGORITHM_COUNT_HARDCODE == 3);
    assert!(K1_STRICT_CHECK_COUNT_HARDCODE == 3);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_algorithm_count_is_3() {
        assert_eq!(SIGNATURE_ALGORITHM_COUNT_HARDCODE, 3);
        assert_eq!(SignatureAlgorithm::Ed25519.as_str(), "ed25519");
        assert_eq!(SignatureAlgorithm::Rsa2048.as_str(), "rsa2048");
        assert_eq!(SignatureAlgorithm::EcdsaP256.as_str(), "ecdsa_p256");
    }

    #[test]
    fn k1_strict_checks_three_failures() {
        let sig = Signature {
            algorithm: SignatureAlgorithm::Ed25519,
            key_id: "  ".into(),
            signature_bytes: "x".into(),
            timestamp_ms: 0,
        };
        assert_eq!(sig.validate_k1(), Err(SignatureError::K1KeyIdEmpty));

        let sig2 = Signature {
            algorithm: SignatureAlgorithm::Ed25519,
            key_id: "alice".into(),
            signature_bytes: "  ".into(),
            timestamp_ms: 0,
        };
        assert_eq!(sig2.validate_k1(), Err(SignatureError::K1SignatureEmpty));

        // K-1.a — sign/verify 内部
        let signer = Ed25519Signer::new("alice".into());
        assert_eq!(
            signer.sign(b""),
            Err(SignatureError::K1PayloadEmpty)
        );
    }

    #[test]
    fn three_mock_signers_sign_and_verify() {
        let payload = b"hello world - apeireth sovereignty";

        // Ed25519
        let ed = Ed25519Signer::new("alice-ed".into());
        let ed_sig = ed.sign(payload).unwrap();
        let ed_verify = ed.verify(payload, &ed_sig).unwrap();
        assert!(matches!(ed_verify, VerificationResult::Valid { .. }));

        // 修改 payload → 应 Invalid
        let bad_verify = ed.verify(b"hello world - tampered", &ed_sig).unwrap();
        assert!(matches!(bad_verify, VerificationResult::Invalid { .. }));

        // Rsa2048
        let rsa = Rsa2048Signer::new("alice-rsa".into());
        let rsa_sig = rsa.sign(payload).unwrap();
        let rsa_verify = rsa.verify(payload, &rsa_sig).unwrap();
        assert!(matches!(rsa_verify, VerificationResult::Valid { .. }));

        // EcdsaP256
        let ec = EcdsaP256Signer::new("alice-ec".into());
        let ec_sig = ec.sign(payload).unwrap();
        let ec_verify = ec.verify(payload, &ec_sig).unwrap();
        assert!(matches!(ec_verify, VerificationResult::Valid { .. }));

        // 跨算法验签应 Invalid (Ed25519 签名送 Rsa2048 验签)
        let cross = rsa.verify(payload, &ed_sig).unwrap();
        assert!(matches!(cross, VerificationResult::Invalid { .. }));
    }
}
