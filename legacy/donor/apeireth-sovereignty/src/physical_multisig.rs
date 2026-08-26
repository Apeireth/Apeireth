//! 物理多签 — 抽象 trait + Rust mock
//!
//! **设计** (阶段 1 §18.6 + 阶段 2 §11):
//! - L5 核武器级操作需要物理多签 (YubiKey + 手机 + 密码管理器)
//! - 抽象 trait — 不依赖具体硬件 SDK (YubiKey / FIDO2)
//! - Rust mock: 用 `PhysicalSigner` trait + `InMemoryPhysicalMultisig` 实现
//!
//! **硬约束**: 不模拟硬件密钥; 测试用 mock 注册一组 signer 即可

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 物理签名设备标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhysicalSignerId {
    /// 设备 ID
    pub id: String,
    /// 设备类型 (yubikey / phone / password_manager / hardware_token)
    pub kind: String,
    /// 持有人 ID (与 HumanId 对齐)
    pub holder_id: String,
}

impl PhysicalSignerId {
    /// 新建设备 ID
    pub fn new(
        id: impl Into<String>,
        kind: impl Into<String>,
        holder_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            holder_id: holder_id.into(),
        }
    }
}

/// 物理签名
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalSignature {
    /// 签名设备
    pub signer: PhysicalSignerId,
    /// 签名摘要 (任意 string, hash 或 nonce)
    pub digest: String,
    /// 签名时间
    pub timestamp: i64,
    /// 是否"亲眼在场" (生物特征 / 物理按键确认)
    pub witness_present: bool,
}

/// 物理多签裁决
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MultisigOutcome {
    /// 通过 (≥2 不同 kind 签名 + ≥1 witness_present)
    Approved {
        signature_count: usize,
        witness_count: usize,
    },
    /// 拒绝 (签名数不足或无见证)
    Rejected {
        signature_count: usize,
        reason: String,
    },
    /// 待签名
    PendingSignatures { collected: usize, required: usize },
}

/// 物理多签错误
#[derive(Debug, Error)]
pub enum MultisigError {
    #[error("signer `{0}` not registered")]
    UnknownSigner(String),
    #[error("signer `{0}` already signed")]
    DuplicateSignature(String),
}

/// PhysicalSigner trait — 抽象物理签名
///
/// **不变量**:
/// - 每个物理设备 (YubiKey / 手机 / 密码管理器) 视为一个 signer
/// - 同一 signer 不可重复签名
/// - ≥2 不同 kind 签名 + ≥1 witness_present 才算 Approved
///
/// **dyn 兼容性**: `digest` 参数为 `String` 而非 `impl Into<String>`
pub trait PhysicalMultisig: Send + Sync {
    /// 注册物理设备
    fn register(&mut self, signer: PhysicalSignerId);

    /// 收集签名 (digest = 决策摘要 hash)
    fn collect_signature(
        &mut self,
        signer_id: &str,
        digest: String,
        witness_present: bool,
    ) -> Result<PhysicalSignature, MultisigError>;

    /// 聚合签名结果
    fn tally(&self) -> MultisigOutcome;

    /// 已注册的设备数
    fn registered_count(&self) -> usize;

    /// 已收集签名数
    fn signature_count(&self) -> usize;
}

/// 内存 mock 实现
#[derive(Debug, Default)]
pub struct InMemoryPhysicalMultisig {
    signers: Vec<PhysicalSignerId>,
    signatures: Vec<PhysicalSignature>,
    /// 最低签名数 (默认 2)
    pub required_signatures: usize,
}

impl InMemoryPhysicalMultisig {
    /// 新建空 multisig (required_signatures = 2)
    pub fn new() -> Self {
        Self {
            signers: Vec::new(),
            signatures: Vec::new(),
            required_signatures: 2,
        }
    }

    /// 自定义最低签名数
    pub fn with_required(mut self, n: usize) -> Self {
        self.required_signatures = n.max(1);
        self
    }
}

impl PhysicalMultisig for InMemoryPhysicalMultisig {
    fn register(&mut self, signer: PhysicalSignerId) {
        if !self.signers.iter().any(|s| s.id == signer.id) {
            self.signers.push(signer);
        }
    }

    fn collect_signature(
        &mut self,
        signer_id: &str,
        digest: String,
        witness_present: bool,
    ) -> Result<PhysicalSignature, MultisigError> {
        let signer = self
            .signers
            .iter()
            .find(|s| s.id == signer_id)
            .cloned()
            .ok_or_else(|| MultisigError::UnknownSigner(signer_id.into()))?;
        if self.signatures.iter().any(|s| s.signer.id == signer_id) {
            return Err(MultisigError::DuplicateSignature(signer_id.into()));
        }
        let sig = PhysicalSignature {
            signer,
            digest,
            timestamp: chrono::Utc::now().timestamp(),
            witness_present,
        };
        self.signatures.push(sig.clone());
        Ok(sig)
    }

    fn tally(&self) -> MultisigOutcome {
        let collected = self.signatures.len();
        if collected < self.required_signatures {
            return MultisigOutcome::PendingSignatures {
                collected,
                required: self.required_signatures,
            };
        }
        // 检查 ≥1 witness_present
        let witness_count = self.signatures.iter().filter(|s| s.witness_present).count();
        if witness_count == 0 {
            return MultisigOutcome::Rejected {
                signature_count: collected,
                reason: "无任何 witness_present 签名 (无人在场)".into(),
            };
        }
        // 检查 ≥2 不同 kind
        let mut kinds = std::collections::HashSet::new();
        for s in &self.signatures {
            kinds.insert(s.signer.kind.clone());
        }
        if kinds.len() < 2 {
            return MultisigOutcome::Rejected {
                signature_count: collected,
                reason: "签名设备 kind 不足 2 种 (单点故障风险)".into(),
            };
        }
        MultisigOutcome::Approved {
            signature_count: collected,
            witness_count,
        }
    }

    fn registered_count(&self) -> usize {
        self.signers.len()
    }

    fn signature_count(&self) -> usize {
        self.signatures.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yubikey() -> PhysicalSignerId {
        PhysicalSignerId::new("yubi-001", "yubikey", "alice")
    }
    fn phone() -> PhysicalSignerId {
        PhysicalSignerId::new("phone-001", "phone", "bob")
    }
    fn password_manager() -> PhysicalSignerId {
        PhysicalSignerId::new("pm-001", "password_manager", "carol")
    }

    #[test]
    fn physical_multisig_approved_with_two_distinct_kinds_and_witness() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(yubikey());
        m.register(phone());
        m.register(password_manager());
        m.collect_signature("yubi-001", "digest-abc".to_string(), true)
            .unwrap();
        m.collect_signature("phone-001", "digest-abc".to_string(), false)
            .unwrap();
        match m.tally() {
            MultisigOutcome::Approved {
                signature_count,
                witness_count,
            } => {
                assert_eq!(signature_count, 2);
                assert_eq!(witness_count, 1);
            }
            _ => panic!("应 Approved"),
        }
    }

    #[test]
    fn physical_multisig_pending_with_one_signature() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(yubikey());
        m.register(phone());
        m.collect_signature("yubi-001", "digest".to_string(), true)
            .unwrap();
        assert!(matches!(
            m.tally(),
            MultisigOutcome::PendingSignatures {
                collected: 1,
                required: 2
            }
        ));
    }

    #[test]
    fn physical_multisig_rejected_without_witness() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(yubikey());
        m.register(phone());
        m.collect_signature("yubi-001", "d".to_string(), false)
            .unwrap();
        m.collect_signature("phone-001", "d".to_string(), false)
            .unwrap();
        assert!(matches!(m.tally(), MultisigOutcome::Rejected { .. }));
    }

    #[test]
    fn physical_multisig_rejects_same_kind_only() {
        let mut m = InMemoryPhysicalMultisig::new();
        // 注册 2 个 yubikey (同 kind)
        m.register(PhysicalSignerId::new("y1", "yubikey", "alice"));
        m.register(PhysicalSignerId::new("y2", "yubikey", "alice"));
        m.collect_signature("y1", "d".to_string(), true).unwrap();
        m.collect_signature("y2", "d".to_string(), true).unwrap();
        assert!(matches!(m.tally(), MultisigOutcome::Rejected { .. }));
    }

    #[test]
    fn physical_multisig_rejects_duplicate_signature() {
        let mut m = InMemoryPhysicalMultisig::new();
        m.register(yubikey());
        m.collect_signature("yubi-001", "d".to_string(), true)
            .unwrap();
        assert!(matches!(
            m.collect_signature("yubi-001", "d".to_string(), true),
            Err(MultisigError::DuplicateSignature(_))
        ));
    }
}
