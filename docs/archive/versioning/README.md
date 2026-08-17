# Apeireth 版本号系统 — 7 子系统

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从顶层 APEIRETH-VERSIONING.md 12.8KB 拆为 9 文件目录结构。核验后写:workspace.version 1.1.0 (semver 严守), Fix-3..Fix-17 (R114-R118 加 Fix-15/16/17), Manual-Rev-A..L, R 周期 R11-R118, 指标 V0.5/V1136/V1331/V0172/V0164/V0180 等。

```
[Document-Meta]
Document: docs/versioning/README.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Commit: R119-3a-1 后续
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 7 子系统索引(8 子文件 + 1 README = 9 文件)

| # | 文件 | 主题 | 状态 |
|---|---|---|---|
| 01 | [`01-code.md`](01-code.md) | 主代码版本 (semver, `Apeireth-MAJOR.MINOR.PATCH`) | 🟢 |
| 02 | [`02-design.md`](02-design.md) | 设计层版本 (`Design-X.Y`, 阶段 1-5 + omnibus) | 🟢 |
| 03 | [`03-fix.md`](03-fix.md) | 修正链版本 (`Fix-N`, Fix-3..Fix-17) | 🟢 |
| 04 | [`04-r-cycle.md`](04-r-cycle.md) | R 周期版本 (`R-N`, R11-R118) | 🟢 |
| 05 | [`05-metric.md`](05-metric.md) | 指标版本 (`V<n>`, V0.5/V1136/V1331/V0172/V0164/V0180) | 🟢 |
| 06 | [`06-snapshot.md`](06-snapshot.md) | 基线快照 (`snap-<hash>`) | 🟢 |
| 07 | [`07-manual.md`](07-manual.md) | 手册修订 (`Manual-Rev-X`, A..L) | 🟢 |
| 08 | [`08-document-meta.md`](08-document-meta.md) | Document-Meta 格式 (跨系统) | 🟢 |

## 7 子系统一句话定位

Apeireth = **7 个独立子系统** + **12 子规范** ([`docs/conventions/`](../conventions/README.md)) + **Document-Meta** + **21 词条术语** ([`docs/glossary/`](../glossary/README.md))。

## 8 项不修改承诺 (R119 形式撤销, 原意保留)

详见 [`docs/conventions/10-locked.md`](../conventions/10-locked.md) §"8 项原意保留"。**workspace.version 1.1.0 严守**, **R11 baseline 3 值严守**, 其他 6 项形式撤销, 原意保留。

## 6 哲学锚穿透

| 锚 | 落实 |
|---|---|
| S-1 | 7 子系统服务 ASI 北极星 |
| S-2 | 核验后写 (per R119 主人 8/10 01:14) |
| O-2 | 借鉴 semver / Linux kernel / Rust crate / Cargo workspace version |
| O-3 | 7 子系统独立编号, 不重置 (1.1.0 / 1.1.2-R72 / 1.1.0-R114) |
| O-4 | 8 文件目录索引 + 跳 |
| O-5 | workspace.version 1.1.0 严守, R11 baseline 3 值严守 |

---

_本目录由 Mavis R119-3a-1 重建,原 APEIRETH-VERSIONING.md 12.8KB 7 子系统全拆为 8 文件。核验:R114-R118 状态同步, Fix-3..Fix-17 实际, Manual-Rev-A..L 实际, R 周期 R11-R118 实际。_
