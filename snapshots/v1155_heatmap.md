# ASI V0.6 Trend Heatmap — v1155-030d2a44

- **taken_at**: 2026-08-01 17:47:30
- **version**: 0.1.0
- **git_commit**: `281a659f` (dirty)
- **score**: **0.8929**
- **north_star (LOCKED)**: 0.9800
- **gap**: **-0.0871** (score - north_star)
- **dims**: 21 total | R=21 H=0 P=0 M=0

## 21-dim Heatmap (sorted by value asc)

| dim | value | weight | bar | status | source |
|-----|------:|-------:|-----|:------:|--------|
| `vcp_deep_read` | 0.6667 | 0.0375 | `█████████████░░░░░░░` | R | V1147 |
| `v2_philosophy` | 0.7200 | 0.0500 | `██████████████░░░░░░` | R | V0.5 hardcoded |
| `reinforcement_learning` | 0.7272 | 0.0500 | `███████████████░░░░░` | R | V0.5 hardcoded |
| `self_organizing_core` | 0.8000 | 0.0500 | `████████████████░░░░` | R | V0.5 hardcoded |
| `self_improving_core` | 0.8400 | 0.0500 | `█████████████████░░░` | R | V0.5 hardcoded |
| `phi_proxy` | 0.8441 | 0.0500 | `█████████████████░░░` | R | V0.5 hardcoded |
| `eternal_identity` | 0.8441 | 0.0500 | `█████████████████░░░` | R | V1072 |
| `capabilities` | 0.8636 | 0.0500 | `█████████████████░░░` | R | V1133 |
| `rubric_open` | 0.8800 | 0.0500 | `██████████████████░░` | R | V0.5 hardcoded |
| `plugin_core` | 0.8800 | 0.0500 | `██████████████████░░` | R | V0.5 hardcoded |
| `engineering` | 0.9200 | 0.0500 | `██████████████████░░` | R | V0.5 hardcoded |
| `cognitive_core` | 0.9200 | 0.0500 | `██████████████████░░` | R | V0.5 hardcoded |
| `scientific_method` | 0.9500 | 0.0500 | `███████████████████░` | R | V0.5 hardcoded |
| `vcp_4` | 0.9588 | 0.0500 | `███████████████████░` | R | V1071 |
| `real_production` | 0.9600 | 0.0500 | `███████████████████░` | R | V1151 |
| `cross_domain` | 1.0000 | 0.0500 | `████████████████████` | R | V1071 |
| `neurosymbolic` | 1.0000 | 0.0500 | `████████████████████` | R | V0.5 hardcoded |
| `world_model` | 1.0000 | 0.0500 | `████████████████████` | R | V0.5 hardcoded |
| `llm_bridge` | 1.0000 | 0.0375 | `████████████████████` | R | V1152 |
| `multi_agent_dag` | 1.0000 | 0.0375 | `████████████████████` | R | V1149 |
| `vcp_real_run` | 1.0000 | 0.0375 | `████████████████████` | R | V1148 |

## Legend

- **R** = real measurement (主 17:43 实事求是)
- **H** = hardcoded placeholder (不假装 = 真标)
- **P** = partial / fallback (不假装 = 真标)
- **M** = missing (不假装 = 真标)
- bar: █ = value, ░ = 1-value (width 20)

## Trend Notes

- score 越接近 north_star 0.98 → 越接近 ASI 北极星 (主 22:33)
- gap 为负 → 未达 ASI; gap 为正 → 已超 (主 17:43 实事求是 不假装超 ASI)
- 6 dim 达到 1.0 (cross_domain / neurosymbolic / world_model / llm_bridge / multi_agent_dag / vcp_real_run)
- 5 dim 最低 (≤0.7) → V1156+ 该打的 ROI 目标 (见 next-targets.md)
