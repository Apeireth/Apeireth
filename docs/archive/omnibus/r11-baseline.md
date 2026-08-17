# R11 Baseline 3 值 (技术数据, 持续更新, R125 数字严守)

> **R119-3b Mavis 重建 (2026-08-10)**: R11 baseline 3 值严守, 数据不动。
> **R119-8 原则调整 (2026-08-10, 主人 1:49 拍板)**: 技术类文档不锁。R11 baseline 3 值 (0.8682 / 0.8532 / 0.9063) 是 8 项不修改承诺**严守**的 (数字 0 改), 但文档本身 (增量历史 / R-Method 状态 / 持续更新) 持续更新。
> **R125 A1 严守 (2026-08-10 16:55, Mavis 自主, 主人 16:31 最高权限)**: 0 改 R11 baseline 3 值数字, 仅扩展 V0.5 公式维度 (24→25 B3 升).

```
[Document-Meta]
Document: docs/omnibus/r11-baseline.md
Version: Manual-Rev-L + Fix-17 + R125-A1
R-Cycle: R125-A1
Last-Modified: 2026-08-10 (R125 A1 严守 16:55)
Status: 🟢 活跃 (技术数据, 数字 0 改, 文档持续更新)
```

## 3 值 (严守, 数字 0 改)

| 指标 | 值 | 含义 | R125 状态 |
|---|---|---|---|
| **V1141-R11** | **0.8682** | IC-001 fresh 测量 (24 维 V0.5 — `crates/apeireth-asi/src/lib.rs:pub const V05_DIM_COUNT: usize = 24`) | 🔒 0 改 (R125 B3 升 25 维, baseline 数字 0 改) |
| **V1131-R11** | **0.8532** | dashboard v05_total | 🔒 0 改 |
| **V1136-R11** | **0.9063** | 真测引擎 (9 子测度 — `crates/apeireth-asi/src/lib.rs:pub const V1136_SUBMEASURE_COUNT: usize = 9`) | 🔒 0 改 |

## R125 增量 (在 baseline 之上, 0 重写 baseline)

- R38 1.1 RC: 0.9063 → 0.92 (+0.014, 跟 9 organ + 8 LOCKED 落实 + R38 B1-B9 增量一致)
- R54 1.1.2 patch: 0.92 → ? (待 verify)
- R125-10 Kani 形式化: 0.92 → ? (Robustness 维增量)
- R125-13 LangGraph StateGraph: ? → ? (B3 30 维扩展, 待 R125-13 实施)

## R-Measure 守门原则

- 🔒 严守 R11 baseline 3 值 (不动)
- 🟢 后续 R-Measure 重测 = 在 baseline 之上增量 (0 重写 baseline)
- 0 重写 R11 baseline
- 0 假装 0 漂移
- **R125 A1 严守**: V1141=0.8682 / V1131=0.8532 / V1136=0.9063 数字 0 改, 0 装 "R125 已升级 baseline"

## 不漂移 (R125 A1 严守后)

- 🔒 24 LOCKED crate mtime 16:34 之前严守
- 🔒 workspace.version 1.1.0 (R125 末 B2 升 1.2.0, R127 release 1.0.0)
- 🔒 R11 baseline 3 值 (数字 0 改, A1 严守)
- 0 改 V0.5 公式 sum=1.00 守门 (R125 B3 升 25 维 0 改公式)
- 0 改 V1136 9 子测度 (R125 B3 升 25 维 0 改子测度数)
- 0 改 6 哲学锚 (R125 末 B5 升 8 锚)
- 0 改 9 organ 文件名 + 入口签名
- 0 改 12 键 (R125-12 后新增 PHL-07 = 13 键)

## 历史脉络

- R11 末: 主人 2026-07-31 明确不动 3 值
- R20 阶段 6: 8 项不修改承诺统一收口 (8-locked-unified §2 第 6 项)
- R38 1.1 RC: baseline 之上增量 (+0.014 → 0.92)
- R119 形式撤销: 8 项不修改承诺 (本文件数字 0 改)
- R119-8: 3 技术类 LOCKED 撤销 (baseline 数字仍 0 改, 仅文档结构更新)
- R125 A1 严守: 0 改 baseline 3 值, 0 装 "R125 已升级 baseline"
