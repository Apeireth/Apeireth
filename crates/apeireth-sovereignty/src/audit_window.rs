//! Q18 三域分离 — Thought 审计窗口 (best-effort + B 强制 + C 审计)
//!
//! **Q18 设计**:
//! - **B 强制**: 窗口外 (after `window_ms` since Thought) → Proposal / Action 必须走强制点
//! - **C 强制**: 主人可审计思维链 — `AuditWindowHistory` trait 暴露历史
//! - **Best-effort**: 窗口内 (within `window_ms` since Thought) → Thought 可 best-effort 触发 Proposal / Action
//!   即同一 request id 在窗口内免 Proposal 5 哲学键 / Action 6 权限层 强审
//!   (仅在该 Thought 已被 free 通过的前提下)
//!
//! **默认窗口**: 1000ms (1s)
//!
//! **诚实登记**:
//! - ❌ 不依赖 PyO3 / 外部 SDK
//! - ✅ 纯 Rust trait + in-memory impl, 可换 Redis / SQLite

use crate::decision::{DecisionRequest, SovereigntyDomain};
use crate::three_domain::ThreeDomainGuard;
use serde::{Deserialize, Serialize};

/// 默认审计窗口长度 (ms, 1 秒)。
pub const DEFAULT_AUDIT_WINDOW_MS: i64 = 1_000;

/// 窗口内最佳尝试决策。
///
/// **B 强制** + **Best-effort** 组合输出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowDecision {
    /// 窗口内 best-effort 触发 Proposal / Action (跳过 B 强制)
    BestEffortAllowed {
        /// 原始 request id
        request_id: String,
        /// 触发的下游域
        downstream: SovereigntyDomain,
        /// 距 Thought 通过的毫秒数
        elapsed_ms: i64,
    },
    /// 窗口内 best-effort 触发, 但检测到胁迫信号 — 仍然通过但记录在案
    BestEffortAllowedWithCoercion {
        /// 原始 request id
        request_id: String,
        /// 触发的下游域
        downstream: SovereigntyDomain,
        /// 距 Thought 通过的毫秒数
        elapsed_ms: i64,
        /// 胁迫等级 (0.0-1.0)
        stress_level: f32,
    },
    /// 窗口已过期 — 必须走 B 强制 (Proposal / Action 强制点)
    WindowExpired {
        /// 原始 request id
        request_id: String,
        /// 触发的下游域
        downstream: SovereigntyDomain,
        /// 距 Thought 通过的毫秒数 (超出窗口)
        elapsed_ms: i64,
    },
    /// Thought 域不允许触发 (仅 Thought 内 best-effort 可触发下游)
    NotApplicable {
        /// 原始 request id
        request_id: String,
        /// 当前域
        current_domain: SovereigntyDomain,
    },
}

/// 审计历史条目。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditHistoryEntry {
    /// request id
    pub request_id: String,
    /// Thought 通过时间 (ms)
    pub thought_at_ms: i64,
    /// 窗口内触发的下游请求 (可选)
    pub downstream: Option<SovereigntyDomain>,
    /// best-effort 触发的实际时间 (ms)
    pub downstream_at_ms: Option<i64>,
    /// 主人审计标记 (true = 已审)
    pub audited_by_owner: bool,
}

/// C 强制: 主人审计思维链 trait。
///
/// **C 强制**: 主人 (owner) 可查询 / 标记审计历史条目。
pub trait AuditWindowHistory {
    /// 记录 Thought 通过事件
    fn record_thought(&mut self, request_id: &str, thought_at_ms: i64);
    /// 记录 best-effort 触发下游事件
    fn record_downstream(
        &mut self,
        request_id: &str,
        downstream: SovereigntyDomain,
        downstream_at_ms: i64,
    );
    /// 标记主人审计
    fn mark_audited(&mut self, request_id: &str) -> bool;
    /// 查询历史
    fn history(&self) -> Vec<AuditHistoryEntry>;
    /// 按 request_id 查询
    fn lookup(&self, request_id: &str) -> Option<AuditHistoryEntry>;
    /// 清空历史
    fn clear(&mut self);
    /// 当前条目数
    fn len(&self) -> usize;
    /// 是否为空
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 默认内存实现 — 可换 Redis / SQLite 实现 `AuditWindowHistory`.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAuditHistory {
    entries: Vec<AuditHistoryEntry>,
}

impl InMemoryAuditHistory {
    /// 创建新空 history
    pub fn new() -> Self {
        Self::default()
    }
}

impl AuditWindowHistory for InMemoryAuditHistory {
    fn record_thought(&mut self, request_id: &str, thought_at_ms: i64) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.request_id == request_id) {
            existing.thought_at_ms = thought_at_ms;
        } else {
            self.entries.push(AuditHistoryEntry {
                request_id: request_id.to_string(),
                thought_at_ms,
                downstream: None,
                downstream_at_ms: None,
                audited_by_owner: false,
            });
        }
    }

    fn record_downstream(
        &mut self,
        request_id: &str,
        downstream: SovereigntyDomain,
        downstream_at_ms: i64,
    ) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.request_id == request_id) {
            existing.downstream = Some(downstream);
            existing.downstream_at_ms = Some(downstream_at_ms);
        } else {
            self.entries.push(AuditHistoryEntry {
                request_id: request_id.to_string(),
                thought_at_ms: downstream_at_ms,
                downstream: Some(downstream),
                downstream_at_ms: Some(downstream_at_ms),
                audited_by_owner: false,
            });
        }
    }

    fn mark_audited(&mut self, request_id: &str) -> bool {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.request_id == request_id) {
            existing.audited_by_owner = true;
            true
        } else {
            false
        }
    }

    fn history(&self) -> Vec<AuditHistoryEntry> {
        self.entries.clone()
    }

    fn lookup(&self, request_id: &str) -> Option<AuditHistoryEntry> {
        self.entries
            .iter()
            .find(|e| e.request_id == request_id)
            .cloned()
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Best-effort 流程控制器 — Thought → Proposal / Action 编排.
///
/// **Q18 行为**:
/// - Thought 请求通过 → 记录到 history (C 强制)
/// - 同 request_id 的 Proposal / Action 在 `window_ms` 内 → best-effort 放行 (无需 B 强制)
/// - 超出窗口 → 走 ThreeDomainGuard B 强制
pub struct BestEffortFlow<H: AuditWindowHistory> {
    /// 三域强制点
    pub guard: ThreeDomainGuard,
    /// 审计窗口长度 (ms)
    pub window_ms: i64,
    /// 审计历史 (C 强制)
    pub history: H,
}

impl<H: AuditWindowHistory> BestEffortFlow<H> {
    /// 创建新 best-effort flow
    pub fn new(history: H) -> Self {
        Self {
            guard: ThreeDomainGuard::new(),
            window_ms: DEFAULT_AUDIT_WINDOW_MS,
            history,
        }
    }

    /// 自定义窗口长度
    pub fn with_window_ms(mut self, window_ms: i64) -> Self {
        self.window_ms = window_ms;
        self
    }

    /// 处理 Thought 请求 — 记录到审计历史
    pub fn pass_thought(&mut self, request: &DecisionRequest) {
        self.history
            .record_thought(&request.id, request.submitted_at_ms);
    }

    /// 处理 Proposal / Action — 判断 best-effort or 强制
    ///
    /// **逻辑**:
    /// - request 域不是 Proposal / Action → NotApplicable
    /// - 找不到对应的 Thought 记录 → WindowExpired (强制)
    /// - 在窗口内 → BestEffortAllowed
    /// - 超出窗口 → WindowExpired
    pub fn process_downstream(&mut self, request: &DecisionRequest) -> WindowDecision {
        if !matches!(
            request.domain,
            SovereigntyDomain::Proposal | SovereigntyDomain::Action
        ) {
            return WindowDecision::NotApplicable {
                request_id: request.id.clone(),
                current_domain: request.domain,
            };
        }

        let entry = self.history.lookup(&request.id);
        let Some(entry) = entry else {
            return WindowDecision::WindowExpired {
                request_id: request.id.clone(),
                downstream: request.domain,
                elapsed_ms: i64::MAX,
            };
        };

        let elapsed = request.submitted_at_ms - entry.thought_at_ms;
        if elapsed <= self.window_ms {
            self.history
                .record_downstream(&request.id, request.domain, request.submitted_at_ms);
            WindowDecision::BestEffortAllowed {
                request_id: request.id.clone(),
                downstream: request.domain,
                elapsed_ms: elapsed,
            }
        } else {
            WindowDecision::WindowExpired {
                request_id: request.id.clone(),
                downstream: request.domain,
                elapsed_ms: elapsed,
            }
        }
    }

    /// 主人审计思维链 (C 强制)
    ///
    /// 返回成功审计的条目数。
    pub fn audit_by_owner(&mut self, request_id: &str) -> bool {
        self.history.mark_audited(request_id)
    }

    /// 取所有审计历史 (C 强制)
    pub fn audit_history(&self) -> Vec<AuditHistoryEntry> {
        self.history.history()
    }

    /// 取三域 guard (用于窗口外的强制点强制检查 — 仅检查不更新历史)
    pub fn guard_enforce(
        &self,
        request: &DecisionRequest,
    ) -> crate::three_domain::DomainCheckResult {
        self.guard.check(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_window_is_1_second() {
        assert_eq!(DEFAULT_AUDIT_WINDOW_MS, 1_000);
    }

    #[test]
    fn history_records_thought() {
        let mut h = InMemoryAuditHistory::new();
        h.record_thought("r-1", 100);
        assert_eq!(h.len(), 1);
        let e = h.lookup("r-1").unwrap();
        assert_eq!(e.thought_at_ms, 100);
        assert!(!e.audited_by_owner);
    }

    #[test]
    fn history_records_downstream() {
        let mut h = InMemoryAuditHistory::new();
        h.record_thought("r-1", 100);
        h.record_downstream("r-1", SovereigntyDomain::Action, 200);
        let e = h.lookup("r-1").unwrap();
        assert_eq!(e.downstream, Some(SovereigntyDomain::Action));
        assert_eq!(e.downstream_at_ms, Some(200));
    }

    #[test]
    fn history_mark_audited() {
        let mut h = InMemoryAuditHistory::new();
        h.record_thought("r-1", 100);
        assert!(h.mark_audited("r-1"));
        assert!(h.lookup("r-1").unwrap().audited_by_owner);
        assert!(!h.mark_audited("non-existent"));
    }

    #[test]
    fn best_effort_within_window() {
        let mut flow = BestEffortFlow::new(InMemoryAuditHistory::new()).with_window_ms(1000);
        let thought = DecisionRequest::new("r-1", SovereigntyDomain::Thought, "x", 100);
        flow.pass_thought(&thought);

        let action = DecisionRequest::new("r-1", SovereigntyDomain::Action, "y", 500);
        let r = flow.process_downstream(&action);
        match r {
            WindowDecision::BestEffortAllowed { elapsed_ms, .. } => assert_eq!(elapsed_ms, 400),
            other => panic!("expected BestEffortAllowed, got {:?}", other),
        }
    }

    #[test]
    fn best_effort_window_expired() {
        let mut flow = BestEffortFlow::new(InMemoryAuditHistory::new()).with_window_ms(1000);
        let thought = DecisionRequest::new("r-1", SovereigntyDomain::Thought, "x", 100);
        flow.pass_thought(&thought);

        let action = DecisionRequest::new("r-1", SovereigntyDomain::Action, "y", 1200);
        let r = flow.process_downstream(&action);
        assert!(matches!(r, WindowDecision::WindowExpired { .. }));
    }

    #[test]
    fn best_effort_no_thought_history() {
        let mut flow = BestEffortFlow::new(InMemoryAuditHistory::new());
        let action = DecisionRequest::new("r-unknown", SovereigntyDomain::Action, "y", 100);
        let r = flow.process_downstream(&action);
        assert!(matches!(r, WindowDecision::WindowExpired { .. }));
    }

    #[test]
    fn best_effort_thought_request_is_not_applicable() {
        let mut flow = BestEffortFlow::new(InMemoryAuditHistory::new());
        let t = DecisionRequest::new("r-1", SovereigntyDomain::Thought, "x", 100);
        let r = flow.process_downstream(&t);
        assert!(matches!(r, WindowDecision::NotApplicable { .. }));
    }

    #[test]
    fn best_effort_audit_by_owner() {
        let mut flow = BestEffortFlow::new(InMemoryAuditHistory::new());
        let t = DecisionRequest::new("r-1", SovereigntyDomain::Thought, "x", 100);
        flow.pass_thought(&t);
        assert!(flow.audit_by_owner("r-1"));
        let entries = flow.audit_history();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].audited_by_owner);
    }
}
