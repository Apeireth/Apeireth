# 11 R-Measure Baseline 3 值 + V0.5 25 维 (技术数据, 持续更新)

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §11 拆出。
> **R119-8 原则调整 (2026-08-10, 主人 1:49 拍板)**: 技术类文档不锁, 时刻保持最新。**数据严守 ≠ 文档结构锁** — R11 baseline 3 值 (0.8682 / 0.8532 / 0.9063) 是 8 项不修改承诺里严守的 (数字 0 改), 但文档结构 (增量历史 / 修真记录 / R-Method 状态) 持续更新。
> **R125 B3 升 25 维 (2026-08-10 16:55, Mavis 自主, 主人 16:31 最高权限授权)**: V0.5 24 维 → 25 维, 加 Robustness 鲁棒性 (R125-10 Kani 形式化借鉴触发). R11 baseline 3 值数字严守 0 改.

```
[Document-Meta]
Document: docs/conventions/11-baseline.md
Version: Manual-Rev-L + Fix-17 + R125-B3
R-Cycle: R125-B3
Last-Modified: 2026-08-10 (R125 B3 16:55 升 25 维)
Status: 🟢 活跃 (技术数据, 数字严守 + 25 维扩展 + 文档结构持续更新)
```

## 3 值 (严守, 数字 0 改)

| 指标 | 值 | 含义 |
|---|---|---|
| **V1141-R11** | **0.8682** | IC-001 fresh 测量 (V0.5 R125 升 25 维 — `crates/apeireth-asi/src/lib.rs:pub const V05_DIM_COUNT: usize = 25`) |
| **V1131-R11** | **0.8532** | dashboard v05_total (R125 升 V0.5 v3 25 维) |
| **V1136-R11** | **0.9063** | 真测引擎 (9 子测度 — `crates/apeireth-asi/src/lib.rs:pub const V1136_SUBMEASURE_COUNT: usize = 9`) |

## V0.5 25 维公式 (R125 B3 升, sum=1.00 守门, 编译期 hardcode enum)

R125-10 Kani 形式化借鉴 + R125-13 LangGraph StateGraph 借鉴触发 V0.5 公式扩展:
- **24 维** (R11 实质) — 24 个 V0.5 维度
- **+1 维 Robustness 鲁棒性** (R125 B3 新加, R125-10 Kani 形式化触发) — total 25 维
- 公式: sum=1.00 守门 (per V0.5 公式, R125 升 25 维后 0 改)
- 编译期 hardcode enum (per O-5 不假装)

## R119 当前 R-Measure 状态

- README badge: 0.92 (R-Measure 0.92 success)
- R-Method 实际: 0.92 (R38 1.1 RC 后)
- R-Measure 增量: 0.9063 → 0.92 (+0.014, 跟 9 organ + 8 LOCKED 落实 + R38 B1-B9 增量一致)
- **R125 升 25 维后**: 0.92 → ? (R125-10 实施时, Robustness 维增量, 0 改 baseline 3 值)

## R-Measure 守门原则

- 🔒 严守 R11 baseline 3 值 (不动)
- 🟢 后续 R-Measure 重测 (R38 R54 R70-R72 R78-R113 R114-R118 R125-10 R125-13) = 在 baseline 之上增量
- 0 重写 R11 baseline
- 0 假装 0 漂移
- **R125 B3 升 25 维**: 0 改 baseline 3 值, 0 改 V0.5 公式 sum=1 守门, 0 改 V1136 9 子测度

## 不漂移 (R125 B3 升 25 维)

- 🔒 24 LOCKED crate mtime 16:34 之前
- 🔒 workspace.version 1.1.0 (R125 末 B2 升 1.2.0)
- 🔒 R11 baseline 3 值 (数字 0 改, A1 严守)
- 0 改 6 哲学锚 (R125 末 B5 升 8 锚)
- 0 改 9 organ 文件名 + 入口签名 (B7 内部借)
- 0 改 12 键 (R125-12 后新增 PHL-07 = 13 键)
