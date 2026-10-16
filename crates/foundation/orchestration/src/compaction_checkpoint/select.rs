//! Selection: when to compact, and which span to compress.
//!
//! Both halves are pure functions over pure parameters, layered on the shared
//! overflow trigger math:
//!
//! - **When**: [`compaction_due`] is the boundary comparator of
//!   `context_overflow::trigger_threshold_tokens` — the same threshold the
//!   overflow recovery path uses, so "crossed the trigger" means one thing in
//!   this codebase.
//! - **Which span**: [`select_compaction_range`] keeps the tail verbatim within
//!   the configured `retain_tail` share of the post-overhead window and
//!   compresses the rest, then retreats the cut to a tool-pair-safe boundary:
//!   the cut may never fall between a tool call and its tool result.

use serde::{Deserialize, Serialize};

use crate::context_overflow::{
    exceeds_trigger, retain_tail_tokens, trigger_threshold_tokens, DEFAULT_RETAIN_TAIL_RATIO,
    RESERVE_TOKENS,
};

use super::stream::CompactionMessage;

/// Which span one compaction surface-replaces. Sequence numbers index the
/// append-only message stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionRange {
    /// First replaced message index (inclusive).
    pub start_seq: usize,
    /// First unreplaced message index (exclusive).
    pub end_seq: usize,
}

/// Window accounting for one compaction decision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompactionBudget {
    /// The model's context window in tokens.
    pub window_tokens: u64,
    /// Tokens already committed by the system block, tool declarations, and
    /// protocol framing.
    pub overhead_tokens: u64,
    /// Tokens kept free for the reply scaffold.
    pub reserve_tokens: u64,
    /// Share of the post-overhead window kept verbatim at the tail.
    pub retain_tail_ratio: f64,
}

impl CompactionBudget {
    /// A budget with the shared reserve and the default tail share.
    pub fn new(window_tokens: u64, overhead_tokens: u64) -> Self {
        Self {
            window_tokens,
            overhead_tokens,
            reserve_tokens: RESERVE_TOKENS,
            retain_tail_ratio: DEFAULT_RETAIN_TAIL_RATIO,
        }
    }

    /// Override the tail share (clamped to `0.0..=1.0` by
    /// `context_overflow::retain_tail_tokens`).
    #[must_use]
    pub fn with_retain_tail_ratio(mut self, ratio: f64) -> Self {
        self.retain_tail_ratio = ratio;
        self
    }

    /// The trigger threshold in tokens (the shared overflow trigger math).
    pub fn trigger_threshold(&self) -> u64 {
        trigger_threshold_tokens(
            self.window_tokens,
            self.overhead_tokens,
            self.reserve_tokens,
        )
    }

    /// Whether `estimated_tokens` has crossed the trigger — strictly above it.
    pub fn is_due(&self, estimated_tokens: u64) -> bool {
        exceeds_trigger(
            estimated_tokens,
            self.window_tokens,
            self.overhead_tokens,
            self.reserve_tokens,
        )
    }

    /// Tokens kept verbatim at the tail (the shared `retain_tail` parameter).
    pub fn retain_tail(&self) -> u64 {
        retain_tail_tokens(
            self.window_tokens,
            self.overhead_tokens,
            self.retain_tail_ratio,
        )
    }
}

/// Whether `estimated_tokens` has crossed the compaction trigger — the shared
/// overflow trigger threshold, one definition of "the window is filling up".
pub fn compaction_due(
    estimated_tokens: u64,
    window_tokens: u64,
    overhead_tokens: u64,
    reserve_tokens: u64,
) -> bool {
    exceeds_trigger(
        estimated_tokens,
        window_tokens,
        overhead_tokens,
        reserve_tokens,
    )
}

/// Select the span one compaction would surface-replace.
///
/// Rules, in order:
///
/// 1. Persistent system messages are never compressed: the span starts after
///    the leading system run, exactly like a core block is never cut.
/// 2. The tail is retained verbatim within `retain_tail_tokens`, greedy from
///    the newest message backwards.
/// 3. The newest message always survives verbatim (`end_seq <= len - 1`).
/// 4. The cut retreats to a tool-pair-safe boundary: no tool call may be
///    separated from its tool result. A pair that straddles the cut moves
///    whole into the retained tail.
///
/// Returns `None` when nothing compressible remains (the span would be empty,
/// or pair-safety retreats it to nothing): then no compaction happens at all.
pub fn select_compaction_range(
    messages: &[CompactionMessage],
    retain_tail_tokens: u64,
) -> Option<CompactionRange> {
    if messages.len() < 2 {
        return None;
    }

    // Rule 1: skip the leading system run.
    let mut start_seq = 0;
    while start_seq < messages.len()
        && messages[start_seq].role == super::stream::CompactionRole::System
    {
        start_seq += 1;
    }

    // Rule 2: greedy tail fit from the newest message backwards.
    let mut end_seq = messages.len();
    let mut tail_budget = retain_tail_tokens;
    while end_seq > start_seq {
        let cost = messages[end_seq - 1].estimated_tokens();
        if cost > tail_budget {
            break;
        }
        tail_budget -= cost;
        end_seq -= 1;
    }

    // Rule 3: the newest message always survives verbatim.
    end_seq = end_seq.min(messages.len() - 1);

    // Rule 4: retreat to a tool-pair-safe boundary.
    end_seq = pair_safe_end(messages, end_seq, start_seq);

    if end_seq <= start_seq {
        return None;
    }
    Some(CompactionRange { start_seq, end_seq })
}

/// Retreat `desired_end` until no tool call / tool result pair straddles the
/// cut at `start_seq`/`desired_end`. The boundary only ever moves backwards,
/// so the loop terminates and the retained tail only grows.
pub fn pair_safe_end(
    messages: &[CompactionMessage],
    desired_end: usize,
    start_seq: usize,
) -> usize {
    // First occurrence of each call site and each result site.
    let mut call_sites: Vec<(&str, usize)> = Vec::new();
    let mut result_sites: Vec<(&str, usize)> = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        for call_id in &message.tool_call_ids {
            if !call_sites.iter().any(|(id, _)| *id == call_id.as_str()) {
                call_sites.push((call_id.as_str(), index));
            }
        }
        if let Some(result_id) = &message.tool_result_id {
            if !result_sites.iter().any(|(id, _)| *id == result_id.as_str()) {
                result_sites.push((result_id.as_str(), index));
            }
        }
    }

    let mut end = desired_end.min(messages.len());
    loop {
        let mut retreat_to = end;
        // A call inside the compressed span whose result waits in the tail.
        for (call_id, call_index) in &call_sites {
            if *call_index >= end || *call_index < start_seq {
                continue;
            }
            if result_sites
                .iter()
                .any(|(result_id, result_index)| *result_id == *call_id && *result_index >= end)
            {
                retreat_to = retreat_to.min(*call_index);
            }
        }
        // A result inside the compressed span whose call waits in the tail
        // (a malformed order, but the cut must not deepen the malformation).
        for (result_id, result_index) in &result_sites {
            if *result_index >= end || *result_index < start_seq {
                continue;
            }
            if call_sites
                .iter()
                .any(|(call_id, call_index)| *call_id == *result_id && *call_index >= end)
            {
                retreat_to = retreat_to.min(*result_index);
            }
        }
        if retreat_to == end {
            return end;
        }
        end = retreat_to;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compaction_checkpoint::stream::CompactionRole;

    fn user(text: &str) -> CompactionMessage {
        CompactionMessage::new(CompactionRole::User, text)
    }

    /// The trigger comparator is the overflow trigger threshold: boundary is
    /// exclusive, one token over does it.
    #[test]
    fn trigger_comparator_is_the_overflow_threshold() {
        let budget = CompactionBudget::new(10_000, 1_000);
        let trigger = budget.trigger_threshold();
        assert_eq!(
            trigger,
            trigger_threshold_tokens(10_000, 1_000, RESERVE_TOKENS)
        );
        assert!(!budget.is_due(trigger));
        assert!(budget.is_due(trigger + 1));
    }

    /// The tail is the configured share of the post-overhead window, greedy
    /// from the newest message backwards.
    #[test]
    fn tail_retention_respects_the_share() {
        // 10 messages of 40 chars = 10 tokens each.
        let messages: Vec<_> = (0..10)
            .map(|i| user(&format!("m{i:02}{}", "x".repeat(37))))
            .collect();
        let range = select_compaction_range(&messages, 25).expect("compressible span exists");
        assert_eq!(range.start_seq, 0);
        assert_eq!(
            range.end_seq, 8,
            "尾部预算 25 tokens → 只留 2 条 (20 tokens)"
        );
    }

    /// The cut never separates a tool call from its tool result: a boundary
    /// inside a pair retreats until the whole pair sits on one side.
    #[test]
    fn the_cut_never_splits_a_tool_pair() {
        let messages = vec![
            user("head"),
            CompactionMessage::new(CompactionRole::Assistant, "")
                .with_tool_calls(vec!["c1".into()]),
            CompactionMessage::new(CompactionRole::Tool, "result-1").with_tool_result_id("c1"),
            CompactionMessage::new(CompactionRole::Assistant, "")
                .with_tool_calls(vec!["c2".into()]),
            CompactionMessage::new(CompactionRole::Tool, "result-2").with_tool_result_id("c2"),
            user("tail"),
        ];
        // A cut between the call at 1 and the result at 2 retreats to 1.
        assert_eq!(pair_safe_end(&messages, 2, 0), 1);
        // A cut between the call at 3 and the result at 4 retreats to 3.
        assert_eq!(pair_safe_end(&messages, 4, 0), 3);
        // Cuts outside pairs are already safe.
        assert_eq!(pair_safe_end(&messages, 1, 0), 1);
        assert_eq!(pair_safe_end(&messages, 3, 0), 3);
        assert_eq!(pair_safe_end(&messages, 6, 0), 6);
    }

    /// Whatever boundary is asked for, the resulting cut never straddles a
    /// call/result pair — swept over every possible cut.
    #[test]
    fn every_retreated_cut_is_pair_safe() {
        let messages = vec![
            user("m0"),
            CompactionMessage::new(CompactionRole::Assistant, "")
                .with_tool_calls(vec!["a".into(), "b".into()]),
            CompactionMessage::new(CompactionRole::Tool, "ra").with_tool_result_id("a"),
            CompactionMessage::new(CompactionRole::Tool, "rb").with_tool_result_id("b"),
            user("m4"),
            CompactionMessage::new(CompactionRole::Assistant, "").with_tool_calls(vec!["c".into()]),
            CompactionMessage::new(CompactionRole::Tool, "rc").with_tool_result_id("c"),
            user("m7"),
        ];
        for desired in 0..=messages.len() {
            let end = pair_safe_end(&messages, desired, 0);
            assert!(end <= desired, "切点只能回退: {desired} -> {end}");
            for (index, message) in messages.iter().enumerate() {
                for call_id in &message.tool_call_ids {
                    for (other, other_message) in messages.iter().enumerate() {
                        let answers =
                            other_message.tool_result_id.as_deref() == Some(call_id.as_str());
                        assert!(
                            !(index < end && other >= end) || !answers,
                            "切点 {end} 劈开了 {call_id} 的调用/结果对"
                        );
                    }
                }
            }
        }
    }

    /// With nothing compressible the selector declines instead of inventing a
    /// span.
    #[test]
    fn nothing_compressible_yields_no_range() {
        assert!(select_compaction_range(&[], 100).is_none());
        assert!(select_compaction_range(&[user("only")], 100).is_none());
    }
}
