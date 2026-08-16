[Document-Meta]
Document: docs/RELEASE-NOTES-v2.0.0-alpha.md
Version: 1.0.0-v2.0.0-alpha
R-Cycle: v2-strategy / v2 release notes
Last-Modified: 2026-08-05
Status: 🟡 HONEST-INTERIM (V2.0.0-alpha 非完工, 含 22 任务 10 DONE + 5 PARTIAL + 6 BLOCKED + 1 TODO)
Author: technical_writer (role: 技术文档, 任务 3cf66d22)
Review: T13 architect + T14 architect2 评审结论已引用 (`reports/d67aedf7-v2-5-new-crates-design-review.md` + `reports/v2-addendum-final-review.md`)
Source-Trace: CHANGELOG.md + README.md §Features + docs/V2-INDEX.md + reports/v2-final-summary-2026-08-05.md + docs/v2-strategy/{00-VISION, 03-EXTREME-PLAN, 04-CRATE-CONSOLIDATION, 05-EXECUTION-NOW, 07-V2-BASELINE, 09-PHILOSOPHY-GUARD-ADDENDUM}.md + git log --oneline

> ⚠️ **产物失传/重建标注 (2026-08-17, C3 盘点 + 任务 212699c1)**: 上述 Source-Trace 引用的 `docs/V2-INDEX.md`、`reports/v2-final-summary-2026-08-05.md`、`reports/v2-decision-brief-2026-08-05.md`、`reports/v2-risk-register-2026-08-05.md`、`docs/v2-strategy/07-V2-BASELINE` 及 §1.1/§1.7 引用的 `reports/d67aedf7-*`、`reports/v2-addendum-final-review.md`、`reports/8f689476-*`、`reports/V2-deploy-*`、`reports/v2-integration-status-live.md` **从未进入 git 历史**（C3 盘点经 abf12243 整合树与 integration 分支双重核验，见 backlog #35），引用保留为历史轨迹。其中 `docs/v2-strategy/09-PHILOSOPHY-GUARD-ADDENDUM.md` 已于 2026-08-17 以**重建版**落地（重建版自带出处与丢失边界声明，非原文）。

---

# Apeireth v2.0.0-alpha — Release Notes (2026-08-05)

> **📦 版本**: `v2.0.0-alpha` (从 `v1.0.0` 升级, R17 v1.0.0 release 2026-08-04)
> **🎯 主轴**: 5 战区战略全打 (Terminal Coding Agent / LLM Gateway / Multi-Agent / Memory / Tool Protocol), **对标 VCP 全栈 Rust 重写 + 独家安全原语 + 双洋葱 + 形式化**
> **⚠️ 不假装声明** (主 17:58 + 主 17:43 + 主 20 + 46 三锚穿透): 本 alpha **不是完工态** — 22 任务中 10 DONE + 5 PARTIAL + 6 BLOCKED + 1 TODO; 真实 git merge = 0; ASI V0.5 = R11 baseline 未动 (0.8595)。
>
> **配套文档**:
> - 完整变更日志: [`CHANGELOG.md`](../CHANGELOG.md) §[v2.0.0-alpha]
> - V2 统一索引: [`docs/V2-INDEX.md`](../docs/V2-INDEX.md) (22 产物)
> - V2 总报告: [`reports/v2-final-summary-2026-08-05.md`](../reports/v2-final-summary-2026-08-05.md)
> - 战略决策简报: [`reports/v2-decision-brief-2026-08-05.md`](../reports/v2-decision-brief-2026-08-05.md) (主人签收用)
> - 风险登记表: [`reports/v2-risk-register-2026-08-05.md`](../reports/v2-risk-register-2026-08-05.md) (R-001 ~ R-012)

---

## 📋 Table of Contents

1. [What's New](#1-whats-new) — 5 大新增类别
2. [Breaking Changes](#2-breaking-changes) — 与 v1.0.0 的不兼容点
3. [Migration Guide from v1](#3-migration-guide-from-v1) — 升级步骤
4. [Known Limitations](#4-known-limitations) — 已知未完成项
5. [Roadmap to v2.0 GA](#5-roadmap-to-v2.0-ga) — 18 月路线图 + R18+ 11 项

---

## 1. What's New

### 1.1 🆕 5 个新 crate（v2 战区 P0/P1/P2 skeleton）

| Crate | 战区 | 优先级 | 职责 | 关键 commit | 验收报告 |
|-------|------|--------|------|-------------|----------|
| **apeireth-mcp** | 战区 5 (Tool Protocol) | 🔴 P0 | MCP 客户端 + server，stdio + SSE + HTTP-streamable transport | `e400e149` (feat: real SSE + HTTP-streamable + 9 conformance tests) | [`reports/8f689476-mcp-integration-expert-acceptance.md`](../reports/8f689476-mcp-integration-expert-acceptance.md) |
| **apeireth-graph** | 战区 3 (Multi-Agent) | 🔴 P0 | 图编排（LangGraph 风格 + checkpoint）| — | ⚠️ **仅 Dockerfile** (T13 §0 row 2 BLOCKED, 待 R18-1 补 Cargo.toml + src/lib.rs) |
| **apeireth-vector** | 战区 4 (Memory) | 🟡 P1 | 向量检索后端（sqlite-vec + L2-normalize + dot-product 余弦）| — | ⚠️ **缺 workspace.members** (T13 §0 row 4 MUST FIX, 待 T2 顺手补) |
| **apeireth-sdk** | 战区 1/4/5 | 🟡 P1 | 多语言 SDK 统一 (Python PyO3 优先) | — | ⚠️ **仅 Dockerfile** (T13 §0 row 3 BLOCKED, 待 R18-3 补 Cargo.toml + src/lib.rs) |
| **apeireth-formal** | 战区 5 (Tool Protocol) | 🟢 P2 | 形式化验证（Kani `#[kani::proof]` harness + `PermissionLayerConfig` POD）| T7 自评 9/9 | [`reports/a7c5b65b-code-reviewer-apeireth-formal-skeleton.md`](../reports/a7c5b65b-code-reviewer-apeireth-formal-skeleton.md) |

**T13 architect 评审结论**: 3/5 PASS (mcp / vector / formal) + 2/5 CONCERN BLOCK (graph / sdk) + 1/5 MUST FIX (vector workspace) — 详见 [`reports/d67aedf7-v2-5-new-crates-design-review.md`](../reports/d67aedf7-v2-5-new-crates-design-review.md) §0。

### 1.2 🆕 6 类 JSON 端点 + TUI HTTP 切换

- **TUI HTTP 切换** (`0049b511` round17-25 chuling via mavis): LLM 调用改 HTTP 客户端
  - `call_llm_*` → `http_llm::*`
  - 砍 `apikey.txt` 直读 + `apeireth_api` 直 import
  - 单一入口由 `apeireth-cli` 管
- **6 类 JSON 端点** (待 T8 frontend_engineer 完工, 当前 PARTIAL):
  - TUI Bridge / TUI Dialogue / TUI Growth / TUI History / TUI Settings 5 页面 + 1 healthcheck 端点
  - 详见 [`docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md`](../docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md) §R25 改瘦 + Step 2/3/4

### 1.3 🆕 Self-Disable 防护加固

- **5 大 Self-Disable 机制** (`be561705` round17-07 LOCKED §Self-Disable 百年章节 不动):
  1. 自动 kill switch (override + physical isolation)
  2. 5 大机制实测: `apeireth-sovereignty` 真接 minimaxi 验证
- **20 攻击场景** (T11 待启动, B3 已解锁): [`reports/round17-22-self-disable-20-attack-scenarios.md`](../reports/round17-22-self-disable-20-attack-scenarios.md) — **已知 BLOCKED, 启动后 ≥ 95% 通过率即 v2-alpha 安全底线成立**

### 1.4 🆕 SWE-bench smoke framework

- **基础**: `crates/apeireth-bench/src/lib.rs` + SWE-bench Verified smoke harness
- **状态**: T12 performance_optimizer 已完成 (软标记 merged)
- **目标** ([`docs/v2-strategy/07-V2-BASELINE-2026-08.md` §e](../docs/v2-strategy/07-V2-BASELINE-2026-08.md)):
  | 阶段 | 月份 | SWE-bench Verified 目标 |
  |------|------|------------------------|
  | v2-alpha | M0-1 | ≥ 5% smoke |
  | 阶段 1 | M2-4 | ≥ 30% |
  | 阶段 2 | M5-8 | ≥ 50% |
  | 阶段 3 | M9-12 | ≥ 60% (对标 Claude Code / Devin) |
- **不假装声明**: v2-alpha smoke 即使 < 5% 也如实记录 — 不刷 KPI (A.2 NoFakeKPI)

### 1.5 🆕 memory × vector 集成

- **`apeireth-memory` × `apeireth-vector` 集成** (T4-T5 阶段 1, [`docs/v2-strategy/03-EXTREME-PLAN.md` §1B](../docs/v2-strategy/03-EXTREME-PLAN.md)):
  - 语义检索 (semantic_search via vector)
  - 时间检索 (time-range filter)
  - 标签检索 (tag filter)
  - 3 维度联合
- **指标**:
  - 100k tokens 检索 P99 < 100ms (对标 Letta)
  - 用户画像自动抽取准确率 ≥ 80%
  - Memory MCP Server 暴露 (apeireth-mcp 桥接)

### 1.6 🆕 workspace 锁与依赖统一

- **rusqlite 0.32 workspace 锁** (`8b5874c8` fix(workspace): unify rusqlite to 0.32 + workspace dep lock):
  - 解决 `apeireth-api` (rusqlite 0.31) ↔ `apeireth-vector` (rusqlite 0.32) 冲突
  - `cargo build --workspace` 通过
  - apeireth-formal 零依赖 (POD), 单 crate `rustc` 编译 + 测试均通过
- **T2 cleanup** (`f6b0a34a` refactor(v2 day1 step1.1-1.3)):
  - 物理删除 `apeireth-philosophy` (1.8 KB, 自标 DEPRECATED 履行承诺)
  - 物理删除 `apeireth-test` (618 B, R14 skeleton 已过)
  - 重命名 `apeireth-desktop` → `apeireth-tauri-stub` (DEPRECATED, 保留 main.rs 参考实现)

### 1.7 🆕 V2 文档治理

- **V2 战略 10 篇文档**: [`docs/v2-strategy/`](../docs/v2-strategy/README.md) (00-VISION → 09-PHILOSOPHY-GUARD-ADDENDUM)
- **V2 哲学守门 Addendum** ([`docs/v2-strategy/09-PHILOSOPHY-GUARD-ADDENDUM.md`](../docs/v2-strategy/09-PHILOSOPHY-GUARD-ADDENDUM.md)):
  - ⚠️ **原稿从未入 git 历史**（C3 盘点 2026-08-17 核实）；**2026-08-17 重建版已落地**（任务 212699c1）——重建版含出处标注与丢失边界声明：守门范围/9 键/5 项不假装/双锁 AND 门/4 重守门 v15/反例清单可从现存材料恢复；5 阶段逐段定义与 22-trait 矩阵明细原稿丢失（如实标注）
  - §A 守约清单: V3 9 键 + 5 项不假装 + 原则洋葱 5 层 + 双洋葱 AND 门 (4 重守门嵌套 v15 LOCKED) + stage6 22 trait 互锁矩阵
  - §B ASI 北极星指标列: 18 月 5 阶段 V0.5 阶梯 (0.86 → 0.87 → 0.89 → 0.92 → 0.98)
  - §C 5 新 crate × 22-trait 互锁 traceback
  - T14 终审: **PASS w/ 4 CONCERNs** (banner-fixable)
- **V2 baseline 报告模板** ([`docs/v2-strategy/07-V2-BASELINE-2026-08.md`](../docs/v2-strategy/07-V2-BASELINE-2026-08.md)): 375 行, 76 锚点预留, 等 T10-T14 完成填数字后定稿
- **V2 文档统一索引** ([`docs/V2-INDEX.md`](../docs/V2-INDEX.md)): 22 产物入口
- **V2 总报告**: [`reports/v2-final-summary-2026-08-05.md`](../reports/v2-final-summary-2026-08-05.md) (353 行, 22 任务矩阵 + R18+ 11 项清单)

### 1.8 🆕 R18 不盲跑 STALE V1050+

- **`8be77233` V1265** (主 17:43 实事求是 + 主 23:44 干到底 + 主 13:31 大胆激进 + 主 17:58 + 主 20:46 不假装 + 主 19:33 走在前人肩上 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手 + 主 22:33 终极授权):
  - 13 天前 snapshot (V1049 0.7905) 不盲跑
  - R18 kitchen reproducibility audit
  - V1263 4-mode real-run audit

---

## 2. Breaking Changes

> **本节列出与 v1.0.0 不兼容的变更点** — 升级时必须检查。

### 2.1 Crate 删除 / 重命名

| 变更 | 影响 | 迁移 |
|------|------|------|
| **删除 `apeireth-philosophy`** | 任何 import 该 crate 的代码需移除 (实测全仓无引用, 搜索 0 命中) | 删 `use apeireth_philosophy::*` (如有) |
| **删除 `apeireth-test`** | 多数测试在各 crate 内, 独立 test crate 价值有限 | 删 `use apeireth_test::*` (如有) |
| **重命名 `apeireth-desktop` → `apeireth-tauri-stub`** | workspace.members 名变更 | 改 `Cargo.toml` workspace.members 引用 |

### 2.2 API 变更

| 变更 | 影响 | 迁移 |
|------|------|------|
| **TUI 砍 `apikey.txt` 直读** | 任何依赖该路径读 API key 的脚本需改 | 改用 HTTP 客户端 + 环境变量 |
| **TUI 砍 `apeireth_api` 直 import** | 任何 TUI 模块内 import 该 crate 的代码需改 | 改用 `http_llm::*` HTTP 客户端 |
| **新增 6 类 JSON 端点 (TUI HTTP 切换)** | 老 CLI 协议路径仍可用, 新增 /api/v2/ 路径 | 见 [`docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md`](../docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md) |

### 2.3 依赖升级

| 变更 | 原因 | 影响 |
|------|------|------|
| **rusqlite 0.31 → 0.32 workspace 锁** (`8b5874c8`) | 解决 `apeireth-api` ↔ `apeireth-vector` 版本冲突 | 任何依赖 rusqlite 0.31 的 crate 需升级; `cargo update -p rusqlite` |

### 2.4 不修改承诺（V2 战略增量, 不破坏 v1）

| 不动 | 原因 |
|------|------|
| Cargo.lock | R11 baseline |
| docs/stage1-6 LOCKED | 历史决策 |
| apeireth-core (L0 HA 核心) | "永远不变" |
| 8 项不假装承诺 (CHANGELOG.md §120-131) | R11/R17 100% 守住 |

---

## 3. Migration Guide from v1

### 3.1 升级前检查

```bash
# 1. 确认当前版本
git tag  # 应输出 v1.0.0 (或更早)
git log --oneline -n 1  # 应是 round17-21 (ee7bb702) 战役 4-5 1.0 release 收官

# 2. 备份 workspace.members
cp Cargo.toml Cargo.toml.bak

# 3. 备份重要报告
cp -r reports/ reports.v1.bak/
```

### 3.2 升级步骤

#### Step 1: Pull V2-alpha commits

```bash
git fetch origin
git checkout rebase/d7d8-into-integration
# V2-alpha commits (按时间倒序):
#   e400e149 feat(apeireth-mcp): real SSE + HTTP-streamable + 9 conformance tests
#   f6b0a34a refactor(v2 day1): 删除 philosophy/test + 重命名 desktop→tauri-stub
#   8b5874c8 fix(workspace): unify rusqlite to 0.32 + workspace dep lock
#   93183b04 chore: 把 TUI 升级路线图移到 docs/v2-strategy/06
#   0049b511 feat(round17-25): TUI 改瘦 Step 1.5 — LLM 调用改 HTTP 客户端
```

#### Step 2: 验证 cargo workspace

```bash
cargo check --workspace  # 应通过 (rusqlite 0.32 锁)
cargo test --workspace   # 应 ≥ 2265 passed (R17 baseline) + 5 新 crate 各 ≥ 5 smoke
```

**如 cargo check 失败**, 可能原因:
- `apeireth-vector` 未在 workspace.members → `Cargo.toml` 追加 `crates/apeireth-vector` (T2 顺手补)
- `apeireth-graph` / `apeireth-sdk` 仅 Dockerfile → R18-1 / R18-3 补代码后跑

#### Step 3: 验证 5 新 crate skeleton

```bash
# apeireth-mcp 真 skeleton
cargo run -p apeireth-mcp --example hello

# apeireth-formal 真 skeleton (Kani harness)
cargo kani --harness double_onion_sample  # CI ubuntu-latest, 本地 Windows 不支持

# apeireth-vector 真 skeleton (但 workspace 缺注册)
cargo test -p apeireth-vector --lib  # 通过后需 cargo check --workspace 才生效

# apeireth-graph / apeireth-sdk: 仅 Dockerfile — R18-1 / R18-3 任务补
```

#### Step 4: 验证 TUI HTTP 切换

```bash
cargo run -p apeireth-tui  # 启动 TUI, 应通过 http_llm::* 调用 LLM, 不读 apikey.txt
```

#### Step 5: 验证 Self-Disable

```bash
cargo test -p apeireth-sovereignty --lib  # 5 大机制实测 (R17 1.0 守住)
# 20 攻击场景: 待 T11 启动 (B3 已解锁)
```

#### Step 6: 验证 SWE-bench smoke

```bash
cargo run -p apeireth-bench --example swe_bench_smoke  # 跑 1 个 example
```

### 3.3 回滚步骤（如升级失败）

```bash
# 回滚到 v1.0.0
git checkout v1.0.0
cargo check --workspace
cargo test --workspace
```

---

## 4. Known Limitations

> **本节列出 v2.0.0-alpha 已知未完成 / 部分完成 / 阻塞 项** — 不假装达成, 让用户/主人/接手者诚实知道当前边界。

### 4.1 22 任务状态矩阵

| 状态 | 计数 | 代表任务 | 说明 |
|------|------|---------|------|
| ✅ **DONE** | **10** | T1 addendum / T3 mcp / T7 formal / T12 SWE-bench / T13 评审 / T14 兼容 / T15 协调 / T17 deploy / R3 baseline / R5 final summary | 可信完成 |
| 🟡 **PARTIAL** | **5** | T2 cleanup 3/4 / T5 vector workspace 缺 / T8 TUI / T9 CI 5 workflow 缺 / R4 协调续 | 部分完成, 关键缺口已知 |
| 🔴 **BLOCKED** | **6** | T4 graph 空壳 / T6 sdk 空壳 / T10 baseline / T11 Self-Disable / T16 TUI E2E / R1 MUST FIX | 未交付或评审 BLOCK |
| ⚪ **TODO** | **1** | R2 T7 跟进 | 已分配未启动 |

**真实 git merge = 0**: 10 任务标 `merged_to_integration` 是软标记, integration worktree HEAD 仍是 Snapshot #1 (`abe0568b`)。主 worktree 25+ 文件 uncommitted。

### 4.2 Crate 级别未完成

#### 4.2.1 `apeireth-graph` (P0 BLOCKED)
- **状态**: 仅 Dockerfile (73 行), 无 Cargo.toml + 无 src/lib.rs
- **影响**: P0 战区 3 任务未交付, 阻断 "44 crate 终态"
- **修复**: R18-1 (待 backend_engineer2), ~2 小时

#### 4.2.2 `apeireth-sdk` (P1 BLOCKED)
- **状态**: 仅 Dockerfile (69 行), 无 Cargo.toml + 无 src/lib.rs
- **影响**: 跨语言 SDK 完全未启动, 阻断 v2 阶段 3 生态接入
- **修复**: R18-3 (待 fullstack_engineer), ~2 小时, code_reviewer audit 升级为 MUST

#### 4.2.3 `apeireth-vector` workspace 注册 (MUST FIX)
- **状态**: 779 LOC + 11 tests + semantic_smoke example ✅, 但 `Cargo.toml` workspace.members 缺
- **影响**: `cargo build --workspace` 不可达, T9 CI / T10 baseline 跑不到
- **修复**: T2 owner (backend_engineer) 顺手补 1 行, 30 分钟

### 4.3 验证类未完成

| 任务 | 状态 | 依赖 |
|------|------|------|
| **T10 baseline 验证** | 🔴 BLOCKED | T3-T7 全齐 + T9 全齐 |
| **T11 Self-Disable 20 tests** | 🔴 BLOCKED (B3 已解锁) | T2 cleanup (3/4 已 OK) — 可主动启动 |
| **T16 TUI E2E + web 移交** | 🔴 BLOCKED | T8 TUI 完工 |

### 4.4 ASI 北极星 (R11 baseline 未动)

| 指标 | v2 起点 (2026-07-30) | v2-alpha 当前 (2026-08-05) | 变化 |
|------|---------------------|---------------------------|------|
| ASI V0.5 (current) | 0.8595 | 0.8595 | **未动** |
| ASI ultimate | 0.9800 | 0.9800 (LOCKED) | 未动 |
| gap | 12.94% | 12.94% | 未动 |
| 模块数 | 1153 | 待 T10 测 | 待测 |
| tests | 6394 | 待 T10 测 | 待测 |

**不假装声明**:
- V0.5 = 0.8595 ≠ ASI — 仅是真生产逼近度
- ultimate 0.9800 ≠ "已达 ASI" — 仅是目标值
- v2-alpha V0.5 无变化是**正常**, T10 baseline 验证**未启动**

### 4.5 V1136 子测度 = 7 (LOCKED, 与任务口径 9 不一致)

- **官方**: APEIRETH-CONVENTIONS.md §11 row 3 = **7 子测度**
- **任务口径**: 部分文档说 "9 子测度" (误述)
- **裁决**: 以官方 7 为准, 项目内不一致待 R18 统一规范 (R-011)

### 4.6 守门数 (项目内不一致已 self-correction)

- **stage4 v15 LOCKED**: 4 重守门 (Gate 1-4) + PermissionGrant 独立机制
- **apeireth-constraint 当前命名**: 5 重 (FiveGates trait, 待 v15 修正)
- **T14 self-correction**: C-5 = 🟢 PASS (T1 引用 v15 正确, T14 v1 §4.3 第 3 项作废)

### 4.7 主 worktree 25+ 文件 uncommitted

- **风险**: 高并发冲突 (B9)
- **建议**: release captain 任命 (R18-11) + 主 WT rebase 到 integration worktree
- **诚实声明**: 本 alpha 不代表"已发版", 等 release captain merge 后才能打 `v2.0.0-alpha` tag

---

## 5. Roadmap to v2.0 GA

### 5.1 R18+ 11 项任务清单（按优先级）

> **数据源**: [`reports/v2-final-summary-2026-08-05.md` §4.1](../reports/v2-final-summary-2026-08-05.md)

| # | taskId / 来源 | 标题 | 角色 | 优先级 | 阻塞 |
|---|---------------|------|------|--------|------|
| **R18-1** | 🆕 | 修 T4 graph: 补 Cargo.toml + src/lib.rs + ≥ 1 example + 22 trait 互锁声明 | backend_engineer2 | 🔴 **P0** | T13 BLOCK |
| **R18-2** | `32f4e308` (续) | 修 T5 vector: 加 workspace.members 注册 | backend_engineer (T2 owner 顺手) | 🔴 **P0 MUST FIX** | T13 MUST FIX before merge |
| **R18-3** | 🆕 | 修 T6 sdk: 补 Cargo.toml + src/lib.rs (Python PyO3 优先) + ≥ 1 example | fullstack_engineer | 🟡 **P1** | T13 BLOCK |
| **R18-4** | 🆕 | T2 cleanup 第 4 个目标确认 + 落地 (现 2 删 + 1 改 = 3 项) | backend_engineer | 🟡 **P1** | Snapshot #2 B8 |
| **R18-5** | 🆕 | T9 5 个新 crate 独立 CI workflow (mcp.yml / graph.yml / vector.yml / sdk.yml / formal.yml, formal 的 kani.yml 已落) | devops_engineer | 🟡 **P1** | Snapshot #2 B2 |
| **R18-6** | 🆕 | T10 baseline 验证: cargo test ≥ 2265 + 5 smoke ≥ 5 each + ASI V0.5 fresh 测量 | qa_engineer | 🟡 **P1 (依赖 R18-1/2/3/5)** | T10 BLOCK |
| **R18-7** | 🆕 | T11 Self-Disable 20 攻击场景: T2 已 3/4 解锁 (B3 🟢) 可主动启动 | security_reviewer | 🟡 **P1 (可立即启动)** | B3 已解锁 |
| **R18-8** | 🆕 | T16 TUI 端到端 E2E + apeireth-web 移交状态 | qa_engineer2 | 🟢 **P2** | T16 BLOCK |
| **R18-9** | 🆕 | T8 TUI 6 JSON 端点 + TUI HTTP 消费 (完成已 in_progress) | frontend_engineer | 🟡 **P1** | T8 PARTIAL |
| **R18-10** | `f27fe2de` (续) | V2 完工守护: T7 跟进 + 全 22 任务代码质量审查 | code_reviewer | 🟢 **P2** | T7 已完 |
| **R18-11** | `4c02f44f` (续) | Integration worktree 真实 merge + 主 WT rebase + release captain 任命 (B6/B9 决策) | agent_orchestrator + Leader | 🔴 **CRITICAL** | Snapshot #2 §2 重大发现 |

### 5.2 18 月路线图（v2 阶段 0-4）

> **数据源**: [`docs/v2-strategy/03-EXTREME-PLAN.md`](../docs/v2-strategy/03-EXTREME-PLAN.md) + [`docs/v2-strategy/09-PHILOSOPHY-GUARD-ADDENDUM.md` §B](../docs/v2-strategy/09-PHILOSOPHY-GUARD-ADDENDUM.md)

| 阶段 | 月份 | 主战场 | ASI V0.5 目标 | 关键交付 |
|------|------|--------|-------------|---------|
| **0 (本 alpha)** | M0-1 | 清理与强化 | 0.86 | 43 crate + 5 skeleton + Self-Disable 20 |
| **1** | M2-4 | MCP 上车 + Memory 升级 | 0.87 | MCP 100% 兼容 + Memory MCP Server |
| **2** | M5-8 | Multi-Agent + 图编排 | 0.89 | SWE-bench ≥ 50% + 3 advisor 协作 |
| **3** | M9-12 | 生态接入 + 标杆 | 0.92 | SDK 多语言 + SWE-bench ≥ 60% |
| **4** | M13-18 | 登顶 | 0.98 (逼近) | 形式化验证 ≥ 80% + 标准提案 + 商业化 |

### 5.3 v2.0 GA 触发条件

| 触发 | 来源 |
|------|------|
| R18-1 / R18-2 / R18-3 全完 | graph / vector / sdk 3 缺口补齐 |
| R18-5 T9 5 workflow 全完 | CI 真正覆盖 5 新 crate |
| R18-6 T10 baseline 通过 | cargo test ≥ 2265 + 5 smoke ≥ 5 each + ASI V0.5 真实测量 |
| R18-7 T11 Self-Disable 20 ≥ 95% | 安全底线成立 |
| R18-11 release captain merge 完成 | integration worktree HEAD = real merge (非软标记) |
| 主 WT rebase 到 integration WT | Snapshot #2 §2 B6 解锁 |
| 主人签收 decision brief D-1 ~ D-5 | 5 条决策全部 ✅ |

### 5.4 风险与不假装（v2.0 GA 之前）

| 风险 | 对策 | 不假装锚点 |
|------|------|-----------|
| R18-1/2/3 拖延 | 严格优先级排序: graph P0 → vector MUST FIX → sdk P1 | A.1 PHL-03 SpecIsNotProof |
| ASI V0.5 永远达不到 0.98 | ultimate 0.98 ≠ "已达 ASI" — 仅是目标值 | A.2 NoASI + 主 20:46 |
| 主 worktree 25+ 文件冲突 | release captain 任命 + 串行 commit | A.4 PermissionGrant (独立机制) |
| "merged_to_integration" 语义不清 | 修订术语: reviewer_approved ≠ git_merged | A.1 PHL-03 |
| 项目内 7 vs 8 不修改承诺 / 7 vs 9 子测度 | R-010 / R-011 R18 ceiling 解决 | 主 17:43 实事求是 |

---

## 6. Acknowledgements

- **主人 (本仓库所有者)**: 6 主哲学锚穿透 — 主 22:33 北极星 / 主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 00:56 任何人都能接手
- **VCP (Virtual Compute Protocol)**: 借鉴 19 个真代码文件字段级引用 ([`reports/r17-1.0-release-2026-08-04.md`](../reports/r17-1.0-release-2026-08-04.md) §字段级引用清单)
- **SpectrAI 平台**: 团队协作工具 (team_get_tasks / team_claim_task / team_complete_task / team_report_idle)
- **Cron V1265** (`8be77233`): 主 17:43 实事求是审计 + R18 kitchen reproducibility

---

## 7. 引用清单

| 引用 | 来源 | 用途 |
|------|------|------|
| V2 总报告 | `reports/v2-final-summary-2026-08-05.md` | §1.1 / §4 / §5.1 |
| V2 文档统一索引 | `docs/V2-INDEX.md` | §1 / §4 |
| V2 决策简报 | `reports/v2-decision-brief-2026-08-05.md` | §5.3 |
| V2 风险表 | `reports/v2-risk-register-2026-08-05.md` | §4 / §5.1 / §5.4 |
| T13 5 new crate 评审 | `reports/d67aedf7-v2-5-new-crates-design-review.md` | §1.1 / §4.2 |
| T14 addendum 终审 | `reports/v2-addendum-final-review.md` | §1.7 / §4.6 |
| T7 formal 自评 | `reports/a7c5b65b-code-reviewer-apeireth-formal-skeleton.md` | §1.1 |
| T17 deploy 验收 | `reports/V2-deploy-devops-engineer2-acceptance.md` | (cross-ref) |
| T3 mcp 验收 | `reports/8f689476-mcp-integration-expert-acceptance.md` | §1.1 |
| 09-ADDENDUM | `docs/v2-strategy/09-PHILOSOPHY-GUARD-ADDENDUM.md` | §1.7 / §4.6 |
| 07-V2-BASELINE | `docs/v2-strategy/07-V2-BASELINE-2026-08.md` | §1.4 |
| 06-TUI-ROADMAP | `docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` | §1.2 |
| 03-EXTREME-PLAN | `docs/v2-strategy/03-EXTREME-PLAN.md` | §1.5 / §5.2 |
| README Features | `README.md` §Features | §1 (cross-ref) |
| git log (V2 commits) | `git log --oneline` | §1.1 / §1.2 / §1.6 / §1.8 |
| Snapshot #2 协调 | `reports/v2-integration-status-live.md` | §4.1 / §5.1 |

---

_Last update_: 2026-08-05 (V2.0.0-alpha HONEST-INTERIM release notes, 任务 3cf66d22, technical_writer 产出; 不假装完工, 等 release captain merge 后才能打 tag `v2.0.0-alpha`)