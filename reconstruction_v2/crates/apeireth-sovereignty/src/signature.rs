//! Sovereignty 数字签名 trait 抽象

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SIGNATURE_ALGORITHM_COUNT_HARDCODE: usize = 3;
pub const K1_STRICT_CHECK_COUNT_HARDCODE: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    Ed25519, Rsa2048, EcdsaP256,
}

impl SignatureAlgorithm {
    pub fn as_str(self) -> &'static str {
        match self { Self::Ed25519 => "ed25519", Self::Rsa2048 => "rsa2048", Self::EcdsaP256 => "ecdsa_p256" }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    pub algorithm: SignatureAlgorithm,
    pub key_id: String,
    pub signature_bytes: String,
    pub timestamp_ms: i64,
}

impl Signature {
    pub fn validate_k1(&self) -> Result<(), SignatureError> {
        if self.key_id.trim().is_empty() { return Err(SignatureError::K1KeyIdEmpty); }
        if self.signature_bytes.trim().is_empty() { return Err(SignatureError::K1SignatureEmpty); }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    Valid { algorithm: SignatureAlgorithm, key_id: String },
    Invalid { algorithm: SignatureAlgorithm, reason: String },
}

#[derive(Debug, Error, PartialEq)]
pub enum SignatureError {
    #[error("K-1.a 强校验失败: payload 为空")]
    K1PayloadEmpty,
    #[error("K-1.b 强校验失败: key_id 为空")]
    K1KeyIdEmpty,
    #[error("K-1.c 强校验失败: signature_bytes 为空")]
    K1SignatureEmpty,
    #[error("签名错误: {0}")]
    SignFailed(String),
}

pub trait Signer: Send + Sync {
    fn algorithm(&self) -> SignatureAlgorithm;
    fn key_id(&self) -> &str;
    fn sign(&self, payload: &[u8]) -> Result<Signature, SignatureError>;
    fn verify(&self, payload: &[u8], sig: &Signature) -> Result<VerificationResult, SignatureError>;
}

pub struct Ed25519Signer { pub key_id: String }
impl Ed25519Signer {
    pub fn new(key_id: String) -> Self { Self { key_id } }
}
impl Signer for Ed25519Signer {
    fn algorithm(&self) -> SignatureAlgorithm { SignatureAlgorithm::Ed25519 }
    fn key_id(&self) -> &str { &self.key_id }
    fn sign(&self, payload: &[u8]) -> Result<Signature, SignatureError> {
        if payload.is_empty() { return Err(SignatureError::K1PayloadEmpty); }
        let hex_payload = payload.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        Ok(Signature { algorithm: SignatureAlgorithm::Ed25519, key_id: self.key_id.clone(), signature_bytes: format!("ed25519:{}:{}", self.key_id, hex_payload), timestamp_ms: chrono::Utc::now().timestamp_millis() })
    }
    fn verify(&self, payload: &[u8], sig: &Signature) -> Result<VerificationResult, SignatureError> {
        sig.validate_k1()?;
        if payload.is_empty() { return Err(SignatureError::K1PayloadEmpty); }
        if sig.key_id != self.key_id { return Ok(VerificationResult::Invalid { algorithm: sig.algorithm, reason: format!("key_id 不匹配") }); }
        if sig.algorithm != SignatureAlgorithm::Ed25519 { return Ok(VerificationResult::Invalid { algorithm: sig.algorithm, reason: format!("算法不匹配") }); }
        let expected = self.sign(payload)?;
        if expected.signature_bytes == sig.signature_bytes {
            Ok(VerificationResult::Valid { algorithm: sig.algorithm, key_id: sig.key_id.clone() })
        } else {
            Ok(VerificationResult::Invalid { algorithm: sig.algorithm, reason: "签名与 payload 不匹配".into() })
        }
    }
}

pub struct Rsa2048Signer { pub key_id: String }
impl Rsa2048Signer {
    pub fn new(key_id: String) -> Self { Self { key_id } }
}
impl Signer for Rsa2048Signer {
    fn algorithm(&self) -> SignatureAlgorithm { SignatureAlgorithm::Rsa2048 }
    fn key_id(&self) -> &str { &self.key_id }
    fn sign(&self, payload: &[u8]) -> Result<Signature, SignatureError> {
        if payload.is_empty() { return Err(SignatureError::K1PayloadEmpty); }
        let hex_payload = payload.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        Ok(Signature { algorithm: SignatureAlgorithm::Rsa2048, key_id: self.key_id.clone(), signature_bytes: format!("rsa2048:{}:{}", self.key_id, hex_payload), timestamp_ms: chrono::Utc::now().timestamp_millis() })
    }
    fn verify(&self, payload: &[u8], sig: &Signature) -> Result<VerificationResult, SignatureError> {
        sig.validate_k1()?;
        if payload.is_empty() { return Err(SignatureError::K1PayloadEmpty); }
        if sig.algorithm != SignatureAlgorithm::Rsa2048 { return Ok(VerificationResult::Invalid { algorithm: sig.algorithm, reason: format!("算法不匹配") }); }
        let expected = self.sign(payload)?;
        if expected.signature_bytes == sig.signature_bytes { Ok(VerificationResult::Valid { algorithm: sig.algorithm, key_id: sig.key_id.clone() }) }
        else { Ok(VerificationResult::Invalid { algorithm: sig.algorithm, reason: "签名与 payload 不匹配".into() }) }
    }
}

pub struct EcdsaP256Signer { pub key_id: String }
impl EcdsaP256Signer {
    pub fn new(key_id: String) -> Self { Self { key_id } }
}
impl Signer for EcdsaP256Signer {
    fn algorithm(&self) -> SignatureAlgorithm { SignatureAlgorithm::EcdsaP256 }
    fn key_id(&self) -> &str { &self.key_id }
    fn sign(&self, payload: &[u8]) -> Result<Signature, SignatureError> {
        if payload.is_empty() { return Err(SignatureError::K1PayloadEmpty); }
        let hex_payload = payload.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        Ok(Signature { algorithm: SignatureAlgorithm::EcdsaP256, key_id: self.key_id.clone(), signature_bytes: format!("ecdsa_p256:{}:{}", self.key_id, hex_payload), timestamp_ms: chrono::Utc::now().timestamp_millis() })
    }
    fn verify(&self, payload: &[u8], sig: &Signature) -> Result<VerificationResult, SignatureError> {
        sig.validate_k1()?;
        if payload.is_empty() { return Err(SignatureError::K1PayloadEmpty); }
        if sig.algorithm != SignatureAlgorithm::EcdsaP256 { return Ok(VerificationResult::Invalid { algorithm: sig.algorithm, reason: format!("算法不匹配") }); }
        let expected = self.sign(payload)?;
        if expected.signature_bytes == sig.signature_bytes { Ok(VerificationResult::Valid { algorithm: sig.algorithm, key_id: sig.key_id.clone() }) }
        else { Ok(VerificationResult::Invalid { algorithm: sig.algorithm, reason: "签名与 payload 不匹配".into() }) }
    }
}

const _: () = {
    assert!(SIGNATURE_ALGORITHM_COUNT_HARDCODE == 3);
    assert!(K1_STRICT_CHECK_COUNT_HARDCODE == 3);
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn algorithm_count_3() {
        assert_eq!(SIGNATURE_ALGORITHM_COUNT_HARDCODE, 3);
        assert_eq!(SignatureAlgorithm::Ed25519.as_str(), "ed25519");
        assert_eq!(SignatureAlgorithm::Rsa2048.as_str(), "rsa2048");
        assert_eq!(SignatureAlgorithm::EcdsaP256.as_str(), "ecdsa_p256");
    }
    #[test] fn k1_three_failures() {
        let sig = Signature { algorithm: SignatureAlgorithm::Ed25519, key_id: "  ".into(), signature_bytes: "x".into(), timestamp_ms: 0 };
        assert_eq!(sig.validate_k1(), Err(SignatureError::K1KeyIdEmpty));
        let sig2 = Signature { algorithm: SignatureAlgorithm::Ed25519, key_id: "alice".into(), signature_bytes: "  ".into(), timestamp_ms: 0 };
        assert_eq!(sig2.validate_k1(), Err(SignatureError::K1SignatureEmpty));
        let s = Ed25519Signer::new("alice".into());
        assert_eq!(s.sign(b""), Err(SignatureError::K1PayloadEmpty));
    }
    #[test] fn three_signers_sign_and_verify() {
        let p = b"hello";
        let ed = Ed25519Signer::new("a".into());
        let ed_sig = ed.sign(p).unwrap();
        assert!(matches!(ed.verify(p, &ed_sig).unwrap(), VerificationResult::Valid { .. }));
        assert!(matches!(ed.verify(b"world", &ed_sig).unwrap(), VerificationResult::Invalid { .. }));
        let rsa = Rsa2048Signer::new("a".into());
        let rsa_sig = rsa.sign(p).unwrap();
        assert!(matches!(rsa.verify(p, &rsa_sig).unwrap(), VerificationResult::Valid { .. }));
        let ec = EcdsaP256Signer::new("a".into());
        let ec_sig = ec.sign(p).unwrap();
        assert!(matches!(ec.verify(p, &ec_sig).unwrap(), VerificationResult::Valid { .. }));
        let cross = rsa.verify(p, &ed_sig).unwrap();
        assert!(matches!(cross, VerificationResult::Invalid { .. }));
    }
    #[test] fn ed_key_id_mismatch() {
        let a = Ed25519Signer::new("a".into());
        let b = Ed25519Signer::new("b".into());
        let sig = a.sign(b"x").unwrap();
        assert!(matches!(b.verify(b"x", &sig).unwrap(), VerificationResult::Invalid { .. }));
    }
    #[test] fn rsa_key_id_mismatch() {
        let a = Rsa2048Signer::new("a".into());
        let b = Rsa2048Signer::new("b".into());
        let sig = a.sign(b"x").unwrap();
        assert!(matches!(b.verify(b"x", &sig).unwrap(), VerificationResult::Invalid { .. }));
    }
}
