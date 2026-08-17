# R240 -- runtime lifecycle event emission

## Problem
\Runtime::start()\ / \Runtime::shutdown()\ 是 \u201c\u9759\u9ed8\u201d \u4e24\u4e2a\u751f\u547d\u5468\u671f\u70b9.
\u8c03\u7528\u65b9\u4ee5\u53ca\u5916\u90e8\u89c2\u6d4b\u4eea\u8868\u90fd\u4e0d\u77e5\u9053:
- \u8c01\u8c03\u4e86 start / \u8c01\u8c03\u4e86 shutdown
- \u4ec0\u4e48\u65f6\u5019\u53d1\u751f
- shutdown \u65f6\u603b\u5171\u8dd1\u8fc7\u591a\u5c11\u4e2a cycle / \u53d1\u51fa\u591a\u5c11\u4e2a decay event

## Solution: \u4e24\u4e2a Counter + \u4e24\u4e2a bus topic

### \u8ba1\u6570
\\\ust
runtime_lifecycle_started_total   (Counter)
runtime_lifecycle_shutdown_total  (Counter)
\\\

### Bus event
- start -> \u5230 topic \untime.started\, payload JSON (\u542b tick_interval, total_tasks)
- shutdown -> \u5230 topic \untime.shutdown\, payload JSON (\u542b cycle_total, decay_emit_total)

## Tests (3 new tests pass)
- r240_01: start() inc \u542f\u52a8\u8ba1\u6570 1
- r240_02: shutdown() inc \u5173\u95ed\u8ba1\u6570 1
- r240_03: Prometheus text export \u542b\u4e24\u4e2a metric name

## Files
- \crates/apeireth-runtime/src/lib.rs\ (+2 fields, +2 metrics, +1 each in start/shutdown, +3 tests)

cumulative: ~6321 tests pass.