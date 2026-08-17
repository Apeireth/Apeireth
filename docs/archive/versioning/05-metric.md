# 05 指标版本 (V<n>)

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-VERSIONING.md §5 拆出,核验后写。

```
[Document-Meta]
Document: docs/versioning/05-metric.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`V<n>` 或 `V<n>-<sub>`

## 指标清单 (核验后, R38-R118)

| 指标 | 含义 | R 周期 |
|---|---|---|
| `V0.5-R11` | ASI 真实值公式 (17 维 LOCKED) | R11 |
| `V0.5-v2-R14` | ASI 真实值公式 (24 维 v4.1 提议) | R14 |
| `V1136-R11` | R-Measure 真测基线 (0.9063) | R11 |
| `V1136-engine-R11` | R-Measure 真测引擎 (9 子测度) | R11 |
| `V3-9keys-R11` | 哲学守门 (9 键 LOCKED) | R11 |
| `V3-12keys-R14` | 哲学守门 (12 键 v4.1 提议) | R14 |
| `V0.5-24d-R38` | ASI 真实值公式 (24 维 V0.5 LOCKED, sum=1.00 守门, 编译期 hardcode enum) | R38 |
| `V1136-R38` | R-Measure 真测基线 (R38 重测) | R38 |
| `V1331-R38` | cognition graph 26 节点真跑 (R38 B8) | R38 |
| `V1136-R54` | R-Measure 重测基线 (R54 后 mid/long 真接有变动) | R54 |
| `V0172-R72` | MCP subscribe push 模式 (resources/subscribe 真接) | R70-R72 |
| `V0164-R72` | cross_model_benchmark 8 model 跨 tier (HELM tier 范式) | R70-R72 |
| `V0180-R72` | council deliberation stress test runner (p50/p95/p99 + consensus rate) | R70-R72 |
| `V1136-R72` | R-Measure 重测基线 (R70-R72 1.2 patch LIVE) | R70-R72 |
| **R119 新增 (R114-R118)** | (R114-R118 batch final 报告 5 个 V 指标) | R114-R118 |

## 当前 (R119 核验)

- `V1136-R114` (新增 R114-R118 后, 源仓 4921 / 88 suites / 0 failed)
- `V0.5-24d` 严守 (R38 LOCKED, 24 维公式)
- R-Measure 当前: 0.92 (README badge)

## 严守原则

- 🔒 `V1141=0.8682 / V1131=0.8532 / V1136=0.9063` R11 baseline 3 值严守
- 🟢 后续 V<n> 增量 (V0172 / V0164 / V0180 / V1136-R54 / V1136-R72) 形式可调
- 🟢 V0.5 24 维公式 (sum=1.00 守门) 编译期 hardcode 严守

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
- 0 改 V0.5 24 维公式
