# 1.0 release 根 README 续补 — E-1 ~ E-8 草稿索引

```
[Document-Meta]
Document:       docs/1.0-release-prep/README.md
Version:        R20-Rev-A
R-Cycle:        R20 阶段 6 — 1.0 release 12 项 #1 doc 续补 (续 #1 doc 30% 报告)
Last-Modified:  2026-08-06
Status:         🟢 7 草稿 + 1 真实文件 (E-6 roadmap 写 docs/roadmap/ 真实子节文件)
Author:         Mavis (Mavis@local)
Originated:     主人 2026-08-06 01:14 拍 "后续有需要决定的都按 Mavis 想法倾向来"
Target:         接手者 + Mavis 整合 #3 拍板 (本目录不动根 README, 等主人解除 LOCKED)
```

> **性质**: 本目录是 **1.0 release 12 项 #1 doc 续补 (E-1 ~ E-8) 的草稿合集**。
> 续 `reports/1.0-release-doc-30-2026-08-06.md` (bg_2db4f73e 跑完的 30% 续补验证报告, 总评 85%)。
>
> **为什么不直接改根 README.md**:
> - 根 README.md mtime 2026/8/5 21:08:33 = 🔒 **LOCKED baseline** (per 主人 22:13 拍 "1.0 release 暂缓, #1 doc 该补就补")
> - 根 CHANGELOG.md mtime 2026/8/5 21:32:31 = 🔒 **LOCKED baseline**
> - 任何 Mavis sub-agent **不主动 commit** 触碰 LOCKED 文件 (留 Mavis 整合 #3 拍板)
> - **本目录 = 草稿合集**, 等 Mavis 整合 #3 拍板后, 由 R21 sub-agent 一次性合入根 README
>
> **6 哲学锚穿透** (per `APEIRETH-CONVENTIONS.md` §9):
> - **S-1** 北极星: 1.0 release 续补仍服务"接手者 1 跳可达" 目标
> - **S-2** 实事求是: 8 项草稿全部基于实查 (DEPENDENCY §2 / 1.0-release/README.md §1 / ROADMAP.md §R20 阶段 6 / CONTRIBUTING.md / THIRD-PARTY-NOTICES.md)
> - **O-2** 前人肩上: 8 节草稿全引 6 哲学锚 (0010-6-philosophy-anchors.md) + 8 项承诺 (8-locked-unified-2026-08-05.md) + cosign 借鉴 sigstore + 8 包借鉴 业界主流
> - **O-3** 干到底: 8 项草稿 = 8 文件 (1 索引 + 6 草稿 + 1 真实文件)
> - **O-4** 接手可达: 草稿合 Markdown, 接手者读本目录即可知 1.0 release 续补内容
> - **O-5** 不假装: 草稿明确标 "**根 README.md LOCKED 严守, 本目录仅草稿**"

> **8 项不修改承诺**: 详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2 (本目录严守)
> - 第 1 项 阶段 1+2+3 LOCKED 文档: 0 改
> - 第 2 项 v2 / v4 / v4.1 LOCKED: 0 改
> - 第 3 项 阶段 4 核心文档 LOCKED (`6ca80776`): 0 改
> - 第 4 项 阶段 5 施工文档 LOCKED (631 行): 0 改
> - 第 5 项 v6 基础架构 LOCKED: 0 改
> - 第 6 项 R11 baseline 3 值 (V1141/V1131/V1136): 0 改
> - 第 7 项 顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY): 0 改
> - 第 8 项 workspace version 1.0.0 (semver 严守): **0 改** (本任务仅新增 docs/1.0-release-prep/ 7 文件 + docs/roadmap/ 1 文件, 0 改 Cargo.toml)

---

## §0. TL;DR

续补 8 项小缺 (E-1 ~ E-8), **不**直接改根 README (LOCKED) / 根 CHANGELOG (LOCKED) / Cargo.toml workspace version (LOCKED)。**改放 `docs/1.0-release-prep/` 草稿合集 + `docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md` 1 个真实子节文件**。

| 缺项 | 主题 | 文件 | 状态 |
|------|------|------|:----:|
| **E-1** | 根 README 缺"快速开始"节 | [`01-quick-start.md`](./01-quick-start.md) | ✅ 草稿 |
| **E-2** | 根 README 缺"借鉴"节 | [`02-borrow.md`](./02-borrow.md) | ✅ 草稿 |
| **E-3** | 根 README 缺"引用"节 | [`03-citation.md`](./03-citation.md) | ✅ 草稿 |
| **E-4** | 根 README 没"贡献"明文入口 | [`04-contribution.md`](./04-contribution.md) | ✅ 草稿 |
| **E-5** | 根 README 没跳 docs/1.0-release/ | [`05-1.0-release-link.md`](./05-1.0-release-link.md) | ✅ 草稿 |
| **E-6** | docs/roadmap/ 缺 v1.0.0 release roadmap 子节 | [`../roadmap/v1.0.0-release-roadmap-2026-08-06.md`](../roadmap/v1.0.0-release-roadmap-2026-08-06.md) | ✅ 真实文件 |
| **E-7** | 根 README 缺 1 张三架构 mermaid 图 | [`07-architecture-mermaid.md`](./07-architecture-mermaid.md) | ✅ 草稿 |
| **E-8** | 根 README 没引 1.0 release 13 收口文档索引 | [`05-1.0-release-link.md`](./05-1.0-release-link.md) §2 | ✅ 草稿 |

**完成度预估**: 8/8 草稿落地 → **#1 doc 整体从 85% 提升到 ~95%** (缺项: 仅剩根 README LOCKED 解除后合入动作, 1 个 sub-agent 0.5h 即可 100% 补完)。

---

## §1. 文件清单 (8 文件)

| # | 文件路径 | 大小估 | 主题 |
|---:|---------|------:|------|
| 1 | `docs/1.0-release-prep/README.md` | 200+ | 本索引 |
| 2 | `docs/1.0-release-prep/01-quick-start.md` | 80+ | E-1 根 README "## 🚀 快速开始" 节草稿 (5 分钟跑通) |
| 3 | `docs/1.0-release-prep/02-borrow.md` | 100+ | E-2 根 README "## 🏛️ 借鉴" 节草稿 (P0/P1/P2/P3 + SpectrAI/VCP/Yinta) |
| 4 | `docs/1.0-release-prep/03-citation.md` | 80+ | E-3 根 README "## 📚 引用" 节草稿 (BibTeX + 6 哲学锚) |
| 5 | `docs/1.0-release-prep/04-contribution.md` | 60+ | E-4 根 README "## 🤝 贡献" 节草稿 (1 行入口 + 1 表) |
| 6 | `docs/1.0-release-prep/05-1.0-release-link.md` | 80+ | E-5 + E-8 根 README 1.0 release 13 文档入口 (1 段 + 1 索引表) |
| 7 | `docs/1.0-release-prep/07-architecture-mermaid.md` | 50+ | E-7 根 README 三架构 mermaid 图 (v2 → v4 → v4.1 → 22 trait 互锁) |
| 8 | `docs/roadmap/v1.0.0-release-roadmap-2026-08-06.md` | 200+ | E-6 docs/roadmap/ v1.0.0 release roadmap 子节 (9-30 tag + 14 R20 阶段 commit 时间线) |

**合计**: 8 文件 / ~850 行 / 0 LOCKED 触碰 / 0 改 workspace version / 0 主动 commit。

---

## §2. 合入流程 (Mavis 整合 #3 拍板后)

| 步骤 | 动作 | 工时估 | 责任 |
|------|------|------:|------|
| 1 | 主人解除根 README.md LOCKED (per 主 22:13 拍 "1.0 release 暂缓", 解锁是 Mavis 整合 #3 范畴) | — | 主人 |
| 2 | R21 sub-agent 读本目录 7 草稿, 按 §3 节建议合入根 README 6 节 | 1h | sub-agent |
| 3 | R21 sub-agent 同步更新根 README 1.1 节 + 1.2 节 (LOCKED baseline 校验) | 0.5h | sub-agent |
| 4 | R21 sub-agent 提 1 commit `docs: R21 续 — 根 README 6 节合入 E-1 ~ E-8 (per #1 doc 续补)` | — | sub-agent |
| 5 | 主人 review + 接受 commit, 1.0 release #1 doc 达 100% | — | 主人 |
| **合计** | **R21 sub-agent × 1.5h** | **1.5h** | — |

**注**: 本任务 (Mavis 派) 仅产草稿, 不动根 README. 合入是 R21 续, 取决于主人解除 LOCKED.

---

## §3. 草稿合入建议 (Mavis 整合 #3 拍板用)

> **本节**: 给 R21 sub-agent 的合入路径建议 (Mavis 整合 #3 拍板后用).

### 3.1 根 README 6 节模板 (per 任务规范)

| # | 节 | 草稿文件 | 合入行位 (per `README.md` 1.2 实查) |
|---:|----|---------|------------------------------------|
| 1 | 🎯 介绍 (Introduction) | 已有 (line 68-74 🎯 一句话总结) | — |
| 2 | 🚀 快速开始 (Quick Start) | [`01-quick-start.md`](./01-quick-start.md) | **新增** H2 "## 🚀 快速开始" 在 line 74 后 |
| 3 | 🏗️ 架构 (Architecture) | 已有 (line 210-275) + [`07-architecture-mermaid.md`](./07-architecture-mermaid.md) | 在 line 263 后**插入** mermaid 图 |
| 4 | 🏛️ 借鉴 (Borrow) | [`02-borrow.md`](./02-borrow.md) | **新增** H2 "## 🏛️ 借鉴" 在架构节后 |
| 5 | 📚 引用 (Citation) | [`03-citation.md`](./03-citation.md) | **新增** H2 "## 📚 引用" 在借鉴节后 |
| 6 | 🤝 贡献 (Contribution) | [`04-contribution.md`](./04-contribution.md) | **新增** H2 "## 🤝 贡献" 在引用节后 (或并入 8 套规范节) |

### 3.2 1.0 release 入口插入位 (E-5 + E-8)

| 位置 | 草稿 | 行位 (per 1.2 实查) |
|------|------|---------------------|
| 🆕 Recent Updates 节 (line 20) | [`05-1.0-release-link.md`](./05-1.0-release-link.md) §2 | line 20 后**插入** 1 段 "**1.0 release 收口 13 文档**" |
| 🎯 一句话总结 节 (line 68-74) | [`05-1.0-release-link.md`](./05-1.0-release-link.md) §1 | line 74 后**插入** 1 行 "**1.0 release 完整收口**" |

---

## §4. 0 LOCKED 触碰验证

| LOCKED 文件 | mtime (基线) | 本任务触碰? |
|------------|------------|:-----------:|
| 根 README.md | 2026/8/5 21:08:33 | ✅ 0 触碰 (草稿全在 `docs/1.0-release-prep/`) |
| 根 CHANGELOG.md | 2026/8/5 21:32:31 | ✅ 0 触碰 |
| 根 CONTRIBUTING.md | 2026/8/5 21:23:54 | ✅ 0 触碰 (仅 read 引用) |
| 根 INSTALL.md | 2026/8/2 11:11:24 | ✅ 0 触碰 (仅 read 引用) |
| 根 ROADMAP.md | 2026/8/5 21:04:31 | ✅ 0 触碰 (仅 read 引用 §R20 阶段 6) |
| 根 SECURITY.md | (无 mtime 列) | ✅ 0 触碰 |
| 根 LICENSE | (无 mtime 列) | ✅ 0 触碰 |
| 根 NOTICE | (无 mtime 列) | ✅ 0 触碰 |
| 根 DEPENDENCY | 2026/8/6 1:02:13 | ✅ 0 触碰 (1.0 release #11 收尾已落) |
| Cargo.toml | 2026/8/6 2:00:36 | ✅ 0 触碰 (workspace version 1.0.0 严守) |
| 24 LOCKED crate src/ | (mtime baseline 16:34 之前) | ✅ 0 触碰 (本任务不涉及 Rust 代码) |

---

## §5. 关键诚实标缺 (R21 续 / 主 决策)

1. **根 README.md 6 节合入** 仍需主人解除 LOCKED — Mavis 整合 #3 拍板范畴
2. **根 CHANGELOG.md v1.0.0 release entry** 仍需主人解除 LOCKED — 同上
3. **E-6 docs/roadmap/ v1.0.0 release roadmap** 已落地为真实子节文件 (LOCKED 1 文件例外, 主人可审核)
4. **0 主动 commit** — 8 文件全部 untracked, 留 Mavis 整合 #3 拍板

---

_本目录路径: `docs/1.0-release-prep/`_
_生成时间: 2026-08-06_
_派工来源: Mavis 1.0 release 治理收尾, 续 bg_2db4f73e 跑完的 #1 doc 30% 续补验证报告_
_6 哲学锚穿透 + 8 项不修改承诺 0 触碰 + 0 改 workspace version + 0 主动 commit + 0 sandbox 错路径_
