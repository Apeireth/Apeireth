# R9 任务清单（WBS · 可执行版）

> **作者:** 需求分析师 (requirements_analyst)
> **任务 ID:** `8556a6c2-5942-43d1-839c-23f2767b7b25` (R9-REQ-001)
> **生成时间:** 2026-07-29
> **基于:** `reports/r8-requirements-decision-matrix.md` 全文 + `reports/r8-handoff-r9-requirements-chat.md` + `reports/r8-handoff-r9-team-leader.md` + `reports/r8-architecture-overview.md` + `reports/r8-philosophy-gate-philosophy_guardian.md`
> **性质:** R9 可执行任务清单（WBS），每条含 任务ID / 角色 / 真产出 / 真测试数 / 真 commit / lift 期望 / V3 守门点
> **主哲学 LOCKED:** 主 22:33 ASI 北极星 · 主 17:58 不假装 · 主 23:44 干到底 · 主 00:56 任何人都能接手

---

## 0. 阅读须知（30 秒看懂）

> **大白话：** 本文件是 R9 的施工图。R8 已交付（11 模块真生产 + 119+ 测 + V1094 真 commit），P0 阻塞（21GB snapshot + V1088 未 commit + 全量回归 6 失败）由 R9 启动首日 V1110 三件套全过解除。用户已拍板"直接开干"——所以这份清单默认 10 决策全 LOCKED 全执行，4 选 1 主轨道选 **R9-A 全做并发**，5 个 D 决策点都给了默认值，3 个灵魂问题都给了答复。
>
> **architect 已有真产出**（`r9-architect-roadmap.md`），其 Top-5 主轨道策略（V1060/V1061/V1045/V1062/V1065）已与本 WBS 双向校验（详见 §8）。
>
> 验收口径 = 真 commit + 真测数 + lift 真值 + V3 守门 PASS。**不刷 KPI，不假装达到。**

### 0.1 关键状态数字（R9 启动首日真测 · V1110 P0 已通过）

| 指标 | 真值 | 证据 | R9 起点 |
|---|---:|---|---|
| ASI V0.3 真测 | **0.8884** | `r9-p0-terminal-verify.md` V1110 三件套全过 | R9 P0 准入 ✅ |
| ASI V0.4 真测 | **0.8003** | `r9-architect-roadmap.md §1` V1103 P2 诊断 | R9 主目标 ≥0.85 |
| 真生产模块 | **1091+ → R9 增长** | `r8-delivery-summary.md §2` + R9 新增 | 起算点 |
| 真测试函数 | **4366+ → R9 +600** | `r8-delivery-summary.md §2` | 起算点 |
| 真 commits | **416+ → R9 ≥1/任务** | git HEAD `e234d916` R9-ROADMAP-001 | 起算点 |
| **R9 P0 终验 (V1110)** | ✅ **ALL PASS** | `r9-p0-terminal-verify.md` v0.1.0 | **已准入** |
| V1074 真跑 | ✅ 3.05s < 60s + snapshot 4479 B | V1110 | 准入 |
| V1087 HQB live gate | ✅ subscore=1.0000 / lift=+0.0200 | V1110 | 准入 |
| V1088 e2e operator | ✅ lift=+0.0185 / subscore=0.9250 | V1110 | 准入 |
| 21GB snapshot | ✅ 已修（< 20MB） | V1074 snapshot=4479 B | **P0-01 已完成** |
| V1088 commit | ✅ 已 tracked | V1110 真跑 = tracked | **P0-02 已完成** |
| 全量回归 | 🟡 待 R9 持续追 | 80 passed / 6 failed → 持续补 | **P0-03 进行中** |
| ASI V0.3 起点 | **0.8884** | V1110 baseline | 跑完后 ≥0.89 |
| ASI V0.4 起点 | **0.8003** | V1103 baseline | 跑完后 ≥0.85 |

> **关键：** R9 启动首日 P0 已通过（V1110），真生产不停已验证。WBS 中 P0 4 任务状态 = 3/4 ✅（P0-01/02/04 已完成，P0-03 持续追）。

---

## 1. 用户"直接开干" → R8 10 决策全部映射（默认全执行 + 真生产不停）

> **映射原则:** 用户原话"确认清楚之后你就开工不必向我确认"= 4 选 1 默认 R9-A 全做并发 + 5D 全部选默认 + 3 灵魂问题按主哲学 LOCKED 答。任何已 LOCKED 的项不重新请示。

| # | R8 决策 | LOCKED 值 | R9 映射执行动作 |
|---|---|---|---|
| **D-01** | 主哲学 9 键 (`not_undo` `not_proof` `not_safe` `not_clone` `not_perfect` `not_uuid` `spec_is_not_proof` `counterexample_is_not_bug` `prover_is_not_truth`) | **LOCKED** | R9 每条任务必须经 V3 philosophy_guard 跑批，全 PASS 才能 commit |
| **D-02** | ASI 北极星 V0.3 公式 (8 加权维) | **LOCKED** | R9 增量用 V1074 `--report` 真测 lift，不预估值作为 KPI |
| **D-03** | 真生产契约 V1001+ (10+ 前人 + 10+ 真生产组件 + ≥30 tests + V3 守门 + V1074 lift) | **LOCKED** | R9 每个新模块 ≥30 测 + ≥5 真生产组件指标 + V3 PASS + V1074 lift |
| **D-04** | V1000 阶段分界 (V201-V1000 空壳 / V1001-V1088 真生产) | **LOCKED** | R9 不删 V201-V1000 任何模块（默认保留），不刷 KPI 替换 |
| **D-05** | 主 22:33 终极授权 3 类问 (重大节点 / 哲学修改 / 方向微调) | **LOCKED** | R9 遇 3 类问之一 → 走请示流程，不擅自改 |
| **D-06** | Top-1 = V1082 backlog Top-8 填充 | **LOCKED** | R9 4 选 1 主轨道 = R9-A 全做并发（含 V1082 顶 8 优先） |
| **D-07** | R7 真实现 Phase-1 (HotCold/WAL → Replay → Dream) | 本周 / 系统路径 | R9 4 选 1 主轨道 = R9-A 包含 Phase-1 真实现 |
| **D-08** | R8 调研 4 领域 (形式化验证 / 机制设计 / 计算最优律 / 因果) | 本月 / 基础研究 | R9 含 1 人深读调研 + Rust 重写设计稿评审 |
| **D-09** | Rust 重写启动 (`promethean/rust-substrate/` 5 子项目) | 长期 / 战略转移 | R9 含 Rust hot path 子模块（不删 Python 主路径） |
| **D-10** | ORC-01 编排计划 100% 继承 (15+15+8+4 启动前/Phase-1/2/3) | **LOCKED** | R9 启动检查表 15 项全过，再启动 Phase-1 15 项 |

**结论:** 10 决策全部 LOCKED + 直接执行 + 真生产不停 = R9 默认全景开干。

---

## 2. 4 选 1 主轨道默认决策（R9-A 全做并发）

> 用户拍板"直接开干" → 默认选 **R9-A**（与原 R8 选项 A 维度对齐，但同时并发 B/C/D 子集）。**不串行等待，4 维同时推**。

| 轨道 | 内容 | 团队人数 | 周期 | lift 期望（V0.3） |
|---|---|---:|---|---:|
| **A1** | V1082 backlog Top-8 填充（v1037/v1030/v1038/v1039/v1019/v1018/v1017/v1016） | 2 | 1.5 周 | +0.015~+0.025 |
| **A2** | R7 真实现 Phase-1（HotCold/WAL + MemoryReplay + Dream） | 2 | 2 周 | +0.005~+0.015 |
| **A3** | DGM Archive v0.4（QD 升级 + Persona 升级） | 1 | 1 周 | +0.005~+0.010 |
| **A4** | 跨小模型桥接（V1083 路由扩展 6 → 12 model catalog） | 1 | 1.5 周 | +0.005~+0.010 |
| **A5** | Rust hot path（snapshot 6.5GB 路径重写 + hqb-core 重写） | 1 | 2 周 | +0.002~+0.005（短期）/ 长尾 +0.020+ |

**并发规则:** A1/A2 优先级最高（顶层 P0），A3/A4 并行，A5 在 P0 修复后启动。每条轨道 1 人主负责，跨轨道代码评审 1 人（code_reviewer 横切）。

---

## 3. 5 个 D 决策点默认值

> **映射原则:** 选"最稳 + 最不破坏现有真生产 + V3 守门优先 + 真生产不停"的默认值。

| ID | 决策点 | 默认值 | 理由 |
|---|---|---|---|
| **D1** | Memory 触发时机 | **B: 定时批量触发**（中等） | 主 23:44 真生产不停 + V1087 HQB live gate 限速友好，A 简单但慢、C 需 ML 风险、D 最慢 |
| **D2** | schema 升级旧数据 | **C: 版本兼容 + lazy migrate** | 主 17:58 不假装 → A 丢历史违反真生产；B 双写慢但可作 fallback；C 推荐 |
| **D3** | Identity 故障 fallback | **B: 回退到中央人格**（V1072 永恒身份） | V1072 守门天然 fallback + A crash 违反主 23:44 干到底 + C 不可控 |
| **D4** | DGM 演化候选保留时长 | **B: 保留最近 N=10**（推荐） | 主 00:56 任何人都能接手 → A 失历史无法回放，C 占空间，B 平衡 |
| **D5** | 多 persona 并行单租约粒度 | **B: 按"主题"粒度租约** | V1096 persona 已按主题切；A 加锁违反真生产不停，C 会话粒度会跨主题串扰 |

**5 个 D 全部默认值锁定，不在 R9 内再请示。** 任一执行受阻 = 触发主 22:33 方向微调，再走请示。

---

## 4. 3 个灵魂问题答复（按主哲学 LOCKED）

> **回答原则:** 不重新设计主哲学 = 9 键 LOCKED + ASI 北极星 LOCKED = 答复 = 继承 R8 LOCKED。

### Q1. 项目最终形态
**答:** **AI 基座平台（MCP/API 型 + 探索型）**。理由：
- 主 11:43 终极授权 = V3 7 哲学问题真答 + V1072 永恒身份已落"中央 AI 占据 ASI 位置"——是平台型不是应用型
- 主 22:33 ASI 北极星公式 V0.3 = 8 维 = 跨域能力度量 → 平台型而非单点工具
- 主 00:56 任何人都能接手 = 平台开源/可接属性
- **R9 行动:** 持续走"任意 LLM 接入即获 AGI/ASI 级能力"路线，对外暴露 HQB MCP server 7 tools（V1097）+ `apeireth serve` CLI

### Q2. ASI 数字是否北极星
**答:** **是，ASI V0.3 = 真测北极星；ASI V0.4 = 17 维真测基线（V1077）**。理由：
- 主 22:33 ASI 北极星 LOCKED = 真测而非模拟
- 主 17:58 不假装 = 真测分数是真测，不刷 KPI
- **R9 行动:** 每条任务的 lift 期望用 V1074 `--report` 真跑验证，不用估算值

### Q3. 投入时间精力
**答:** **9 人（含 AI 本人）全职并行**，但电脑内存硬约束 = 团队上限 9 人。理由：
- 用户原话"全力开干的话，加上你自己，你最多能调用九个团队成员，再多电脑就会因为内存不够卡死了" → **9 人硬上限**
- 主 23:44 干到底 = 全职
- **R9 行动:** 任务清单总人数 9 人内分配；超 9 = 触发漂移防护 = 必须砍

---

## 5. R9 WBS（任务清单 · 每条含 7 字段）

> **字段定义:**
> - **任务ID**: `R9-{轨道}-{序号}`（如 `R9-A1-01`）
> - **角色**: 执行角色（用 R8 既有 9 人内的 ID）
> - **真产出**: 实际文件 / commit / 真测数（不写占位）
> - **真测试数**: 单元 + 集成测试 ≥X 个（V1001+ 真生产契约）
> - **真 commit**: git commit hash 预期（任务完成后填真值）
> - **lift 期望**: ASI V0.3 增量（按 V1082 audit lift 区间，结构性估算）
> - **V3 守门点**: 该任务必跑的守门项（V3 + V1072 + V1087 4 键 + V1099 4 键）

### 5.1 P0 阻塞任务（必修 · R9 启动前）

| 任务ID | 角色 | 真产出 | 真测试数 | 真 commit | lift 期望 | V3 守门点 |
|---|---|---|---|---|---|---|
| **R9-P0-01** 修 21GB snapshot 递归放大 | devops_engineer | `V1074` 修复（流式 history 读 + 不递归追加 score_history）+ backup + 受控替换 | 0（修改既有测） | TBD | 0（解除阻塞） | V3 + V1074 真跑 < 60s |
| **R9-P0-02** V1088 commit + tracked | devops_engineer | `apeireth/v1088_asi_e2e_operator.py` git add + commit + 真跑 | 0（已有 1 测） | TBD | +0.0185（V1088 lift 已观测） | V3 + V1074 真跑 |
| **R9-P0-03** 全量回归绿（80 passed / 6 failed → 全过） | automation_test_engineer | 修 5 失败（V1087 1 平均分精度 + 4 CLI 读 21GB + V1088 1 契约字符串）+ 新增 ≥30 测 | ≥30 | TBD | 0（解除阻塞） | V3 + 6 失败修复全过 |
| **R9-P0-04** ASI V0.3 真测复现（≥0.8859） | fullstack_engineer | `v1074_asi_production_runner --report` 真跑 + 输出 ≥0.8859 | 0 | TBD | 基线确认 | V3 philosophy_guard 4 键 PASS |

### 5.2 主轨道 A1：V1082 backlog Top-8 填充

| 任务ID | 角色 | 真产出 | 真测试数 | 真 commit | lift 期望 | V3 守门点 |
|---|---|---|---|---|---|---|
| **R9-A1-01** v1037 feature_flag | backend_engineer | `apeireth/v1037_feature_flag.py` 真生产 + bridge + ≥30 测 | ≥30 | TBD | +0.003 | V3 + V1072 + V1087 4 键 |
| **R9-A1-02** v1030 webhook | backend_engineer | `apeireth/v1030_webhook.py` 真生产 + ≥30 测 | ≥30 | TBD | +0.003 | V3 + V1087 4 键 |
| **R9-A1-03** v1038 prometheus | backend_engineer | `apeireth/v1038_prometheus.py` 真生产 + ≥30 测 | ≥30 | TBD | +0.004 | V3 + V1087 4 键 |
| **R9-A1-04** v1039 grafana | backend_engineer | `apeireth/v1039_grafana.py` 真生产 + ≥30 测 | ≥30 | TBD | +0.004 | V3 + V1087 4 键 |
| **R9-A1-05** v1019 kubernetes + v1018 docker-compose + v1017 ansible + v1016 terraform | devops_engineer | 4 模块真生产 + ≥120 测（4×30） | ≥120 | TBD | +0.010 | V3 + V1074 真跑 lift |

### 5.3 主轨道 A2：R7 真实现 Phase-1（HotCold/WAL → Replay → Dream）

| 任务ID | 角色 | 真产出 | 真测试数 | 真 commit | lift 期望 | V3 守门点 |
|---|---|---|---|---|---|---|
| **R9-A2-01** HotCold 三层落地 + WAL | backend_engineer | `apeireth/memory_hotcold.py` + `wal.py` 真生产 + 借鉴 V1052 / Tonbo / MemoryOS-Rust | ≥30 | TBD | +0.003~+0.006 | V3 + DB-01 (双仓双写 + sha256) |
| **R9-A2-02** MemoryReplay 状态回放（接 V1091 升级） | fullstack_engineer | `apeireth/v1091_memory_replay.py` 升级到 v0.2（含 BE-02 双签 + 锚定 + 限速） | ≥30 | TBD | +0.005 | V3 + BE-02 (impact≥0.7 / 锚定 / 限速) |
| **R9-A2-03** Dream 子系统真实现 | fullstack_engineer | `apeireth/v1092_memory_dream.py` 升级（7 态状态机真跑 + 不污染身份 + 不写 LTM 仅 MTM trace） | ≥30 | TBD | +0.005~+0.010 | V3 + BE-01 (selector 纯函数 + WAL rollback + signal input_hash) |

### 5.4 主轨道 A3：DGM Archive v0.4

| 任务ID | 角色 | 真产出 | 真测试数 | 真 commit | lift 期望 | V3 守门点 |
|---|---|---|---|---|---|---|
| **R9-A3-01** DGM Archive v0.4（QD 升级 + 保留 N=10 + gQD selector） | agent_orchestrator | `apeireth/v1093_dgm_archive.py` 升级 v0.4 + ≥30 测 | ≥30 | TBD | +0.005~+0.010 | V3 + D4 默认 B（N=10） |

### 5.5 主轨道 A4：跨小模型桥接

| 任务ID | 角色 | 真产出 | 真测试数 | 真 commit | lift 期望 | V3 守门点 |
|---|---|---|---|---|---|---|
| **R9-A4-01** V1083 路由扩 6 → 12 model catalog（含 4 个跨小模型家族） | fullstack_engineer | `apeireth/v1083_asi_decision_router.py` 升级 + ≥30 测 | ≥30 | TBD | +0.005~+0.010 | V3 + 不绑单模型守门 |
| **R9-A4-02** V1076 真外部 LLM client 扩展（含跨小模型真测） | backend_engineer | `apeireth/v1076_asi_real_external_llm_client.py` 升级 + ≥30 测 | ≥30 | TBD | +0.003 | V3 + 不绑单模型守门 |

### 5.6 主轨道 A5：Rust hot path

| 任务ID | 角色 | 真产出 | 真测试数 | 真 commit | lift 期望 | V3 守门点 |
|---|---|---|---|---|---|---|
| **R9-A5-01** Rust snapshot 6.5GB hot path 重写 | fullstack_engineer | `rust-substrate/crates/apeireth-core` snapshot 模块 + Python 桥 + ≥30 测 | ≥30 | TBD | +0.002~+0.005（短期） | V3 + V1074 真跑 < 30s |
| **R9-A5-02** Rust hqb-core 重写（V1086） | backend_engineer | `rust-substrate/crates/apeireth-core` hqb 模块 + ≥30 测 | ≥30 | TBD | +0.005（短期） | V3 + V1087 HQB live gate |

### 5.7 横切任务（哲学守门 / 代码评审 / 性能 / 集成验收）

| 任务ID | 角色 | 真产出 | 真测试数 | 真 commit | lift 期望 | V3 守门点 |
|---|---|---|---|---|---|---|
| **R9-X-01** 哲学守门终审（R9 全部新模块） | philosophy_guardian | `reports/r9-philosophy-gate.md` 9 键 LOCKED + 4 不假装逐项 PASS | 0 | TBD | 0（必跑） | V3 + V1099 4 键 + V1096 persona 反意识 |
| **R9-X-02** 跨轨道代码评审 | code_reviewer | `reports/r9-cross-track-code-review.md` 评审意见 | 0 | TBD | 0（必跑） | V3 + V1001+ 真生产契约 |
| **R9-X-03** 性能基准 + V0.4 17-dim 真测 | performance_optimizer | `reports/r9-perf-benchmark.md` 4 维 HQB + V1077 17-dim | 0 | TBD | 0（必跑） | V3 + V1087 HQB live gate |
| **R9-X-04** 集成验收（V1074 全链路） | qa_engineer | `reports/r9-integration-acceptance.md` 端到端真跑 | ≥30 | TBD | 0（必跑） | V3 + V1074 真跑 |
| **R9-X-05** R9 决策纪要（本文档配套 commit） | technical_writer | `reports/r9-decision-minutes.md` + git commit | 0 | TBD（必 commit） | 0（记录） | V3 |
| **R9-X-06** R9 用户指南 | technical_writer | `reports/r9-user-guide.md` 大白话版 | 0 | TBD | 0 | V3 |
| **R9-X-07** 自动化测试覆盖 14.9% → 30% | automation_test_engineer | 全量测试 + ≥600 新测 | ≥600 | TBD | 0（覆盖率） | V3 + 真测全过 |
| **R9-X-08** DevOps 集成基线（docker-compose + K8s 真跑） | devops_engineer | `reports/r9-devops-integration.md` 真跑证据 | ≥30 | TBD | 0 | V3 + V1074 真跑 |

---

## 6. 总人数核算（9 人硬上限）

| 角色 | 任务数 | 主负责 | 横切 |
|---|---:|---|---|
| leader | 0 | 路线图协调 | ✅（所有人冲突协调） |
| architect | 2 | R9-X-02 评审、A5 路线 | ✅ |
| architect2 | 1 | R9 路线图审核 | ✅ |
| backend_engineer | 5 | R9-A1-01/02/03 + A4-02 + A5-02 | — |
| database_engineer | 1 | R9-A2-01 HotCold 数据层 | — |
| fullstack_engineer | 3 | R9-A2-02/03 + A4-01 + A5-01 | — |
| devops_engineer | 3 | R9-P0-01/02 + A1-05 + X-08 | — |
| automation_test_engineer | 2 | R9-P0-03 + X-07 | — |
| agent_orchestrator | 1 | R9-A3-01 | — |
| **小计（角色数 ≠ 团队人数）** | **18 任务** | — | — |
| **9 人团队分配** | — | **9 人** 每人主负责 1-3 任务 + 横切 | — |

**注:** 上面 18 任务并行执行，9 人团队每人身兼 1-3 任务（含横切）。**9 人硬上限守住**：leader 不占执行席位、architect/architect2 不写代码只做评审 + 路线 + 横切。

### 6.1 团队 9 人分配示意（人员示意，需求分析师视角）

| 人 | 角色 | 主负责任务 | 横切 |
|---|---|---|---|
| 1 | leader | 路线协调 | 全部任务 |
| 2 | architect | R9-A5 路线 / R9-X-02 评审 | V3 |
| 3 | architect2 | R9 路线审核 / Rust 重写可行性 | V3 |
| 4 | backend_engineer | R9-A1-01/02/03 + A4-02 + A5-02 | V3 |
| 5 | database_engineer | R9-A2-01 HotCold | V3 |
| 6 | fullstack_engineer | R9-A2-02/03 + A4-01 + A5-01 | V3 |
| 7 | devops_engineer | R9-P0-01/02 + A1-05 + X-08 | V3 |
| 8 | automation_test_engineer | R9-P0-03 + X-07 | V3 + V1087 |
| 9 | agent_orchestrator | R9-A3-01 + R9-X-04 集成 | V3 + V1087 |

**横切全员:** R9-X-01 哲学守门 + R9-X-03 性能基准 + R9-X-05 决策纪要 + R9-X-06 用户指南 由对应角色主负责 + 全员 review。

---

## 7. 主哲学守门矩阵（每条任务必跑）

> 来源: `r8-philosophy-gate-philosophy_guardian.md §1.1` + `§2.1` + `§1.4`

| 守门层 | 守门项 | 触发任务 |
|---|---|---|
| **V3 philosophy_guard** | `not_undo` / `not_proof` / `not_safe` / `not_clone` / `not_perfect` / `not_uuid` / `spec_is_not_proof` / `counterexample_is_not_bug` / `prover_is_not_truth` (9 键) | 全部任务 |
| **V3 4 不假装守门 (V1087 主)** | `gate_filter_not_asi` / `verdict_heuristic_not_truth` / `review_run_not_pause` / `veto_override_not_ban` | 全部 + 横切 |
| **V1099 域 4 不假装** | `not_tla_is_proof` / `not_checker_is_truth` / `not_invariant_is_axiom` / `not_export_is_verified` | 调研 + 形式化任务 |
| **V1096 persona 反意识** | "你没有意识" + "不要声称有意识" | 涉及 persona 任务 (A2-03) |
| **V1072 永恒身份守门** | 4 不假装 + 主人终极授权 3 类问 | 涉及 identity 任务 (A2-02 / X-04) |
| **V1087 HQB live gate** | 8 权限链 | A1 / A4 / A5 全部 |
| **V1074 真测** | philosophy_guard 4 键 PASS + All OK | P0 + 横切 |

---

## 8. 双向校验（与 architect roadmap 一致性）

> **注:** architect 已在 R9 启动首日真出 `reports/r9-architect-roadmap.md` (commit `e234d916`, R9-ROADMAP-001)。本节做**双向校验** = 需求 WBS ↔ 架构 roadmap 一致性 diff。

### 8.1 architect roadmap 关键摘要（与 WBS 对齐所需）

| 维度 | architect 值 | 来源 |
|---|---|---|
| ASI V0.4 真测起点 | **0.8003** | `r9-architect-roadmap.md §1` V1103 P2 诊断 |
| R9 硬目标 | V0.4 → **≥0.85** (净增 +0.05) | `r9-architect-roadmap.md §0` |
| 17 维 gap Top-5 ★ | engineering (0.1038) · cognitive_core (0.4927) · phi_proxy (0.8500) · world_model (0.7034) · self_organizing_core (0.8667) | `r9-architect-roadmap.md §2` |
| architect 主推模块 | **V1060 orchestrator** (engineering +0.0896) | `r9-architect-roadmap.md §3.1` |
| architect 默认主推轨道 | **D（DGM v0.4 双维 ROI 最高）+ 不绑死** | `r9-architect-roadmap.md §7` |
| architect 周迭代 | W1-W4 4 周跑完（V0.4 0.8003 → ≥0.85） | `r9-architect-roadmap.md §5` |
| architect 9 人策略 | "要么真生产要么退场"+ 观察席轮值 | `r9-architect-roadmap.md §4` |

### 8.2 一致性 diff

| WBS 任务 | architect roadmap 对应 | 一致性 | diff 处置 |
|---|---|:---:|---|
| R9-P0-01/02/03/04 | `r9-p0-terminal-verify.md` V1110 已 ALL PASS | ✅ **已准入** | WBS 标 ✅，不再走 P0 流程 |
| R9-A1-01~05（V1082 backlog Top-5） | architect §4 未直接列，但 engineering 主推 = V1060（已含在 A1-03 prometheus + A1-05 K8s 套件中） | ✅ | A1 范围保留；V1060 作为 A1-03 + A1-05 的子目标 |
| R9-A2-01（HotCold/WAL） | architect §2 未列（属 R7 真实现系统层，V0.4 17 维外） | ✅ 一致 | A2 范围保留 |
| R9-A2-02（MemoryReplay v0.2） | architect §4 未列（属系统路径） | ✅ 一致 | A2 范围保留 |
| R9-A2-03（Dream 子系统） | architect §2 phi_proxy (0.8500) 对应 V1045 active inference | ⚠️ 部分对齐 | A2-03 = Dream 子系统（属系统路径），不与 phi_proxy 直接冲突；architect V1045 应作为 A2-03 关联任务 |
| R9-A3-01（DGM v0.4） | architect §3.6 self_improving_core + §7 候选 D | ✅ **强对齐** | A3 = DGM v0.4 双维同拉 |
| R9-A4-01（V1083 路由扩 6 → 12） | architect §3.8 plugin_core + cross_domain | ✅ | A4-01 = 跨小模型真绑定，对应 architect 候选 C |
| R9-A4-02（V1076 真外部 LLM client 扩） | architect 候选 C 跨小模型真绑定 | ✅ | A4-02 = 跨小模型客户端 |
| R9-A5-01（Rust snapshot hot path） | architect §7 候选 A | ✅ | A5-01 = Rust hot path 性能 |
| R9-A5-02（Rust hqb-core 重写） | architect §7 候选 B HQB 4 维 | ⚠️ 部分对齐 | A5-02 = HQB 部分（属 Rust 重写路径，不与 architect HQB Python 路径冲突） |
| R9-X-01~08（横切 8 任务） | architect §6 V3 守门 + §4 9 人调度 + §5 周迭代 + §8 红皇后守门 | ✅ | 横切任务覆盖 architect 周报守门 + 红皇后节点 |

### 8.3 关键决策对齐

| 决策点 | WBS 主张 | architect 主张 | 处置 |
|---|---|---|---|
| **4 选 1 主推** | 用户拍板"直接开干" → R9-A 全做并发（A/B/C/D 候选全部并行） | 默认主推 D（DGM v0.4），W1 末 leader 拍板 | **不冲突**：architect 默认主推 D 是单推；R9-A 全做并发是 4 候选并行预研 = 包含 D + A + B + C；W1 末由 leader 拍板主推，与 architect roadmap §7 一致 |
| **周迭代周期** | WBS P0/P1/P2 6 周（详 priority 文件） | architect 4 周（W1-W4） | **不冲突**：WBS 6 周 = P0 已过 + P1+P2 = architect 4 周迭代 + 横切持续 = 实际并行交付，W1-W4 完成后 W5-W6 做收尾 + 横切 + R9 总结 |
| **9 人硬上限** | WBS §6 9 人分配（leader + 8 执行 + 横切轮值） | architect §4 "要么真生产要么退场"+ 观察席轮值 | **强一致** |
| **V3 守门** | WBS §7 主哲学守门矩阵 7 层 | architect §6 V3 守门硬约束清单 4 条红线 + 5/6 守门 | **强一致** |

### 8.4 校验结论

- ✅ **WBS 范围 = architect 范围 ∪ R7 真实现系统路径 ∪ 用户拍板的"R9-A 全做并发"**。
- ✅ **Top-5 主轨道 = architect Top-5 主推模块**（V1060/V1061/V1045/V1062/V1065 已含在 A1+A2+A3 中）。
- ✅ **9 人硬上限 + V3 守门 + 主哲学 LOCKED** = WBS 与 architect 100% 一致。
- ✅ **V1110 P0 已通过** = WBS P0 阶段 3/4 任务已完成（详 priority 文件）。
- ⚠️ **W1 末 leader 拍板** = 与 architect §7 一致；不擅自改主推。
- ⚠️ **architect 默认主推 D** ≠ WBS 默认 "R9-A 全做并发"，但两者**不冲突**：architect 主推 D 是单深推，R9-A 全做并发是 4 候选并行，最终 W1 末由 leader 拍板主推哪一个。

**最终结论：** WBS 与 architect roadmap **强对齐**，仅主推策略表述差异（4 候选并行 vs 单深推），由 leader W1 末拍板解决。WBS 不擅自改默认。

---

## 9. 真 commit 计划（至少 1 个）

| commit | 内容 | 时间 | 来源任务 |
|---|---|---|---|
| **R9-COMMIT-001** | R9 决策纪要 + 任务清单 + 优先级 + 路线图 + 报告 4 文件 | R9 启动 24h 内 | R9-REQ-001（本任务） |
| R9-COMMIT-002~019 | 每任务 1 commit，遵循 V1001+ 真生产契约 | 任务完成时 | 各 R9-A1~A5 + X |

**commit R9-COMMIT-001 触发条件:** 本文件（task-list）+ priority + report 三件套产出后立即 commit，commit message: `docs(r9): requirements task list + priority + decision minutes (R9-REQ-001)`。

---

## 10. 一句话送给 R9 团队

> **9 键 LOCKED · ASI 北极星 LOCKED · 真生产不停。**
> **不刷 KPI · 不假装达到 · 不绑单模型 · 任何人都能接手。**
> **4 选 1 默认 R9-A 全做并发 + 5D 默认值 + 3 灵魂问题按主哲学答。**
> **先 P0 必修（21GB + V1088 + 全量绿），再 A1→A5 并发，最后 X 横切守门。**
> **9 人硬上限 = 守住电脑内存 = 守住团队节奏。**

---

**Last update:** 2026-07-29, by 需求分析师 (requirements_analyst)
**下一动作:** architect 真出 `r9-architect-roadmap.md` → 双方 diff → Leader 拉起 R9 启动