# Next-ROI Targets — V1156+ Roadmap (from v1155-2b6a4473)

- **score 当前**: 0.8593
- **gap**: -0.1207
- **推荐 top-5 dim** (按 potential_gain = weight × (1-value))

| rank | dim | value | weight | potential_gain | rationale |
|-----:|-----|------:|-------:|---------------:|-----------|
| 1 | `plugin_core` | 0.6500 | 0.0500 | **0.0175** | value=0.6500 R 真测, weight=0.0500 高, 潜在涨 0.0175 |
| 2 | `engineering` | 0.6636 | 0.0500 | **0.0168** | value=0.6636 R 真测, weight=0.0500 高, 潜在涨 0.0168 |
| 3 | `rubric_open` | 0.7000 | 0.0500 | **0.0150** | value=0.7000 R 真测, weight=0.0500 高, 潜在涨 0.0150 |
| 4 | `v2_philosophy` | 0.7143 | 0.0500 | **0.0143** | value=0.7143 R 真测, weight=0.0500 高, 潜在涨 0.0143 |
| 5 | `reinforcement_learning` | 0.7272 | 0.0500 | **0.0136** | value=0.7272 R 真测, weight=0.0500 高, 潜在涨 0.0136 |

## Suggested V1156+ Module Names

- **V1156** = `plugin_core` 真补 (potential_gain=0.0175)
- **V1157** = `engineering` 真补 (potential_gain=0.0168)
- **V1158** = `rubric_open` 真补 (potential_gain=0.0150)
- **V1159** = `v2_philosophy` 真补 (potential_gain=0.0143)
- **V1160** = `reinforcement_learning` 真补 (potential_gain=0.0136)

## Notes (主 17:43 实事求是)

- potential_gain 是数学, 不预测实现路径
- 推荐 dim 是 V1156+ 候选, 不强制顺序
- 真补 R > 标 H/P > 标 M (主 17:58 不假装)
