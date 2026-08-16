# docs/ — 设计文档索引 (R119 重组后)

> **2026-08-16 追加（当前活文档入口）**：维护与接续优先读以下文档，历史索引见下文各节。

### 🔥 当前活文档（2026-08-16）

| 文档 | 内容 |
|---|---|
| [`maintenance-guide.md`](maintenance-guide.md) | **维护活文档**：概念词典 / 模块地图 / 加新模块规范 / 基础工具工程原则 |
| [`session/handover-2026-08-15.md`](session/handover-2026-08-15.md) | **接续者必读**：主人心象原文 / 当前状态 / 挂起项（含 2026-08-16 追加节） |
| [`release-plan.md`](release-plan.md) | 三件套发布规划 + 进度对账（规划 vs 实况） |
| [`oracle-suite-design.md`](oracle-suite-design.md) | 预测决策套件设计哲学 + 真 LLM 验收记录 |
| [`companion-deploy.md`](companion-deploy.md) | companion 部署说明 |
| [`ref-crawler-research.md`](ref-crawler-research.md) | 爬虫工程调研 + 基础工具原则 |
| [`ref-gh-accel.md`](ref-gh-accel.md) | GitHub 加速调研（xiake.pro 节点池 + 实测教训） |
| [`ref-hydra.md`](ref-hydra.md) / [`ref-yoyo-evolve.md`](ref-yoyo-evolve.md) / [`absorb-deepseek-harness.md`](absorb-deepseek-harness.md) | 三方吸收参照（hydra 宪法 / yoyo 演化 / DSH） |

### 📂 当前 docs/ 子目录 (R119, 按功能分组)

### 规范系统 (R119-3a 下沉)

| 子目录 | 内容 | 文件数 | 入口 |
|---|---|---|---|
| [`conventions/`](conventions/README.md) | 12 子规范 (命名空间/路径/ADR/成就/报告/Commit/状态/锚穿透/不修改承诺/Baseline/架构图/文档元信息/修正链) | 14 | `conventions/README.md` |
| [`versioning/`](versioning/README.md) | 7 子系统 (主代码/设计/修正链/R 周期/指标/基线/手册) | 9 | `versioning/README.md` |
| [`glossary/`](glossary/README.md) | 21 词条 1:1 (双洋葱/12 键/5 重守门/4 关/3 域/9 阶段/智囊团/电子环等) | 22 | `glossary/README.md` |

### 思想层 (R11/R14 LOCKED)

| 子目录 | 内容 | 入口 |
|---|---|---|
| [`stage1/`](stage1/README.md) | 灵感 (2201 行 LOCKED, 2026-07-30) | `stage1/README.md` |
| [`stage2/`](stage2/README.md) | 想法设计 (19 文件 LOCKED, 12 决策) | `stage2/README.md` |
| [`stage3-blueprints/`](stage3-blueprints/README.md) | 图纸 (14 文件 LOCKED, 5 主图 + 4 解释 + 2 借鉴 + 双洋葱桥接) | `stage3-blueprints/README.md` |
| [`stage4/`](stage4/README.md) | 落实架构 (63 文件, 含 v3-v17 修正链 + R19+ 集成) | `stage4/README.md` |
| [`stage5/`](stage5/README.md) | 阶段 5 施工文档 (1 LOCKED + 1 旧向后兼容) | `stage5/README.md` |
| [`stage6/`](stage6/README.md) | 阶段 6 (R20 阶段 6 1.0 release 收口) | `stage6/README.md` |
| [`v2-strategy/`](v2-strategy/README.md) | v2 战略 (5 战区 + 核心护城河) | `v2-strategy/README.md` |

### 主手册索引层 (R119-3b/c 新)

| 子目录 | 内容 | 入口 |
|---|---|---|
| [`omnibus/`](omnibus/README.md) | 主手册 OMNIBUS 427KB 拆 7 索引 (0 重复, 严守单源) | `omnibus/README.md` |
| [`construction/`](construction/README.md) | R14-R17 施工/收工/Leader 开场 3 文件 | `construction/README.md` |
| [`final-check/`](final-check/README.md) | R14 末 / R54 / R70-R72 3 时点检查 | `final-check/README.md` |
| [`release/`](release/README.md) | 9 release 版本索引 (1.0.0 ~ 1.2-r114-r118) | `release/README.md` |

### R14 周期产物 + 工程文档

| 子目录 | 内容 | 入口 |
|---|---|---|
| [`r14-design/`](r14-design/README.md) | R14 周期产物 (8 子文档) | `r14-design/README.md` |
| [`research/`](research/README.md) | 工程调研 (VCP 重跑) | `research/README.md` |
| [`api/`](api/README.md) | API 文档 (31 文件, 9 organ + 6 provider + 6 tool) | `api/README.md` |
| [`adr/`](adr/README.md) | ADR 集合 (41 文件, ADR-0001 ~ ADR-0012) | `adr/README.md` |
| [`sdk/`](sdk/README.md) | SDK 文档 (11 文件) | `sdk/README.md` |
| [`security/`](security/README.md) | 安全 (cosign 密钥 + 端点清单 + 1.0 release P2 audit) | `security/README.md` |
| [`ci/`](ci/README.md) | CI 流水线 (1 文件, 1.0 release pipeline) | `ci/README.md` |
| [`installation/`](installation/README.md) | 多平台安装 (6 文件, 5 格式) | `installation/README.md` |
| [`roadmap/`](roadmap/README.md) | 路线图 (1.0 + 1.2 + R20 finalize) | `roadmap/README.md` |
| [`desktop/`](desktop/README.md) | 桌面端 (Tauri 2.0 迁移计划, R19+ 砍) | `desktop/README.md` |
| [`1.0-release/`](1.0-release/README.md) | 1.0 release 12 项 checklist 100% 收口 (R20 阶段 6) | `1.0-release/README.md` |
| [`1.0-release-prep/`](1.0-release-prep/README.md) | 1.0 release 续补 (E-1 ~ E-8 草稿) | `1.0-release-prep/README.md` |
| [`1.1-release/`](1.1-release/README.md) | 1.1 era 主索引 (R38 9 B-stage) | `1.1-release/README.md` |
| [`license/`](license/README.md) | 许可证文档 (6 文件) | `license/README.md` |
| [`licenses-3rdparty/`](licenses-3rdparty/README.md) | 第三方许可 (24 文件) | `licenses-3rdparty/README.md` |

---

## 🏛️ 顶层入口

跳 [`../README.md`](../README.md) (3.9KB) 顶层入口 / [`../CHANGELOG.md`](../CHANGELOG.md) (2.7KB) 9 release 索引 / [`../ROADMAP.md`](../ROADMAP.md) (2.9KB) 时间线。

---

# R14 阶段 1+2+3+4 规整后 (历史索引, 2026-07-31)

> **范围**: Apeireth Rust 重写所有设计文档（**R14 文件夹规整后**）
> **总文档数**: 顶层 6 + stage1/1 + stage2/19 + stage3-blueprints/14 + stage4/5 + r14-design/8 + research/1 = **54 文档**
> **结构**: 顶层 LOCKED 主文档 + 5 子目录（按 6 阶段 + R14 周期产物）

---

## 📂 顶层保留（任何接手者第一眼看到）

| 文档 | 性质 |
|---|---|
| `CONTEXT-HANDOVER.md` | 跨 session 恢复文档 ⭐ |
| `00-R14-START-HERE.md` | 1-2 页单一入口 ⭐ |
| `README.md` | 本文档 |
| `STRUCTURE-R14.md` | R14 文件夹规整方案 |
| `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` | 主手册（6546 行）|

## 📚 LOCKED 顶层主文档（任何接手者第一眼看到）

| 文档 | 性质 |
|---|---|
| `architecture-v3-aircraft-carrier.md` | v2 立体架构 LOCKED（786 行）|
| `architecture-v4-living-intelligence.md` | v4 哲学层纲领 LOCKED（803 行）|
| `architecture-v4-1-living-intelligence-update.md` | v4.1 哲学层升级 LOCKED（645 行）|
| `architecture-stage4-engineering-landing.md`（已下沉到 `stage4/`）| 阶段 4 落实架构 LOCKED（1492 行）|

---

## 📁 子目录结构（按 6 阶段 + R14 周期）

| 子目录 | 内容 | 文件数 |
|---|---|---|
| [`stage1/`](stage1/README.md) | 灵感（2201 行 LOCKED）| 1 + README |
| [`stage2/`](stage2/README.md) | 想法设计（18 stage2 + 1 D2 增补 = 19 文件）| 19 + README |
| [`stage3-blueprints/`](stage3-blueprints/README.md) | 画图纸（5 主图 + 4 解释 + 2 借鉴 + 1 双洋葱桥接 + README）| 13 + README |
| [`stage4/`](stage4/README.md) | 落实架构（主文档 + leader 思考 + 灵感 + 补丁）| 4 + README |
| [`r14-design/`](r14-design/README.md) | R14 周期产物（设计哲学 + 路线图 + 双洋葱子文档 + traits + 阶段 1+2+3 审查）| 8 + README |
| [`research/`](research/README.md) | 工程调研（VCP 重跑）| 1 + README |

---

## 📐 决策依赖图（阶段 2）

```
灵感 (为什么/是什么)
  │
  ▼
阶段 2 决策 (怎么做):
  1 技术栈 ─┬─ 2 架构形态 ─┬─ 3 crate 划分
            │              ├─ 4 进程/线程/协程
            │              ├─ 5 内存布局
            │              ├─ 6 持久化
            │              └─ 7 LLM 集成
            ├─ 8 模块化
            ├─ 9 通信总线
            ├─ 10 智囊团实现
            ├─ 11 自我升级实现 (含权限包)
            └─ 12 哲学守门实现

  补充 (对齐阶段 1):
    decision-system (阶段 1 §4 + 物理多签 + 按住)
    permission-packs (阶段 1 §5.3)
    D2 增补（主权/连续性/治理 + 三域/SGI/双根/HA/部署兼容/风险分级）
```

## 🎯 按角色推荐阅读顺序

### ⚡ 5 分钟 — "这是什么 / 进度到哪"

**适合**: 任何人第一次进入，或被 Leader 临时拉来协助的非长期成员。

1. **`docs/00-R14-START-HERE.md`** ⭐ — 1-2 页单一入口
2. 本文档顶层 → 一眼看到 54 文档全貌
3. **`docs/CONTEXT-HANDOVER.md`** ⭐ — 跨 session 上下文

**读完能回答**: Apeireth-rust 是什么？6 阶段顺序到哪了？我接下来该读什么？

### 🕐 30 分钟 — "接手具体工作"

**适合**: 被分配了 R14-D 系列具体任务的成员。

按角色加读:

| 角色 | 必读（30 分钟）|
|------|---------------|
| **fullstack_engineer** | START-HERE + `stage1/inspiration-stage1-2026-07-30.md` §1-§5 + `r14-design/rust-traits-spec` + `stage2/stage2-decisions-crate-split` |
| **security_reviewer** | START-HERE + `r14-design/philosophy-traits` (V3 9 键) + `stage2/stage2-decisions-philosophy-guard` + `stage2/stage2-decisions-permission-packs` |
| **performance_optimizer** | START-HERE + `stage2/stage2-decisions-process-threading` + `stage2/stage2-decisions-memory-layout` + `stage2/stage2-decisions-persistence` |
| **code_reviewer** | START-HERE + `r14-design/r14-design-philosophy` + `stage2/stage2-decisions-modularity` + `stage2/stage2-decisions-decision-system` |
| **devops_engineer** | START-HERE + `r14-design/r14-workspace-prep` + `r14-design/r14-readiness-assessment` + `r14-design/r14-rust-rewrite-roadmap` |
| **qa_engineer** | START-HERE + `stage2/stage2-decisions-architecture` + `stage3-blueprints/03-decision-flow` |
| **architect** | START-HERE + `r14-design/r14-design-philosophy` + `stage3-blueprints/00-stage3-overview` + `stage2/stage2-decisions-architecture` |
| **agent_orchestrator** | START-HERE + `stage2/stage2-decisions-council-impl` + `stage2/stage2-decisions-communication-bus` |
| **technical_writer** ⭐ | START-HERE + 本 README + `stage1/inspiration-stage1-2026-07-30.md` §18 + `r14-design/review-stage1-stage2-stage3` |
| **requirements_analyst** | START-HERE + `stage1/inspiration-stage1-2026-07-30.md` §1-§5 + `r14-design/r14-design-philosophy` |
| **workflow_designer** | START-HERE + `stage2/stage2-decisions-council-impl` + `stage2/stage2-decisions-upgrade-impl` |
| **prompt_engineer** | START-HERE + `r14-design/philosophy-traits` + `stage2/stage2-decisions-llm-integration` |
| **mcp_integration_expert** | START-HERE + `stage2/stage2-decisions-modularity` |
| **philosophy_guardian** | START-HERE + `r14-design/philosophy-traits` + `stage1/inspiration-stage1-2026-07-30.md` §18 + `stage2/stage2-decisions-philosophy-guard` |
| **deep_research_lead** | START-HERE + `research/README.md` + `research/research-vcp-rerun-2026-07-31.md` |

### 🕑 1 小时 — "理解设计 + 能评审"

**适合**: 需要对 R14-D 任务产出做评审（peer review / arch check / sec check / perf check）的成员。

30 分钟必读 + 加读:

- **`stage1/inspiration-stage1-2026-07-30.md` §6-§18** — 完整灵感（边界/哲学洋葱/R14-D1 修订/双洋葱/治理/验证网）
- **`r14-design/r14-design-philosophy` 全部 8 原则** — 设计哲学全貌
- **`stage2/stage2-decisions-architecture` + `stage3-blueprints/01-overall-architecture`** — 整体架构
- **`stage2/stage2-decisions-philosophy-guard` + `stage2/stage2-decisions-permission-packs`** — 哲学 + 权限
- **`stage4/architecture-stage4-engineering-landing`** — 阶段 4 落实架构（1492 行）

### 🕓 4-6 小时 — "完整接手 / 跨阶段把关"

**适合**: 跨阶段协调者（Leader / 接手整个 R14 阶段的成员）或新加入的长期成员。

1 小时路径 + 加读:

- **`APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md`**（主手册 6546 行）— R11 末态 + R12 接手全记录
- **所有 19 份 `stage2-decisions-*.md`** — 按本 README §"决策依赖图" 顺序
- **所有 13 份 `stage3-blueprints/`** — 完整图纸 + 借鉴 + 说明 + 双洋葱桥接
- **所有 4 份 `stage4/`** — 落实架构 + leader 思考 + 灵感 + 补丁
- **`reports/`** 关键报告

---

## 🏛️ 主哲学 anchor 6 个全贯穿（所有阶段）

```
主 22:33 S-1 北极星导向   — 所有决策服务 ASI 方向
主 17:43 S-2 实事求是      — 基于 R11 现状 + 不重写 LOCKED
主 17:58 O-5 不假装        — 12 键编译时 hardcode
主 19:33 O-2 走在前人经验上 — Hermes/OpenClaw/VCP/claude-mem + 20 优秀项目（20 项灵感补充）
主 23:44 O-3 干到底        — 文件夹规整 + 阶段 1-4 全部 LOCKED + 阶段 5/6 待落
主 00:56 O-4 任何人都能接手 — 顶层保留 LOCKED 主文档 + 子目录 README 索引
```

---

## 🔧 R14 文件夹规整（2026-07-31 完成）

**触发**: 主人指令"规整一下 Apeireth-rust 的文件夹和其中阶段一二三四的内容，便于下一阶段的施工"。

**操作**:
- ✅ 31 平铺文档 → 5 子目录化（stage1/2/3/4 + r14-design + research）
- ✅ 顶层保留 6 重要文档（CONTEXT-HANDOVER / START-HERE / README / STRUCTURE-R14 / 主手册 + 4 LOCKED 主文档）
- ✅ 5 个子目录各新建 README 索引
- ✅ 2 个既有 README 更新（docs/README.md + docs/stage3-blueprints/README.md）
- ✅ **不修改任何 LOCKED 内容**（仅移动 + 创建索引）

**详见**: `STRUCTURE-R14.md`（规整提案 + 实施步骤 + 拍板位置 + 6 锚自检）

---

## 🔜 下一阶段（阶段 5 设计施工文档）

阶段 5 = **施工图纸** = 把阶段 4 落实架构 → 工程化：

---

## 📋 ADR / Drift / Reports 三张目录索引（2026-08-02 新增，让文档不再隐身）

> **目的**：把"工程期新增但默认不在 stage1-5 LOCKED 文档中"的 ADR + drift 报告 + 本轮 reports 加入可见索引。
> **触发**：fe603044 任务（technical_writer）README 索引更新。
> **约束**：❌ 不修改任何 LOCKED 文档（阶段 1-5）；✅ 仅新增命名空间独立索引。

### ADR 目录索引（`docs/adr/`）

| # | 文件 | 状态 | 摘要 |
|---|---|:---:|---|
| [ADR-0001](adr/0001-double-onion-unity.md) | double-onion-unity（双洋葱统一体）| ✅ | 原则洋葱嵌入权限洋葱，不是两个独立锁 |
| [ADR-0002](adr/0002-cli-session-api-binding.md) | cli-session-api-binding（CLI 接入 core Session API）| ✅ | CLI lib 抽象层绑定模式，main.rs 仅走 lib |
| [ADR-0007](adr/0007-compat-components-layer.md) | compat-components-layer（兼容组件层）| ✅ | 3 类统一抽象（Plugin + PyO3 + MCP）+ 3 种部署模式 |
| [ADR-0008](adr/0008-feature-gating-pybridge.md) | feature-gating-pybridge（PyBridge 默认 feature-gated）| ✅ | `default = []` + `python-ext = pyo3/extension-module` |
| [ADR-0009](adr/0009-integration-rebase-skip-policy.md) | integration-rebase-skip-policy（rebase skip 策略）| ✅ | `team_conflict_skip` 4 项触发 + 4 项反例 + 决策树 |

### Drift 报告索引（`reports/`）

| 文件 | 范围 | 状态 |
|---|---|---|
| [drift-stage4-§2.3-sovereignty-17vs18vs24-2026-08-02.md](../reports/drift-stage4-§2.3-sovereignty-17vs18vs24-2026-08-02.md) | stage4 §2.3 + stage5 §2.3 Sovereignty 漂移 | 🟡 报告完成（不修改 LOCKED）|
| [P30-sovereignty-drift-stage5-crate-count-report.md](../reports/P30-sovereignty-drift-stage5-crate-count-report.md) | stage5 §2 17 ↔ 18 crate 修订（前置报告）| ✅ |
| [c0cbd0b3-57f8-440c-92a8-f3d057ecc163-technical-writer-requirement-validation-signoff.md](../reports/c0cbd0b3-57f8-440c-92a8-f3d057ecc163-technical-writer-requirement-validation-signoff.md) | 需求裁决与用户有效性确认单（4 冲突）| 🟡 待用户签收 |
| [V17-writer-stage1-2-traceability.md](../reports/V17-writer-stage1-2-traceability.md) | 阶段 1-2 需求追踪矩阵（130 条 LOCKED → 代码 → 测试）| ✅ |
| [V18-writer2-stage3-4-traceability.md](../reports/V18-writer2-stage3-4-traceability.md) | 阶段 3-4 追踪矩阵 | ✅ |
| [V25-cargo-workspace-acceptance.md](../reports/V25-cargo-workspace-acceptance.md) | Cargo workspace 真实集成验证 | ✅ |
| [V26.1-cargo-workspace-independent-verification.md](../reports/V26.1-cargo-workspace-independent-verification.md) | 独立旁路 cargo workspace 验证 | ✅ |
| [V26-cargo-build-test-real-validation.md](../reports/V26-cargo-build-test-real-validation.md) | V26 真实 build/test 验证（architect2）| ✅ |
| [V26.2-real-validation-baseline-2026-08-02.md](../reports/V26.2-real-validation-baseline-2026-08-02.md) | V26.2 real validation baseline | ✅ |
| [stage1-5-implementation-gap-matrix-2026-08-02.md](../reports/stage1-5-implementation-gap-matrix-2026-08-02.md) | 阶段 1-5 实现缺口矩阵 | ✅ |
| [round5-engineering-decisions-tasks.md](../reports/round5-engineering-decisions-tasks.md) | round5 工程决定 → 派活清单 | ✅ |
| [P29-pybridge-restoration.md](../reports/P29-pybridge-restoration.md) | P29 PyBridge 恢复报告 | ✅ |
| [P30-sovereignty-drift-stage5-crate-count-report.md](../reports/P30-sovereignty-drift-stage5-crate-count-report.md) | P30 sovereignty 漂移报告 | ✅ |
| [P31-seven-bugs-fix-report.md](../reports/P31-seven-bugs-fix-report.md) | P31 7 处破损修复报告 | 🟡 4/7 完全关闭 + 3/7 部分关闭 |

### Reports 索引（按类型分）

| 类型 | 路径前缀 | 数量（约）| 状态 |
|---|---|---:|---|
| 验收报告 (V 系列) | `V*.md` | 17+ | 🟢 持续累计 |
| 漂移报告 (drift) | `drift-*.md` | 1（2026-08-02 新增）| 🟡 阶段 4+ 启动后扩展 |
| 工程期决定 (P 系列) | `P*.md` | 31+ | 🟢 |
| 成就 (achievement) | `achievement-*.md` | 20+ | 🟢 |
| 需求裁决 / 签收 (technical-writer) | `*-technical-writer-*.md` | 2（V17 + c0cbd0b3）| 🟡 |
| 索引 / README 更新 | `*-readme-index-update.md` | 1（本轮 fe603044）| 🟢 |
| R12 / R13 / R14 baseline | `r12-*.md` / `r13-*.md` / `r14-*.md` | 30+ | 🟢 |

- **9 crate 占位重写**（按阶段 4 §2 18 crate 推导）
- **V0.5 v2 24 维落地**（v4.1 §13 提议真正改到 v1077 原始）
- **V1136 v2 9 子测度落地**（v4.1 §14 提议真正改到 v1136 原始）
- **V3 v2 12 键落地**（v4.1 §15 提议加 v2 章节到 philosophy-traits）
- **R11 1100 重写方案**（保留 + 归档 / 合并 / 重写 / 砍）
- **Cargo.toml metadata 更新**
- **5 重守门编译时 hardcode 实现**
- **OTA 升级 7 阶段工程化**

---

_主哲学 anchor 6 个全贯穿. 54 文档按阶段 + R14 周期规整. 任何接手者能查. 不会丢失上下文._
_下次对话启动: 阶段 5 施工 or 阶段 6 验证 or 主人拍板下一步._

---

## R19+ 集成期 索引 (2026-08-05 追加)

> 本节是 2026-08-05 R19+ 集成期追加的索引, R14-era 入口 (前 220 行) 完整保留.

### R19+ 集成期入口

1. **新人 5min**: `stage4/r19-integration-quickstart-2026-08-05.md` §2
2. **看全局**: `stage4/global-architecture-map-2026-08-05.md` §2.1 (13 张 Mermaid)
3. **看具体实施**: `stage4/r19-integration-doc-index-2026-08-05.md` §2-§3 (35 份索引)
4. **拍板 12 项**: `stage4/pending-decisions-overview-2026-08-05.md` §2 (D-01 ~ D-12)
5. **8 项 LOCKED**: `stage4/8-locked-unified-2026-08-05.md` §2

### docs/ 子目录 R19+ 新增 (15 份)

**ADR (3 份, 0010-0012)**:

| ADR | 文件 | 大小 | 摘要 |
|---|---|---:|---|
| [ADR-0010](adr/0010-mcp-from-spectrai-agentmcpserver.md) | mcp-from-spectrai-agentmcpserver | 12.7KB | mcp 翻译决策 |
| [ADR-0011](adr/0011-apeireth-team-lead-supervisor-prompt-translation.md) | apeireth-team-lead-supervisor-prompt-translation | 19KB | A 方案拍板 |
| [ADR-0012](adr/0012-team-lead-council-collaboration.md) | team-lead-council-collaboration | 13.8KB | trait 协同 |

**roadmap (1 份)**:

| 文件 | 大小 | 摘要 |
|---|---:|---|
| `roadmap/r20-product-finalize-2026-08-05.md` | 28.3KB | R20 收产品 |

**stage4 (16 份)**:

蓝图 / 集成 (2 份):

| 文件 | 大小 | 摘要 |
|---|---:|---|
| `stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` | 60.9KB | 0 蓝图 |
| `stage4/global-architecture-map-2026-08-05.md` | 47.8KB | 13 张 Mermaid |

实施指南 (5 份):

| 文件 | 大小 | 摘要 |
|---|---:|---|
| `stage4/apeireth-team-lead-implementation-guide-2026-08-05.md` | 40.1KB | team-lead 实施 |
| `stage4/apeireth-session-blueprint-2026-08-05.md` | 75KB | session 蓝图 |
| `stage4/apeireth-formal-invariants-2026-08-05.md` | 62.3KB | 5 Kani invariants |
| `stage4/r-measure-verification-design-2026-08-05.md` | 36KB | R-Measure 守门 |
| `stage4/apeireth-sdk-gap-analysis-2026-08-05.md` | 20.7KB | sdk 现状 |

R20 实施 (2 份):

| 文件 | 大小 | 摘要 |
|---|---:|---|
| `stage4/r20-stage-1-2-implementation-2026-08-05.md` | 53.3KB | 10 子阶段 |
| `stage4/r20-stage-3-5-implementation-2026-08-05.md` | 54.8KB | 13 子阶段 |

资产 / 协同 (3 份):

| 文件 | 大小 | 摘要 |
|---|---:|---|
| `stage4/tauri-assets-from-spectrAI-2026-08-05.md` | 15.9KB | 13 项 T-001~T-013 |
| `stage4/tauri-team-collab-sop-2026-08-05.md` | 22.8KB | Tauri SOP |
| `stage4/glossary-spectrAI-additions-2026-08-05.md` | 11.1KB | 8 词条 |

元规范 / 工具 (4 份, NEW 统一):

| 文件 | 大小 | 摘要 |
|---|---:|---|
| `stage4/8-locked-unified-2026-08-05.md` | 15.7KB | 8 项 LOCKED 统一 |
| `stage4/r19-r20-stage-unified-2026-08-05.md` | 18.7KB | 3 套阶段对照 |
| `stage4/pending-decisions-overview-2026-08-05.md` | 13.2KB | 12 项 D-# |
| `stage4/r19-integration-doc-index-2026-08-05.md` | 15.9KB | 35 份总索引 |

维护 / 模板 (3 份):

| 文件 | 大小 | 摘要 |
|---|---:|---|
| `stage4/docs-maintenance-sop-2026-08-05.md` | 30.9KB | 5 步 SOP |
| `stage4/r19-integration-commit-template-2026-08-05.md` | 38.6KB | 5 类模板 |
| `stage4/r19-integration-quickstart-2026-08-05.md` | 25.3KB | 5min 5 步 |

拍板占位 (1 份):

| 文件 | 大小 | 摘要 |
|---|---:|---|
| `stage4/d-01-d-12-commit-plan-2026-08-05.md` | 19.4KB | 12 项 D-# 占位 commit 计划 |

风险 (1 份, 在 reports/ 不在 docs/):

| 文件 | 大小 | 摘要 |
|---|---:|---|
| `../../spectrai/reports/r19-risks-v2-2026-08-05.md` | 31.4KB | 30 风险总表 (P0/P1/P2) |

Mermaid 总览 (1 份):

| 文件 | 大小 | 摘要 |
|---|---:|---|
| `stage4/r19-integration-mermaid-overview-2026-08-05.md` | 45KB | 6 张 Mermaid |

### 8 项不修改承诺 (per 8-locked-unified §2)

1. 阶段 1+2+3 LOCKED
2. v2/v4/v4.1 LOCKED
3. 阶段 4 核心 LOCKED (6ca80776)
4. 阶段 5 施工 LOCKED (631 行)
5. v6 基础架构
6. R11 baseline 3 值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063)
7. APEIRETH-CONVENTIONS / VERSIONING / GLOSSARY
8. workspace version 1.0.0

### 6 哲学 anchor (per APEIRETH-CONVENTIONS §9)

S-1 / S-2 / O-5 / O-2 / O-3 / O-4

### 关键数字 (2026-08-05)

- docs/ 文档总数: **25 份** (R19+ 新增 15 份 + LOCKED 9 份 ADR + 顶层 README 1 份)
- reports/ 文档总数: **15 份** (R19+ 新增 14 份 + 顶层 README 1 份)
- 12 项 D-# 待 Mavis 拍板 (per `stage4/pending-decisions-overview`)
- 5 阶段 R19+ 路线 (per `stage4/r19-r20-stage-unified` §3)
- 8 项 LOCKED (per `stage4/8-locked-unified` §2)
- 30 项风险 (per reports/r19-risks-v2)
- 6 张 Mermaid 总览 (per `stage4/r19-integration-mermaid-overview`)

### 关联 reports/ 索引

R19+ 集成期 reports/ 15 份, 见 `../../spectrai/reports/README.md`.

### 关联文档 (Mermaid / 风险 / 收口)

- `stage4/r19-integration-mermaid-overview-2026-08-05.md` — 6 张 Mermaid 总览图
- `../../spectrai/reports/r19-risks-v2-2026-08-05.md` — 30 项风险总表
- `../../spectrai/reports/r19-integration-wrap-up-v2-2026-08-05.md` — v2 总收口