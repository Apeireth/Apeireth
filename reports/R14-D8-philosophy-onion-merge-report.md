# R14-D8：哲学守门并入洋葱内墙 + V0.5/V1136 基线化措辞精化

## 1. 任务元信息

| 字段 | 值 |
|---|---|
| 任务 ID | `57871a3e-4d0c-48ac-8f75-f07572110536` |
| 角色 | 技术文档（technical_writer） |
| 触发 | 主人 2026-07-31 走法乙: (1) 哲学守门并入原则洋葱外层，不是独立 crate；(2) V0.5/V1136 是 R11 对照基线，不是 R14 重设计对象 |
| 修改性质 | 原文保留 + 最小追加/并列精化 + 新增 1 份概念文档 |
| 提交主题 | `R14-D8：哲学守门并入洋葱内墙 + V0.5/V1136 基线化措辞精化` |

## 2. 修改摘要（4 文件）

### 文件 1：`Apeireth-rust/crates/README.md`（8 处最小精化）

| # | 位置 | 改动 |
|---|------|------|
| 1 | 顶部 `本版本` 行 | 追加 R14-D8 主人精化版本说明 |
| 2 | 表格 `apeireth-asi` 行 | 措辞改为"借 R11 真测作 baseline（不重写 V0.5 公式）" |
| 3 | 表格 `apeireth-bench` 行 | 措辞改为"借 R11 真测作 baseline（不重做 V1136 真测引擎）" |
| 4 | 表格 `apeireth-core` 行 | 追加"洋葱内墙模块（原则洋葱+权限洋葱交叉咬合）" |
| 5 | 表格 `apeireth-philosophy` 行 | 整行删除式标注：~~`apeireth-philosophy`~~ + R14-D8 主人精化说明 + "已并入 core" |
| 6 | §3 30 crate v1 块 | 合并"原则洋葱层" + 删除"权限层"行 → "原则/权限洋葱内墙层: 合并入 core" |
| 7 | §4 v2 收敛备选块 | 追加 3 项已实现合并: principle → core / philosophy → core / permission → core |
| 8 | §5 R11→9 crates 映射表 | `v1003 / v1121` 行映射列改为"已并入 core" |

**硬约束遵守**: cargo metadata `description` 字段未触碰（仅修改 README 表格与措辞）。

### 文件 2：`Apeireth-rust/docs/stage2-decisions-philosophy-guard.md`（2 处最小插入）

| # | 位置 | 改动 |
|---|------|------|
| 1 | 文档头（"---" 之后） | 新增"R14-D8 主人精化勘误"段，明确哲学守门并入 onion_wall/、9 键守护对象迁移、V0.5/V1136 改为 R11 基线 |
| 2 | §2.1 段前 | 新增 R14-D8 标注行，说明 trait 现在归 `onion_wall/keys/` 模块 |

**硬约束遵守**: 21KB 既有主体内容 0 改动（grep 验证 §2.1-§10 全部保留）。

### 文件 3：`Apeireth-rust/docs/philosophy-traits-2026-07-30.md`（1 处最小插入）

| # | 位置 | 改动 |
|---|------|------|
| 1 | 文档头（"---" 之后） | 新增"R14-D8 主人精化勘误"段，明确 9 键 trait 框架保留为辅助语义网、唯一守护对象迁移为阶段1+2 沉淀的 9 类决策 |

**硬约束遵守**: 既有 trait 框架（`PhilosophyKey` 枚举、`PhilosophyChecker`、5 项不假装）0 改动。

### 文件 4：`Apeireth-rust/docs/onion-wall-architecture-2026-07-31.md`（新增 363 行 / 7 节）

| § | 标题 | 关键内容 |
|---|------|----------|
| §0 | 范围声明 | 触发 / 依据 / 不修改承诺（6 条硬约束） |
| §1 | 比喻起源（主人原话 + 走法乙） | 主人 2026-07-31 原话引用 + 走法甲 vs 走法乙对比表 + 走法乙的 3 核心细节 |
| §2 | 内墙咬合形态 | ASCII 整体洋葱结构图 + per-layer 双重过滤示意 + 13 项阶段 1+2 沉淀的衔接表 |
| §3 | 模块边界映射 | `apeireth-core/src/onion_wall/` 12 子模块结构 + `OnionGate` trait 签名 + `DecisionSignature` 结构 + 10 项守门映射表 |
| §4 | 9 键 → 决策签名迁移 | 9 键保留为辅助语义网，列出 9 键 → DecisionCategory 的映射关系 |
| §5 | V0.5 / V1136 在 R14 的角色定位 | 明确 R11 对照基线（v1077 / v1106），不重写不重做；与 crates/README 措辞对应；与既有文档的关系 |
| §6 | 阶段 4 衔接锚点 | 给阶段 4 SCHEMA.md / ADR.md + 阶段 5 施工的衔接清单 + 边界声明锚点 + 不做事清单 |
| §7 | 主哲学 anchor 6 个全贯穿 | 表格形式逐项映射 |

## 3. 完整目标文档 diff

> 本节为 3 份修改文件 + 1 份新增文件的完整 diff。报告文件自身是新增交付物，不纳入本节。

```diff
diff --git a/Apeireth-rust/crates/README.md b/Apeireth-rust/crates/README.md
index 02b31289..53df66f1 100644
--- a/Apeireth-rust/crates/README.md
+++ b/Apeireth-rust/crates/README.md
@@ -3,7 +3,7 @@
 > **范围**: Apeireth Rust 重写源代码 (R14 Phase 1+ 阶段 3+)
 > **当前**: 9 个 crate 占位实现 (R11 已落)
 > **目标**: 30 个 crate (阶段 2 §3 设计, B+E 架构)
-> **本版本**: R14-D6-C E4 重写 (新增 "R11 对应模块" 列, 不动 cargo metadata description)
+> **本版本**: R14-D6-C E4 重写 (新增 "R11 对应模块" 列, 不动 cargo metadata description) → R14-D8 主人精化 (哲学守门并入 onion_wall/ 内墙, V0.5/V1136 改为 R11 对照基线)

 ---

@@ -11,12 +11,12 @@

 | Crate | 职责 | R11 对应模块 | R11 状态 | 阶段 2 设计 |
 |-------|------|------------|---------|-----------|
-| `apeireth-asi` | ASI 北极星导向 + V0.5/V1136 重设计 | **v1077 + v1101 + v1106 + v1115** (4 个真生产 Python 锚点: ASI V0.4 17 维真测 / V0.4 维度自动拉升 / 真工程 (error handling/retry/circuit breaker/health check/metrics) / Cognitive-Dream e2e 真集成) | ✅ 占位 | 保持 |
-| `apeireth-bench` | 性能基准 (V1130 wallclock 验证) | **v1012 + v1106** (2 个真生产 Python 锚点: SWE-bench/MMLU 真借鉴 agent benchmark / 工程韧性基准点) | ✅ 占位 | 保持 |
+| `apeireth-asi` | ASI 北极星导向 + **借 R11 真测（v1077/v1101）作 baseline（不重写 V0.5 公式）** | **v1077 + v1101 + v1106 + v1115** (4 个真生产 Python 锚点: ASI V0.4 17 维真测 / V0.4 维度自动拉升 / 真工程 (error handling/retry/circuit breaker/health check/metrics) / Cognitive-Dream e2e 真集成) | ✅ 占位 | 保持 |
+| `apeireth-bench` | 性能基准 (V1130 wallclock) + **借 R11 真测（v1012/v1106）作 baseline（不重做 V1136 真测引擎）** | **v1012 + v1106** (2 个真生产 Python 锚点: SWE-bench/MMLU 真借鉴 agent benchmark / 工程韧性基准点) | ✅ 占位 | 保持 |
 | `apeireth-cli` | CLI 入口 + TUI + slash commands | **v1009 + v1016** (2 个真生产 Python 锚点: FastAPI 真借鉴 Web UI / FastAPI+Kong 真借鉴 REST gateway) | ✅ 占位 | 保持 |
-| `apeireth-core` | 核心抽象 (traits / 错误 / 配置) | **v1004 + v1107 + v1108 + v1115** (4 个真生产 Python 锚点: V49 DGM+UCB1 bandit 自演化 / IDENTITY 5 Module + 真认知能力 / 6 状态机 (IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED) / Cognitive-Dream 真贯连) | ✅ 占位 | 保持 |
+| `apeireth-core` | 核心抽象 (traits / 错误 / 配置) + **洋葱内墙模块（原则洋葱 + 权限洋葱交叉咬合，守护阶段1+2 沉淀的具体决策）** | **v1004 + v1107 + v1108 + v1115** (4 个真生产 Python 锚点: V49 DGM+UCB1 bandit 自演化 / IDENTITY 5 Module + 真认知能力 / 6 状态机 (IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED) / Cognitive-Dream 真贯连) | ✅ 占位 | 保持 |
 | `apeireth-memory` | 记忆系统 (A/M 层经验沉淀) | **v1005 + v1019 + mvp/memory/** (2 真生产 Python 锚点 + mvp/ 4 文件: AnySearch 调研结果索引 23 真调研文档 / OpenAI+BAAI bge-m3 真借鉴 embeddings / store.py + retrieve.py + consolidate.py + forget.py 13 文件 2292 insertions) | ✅ 占位 | 扩容 |
-| `apeireth-philosophy` | V3 9 键 + 5 项不假装 | **v1003 + v1121** (2 个真生产 Python 锚点: V4 哲学完整版 (V3 7 哲学问题 + V2 5 位置 + 5 哲学方法论 Popper/Kuhn/Lakatos/Feyerabend/Laudan + V166 真哲学) / R11-SEC-001 (OWASP Top 10 + NIST SSDF + STRIDE + Identity 守门 + 5 项不假装)) | ✅ 占位 | 增强 |
+| ~~`apeireth-philosophy`~~ | ~~V3 9 键 + 5 项不假装~~ | **R14-D8 主人精化**: 哲学守门不再独立 crate；并入 `apeireth-core/src/onion_wall/`（原则洋葱 + 权限洋葱交叉咬合形成"城堡内墙"，守护阶段1+2 沉淀的具体决策）。原 9 键 + 5 项不假装**保留为历史轨迹**（见 `docs/onion-wall-architecture-2026-07-31.md` §4）。 | ~~占位~~ | **已并入 core** |
 | `apeireth-pybridge` | PyO3 兼容桥 (1100+ Python 模块) | **(新) PyO3 桥接 1100+ v*.py 模块** (`apeireth/v1000-v1155*.py` 1100+ 真生产 Python 模块, R11 不砍, R14 不强求重写, 只在 pybridge 层桥接, 性能优化留给 Phase 2+) | ✅ 占位 | 保持 |
 | `apeireth-test` | 测试基础设施 | **v1114 + v1115** (2 个真生产 Python 锚点: R9-INT-003 每周集成评估器 (三件套真测 + ASI dashboard + 4 选 1 主轨道 + 守门自检) / Cognitive-Dream e2e 真集成) | ✅ 占位 | 扩容 |
 | `apeireth-tools` | 工具集合 | **v1000 + v1027** (2 个真生产 Python 锚点: safe YAML serialization (PyYAML safe_load/safe_dump + ruamel round-trip, 借 Letta/LangGraph/VCPToolBox) / validator/schema (借 JSON Schema + Pydantic + Cerberus + V116 整合)) | ✅ 占位 | 拆分 |
@@ -36,9 +36,8 @@
 核心抽象层 (2):     core, runtime
 智能层 (3):         asi, sovereignty, prompt
 智囊团层 (1):       council
-原则洋葱层 (2):     principle, philosophy
+原则/权限洋葱内墙层: 合并入 core（principle + philosophy + permission 合并到 core/onion_wall/）— R14-D8 主人精化
 经验方法论层 (4):   memory, experience, methodology, reflection
-权限层 (2):         permission, keys
 兼容组件层 (5):     plugin, tools, pybridge, mcp, environment
 升级层 (1):         upgrade
 通信总线层 (4):     bus, gateway, server, supervisor
@@ -47,7 +46,7 @@
 监控层 (2):         telemetry, config
 测试层 (3):         test, bench, eval

-合计: 9 + 21 = 30 个 crate (v1 推荐)
+合计: 9 + 21 = 30 个 crate (v1 推荐) — 原则洋葱 / 权限洋葱 / 哲学守门全部并入 core 内墙 (R14-D8)
 ```

 ## v2 收敛备选 (8 个合并)
@@ -61,6 +60,9 @@
 - skills → tools
 - telemetry → server
 - eval → test
+- **principle → core** (R14-D8 主人精化: 原则洋葱并入 onion_wall/ 内墙)
+- **philosophy → core** (R14-D8 主人精化: 哲学守门并入 onion_wall/ 内墙)
+- **permission → core** (R14-D8 主人精化: 权限洋葱并入 onion_wall/ 内墙)

 ---

@@ -72,7 +74,7 @@
 | v1009 / v1016 (入口层) | 2 | cli |
 | v1004 / v1107 / v1108 (核心抽象层) | 3 | core |
 | v1005 / v1019 / mvp/memory (记忆层) | 6 (含 mvp 4 文件) | memory |
-| v1003 / v1121 (原则洋葱层) | 2 | philosophy |
+| v1003 / v1121 (原则洋葱层) | 2 | **已并入 core** (R14-D8: philosophy → core/onion_wall/) |
 | v1000 / v1027 (工具层) | 2 | tools |
 | v1012 (bench 层) | 1 | bench |
 | v1114 / v1115 (test 层, 与 asi/core 共享) | 2 | test |

diff --git a/Apeireth-rust/docs/stage2-decisions-philosophy-guard.md b/Apeireth-rust/docs/stage2-decisions-philosophy-guard.md
index 2c9b9a2b..07481d16 100644
--- a/Apeireth-rust/docs/stage2-decisions-philosophy-guard.md
+++ b/Apeireth-rust/docs/stage2-decisions-philosophy-guard.md
@@ -6,6 +6,20 @@

 ---

+## R14-D8 主人精化勘误 (2026-07-31)
+
+> **本节性质 (主 17:58 不假装 + 主 17:43 实事求是)**: 在不删除、不重写本文档 21KB 既有内容的前提下, 由主人 2026-07-31 最新精化追加的**勘误与边界声明**。
+> 原 21KB 内容**完整保留为历史轨迹**——"以前没说错, 只是现在看得更清"。
+
+- **主人 2026-07-31 精化**: 哲学守门**不是独立 crate**, 而是原则洋葱 + 权限洋葱**交叉咬合**形成的"城堡内墙", 归入 `apeireth-core/src/onion_wall/` 模块。
+- 本文档假设的"`apeireth-philosophy` crate 独立实现"已**过时**——按 R14-D8 主人走法乙, 哲学守门与原则洋葱、权限洋葱三者合并到 `apeireth-core/onion_wall/`, 不再独立。
+- V3 9 键 + 5 项不假装**已过时**——唯一守护对象 = **阶段1+2 沉淀的具体决策** (双根 / 双洋葱 / 三件套 / 七席 / L1-L5 / MEWG / HA / 旧规则合法性 / 漂移 P0)。9 键作为抽象 trait 框架**保留为历史轨迹** (见 `docs/philosophy-traits-2026-07-30.md`), 守护对象已迁移到 `OnionGate::guard_decision(decision: DecisionSignature)` (见 `docs/onion-wall-architecture-2026-07-31.md` §3-§4)。
+- V0.5 / V1136 在 R14 角色 = **R11 对照基线 (v1077 / v1106)**, **不重写不重做**——见新文档 `docs/onion-wall-architecture-2026-07-31.md` §5。
+- **本文档保留为历史轨迹, 不重写** (主 17:58 不假装"以前没说过")。
+- 详细架构与走法乙的 3 细节见 `docs/onion-wall-architecture-2026-07-31.md`。
+
+---
+
 ## 0. 元信息

 | 字段 | 值 |
@@ -47,6 +61,8 @@

 ## 2. V3 9 键 trait 框架 (强化)

+> **R14-D8 标注**: 以下 trait 现在归 `apeireth-core/src/onion_wall/keys` 模块（**不再在独立 philosophy crate**）。模块路径变化仅为归属调整，trait 签名与 9 键语义**保留**作为历史轨迹，与 `docs/philosophy-traits-2026-07-30.md` 一致。具体迁移到 `OnionGate::guard_decision(decision: DecisionSignature)` 的映射见 `docs/onion-wall-architecture-2026-07-31.md` §4。
+
 ### 2.1 trait 定义 (在 apeireth-philosophy crate)

 ```rust

diff --git a/Apeireth-rust/docs/philosophy-traits-2026-07-30.md b/Apeireth-rust/docs/philosophy-traits-2026-07-30.md
index cd95923f..75ad8a82 100644
--- a/Apeireth-rust/docs/philosophy-traits-2026-07-30.md
+++ b/Apeireth-rust/docs/philosophy-traits-2026-07-30.md
@@ -2,6 +2,26 @@

 > **范围声明** (主 17:43 实事求是 + 主 17:58 不假装): 本文档是 **R14 Rust 重写 Phase 2 (V0.5/V1136/哲学守门 Rust 重设计, 6 周目标) 的前置 trait 框架**. 仅记录 Rust trait 接口 + 错误类型 + 集成模式, **不写完整实现** (Phase 2 团队根据本框架 + T23 §4 Phase 2 + T26 workspace 骨架 + T27 Python → Rust trait 规范实现). 主人哲学硬约束 6 大 anchor 全保留, **不重写规则, 只用 Rust 重实现** (规则由 V3 9 键 LOCKED + 5 项不假装规则固定). 不动 apeireth/v*.py 1100+ 个 v 模块 / 不动 mvp/ 子项目 / 不动主手册 / 不砍 1100 空壳 / 不写 ASI 公式.

+## R14-D8 主人精化勘误 (2026-07-31)
+
+> **本节性质 (主 17:58 不假装 + 主 17:43 实事求是)**: 在不删除、不重写本文档既有 trait 框架的前提下, 由主人 2026-07-31 最新精化追加的**勘误与边界声明**。
+> 原 9 键 trait 框架**完整保留为历史轨迹**——"抽象 trait 没有错, 只是守护对象已迁移"。
+
+- **主人 2026-07-31 精化**: V3 9 键 + 5 项不假装**已过时**——按 R14-D8 主人走法乙, 9 键作为抽象 trait 框架**保留**（trait 签名 / 错误类型 / 集成模式不变）, 但**唯一守护对象**已迁移为 **阶段1+2 沉淀的具体决策**：
+  - 阶段 1 §18 双根 (原则根 + 权限根)
+  - 阶段 1 §18.7 双洋葱 (原则洋葱 + 权限洋葱 + 洋葱 0 层真实人类批准)
+  - 阶段 1 §18.5 平台三件套 (提供 / 约束 / 记录)
+  - 阶段 1 §18.8 七席审议庭 (风险分级 → 席位触发)
+  - 阶段 1 §18.9 L1-L5 分层验证网
+  - 阶段 2 §8 MEWG 多证据加权治理
+  - 阶段 2 §9 HA 人类批准硬门槛
+  - 阶段 2 §10 旧规则合法性
+  - 阶段 2 §14 漂移 P0 优先级
+- 守护入口从 `PhilosophyChecker::check(claim)` **迁移**到 `OnionGate::guard_decision(decision: DecisionSignature)` (签名见 `docs/onion-wall-architecture-2026-07-31.md` §3-§4)。
+- 9 键的字符串匹配 / 关键词检测**保留**作为辅助语义网 (semantic net), 但**不**再是主入口；主入口是 `DecisionSignature` 的结构化校验。
+- V0.5 / V1136 在 R14 角色 = **R11 对照基线**, **不重写不重做** (详见 `docs/onion-wall-architecture-2026-07-31.md` §5)。
+- **本文档保留为历史轨迹, 不重写** (主 17:58 不假装"以前没说错, 只是现在看得更清" + 主 23:44 不假装哲学守门已改变)。
+
 ---

 ## 0. 元信息 (主 17:43 实事求是)
```

> 文件 4 为新增（363 行 / 7 节），完整内容见 `Apeireth-rust/docs/onion-wall-architecture-2026-07-31.md`，本报告不重复贴出。

## 4. 14 项自检清单

- [x] **文件 1** crates/README.md：8 处最小精化措辞（顶部 1 + 表格 4 + §3 1 + §4 1 + §5 1）
- [x] **文件 1** cargo metadata `description` 字段未触碰（仅修改 README 表格）
- [x] **文件 2** stage2-decisions-philosophy-guard.md：头部 1 段 + §2.1 段前 1 标注，原 21KB 主体 0 改动
- [x] **文件 3** philosophy-traits-2026-07-30.md：头部 1 段，原 trait 框架 0 改动
- [x] **文件 4** 新增 onion-wall-architecture-2026-07-31.md（363 行 / 7 节：§1-§7）
- [x] **未写新 Rust 代码**（仅 trait 签名 stub，不写实现）
- [x] **未画 Mermaid 图**（仅 ASCII 简化示意）
- [x] **未重写 V0.5 / V1136 / 哲学守门 9 键**（保留为历史轨迹 + 勘误标注）
- [x] **未修改其他 16 份 stage2 文档**
- [x] **未修改 crates/ 占位实现**（仅 crates/README）
- [x] **未修改 cargo metadata `description` 字段**
- [x] **原文措辞保留为历史轨迹 + 加勘误标注**
- [x] **主 17:58 不假装**：6 个主哲学 anchor 在新文档 §7 全贯穿
- [x] **主 17:43 实事求是**：走法乙 3 细节基于 R11 现状 + 主人洞察，不假装"以前没说错"

## 5. 边界声明锚点

- **主 17:58 不假装**: 哲学守门**不是**独立 crate；V0.5 / V1136 **不是** R14 重设计对象；9 键 trait 框架**保留**为历史轨迹
- **主 17:43 实事求是**: 走法乙的 3 细节是基于 R11 现状 + 主人洞察的精化，不假装"以前没说错, 只是现在看得更清"
- **主 19:33 走在前人经验上**: 嵌套洋葱是经典的分层架构思想；per-layer 双重过滤借鉴权限模型的"AND 门"思路
- **主 22:33 ASI 北极星**: 洋葱核心 0 层 = 真实人类批准，保留最后护栏
- **主 23:44 干到底**: 9 键 + 5 项不假装 trait 框架**保留**为历史轨迹，不假装"哲学守门已改变"
- **主 00:56 任何人都能接手**: §0 范围 + §1 比喻起源 + §2 咬合形态 + §3 模块映射 + §6 衔接锚点 = 任何接手者能看清 R14-D8 演化脉络

## 6. 结论

R14-D8 完成了主人走法乙的 3 个核心细节（哲学守门并入 onion_wall/ 内墙 / 守护对象迁移到阶段1+2 沉淀的具体决策 / V0.5/V1136 改为 R11 对照基线），并通过 4 文件改动（3 措辞精化 + 1 新增概念文档）将走法乙的结构化、模块化、阶段 4 衔接全部沉淀，为阶段 4 SCHEMA.md / ADR.md 写作提供单一入口参考。
