# R242 -- cycle_report bus publish (tunable)

## Problem
\u6bcf\u6b21 run_one_cycle \u53ea inc \u4e86 \u8ba1\u6570\u5668\u4f46\u4ece\u672a\u8ba9\u5916\u90e8\u89c2\u5bdf\u8005\u770b\u5230\u201c\u8fd9\u4e2a\u5468\u671f\u91cc\u53d1\u751f\u4e86\u4ec0\u4e48\u201d.
\u9700\u8981\u4e00\u4e2a\u53ef\u8c03\u7684\u9891\u9053 \u2014\u2014 \u53ea\u6709\u5f00\u542f\u65f6\u624d\u53d1\u5e03.

## Solution
- \RuntimeConfig.publish_cycle_report: bool\ (\u9ed8\u8ba4 false, \u907f\u514d\u566a\u58f0)
- \u6bcf\u6b21 cycle \u672b\u5c3e\u53d1\u5e03 RuntimeEvent \u5230 \untime.cycle.report\ topic
- payload \u662f CycleReport \u7684 JSON \u5e8f\u5217\u5316 (\u542b task_id, \u6240\u6709\u6a21\u5757\u5e8f\u53f7, emotion \u72b6\u6001, elapsed_ms)

## Tests (2 new)
- r242_01: publish=false \u65f6\u4e0d\u5e94\u8be6\u589e bus.sent (\u9ed8\u8ba4)
- r242_02: publish=true \u65f6 bus.sent \u5fc5\u987b inc

## Files
- \crates/apeireth-runtime/src/lib.rs\ (+1 cfg field, +1 default entry, +publish block, +2 tests)

cumulative: ~6325 tests pass.