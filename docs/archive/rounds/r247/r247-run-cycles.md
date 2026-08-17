# R247 -- runtime run_cycles(n) batch API

## Problem
\u6d4b\u8bd5\u573a\u666f\u4e0e batch simulation \u9700\u8981 "run N cycles in a row", \u4f46\u73b0\u6709 API \u53ea\u63d0\u4f9b\u5355\u6b21 run_one_cycle(), n \u6b21 loop \u8c03\u7528\u4f1a\u91cd\u590d\u5e8f\u5217\u5bb9\u5668 / lifetime boilerplate.

## Solution
\\\ust
pub async fn run_cycles(&self, n: usize) -> RuntimeResult<Vec<CycleReport>>
\\\

\u00b7 \u8fd4\u56de\u6240\u6709\u5468\u671f\u62a5\u544a
\u00b7 \u4efb\u4e00\u4e2a cycle Err \u4f1a\u4e2d\u65ad\u5e8f\u5217
\u00b7 n=0 \u8fd4\u56de\u7a7a vec

## Tests (2 new tests pass)
- r247_01: n=0 \u8fd4\u56de empty, cycle_total \u4ecd\u4e3a 0
- r247_02: n=3 \u8fd4\u56de 3 reports, cycle_total=3, cycle_latency_summary.count=3

## Files
- \crates/apeireth-runtime/src/lib.rs\ (+1 method, +2 tests)

cumulative: ~6341 tests pass.