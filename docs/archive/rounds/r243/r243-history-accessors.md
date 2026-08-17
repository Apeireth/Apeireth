# R243 -- consciousness history accessor API

## Problem
\EmotionEngine::history()\ returns ALL snapshots in chronological order, expensive on long-running
agents. There's no way to:
- get last N
- filter by timestamp
- clear history (testing / reset path)

## Solution: 4 new public methods

\\\ust
pub fn history_recent(&self, limit: usize) -> Vec<EmotionSnapshot>
    // last N snapshots in reverse order (newest first)

pub fn history_since(&self, since_ms: i64) -> Vec<EmotionSnapshot>
    // all snapshots whose timestamp_ms >= since (chronological)

pub fn history_clear(&mut self)
    // reset history

pub fn history_len(&self) -> usize
    // current depth (mirror of len() but explicit naming)
\\\

## Tests (5 new tests pass)
- r243_01: history_recent returns N most recent in reverse order
- r243_02: history_since filters by timestamp
- r243_03: history_clear empties
- r243_04: history_recent(0) returns empty; history_recent(overflow) returns all
- r243_05: history() chronologically ordered

## Files
- \crates/apeireth-consciousness/src/emotion.rs\ (+4 methods, +5 tests)

cumulative: ~6330 tests pass.