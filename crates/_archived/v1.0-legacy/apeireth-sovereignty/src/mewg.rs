//! MEWG 最高优先级解释权 — Multi-Evidence Weighted Governance
//!
//! **设计** (阶段 2 D2 §8.1 + §8.3):
//! - MEWG = 治理层多证据加权 (governance-level), 与 `apeireth-council` 的 synthesis
//!   (内部意见加权) 是不同层级
//! - MEWG 是**最高优先级解释权**: 当 MEWG 拒绝某决策时, 其他治理层必须尊重
//! - E 层在 MEWG 中是**硬门槛之一** (不可被分数抵消, §8.3)
//! - MEWG 系数初始值待生产校准, 不冻结
//!
//! **用法**:
//! ```ignore
//! use apeireth_sovereignty::mewg::{MewgAuthority, MewgVerdict, MewgEvidence};
//!
//! let authority = DefaultMewgAuthority::new();
//! let verdict = authority.evaluate(&decision, &evidences)?;
//! match verdict {
//!     MewgVerdict::Approved => { /* proceed */ }
//!     MewgVerdict::Blocked(reason) => { /* stop */ }
//! }
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// MEWG 错误
#[derive(Debug, Error)]
pub enum MewgError {
    #[error("missing required evidence: {0}")]
    MissingEvidence(String),
    #[error("invalid evidence weight: {0}")]
    InvalidWeight(f64),
}

/// 治理决策 — 提交给 MEWG 的输入
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    /// 决策 ID
    pub id: String,
    /// 决策标题
    pub title: String,
    /// 决策描述
    pub description: String,
    /// 是否涉及 E 层哲学修改 (硬门槛)
    pub touches_e_layer: bool,
    /// 决策分类标签
    pub tags: Vec<String>,
    /// 提交时间 (epoch seconds)
    pub submitted_at: i64,
    /// 可选元数据 (用于 owner_token 等扩展)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Decision {
    /// 创建新决策 (无 metadata)
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        touches_e_layer: bool,
        tags: Vec<String>,
        submitted_at: i64,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            touches_e_layer,
            tags,
            submitted_at,
            metadata: None,
        }
    }

    /// 设置 metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// 多证据 — MEWG 加权的输入
///
/// 每条 evidence 是单一来源的事实 (如: 智囊团 synthesis 报告 / 物理多签记录 /
/// 反思期倒计时等)。MEWG 综合这些 evidence 计算加权总分。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MewgEvidence {
    /// evidence ID
    pub id: String,
    /// 来源 (mewg / multi_human / multi_ai / physical_multisig / reflection)
    pub source: EvidenceSource,
    /// 加权分数 (-1.0 .. +1.0)
    pub score: f64,
    /// 权重 (0.0 .. 1.0, 编译时校验)
    pub weight: f64,
    /// 人类可读理由
    pub rationale: String,
    /// 时间戳
    pub timestamp: i64,
}

/// 证据来源 — 5 重治理标签
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceSource {
    /// MEWG 自身解释
    MewgSelf,
    /// 多人投票
    MultiHuman,
    /// 多 AI 表决
    MultiAi,
    /// 物理多签
    PhysicalMultisig,
    /// 反思期
    Reflection,
    /// 其他 (扩展位)
    Other,
}

impl MewgEvidence {
    /// 构造 evidence; weight 必须在 0.0..=1.0
    pub fn new(
        id: impl Into<String>,
        source: EvidenceSource,
        score: f64,
        weight: f64,
        rationale: impl Into<String>,
    ) -> Result<Self, MewgError> {
        if !(0.0..=1.0).contains(&weight) {
            return Err(MewgError::InvalidWeight(weight));
        }
        if !(-1.0..=1.0).contains(&score) {
            return Err(MewgError::InvalidWeight(score));
        }
        Ok(Self {
            id: id.into(),
            source,
            score: score.clamp(-1.0, 1.0),
            weight,
            rationale: rationale.into(),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }
}

/// MEWG 裁决
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MewgVerdict {
    /// 通过 — 加权分 ≥ 阈值
    Approved {
        /// 综合加权分
        weighted_score: f64,
        /// 综合理由
        rationale: String,
    },
    /// 拒绝 — 任一硬门槛失败 或 加权分 < 阈值
    Blocked {
        /// 综合加权分
        weighted_score: f64,
        /// 拒绝原因 (硬门槛失败或分数不足)
        reason: String,
    },
    /// 重审 — 反思期未结束 / 票数不足等
    PendingReview {
        /// 当前状态描述
        state: String,
        /// 加权分 (供参考)
        weighted_score: f64,
    },
}

/// MEWG 阈值 — 加权分 ≥ 此值才算 Approved
pub const DEFAULT_MEWG_APPROVAL_THRESHOLD: f64 = 0.6;

/// MEWG Authority trait — 最高优先级解释权
///
/// **不变量**:
/// - `evaluate` 必须返回确定性结果 (相同输入 → 相同输出)
/// - `evaluate` 不能 panic (返回 Err 即可)
pub trait MewgAuthority: Send + Sync {
    /// 评估决策 + 多证据 → 裁决
    fn evaluate(
        &self,
        decision: &Decision,
        evidences: &[MewgEvidence],
    ) -> Result<MewgVerdict, MewgError>;

    /// 该 authority 的标识 (用于审计)
    fn authority_id(&self) -> &str {
        "mewg-default"
    }
}

/// 默认 MEWG Authority — 简单加权求和 + 阈值判定
///
/// **算法**:
/// 1. 过滤 weight > 0 的 evidence
/// 2. `weighted_score = Σ(evidence.score × evidence.weight) / Σ(evidence.weight)`
/// 3. 若 `decision.touches_e_layer == true`, 至少需要一条 source=MultiHuman + score ≥ 0.5
///    的 evidence (E 层硬门槛, §8.3)
/// 4. `weighted_score >= threshold` → Approved; 否则 Blocked
pub struct DefaultMewgAuthority {
    /// 通过阈值
    pub threshold: f64,
}

impl DefaultMewgAuthority {
    /// 新建默认 MEWG authority (threshold = 0.6)
    pub fn new() -> Self {
        Self {
            threshold: DEFAULT_MEWG_APPROVAL_THRESHOLD,
        }
    }

    /// 自定义阈值
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
        }
    }
}

impl Default for DefaultMewgAuthority {
    fn default() -> Self {
        Self::new()
    }
}

impl MewgAuthority for DefaultMewgAuthority {
    fn evaluate(
        &self,
        decision: &Decision,
        evidences: &[MewgEvidence],
    ) -> Result<MewgVerdict, MewgError> {
        let mut sum_weighted = 0.0_f64;
        let mut sum_weight = 0.0_f64;
        let mut has_multi_human = false;
        let mut has_multi_human_approve = false;

        for e in evidences {
            if e.weight <= 0.0 {
                continue;
            }
            sum_weighted += e.score * e.weight;
            sum_weight += e.weight;
            if matches!(e.source, EvidenceSource::MultiHuman) {
                has_multi_human = true;
                if e.score >= 0.5 {
                    has_multi_human_approve = true;
                }
            }
        }

        let weighted_score = if sum_weight > 0.0 {
            (sum_weighted / sum_weight).clamp(-1.0, 1.0)
        } else {
            0.0
        };

        // E 层硬门槛 (§8.3): touches_e_layer 必须有 ≥1 MultiHuman 批准
        if decision.touches_e_layer && !(has_multi_human && has_multi_human_approve) {
            return Ok(MewgVerdict::Blocked {
                weighted_score,
                reason: "E 层修改硬门槛 (§8.3): 需至少一条 MultiHuman 批准的 evidence".into(),
            });
        }

        if weighted_score >= self.threshold {
            Ok(MewgVerdict::Approved {
                weighted_score,
                rationale: format!("加权分 {:.3} ≥ 阈值 {:.2}", weighted_score, self.threshold),
            })
        } else {
            Ok(MewgVerdict::Blocked {
                weighted_score,
                reason: format!("加权分 {:.3} < 阈值 {:.2}", weighted_score, self.threshold),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, source: EvidenceSource, score: f64, weight: f64) -> MewgEvidence {
        MewgEvidence::new(id, source, score, weight, "test").unwrap()
    }

    #[test]
    fn mewg_authority_approved_when_score_exceeds_threshold() {
        let auth = DefaultMewgAuthority::new();
        let decision = Decision {
            id: "d1".into(),
            title: "t".into(),
            description: "d".into(),
            touches_e_layer: false,
            tags: vec![],
            submitted_at: 0,
            metadata: None,
        };
        let evidences = vec![
            ev("e1", EvidenceSource::MultiHuman, 0.8, 0.5),
            ev("e2", EvidenceSource::MultiAi, 0.7, 0.5),
        ];
        let verdict = auth.evaluate(&decision, &evidences).unwrap();
        assert!(matches!(verdict, MewgVerdict::Approved { .. }));
    }

    #[test]
    fn mewg_authority_blocked_when_score_below_threshold() {
        let auth = DefaultMewgAuthority::new();
        let decision = Decision {
            id: "d1".into(),
            title: "t".into(),
            description: "d".into(),
            touches_e_layer: false,
            tags: vec![],
            submitted_at: 0,
            metadata: None,
        };
        let evidences = vec![
            ev("e1", EvidenceSource::MultiHuman, -0.3, 0.5),
            ev("e2", EvidenceSource::MultiAi, 0.2, 0.5),
        ];
        let verdict = auth.evaluate(&decision, &evidences).unwrap();
        assert!(matches!(verdict, MewgVerdict::Blocked { .. }));
    }

    #[test]
    fn mewg_authority_e_layer_hard_gate_blocks_without_human_approval() {
        let auth = DefaultMewgAuthority::new();
        let decision = Decision {
            id: "e-mod".into(),
            title: "modify E layer".into(),
            description: "changes core philosophy".into(),
            touches_e_layer: true,
            tags: vec![],
            submitted_at: 0,
            metadata: None,
        };
        // 没有 MultiHuman 证据 → 硬门槛失败
        let evidences = vec![
            ev("e1", EvidenceSource::MultiAi, 0.9, 0.5),
            ev("e2", EvidenceSource::PhysicalMultisig, 0.9, 0.5),
        ];
        let verdict = auth.evaluate(&decision, &evidences).unwrap();
        match verdict {
            MewgVerdict::Blocked { reason, .. } => {
                assert!(reason.contains("E 层"));
            }
            _ => panic!("E 层硬门槛应 Blocked"),
        }
    }

    #[test]
    fn mewg_authority_e_layer_passes_with_human_approval() {
        let auth = DefaultMewgAuthority::new();
        let decision = Decision {
            id: "e-mod".into(),
            title: "modify E layer".into(),
            description: "d".into(),
            touches_e_layer: true,
            tags: vec![],
            submitted_at: 0,
            metadata: None,
        };
        let evidences = vec![
            ev("h", EvidenceSource::MultiHuman, 0.8, 0.5), // 人类批准
            ev("e2", EvidenceSource::MultiAi, 0.9, 0.5),
        ];
        let verdict = auth.evaluate(&decision, &evidences).unwrap();
        assert!(matches!(verdict, MewgVerdict::Approved { .. }));
    }

    #[test]
    fn mewg_evidence_validates_weight_range() {
        assert!(MewgEvidence::new("e", EvidenceSource::Other, 0.5, 1.5, "x").is_err());
        assert!(MewgEvidence::new("e", EvidenceSource::Other, 0.5, -0.1, "x").is_err());
        assert!(MewgEvidence::new("e", EvidenceSource::Other, 2.0, 0.5, "x").is_err());
    }
}
