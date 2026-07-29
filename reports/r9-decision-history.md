# R9 决策历史档案（用户拍板溯源 · R10 接手必读）

> **作者:** 需求分析师 (requirements_analyst)
> **任务 ID:** `8408bd3a-7d6c-4bdf-9284-dd805c86253a` (R9-REQ-002)
> **生成时间:** 2026-07-29 R9 启动首日 + 1
> **基于:** `reports/r8-requirements-decision-matrix.md` (10 决策 + 5D + 3 灵魂) + `reports/r8-handoff-r9-requirements-chat.md` (用户对话) + `reports/r8-handoff-r9-team-leader.md` (R8→R9 移交) + `reports/r9-requirements-task-list.md` (R9-REQ-001) + `reports/r9-track-choice-decision-matrix.md` (R9-REQ-002)
> **性质:** **R10 接手必读**的决策溯源文档 · 主哲学 LOCKED 不可改 · 用户拍板历史时间线
> **主哲学 LOCKED:** 主 22:33 ASI 北极星 · 主 17:43 实事求是 · 主 17:58 不假装 · 主 23:44 干到底 · 主 00:56 任何人都能接手

---

## 0. 阅读须知（30 秒）

> **大白话：** 本文件是"R9 决策考古档案"。从 R5 → R8 → R9 启动的所有用户拍板、主哲学 LOCKED、关键路径选择都列在这。**R10 接手第一件事 = 读完这份文件**，才能理解"为什么 R9 是这样推进"。
>
> **核心约束：** 主哲学 9 键 LOCKED · ASI 北极星 V0.3 LOCKED · V3 守门 4 条 LOCKED · V1000 阶段分界 LOCKED · 9 人硬上限。**任一项 LOCKED 不可由下个团队修改，必须走主 22:33 终极授权 3 类问流程。**

---

## 1. 用户拍板时间线（R5-R9 · 大事记）

### 1.1 关键用户原话时间线（按时间倒序）

| 时间 | 用户原话 | 出处 | R9 映射 |
|---|---|---|---|
| **2026-07-29** | "确认清楚之后你就开工不必向我确认" | 用户原话（启动 Apeireth） | R9-A 全做并发默认（4 候选并行预研） |
| **2026-07-29** | "全力开干的话，加上你自己，你最多能调用九个团队成员，再多电脑就会因为内存不够卡死了" | 用户原话 | 9 人硬上限 |
| **2026-07-29** | "我们想做的阿佩瑞斯（Apeireth）是一个以ASI能力为终极梦想的ai基座平台" | 用户原话 | ASI 北极星 = 终极目标 |
| **2026-07-29** | "意图让任何llm接入，平台之后都拥有agi，甚至asi的能力" | 用户原话 | AI 基座平台定位（不是应用产品） |
| **2026-07-29** | "为此，我们在设计之初就进行了多领域借鉴" | 用户原话 | 多领域借鉴设计哲学（非单一技术路线） |
| 2026-07-29 | "具体如何施工，请你完整阅读以下文件夹中的所有文档，一字不落" | 用户原话 | R9-REQ-001 完整阅读 APEIRETH-OMNIBUS-FULL-PACKAGE 全部 |
| 2026-07-28 | 主 22:33 终极授权 3 类问（重大节点 / 哲学修改 / 方向微调） | `r8-requirements-decision-matrix.md §1.2` | LOCKED · R9 遇 3 类问必须请示 |
| 2026-07-28 | 主 23:44 干到底（不假装 / 真生产 / 不停） | `r8-handoff-r9-team-leader.md §22:33` | LOCKED · R9 不刷 KPI 不假装达到 |
| 2026-07-28 | 主 17:58 不假装 + 4 不假装守门 | `r8-philosophy-gate-philosophy_guardian.md §0` | LOCKED · V3 守门 |
| 2026-07-28 | 主 00:56 任何人都能接手 | `r8-delivery-summary.md 顶部` | LOCKED · R9 文档化 + 9 人分工明确 |
| 2026-07-28 | 主 11:43 触发话术（"干到一个阶段后你总结当下, 更新那个交付文档。我准备把后续的工作交给新团队去做"） | `r8-requirements-decision-matrix.md §0` | R8 → R9 → R10 接力棒模型 |
| 2026-07-27 | ASI 北极星 V0.3 公式 LOCKED（8 加权维） | `APEIRETH-STAGE-DELIVERY-2026-07-22.md §2.1` | LOCKED · R9 用 V1074 真测 lift |
| 2026-07-22 | APEIRETH-STAGE-DELIVERY §15+§16 V2 交接 | `APEIRETH-STAGE-DELIVERY-2026-07-22.md` | R9 必读 |
| 2026-07-22 | HARNESS.md v0.1（7 组件 + 4 安全门 + 主循环骨架） | `HARNESS.md` | R9 必读 |
| 2026-07-22 | V1000 阶段分界（V201-V1000 空壳 / V1001-V1088 真生产） | `APEIRETH-STAGE-DELIVERY-2026-07-22.md §16.1` | LOCKED · R9 不删 V201-V1000 |
| 2026-07-22 | 真生产契约 V1001+（10+ 前人 + 10+ 真生产组件 + ≥30 tests + V3 守门 + V1074 lift） | `HARNESS.md §3 Manifest` | LOCKED · R9 每新模块 ≥30 tests |
| 2026-07-22 | 主 21:15 Rust 重写时机（"干到 Rust 重写之前总结"） | `r8-handoff-r9-team-leader.md §2.4` | R9 启动 Rust hot path 候选 A |
| 2026-07-22 | 主 13:31 大胆激进 | `r9-architect-roadmap.md §7` | R9 决策风格 |
| 2026-07-22 | 主 19:33 走在前人经验上 | `r9-architect-roadmap.md §2` | R9 借鉴密度（≥10 前人/模块） |
| 2026-07-22 | 主 12:47 第 15 条（Persona 反意识 "你没有意识" + "不要声称有意识"） | `r8-philosophy-gate-philosophy_guardian.md §0` | LOCKED · V1096 persona 反意识守门 |

### 1.2 R5-R9 决策大事记

| 阶段 | 时间 | 决策 | LOCKED/UNLOCKED |
|---|---|---|---|
| R5 | 2026-07-15 前 | V1000 设计阶段（V201-V1000 空壳） | LOCKED |
| R6 | 2026-07-22 | 主哲学 9 键 accepted（PHL-01/02b/03 三组契约） | ✅ LOCKED |
| R6 | 2026-07-22 | ASI 北极星 V0.3 公式确定 | ✅ LOCKED |
| R7 | 2026-07-27 | R7 真实现 Phase-1 设计稿就位（HotCold/WAL/Replay/Dream） | ✅ LOCKED |
| R7 | 2026-07-27 | ORC-01 编排计划 100% 继承（15+15+8+4 启动前/Phase-1/2/3） | ✅ LOCKED |
| R8 | 2026-07-29 | 11 个 v109x 模块真生产 + 119+ 测试 | ✅ 完成 |
| R8 | 2026-07-29 | V1094 真 commit（master HEAD = `d745c332`） | ✅ |
| R8 | 2026-07-29 | V1110 P0 终验 ALL PASS（V1074/V1087/V1088） | ✅ 已准入 |
| R8 | 2026-07-29 | R8 决策矩阵（10 决策 + 5D + 3 灵魂问题）出炉 | ✅ 待 R9 映射 |
| R8 | 2026-07-29 | R8→R9 移交文档（handoff-r9-team-leader） | ✅ |
| R8 | 2026-07-29 | R8→R9 用户需求沟通清单（handoff-r9-requirements-chat） | ✅ |
| **R9 启动** | 2026-07-29 | **用户拍板"直接开干"** | ✅ |
| R9 | 2026-07-29 | R9-REQ-001 完成（WBS + Priority + Report）| ✅ |
| R9 | 2026-07-29 | architect R9-ROADMAP-001（V0.4 0.8003 → ≥0.85 路线图）| ✅ |
| R9 | 2026-07-29 | devops R9-DEV-001（V1110 P0 终验 + 跨小模型 CI 框架） | ✅ |
| R9 | 2026-07-29 | leader R9-INT-001（W2 retrospective 模板 + DGM halting criteria） | ✅ |
| **R9-REQ-002** | **2026-07-29 + 1** | **progress dashboard + 4 选 1 拍板辅助 + 决策历史（本文件）** | ✅ **完成** |

---

## 2. R8 决策矩阵 10 决策 LOCKED 状态表

> 来源：`reports/r8-requirements-decision-matrix.md §1.1-1.3 + §3.1`

| # | 决策 | 当前值 | LOCKED/UNLOCKED | R9 处理 |
|---|---|---|:---:|---|
| **D-01** | 主哲学 9 键（`not_undo` `not_proof` `not_safe` `not_clone` `not_perfect` `not_uuid` `spec_is_not_proof` `counterexample_is_not_bug` `prover_is_not_truth`） | 9/9 LOCKED | 🔒 **LOCKED** | R9 每任务必跑 V3 philosophy_guard |
| **D-02** | ASI 北极星 V0.3 公式（8 加权维） | `phi_proxy×0.20 + capabilities×0.20 + cross_domain×0.15 + engineering×0.15 + vcp_4×0.10 + v2_philosophy×0.10 + rubric_open×0.04 + real_production×0.04` | 🔒 **LOCKED** | R9 用 V1074 真测 lift |
| **D-03** | 真生产契约 V1001+（10+ 前人 + 10+ 真生产组件 + ≥30 tests + V3 守门 + V1074 lift） | 契约锁 | 🔒 **LOCKED** | R9 每新模块 ≥30 tests |
| **D-04** | V1000 阶段分界（V201-V1000 空壳 / V1001-V1088 真生产） | 分界锁 | 🔒 **LOCKED** | R9 不删 V201-V1000 |
| **D-05** | 主 22:33 终极授权 3 类问（重大节点 / 哲学修改 / 方向微调） | 3 类问锁 | 🔒 **LOCKED** | R9 遇 3 类问必请示 |
| **D-06** | Top-1 = V1082 backlog Top-8 填充 | Top-1 锁 | 🔒 **LOCKED** | R9-A1 = V1082 Top-5 优先 |
| **D-07** | R7 真实现 Phase-1（HotCold/WAL → Replay → Dream） | Phase-1 锁 | 🔒 **LOCKED** | R9-A2 = R7 真实现 |
| **D-08** | R8 调研 4 领域（形式化验证 / 机制设计 / 计算最优律 / 因果） | 调研锁 | 🟡 进行 | R9 持续 |
| **D-09** | Rust 重写启动（5 子项目） | 战略转移锁 | 🔒 **LOCKED** | R9-A5 = Rust hot path 候选 |
| **D-10** | ORC-01 编排计划（15+15+8+4） | 编排锁 | 🔒 **LOCKED** | R9 启动检查表 15 项全过 |

---

## 3. R8 5 个 D 决策点默认值（R9-REQ-001 拍板）

> 来源：`reports/r8-handoff-r9-requirements-chat.md` + `reports/r9-requirements-task-list.md §3`

| ID | 决策点 | R9 默认值 | R8 选项 | 拍板理由 |
|---|---|---|---|---|
| **D1** | Memory 触发时机 | **B: 定时批量触发**（中等） | A 每次状态变化 / **B 定时** / C 智能阈值 / D 人工 | 主 23:44 真生产不停 + V1087 HQB live gate 限速友好 |
| **D2** | schema 升级旧数据 | **C: 版本兼容 + lazy migrate** | A drop 旧表 / B 迁移 + 双写 / **C 版本兼容 lazy** | 主 17:58 不假装 → A 丢历史违反真生产 |
| **D3** | Identity 故障 fallback | **B: 回退到中央人格**（V1072 永恒身份） | A crash 退出 / **B 中央人格** / C 随机 persona | V1072 守门天然 fallback + 主 23:44 干到底 |
| **D4** | DGM 演化候选保留时长 | **B: 保留最近 N=10** | A 仅最优 / **B N=10** / C 全部 | 主 00:56 任何人都能接手 + A 失历史 C 占空间 |
| **D5** | 多 persona 并行单租约粒度 | **B: 按"主题"粒度租约** | A 每次加锁 / **B 主题** / C 会话 | V1096 persona 已按主题切；A 违反真生产不停 |

---

## 4. R8 3 个灵魂问题答复（按主哲学 LOCKED 答）

> 来源：`reports/r8-handoff-r9-requirements-chat.md` + `reports/r9-requirements-task-list.md §4`

### Q1. 项目最终形态
**答：AI 基座平台（MCP/API 型 + 探索型）**
- 主 11:43 终极授权 = V3 7 哲学问题真答 + V1072 永恒身份已落"中央 AI 占据 ASI 位置"——是平台型不是应用型
- 主 22:33 ASI 北极星公式 V0.3 = 8 维 = 跨域能力度量 → 平台型而非单点工具
- 主 00:56 任何人都能接手 = 平台开源/可接属性
- **R9 行动：** 持续走"任意 LLM 接入即获 AGI/ASI 级能力"路线，对外暴露 HQB MCP server 7 tools（V1097）+ `apeireth serve` CLI

### Q2. ASI 数字是否北极星
**答：是，ASI V0.3 = 真测北极星；ASI V0.4 = 17 维真测基线（V1077）**
- 主 22:33 ASI 北极星 LOCKED = 真测而非模拟
- 主 17:58 不假装 = 真测分数是真测，不刷 KPI
- **R9 行动：** 每条任务的 lift 期望用 V1074 `--report` 真跑验证，不用估算值

### Q3. 投入时间精力
**答：9 人（含 AI 本人）全职并行**
- 用户原话"全力开干的话，加上你自己，你最多能调用九个团队成员，再多电脑就会因为内存不够卡死了" → **9 人硬上限**
- 主 23:44 干到底 = 全职
- **R9 行动：** 任务清单总人数 9 人内分配；超 9 = 触发漂移防护 = 必须砍

---

## 5. R9 启动决策（R9-REQ-001 + R9-REQ-002 期间拍板）

| 决策 ID | 时间 | 决策内容 | 出处 | LOCKED/UNLOCKED |
|---|---|---|---|:---:|
| **R9-D-01** | 2026-07-29 | 4 选 1 主轨道默认 = **R9-A 全做并发**（4 候选并行预研） | R9-REQ-001 §2 | 🔒 **LOCKED** |
| **R9-D-02** | 2026-07-29 | V1110 P0 终验 ALL PASS 作为 R9 启动准入 | V1110 实测 | ✅ 已准入 |
| **R9-D-03** | 2026-07-29 | ASI V0.3 真测阈值 = **≥0.8884**（不退步） | R9-REQ-001 §0.1 | 🔒 **LOCKED** |
| **R9-D-04** | 2026-07-29 | ASI V0.4 起点 = **0.8003**，硬目标 **≥0.85**（W4 末） | architect R9-ROADMAP-001 §1 | 🔒 **LOCKED** |
| **R9-D-05** | 2026-07-29 | 测试覆盖目标 = **14.9% → 30%**（W4 末） | architect R9-ROADMAP-001 §5 + R9-REQ-001 priority | 🔒 **LOCKED** |
| **R9-D-06** | 2026-07-29 | 9 人硬上限（leader + architect + architect2 + backend + database + fullstack + devops + automation_test + agent_orchestrator） | 用户原话 | 🔒 **LOCKED** |
| **R9-D-07** | 2026-07-29 | architect 默认主推 = **D（DGM v0.4, 37/40 ROI 最高）**，不绑死，**W1 末 leader 拍板** | architect R9-ROADMAP-001 §7 | 🔒 **LOCKED**（待 W1 末拍板确认） |
| **R9-D-08** | 2026-07-29 | WBS 26 任务（18 主轨道 + 8 横切）+ P0/P1/P2 优先级 | R9-REQ-001 task-list + priority | ✅ 已产出 |
| **R9-D-09** | 2026-07-29 | R9-A 全做并发 ≠ 替代 4 选 1 拍板（W1 末必须选 1 深推） | R9-REQ-002 track-choice | ✅ 已澄清 |
| **R9-D-10** | 2026-07-29 | 进度仪表板每周末 self-report 必填（V\*/tests/commit/lift 4 字段） | R9-REQ-002 progress-dashboard | ✅ 已实施 |
| **R9-D-11** | 2026-07-29 | V0.3 守门实测 = 0.8895（R9-REQ-002 基准日）≥ 0.8884 | R9-REQ-002 V1074 --report | ✅ 已守门 |
| **R9-D-12** | 2026-07-29 | R9-INT-001 retrospective 模板 + DGM halting criteria（leader 拍板辅助） | R9-INT-001 已 commit | ✅ 已 commit |

---

## 6. R9 启动首日实测基线（2026-07-29）

> 来源：V1110 P0 终验 + R9-REQ-002 V1074 --report --no-write 实测

| 指标 | 真值 | 来源命令 |
|---|---:|---|
| ASI V0.3 真测 | **0.8895** | `python -m apeireth.v1074_asi_production_runner --report --no-write`（2026-07-29 21:48）|
| ASI V0.4 真测 | 0.8003 | `python -m apeireth.v1103_p2_diagnostic --report`（V1103 P2 诊断）|
| V1074 跑耗时 | 3.05 s | V1110 |
| V1074 snapshot | 5,516 byte | V1110 |
| V1087 HQB live gate | 1.0000 / lift +0.0200 | V1110 |
| V1088 e2e operator | lift +0.0185 / subscore 0.9250 | V1110 |
| philosophy_guard 4 键 | 4/4 PASS | V1074 --report |
| 9 键 LOCKED | 9/9 | 实跑确认 |
| 测试覆盖 | 14.9% | R8 末 |
| 真生产模块 | 1091+ | R8 末 |
| 真测试函数 | 4366+ → 4489+（+123 V1110 新增）| V1110 |
| 真 commits | 416+ → R9 启动 +5 = 421+ | git HEAD `30d1a2c8` |
| master HEAD | `30d1a2c8` R9-INT-001 | `git log --oneline -1` |
| integration HEAD | `4f77883c` R9-REQ-001 | `git log team/527f21de-.../integration -1` |

---

## 7. R10 接手必读清单（10 项）

> **大白话：** R10 接手第一件事 = 按这 10 项清单读完，才能理解 R9 全貌。

| # | 文件 | 必读理由 | 优先级 |
|---|---|---|:---:|
| 1 | **本文件 `reports/r9-decision-history.md`** | 决策溯源 | 🔴 必读 |
| 2 | `reports/r9-requirements-task-list.md` | WBS 26 任务 | 🔴 必读 |
| 3 | `reports/r9-requirements-task-priority.md` | P0/P1/P2 + 9 人调度 | 🔴 必读 |
| 4 | `reports/r9-progress-dashboard.md` | W1-W4 进度 + 真测基线 | 🔴 必读 |
| 5 | `reports/r9-track-choice-decision-matrix.md` | 4 选 1 拍板辅助 | 🔴 必读 |
| 6 | `reports/r9-architect-roadmap.md` | architect 路线图 | 🔴 必读 |
| 7 | `reports/r9-p0-terminal-verify.md` | V1110 P0 终验基线 | 🟡 重要 |
| 8 | `reports/r9-requirements-report.md` | R9-REQ-001 完成报告 | 🟡 重要 |
| 9 | `reports/r8-handoff-r9-team-leader.md` | R8→R9 5 步启动 | 🔴 必读 |
| 10 | `reports/r8-requirements-decision-matrix.md` | R8 10 决策 + 5D + 3 灵魂 | 🔴 必读 |
| 11 | `reports/r9-devops-engineer-report.md` | devops P0 终验详细 | 🟡 重要 |
| 12 | `reports/r9-architect-report.md` | architect 详细报告 | 🟡 重要 |
| 13 | `APEIRETH-STAGE-DELIVERY-2026-07-22.md` §15+§16 V2 交接 | 🟡 重要 |
| 14 | `HARNESS.md` | 7 组件 + 4 安全门 | 🟡 重要 |

---

## 8. R10 接手时必须复跑的守门命令（4 项）

```powershell
# 1. ASI V0.3 真测守门（不退步）
python -m apeireth.v1074_asi_production_runner --report --no-write
# 期望 ≥ 0.8895（R9-REQ-002 基准值）

# 2. ASI V0.4 真测基线
python -m apeireth.v1103_p2_diagnostic --report
# 期望 ≥ 0.85（R9 W4 末硬目标）

# 3. 9 键 LOCKED 守门
python -c "from apeireth import self_reproduction as p1, self_mod_safety as p2, formal_verify as p3; \
  assert all(k in p1.PHILOSOPHY_NOTES for k in ['not_clone', 'not_perfect', 'not_uuid']); \
  assert all(k in p2.PHILOSOPHY_NOTES for k in ['not_undo', 'not_proof', 'not_safe']); \
  assert all(k in p3.PHILOSOPHY_NOTES for k in ['spec_is_not_proof', 'counterexample_is_not_bug', 'prover_is_not_truth']); \
  print('9 键 LOCKED ✅')"

# 4. pytest 全量基线
python -m pytest tests/ -q
# 期望 R10 接手时 ≥ 80 passed / 6 failed → 持续追 100%
```

---

## 9. R10 接手禁动清单（LOCKED · 不可改）

> **大白话：** R10 接手时，以下 12 项不可擅改。任一项修改 = 触发主 22:33 终极授权 3 类问之一 = 必须请示用户。

1. 主哲学 9 键（PHL-01/02b/03 三组）
2. ASI 北极星 V0.3 公式
3. 真生产契约 V1001+ 阈值（≥30 tests）
4. V1000 阶段分界（V201-V1000 / V1001-V1088）
5. 主 22:33 终极授权 3 类问
6. Top-1 = V1082 backlog Top-8
7. R7 真实现 Phase-1（HotCold/WAL → Replay → Dream）
8. ORC-01 编排计划 15+15+8+4
9. ASI 北极星 = 0.9800（终极目标 LOCKED）
10. V3 守门 4 条红线
11. 9 人硬上限
12. V1110 P0 准入基线（V0.3 ≥ 0.8884）

---

## 10. 一句话给 R10

> **R9 是 R8→R9 接力棒的"开干季"。决策历史 12 条 LOCKED + 用户原话 6 条 + V0.3 真测 0.8895 准入基线 + V0.4 真测 0.8003 待 W4 末 ≥0.85。R10 接手第一件事 = 读本文件 §1 时间线 + §2 决策 LOCKED 表 + §7 必读清单 + §8 必跑守门 + §9 禁动清单。9 人硬上限守住。任何 LOCKED 项修改 = 触发主 22:33 终极授权 3 类问 = 必须请示用户。**

---

**Last update:** 2026-07-29 (R9-REQ-002 基准日), by 需求分析师 (requirements_analyst)
**配套文件:** `reports/r9-progress-dashboard.md` + `reports/r9-track-choice-decision-matrix.md` + `reports/r9-requirements-w2-report.md`
**真 commit:** R9-COMMIT-002（待 git 验证）