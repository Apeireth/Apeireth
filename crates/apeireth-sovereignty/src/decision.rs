//! Decision: 主权决策请求与产出
//!
//! **设计**:
//! - `SovereigntyDomain` 标识请求来源域 (Thought / Proposal / Action)
//! - `DecisionRequest` 是输入 (含 risk_level, action_description, timestamps)
//! - `Decision` 是产出 (Approved / Rejected / Pending)
//! - `DecisionOutcome` 含完整追溯 (签名 / 决策时间戳 / 决策理由)

use serde::{Deserialize, Serialize};
use std::fmt;

/// 主权决策请求来源域 (三域分离对应)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SovereigntyDomain {
    /// 思维域 (自由, 无强制点)
    Thought,
    /// 提案域 (过 5 哲学键 — E/S/A/M/O 原则洋葱)
    Proposal,
    /// 行动域 (过 6 权限洋葱 — L0-L5)
    Action,
}

impl fmt::Display for SovereigntyDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Thought => "thought",
            Self::Proposal => "proposal",
            Self::Action => "action",
        };
        f.write_str(s)
    }
}

/// 主权决策请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionRequest {
    /// 唯一请求 ID
    pub id: String,
    /// 请求来源域
    pub domain: SovereigntyDomain,
    /// 动作描述
    pub action_description: String,
    /// 风险分级 (low / medium / high / nuclear)
    pub risk_level: String,
    /// 提交时间 (epoch ms)
    pub submitted_at_ms: i64,
    /// 引用历史 ID
    pub history_refs: Vec<String>,
}

impl DecisionRequest {
    /// 便利构造
    pub fn new(
        id: impl Into<String>,
        domain: SovereigntyDomain,
        action_description: impl Into<String>,
        submitted_at_ms: i64,
    ) -> Self {
        Self {
            id: id.into(),
            domain,
            action_description: action_description.into(),
            risk_level: "low".into(),
            submitted_at_ms,
            history_refs: Vec::new(),
        }
    }

    /// 设置风险等级
    pub fn with_risk(mut self, risk: impl Into<String>) -> Self {
        self.risk_level = risk.into();
        self
    }

    /// 添加历史引用
    pub fn with_history_ref(mut self, ref_id: impl Into<String>) -> Self {
        self.history_refs.push(ref_id.into());
        self
    }
}

/// 主权决策产出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Decision {
    /// 批准 (含决策理由)
    Approved {
        /// 理由
        reason: String,
        /// 决策时间 (epoch ms)
        decided_at_ms: i64,
        /// 决策者签名 (single=1 / multi=M-of-N)
        signatures: Vec<String>,
    },
    /// 拒绝 (含拒绝理由)
    Rejected {
        /// 拒绝理由
        reason: String,
        /// 决策时间 (epoch ms)
        decided_at_ms: i64,
        /// 决策者签名
        signatures: Vec<String>,
    },
    /// 挂起 (等待进一步审议)
    Pending {
        /// 等待原因
        reason: String,
        /// 决策时间 (epoch ms)
        decided_at_ms: i64,
        /// 复审时间 (epoch ms)
        review_at_ms: i64,
    },
}

impl Decision {
    /// 是否批准
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved { .. })
    }

    /// 是否拒绝
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }

    /// 是否挂起
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }

    /// 决策时间 (Pending 返回 review_at_ms)
    pub fn decided_at_ms(&self) -> i64 {
        match self {
            Self::Approved { decided_at_ms, .. } => *decided_at_ms,
            Self::Rejected { decided_at_ms, .. } => *decided_at_ms,
            Self::Pending { decided_at_ms, .. } => *decided_at_ms,
        }
    }

    /// 决策签名
    pub fn signatures(&self) -> &[String] {
        match self {
            Self::Approved { signatures, .. } => signatures,
            Self::Rejected { signatures, .. } => signatures,
            Self::Pending { .. } => &[],
        }
    }

    /// 决策理由
    pub fn reason(&self) -> &str {
        match self {
            Self::Approved { reason, .. } => reason,
            Self::Rejected { reason, .. } => reason,
            Self::Pending { reason, .. } => reason,
        }
    }
}

/// 主权决策产出 (完整追溯).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionOutcome {
    /// 决策请求 ID
    pub request_id: String,
    /// 决策来源域
    pub domain: SovereigntyDomain,
    /// 决策
    pub decision: Decision,
    /// 签发时间 (epoch ms)
    pub issued_at_ms: i64,
}

impl DecisionOutcome {
    /// 便利构造
    pub fn new(
        request_id: impl Into<String>,
        domain: SovereigntyDomain,
        decision: Decision,
        issued_at_ms: i64,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            domain,
            decision,
            issued_at_ms,
        }
    }

    /// 是否通过 (批准)
    pub fn is_allowed(&self) -> bool {
        self.decision.is_approved()
    }

    /// 是否拒绝
    pub fn is_rejected(&self) -> bool {
        self.decision.is_rejected()
    }
}

impl fmt::Display for DecisionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DecisionOutcome(req={}, domain={}, {:?})",
            self.request_id, self.domain, self.decision
        )
    }
}
