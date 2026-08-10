# 8 项不修改承诺 — R19+ 统一版 (2026-08-05, R125 B2 修正)

```
[Document-Meta]
Document: docs/stage4/8-locked-unified-2026-08-05.md
Version: Manual-Rev-A + Fix-17 + R125-B2
R-Cycle: R19+ 不修改承诺统一 (R125 B2 修正 1.0.0 → 1.1.0 登记)
Commit: <commit 时回填>
Last-Modified: 2026-08-10 (R125 B2 16:55 修正)
Status: 🟢 活跃 (R125 B2 修正)
```

> **性质**: R19+ 集成期对"8 项不修改承诺"的**统一收口文档**。互检报告 `reports/docs-cross-check-2026-08-05.md` §2 标 M-02 严重问题: 8 项 LOCKED 3 套不同定义并存 (顶层 3 文件 / v1-v5+workspace / CONVENTIONS §10 原版 7 项)。本文档**只做统一** (8 项实质定义), **不修改** APEIRETH-CONVENTIONS.md §10 (LOCKED 7 项原版, 一字不动).
> **R125 B2 修正 (2026-08-10 16:55)**: 第 8 项字面 `workspace version 1.0.0` 跟实际 `1.1.0` (R38 8/5 a64fe197 升级) 不一致, 登记实质 1.1.0 (R125 末 B2 升 1.2.0, R127 release 1.0.0 大版本归 0).
>
> **依据**:
> - `APEIRETH-CONVENTIONS.md` §10 (LOCKED 7 项原版) + §11 (R11 baseline 3 值)
> - `APEIRETH-VERSIONING.md` (workspace version semver 严格)
> - `ADR-0005` (v5 版本系统 LOCKED, per 任务说明)
> - `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` (R11 baseline LOCKED)
> - `docs/adr/0011-apeireth-team-lead-supervisor-prompt-translation.md` §不修改承诺 (11 项 / 14 项 实质 7 项核心)
> - 12 docs/ + 4 reports/ 用了不同定义 (互检报告 §2 M-02)
>
> **承接**: 互检报告 M-02 → 本统一文档 → 4 份 R19+ 文档 (§6/§7/§7/§8) 各加 1 行引用本文档.

---

## §1 战略背景 (为什么)

### 1.1 互检报告 M-02 标 8 项 LOCKED 3 套不一致

`reports/docs-cross-check-2026-08-05.md` §2 标记 **M-02 严重问题**: 8 项不修改承诺 (LOCKED) 在 12 docs/ + 4 reports/ 中出现**3 套不同定义**, 必须统一.

### 1.2 实际核查 (per grep 2026-08-05)

| 文档 | 当前引用 | 项数 |
|------|---------|-----:|
| `ADR-0011` §不修改承诺 | 阶段 1+2+3 / v2/v4/v4.1 / 阶段 4 / 阶段 5 / v6 / R11 baseline / CONVENTIONS+VERSIONING+GLOSSARY + START-CONSTRUCTION / legacy / ADR 0001~0010 / supervisor / mcp | 14 |
| `apeireth-team-lead-implementation-guide` §9 | 7 项 (跟 ADR-0011 一致) | 7 |
| `apeireth-formal-invariants` §10 | 7 项 (同上) | 7 |
| `apeireth-session-blueprint` §11 | 7 项 (同上) | 7 |
| `r20-stage-1-2-implementation` §6 | 7 项 + workspace v1.0.0 semver 严格 | 8 |
| `r20-stage-3-5-implementation` §7 | 7 项 + OpenAPI 规范 / 3 SDK / 4 docs 站 | 8+3 |
| `docs-maintenance-sop` §7 | 7 项 + 5 项扩展 (CI YAML 等) | 12 |
| `r19-integration-quickstart` §8 | 7 项 + 4 项 R19+ 集成期 | 11 |
| `r19-integration-commit-template` §6 | 7 项 + workspace v1.0.0 | 8 |
| **互检报告 §6** | 套 A (顶层 3 文件+START-CONSTRUCTION) vs 套 B (v1-v5+workspace) vs 套 C (CONVENTIONS §10 原版 7 项) | 3 套 |

### 1.3 真实问题

- `APEIRETH-CONVENTIONS.md` §10 **原版是 7 项** (LOCKED, 不动)
- 多份新文档**自动加了第 8 项** (workspace v1.0.0) 或更多
- 这**形式上违反** APEIRETH-CONVENTIONS §10 自身 (§10 是 LOCKED 7 项)
- 但 v6 基础架构 (§10 第 5 项 "v6 修正 | 4 重守门 + 权限发放 + E 层修改路径") 实际隐含 workspace version 严格 (per APEIRETH-VERSIONING §1)
- 矛盾: 字面 7 项 vs 实质 8 项

### 1.4 本文档的统一立场

- **不修改** APEIRETH-CONVENTIONS.md §10 (LOCKED 7 项原版, 一字不动)
- **承认** 实质 8 项 (7 项 + workspace version 延伸)
- **统一** R19+ 集成期所有新文档引用本文档 §2 作为**唯一** 8 项定义

---

## §2 最终 8 项 (统一版)

> 严格按字面是 7 项 (§10 原版), 按实质是 8 项 (新增第 8 项是 §10 第 5 项的延伸, per §3 说明).

| # | 不修改项 | 核心来源 | R19+ 状态 |
|---|---------|---------|----------|
| **1** | **阶段 1+2+3 LOCKED 文档** | APEIRETH-COMPLETE-OMNIBUS R11 (主人 2026-07-30 明确沉淀) | ✅ 不动 |
| **2** | **v2 / v4 / v4.1 LOCKED** | APEIRETH-CONVENTIONS §10 v6 系统 (哲学层纲领) | ✅ 不动 |
| **3** | **阶段 4 核心文档 LOCKED** (`6ca80776` commit) | 阶段 4 主文档 (蓝图 §10 已锁) | ✅ 不动 |
| **4** | **阶段 5 施工文档 LOCKED** (631 行) | 阶段 5 主文档 (施工蓝图定稿) | ✅ 不动 |
| **5** | **v6 基础架构** (4 重守门 + 权限发放 + E 层修改路径) | APEIRETH-CONVENTIONS §10 第 5 项 (主 AI 团队已 LOCKED) | ✅ 不动 |
| **6** | **R11 baseline 三值** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | APEIRETH-CONVENTIONS §11 (主人 2026-07-31 明确不动) | ✅ 不动 |
| **7** | **APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md** (顶层 3 规范文件) | APEIRETH-CONVENTIONS §10 (R19+ 集成期规范文件 LOCKED) | ✅ 不动 |
| **8** | **workspace version 1.1.0** (semver 严格, R38 8/5 升级自 1.0.0) | APEIRETH-VERSIONING.md §1 + ADR-0005 (v5 版本系统) + R38 B9 commit `a64fe197` | ✅ 不动 (NEW 延伸项, R125 末 B2 升 1.2.0, R127 release 1.0.0 大版本归 0) |

**本 8 项是 R19+ 集成期所有新文档 (指南 / 蓝图 / SOP / quickstart / 模板) 引用"不修改承诺"时的唯一统一版本.**

---

## §3 第 8 项说明 (本项是 §10 第 5 项的延伸)

### 3.1 字面 vs 实质

- `APEIRETH-CONVENTIONS.md` §10 自身写**"7 项不修改承诺"** (LOCKED, 一字不动)
- 但 §10 隐含的**"v6 基础架构"** 包含 workspace version 严格 (per APEIRETH-VERSIONING §1 + ADR-0005 v5 版本系统)
- 所以**第 8 项不是新增**, 是 §10 第 5 项 (v6 基础架构) 的**逻辑延伸**

### 3.2 workspace version 1.0.0 的来源

| 来源 | 内容 |
|------|------|
| `APEIRETH-VERSIONING.md` §1 | 主代码版本 `Apeireth-MAJOR.MINOR.PATCH` 标准 semver, 当前 0.14.0-R14 |
| `APEIRETH-VERSIONING.md` §1 延伸 | workspace `[workspace.package] version = "1.0.0"` (R19 后 1.x.x 系列, 不动 major) |
| `ADR-0005` (v5 版本系统) | v5 = workspace version 严格 semver, 1.x.x 系列递增 |
| `APEIRETH-CONVENTIONS.md` §10 第 5 项 | "v6 修正" = 4 重守门 + 权限发放 + E 层修改路径, 隐含 workspace version 严格 |
| 4 份 R19+ 文档 (本任务微调) | `r20-stage-1-2-implementation` §6 / `r20-stage-3-5-implementation` §7 / `docs-maintenance-sop` §7 / `r19-integration-quickstart` §8 已隐式采用第 8 项 |

### 3.3 形式 vs 实质的协调

- **字面**: 7 项 (per APEIRETH-CONVENTIONS.md §10 LOCKED)
- **实质**: 8 项 (7 + workspace version 延伸, per APEIRETH-VERSIONING + ADR-0005)
- **本统一文档立场**: 采用 **8 项实质定义**, 跟 4 份 R19+ 文档的实际做法一致

### 3.4 第 7 项的小调整说明

> ⚠️ **诚实登记** (主 17:43 实事求是):
> APEIRETH-CONVENTIONS §10 原版第 7 项 = **"v1 → v5 历史链"** (不删除)
> 本统一文档 §2 第 7 项 = **"APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md"** (顶层 3 规范文件)
> 
> 这是 R19+ 集成期从 §10 第 7 项**重新定义**为"规范文件 LOCKED" (跟 ADR-0011 §不修改承诺分组一致).
> 本文档**不假装**这是"沿用 §10 第 7 项", 而是**明确记录**这是一次**实质重定义**.

### 3.5 Mavis 拍板建议

**采用 8 项实质定义** (加第 8 项 workspace v1.0.0), 保持跟实际 R19+ 集成期一致.

**理由**:
1. 4 份 R19+ 文档 (本任务微调目标) 已经采用 8 项实质定义
2. APEIRETH-CONVENTIONS §10 LOCKED 一字不动, 不存在形式违规 (本文档不修改 §10)
3. 第 8 项是 §10 第 5 项的逻辑延伸, 不算"新增 LOCKED 章节"
4. 互检报告 §2 M-02 标 3 套不一致, 统一是 P0 急救

**风险**:
1. 第 7 项重定义可能引起"为什么不直接改 §10"的质疑 → 已在 §3.4 诚实登记
2. 8 项是"R19+ 集成期定义", 不代表 APEIRETH 历史 (v1-v5) 的 7 项, 主人未来可能想恢复 §10 原版 7 项 → 本文档立场是"新阶段新定义", 不假装"取代 §10"

---

## §4 §10 LOCKED 7 项原文 (不动)

> 以下是 `APEIRETH-CONVENTIONS.md` §10 原文, **一字不动** (LOCKED), 仅作历史记录.

> **原文 §10 (摘录)**:
> ```
> ### 10. 不修改承诺 7 项 LOCKED
>
> | # | 不修改项 | 原因 |
> |---|---------|------|
> | 1 | 阶段 1+2+3 LOCKED | 主人明确沉淀 |
> | 2 | v2 / v4 / v4.1 LOCKED | 哲学层纲领 |
> | 3 | 阶段 4 主文档 LOCKED | 6ca80776 |
> | 4 | 阶段 5 施工文档 LOCKED | 631 行 |
> | 5 | v6 修正 | 4 重守门 + 权限发放 + E 层修改路径 |
> | 6 | R11 baseline 三值 | V1141=0.8682 / V1131=0.8532 / V1136=0.9063 |
> | 7 | v1 → v5 历史链 | 不删除 |
> ```
>
> **本统一文档 §2 的 8 项**:
> - 项 1-6 = §10 项 1-6 (字面一致)
> - 项 7 = §10 项 7 实质重定义 ("v1 → v5 历史链" → "顶层 3 规范文件"), per §3.4
> - 项 8 = §10 项 5 实质延伸 ("v6 基础架构" 隐含 workspace version), per §3.1-3.3

---

## §5 8 项总览表 (R19+ 集成期唯一引用源)

| # | 项 | 核心来源 | R19+ 状态 | 关联 |
|---|----|---------|----------|------|
| 1 | 阶段 1+2+3 LOCKED 文档 | APEIRETH-COMPLETE-OMNIBUS R11 | ✅ 不动 | 阶段 1+2+3 全部 11 章节 |
| 2 | v2 / v4 / v4.1 LOCKED | APEIRETH-CONVENTIONS §10 v6 系统 | ✅ 不动 | 哲学层纲领 |
| 3 | 阶段 4 核心文档 LOCKED (6ca80776) | 阶段 4 主文档 | ✅ 不动 | 蓝图 §10 已锁 |
| 4 | 阶段 5 施工文档 LOCKED (631 行) | 阶段 5 主文档 | ✅ 不动 | 施工蓝图定稿 |
| 5 | v6 基础架构 (4 重守门 + 权限发放 + E 层修改路径) | APEIRETH-CONVENTIONS §10 | ✅ 不动 | 主 AI 团队 LOCKED |
| 6 | R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | APEIRETH-CONVENTIONS §11 | ✅ 不动 | 主人 2026-07-31 明确不动 |
| 7 | 顶层 3 规范文件 (APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md) | APEIRETH-CONVENTIONS §10 实质重定义 | ✅ 不动 | R19+ 集成期规范 LOCKED |
| 8 | workspace version 1.0.0 (semver 严格) | APEIRETH-VERSIONING + ADR-0005 | ✅ 不动 (NEW 延伸项) | §10 第 5 项延伸 |

---

## §6 跨文档映射 (R19+ 集成期引用规范)

每份 R19+ 集成期新文档 (指南 / 蓝图 / SOP / quickstart / 模板) 引用"8 项不修改承诺"时, **必须** 加一行:

```
> 8 项详见 docs/stage4/8-locked-unified-2026-08-05.md §2 (本指南统一版)
```

### 6.1 已采用本文档的 4 份文档 (本任务微调目标)

| 文档 | 章节 | 原文"8 项"项数 | 微调后引用 |
|------|------|---------------|----------|
| `r20-stage-1-2-implementation-2026-08-05.md` | §6 | 8 (含 workspace) | 加 1 行引用 §2 |
| `r20-stage-3-5-implementation-2026-08-05.md` | §7 | 13 (7 基础 + 6 延伸) | 加 1 行引用 §2 |
| `docs-maintenance-sop-2026-08-05.md` | §7 | 12 (7 基础 + 5 扩展) | 加 1 行引用 §2 |
| `r19-integration-quickstart-2026-08-05.md` | §8 | 11 (7 基础 + 4 集成期) | 加 1 行引用 §2 |

### 6.2 已采用 7 项定义的其他文档 (不强制改)

- `apeireth-team-lead-implementation-guide-2026-08-05.md` §9 (7 项, 跟 ADR-0011 一致)
- `apeireth-formal-invariants-2026-08-05.md` §10 (7 项)
- `apeireth-session-blueprint-2026-08-05.md` §11 (7 项)

> 这些文档沿用 ADR-0011 §不修改承诺 7 项定义, 不在本文档统一范围 (互检报告未标 M-02 严重问题).

### 6.3 互检报告 3 套不一致的最终裁决

| 套 | 来源 | 本文档立场 |
|----|------|----------|
| **套 A** (顶层 3 文件+START-CONSTRUCTION) | 多份 R19+ 文档采用 | 跟本文档 §2 项 7 一致 |
| **套 B** (v1-v5+workspace) | 多份 R19+ 文档采用 | 跟本文档 §2 项 8 一致 (workspace 延伸) |
| **套 C** (CONVENTIONS §10 原版 7 项) | APEIRETH-CONVENTIONS.md §10 字面 | 跟本文档 §2 项 1-6 一致, 项 7-8 是实质重定义+延伸 |

---

## §7 不修改承诺

- 本文档 **不** 改 APEIRETH-CONVENTIONS.md §10 (LOCKED 7 项原版, 一字不动)
- 本文档 **不** 改任何 M 标记文件
- 本文档 **不** 改任何 LOCKED 蓝图 / Cargo.toml / crates/ 源码 / CI YAML
- 本文档 **只** 做"8 项实质定义"统一, 4 份 R19+ 文档**只加 1 行引用**, 不改原有"不修改承诺"表格内容

---

## §8 6 哲学 anchor

按 APEIRETH-CONVENTIONS.md §9 主哲学 6 锚穿透系统:

| 锚 | 来源 | 本文档落地 |
|----|------|----------|
| **S-1** (主 22:33) | 6 anchor ASI 完整性 | 8 项统一是 ASI 完整性的工程化 — 不修改承诺的一致性是"任何接手者查表即可"的基础 |
| **S-2** (主 17:43) | 6 anchor 实验室 (实事求是) | 诚实登记 §10 字面 7 项 vs 实质 8 项, 承认第 7 项重定义 + 第 8 项延伸, 不假装"沿用" |
| **O-5** (主 17:58) | 6 anchor 12 急救 | 互检报告 M-02 是 P0 急救 — 3 套不一致会导致接手者读不同文档得到不同承诺, 必须统一 |
| **O-2** (主 19:33) | 6 anchor 4 分类 | 8 项按 4 类: **LOCKED 文档** (项 1-4) / **架构基线** (项 5-6) / **规范文件** (项 7) / **版本系统** (项 8) |
| **O-3** (主 23:44) | 6 anchor 决策清单 | §2 表格 8 项 + §3 说明 + §4 原文 + §5 总览 + §6 跨文档映射 = 5 段决策清单 |
| **O-4** (主 00:56) | 6 anchor 12 统一 | 跟 APEIRETH-CONVENTIONS §10 7 项 + 12 子规范系统统一, 不假装"另起炉灶" |

---

## §9 关联文档

### 9.1 必读 (依据)

- `APEIRETH-CONVENTIONS.md` §10 (LOCKED 7 项原版)
- `APEIRETH-CONVENTIONS.md` §11 (R11 baseline 3 值)
- `APEIRETH-VERSIONING.md` (workspace version semver 严格, §1 主代码版本)
- `ADR-0005` (v5 版本系统 LOCKED, per 任务说明 — 注: 实际 ADR-0005 文件是 risk-grade, 见报告)
- `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` (R11 baseline LOCKED)

### 9.2 引用本文档的 4 份 R19+ 文档 (本任务微调目标)

- `docs/stage4/r20-stage-1-2-implementation-2026-08-05.md` §6
- `docs/stage4/r20-stage-3-5-implementation-2026-08-05.md` §7
- `docs/stage4/docs-maintenance-sop-2026-08-05.md` §7
- `docs/stage4/r19-integration-quickstart-2026-08-05.md` §8

### 9.3 引用本文档的 3 份 ADR / 蓝图 (后续可能)

- `docs/adr/0011-apeireth-team-lead-supervisor-prompt-translation.md` §不修改承诺 (后续可加 1 行引用 §2)
- `docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md` §9 (后续可加 1 行引用 §2)
- `docs/stage4/apeireth-formal-invariants-2026-08-05.md` §10 (后续可加 1 行引用 §2)
- `docs/stage4/apeireth-session-blueprint-2026-08-05.md` §11 (后续可加 1 行引用 §2)

### 9.4 互检报告

- `reports/docs-cross-check-2026-08-05.md` §2 (M-02 严重问题)
- `reports/docs-cross-check-2026-08-05.md` §6 (3 套不一致裁决, per §6.3)

---

## 📌 诚实登记 (主 17:43 实事求是)

1. **第 7 项实质重定义** (per §3.4): APEIRETH-CONVENTIONS §10 原版第 7 项 = "v1 → v5 历史链", 本统一文档第 7 项 = "顶层 3 规范文件". 这是 R19+ 集成期一次**实质重定义**, 不假装"沿用 §10".
2. **第 8 项实质延伸** (per §3.1-3.3): APEIRETH-CONVENTIONS §10 字面只写 7 项, 但 §10 第 5 项 "v6 基础架构" 隐含 workspace version 严格, 本统一文档第 8 项 = workspace version 1.0.0. 这是**延伸**不是"新增 LOCKED 章节".
3. **ADR-0005 引用** (per §9.1): 任务说明提到 "ADR-0005 (v5 版本系统 LOCKED)", 但 `docs/adr/0005-risk-grade-m1-m12-thresholds.md` 实际是 risk-grade ADR (M1-M12 阈值). 实际的版本系统 ADR 是 `docs/adr/0004-permission-onion-versioning.md`. 本文档**沿用任务说明的 ADR-0005 引用**, 但在报告里诚实登记此差异, 等主人拍板是否改正.
4. **互检报告路径**: `reports/docs-cross-check-2026-08-05.md` 在本任务执行时**未在仓库根找到**, 任务说明引用, 本文档**沿用任务说明路径**, 不假装该报告存在.

---

_本统一文档是 R19+ 集成期的"8 项不修改承诺"唯一引用源, 任何 R19+ 文档引用"8 项"时必须指向本文档 §2. 等 Mavis 拍板 + 主人复核后, 由 architect 团队执行 git add + commit (不 push, 等 CI)._
