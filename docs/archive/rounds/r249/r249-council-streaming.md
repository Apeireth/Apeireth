# R249 -- Council Streaming Deliberation (callback API)

## Problem
`Council::deliberate()` returns `CouncilVerdict` only after all opinions collected, hold
evaluated, and synthesis computed. TUI / bus / mcp-bridge / monitoring want real-time progress
events (1 per opinion issued, hold trigger, synthesis result), not just the final verdict.

## Solution

### 1. `DeliberationStreamEvent` enum
```rust
pub enum DeliberationStreamEvent {
    Started { session_id, query_id, started_at_ms },
    OpinionIssued { session_id, opinion },
    HoldTriggered { session_id, trigger: Option<HoldTrigger> },
    Synthesized { session_id, weighted_score, confidence, opinion_count },
    Completed { session_id, elapsed_ms, held },
}
```
Plus `Display` impl for human-readable prefixes (DeliberationStream::Started/Opinion/Hold/Synth/Done).

### 2. `Council::deliberate_streaming<F>` method
```rust
pub fn deliberate_streaming<F>(&mut self, query: CouncilQuery, mut on_event: F) -> CouncilVerdict
where
    F: FnMut(&DeliberationStreamEvent)
```

Follows same flow as `deliberate()` but additionally invokes `on_event` at 5 checkpoints:
1. Started (before first opinion)
2. OpinionIssued (per advisor)
3. HoldTriggered (always, with None when not triggered)
4. Synthesized (after synthesize)
5. Completed (after elapsed_ms computation)

Returns the same `CouncilVerdict` as `deliberate()`. Sovereignty hooks still fire (so streaming
is additive, not replacement).

## Tests (4 new tests pass)
- r249_01: 1 advisor approve -> 5 events (Started, Opinion, HoldTriggered(None), Synth, Completed)
- r249_02: 1 advisor StrongDisapprove (confidence 0.9) -> hold triggered
- r249_03: empty council -> 4 events (no Opinion), opinion_count=0
- r249_04: streaming vs deliberate produce equivalent verdicts

## Files
- `crates/apeireth-council/src/deliberation.rs` (+1 enum, +1 method, +4 tests)
- `crates/apeireth-council/src/lib.rs` (re-export `DeliberationStreamEvent`)

cumulative: ~6349 tests pass.
