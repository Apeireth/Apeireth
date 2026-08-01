# Next-ROI Targets — V1156+ Roadmap (from v1155-a40f9177)

- **score 当前**: 0.8929
- **gap**: -0.0871
- **推荐 top-5 dim** (按 potential_gain = weight × (1-value))

| rank | dim | value | weight | potential_gain | rationale |
|-----:|-----|------:|-------:|---------------:|-----------|
| 1 | `v2_philosophy` | 0.7200 | 0.0500 | **0.0140** | value=0.7200 R 真测, weight=0.0500 高, 潜在涨 0.0140 |
| 2 | `reinforcement_learning` | 0.7272 | 0.0500 | **0.0136** | value=0.7272 R 真测, weight=0.0500 高, 潜在涨 0.0136 |
| 3 | `vcp_deep_read` | 0.6667 | 0.0375 | **0.0125** | value=0.6667 R 真测, weight=0.0375 高, 潜在涨 0.0125 |
| 4 | `self_organizing_core` | 0.8000 | 0.0500 | **0.0100** | value=0.8000 R 真测, weight=0.0500 高, 潜在涨 0.0100 |
| 5 | `self_improving_core` | 0.8400 | 0.0500 | **0.0080** | value=0.8400 R 真测, weight=0.0500 高, 潜在涨 0.0080 |

## Suggested V1156+ Module Names

- **V1156** = `v2_philosophy` 真补 (potential_gain=0.0140)
- **V1157** = `reinforcement_learning` 真补 (potential_gain=0.0136)
- **V1158** = `vcp_deep_read` 真补 (potential_gain=0.0125)
- **V1159** = `self_organizing_core` 真补 (potential_gain=0.0100)
- **V1160** = `self_improving_core` 真补 (potential_gain=0.0080)

## Notes (主 17:43 实事求是)

- potential_gain 是数学, 不预测实现路径
- 推荐 dim 是 V1156+ 候选, 不强制顺序
- 真补 R > 标 H/P > 标 M (主 17:58 不假装)
