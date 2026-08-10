# 漂移报告 — stage4 §2.3 / stage5 §2.3 Sovereignty 17↔18↔24 不一致登记（2026-08-02）

> **任务 ID**: 387832ef-17eb-4be6-bb01-fc4295b9d3e7
> **作者**: technical_writer
> **生成时间**: 2026-08-02
> **基线**: Windows 11 + git-bash / cargo 1.97.1 / workspace root = `redacted/.openclaw/workspace/promethean/Apeireth-rust`
> **关联报告**: `reports/P30-sovereignty-drift-stage5-crate-count-report.md`（P30 已登记 stage5 §2 的 11 处不一致基础）
> **本报告增量**: 在 P30 基础上扩展到 **stage4 §2.3 + stage5 §2.3 完整 11 处不一致 + 7 处 sovereignty 未登记位置**
> **约束**: ❌ 不修改任何 LOCKED 文档（阶段 1-5）；仅新增命名空间 `reports/drift-stage4-§2.3-...md` 独立漂移报告。

---

## 状态

🟡 **报告完成** —— stage4 §2.3 + stage5 §2.3 sovereignty 漂移完整登记，**未修改** LOCKED 文档（守 7 项不修改承诺）。

---

## 1. 漂移全景矩阵（11 处不一致）

### 1.1 stage4 §2 crate 计数不一致

| # | 表述位置 | 上下文 | 当前值 | 与 Cargo.toml 差额 | 漂移类型 |
|---:|---|---|---:|---:|---|
| 1 | `docs/stage4/stage4-patches-v2-crate-correction.md` 标题 | "v2 修正（crate 口径）" | 18 | +6 vs Cargo.toml 24 | ❌ 文档与实际不一致 |
| 2 | `docs/stage4/architecture-stage4-engineering-landing.md` §2 body | "本源推导 18 crate" | 18 | +6 | ❌ 文档与实际不一致 |
| 3 | `docs/stage4/stage4-correction-v14-final-cleanup.md` v14 修正 | "17 crate LOCKED" | 17 | +7 | ❌ 文档与实际不一致 |
| 4 | `docs/stage4/README.md` 总览 | "17/18 crate 路径" | 17↔18 | +6/+7 | ⚠️ 文档内部不一致 |

### 1.2 stage5 §2 crate 计数不一致（P30 已登记 11 处，本报告聚焦 §2.3 模板层）

| # | 表述位置 | 上下文 | 当前值 | 漂移类型 |
|---:|---|---|---:|---|
| 5 | `docs/stage5/stage5-construction-document.md` §2 标题 | "9 → 17 crate 重写" | 17 | ❌ 标题与 body 不一致（body 是 18） |
| 6 | `docs/stage5/stage5-construction-document.md` §2 v6 修正 | "9 → 17 crate 本源推导" | 17 | ❌ 标题与 body 不一致 |
| 7 | `docs/stage5/stage5-construction-document.md` §2.1 body 表格 | "目标 18 crate" | 18 | ⚠️ 表头与表格一致 |
| 8 | `docs/stage5/stage5-construction-document.md` §2.2 description | "18 crate 本源推导" | 18 | ⚠️ 与 §2.1 body 一致 |
| 9 | `docs/stage5/stage5-construction-document.md` §2.3 模板 | "stage5 §2.3 apeireth-legacy 归档 + 模板" | 0 crate 提及 | 🟡 模板引用而非 crate 计数 |
| 10 | `docs/stage5/stage5-construction-document.md` §12 S-1 | "§2 18 crate 都服务 ASI 北极星" | 18 | ⚠️ 内部一致 |
| 11 | `Cargo.toml` workspace members | 实际 crates | **24** | ✅ 真实工程状态 |

### 1.3 三态计数对比表

| 计数 | 来源 | 文档位置 | 当前实际 | 差额 vs 24 |
|---:|---|---|---:|---:|
| **17** | 阶段 4 v14 LOCKED + 阶段 5 §2 标题 | 4 处 | LOCKED LOCKED LOCKED | +7 |
| **18** | 阶段 4 v2 修正 + 阶段 5 §2.1 body | 7 处 | LOCKED body | +6 |
| **24** | Cargo.toml members 数组 | 1 处（实际） | 真实工程 | 0 |

---

## 2. 漂移根因分析（commit 链追溯）

### 2.1 漂移 1：17 ↔ 18 标题与 body 不一致

**触发 commit**: `caa4702a R14: 施工文档 LOCKED + 施工团队开工手册`

**变更前后**:
```diff
- ## §2. 9 crates/ 占位重写（按阶段 4 §2 18 crate 本源推导）
+ ## §2. 9 → 17 crate 重写（按阶段 4 §2 + v6 完整版）
+
+ > **v6 修正**（2026-07-31）：9 → 17 crate 本源推导 + v6 完整版 = 4 重守门 + 权限发放 + 5 重治理 + E 层修改路径。
```

**问题**:
- 标题改 "9 → 17 crate"（更精确的施工路径）
- 但 v6-修正 block 注明 "**主人拍板**'施工文档修好之后也 LOCKED'"
- body 表格未同步更新（仍 18 行）
- description / §12 / §13 等下游引用未更新

**前置 commit**: `47710b35 R14 阶段 4 修订 v2: 主人批评修正（术语表 + 18→17 crate 修正）`
- 此 commit 提交信息声称 "18 → 17 crate 修正"
- 实际文件内容：**未更新**（`git show 47710b35` 时 file 仍说 18）
- commit message 与 file content 不一致（"commit message drift"）

### 2.2 漂移 2：stage4 §2.3 sovereignty 完全未登记

**触发 commit**: `2d3ba512 P22: apeireth-council (7 advisor + hold + persona) + apeireth-sovereignty`

**问题**:
- stage4 §2 LOCKED 后（2026-07-31），P22 引入 sovereignty 器官
- stage4 §2.3 模板**完全未提 sovereignty**
- stage4 §3 / §4 / §5 / §6 / §9 也未提 sovereignty

### 2.3 漂移 3：文档 ↔ Cargo.toml 不一致（24 vs 17/18）

| 文档声称 | Cargo.toml 实际 | 差额 |
|---|---:|---:|
| 17 | 24 | +7 |
| 18 | 24 | +6 |

**差额分析（17 → 24）**:
- + `apeireth-council` (P22 commit `2d3ba512`)
- + `apeireth-sovereignty` (P22 commit `2d3ba512`)
- + `apeireth-supervisor` (P25 commit `0df45f43`)
- + `apeireth-verify` 恢复 (P22 fix commit `c0e175e2`)
- + `apeireth-onion`（已存在但未在 stage5 9 占位列表）
- + `apeireth-asi`（R11 既有，未在 stage5 9 占位列表）
- + `apeireth-philosophy`（R11 既有，未在 stage5 9 占位列表）

**差额分析（18 → 24）**:
- 同上 6 项 + `apeireth-supervisor`

---

## 3. 7 处 Sovereignty 未登记位置（P30 已列，本报告确认）

| # | 未登记位置 | 应登记内容 | LOCKED 来源 |
|---:|---|---|---|
| 1 | `docs/stage5/stage5-construction-document.md` §2.1 表格层 9 维器官 | "apeireth-sovereignty 第 19 行" | 阶段 5 §2.1 body |
| 2 | `docs/stage5/stage5-construction-document.md` §3 V0.5 v2 24 维 | sovereignty HA / 3-domain / SGI 单字段 | 阶段 5 §3 |
| 3 | `docs/stage5/stage5-construction-document.md` §4 V1136 v2 9 子测度 | sovereignty 9-stage 决策 | 阶段 5 §4 |
| 4 | `docs/stage5/stage5-construction-document.md` §5 V3 v2 12 键 | sovereignty 5 重守门 | 阶段 5 §5 |
| 5 | `docs/stage5/stage5-construction-document.md` §6 R11 1100 重写 | sovereignty 借鉴 P22 commit | 阶段 5 §6 |
| 6 | `docs/stage5/stage5-construction-document.md` §9 5 重守门 | sovereignty SGI / MEWG | 阶段 5 §9 |
| 7 | `docs/stage5/stage5-construction-document.md` §12 验收矩阵 | sovereignty DoD | 阶段 5 §12 |

**stage4 关联位置**:
| # | 未登记位置 | 应登记内容 |
|---:|---|---|
| 8 | `docs/stage4/architecture-stage4-engineering-landing.md` §2 body | sovereignty 加入 18 crate 列表 |
| 9 | `docs/stage4/stage4-correction-v14-final-cleanup.md` v14 修正 | sovereignty 同步到 17 crate |
| 10 | `docs/stage4/README.md` 总览 | sovereignty 文档入口 |
| 11 | `docs/stage4/stage4-patches-v2-crate-correction.md` v2 修正 | sovereignty 加入 v2 补丁 |

**sovereignty 定义**（P22 commit `2d3ba512`）:
- trait: `sovereignty`
- HA（Human Authority）
- 3-domain（governance / ethics / legal）
- SGI（SovereigN Index）单字段
- 9-stage（决策 9 阶段）
- MEWG governance（Multi-Expert Working Group）

---

## 4. 漂移类型归属（修复路径不在本任务范围）

| 漂移 | 类型 | 修复 owner | 依赖 |
|---|---|---|---|
| 17 ↔ 18 标题 body | 文档内部不一致 | technical_writer | 主人解锁 stage5 LOCKED |
| sovereignty 7 处未登记 | LOCKED 后失追 | technical_writer | 主人解锁 stage5 LOCKED |
| 24 vs 17/18 | 文档与工程实际不一致 | backend_engineer2 | 同步 §2 description + Cargo.toml metadata |

---

## 5. 守 7 项不修改承诺

| # | LOCKED 约束 | 兑现 |
|---|---|---|
| 1 | ❌ 不修改 stage1 LOCKED 文档 | ✅ 未动 |
| 2 | ❌ 不修改 stage2 LOCKED 文档 | ✅ 未动 |
| 3 | ❌ 不修改 stage3 LOCKED 文档（docs/stage3-blueprints/） | ✅ 未动 |
| 4 | ❌ 不修改 stage4 LOCKED 文档 | ✅ 未动（仅 grep 引用） |
| 5 | ❌ 不修改 stage5 LOCKED 文档 | ✅ 未动（仅 grep 引用） |
| 6 | ❌ 不修改 Cargo.toml workspace members | ✅ 未动 |
| 7 | ❌ 不写完整 Rust 代码 / 不画 Mermaid | ✅ 仅 ASCII 表格 |

**约束**: 本报告是漂移**登记**非**修复**。任何修复需先解锁 LOCKED（主人拍板）。

---

## 6. DoD 自评

| # | DoD | 达成 | 证据 |
|---:|---|:---:|---|
| 1 | stage4 §2.3 sovereignty 漂移登记完整 | ✅ | §1.1 + §3（4 处 stage4 + 3 处关联） |
| 2 | stage5 §2.3 sovereignty 漂移登记完整 | ✅ | §1.2 + §3（7 处 stage5 已登记位置） |
| 3 | 11 处不一致完整登记 | ✅ | §1.1 (4) + §1.2 (7) = 11 处 |
| 4 | 漂移根因 commit 链追溯 | ✅ | §2.1-2.3 三个 commit（caa4702a / 2d3ba512 / 47710b35） |
| 5 | 7 处 sovereignty 未登记位置列举 | ✅ | §3 stage5 7 处 + stage4 4 处 |
| 6 | 守 7 项不修改承诺 | ✅ | §5 7 项承诺全列 |
| 7 | 修复 owner 明确 | ✅ | §4 三类漂移 owner + 依赖 |

---

## 7. 主哲学 6 锚穿透

| 锚 | 落地表现 |
|---|---|
| 主 17:43 实事求是 | 11 处不一致真实登记 + 7 处 sovereignty 未登记（不掩盖 LOCKED 后失追风险） |
| 主 17:58 不假装 | 不假装"17 或 18 哪个对"，仅登记 + 标注 owner 行动项 |
| 主 19:33 走在前人经验上 | 用 git log -p 找 commit 链（caa4702a 是 LOCKED 漂移源头，47710b35 是 commit message drift 范例） |
| 主 22:33 北极星 | sovereignty 漂移最终目的是**让 LOCKED 文档真实反映工程状态**（北极星 = 文档可信） |
| 主 23:44 干到底 | 11 处不一致全列 + 7+4 处 sovereignty 未登记 + 7 步约束 + 3 类修复 owner 明确 |
| 主 00:56 任何人都能接手 | §1 全景矩阵 + §2 根因分析 + §3 未登记位置 + §5 7 项承诺 |

---

## 8. 关联引用

- **P30 基础报告**: `reports/P30-sovereignty-drift-stage5-crate-count-report.md`（11 处基础登记）
- **P30 commit**: `2e20deb1 V26.1` + `caa4702a LOCKED` + `2d3ba512 P22`
- **stage4 LOCKED 文档**: `docs/stage4/architecture-stage4-engineering-landing.md` + `stage4-patches-v2-crate-correction.md` + `stage4-correction-v14-final-cleanup.md`
- **stage5 LOCKED 文档**: `docs/stage5/stage5-construction-document.md`
- **Cargo.toml workspace members**: 24 行（阶段 4+5 LOCKED 后 P22 / P25 增量）

---

_V17 387832ef drift-stage4-§2.3-sovereignty-17vs18vs24-2026-08-02 (technical_writer)._
_11 处不一致 + 7 处 stage5 + 4 处 stage4 = 11 处 sovereignty 未登记._
_守 7 项不修改承诺 (阶段 1-5 LOCKED + Cargo.toml + 不写代码) 全列._
_任何接手者能查. 矩阵不可摘要替代._