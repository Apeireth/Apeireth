# R246 -- cycle latency summary API

## Problem
R238 \u52a0\u4e86 \untime_cycle_duration_ms\ Histogram, \u4f46\u8c03\u7528\u65b9\u53ea\u80fd:
- \u8bbf \Histogram::count()\
- \u8bbf \Histogram::sum()\
- \u8bbf \Histogram::mean()\

\u4e0d\u65b9\u4fbf. \u4e00\u4e2a \CycleLatencySummary\ struct \u66f4\u9002\u5408:
- \u4f20\u9012 / JSON serialize
- \u5b58\u4e8b\u4ef6\u8bb0\u5f55
- \u5728 dashboard \u4e00\u6b21\u6027\u5448\u73b0

## Solution
\\\ust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CycleLatencySummary {
    pub count: u64,
    pub sum_ms: f64,
    pub mean_ms: f64,
}

impl Runtime {
    pub fn cycle_latency_summary(&self) -> CycleLatencySummary { ... }
}
\\\

## Tests (3 new tests pass)
- r246_01: initial zeros
- r246_02: \u8df3\u8fc7\u4e00\u4e2a cycle \u540e count=1, sum>=0, mean == sum/count
- r246_03: \u76f8\u540c\u5b57\u6bb5 == (\u7ed3\u6784\u4f26\u7406\u6bd4\u8f83)

## Files
- \crates/apeireth-runtime/src/lib.rs\ (+1 struct, +1 method, +3 tests)

cumulative: ~6339 tests pass.