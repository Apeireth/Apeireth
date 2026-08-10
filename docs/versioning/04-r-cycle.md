# 04 R 周期版本 (R-N)

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-VERSIONING.md §4 拆出,核验后写。

```
[Document-Meta]
Document: docs/versioning/04-r-cycle.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`R-N` (R 后跟数字, 表示 R 周期)

## R 周期清单 (核验后, R11-R118)

| R | 周期 | 状态 |
|---|---|---|
| R11 | R11 baseline | 🔒 归档 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063 LOCKED) |
| R12 | R12 接手 | 🔒 归档 |
| R13 | R13 MVP | 🔒 归档 |
| R14 | R14 Rust 重写 | 🔒 归档 |
| R17 | R17 战役 0-4 收官 | 🔒 归档 |
| R23 | R23 baseline 24 LOCKED | 🔒 归档 |
| R33-R37 | LangGraph + AutoGen + MCP + RFC 8628 | 🔒 归档 |
| R38 | R38 1.1 RC 9 B-stage 一气呵成 | 🟡 1.1 主轴归档 |
| R46-R53 | 1.1.1 follow-up (5 R: mini-redis + cognition_graph + docs index + CI summary + README badges) | 🟡 归档 |
| R54 | B8 续升级: backend 真接 mid/long_term + cognition wire-up + render 0 假装 | 🟡 归档 (1.1.2 patch) |
| R57-R62 | 1.1.2 follow-up-2 (Cargo audit + RUSTSEC 续) | 🟡 归档 |
| R63-R68 | 1.2 candidate (5 既有 crate 加 submodule) | 🟡 归档 |
| R70-R72 | 1.2 patch LIVE (LIVE MiniMax 7 model + MCP subscribe push) | 🟡 归档 |
| R78-R113 | 1.2 patch LIVE 续 (11 R + 1 LIVE: skills / graph / MCP 真接) | 🟡 归档 |
| R114-R118 | 动态运营层 (codex `5c546a84`) | 🟢 当前 |
| R119 | 文档重建 (Mavis) | 🟢 当前 |

## 当前 (R119 核验)

`R114-R118` (codex) / `R119-3a-1` (Mavis 文档重建)

## 命名规范

- `R-N` (单 R): R11, R12, ..., R118
- `R<N>-R<M>` (范围): R33-R37, R46-R53, R57-R62, R63-R68, R70-R72, R78-R113, R114-R118
- `R<N>-<topic>` (短格式): R38-a, R54-b, R70, R114
- `R<N>-<NN>` (实践): 实际 commit message 用 `round<N>-<NN> (<author>)` 长格式

## 报告命名 (per §5 报告路径)

- `r<N>-<topic>-<date>.md` (per reports/)
- 例: `r78-r113-batch-final-2026-08-10.md` / `r82-live-minimax-8model-results.md`

## R119 原则 (主人 8/10 01:14)

> "技术发展史可以不要, 我们就要思想历史 + 最新技术文档"

- ✅ R114-R118 保留 (最新技术)
- ✅ 思想层 R11 / R14 / R17 / R23 保留 (LOCKED)
- 🟡 R-Round 报告链 (R38 / R54 / R70-R72 / R78-R113) 按"思想历史"原则筛选 (Fix 链保留, R-Round 报告按需保留)

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
