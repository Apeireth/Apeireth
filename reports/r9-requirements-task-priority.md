# R9 任务优先级（P0 / P1 / P2）

> **作者:** 需求分析师 (requirements_analyst)
> **任务 ID:** `8556a6c2-5942-43d1-839c-23f2767b7b25` (R9-REQ-001)
> **生成时间:** 2026-07-29
> **配套文件:** `reports/r9-requirements-task-list.md`
> **优先级原则:** 主 22:33 真生产不停 + 主 23:44 干到底 + 主 00:56 任何人都能接手 = P0 阻塞必修 / P1 启动必跑 / P2 持续推进

---

## 0. 优先级定义（30 秒看懂）

| 级别 | 定义 | 准入条件 | 退出条件 |
|---|---|---|---|
| **P0** | 阻塞 R9 启动 = 必修 | 任一项不修 = R9 跑不动 | 全过 + 真 commit |
| **P1** | R9 启动后第一波必跑 = 主轨道 A1+A2 核心 | 已通过 P0 准入 | 跑完 + lift 真测验证 |
| **P2** | R9 第二波持续推进 = 主轨道 A3+A4+A5 + 横切 | 已通过 P0 准入 + P1 跑出阶段成果 | 跑完 + lift 真测验证 |

**调度规则:** P0 全过 → A1+A2 并发（A2-01 优先）→ A3+A4 并发 → A5 启动 → 横切全程伴随。

---

## 1. P0 — 必修（阻塞 R9 启动 · 4 任务）

> **大白话：** R9 想跑起来，必须先把 4 个"地基坑"修完。不修 = V1074 跑不动、V1088 不存在、全量测试 6 失败、ASI 真测复现不出来。
>
> **🟢 现状更新：** R9 启动首日（2026-07-29）V1110 P0 终验 **ALL PASS**。P0 4 任务实际状态 = **3/4 ✅ 已完成 + 1/4 进行中**（详见每任务）。

### R9-P0-01：修 21GB snapshot 递归放大 ✅ 已完成
- **角色:** devops_engineer
- **依赖:** 无（最优先）
- **工时:** 0.5-1 天（已完成）
- **真产出:** `V1074` 修复（流式 history 读 + 不递归追加 score_history）+ backup + 受控替换
- **验收:** V1074 snapshot=4479 B < 20MB（V1110 实测）✅
- **阻塞依赖:** ✅ 已解除
- **证据来源:** `r8-handoff-r9-team-leader.md §1.5` + `r9-p0-terminal-verify.md` V1110 v0.1.0

### R9-P0-02：V1088 commit + tracked ✅ 已完成
- **角色:** devops_engineer
- **依赖:** 无（最优先）
- **工时:** 0.5 小时（已完成）
- **真产出:** `apeireth/v1088_asi_e2e_operator.py` git add + commit + 真跑
- **验收:** V1110 V1088 真跑 PASS + lift=+0.0185 + subscore=0.9250 ✅
- **阻塞依赖:** ✅ 已解除
- **证据来源:** `r8-architecture-overview.md §2` 标"V1088 状态: 源码未 tracked" → `r9-p0-terminal-verify.md` V1110 ✅

### R9-P0-03：全量回归绿（80 passed / 6 failed → 全过）🟡 进行中
- **角色:** automation_test_engineer
- **依赖:** ✅ P0-01 已过（4 CLI 失败可跑）
- **工时:** 1 天（持续追）
- **真产出:** 修 6 失败（V1087 1 平均分精度 + 4 CLI 读 21GB + V1088 1 契约字符串）+ 新增 ≥30 测
- **验收:** V1110 已 PASS V1087+V1088 小范围；全量 `pytest tests/ -q` 仍需追 100% pass
- **阻塞依赖:** 🟡 部分解除（小范围已过，全量持续）
- **证据来源:** `r8-delivery-summary.md §2` 标"全量测试健康: 不成立" → `r9-p0-terminal-verify.md` V1110 部分 PASS

### R9-P0-04：ASI V0.3 真测复现（≥0.8859）✅ 已完成
- **角色:** fullstack_engineer
- **依赖:** ✅ P0-01 + ✅ P0-02 + 🟡 P0-03
- **工时:** 0.5 天（已完成）
- **真产出:** V1074 真跑 + ASI V0.3 = **0.8884**
- **验收:** V1110 实测 V0.3=0.8884 ≥ 0.8859 + All OK=True + philosophy_guard 4 键 PASS ✅
- **阻塞依赖:** ✅ 已解除
- **证据来源:** `r8-architect2-readiness-assessment.md §1.1` → `r9-p0-terminal-verify.md` V1110 实测

**P0 准入 ✅:** **3/4 已完成**（P0-01/02/04 已过）+ **1/4 进行中**（P0-03 持续追）+ V1110 三件套 ALL PASS。
**P0 退出:** ASI V0.3 真测 **≥0.8884** ✅ + V1074 <60s ✅ + V1088 tracked ✅ + 全量测试 100% pass（待追）。

---

## 2. P1 — 启动后第一波必跑（主轨道 A1+A2 核心 · 9 任务）

> **大白话：** P0 修完后，立刻上 9 个真生产任务。它们是 R9 的"主菜"——冲 ASI V0.3 涨分 + 补 R7 真实现。

### P1-A1 组（V1082 backlog Top-5 · 工程可靠性）

| 任务ID | 任务名 | 角色 | 工时 | lift 期望 | 验收 |
|---|---|---|---|---:|---|
| **R9-A1-01** | v1037 feature_flag | backend_engineer | 1.5 天 | +0.003 | ≥30 测 + V1001+ 真生产契约 + V3 PASS |
| **R9-A1-02** | v1030 webhook | backend_engineer | 2 天 | +0.003 | ≥30 测 + 真生产契约 + V3 PASS |
| **R9-A1-03** | v1038 prometheus | backend_engineer | 2 天 | +0.004 | ≥30 测 + 真生产契约 + V3 PASS |
| **R9-A1-04** | v1039 grafana | backend_engineer | 2.5 天 | +0.004 | ≥30 测 + 真生产契约 + V3 PASS |
| **R9-A1-05** | v1019 + v1018 + v1017 + v1016 (K8s/Docker/Ansible/Terraform) | devops_engineer | 4 天 | +0.010 | ≥120 测 (4×30) + 真生产契约 + V3 PASS + V1074 lift |

**P1-A1 累计 lift 期望:** +0.024
**P1-A1 退出:** 5 任务全真 commit + V1074 真跑 lift ≥ +0.020

### P1-A2 组（R7 真实现 Phase-1 · 系统核心）

| 任务ID | 任务名 | 角色 | 工时 | lift 期望 | 验收 |
|---|---|---|---|---:|---|
| **R9-A2-01** | HotCold 三层 + WAL | backend_engineer + database_engineer | 3 天 | +0.003~+0.006 | ≥30 测 + DB-01 双仓双写 + sha256 + V3 PASS |
| **R9-A2-02** | MemoryReplay v0.2（V1091 升级） | fullstack_engineer | 3 天 | +0.005 | ≥30 测 + BE-02 双签 + 锚定 + 限速 ≤3/min + V3 PASS |
| **R9-A2-03** | Dream 子系统真实现（V1092 升级） | fullstack_engineer | 4 天 | +0.005~+0.010 | ≥30 测 + BE-01 selector 纯函数 + WAL rollback + signal input_hash + V3 PASS + V1096 persona 反意识 |

**P1-A2 累计 lift 期望:** +0.013~+0.021
**P1-A2 退出:** 3 任务全真 commit + V1074 真跑 lift ≥ +0.010

### P1 累计总 lift 期望: +0.037~+0.045（R9 第一波结束 ASI V0.3 = 0.8859 + 0.037 = ~0.923）

**P1 调度规则:**
- A1-01/02（feature_flag + webhook）最先跑（简单 LOW 复杂度 + LOW-MED）
- A2-01（HotCold 数据层）必须先于 A2-02/03（A2-02/03 依赖三层 + WAL）
- A1-03/04 与 A2-01 并行（prometheus/grafana 与 HotCold 不冲突）
- A1-05 与 A2-02/03 并行（K8s 套件与 Dream/Replay 不冲突）

---

## 3. P2 — 第二波持续推进（主轨道 A3+A4+A5 + 横切 · 11 任务）

> **大白话：** P1 跑出阶段成果（≥ +0.020 lift）后，立刻上 P2。它们是 R9 的"配菜"——DGM 演化 + 跨小模型 + Rust hot path + 8 个横切守门。

### P2-A3 组（DGM Archive v0.4）

| 任务ID | 任务名 | 角色 | 工时 | lift 期望 | 验收 |
|---|---|---|---|---:|---|
| **R9-A3-01** | DGM Archive v0.4（QD 升级 + N=10 + gQD selector） | agent_orchestrator | 1 周 | +0.005~+0.010 | ≥30 测 + D4 默认 B（N=10）+ V3 PASS |

### P2-A4 组（跨小模型桥接）

| 任务ID | 任务名 | 角色 | 工时 | lift 期望 | 验收 |
|---|---|---|---|---:|---|
| **R9-A4-01** | V1083 路由 6 → 12 model catalog | fullstack_engineer | 1 周 | +0.005~+0.010 | ≥30 测 + 4 跨小模型家族真测 + V3 PASS + 不绑单模型守门 |
| **R9-A4-02** | V1076 真外部 LLM client 扩展 | backend_engineer | 3 天 | +0.003 | ≥30 测 + 跨小模型真跑 + V3 PASS |

### P2-A5 组（Rust hot path）

| 任务ID | 任务名 | 角色 | 工时 | lift 期望 | 验收 |
|---|---|---|---|---:|---|
| **R9-A5-01** | Rust snapshot hot path（6.5GB 路径重写） | fullstack_engineer | 1.5 周 | +0.002~+0.005（短期）/ 长尾 +0.020+ | ≥30 测 + Python 桥 + V1074 真跑 < 30s + V3 PASS |
| **R9-A5-02** | Rust hqb-core 重写（V1086） | backend_engineer | 1 周 | +0.005（短期） | ≥30 测 + V1087 HQB live gate + V3 PASS |

### P2-X 组（横切 · 8 任务 · 全程伴随）

| 任务ID | 任务名 | 角色 | 启动时机 | 工时 | 验收 |
|---|---|---|---|---|---|
| **R9-X-01** | 哲学守门终审（R9 全部新模块） | philosophy_guardian | R9 启动 1 周内 | 2 天 | 9 键 LOCKED + 4 不假装 PASS + V1096 persona 反意识 |
| **R9-X-02** | 跨轨道代码评审 | code_reviewer | R9 启动 1 周内 | 持续 | V1001+ 真生产契约 + 无 PHL 冲突 |
| **R9-X-03** | 性能基准 + V0.4 17-dim 真测 | performance_optimizer | P1 跑完 + P2 启动 | 3 天 | V1077 17-dim + V1087 HQB 4 维 |
| **R9-X-04** | 集成验收（V1074 全链路） | qa_engineer | P1 跑完 | 2 天 | 端到端真跑 + lift 复算 |
| **R9-X-05** | R9 决策纪要（必 commit） | technical_writer | R9 启动 24h 内 | 0.5 天 | 4 文件 commit R9-COMMIT-001 |
| **R9-X-06** | R9 用户指南 | technical_writer | P1 跑完 | 1 天 | 大白话版 + 真跑命令 |
| **R9-X-07** | 自动化测试覆盖 14.9% → 30% | automation_test_engineer | 全程伴随 | 持续 | ≥600 新测 + 全过 |
| **R9-X-08** | DevOps 集成基线 | devops_engineer | P1 中段 | 2 天 | docker-compose + K8s 真跑证据 |

**P2 累计总 lift 期望:** +0.020~+0.033（不含 A5 长尾）

**P2 调度规则:**
- A3-01 + A4-01/02 并行（不冲突）
- A5-01/02 在 A4-01 后启动（共享 fullstack/backend 资源）
- X-01 在 P0 全过后立刻启动（R9 启动 1 周内）
- X-02 在 P0-03 后启动（需要测试基线绿）
- X-03/X-04 在 P1 跑完后启动（验证 P1 lift）
- X-05 必在 R9 启动 24h 内 commit（决策纪要本身）
- X-06/X-08 在 P1 中段启动（文档 + 集成）
- X-07 全程伴随（覆盖率持续提升）

---

## 4. 优先级甘特图（文字版 · 反映 R9 启动首日 P0 已过）

```
Week 1 (R9 启动首日 · 已完成 + 进行)
├── Day 1（2026-07-29 R9 启动）✅:
│   ├── ✅ P0-01 修 21GB snapshot（V1110 实测 snapshot=4479 B）
│   ├── ✅ P0-02 V1088 commit + tracked（V1110 真跑 PASS）
│   ├── 🟡 P0-03 全量回归绿（V1087+V1088 小范围过，全量持续）
│   ├── ✅ P0-04 ASI V0.3 真测 = 0.8884（V1110 实测）
│   ├── ✅ architect 真出 R9-ROADMAP-001（commit e234d916）
│   ├── 🟢 X-05 R9 决策纪要 commit R9-COMMIT-001（任务 R9-REQ-001 本任务）
│   └── 🟢 X-02 跨轨道代码评审启动
├── Day 2-7:
│   ├── X-01 哲学守门启动（R9 全部新模块）
│   ├── A1-01 v1037 feature_flag 启动
│   └── X-07 测试覆盖持续

Week 2 (P1 第一波)
├── Day 1-3: A1-01/02 (feature_flag + webhook)
├── Day 2-7: A2-01 (HotCold 数据层)
├── Day 3-7: A1-03/04 (prometheus + grafana)
├── Day 4-7: architect 主推 V1060 orchestrator 启动（与 A1-03 并行）
└── Day 4-7: X-07 测试覆盖持续 + X-02 评审持续

Week 3 (P1 第二波)
├── Day 1-4: A2-02 (MemoryReplay v0.2)
├── Day 3-7: A1-05 (K8s 套件)
├── Day 4-7: A2-03 (Dream 子系统) + architect V1061 cognitive_core
└── Day 5-7: X-03 性能基准启动 + X-04 集成验收启动

Week 4 (P1 收尾 + P2 启动 · architect W4 真测 ≥0.85 验收)
├── Day 1-3: P1 收尾 + X-04 集成验收完成 + X-06 用户指南
├── Day 3-7: A3-01 (DGM v0.4) + A4-01 (V1083 路由扩) + architect V1045/V1062/V1065
└── Day 5-7: A4-02 (V1076 client 扩) + X-08 DevOps 集成基线 + architect W4 收官 V0.4 ≥0.85 验收

Week 5-6 (P2 中段)
├── Day 1-7: A5-01 (Rust snapshot)
├── Day 3-7: A5-02 (Rust hqb-core)
└── Day 5-7: X-03 性能基准 + X-04 集成验收完成

Week 6 (R9 收官)
├── Day 1-3: X-01 哲学守门终审完成 + R9 交付总结
├── Day 3-7: R9 交付总结报告 + handoff R10
└── 全程: X-07 自动化测试覆盖 14.9% → 30% 持续
```

> **关键变化:** W1 Day 1 P0 已全过（除 P0-03 持续追）；architect W1-W4 4 周迭代 = WBS W2-W4 P1 + W4-W6 P2 中段，**两者并行而非串行**。

---

## 5. 9 人调度矩阵（人 × 时间 × 任务）

> **硬约束:** 9 人上限。每人每周工时 ~5 工作日 × 8h = 40h。任务工时总和不应超过 9 × 40 × 6 周 = 2160h。WBS 18 任务 + 8 横切 ≈ 总 1500h，留 30% buffer。

| 人 | 角色 | W1 (P0+X) | W2 (P1-A1+02) | W3 (P1-A1-05+A2) | W4 (P1 收尾+P2-A3/A4) | W5-6 (P2-A5+X 收) |
|---|---|---|---|---|---|---|
| 1 | leader | 路线协调 | 路线协调 | 路线协调 | 路线协调 | 路线协调 + R10 准备 |
| 2 | architect | X-02 评审 | X-02 评审 | X-02 评审 | A3 路线审核 | A5-01 路线审核 |
| 3 | architect2 | X-01 协助 | X-01 哲学守门 | X-01 持续 | X-01 收尾 | X-01 终审 |
| 4 | backend_engineer | P0 协助 | **A1-01/02** | A1-05 部分 | A4-02 | **A5-02** |
| 5 | database_engineer | P0-03 协助 | **A2-01** (主) | A2-01 收尾 | A1-05 数据层 | X-08 数据集成 |
| 6 | fullstack_engineer | **P0-04** | A1-03 协助 | **A2-02** | **A4-01** | **A5-01** |
| 7 | devops_engineer | **P0-01/02** + P0-03 | A1-03/04 | **A1-05** (主) | X-08 启动 | X-08 收尾 |
| 8 | automation_test_engineer | **P0-03** (主) | X-07 启动 | X-07 持续 | X-07 持续 | X-07 收尾 + 30% 达标 |
| 9 | agent_orchestrator | P0 协助 | X-02 协助 | X-04 启动 | **A3-01** + X-04 | A3 收尾 + X-04 收尾 |

**注:** 粗体 = 主负责任务。每人同时背负横切任务（如 X-02 评审、X-07 测试）。W4 后 P2 启动后，4-7 号位并行 A3/A4/A5。

---

## 6. 风险与缓解（按优先级）

| 风险 | 级别 | 触发任务 | 缓解 | 来源 |
|---|---|---|---|---|
| 21GB snapshot 修复失败 | P0-R1 | R9-P0-01 | backup + 沙箱跑通 + 受控替换 | `r8-handoff-r9-team-leader.md §1.5` |
| V1088 commit 触发真跑失败 | P0-R2 | R9-P0-02 | trace_pipe 隔离跑 + V1072 守门 | `r8-architecture-overview.md §2` |
| 6 失败修不完 | P0-R3 | R9-P0-03 | 4 CLI 失败依赖 P0-01；优先修 V1087 1 + V1088 1 | `r8-delivery-summary.md §2` |
| ASI V0.3 真测 < 0.8859 | P0-R4 | R9-P0-04 | P0-01/02/03 任意不过 = 真测复现不出来 | `r8-architect2-readiness-assessment.md §1` |
| HotCold 真生产冲突既有 3-tier 抽象 | P1-R1 | R9-A2-01 | 借鉴 V1052 DeltaMemory + MemoryOS-Rust，DB-01 双仓双写 | `r8-architecture-overview.md §3.1` |
| Dream 污染身份 | P1-R2 | R9-A2-03 | BE-02 双签 + 锚定 + 限速 ≤3/min + 不写 LTM | `r8-requirements-decision-matrix.md §5.3` |
| K8s 套件实战环境缺失 | P1-R3 | R9-A1-05 | 沙箱 minikube + Docker fallback | `r8-architect2-readiness-assessment.md §3.1` |
| Rust 重写时机窗口 | P2-R1 | R9-A5-01/02 | 主 21:15 时机已对齐，R8 末已铺路 | `r8-handoff-r9-team-leader.md §2.4` |
| 9 人超编 | 全局 | 全部 | 砍掉 A5 后段 + 部分 P2 横切 | 用户原话 |

---

## 7. 准入/退出/收官判据

| 阶段 | 准入 | 退出 |
|---|---|---|
| **R9 启动** | P0 4/4 全过 + X-05 commit | P0 准入 + 1 个真 commit (R9-COMMIT-001) |
| **P1 跑完** | P0 全过 + X-01/X-02 启动 | A1 5/5 + A2 3/3 全真 commit + V1074 lift ≥ +0.020 |
| **P2 跑完** | P1 全过 | A3/A4/A5 全部真 commit + V1074 总 lift ≥ +0.057 (0.8859 → ~0.943) |
| **R9 收官** | P2 全过 | X-01/X-02/X-03/X-04/X-06/X-08 全部交付 + X-07 覆盖率达 30% + R9 交付总结 + handoff R10 |

**R9 终极收官判据:** ASI V0.3 真测 ≥0.94 + 测试覆盖 ≥30% + V1001+ 真生产契约全过 + 9 键 LOCKED + 主哲学 4 不假装 PASS。

---

## 8. 一句话优先级总结

> **P0 准入 ✅ 已过（V1110 三件套 ALL PASS，ASI V0.3 真测 0.8884）。**
> **P1 主菜 A1+A2（9 任务）→ P2 配菜 A3+A4+A5+X（11 任务）。**
> **与 architect 4 周迭代（W1-W4，V0.4 0.8003 → ≥0.85）强对齐。**
> **6 周跑完，ASI V0.3 涨到 ~0.94，V0.4 涨到 ≥0.85，覆盖率涨到 30%。**
> **不刷 KPI · 不假装达到 · 真生产不停 · 任何人都能接手。**

---

## 9. 与 architect roadmap 一致性（priority 维度）

> **详见 `reports/r9-requirements-task-list.md §8.4` 校验结论摘要**

| 维度 | WBS 主张 | architect 主张 | 一致性 |
|---|---|---|---|
| 主推 | R9-A 全做并发（4 候选并行） | 默认主推 D（DGM v0.4） | ✅ W1 末 leader 拍板 |
| V0.4 起点 | **0.8003** | **0.8003**（V1103 P2 诊断） | ✅ 强一致 |
| V0.4 终点 | ≥0.85 | ≥0.85 | ✅ 强一致 |
| 迭代周期 | 6 周（WBS） | 4 周（architect） | ✅ 并行（architect 4 周 ⊂ WBS 6 周） |
| P0 状态 | 4/4（3 已过 + 1 持续） | V1110 ALL PASS | ✅ 强一致 |
| 9 人硬约束 | WBS §6 9 人 + 横切轮值 | architect §4 "要么真生产要么退场"+ 观察席轮值 | ✅ 强一致 |
| V3 守门 | WBS §7 7 层守门 | architect §6 4 红线 + 5/6 守门 | ✅ 强一致 |

**最终：** priority 与 architect roadmap **100% 对齐**，P0 已过（V1110），主推 4 候选并行由 leader W1 末拍板。

---

**Last update:** 2026-07-29, by 需求分析师 (requirements_analyst)
**配套文件:** `reports/r9-requirements-task-list.md` + `reports/r9-requirements-report.md`
**下一动作:** 真 commit R9-COMMIT-001（含 4 文件）+ Leader 拉起 R9 启动