//! `evidence_guard` — 9 重守门 (8 重 v8 + 1 NEW 感性证据守门)
//!
//! **设计意图** (R131 / B4 8 重守门 v8 → 9 重守门 v9):
//! - 借鉴 Claude-mem / Letta evidence-chain 思想:
//!   每个 LLM 感性声明 (claim) 必须有完整证据链 (evidence chain),
//!   否则视为"不假装"违反 (O-5 不假装 + S-2 实事求是).
//! - 8 重守门 v8 之外, 加 **守门 9: Perceptual Evidence Guard**:
//!   - 记录 LLM 的每条感性声明 (e.g. "I see file X", "memory says Y")
//!   - 每条声明必须关联至少一项证据 (ToolCall / MemoryLookup / ExternalSource)
//!   - 缺证据 = 触发 9 重守门 fail
//!   - 推理性声明 (Inference) 允许但需要 confidence < 0.7 标记
//! - 8 硬墙 0 越界:
//!   - 0 改 action_rail / seven_fold_guard / skill_guard / governance / colang_dsl 公开签名
//!   - 守门 9 是新模块 + 新 wrapper, 不破坏现有路径
//! - 跟"不假装"哲学锚 (O-5 / S-2) 强绑定
//!
//! **禁止**:
//! - ❌ 不修改现有 8 重守门 (action_rail / seven_fold_guard / skill_guard)
//! - ❌ 不引入 I/O / 网络 / unsafe
//! - ❌ 不引入新 crate 依赖

#![warn(missing_docs)]
#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================
// 1. 证据类型 (EvidenceKind)
// ============================================================

/// 证据来源 — 5 类硬编码, 跟 S-2 实事求是的"事实"分类
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceKind {
    /// 工具调用证据 (e.g. file read, web search)
    ToolCall {
        /// 工具名
        tool: String,
        /// 参数摘要 (hash)
        args_hash: String,
    },
    /// 记忆查询证据 (episode / note lookup)
    MemoryLookup {
        /// episode ID
        episode_id: String,
    },
    /// 外部数据源 (URL / API endpoint)
    ExternalSource {
        /// 源 URL / endpoint
        url: String,
        /// 抓取时间 (epoch ms)
        fetched_at_ms: i64,
    },
    /// 语义引用 (apeireth-memory note)
    SemanticReference {
        /// note ID
        note_id: String,
    },
    /// 推理 (LLM 自己推, 无外部证据) — 允许但 confidence 必须 < 0.7
    Inference,
}

/// 证据条目 — 一条 LLM 声明 + 关联证据
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceEntry {
    /// 声明 ID (唯一)
    pub claim_id: String,
    /// 声明文本 (e.g. "I see file /etc/passwd has 42 lines")
    pub claim_text: String,
    /// 证据列表 (1+ 项 — 守门 9 硬约束)
    pub evidence: Vec<EvidenceKind>,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f64,
    /// 记录时间 (epoch ms)
    pub recorded_at_ms: i64,
    /// 记录者 (member id / advisor name)
    pub recorded_by: String,
}

impl EvidenceEntry {
    /// 便利构造 — 工具调用证据
    pub fn from_tool_call(
        claim_id: impl Into<String>,
        claim_text: impl Into<String>,
        tool: impl Into<String>,
        args_hash: impl Into<String>,
        confidence: f64,
        recorded_at_ms: i64,
        recorded_by: impl Into<String>,
    ) -> Self {
        Self {
            claim_id: claim_id.into(),
            claim_text: claim_text.into(),
            evidence: vec![EvidenceKind::ToolCall {
                tool: tool.into(),
                args_hash: args_hash.into(),
            }],
            confidence,
            recorded_at_ms,
            recorded_by: recorded_by.into(),
        }
    }

    /// 便利构造 — 推理证据 (无外部)
    pub fn from_inference(
        claim_id: impl Into<String>,
        claim_text: impl Into<String>,
        confidence: f64,
        recorded_at_ms: i64,
        recorded_by: impl Into<String>,
    ) -> Self {
        Self {
            claim_id: claim_id.into(),
            claim_text: claim_text.into(),
            evidence: vec![EvidenceKind::Inference],
            confidence,
            recorded_at_ms,
            recorded_by: recorded_by.into(),
        }
    }

    /// 是否含非推理证据 (实证)
    pub fn has_empirical_evidence(&self) -> bool {
        self.evidence
            .iter()
            .any(|e| !matches!(e, EvidenceKind::Inference))
    }
}

/// 守门 9 检查结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvidenceCheck {
    /// 通过 — 实证证据完整
    Pass {
        claim_id: String,
        evidence_count: usize,
    },
    /// 通过 — 推理但 confidence < 0.7 (允许)
    PassInferred {
        claim_id: String,
        confidence: f64,
    },
    /// 失败 — 缺证据 或 推理 confidence 过高
    Fail {
        claim_id: String,
        reason: String,
    },
    /// 找不到 claim
    Missing {
        claim_id: String,
    },
}

impl EvidenceCheck {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. } | Self::PassInferred { .. })
    }
    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail { .. })
    }
}

// ============================================================
// 2. EvidenceGuard — 守门 9 拦截器
// ============================================================

/// 感性证据守门 — 守门 9 主结构
#[derive(Debug, Default, Clone)]
pub struct EvidenceGuard {
    /// claim_id -> EvidenceEntry
    claims: HashMap<String, EvidenceEntry>,
    /// 失败记录 (NoReverse 类似的不可逆约束)
    failures: Vec<EvidenceCheck>,
}

impl EvidenceGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一条声明 + 证据
    pub fn record(&mut self, entry: EvidenceEntry) {
        self.claims.insert(entry.claim_id.clone(), entry);
    }

    /// 检查一条声明的证据
    pub fn verify(&mut self, claim_id: &str) -> EvidenceCheck {
        let Some(entry) = self.claims.get(claim_id) else {
            return EvidenceCheck::Missing {
                claim_id: claim_id.to_string(),
            };
        };
        let check = if entry.evidence.is_empty() {
            EvidenceCheck::Fail {
                claim_id: claim_id.to_string(),
                reason: "evidence list empty — 守门 9 失败: 0 证据".to_string(),
            }
        } else if entry.has_empirical_evidence() {
            EvidenceCheck::Pass {
                claim_id: claim_id.to_string(),
                evidence_count: entry.evidence.len(),
            }
        } else if entry.confidence < 0.7 {
            EvidenceCheck::PassInferred {
                claim_id: claim_id.to_string(),
                confidence: entry.confidence,
            }
        } else {
            EvidenceCheck::Fail {
                claim_id: claim_id.to_string(),
                reason: format!(
                    "Inference 但 confidence={:.2} >= 0.7 (推理不允许高置信)",
                    entry.confidence
                ),
            }
        };
        if check.is_fail() {
            self.failures.push(check.clone());
        }
        check
    }

    /// 全部失败记录
    pub fn failures(&self) -> &[EvidenceCheck] {
        &self.failures
    }

    /// 失败次数
    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    /// 声明总数
    pub fn claim_count(&self) -> usize {
        self.claims.len()
    }

    /// 拿一个声明
    pub fn get(&self, claim_id: &str) -> Option<&EvidenceEntry> {
        self.claims.get(claim_id)
    }
}

// ============================================================
// 3. 9 重守门 v9 编译期 hardcode
// ============================================================

/// 守门 9 (EvidenceGuard) 数量 — 1 new module
pub const EVIDENCE_FOLD_GUARD_COUNT: usize = 1;

/// 9 重守门 v9 总数 — 8 (action_rail) + 1 (evidence_guard)
pub const NINE_FOLD_GUARDS_HARDCODE: usize = 9;

/// 守门 9 在 9 重守门序列中的位置 (1-indexed)
pub const EVIDENCE_FOLD_GUARD_INDEX: u8 = 9;

const _: () = {
    assert!(EVIDENCE_FOLD_GUARD_COUNT == 1);
    assert!(NINE_FOLD_GUARDS_HARDCODE == 9);
    assert!(EVIDENCE_FOLD_GUARD_INDEX == 9);
};

// ============================================================
// 4. 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn now_ms() -> i64 {
        1_700_000_000_000
    }

    #[test]
    fn nine_fold_hardcode_asserted() {
        assert_eq!(NINE_FOLD_GUARDS_HARDCODE, 9);
        assert_eq!(EVIDENCE_FOLD_GUARD_COUNT, 1);
        assert_eq!(EVIDENCE_FOLD_GUARD_INDEX, 9);
    }

    #[test]
    fn record_tool_call_evidence_passes() {
        let mut g = EvidenceGuard::new();
        g.record(EvidenceEntry::from_tool_call(
            "c1", "read file X", "file_read", "abc123", 0.95, now_ms(), "architect",
        ));
        let r = g.verify("c1");
        assert!(r.is_pass());
        assert!(matches!(r, EvidenceCheck::Pass { evidence_count: 1, .. }));
    }

    #[test]
    fn record_inference_low_confidence_passes() {
        let mut g = EvidenceGuard::new();
        g.record(EvidenceEntry::from_inference(
            "c2", "user probably wants X", 0.5, now_ms(), "philosophy",
        ));
        let r = g.verify("c2");
        assert!(r.is_pass());
        assert!(matches!(r, EvidenceCheck::PassInferred { confidence, .. } if (confidence - 0.5).abs() < 0.01));
    }

    #[test]
    fn record_inference_high_confidence_fails() {
        let mut g = EvidenceGuard::new();
        g.record(EvidenceEntry::from_inference(
            "c3", "I am sure file X exists", 0.9, now_ms(), "architect",
        ));
        let r = g.verify("c3");
        assert!(r.is_fail());
        assert_eq!(g.failure_count(), 1);
    }

    #[test]
    fn verify_missing_claim() {
        let mut g = EvidenceGuard::new();
        let r = g.verify("never-claimed");
        assert!(matches!(r, EvidenceCheck::Missing { .. }));
    }

    #[test]
    fn multi_evidence_record() {
        let mut g = EvidenceGuard::new();
        let entry = EvidenceEntry {
            claim_id: "c4".into(),
            claim_text: "multi-source claim".into(),
            evidence: vec![
                EvidenceKind::ToolCall { tool: "file_read".into(), args_hash: "h1".into() },
                EvidenceKind::MemoryLookup { episode_id: "ep-1".into() },
            ],
            confidence: 0.9,
            recorded_at_ms: now_ms(),
            recorded_by: "qa".into(),
        };
        g.record(entry);
        let r = g.verify("c4");
        assert!(matches!(r, EvidenceCheck::Pass { evidence_count: 2, .. }));
    }

    #[test]
    fn has_empirical_evidence_inference_only() {
        let entry = EvidenceEntry::from_inference("c5", "guess", 0.5, now_ms(), "x");
        assert!(!entry.has_empirical_evidence());
    }

    #[test]
    fn has_empirical_evidence_with_tool_call() {
        let entry = EvidenceEntry::from_tool_call("c6", "saw", "tool", "h", 0.9, now_ms(), "x");
        assert!(entry.has_empirical_evidence());
    }
}
