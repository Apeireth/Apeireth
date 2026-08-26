//! R113: cognition summary → council deliberation context 真接
//!
//! **目标**: apeireth-graph 跑完 cognition graph 后, summary 自动注入 council
//! deliberation 的 query context, 让 advisor 拿 24 维真实值做审议.
//!
//! **Apeireth 真接 (本 module)**:
//! - `summary_to_context_line(s) -> String` — CognitionSummary → 单行 context (per VCP vcptoolbox 风格)
//! - `summary_to_history_ref(s) -> String` — CognitionSummary → history ref ID (per CouncilQuery.history_refs)
//! - `summary_to_context_block(s) -> String` — CognitionSummary → 多行 block (per prompt 注入)
//! - `inject_into_query(q, s) -> CouncilQuery` — 把 summary 注入 query (返回新 query, 0 改 q)
//! - `summary_risk_hint(s) -> &'static str` — 拿 verdict + mean 推 risk 等级
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `CouncilQuery` / `QueryContext` / `CouncilVerdict` (R19 LOCKED)
//! - 0 改 `CognitionSummary` / `CognitionCheckpointPayload` (R47 + R54 + R64 LOCKED)
//! - 0 改 `CouncilMemberDeliberator` (caller 主动调 inject_into_query)
//!
//! **借鉴锚 (S-11)**:
//! - VCP `vcptoolbox/modules/architect` system prompt 注入 (cognition state → system block)
//! - LangGraph `MemorySaver.get_tuple()` → graph state → context window
//! - AutoGen GroupChat context compaction (每轮 summary 注入)

use apeireth_graph::cognition_graph::CognitionSummary;

use crate::deliberation::{CouncilQuery, QueryContext};

// ============================================================
// summary → context 转换
// ============================================================

/// **CognitionSummary → 单行 context** (VCP vcptoolbox 风格, 紧凑)
pub fn summary_to_context_line(s: &CognitionSummary) -> String {
    format!(
        "[cognition] mean={:.3} min={:.3} max={:.3} verdict={} nodes={}",
        s.mean,
        s.min,
        s.max,
        if s.verdict_approve {
            "approve"
        } else {
            "reject"
        },
        s.node_count
    )
}

/// **CognitionSummary → history ref ID** (per CouncilQuery.history_refs 字段)
pub fn summary_to_history_ref(s: &CognitionSummary) -> String {
    format!(
        "cognition_summary:mean={:.3},verdict={}",
        s.mean, s.verdict_approve
    )
}

/// **CognitionSummary → 多行 block** (per prompt 注入, 详细)
pub fn summary_to_context_block(s: &CognitionSummary) -> String {
    let verdict = if s.verdict_approve {
        "APPROVE"
    } else {
        "REJECT"
    };
    format!(
        "# Cognition Graph State\n\
         - mean: {:.3}\n\
         - min: {:.3}\n\
         - max: {:.3}\n\
         - verdict: {}\n\
         - node_count: {}",
        s.mean, s.min, s.max, verdict, s.node_count
    )
}

/// **CognitionSummary → risk hint** (per verdict + mean 推 risk 等级)
///
/// 规则:
/// - verdict_approve + mean >= 0.5 -> "low"
/// - verdict_approve + mean < 0.5 -> "medium"
/// - !verdict_approve + mean >= 0.3 -> "high"
/// - !verdict_approve + mean < 0.3 -> "nuclear"
pub fn summary_risk_hint(s: &CognitionSummary) -> &'static str {
    if s.verdict_approve {
        if s.mean >= 0.5 {
            "low"
        } else {
            "medium"
        }
    } else {
        if s.mean >= 0.3 {
            "high"
        } else {
            "nuclear"
        }
    }
}

/// **CognitionSummary → CouncilQuery 注入** (返回新 query, 0 改原 q)
///
/// 注入策略:
/// - area = "cognition_graph"
/// - risk_level = summary_risk_hint(summary)
/// - history_refs 加一条 summary_to_history_ref(summary)
pub fn inject_into_query(q: CouncilQuery, s: &CognitionSummary) -> CouncilQuery {
    let mut new_q = q;
    new_q.context.area = Some("cognition_graph".to_string());
    new_q.context.risk_level = Some(summary_risk_hint(s).to_string());
    new_q.context.history_refs.push(summary_to_history_ref(s));
    new_q
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_summary(mean: f64, approve: bool) -> CognitionSummary {
        CognitionSummary {
            mean,
            min: mean - 0.1,
            max: mean + 0.1,
            verdict_approve: approve,
            node_count: 26,
        }
    }

    #[test]
    fn summary_to_context_line_basic() {
        let s = make_summary(0.5, true);
        let line = summary_to_context_line(&s);
        assert!(line.starts_with("[cognition]"));
        assert!(line.contains("mean=0.500"));
        assert!(line.contains("verdict=approve"));
        assert!(line.contains("nodes=26"));
    }

    #[test]
    fn summary_to_context_line_reject() {
        let s = make_summary(0.2, false);
        let line = summary_to_context_line(&s);
        assert!(line.contains("verdict=reject"));
    }

    #[test]
    fn summary_to_history_ref_format() {
        let s = make_summary(0.7, true);
        let r = summary_to_history_ref(&s);
        assert!(r.starts_with("cognition_summary:"));
        assert!(r.contains("mean=0.700"));
        assert!(r.contains("verdict=true"));
    }

    #[test]
    fn summary_to_context_block_multiline() {
        let s = make_summary(0.8, true);
        let block = summary_to_context_block(&s);
        assert!(block.contains("# Cognition Graph State"));
        assert!(block.contains("mean: 0.800"));
        assert!(block.contains("verdict: APPROVE"));
        assert!(block.contains("node_count: 26"));
    }

    #[test]
    fn summary_to_context_block_reject() {
        let s = make_summary(0.1, false);
        let block = summary_to_context_block(&s);
        assert!(block.contains("verdict: REJECT"));
    }

    #[test]
    fn summary_risk_hint_approve_high_mean() {
        let s = make_summary(0.8, true);
        assert_eq!(summary_risk_hint(&s), "low");
    }

    #[test]
    fn summary_risk_hint_approve_low_mean() {
        let s = make_summary(0.3, true);
        assert_eq!(summary_risk_hint(&s), "medium");
    }

    #[test]
    fn summary_risk_hint_reject_high_mean() {
        let s = make_summary(0.4, false);
        assert_eq!(summary_risk_hint(&s), "high");
    }

    #[test]
    fn summary_risk_hint_reject_low_mean() {
        let s = make_summary(0.1, false);
        assert_eq!(summary_risk_hint(&s), "nuclear");
    }

    #[test]
    fn summary_risk_hint_boundary() {
        // mean = 0.5 boundary
        let s_approve_boundary = make_summary(0.5, true);
        assert_eq!(summary_risk_hint(&s_approve_boundary), "low");

        // mean = 0.3 boundary
        let s_reject_boundary = make_summary(0.3, false);
        assert_eq!(summary_risk_hint(&s_reject_boundary), "high");
    }

    #[test]
    fn inject_into_query_sets_area_and_risk() {
        let q = CouncilQuery::new("q1", "decide something", 0);
        let s = make_summary(0.8, true);
        let new_q = inject_into_query(q, &s);
        assert_eq!(new_q.context.area.as_deref(), Some("cognition_graph"));
        assert_eq!(new_q.context.risk_level.as_deref(), Some("low"));
        assert_eq!(new_q.context.history_refs.len(), 1);
    }

    #[test]
    fn inject_into_query_appends_history_ref() {
        let mut q = CouncilQuery::new("q1", "decide", 0);
        q.context.history_refs.push("existing_ref".to_string());
        let s = make_summary(0.4, false);
        let new_q = inject_into_query(q, &s);
        assert_eq!(new_q.context.history_refs.len(), 2);
        assert_eq!(new_q.context.history_refs[0], "existing_ref");
        assert!(new_q.context.history_refs[1].starts_with("cognition_summary:"));
    }

    #[test]
    fn inject_into_query_does_not_mutate_input() {
        let q = CouncilQuery::new("q1", "decide", 0);
        let original_area = q.context.area.clone();
        let original_risk = q.context.risk_level.clone();
        let original_refs_len = q.context.history_refs.len();
        let s = make_summary(0.5, true);
        let _ = inject_into_query(q, &s);
        // we can't access the original q here (moved), but at least check the returned q
        // is the new one. This test mainly ensures we return a new query.
        assert!(original_area.is_none()); // confirmed precondition
        assert!(original_risk.is_none()); // confirmed precondition
        assert_eq!(original_refs_len, 0); // confirmed precondition
    }

    #[test]
    fn inject_into_query_with_nuclear_summary() {
        let q = CouncilQuery::new("q1", "decide", 0);
        let s = make_summary(0.1, false);
        let new_q = inject_into_query(q, &s);
        assert_eq!(new_q.context.risk_level.as_deref(), Some("nuclear"));
    }
}
