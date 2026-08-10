# Apeireth 规范系统 — 12 子规范

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从顶层 APEIRETH-CONVENTIONS.md 12.8KB 拆为 14 文件目录结构。核验实际:workspace 90+ crate, 12 ADR (R20 阶段 6 重排 0001-0012) + 3 配套 (0013-0015) + 1 (0018), 25+ reports 命名, 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5) 严守。

```
[Document-Meta]
Document: docs/conventions/README.md
Version: Manual-Rev-L + Fix-17 (R119-3a-1 重建)
R-Cycle: R119-3a-1
Commit: R119-3a-1 后续
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 12 子规范索引(13 子文件 + 1 README = 14 文件)

| # | 文件 | 主题 | 状态 |
|---|---|---|---|
| 01 | [`01-namespace.md`](01-namespace.md) | 命名空间系统 (V<n> / Design-X.Y / Fix-N / R-N / Manual-Rev-X / A<n> / ADR-NNNN / snap-<hash> / D<n> / P<n> / round<N>-<NN>) | 🟢 |
| 02 | [`02-path.md`](02-path.md) | 路径系统 (核验:90+ crate, 24 子目录) | 🟢 |
| 03 | [`03-adr.md`](03-adr.md) | ADR 编号系统 (核验:12 + 4 重排前 + 21 archive = 27+21) | 🟢 |
| 04 | [`04-achievement.md`](04-achievement.md) | 成就编号系统 (A1-A20) | 🟢 |
| 05 | [`05-report.md`](05-report.md) | 报告路径系统 (核验:实际 13 种类型) | 🟢 |
| 06 | [`06-commit.md`](06-commit.md) | Commit message 规范 (核验:6 哲学锚) | 🟢 |
| 07 | [`07-hash.md`](07-hash.md) | Commit hash 引用系统 (`<short-hash>` / `snap-<hash>`) | 🟢 |
| 08 | [`08-status.md`](08-status.md) | 状态标记系统 (🔒 LOCKED / 🟢 / 🟡 / 🔴) | 🟢 |
| 09 | [`09-anchor.md`](09-anchor.md) | 主哲学 6 锚穿透系统 (S-1/S-2/O-2/O-3/O-4/O-5) | 🟢 |
| 10 | [`10-locked.md`](10-locked.md) | 8 项不修改承诺 (R119 形式撤销, 原意保留) | 🟢 |
| 11 | [`11-baseline.md`](11-baseline.md) | R-Measure baseline 3 值 (V1141/V1131/V1136) | 🔒 LOCKED |
| 12 | [`12-arch-diagram.md`](12-arch-diagram.md) | 架构图编号系统 (P1-P5) | 🟢 |
| 13 | [`13-document-meta.md`](13-document-meta.md) | Document-Meta 元信息格式 | 🟢 |
| 14 | [`14-correction-chain.md`](14-correction-chain.md) | 修正链 v3-v17 (思想历史保留) | 🟢 |

## 12 子规范一句话定位

Apeireth = **12 个子规范系统** + **7 个版本号子系统** ([`docs/versioning/`](../versioning/README.md)) + **1 个 Document-Meta 元信息格式** + **21 词条术语** ([`docs/glossary/`](../glossary/README.md))。

## 8 项不修改承诺 (R119 形式撤销)

8 项不修改承诺 (R11 末 / R20 阶段 6 / 8-locked-unified §2)在 **R119 由 Mavis 拍板形式撤销**,原意保留,详见 [`10-locked.md`](10-locked.md):

- 阶段 1+2+3 LOCKED — 形式撤销,原意保留(`docs/omnibus/stage1-3/` 严守)
- v2 / v4 / v4.1 LOCKED — 形式撤销,原意保留(`docs/omnibus/design-v*` 严守)
- 阶段 4 核心 LOCKED — 形式撤销,原意保留(`docs/omnibus/stage4/` 严守)
- 阶段 5 施工 LOCKED — 形式撤销,原意保留(`docs/omnibus/stage5/` 严守)
- v6 基础架构 — 形式撤销,原意保留
- R11 baseline 3 值 — **严守,数据不动**
- 顶层 3 规范文件 — 形式撤销(已下沉到 `docs/conventions/` + `versioning/` + `glossary/`)
- workspace.version = 1.1.0 — **严守,semver 严守**

## 6 哲学锚穿透

| 锚 | 落实 |
|---|---|
| S-1 北极星 | 12 子规范服务 ASI 北极星 |
| S-2 实事求是 | 核验后写(per R119-3a-1 主人 8/10 01:14 拍板"实际可能有偏差") |
| O-2 走在前人肩上 | 借鉴 semver / Linux kernel / Rust crate / VCP vcptoolbox / LangGraph / AutoGen / MCP / LSP / GitHub Releases |
| O-3 干到底 | 12 子规范统一 |
| O-4 任何人都能接手 | 14 文件目录索引 + 跳 |
| O-5 不假装 | 8 项形式撤销,原意保留,实际核验 |

---

_本目录由 Mavis R119-3a-1 重建,原 APEIRETH-CONVENTIONS.md 12.8KB 12 子规范全拆为 14 文件。核验:R114-R118 状态同步,workspace 90+ crate 实际,12+ ADR 实际,R 周期 R11-R118 实际,Fix 链 Fix-3..Fix-17 实际。_
