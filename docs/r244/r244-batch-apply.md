# R244 -- EmotionEngine batch apply API

## Problem
\u5f53\u524d pply(EmotionEvent) \u5355\u4e2a call, \u591a\u4e2a\u4e8b\u4ef6\u9700\u591a\u6b21\u52a0\u9501.
\u5bf9 batch replay (\u91cd\u653e\u8bb0\u5f55/\u5386\u53f2\u4e8b\u4ef6\u91cd\u8ba1\u7b97) \u4e0d\u53cb\u597d.

## Solution: 2 batch helpers

\\\ust
pub fn apply_batch(&mut self, events: &[EmotionEvent]) -> EmResult<usize>
    // apply all sequentially, stop on first Err, return applied count

pub fn apply_batch_sum(&mut self, events: &[EmotionEvent]) -> f32
    // apply all (skipping errors silently), return total resonance added
\\\

## Tests (3 new tests pass)
- r244_01: apply_batch returns 3 on 3-event input
- r244_02: apply_batch([]) returns 0
- r244_03: apply_batch_sum returns positive resonance sum

## Files
- \crates/apeireth-consciousness/src/emotion.rs\ (+2 methods, +3 tests)

cumulative: ~6333 tests pass.