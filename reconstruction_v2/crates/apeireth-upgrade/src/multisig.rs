//! MultiSig — 物理多签收集 (OTA 阶段 3/7).
//!
//! 升级 Intent 通过智囊团审议后, 必须收集物理多签 (m-of-n) 才能进入下载+切换阶段.
//! 物理多签 = 每个签名人持有独立签名密钥 (HSM/TPM/冷钱包), 签名必须包含:
//! - payload_hash (intent 的不可变哈希)
//! - signed_at (签名时间戳)
//! - signer_id (签名人标识)
//! - signature (签名值, base64 或 hex)
//! - witness (见证, 例如其他签名的子集或外部凭证)
//!
//! 多签不变量:
//! 1. m-of-n 阈值校验 (默认 5-of-7)
//! 2. 签名人唯一性 (去重)
//! 3. payload_hash 一致性 (所有签名必须针对同一 payload)
//! 4. 截止时间校验 (deadline 内未达 quorum = Timeout)
//!
//! **禁止**: 不修改 apeireth-core 任何已实装类型签名.

use serde::{Deserialize, Serialize};

/// 单个物理签名.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalSignature {
    /// 签名人 ID (e.g., "carrier-a", "carrier-b", "owner-cold-wallet").
    pub signer_id: String,
    /// 签名的 payload 哈希 (intent 的不可变快照).
    pub payload_hash: String,
    /// 签名时间戳 (Unix seconds).
    pub signed_at: i64,
    /// 签名值 (base64 / hex, 真实实现应使用 ed25519/HMAC).
    pub signature: String,
    /// 见证 (可选, e.g., witness signer_id 或硬件证明 hash).
    pub witness: Option<String>,
}

impl PhysicalSignature {
    /// 构造签名.
    pub fn new(
        signer_id: impl Into<String>,
        payload_hash: impl Into<String>,
        signed_at: i64,
        signature: impl Into<String>,
    ) -> Self {
        Self {
            signer_id: signer_id.into(),
            payload_hash: payload_hash.into(),
            signed_at,
            signature: signature.into(),
            witness: None,
        }
    }

    /// 设置 witness.
    pub fn with_witness(mut self, witness: impl Into<String>) -> Self {
        self.witness = Some(witness.into());
        self
    }

    /// 签名是否合法 (非空 + payload_hash 非空 + signature 非空).
    pub fn is_valid(&self) -> bool {
        !self.signer_id.trim().is_empty()
            && !self.payload_hash.trim().is_empty()
            && !self.signature.trim().is_empty()
    }
}

/// 多签配置.
#[derive(Debug, Clone)]
pub struct MultiSigConfig {
    /// 阈值 m (达到 m 个签名即 quorum).
    pub threshold: usize,
    /// 候选签名人集合 (n, 用于去重校验).
    pub eligible_signers: Vec<String>,
    /// 截止时间戳 (Unix seconds); 0 表示无截止.
    pub deadline: i64,
}

impl MultiSigConfig {
    /// 默认 5-of-7 阈值.
    pub fn five_of_seven() -> Self {
        Self {
            threshold: 5,
            eligible_signers: (0..7).map(|i| format!("signer-{i}")).collect(),
            deadline: 0,
        }
    }

    /// 构造自定义配置.
    pub fn new(threshold: usize, eligible_signers: Vec<String>) -> Self {
        Self {
            threshold,
            eligible_signers,
            deadline: 0,
        }
    }

    /// 设置截止时间.
    pub fn with_deadline(mut self, deadline: i64) -> Self {
        self.deadline = deadline;
        self
    }

    /// 阈值 m 是否合法 (m <= n, m >= 1).
    pub fn is_valid(&self) -> bool {
        self.threshold >= 1
            && self.threshold <= self.eligible_signers.len()
            && !self.eligible_signers.is_empty()
    }
}

/// 多签收集结果.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiSigOutcome {
    /// 等待中 (尚未达到 quorum).
    Pending {
        /// 已收集的有效签名数.
        collected: usize,
        /// 仍需的签名数.
        needed: usize,
    },
    /// 已达 quorum (>= m).
    Quorum {
        /// 实际收集的有效签名数.
        count: usize,
        /// 达到 quorum 的时间戳.
        reached_at: i64,
    },
    /// 截止时间到达但未达 quorum.
    Timeout {
        /// 已收集的有效签名数.
        collected: usize,
        /// 仍需的签名数.
        needed: usize,
    },
    /// 收集过程出现非法签名 (payload_hash 不一致, 或签名人不合格).
    Invalid {
        /// 原因.
        reason: String,
    },
}

impl MultiSigOutcome {
    /// 是否可进入下一阶段 (Download).
    pub fn allows_proceed(&self) -> bool {
        matches!(self, MultiSigOutcome::Quorum { .. })
    }

    /// 是否阻塞 (Pending / Timeout / Invalid).
    pub fn is_blocking(&self) -> bool {
        !self.allows_proceed()
    }
}

/// 多签收集器.
pub struct MultiSigCollector {
    config: MultiSigConfig,
    payload_hash: String,
    signatures: Vec<PhysicalSignature>,
}

impl MultiSigCollector {
    /// 构造收集器, 锁定 payload_hash (intent 的不可变哈希).
    pub fn new(config: MultiSigConfig, payload_hash: impl Into<String>) -> Self {
        Self {
            config,
            payload_hash: payload_hash.into(),
            signatures: Vec::new(),
        }
    }

    /// 取当前 config.
    pub fn config(&self) -> &MultiSigConfig {
        &self.config
    }

    /// 取已收集的有效签名.
    pub fn signatures(&self) -> &[PhysicalSignature] {
        &self.signatures
    }

    /// 取锁定 payload_hash.
    pub fn payload_hash(&self) -> &str {
        &self.payload_hash
    }

    /// 当前有效签名数 (去重 + 合法).
    pub fn collected_count(&self) -> usize {
        self.signatures.len()
    }

    /// 提交一个签名 — 校验后加入; 返回 Err 表示拒绝.
    pub fn submit(&mut self, sig: PhysicalSignature) -> Result<(), MultiSigError> {
        if !sig.is_valid() {
            return Err(MultiSigError::InvalidSignature(
                "empty signer_id/payload_hash/signature".into(),
            ));
        }
        if sig.payload_hash != self.payload_hash {
            return Err(MultiSigError::HashMismatch {
                expected: self.payload_hash.clone(),
                got: sig.payload_hash.clone(),
            });
        }
        if !self
            .config
            .eligible_signers
            .iter()
            .any(|s| s == &sig.signer_id)
        {
            return Err(MultiSigError::SignerNotEligible(sig.signer_id.clone()));
        }
        if self.signatures.iter().any(|s| s.signer_id == sig.signer_id) {
            return Err(MultiSigError::DuplicateSigner(sig.signer_id.clone()));
        }
        // 截止时间校验
        if self.config.deadline > 0 && sig.signed_at > self.config.deadline {
            return Err(MultiSigError::ExpiredDeadline {
                deadline: self.config.deadline,
                signed_at: sig.signed_at,
            });
        }
        self.signatures.push(sig);
        Ok(())
    }

    /// 评估当前收集状态.
    pub fn evaluate(&self, now: i64) -> MultiSigOutcome {
        let count = self.collected_count();
        if count >= self.config.threshold {
            return MultiSigOutcome::Quorum {
                count,
                reached_at: now,
            };
        }
        let needed = self.config.threshold - count;
        if self.config.deadline > 0 && now > self.config.deadline {
            MultiSigOutcome::Timeout {
                collected: count,
                needed,
            }
        } else {
            MultiSigOutcome::Pending {
                collected: count,
                needed,
            }
        }
    }

    /// 校验流程: 是否达到 quorum (默认 now = Utc::now()).
    pub fn is_quorum(&self) -> bool {
        self.collected_count() >= self.config.threshold
    }
}

/// 多签错误.
#[derive(Debug, thiserror::Error)]
pub enum MultiSigError {
    /// 签名非法 (空字段).
    #[error("invalid signature: {0}")]
    InvalidSignature(String),
    /// payload_hash 不匹配.
    #[error("payload hash mismatch: expected `{expected}`, got `{got}`")]
    HashMismatch {
        /// 锁定的 hash.
        expected: String,
        /// 实际 hash.
        got: String,
    },
    /// 签名人不在白名单.
    #[error("signer `{0}` not eligible")]
    SignerNotEligible(String),
    /// 重复签名.
    #[error("duplicate signer `{0}`")]
    DuplicateSigner(String),
    /// 超过截止时间.
    #[error("signature signed_at `{signed_at}` past deadline `{deadline}`")]
    ExpiredDeadline {
        /// 截止时间.
        deadline: i64,
        /// 签名时间.
        signed_at: i64,
    },
}

/// 工具函数: 对 intent 的关键字段做稳定 hash (sha256-like hex, 实际可换算法).
pub fn intent_payload_hash(intent: &crate::intent::UpgradeIntent) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    intent.id.hash(&mut h);
    intent.manifest_id.hash(&mut h);
    intent.target_version.hash(&mut h);
    intent.current_version.hash(&mut h);
    format!("hash-{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::UpgradeIntent;
    use crate::manifest::UpgradeKind;
    use uuid::Uuid;

    fn sample_intent() -> UpgradeIntent {
        UpgradeIntent::new(
            Uuid::new_v4(),
            "v1.1.0",
            "v1.0.0",
            UpgradeKind::Patch,
            "carrier-a",
            "fix",
        )
    }

    fn sig(signer: &str, hash: &str, signed_at: i64) -> PhysicalSignature {
        PhysicalSignature::new(signer, hash, signed_at, format!("sig-{signer}"))
    }

    #[test]
    fn config_five_of_seven_default() {
        let cfg = MultiSigConfig::five_of_seven();
        assert!(cfg.is_valid());
        assert_eq!(cfg.threshold, 5);
        assert_eq!(cfg.eligible_signers.len(), 7);
    }

    #[test]
    fn config_invalid_threshold_zero() {
        let cfg = MultiSigConfig::new(0, vec!["a".into()]);
        assert!(!cfg.is_valid());
    }

    #[test]
    fn config_invalid_threshold_greater_than_n() {
        let cfg = MultiSigConfig::new(5, vec!["a".into(), "b".into()]);
        assert!(!cfg.is_valid());
    }

    #[test]
    fn collector_rejects_hash_mismatch() {
        let intent = sample_intent();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut c = MultiSigCollector::new(cfg, hash.clone());
        let wrong = PhysicalSignature::new("signer-0", "wrong-hash", 100, "sig");
        let err = c.submit(wrong).unwrap_err();
        assert!(matches!(err, MultiSigError::HashMismatch { .. }));
    }

    #[test]
    fn collector_rejects_non_eligible_signer() {
        let intent = sample_intent();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut c = MultiSigCollector::new(cfg, hash.clone());
        let s = sig("outsider", &hash, 100);
        let err = c.submit(s).unwrap_err();
        assert!(matches!(err, MultiSigError::SignerNotEligible(_)));
    }

    #[test]
    fn collector_rejects_duplicate_signer() {
        let intent = sample_intent();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut c = MultiSigCollector::new(cfg, hash.clone());
        c.submit(sig("signer-0", &hash, 100)).unwrap();
        let err = c.submit(sig("signer-0", &hash, 200)).unwrap_err();
        assert!(matches!(err, MultiSigError::DuplicateSigner(_)));
    }

    #[test]
    fn collector_rejects_empty_signature() {
        let intent = sample_intent();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut c = MultiSigCollector::new(cfg, hash.clone());
        let s = PhysicalSignature::new("signer-0", &hash, 100, "   ");
        let err = c.submit(s).unwrap_err();
        assert!(matches!(err, MultiSigError::InvalidSignature(_)));
    }

    #[test]
    fn collector_rejects_expired_deadline() {
        let intent = sample_intent();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::new(2, vec!["a".into(), "b".into()]).with_deadline(100);
        let mut c = MultiSigCollector::new(cfg, hash.clone());
        let s = sig("a", &hash, 200); // 超过 deadline=100
        let err = c.submit(s).unwrap_err();
        assert!(matches!(err, MultiSigError::ExpiredDeadline { .. }));
    }

    #[test]
    fn collector_5_of_7_quorum_reached() {
        let intent = sample_intent();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut c = MultiSigCollector::new(cfg, hash.clone());
        for i in 0..5 {
            c.submit(sig(&format!("signer-{i}"), &hash, 100 + i64::from(i)))
                .unwrap();
        }
        assert_eq!(c.collected_count(), 5);
        assert!(c.is_quorum());
        let outcome = c.evaluate(200);
        assert!(matches!(outcome, MultiSigOutcome::Quorum { count: 5, .. }));
        assert!(outcome.allows_proceed());
    }

    #[test]
    fn collector_below_threshold_pending() {
        let intent = sample_intent();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven();
        let mut c = MultiSigCollector::new(cfg, hash.clone());
        for i in 0..3 {
            c.submit(sig(&format!("signer-{i}"), &hash, 100 + i64::from(i)))
                .unwrap();
        }
        let outcome = c.evaluate(200);
        match outcome {
            MultiSigOutcome::Pending { collected, needed } => {
                assert_eq!(collected, 3);
                assert_eq!(needed, 2);
            }
            _ => panic!("expected Pending"),
        }
        assert!(outcome.is_blocking());
    }

    #[test]
    fn collector_timeout_after_deadline() {
        let intent = sample_intent();
        let hash = intent_payload_hash(&intent);
        let cfg = MultiSigConfig::five_of_seven().with_deadline(500);
        let mut c = MultiSigCollector::new(cfg, hash.clone());
        for i in 0..3 {
            c.submit(sig(&format!("signer-{i}"), &hash, 100 + i64::from(i)))
                .unwrap();
        }
        // 截止时间前: Pending
        let before = c.evaluate(400);
        assert!(matches!(before, MultiSigOutcome::Pending { .. }));
        // 截止时间后: Timeout
        let after = c.evaluate(600);
        assert!(matches!(after, MultiSigOutcome::Timeout { .. }));
        assert!(after.is_blocking());
    }

    #[test]
    fn outcome_allows_proceed_only_for_quorum() {
        assert!(MultiSigOutcome::Quorum {
            count: 5,
            reached_at: 1
        }
        .allows_proceed());
        assert!(!MultiSigOutcome::Pending {
            collected: 1,
            needed: 4
        }
        .allows_proceed());
        assert!(!MultiSigOutcome::Timeout {
            collected: 1,
            needed: 4
        }
        .allows_proceed());
        assert!(!MultiSigOutcome::Invalid { reason: "x".into() }.allows_proceed());
    }

    #[test]
    fn signature_with_witness() {
        let s = PhysicalSignature::new("a", "h", 1, "sig").with_witness("witness-1");
        assert_eq!(s.witness.as_deref(), Some("witness-1"));
        assert!(s.is_valid());
    }

    #[test]
    fn signature_invalid_when_empty() {
        let s = PhysicalSignature::new("", "h", 1, "sig");
        assert!(!s.is_valid());
        let s2 = PhysicalSignature::new("a", "  ", 1, "sig");
        assert!(!s2.is_valid());
        let s3 = PhysicalSignature::new("a", "h", 1, "");
        assert!(!s3.is_valid());
    }

    #[test]
    fn intent_payload_hash_stable() {
        let intent1 = sample_intent();
        let intent2 = sample_intent();
        // 不同的 UUID 应产生不同 hash
        assert_ne!(intent_payload_hash(&intent1), intent_payload_hash(&intent2));

        // 同一 intent 多次 hash 应一致
        let h1 = intent_payload_hash(&intent1);
        let h2 = intent_payload_hash(&intent1);
        assert_eq!(h1, h2);
    }
}
