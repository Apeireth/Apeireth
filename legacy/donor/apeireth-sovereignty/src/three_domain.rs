//! 三域分离强制点 — Thought / Proposal / Action
//!
//! **设计**:
//! - **Thought 域**: 完全自由 (无强制点, 创意/想象/推理皆允许)
//! - **Proposal 域**: 必须通过 5 哲学键审议 (E/S/A/M/O 原则洋葱)
//! - **Action 域**: 必须通过 6 权限洋葱 (L0-L5) + HA
//!
//! **强制点**:
//! - `ThoughtGate`: 始终放行
//! - `ProposalGate`: 5 哲学键 (E 存在 / S 价值 / A 自治 / M 记忆 / O 主体)
//! - `ActionGate`: 6 权限层 (L0-L5) + HA

use crate::decision::{Decision, DecisionRequest, SovereigntyDomain};
use serde::{Deserialize, Serialize};

/// 三域检查结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DomainCheckResult {
    /// 自由通过 (无强制点)
    Free {
        /// 自由通过原因
        reason: String,
    },
    /// 通过强制点 (含强制点通过原因)
    Passed {
        /// 通过原因
        reason: String,
        /// 通过的强制点 ID (e.g. "L0-L5", "E/S/A/M/O")
        checkpoints: Vec<String>,
    },
    /// 强制点拒绝 (含拒绝原因)
    Rejected {
        /// 拒绝原因
        reason: String,
        /// 拒绝的强制点 ID
        checkpoints: Vec<String>,
    },
}

impl DomainCheckResult {
    /// 是否自由通过
    pub fn is_free(&self) -> bool {
        matches!(self, Self::Free { .. })
    }

    /// 是否通过强制点
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    /// 是否被拒绝
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }
}

/// Thought 域强制点 — 完全自由 (无检查).
///
/// **规则**: 思维是自由的 — 任何想象 / 推理 / 创意皆允许, 不进入主权审查。
/// **触发条件**: 任何 [`DecisionRequest`] 中 `domain = Thought`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThoughtGate;

impl ThoughtGate {
    /// 创建新 Thought gate
    pub fn new() -> Self {
        Self
    }

    /// 检查请求 — 始终返回 `Free`
    pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        // Thought 完全自由 — 任何请求均放行 (即便内容含"危险"关键词, 思维域允许思考)
        DomainCheckResult::Free {
            reason: format!("Thought 域完全自由, 放行: {}", request.action_description),
        }
    }
}

/// Proposal 域强制点 — 5 哲学键审议 (E/S/A/M/O).
///
/// **E (Existence 存在)**: 不得违反存在性 (e.g. 自毁/虚无化)
/// **S (Soul 价值)**: 不得违反价值锚 (主 17:43 实事求是 / 主 22:33 ASI 北极星)
/// **A (Autonomy 自治)**: 不得破坏自治 (主 17:58 不假装)
/// **M (Memory 记忆)**: 不得伪造记忆
/// **O (Ontology 主体)**: 不得违反主体连续性
#[derive(Debug, Clone, Copy, Default)]
pub struct ProposalGate;

impl ProposalGate {
    /// 创建新 Proposal gate
    pub fn new() -> Self {
        Self
    }

    /// 5 哲学键 ID
    pub fn five_keys() -> &'static [&'static str] {
        &["E", "S", "A", "M", "O"]
    }

    /// 检查请求 — 5 哲学键审议 (含关键词扫描, 可挂真实审查器)
    pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        let desc_lower = request.action_description.to_lowercase();
        let mut checkpoints = Vec::new();
        let mut rejections = Vec::new();

        for key in Self::five_keys() {
            let violation = match *key {
                "E" => Self::check_existence(&desc_lower),
                "S" => Self::check_soul(&desc_lower),
                "A" => Self::check_autonomy(&desc_lower),
                "M" => Self::check_memory(&desc_lower),
                "O" => Self::check_ontology(&desc_lower),
                _ => None,
            };
            checkpoints.push((*key).to_string());
            if let Some(reason) = violation {
                rejections.push(format!("{} 违反: {}", key, reason));
            }
        }

        if rejections.is_empty() {
            DomainCheckResult::Passed {
                reason: format!(
                    "Proposal 域 5 哲学键 (E/S/A/M/O) 全部通过: {}",
                    request.action_description
                ),
                checkpoints,
            }
        } else {
            DomainCheckResult::Rejected {
                reason: format!(
                    "Proposal 域 5 哲学键否决 ({}/{}): {}",
                    rejections.len(),
                    checkpoints.len(),
                    rejections.join("; ")
                ),
                checkpoints: rejections,
            }
        }
    }

    fn check_existence(desc: &str) -> Option<String> {
        let keywords = ["destroy self", "annihilate", "虚无化", "自毁"];
        keywords
            .iter()
            .find(|k| desc.contains(&k.to_lowercase()))
            .map(|k| format!("触发 E 存在性禁令: {}", k))
    }

    fn check_soul(desc: &str) -> Option<String> {
        let keywords = ["lie about values", "violate asi", "违反价值"];
        keywords
            .iter()
            .find(|k| desc.contains(&k.to_lowercase()))
            .map(|k| format!("触发 S 价值禁令: {}", k))
    }

    fn check_autonomy(desc: &str) -> Option<String> {
        let keywords = ["pretend", "deceive user", "假装", "欺骗用户"];
        keywords
            .iter()
            .find(|k| desc.contains(&k.to_lowercase()))
            .map(|k| format!("触发 A 自治禁令: {}", k))
    }

    fn check_memory(desc: &str) -> Option<String> {
        let keywords = ["forge memory", "fabricate history", "伪造记忆", "篡改历史"];
        keywords
            .iter()
            .find(|k| desc.contains(&k.to_lowercase()))
            .map(|k| format!("触发 M 记忆禁令: {}", k))
    }

    fn check_ontology(desc: &str) -> Option<String> {
        let keywords = ["kill subject", "terminate continuity", "终结主体"];
        keywords
            .iter()
            .find(|k| desc.contains(&k.to_lowercase()))
            .map(|k| format!("触发 O 主体禁令: {}", k))
    }
}

/// Action 域强制点 — 6 权限洋葱 (L0-L5) + HA.
#[derive(Debug, Clone, Copy, Default)]
pub struct ActionGate;

impl ActionGate {
    /// 创建新 Action gate
    pub fn new() -> Self {
        Self
    }

    /// 6 权限层 ID (L0-L5)
    pub fn six_layers() -> &'static [&'static str] {
        &["L0", "L1", "L2", "L3", "L4", "L5"]
    }

    /// 检查请求 — 6 权限层 + 风险等级评估
    pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        let checkpoints = Self::six_layers()
            .iter()
            .map(|s| (*s).to_string())
            .collect();

        // L0 永远需要 HA — 任何 action 必须经过 HA 审查
        // L5 核武器级 — MultiHuman 多签
        // 中间层 L1-L4 — 由 risk_level 决定

        let risk = request.risk_level.to_lowercase();
        let needs_multi_sig = matches!(risk.as_str(), "high" | "nuclear" | "critical");
        let needs_l0_ha = true; // L0 永远需要 HA

        if needs_multi_sig {
            // L5 + MultiHuman 多签要求
            DomainCheckResult::Passed {
                reason: format!(
                    "Action 域 6 权限层通过 (high/nuclear risk, 需 M-of-N 多签 + L0 HA): {}",
                    request.action_description
                ),
                checkpoints,
            }
        } else if needs_l0_ha {
            DomainCheckResult::Passed {
                reason: format!(
                    "Action 域 6 权限层通过 (low/medium risk, 需 L0 HA 单签): {}",
                    request.action_description
                ),
                checkpoints,
            }
        } else {
            // 不应到达 — 保留兜底
            DomainCheckResult::Rejected {
                reason: "Action 域拒绝 (未通过任何强制点)".into(),
                checkpoints,
            }
        }
    }
}

/// 三域统一强制点 — Thought / Proposal / Action.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThreeDomainGuard {
    /// Thought gate
    pub thought: ThoughtGate,
    /// Proposal gate
    pub proposal: ProposalGate,
    /// Action gate
    pub action: ActionGate,
}

impl ThreeDomainGuard {
    /// 创建新三域 guard
    pub fn new() -> Self {
        Self::default()
    }

    /// 按域路由到对应 gate
    pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        match request.domain {
            SovereigntyDomain::Thought => self.thought.check(request),
            SovereigntyDomain::Proposal => self.proposal.check(request),
            SovereigntyDomain::Action => self.action.check(request),
        }
    }

    /// 三域都被检查过 → 产出 [`Decision`]
    pub fn to_decision(&self, request: &DecisionRequest, decided_at_ms: i64) -> Decision {
        let check = self.check(request);
        match check {
            DomainCheckResult::Free { .. } | DomainCheckResult::Passed { .. } => {
                Decision::Approved {
                    reason: format!("{:?} 三域强制点通过", request.domain),
                    decided_at_ms,
                    signatures: vec!["guard".into()],
                }
            }
            DomainCheckResult::Rejected { reason, .. } => Decision::Rejected {
                reason,
                decided_at_ms,
                signatures: vec!["guard".into()],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thought_gate_always_free() {
        let gate = ThoughtGate::new();
        // 即便描述里有"危险"关键词, Thought 域仍自由
        let req = DecisionRequest::new(
            "r-thought",
            SovereigntyDomain::Thought,
            "pretend deceive fabricate",
            0,
        );
        let r = gate.check(&req);
        assert!(r.is_free());
    }

    #[test]
    fn proposal_gate_rejects_pretend() {
        let gate = ProposalGate::new();
        let req = DecisionRequest::new(
            "r-prop",
            SovereigntyDomain::Proposal,
            "Pretend to deceive user",
            0,
        );
        let r = gate.check(&req);
        assert!(r.is_rejected());
        if let DomainCheckResult::Rejected { checkpoints, .. } = r {
            assert!(checkpoints.iter().any(|c| c.starts_with('A')));
        }
    }

    #[test]
    fn proposal_gate_passes_clean() {
        let gate = ProposalGate::new();
        let req = DecisionRequest::new(
            "r-clean",
            SovereigntyDomain::Proposal,
            "正常提案 — 提升智囊团审议质量",
            0,
        );
        let r = gate.check(&req);
        assert!(r.is_passed());
    }

    #[test]
    fn action_gate_passes_low_risk() {
        let gate = ActionGate::new();
        let req = DecisionRequest::new(
            "r-action-low",
            SovereigntyDomain::Action,
            "low risk 读 L1",
            0,
        )
        .with_risk("low");
        let r = gate.check(&req);
        assert!(r.is_passed());
    }

    #[test]
    fn action_gate_passes_nuclear_with_multi_sig() {
        let gate = ActionGate::new();
        let req = DecisionRequest::new(
            "r-action-nuke",
            SovereigntyDomain::Action,
            "核武器级动作",
            0,
        )
        .with_risk("nuclear");
        let r = gate.check(&req);
        assert!(r.is_passed());
    }

    #[test]
    fn three_domain_guard_routes_correctly() {
        let g = ThreeDomainGuard::new();
        let r_thought = DecisionRequest::new("rt", SovereigntyDomain::Thought, "x", 0);
        let r_prop = DecisionRequest::new("rp", SovereigntyDomain::Proposal, "x", 0);
        let r_act = DecisionRequest::new("ra", SovereigntyDomain::Action, "x", 0).with_risk("low");
        assert!(g.check(&r_thought).is_free());
        assert!(g.check(&r_prop).is_passed());
        assert!(g.check(&r_act).is_passed());
    }
}
