# Next-ROI Targets — V1156+ Roadmap (from v1155-5bc488ef)

- **score 当前**: 0.8929
- **gap**: -0.0871
- **推荐 top-21 dim** (按 potential_gain = weight × (1-value))

| rank | dim | value | weight | potential_gain | rationale |
|-----:|-----|------:|-------:|---------------:|-----------|
| 1 | `v2_philosophy` | 0.7200 | 0.0500 | **0.0140** | value=0.7200 R 真测, weight=0.0500 高, 潜在涨 0.0140 |
| 2 | `reinforcement_learning` | 0.7272 | 0.0500 | **0.0136** | value=0.7272 R 真测, weight=0.0500 高, 潜在涨 0.0136 |
| 3 | `vcp_deep_read` | 0.6667 | 0.0375 | **0.0125** | value=0.6667 R 真测, weight=0.0375 高, 潜在涨 0.0125 |
| 4 | `self_organizing_core` | 0.8000 | 0.0500 | **0.0100** | value=0.8000 R 真测, weight=0.0500 高, 潜在涨 0.0100 |
| 5 | `self_improving_core` | 0.8400 | 0.0500 | **0.0080** | value=0.8400 R 真测, weight=0.0500 高, 潜在涨 0.0080 |
| 6 | `phi_proxy` | 0.8441 | 0.0500 | **0.0078** | value=0.8441 R 真测, weight=0.0500 高, 潜在涨 0.0078 |
| 7 | `eternal_identity` | 0.8441 | 0.0500 | **0.0078** | value=0.8441 R 真测, weight=0.0500 高, 潜在涨 0.0078 |
| 8 | `capabilities` | 0.8636 | 0.0500 | **0.0068** | value=0.8636 R 真测, weight=0.0500 高, 潜在涨 0.0068 |
| 9 | `rubric_open` | 0.8800 | 0.0500 | **0.0060** | value=0.8800 R 真测, weight=0.0500 高, 潜在涨 0.0060 |
| 10 | `plugin_core` | 0.8800 | 0.0500 | **0.0060** | value=0.8800 R 真测, weight=0.0500 高, 潜在涨 0.0060 |
| 11 | `engineering` | 0.9200 | 0.0500 | **0.0040** | value=0.9200 R 真测, weight=0.0500 高, 潜在涨 0.0040 |
| 12 | `cognitive_core` | 0.9200 | 0.0500 | **0.0040** | value=0.9200 R 真测, weight=0.0500 高, 潜在涨 0.0040 |
| 13 | `scientific_method` | 0.9500 | 0.0500 | **0.0025** | value=0.9500 R 真测, weight=0.0500 高, 潜在涨 0.0025 |
| 14 | `vcp_4` | 0.9588 | 0.0500 | **0.0021** | value=0.9588 R 真测, weight=0.0500 高, 潜在涨 0.0021 |
| 15 | `real_production` | 0.9600 | 0.0500 | **0.0020** | value=0.9600 R 真测, weight=0.0500 高, 潜在涨 0.0020 |
| 16 | `cross_domain` | 1.0000 | 0.0500 | **0.0000** | value=1.0000 已接近 1.0, 优先打 (weight=0.0500) |
| 17 | `neurosymbolic` | 1.0000 | 0.0500 | **0.0000** | value=1.0000 已接近 1.0, 优先打 (weight=0.0500) |
| 18 | `world_model` | 1.0000 | 0.0500 | **0.0000** | value=1.0000 已接近 1.0, 优先打 (weight=0.0500) |
| 19 | `llm_bridge` | 1.0000 | 0.0375 | **0.0000** | value=1.0000 已接近 1.0, 优先打 (weight=0.0375) |
| 20 | `multi_agent_dag` | 1.0000 | 0.0375 | **0.0000** | value=1.0000 已接近 1.0, 优先打 (weight=0.0375) |
| 21 | `vcp_real_run` | 1.0000 | 0.0375 | **0.0000** | value=1.0000 已接近 1.0, 优先打 (weight=0.0375) |

## Suggested V1156+ Module Names

- **V1156** = `v2_philosophy` 真补 (potential_gain=0.0140)
- **V1157** = `reinforcement_learning` 真补 (potential_gain=0.0136)
- **V1158** = `vcp_deep_read` 真补 (potential_gain=0.0125)
- **V1159** = `self_organizing_core` 真补 (potential_gain=0.0100)
- **V1160** = `self_improving_core` 真补 (potential_gain=0.0080)
- **V1161** = `phi_proxy` 真补 (potential_gain=0.0078)
- **V1162** = `eternal_identity` 真补 (potential_gain=0.0078)
- **V1163** = `capabilities` 真补 (potential_gain=0.0068)
- **V1164** = `rubric_open` 真补 (potential_gain=0.0060)
- **V1165** = `plugin_core` 真补 (potential_gain=0.0060)
- **V1166** = `engineering` 真补 (potential_gain=0.0040)
- **V1167** = `cognitive_core` 真补 (potential_gain=0.0040)
- **V1168** = `scientific_method` 真补 (potential_gain=0.0025)
- **V1169** = `vcp_4` 真补 (potential_gain=0.0021)
- **V1170** = `real_production` 真补 (potential_gain=0.0020)
- **V1171** = `cross_domain` 真补 (potential_gain=0.0000)
- **V1172** = `neurosymbolic` 真补 (potential_gain=0.0000)
- **V1173** = `world_model` 真补 (potential_gain=0.0000)
- **V1174** = `llm_bridge` 真补 (potential_gain=0.0000)
- **V1175** = `multi_agent_dag` 真补 (potential_gain=0.0000)
- **V1176** = `vcp_real_run` 真补 (potential_gain=0.0000)

## Notes (主 17:43 实事求是)

- potential_gain 是数学, 不预测实现路径
- 推荐 dim 是 V1156+ 候选, 不强制顺序
- 真补 R > 标 H/P > 标 M (主 17:58 不假装)
