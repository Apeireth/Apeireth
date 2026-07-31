# Next-ROI Targets — V1156+ Roadmap (from v1155-fe71e8cb)

- **score 当前**: 0.8423
- **gap**: -0.1377
- **推荐 top-5 dim** (按 potential_gain = weight × (1-value))

| rank | dim | value | weight | potential_gain | rationale |
|-----:|-----|------:|-------:|---------------:|-----------|
| 1 | `self_improving_core` | 0.5000 | 0.0500 | **0.0250** | value=0.5000 R 真测, weight=0.0500 高, 潜在涨 0.0250 |
| 2 | `plugin_core` | 0.6500 | 0.0500 | **0.0175** | value=0.6500 R 真测, weight=0.0500 高, 潜在涨 0.0175 |
| 3 | `engineering` | 0.6636 | 0.0500 | **0.0168** | value=0.6636 R 真测, weight=0.0500 高, 潜在涨 0.0168 |
| 4 | `rubric_open` | 0.7000 | 0.0500 | **0.0150** | value=0.7000 R 真测, weight=0.0500 高, 潜在涨 0.0150 |
| 5 | `v2_philosophy` | 0.7143 | 0.0500 | **0.0143** | value=0.7143 R 真测, weight=0.0500 高, 潜在涨 0.0143 |

## Suggested V1156+ Module Names

- **V1156** = `self_improving_core` 真补 (potential_gain=0.0250)
- **V1157** = `plugin_core` 真补 (potential_gain=0.0175)
- **V1158** = `engineering` 真补 (potential_gain=0.0168)
- **V1159** = `rubric_open` 真补 (potential_gain=0.0150)
- **V1160** = `v2_philosophy` 真补 (potential_gain=0.0143)

## Notes (主 17:43 实事求是)

- potential_gain 是数学, 不预测实现路径
- 推荐 dim 是 V1156+ 候选, 不强制顺序
- 真补 R > 标 H/P > 标 M (主 17:58 不假装)
