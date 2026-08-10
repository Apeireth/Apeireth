# 阶段 4 修正 v10 — Apeireth 版本号系统（主人 2026-07-31 关键洞察）

```
[Document-Meta]
Document: docs/stage4/stage4-correction-v10-versioning-system.md
Version: Fix-10 + Design-4.0
R-Cycle: R14
Commit: <latest-commit-hash>
Last-Modified: 2026-07-31
Status: 🟢 active (v10 has Fix-10 meta after v10 versioning)
See: APEIRETH-VERSIONING.md
```


> **性质**: leader 亲自做的**第十次修正**——基于主人 2026-07-31 关键洞察"受外部 agent 启发，是时候给 Apeireth 设计一套版本号了"。
> **触发**: 外部 agent Round 5 指出"v3/v4/v5/v6 修正链 vs v1/v5 修订两种版本号冲突"。
> **精读结果**: 当前有 **6 套版本号系统冲突**（主版本 / 设计层 / 修正链 / R 周期 / 指标 / 基线）。
> **提议**: 7 个独立子系统版本号 + 1 套元信息格式。
> **硬约束**: ❌ 不修改 LOCKED（阶段 1+2+3+4 LOCKED） / ❌ 不修改已 commit 的 LOCKED 文档 / ❌ 不破坏 v1-v9 修正链。
> **主哲学 6 锚穿透**: 主 22:33 北极星 / 主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 00:56 任何人都能接手。

---

## §0. 元信息

| 字段 | 值 |
|---|---|
| **生成时间** | 2026-07-31 |
| **依据** | 主人 2026-07-31 "受外部 agent 启发，是时候给 Apeireth 设计一套版本号了" |
| **性质** | v10 提议（Apeireth 版本号系统） |
| **路径** | Apeireth-rust/docs/stage4/stage4-correction-v10-versioning-system.md |
| **修订链** | v1 → v2 → v3 → v4 → v5 → v6 → v7 → v8 → v9 → **v10（版本号系统）** |

---

## §1. 主人洞察 + 当前冲突盘点

### 1.1 主人原话

> "受此启发，我觉得我们是时候给 Apeireth 也设计一套版本号了，你有什么看法"

### 1.2 当前 6 套版本号系统（冲突盘点）

| # | 用途 | 示例 | 冲突 |
|---|---|---|---|
| 1 | **代码 semver** | `Apeireth-0.14.0`（Cargo.toml）| 与下面都混 |
| 2 | **设计层修正链** | `stage4-correction-v3..v9` | "v3" = 修正链 |
| 3 | **开工手册修订** | `START-CONSTRUCTION v1..v6` | "v3" = 手册 |
| 4 | **R 周期** | R11 / R12 / R13 / R14 | 与代码版本混 |
| 5 | **指标版本** | V0.5 / V1136 / V3 9键 | 独立但类似 |
| 6 | **基线版本** | V1141 / V1131 / V1136 | 数字太相似（V1136 同时是指标和基线）|

**核心问题**："v3" / "v5" / "v6" 指什么？

---

## §2. v10 提议：7 个独立子系统版本号

### 2.1 总览

```
Apeireth 文档树根元信息（顶层任何文档 = 同一格式）：

[子系统版本号 1] + [子系统版本号 2] + ... + [R 周期] + [commit hash]

例: Design-4.0-R14 + Fix-6 (5 重治理 + 4 重守门 + 权限发放 + E 层修改路径) - snap-29d499bb
```

### 2.2 7 个独立子系统（**完全独立，不冲突**）

| # | 子系统 | 格式 | 命名空间 | 例子 | 当前映射 |
|---|---|---|---|---|---|
| **1** | **主代码版本**（semver）| `MAJOR.MINOR.PATCH` | `Apeireth-X.Y.Z` | `Apeireth-0.14.0` | 当前 = 0.14.0 |
| **2** | **设计层版本** | `Design-MAJOR.MINOR` | `Design-X.Y` | `Design-4.0` | 阶段1=D1.0 / 阶段2=D2.0+Design-2.1 / 阶段3=D3.0 / 阶段4=D4.0 / 阶段5=D5.0 |
| **3** | **修正链版本** | `Fix-N` | `Fix-N` | `Fix-9` | v3→Fix-3 / v4→Fix-4 / ... / v9→Fix-9（独立编号）|
| **4** | **R 周期版本** | `R-CYCLE` | `R-N` | `R14` | R11→R12→R13→**R14** |
| **5** | **指标版本** | `V<n>` | `V<n>` | `V0.5` | V0.5 / V1136 / V3 / V1141 / V1131（保留）|
| **6** | **基线快照** | `snap-<commit-hash>` | `snap-<hash>` | `snap-29d499bb` | 保留 commit hash |
| **7** | **手册修订** | `Manual-Rev-X` | `Manual-Rev-X` | `Manual-Rev-F` | v1→A / v2→B / ... / v6→F |

### 2.3 为什么这样分？

**主版本号**（semver）= 标准软件版本实践：
- `0.14.0` → `0.15.0` = 新 MINOR（新增器官 crate）
- `0.14.0` → `0.14.1` = PATCH（修 bug）
- `1.0.0` = 真正可发布

**设计层版本**（Design-X.Y）：
- 阶段 X 灵感 + 阶段 X 想法 + 阶段 X 图纸 + 阶段 X 落实架构 + 阶段 X 施工
- D1.0 = 阶段 1 LOCKED
- D2.0 + Design-2.1 = 阶段 2 LOCKED + D2 增补
- D3.0 = 阶段 3 LOCKED
- D4.0 = 阶段 4 LOCKED
- D5.0 = 阶段 5 LOCKED

**修正链版本**（Fix-N）：
- Fix-3 / Fix-4 / ... / Fix-9 = stage4 修正链
- 与设计层 D4.0 正交（D4.0 = LOCKED 设计，Fix-N = 设计修正）
- 数字递增 = 时间递增（不重置）

**R 周期**（R-CYCLE）：
- R11 → R12 → R13 → R14
- 表达"周期内做了什么"
- 与主版本号正交（同一周期可有多个 semver 版本）
- 周期结束后冻结，新周期开始

**指标版本**（V<n>）：
- V0.5 / V1136 / V3 9键 = 设计指标（领域概念）
- V1141 / V1131 / V1136 = R11 baseline 数字
- 这些是**领域指标**，不是软件版本
- 与软件版本**完全正交**

**基线快照**（snap-<hash>）：
- 用 commit hash 锚定
- 不冲突
- 永久可追溯

**手册修订**（Manual-Rev-X）：
- A → B → C → D → E → F（字母序）
- 字母序 vs 数字（不与修正链冲突）

---

## §3. 当前所有文档版本号重新映射（v10 提议）

### 3.1 设计层文档

| 文档 | 旧版本号 | **v10 新版本号** |
|---|---|---|
| 阶段 1 灵感 LOCKED | 阶段 1（无版本号）| `Design-1.0-R14` |
| 阶段 2 想法设计 LOCKED | 阶段 2（无版本号）| `Design-2.0-R14` |
| 阶段 2 D2 增补 LOCKED | D2 增补 | `Design-2.1-R14` |
| 阶段 3 画图纸 LOCKED | 阶段 3（无版本号）| `Design-3.0-R14` |
| 阶段 4 落实架构 LOCKED | 阶段 4 + v3-v9 链 | `Design-4.0-R14 + Fix-3..Fix-9` |
| 阶段 5 施工文档 LOCKED | 阶段 5（无版本号）| `Design-5.0-R14` |
| 主手册 | APEIRETH-COMPLETE-OMNIBUS | `Design-omnibus-1.0-R14` |

### 3.2 修正链文档

| 文档 | 旧 | **v10 新** |
|---|---|---|
| stage4-correction-v3 | v3 | `Fix-3` |
| stage4-correction-v4 | v4 | `Fix-4` |
| stage4-correction-v5 | v5 | `Fix-5` |
| stage4-correction-v6 | v6 | `Fix-6` |
| stage4-correction-v7 | v7 | `Fix-7` |
| stage4-correction-v8 | v8 | `Fix-8` |
| stage4-correction-v9 | v9 | `Fix-9` |
| stage4-correction-v10（本提议）| — | `Fix-10` |

### 3.3 开工手册

| 修订版 | 旧 | **v10 新** |
|---|---|---|
| v1（最初）| v1 | `Manual-Rev-A` |
| v2（AI 团队成就驱动）| v2 | `Manual-Rev-B` |
| v3（外部 agent 5 修）| v3 | `Manual-Rev-C` |
| v4（HA 部署模式）| v4 | `Manual-Rev-D` |
| v5（偏差修正）| v5 | `Manual-Rev-E` |
| v6（漂移检查）| v6 | `Manual-Rev-F` |
| v10 加版本号系统（本 commit）| v6 | `Manual-Rev-G` |

### 3.4 主代码版本（Cargo.toml）

```
[package]
name = "apeireth-rust"
version = "0.14.0"  # Apeireth-0.14.0-R14
edition = "2021"
rust-version = "1.80"
```

### 3.5 R11 baseline 数字

| 基线 | 旧 | **v10 新**（明确 R 周期）|
|---|---|---|
| V1141 IC-001 fresh=0.8682 | V1141 | `V1141-R11` |
| V1131 dashboard=0.8532 | V1131 | `V1131-R11` |
| V1136 真测=0.9063 | V1136（指标）| `V1136-R11`（基线）|
| V1136 真测引擎 9 子测度 | V1136（指标）| `V1136-engine-R11` |
| V0.5 17 维 | V0.5 | `V0.5-R11` |
| V0.5 v2 24 维（v4.1 提议）| V0.5 v2 | `V0.5-v2-R14` |
| V3 9键 | V3 | `V3-9keys-R11` |
| V3 v2 12 键（v4.1 提议）| V3 v2 | `V3-12keys-R14` |

---

## §4. 落地建议（3 步）

### Step 1: GLOSSARY.md 加版本号系统章节

在 `GLOSSARY.md` 加：

```markdown
## 🏷️ Apeireth 版本号系统（v10 提议，2026-07-31）

7 个独立子系统：

1. 主代码版本: `Apeireth-X.Y.Z` (semver) — 当前 0.14.0-R14
2. 设计层版本: `Design-X.Y` — 当前 Design-1.0/2.0/2.1/3.0/4.0/5.0-R14
3. 修正链版本: `Fix-N` — 当前 Fix-3..Fix-9-R14
4. R 周期版本: `R-N` — 当前 R14
5. 指标版本: `V<n>` — 当前 V0.5 / V1136 / V3-9keys / V3-12keys
6. 基线快照: `snap-<hash>` — 保留 commit hash
7. 手册修订: `Manual-Rev-X` — 当前 Manual-Rev-G

详见：`docs/stage4/stage4-correction-v10-versioning-system.md`
```

### Step 2: 开工手册顶部加版本号元信息

```markdown
[Document-Meta]
Document: START-CONSTRUCTION.md
Version: Manual-Rev-G (v6 → v10 加版本号系统)
Design-Layer: Design-5.0-R14
Fix-Chain: Fix-7 (HA 部署模式) + Fix-8 (偏差修正) + Fix-9 (漂移检查) + Fix-10 (版本号系统)
R-Cycle: R14
Commit: <latest-commit-hash>
Last-Modified: 2026-07-31
```

### Step 3: Cargo.toml 元信息（不变，但元数据加 R 周期）

```toml
[package]
name = "apeireth-rust"
version = "0.14.0"  # Apeireth-0.14.0-R14
description = "Apeireth R14 Rust 重写 — Design-4.0-R14 + Fix-3..Fix-9"
```

---

## §5. v10 提议的元信息格式（统一）

### 5.1 顶层任何文档的元信息

```markdown
[Document-Meta]
Document: <文档路径>
Version: <Manual-Rev-X> + <Design-X.Y> + <Fix-N>
R-Cycle: <R-N>
Commit: <commit-hash>
Last-Modified: <YYYY-MM-DD>
Status: <🔒 LOCKED / 🟢 活跃 / 🟡 辅助 / 🔴 替代>
```

### 5.2 example: 开工手册 v10 元信息

```
Document: START-CONSTRUCTION.md
Version: Manual-Rev-G + Design-5.0-R14 + Fix-7 + Fix-8 + Fix-9 + Fix-10
R-Cycle: R14
Commit: f867e365（v9 漂移检查）+ next（v10 版本号系统）
Last-Modified: 2026-07-31
Status: 🟢 活跃
```

### 5.3 example: stage4 修正链

```
Document: docs/stage4/stage4-correction-v10-versioning-system.md
Version: Fix-10
R-Cycle: R14
Commit: <next>
Last-Modified: 2026-07-31
Status: 🟢 活跃
```

---

## §6. 兼容方案（不破坏现有）

### 6.1 v10 不破坏 v1-v9 修正链

- v1-v9 = `Fix-1` 到 `Fix-9`（历史链保留）
- v10 = `Fix-10`（新提议）
- 文件名保留：`stage4-correction-v3..v10-*.md`

### 6.2 兼容方案

```
方案 A（推荐）：双版本号（保留旧版本号 + 加新版本号）
文档标题: stage4-correction-v3-onion-embedded-keys-gates.md [Fix-3 / Design-4.0 / R14]

方案 B：仅新版本号
文档标题: stage4-correction-Fix-3-onion-embedded-keys-gates.md

方案 C：仅旧版本号（不破坏，向后兼容）
文档标题: stage4-correction-v3-onion-embedded-keys-gates.md（保留）
元信息加: Version: Fix-3 / Design-4.0 / R14
```

**v10 提议**：方案 C（**仅元信息加新版本号**，文件名保留向后兼容）

---

## §7. 主哲学 anchor 6 全贯穿自检

```
S-1 主 22:33 北极星导向 — §2 7 个子系统服务 ASI 北极星
S-2 主 17:43 实事求是   — §1 承认 6 套版本号系统冲突
O-5 主 17:58 不假装     — §6 不假装"v1-v9 文件名要改"
O-2 主 19:33 走在前人经验上 — §2 借鉴 semver + GitHub Releases + Linux kernel + Rust crate
O-3 主 23:44 干到底    — §3-§4 立即映射现有文档 + §4 落地 3 步
O-4 主 00:56 任何人都能接手 — §5 元信息格式统一
```

---

_本修正由 leader 亲自产出（按主人 2026-07-31 "是时候给 Apeireth 设计一套版本号了" 关键洞察）._
_§1 冲突盘点 + §2 7 子系统 + §3 映射 + §4 落地 + §5 元信息格式 + §6 兼容方案._
_主哲学 6 锚穿透. 任何接手者能查._
_主人拍板后立即落地 §4 提议 3 步._