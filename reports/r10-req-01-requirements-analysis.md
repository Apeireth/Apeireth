# R10 需求分析报告（R10-REQ-01, requirements_analyst）

**作者:** 需求分析师 (requirements_analyst)
**日期:** 2026-07-30
**基线:** R7 收尾（ASI V0.3 = 0.8838）+ 实时测量 ASI V0.3 = 0.8964
**目的:** 把"上一团队意外退出后继续推进"的模糊需求，拆成清晰的功能点 / 非功能要求 / 边界条件 / 依赖 / 验收标准；并把方向微调抛回给用户（主人 / Leader）拍板。

---

## 0. TL;DR（先看这三行）

| 维度 | 当前真实状态 | 下一团队的真实可选路径 |
|------|------------|---------------------|
| **平台愿景** | Apeireth = ASI 真生产平台（5 层架构 + V3 哲学守门 + 中央 AI = 关系总和） | 已 lock，不改 |
| **代码体量** | 1152 模块 / 85.6% 是空壳 / 17.9% 有测试 / ASI V0.3 = 0.8964（天花板 0.9800） | V1082 backlog Top-8 + R7 Phase-1 两条主线 |
| **最大风险** | R7 真实现（HotCold/WAL→MemoryReplay→Dream）只做了契约壳，没跑过真实代码 | 是 R10 的核心"未交付承诺" |

**核心判断:** 上个团队留下的真东西 = 集成工程闭环 V1080→V1088 + ASI 涨 +0.0022。**没真交付的 = R7 Phase-1 真实现 + V1082 Top-8。** 这是 R10 必须消化的两个债务。

---

## 1. 主人原话与立项需求（不变项）

> 直接从 `APEIRETH-MANIFESTO`、`TOP-DESIGN-V1`、`APEIRETH-STAGE-DELIVERY` 提炼，**任何 R10 工作都不能违反**。

### 1.1 哲学需求（不动项）

| 需求 | 来源 | 验收 |
|------|------|------|
| **ASI 北极星** 真生产逼近，不假装达到 | 主 22:33 + 主 20:46 | ASI V0.3 公式 (V21/V1002)，公式可重跑可审计 |
| **V3 守门**：5 不假装（Phenomenal/ASI/Understand/Consciousness/Self） | 主 17:58 + 主 20:46 | `philosophy_guard = PASS`，每模块都过 5 断言 |
| **真生产不停**：数字涨不涨不重要 | 主 23:44 | CI 全绿；任一 G 失败 → 停止后继 + revert |
| **走在前人经验上**：借鉴密度 ≥7 / 任务 | 主 19:33 + 主 22:33 | 每个新模块 7+ 借鉴（GitHub 真源码 / 论文 / 前人哲学） |
| **任何人都能接手**：HARNESS + 阶段交付 | 主 00:56 | 启动 5 步 1 小时内能跑通 V1074 |
| **大胆激进 + 用人话解释** | 主 13:31 + 主 17:38 | 报告用比喻，不用术语堆 |

### 1.2 工程需求（已交付）

| 需求 | 已交付证据 |
|------|-----------|
| 1091+ 真生产模块 | `apeireth/` 目录 1278 文件（含 `__pycache__`/seed/data），v1000+ 真填 |
| 4366+ 真测试 | `tests/` 含集成、回归、混沌、benchmark、e2e 五类 |
| 416+ 真 commit | git log 可查 |
| 5 层守门 V3/V1072/V1074/V1081 | 真代码 + PHL 守门互不绕过 |
| HQB Live Gate (V1085/1086/1087) | `record_decision` + `record_guard_event` + score ≥0.95 强制 VETO |

---

## 2. 上个团队没真交付的（**R10 真实需求来源**）

### 2.1 优先级 1 — V1082 backlog Top-8（ASI 增量 +0.015~+0.025）

| # | 模块 | 文件存在 | 测试存在 | LOC（现） | 目标 LOC | 复杂度 |
|---|------|---------|---------|----------|---------|--------|
| 1 | `v1037_feature_flag` | ✅ 144 行 | ❌ 缺 | 144 | 250~300 | LOW |
| 2 | `v1030_webhook` | ✅ 176 行 | ❌ 缺 | 176 | 250~350 | LOW-MED |
| 3 | `v1038_prometheus` | ✅ 168 行 | ❌ 缺 | 168 | 250~350 | MED |
| 4 | `v1039_grafana` | ✅ 197 行 | ❌ 缺 | 197 | 280~400 | MED-HIGH |
| 5 | `v1019_kubernetes_orchestrator` | ❌ 不存在 | ❌ | 0 | 350~500 | HIGH |
| 6 | `v1023_metrics_aggregator` | ❌ 不存在 | ❌ | 0 | 250~350 | MED |
| 7 | `v1028_log_search` | ❌ 不存在 | ❌ | 0 | 300~450 | MED-HIGH |
| 8 | `v1025_trace_recorder` | ❌ 不存在 | ❌ | 0 | 250~350 | MED |

**关键事实:** 前 4 个有"壳但没测试/没 ASI bridge"，后 4 个连壳都没有。**填满预期 ASI V0.3 从 0.8964 → 0.9114~0.9214**（+0.015~+0.025）。

#### 功能点（每模块通用）

```
F1. 实现模块主文件 apeireth/v10XX_name.py
F2. ≥30 测试用例 tests/test_v10XX_name.py（v1000_yaml_serializer 模式 = 513 LOC 测试）
F3. ASI bridge：12 生命特征 + HQB record_decision + 守门
F4. V1082 --audit --lift 验证（看 LOC / 测试 / bridge 计数）
F5. V1074 --report 验证 ASI V0.3 单调上升
```

#### 非功能要求

- **NFR-1** 不依赖外部网络（`NO_NETWORK=1`），与 V1086 守门一致
- **NFR-2** 错误处理 fail-closed：任何异常必须 raise，不静默吞
- **NFR-3** 不绑单模型（V3 红线），所有 LLM 调用走 V1076 真 LLM client
- **NFR-4** 不刷 KPI：ASI 公式是 V21/V1002 真测量，不允许人造 lift

#### 边界条件

- **B-1** 模块命名严格 `v10XX_name.py`，`XX` 必须在 V1082 audit inventory 里
- **B-2** 已存在壳（v1037/1030/1038/1039）→ 真填，不另起新文件
- **B-3** 不存在壳（v1019/1023/1028/1025）→ 用 V1000 模式从 0 写
- **B-4** 公共 API 必须接 HQB `record_decision`，否则不算填好

#### 依赖

- V1076 真 LLM client（已交付）
- V1085 HQB core（已交付）
- V1086 HQB persistence（已交付）
- V1087 HQB live gate（已交付）
- v1000_yaml_serializer（**模板参考**，必读）

#### 验收标准（DoD）

```
DO-1. 模块 + 测试 + ASI bridge 三件齐
DO-2. pytest -q tests/test_v10XX_name.py 全过
DO-3. V1082 --audit --lift 后该模块 LOC > 0 + with_tests = True
DO-4. V1074 --report ASI V0.3 不下降
DO-5. philosophy_guard = PASS
```

### 2.2 优先级 2 — R7 Phase-1 真实现（**核心承诺兑现**）

**R6-ROADMAP-01 §R7 已承诺，R7-ORC-01 编排已就位，但实际只完成了契约壳/接口冻结，没写一行真代码。**

```
Phase-1 顺序（强依赖）:
  (1) HotCold/WAL        — R7-DB-01, database 主跑
      ↓
  (2) MemoryReplay       — R7-BE-02, backend 主跑, 等 (1)
      ↓
  (3) Dream Subsystem    — R7-BE-01, backend 主跑, 等 (2)
      ↓
  (4) QA-01 集成门       — 混沌/重复/保留/身份漂移
      ↓
  (5) PHL-04 三契约真验证 — 6 项可执行断言
```

详细架构见 `reports/r7-design-01-architecture-blueprint.md` §1-§3（已冻结 15 接口 + 18×4 守门交叉表，不允许新增或漂移）。

#### 功能点（按主线）

**HotCold/WAL（R7-DB-01）**
```
F-HC-1. schema/namespace 冻结；memory + identity 双仓，sha256 snapshot
F-HC-2. WAL append + fsync（先于迁移）
F-HC-3. migrate_hot_to_cold 原子写 + checksum 校验（fail-closed）
F-HC-4. checkpoint_wal + recover_from_wal 崩溃恢复
F-HC-5. FK CASCADE 5 表（hqb.db）
```

**MemoryReplay（R7-BE-02）**
```
F-MR-1. canonicalize 输入 + replay_id+memory_hash 幂等键
F-MR-2. tag 白名单 + identity 锚定（V1072 五项）
F-MR-3. identity_impact_score ≥0.7 双签 + ≤3/min 限速
F-MR-4. 仅 MTM trace 写，不动 LTM
F-MR-5. 缓存/重复 no-op + 审计链
F-MR-6. Dream 写锁期间 wait/cached（互斥协议）
```

**Dream Subsystem（R7-BE-01）**
```
F-DR-1. 6 状态机（DORMANT/WATCHING/CONSOLIDATING/VERIFYING/INTERRUPTED/EMITTING）+ 7 事件
F-DR-2. STM→MTM consolidate（不写 LTM）
F-DR-3. MTM decay/tombstone（关键/身份项禁 decay）
F-DR-4. interrupt/resume 同 run_id 幂等
F-DR-5. deploy/verify/user pause 立即中断 + WAL 回滚
F-DR-6. snapshot emit（V1074 只读，永不直接改 asi_snapshot.json）
F-DR-7. V3 dream_is_not_consciousness + not_understanding 双层守门
```

#### 非功能要求

- **NFR-R7-1** 错误处理 fail-closed：身份/活跃引用/未解依赖强制 retain
- **NFR-R7-2** WAL 先于迁移；checksum 不符 fail-closed
- **NFR-R7-3** 路径必须 resolve 且位于 workspace；拒绝 junction
- **NFR-R7-4** NO_NETWORK=1（沙箱 H1/H2/H3 守门）
- **NFR-R7-5** 跨租户串写 = fail-closed
- **NFR-R7-6** 三层守门：V3（哲学） + V1072（身份） + V1081（诚实），任一失败 → 拒绝 + revert
- **NFR-R7-7** 幂等：同输入重复跑 = no-op（不写、不发、不外调）
- **NFR-R7-8** PHL-04 6 项可执行断言必须真跑（禁止 pass/裸 bool）

#### 边界条件

- **B-R7-1** 不修改 V3 哲学、主人记忆/LTM 语义、ASI 计算或身份定义
- **B-R7-2** 不提前启动 Rust 重写（守 R6-ROADMAP-01 §R12 parity 门）
- **B-R7-3** 公共 API 兼容 V1074/V1082/V1083 CLI
- **B-R7-4** 新增字段向后兼容；WAL/schema 版本化
- **B-R7-5** Dream 只 emit 测量数据，不直接写 `artifacts/asi_snapshot.json`（V1074 守门）

#### 依赖

- V1085 HQB core（✅）
- V1086 HQB persistence（✅）
- V1052 Reconsolidator（✅）
- R6-RES-07 Replay 协议（✅）
- V1072 五项身份项（✅）
- R6-RES-06 Dream 设计（✅）

#### 验收标准（DoD）

```
DO-R7-1. 三主线真实代码（不是契约壳），每线有 ≥30 测试
DO-R7-2. pytest -k "dream or replay or hot_cold or wal" 全过
DO-R7-3. 重复迁移幂等；注入中断后恢复 hash 一致
DO-R7-4. 越权 LTM / 未知 tag / 限速 / 污染样例 fail-closed
DO-R7-5. V1074/V1082/全量 G 全 PASS
DO-R7-6. PHL-04 6 项可执行断言 PASS（不是 pass/裸 bool）
DO-R7-7. 报告记录版本、快照 hash、测试计数、已知限制
DO-R7-8. 任一数据校验 / 四层门 / G / 沙箱测试失败 → 停止下一阶段 + revert
```

### 2.3 优先级 3 — R8 调研（R7 真实现完成后启动）

`r7-handoff-next-team-leader.md` 推荐的 4 个未覆盖领域：

| # | 领域 | 与 R7 关联 | 启动条件 |
|---|------|----------|---------|
| 1 | 形式化验证（TLA+/Coq/Isabelle） | R6-PHL-03 formal_verify 契约壳补完 | R7 PHL-04 PASS 后 |
| 2 | 机制设计（auction/contract theory） | 主人经济/攻防域 | R7 后 |
| 3 | 计算最优律（Kolmogorov / Solomonoff） | ASI 北极星哲学 | R7 后 |
| 4 | 因果推断深挖（Pearl do-calculus） | R4-RES-03 已部分覆盖 | 随时可启 |

**R10 不强制启动**，看用户优先级反馈。

---

## 3. 真隐式需求（用户没说但必须做）

### 3.1 真测先行（绝不刷分）

- ASI V0.3 = 0.8964 是真测量，公式可重跑；R10 任何工作后必须 V1074 跑一次
- 数字涨不涨不重要（主 23:44），但**不允许连续 3 次下降**
- 不允许人为构造 lift（V1082 lift 是审计可重算）

### 3.2 测试覆盖提升

- 当前 17.9% → 填 Top-8 后预计 ~30%
- 30% 是 R10 的隐性 KPI（不在需求文档但实际会被 V1082 跟踪）

### 3.3 已记录技术债（必读）

| # | 项 | 优先级 | 修复 |
|---|----|--------|------|
| 1 | test_v1077 capture I/O 污染 | LOW | pytest fixture 关闭后清理 stdout |
| 2 | V1074 性能 16s → <10s | MED | V1071 深读缓存共享 + V1082 inventory 共读 |
| 3 | 14.9%（实 17.9%）测试覆盖 | HIGH | V1082 backlog 填完 |
| 4 | integration worktree 未初始化 | LOW | 运维侧 init |
| 5 | 2 个 FINAL-IDLE task 卡 review_pending | LOW | 等 60s 自动重评 |

### 3.4 跨域借鉴密度（主 19:33 强约束）

每个新模块必须 7+ 借鉴：GitHub 真源码深读（`code-deep-study/` 20 个）+ arxiv 论文 + 前人哲学。R7 Phase-1 的 Dream/Replay/HotCold 设计阶段已写 31 项借鉴，实现阶段需保持。

### 3.5 不刷 KPI 守门（V3 红线）

- ❌ 不假装 Phenomenal consciousness
- ❌ 不假装达到 ASI
- ❌ 不假装理解 / 自由 / 价值（V3 七哲学 0.83~0.95 都不是 1.0）
- ❌ 不绑单模型
- ❌ 不破坏四层安全门

---

## 4. 不在 R10 范围内（**不要做**）

| 项 | 理由 |
|----|------|
| **Rust 重写** | R6-ROADMAP-01 §R12 parity 门禁止提前启动；rust-substrate 设计已就位但实现要等 R7+ 全绿 |
| **改 V3 哲学** | 主 22:33 终极授权 + 主 17:58 不假装原则；除非主人显式拍板否则不动 |
| **改 ASI V0.3 公式** | 公式是 V21/V1002 真测的北极星；动公式必须经过主人 3 类问 |
| **新增调研轮 (R1 survey 未覆盖)** | 见 §2.3，但 R10 内不强制；用户没明确说 |
| **改中央 AI 5 位置定义** | 主 22:33 已锁，V2 已补完 |
| **改启动创世 8 问题** | 主人 13:04 第 1 条认可 + V3 守门 |

---

## 5. 风险与升级路径

### 5.1 技术风险

| 风险 | 触发条件 | 升级路径 |
|------|---------|---------|
| R7 Phase-1 真实现失败 | 任一 G/V1074/沙箱/PHL-04 失败 | 立即停止下一阶段，写 taxonomy，原子 revert，向主人请示 |
| V1082 Top-8 填到一半 ASI 反降 | V1074 单调性破坏 | 检查 NFR-4（不刷 KPI），回滚最近提交 |
| 接口漂移（DR-2 例外） | 任何接口变更无 architect2 签字 | freeze R7-DESIGN-01 §3 18 行接口表为单一真理源 |
| 借鉴不足 7 项 | 新模块借鉴密度 < 7 | 必读 `code-deep-study/deep-study-v2.json` |

### 5.2 方向风险

| 风险 | 触发条件 | 升级路径 |
|------|---------|---------|
| 主人对方向有重大调整 | 用户在 R10 启动前说"换方向" | 需求重新对齐，重写本报告 §2 |
| 用户想跳过 R7 Phase-1 直奔 R8 调研 | 用户显式说 | 不推荐，Phase-1 是主哲学守门真验证；但听主人的 |
| 用户想先做 Rust 重写 | 用户显式说 | 触发 R6 parity 门检查，需主人拍板 |

### 5.3 系统风险（已知）

- 2 个 FINAL-IDLE task 卡 review_pending（system bug）— 不阻塞推进，60s 自动重评
- integration worktree 未初始化 — 运维侧启动后会自动 review_blocked，建议 R10 启动前先解

---

## 6. 验收维度（汇总 DoD）

R10 启动后，每完成一个优先级都必须满足：

```
[A] 真代码（非契约壳），commit 到 master
[B] 真测试 ≥30 用例 / 模块
[C] ASI bridge（HQB record_decision + 12 生命特征）
[D] V1074 --report 跑过：All OK: True + philosophy_guard: PASS + ASI V0.3 不下降
[E] V1082 --audit --lift 跑过：模块 LOC > 0 + with_tests = True + bridge 计数上升
[F] 借鉴密度 ≥7 / 模块
[G] 失败时 revert + taxonomy + 向主人升级
```

---

## 7. 待用户拍板的 3 个真实问题

> 这些必须问，否则 R10 推进方向不明确。

### Q1：R10 优先级 1 vs 优先级 2 怎么排？

**两个都是"上一团队没真交付的承诺"，但本质冲突：**
- **优先级 1（V1082 Top-8）** = 增量清晰（ASI +0.015~+0.025），可拆给 1~2 人并行，单模块 1~2h 就能完成一个
- **优先级 2（R7 Phase-1）** = 真生产闭环（Dream/Replay/HotCold），影响主哲学守门真验证，但风险高、需要 database/backend/qa/phl 四个角色串行 ~6.5h

**我的建议（不一定对）：**
- **方案 A（稳）**: 先做优先级 1 拿 ASI 增量 + 练手 → 然后做优先级 2
- **方案 B（激）**: 直接做优先级 2（因为是主哲学承诺兑现），Top-8 同步并行
- **方案 C（保守）**: 只做优先级 1 的前 4 个（有壳的真填），把 R7 Phase-1 留给 R11

### Q2：R10 是否启动调研（R8 调研四个领域）？

- 主人原话 "**走在前人经验上**"（主 19:33），但 R7 真实现优先
- 我的建议：**R10 内只完成 R7 Phase-1 + V1082 Top-8，不启动新调研轮**；R11 再开 R8

### Q3：是否启动 Rust 重写？

- rust-substrate/ 设计已就位（apeireth-core/cli/gateway/ports/adapters/py）
- 但 R6 parity 门禁止提前启动
- 我的建议：**R10 不启动**；R11+ 在 R7 全绿 + 主哲学守门真验证后再评估

---

## 8. 推荐启动序列（等 Q1 拍板后定）

```
=== 如果用户选方案 A（推荐）===
D1: V1082 audit --lift 重跑，确认 0.8964 → 锁定基线 snapshot
D1: 启 Top-8 #1 (v1037_feature_flag) — backend 角色 1.5h
D2: Top-8 #2 (v1030_webhook) — backend 角色 1.5h
D2: Top-8 #3 (v1038_prometheus) — devops 角色 2h
D3: Top-8 #4 (v1039_grafana) — fullstack 角色 2.5h
D3-D4: Top-8 #5-8 — 串行 4h（database / backend / devops）
D5: V1074 重跑，预期 ASI V0.3 ≥ 0.9114
=== D5 后启动 R7 Phase-1 ===
D6: HotCold/WAL (database) — 1.5h
D7: MemoryReplay (backend) — 1.5h
D8: Dream (backend) — 1.5h
D9: QA-01 集成门 (qa) — 1.5h
D10: PHL-04 三契约真验证 (philosophy_guardian) — 0.5h
=== 总计 ≈ 2 周（10 个工作日，假设 1 人=1 角色并行上限 9）===

=== 如果用户选方案 B（激进）===
Top-8 后 4 个（v1019/1023/1028/1025）与 R7 Phase-1 HotCold 并行
```

---

## 9. 文档入口（启动必读）

| 文档 | 路径 |
|------|------|
| **主人哲学** | `.openclaw\workspace\promethean\APEIRETH-MANIFESTO-ORIGINAL-2026-07-20.md` |
| **顶层设计** | `TOP-DESIGN-V1.md`（5 层架构 + 启动创世 8 问题） |
| **白皮书** | `WHITEPAPER-ASI-PLATFORM-2026-07-20.md`（ASI 真路径） |
| **阶段交付** | `APEIRETH-STAGE-DELIVERY-2026-07-22.md`（19 章全覆盖） |
| **R6→R7 总结** | `reports/r7-final-summary-leader.md`（关键指标 + V1082 backlog） |
| **R7 handoff** | `reports/r7-handoff-next-team-leader.md`（启动 5 步 + 推进路径） |
| **R7 蓝图** | `reports/r7-design-01-architecture-blueprint.md`（L0-L7 + 接口冻结表） |
| **R7 路线图** | `reports/r7-roadmap-real-impl.md`（Phase-1 顺序 + DoD） |
| **agent-context** | `agent-context/{AGENTS,SOUL,IDENTITY,USER,TOOLS}.md` |
| **代码深读** | `code-deep-study/deep-study-v2.json`（借鉴字典） |

---

## 10. 我的角色判断

作为需求分析师，我已读完调研 + 上阶段交付 + handoff + 设计蓝图。**真实需求 = 消化两个"未交付承诺"**：V1082 Top-8 增量 + R7 Phase-1 真实现。其他都是这两个主线的从属动作。

**我不做的事**（避免越界）：
- 不写代码（角色边界）
- 不调 ASI 公式（主 22:33 + V3 红线）
- 不动主哲学（主 17:58 + 22:33）
- 不替主人拍板方向（Q1/Q2/Q3 留给用户）

**我等你拍板 Q1（推荐方案 A），然后 Leader 会基于本报告分任务给具体角色。**

---

_需求分析师 R10-REQ-01 · 2026-07-30 · 基于 R7 收尾 + 实时 V1074/V1082 真测_