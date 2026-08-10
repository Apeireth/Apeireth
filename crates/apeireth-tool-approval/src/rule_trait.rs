//! **战役 2-3 — ApprovalRule trait 抽象**
//!
//! **目标**: 5 规则 (Trust / Risk / Frequency / Whitelist / Blacklist) 都实现这个 trait,
//! ApprovalManager 按顺序调 `check` 拿 `ApprovalDecision`.
//!
//! **字段级引用 VCP**: `toolApprovalManager.js:144-225 getApprovalDecision` → 5 规则独立 check

use apeireth_tool_runtime::ParsedToolCall;

use crate::decision::ApprovalDecision;
use crate::history::CallRecord;

/// **战役 2-3 — 审批规则 trait**
///
/// 5 规则 (Trust / Risk / Frequency / Whitelist / Blacklist) 都实现此 trait.
/// ApprovalManager 按顺序调 `check`, 第一个非 `NoMatch` 的决策生效.
pub trait ApprovalRule: Send + Sync {
    /// 规则名 (供 log + 调试 + 报告 matchedRule 字段)
    ///
    /// VCP `matchedRule` 字段对应此 name (VCP 用魔法字符串, 我们用 typed name).
    fn name(&self) -> &str;

    /// 检查该调用
    ///
    /// **返回**:
    /// - `Allow` — 规则允许, 立即生效 (直接跳过下条规则)
    /// - `RequireApproval { timeout_ms }` — 需主人审批, 立即生效
    /// - `Deny { reason, silent }` — 拒绝, 立即生效
    /// - `NoMatch` — 当前规则不判断, 由下条规则接手
    fn check(&self, call: &ParsedToolCall, history: &[CallRecord]) -> ApprovalDecision;
}
