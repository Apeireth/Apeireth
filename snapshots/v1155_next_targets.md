# Next-ROI Targets — V1156+ Roadmap (from v1155-dc598e78)

- **score 当前**: 0.8836
- **gap**: -0.0964
- **推荐 top-5 dim** (按 potential_gain = weight × (1-value))

| rank | dim | value | weight | potential_gain | rationale |
|-----:|-----|------:|-------:|---------------:|-----------|
| 1 | `rubric_open` | 0.7000 | 0.0500 | **0.0150** | value=0.7000 R 真测, weight=0.0500 高, 潜在涨 0.0150 |
| 2 | `v2_philosophy` | 0.7143 | 0.0500 | **0.0143** | value=0.7143 R 真测, weight=0.0500 高, 潜在涨 0.0143 |
| 3 | `reinforcement_learning` | 0.7272 | 0.0500 | **0.0136** | value=0.7272 R 真测, weight=0.0500 高, 潜在涨 0.0136 |
| 4 | `vcp_deep_read` | 0.6667 | 0.0375 | **0.0125** | value=0.6667 R 真测, weight=0.0375 高, 潜在涨 0.0125 |
| 5 | `self_organizing_core` | 0.8000 | 0.0500 | **0.0100** | value=0.8000 R 真测, weight=0.0500 高, 潜在涨 0.0100 |

## Suggested V1156+ Module Names

- **V1156** = `rubric_open` 真补 (potential_gain=0.0150)
- **V1157** = `v2_philosophy` 真补 (potential_gain=0.0143)
- **V1158** = `reinforcement_learning` 真补 (potential_gain=0.0136)
- **V1159** = `vcp_deep_read` 真补 (potential_gain=0.0125)
- **V1160** = `self_organizing_core` 真补 (potential_gain=0.0100)

## Notes (主 17:43 实事求是)

- potential_gain 是数学, 不预测实现路径
- 推荐 dim 是 V1156+ 候选, 不强制顺序
- 真补 R > 标 H/P > 标 M (主 17:58 不假装)
