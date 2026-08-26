//! **战役 2-3 — 审批历史 + CallRecord + 时间工具**
//!
//! **目标**: ApprovalManager 维护一个 `VecDeque<CallRecord>` 作为审批历史,
//! 用于 FrequencyRule 反刷检测 + 调试 + 审计.
//!
//! **字段级引用 VCP**: `toolApprovalManager.js:11-19 config` 不带历史, 我们加 (Apeireth 创新)

use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_tool_runtime::ParsedToolCall;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::decision::ApprovalDecision;

/// **战役 2-3 — 审批历史 (单条记录)**
///
/// ApprovalManager 内部维护 `VecDeque<CallRecord>` 滑动窗口, 给 FrequencyRule 用
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CallRecord {
    /// 记录 ID (UUID v4, 战役 2-2 record 借鉴)
    pub id: String,
    /// 工具名
    pub tool_name: String,
    /// 调用参数
    pub args: Value,
    /// 评估时间戳 (unix ms)
    pub timestamp_ms: i64,
    /// 该调用的审批决策
    pub decision: ApprovalDecision,
    /// 命中的规则名 (VCP `matchedRule`)
    pub matched_rule: Option<String>,
    /// 命中的命令级键 (VCP `matchedCommand`; 命令级粒度审批, 无 = None)
    pub matched_command: Option<String>,
    /// 拒绝时是否静默 (VCP `notifyAiOnReject == false`; 留痕审计用)
    pub silent_on_reject: bool,
}

impl CallRecord {
    /// 构造新记录 (从 ParsedToolCall + decision)
    ///
    /// 新增审计字段 (`matched_command` / `silent_on_reject`) 默认空/false,
    /// 由 `ApprovalManager::check_detailed` 事后填入 (签名保持向后兼容).
    pub fn new(
        call: &ParsedToolCall,
        decision: ApprovalDecision,
        matched_rule: Option<String>,
    ) -> Self {
        Self {
            id: format!("apr-{}", Uuid::new_v4()),
            tool_name: call.tool_name.clone(),
            args: call.args.clone(),
            timestamp_ms: now_ms(),
            decision,
            matched_rule,
            matched_command: None,
            silent_on_reject: false,
        }
    }
}

/// **取当前 unix 毫秒** (单测可注入 mock, 默认 real time)
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_runtime::ParsedToolCall;
    use serde_json::json;

    #[test]
    fn now_ms_is_monotonic_nonzero() {
        let t1 = now_ms();
        let t2 = now_ms();
        assert!(t1 > 0, "now_ms 应 > 0 (2026 年 unix ms)");
        assert!(t2 >= t1, "now_ms 应单调递增, 实际 t1={t1} t2={t2}");
    }

    #[test]
    fn call_record_new_populates_fields() {
        let call = ParsedToolCall {
            tool_name: "TestTool".to_string(),
            args: json!({"foo": "bar"}),
            raw_marker: "tool_name:<<<TestTool>>>".to_string(),
            archery: false,
            archery_no_reply: false,
        };
        let rec = CallRecord::new(&call, ApprovalDecision::Allow, Some("trust".to_string()));
        assert!(rec.id.starts_with("apr-"));
        assert_eq!(rec.tool_name, "TestTool");
        assert_eq!(rec.matched_rule, Some("trust".to_string()));
        assert_eq!(rec.decision, ApprovalDecision::Allow);
        assert!(rec.timestamp_ms > 0);
    }

    #[test]
    fn call_record_unique_ids() {
        let call = ParsedToolCall {
            tool_name: "X".to_string(),
            args: json!({}),
            raw_marker: "tool_name:<<<X>>>".to_string(),
            archery: false,
            archery_no_reply: false,
        };
        let r1 = CallRecord::new(&call, ApprovalDecision::Allow, None);
        let r2 = CallRecord::new(&call, ApprovalDecision::Allow, None);
        assert_ne!(r1.id, r2.id, "UUID v4 应唯一");
    }
}
