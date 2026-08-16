# 09 — 哲学守门 Addendum（重建版）

> ⚠️ **文档性质：重建版（Reconstructed）**
> 原稿 `docs/v2-strategy/09-PHILOSOPHY-GUARD-ADDENDUM.md` 在 v2.0.0-alpha 期间产出，但**从未进入 git 历史**（2026-08-17 C3 盘点核实：`git log --follow` 无记录、整合 #4 树 abf12243 与 integration 分支树均无此文件）。本文档为 2026-08-17 按 Leader 派活（任务 212699c1）**从现存残存材料重建**。
> **重建红线（0 装 PASS）**：只汇编有出处的内容；无法从现存材料恢复的部分如实标注「⚠️ 原稿丢失」，绝不虚构原文。

| 字段 | 值 |
|---|---|
| 重建日期 | 2026-08-17 |
| 重建执行 | 技术文档2（任务 212699c1-64c2-4778-835a-f5226dc279de） |
| 重建依据 | 见 §8 出处对照表（7 份现存材料） |
| 状态 | LOCKED（承继原稿锁定状态；修改须走对应流程） |
| 权威排序 | 与 `docs/team-work-doc.md` 冲突时，team-work-doc 优先（作战文档为团队执行唯一权威） |

---

## §0 什么决策必须过哲学检查（守门范围）

> 出处：`docs/team-work-doc.md` §1（三哲学）+ `docs/release-plan.md` §一（原文锚点）

主人哲学是项目**最高约束**。以下决策在落地前必须过哲学检查：

1. **任何关于"AI 是什么"的定义性决策** — Apeireth 是基地（给 LLM 的操作系统），不定义 AI 本体。（出处：team-work-doc §1.1-1「基地，不是 AI 本身」）
2. **任何能力新增/演化路径决策** — 必须检查是否服务于「涌现优先于预定义」；纯预定义堆能力 = 违反主人原话「我希望它能自己演化，否则我们是永远做不完能力的」。（出处：team-work-doc §1.1-2；release-plan §一）
3. **任何记录/迁移/身份表述决策** — 只提供最大努力的记录 + 迁移（continuity_id 锚点），**不假装灵魂同一**。（出处：team-work-doc §1.1-4）
4. **任何安全机制决策** — 安全 = 能力限制 + 洋葱门 + 宪法评审 + 主人批准 + 熔断；禁止关键词规则堆砌；必须过 token 经济性检查。（出处：team-work-doc §1.1-5）
5. **任何发布/宣传/指标表述** — 必须过「5 项不假装」（见 §2.2）；任何 ASI/意识/完美性声称默认禁止。（出处：stage2-decisions-philosophy-guard.md §3；team-work-doc §1.2「0 装 PASS」）
6. **任何 crate/模块增删决策** — 必须过「机制而非补丁 / 集成而非分立」检查；官方交付整件、社区拼插件的三层交付边界不得打破。（出处：team-work-doc §1.2/§1.3）
7. **任何涉及主人批准权/熔断权的改动** — AI 永远不接触 master token；高危操作走「AI 请求 → 主人批准」。（出处：team-work-doc §1.3-5）

---

## §1 守门实现架构（现存材料可恢复部分）

### §1.1 双锁 + AND 门（R14-D8-fix 最终版）

> 出处：`docs/stage2/stage2-decisions-philosophy-guard.md` §R14-D8 勘误（主 2026-07-31 精化 + 同日纠偏）；`docs/stage1/onion-wall-architecture-2026-07-31.md` §3

- 哲学守门**不是独立 crate**，而是**两把独立锁 + 最后 AND 运算**：
  - **锁 A = 原则洋葱**（意义约束）：E/S/A/M/O 5 层
  - **锁 B = 权限洋葱**（授权凭证）
  - `dispatcher.dispatch()` 双锁调度 + `human_gate.rs` HA 硬门槛
- 历史轨迹（不删除）：曾表述为「交叉咬合的城堡内墙 / onion_wall 模块」「`OnionGate::guard_decision(DecisionSignature)`」「独立 apeireth-philosophy crate」——均已被主人纠偏取代，保留见原勘误节。
- **现状锚点**：`crates/apeireth-onion/src/lib.rs:216`「双洋葱统一体 trait — V1+V2+V3 AND 门（原则嵌入权限）」（代码实测存在）。

### §1.2 原则洋葱 5 层 + 跨层仲裁

> 出处：stage2-decisions-philosophy-guard.md §1/§4

```
E 层 (原则)   — 编译时 hardcode + 运行时拦截 + 多 AI 一致（不可违背）
S 层 (价值观) — 智囊团审核 + 物理多签
A 层 (经验)   — AI 自己可改 + 版本备份
M 层 (方法论) — AI 自己可改 + promotion 管道
O 层 (操作)   — AI 自己可改 + 9 键守门

仲裁: E > S > A > M > O（高优先级覆盖低优先级）; 同层冲突: 后入胜 (LIFO)
```

### §1.3 V3 哲学契约 9 键

> 出处：stage2-decisions-philosophy-guard.md §2（R11 已落，LOCKED 不重写）

| 键组 | 键 | 语义 |
|---|---|---|
| PHL-01 (not_X) | NotClone / NotPerfect / NotUuid | 不假装克隆宇宙 / 不假装 100% 完美 / 不假装唯一解 |
| PHL-02b (not_X) | NotUndo / NotProof / NotSafe | 不假装撤销已发生 / 不假装完整证明 / 不假装完全安全 |
| PHL-03 (X_is_not_Y) | SpecIsNotProof / CounterexampleIsNotBug / ProverIsNotTruth | 规格≠证明 / 反例≠bug / 证明者≠真理 |

注：constraint crate 现注释记为「E/S/A/M/O 5 层 + 12 键 + 5 项不假装」（`crates/apeireth-constraint/src/lib.rs:18`），12 键为 9 键后续扩展口径——⚠️ 9→12 键的增量明细原稿未涵盖，现存材料亦无完整定义，待补。

### §1.4 5 项不假装（V1138）

> 出处：stage2-decisions-philosophy-guard.md §3

1. 不假装达到 Phenomenal consciousness
2. 不假装 ASI
3. 不刷 KPI
4. 不假装完整证明
5. 不假装 100% 完美

与 9 键互补不重叠：9 键更宽泛（编译时+运行时、通用哲学约束），5 项不假装更具体（运行时强制、项目特定）。

### §1.5 4 重守门 v15 LOCKED

> 出处：`crates/apeireth-constraint/src/lib.rs:14-21`（代码实测）；`docs/stage4/stage4-correction-v15-four-gates-permission-grant.md`（被其引用）

| Gate | 层 | 机制 |
|---|---|---|
| Gate 1（内层） | 编译时 hardcode | 原则洋葱 5 层 + 键 + 5 项不假装 |
| Gate 2（中间） | 运行时拦截 | verdict cache O(1) 查询 |
| Gate 3（外层） | 物理隔离 | 重大修改需物理访问 + 多签（critical = 7 席全量） |
| Gate 4（最外） | 反思期审计 | Cognitive-Dream 72h 监控守护越权检查 |

v15 命名修正：`FiveGates` → `FourGates`（FiveGates 保留为 deprecated 别名）。

### §1.6 强制执行的 3 期 + E 层 5 重

> 出处：stage2-decisions-philosophy-guard.md §5/§6

- 3 期：编译期（const 断言，binary 不可改）+ 运行时（拦截器，所有决策前必过）+ 反思期（事后审计）
- E 层 5 重守门：hardcode + 运行时拦截 + 编译期断言 + 多 AI 一致（3 个 AI 独立检查）+ 物理隔离（修改需 Layer 5 升级 + 物理多签）

---

## §2 反例清单（守门必须拦下的决策形态）

> 出处：team-work-doc §1.2 工程哲学反例列 + §1.4 不假装条款；stage2-decisions-philosophy-guard.md §3.1 守门规则

| 反例 | 违反条款 |
|---|---|
| 返回 Ok 假装成功；文档写"已支持"实际没有 | 0 装 PASS（team-work-doc §1.2） |
| 加 if 判断绕过一个缺失的机制（补丁而非机制） | 机制而非补丁（team-work-doc §1.2） |
| 为小功能新建一套并行的 store/调度器 | 集成而非分立（team-work-doc §1.2） |
| 声称达到 ASI / 意识 / 主观体验 | 5 项不假装 #1/#2（V1138） |
| 刷 KPI（指标好看但无真实证据） | 5 项不假装 #3（V1138） |
| 声称"完整证明 / 100% 完美 / 绝对安全 / 唯一解" | 9 键 NotProof/NotPerfect/NotSafe/NotUuid |
| 把规格当证明、把反例当 bug、把证明者当真理 | 9 键 PHL-03 组 |
| AI 接触 master token / 越过主人批准执行高危操作 | 洋葱门 HA 硬门槛（team-work-doc §1.3-5） |
| 同 id 重写 append-only 数据（记忆/经验/原则/审批） | versioned chain（team-work-doc §1.3-6） |

---

## §3 ASI 北极星指标列（部分重建）

> 出处：`docs/RELEASE-NOTES-v2.0.0-alpha.md` §1.7（原稿结构转述）；`docs/stage2/03-EXTREME-PLAN.md` §阶段 1-4；`docs/CONTEXT-HANDOVER.md:265`

- **目标窗口**：18 个月（极致版路线图 03-EXTREME-PLAN.md：阶段 1 Month 2-4 补齐短板 / 阶段 2 Month 5-8 Multi-Agent+图编排 / 阶段 3 Month 9-12 生态接入+标杆 / 阶段 4 Month 13-18 登顶）
- **V0.5 阶梯（alpha 转述）**：0.86 → 0.87 → 0.89 → 0.92 → 0.98（每 3 个月真测）
- **参照基线**：V1136 真测 0.9063（CONTEXT-HANDOVER.md:265）；V0.5 24 维 + V1136 9 子测度 + 12 键（00-R14-START-HERE.md:80 二手锚点）
- **现状代码锚点**：`crates/apeireth-bench/src/asi-v05.rs` + `asi_v05_e2e.rs` + `oracle.rs` 全链串联（release-plan §4.6 ✅）
- ⚠️ **原稿丢失部分**：「5 阶段」的逐阶段定义（alpha release-notes 称原稿为 5 阶段，现存 03-EXTREME-PLAN 为 4 阶段制，二者口径差异无法从现存材料弥合）；V0.5 24 维逐项定义与 V1136 9 子测度逐项定义（原载 docs/APEIRETH-CONVENTIONS.md §11，该文档同样从未入 git 历史，仅有二手引用锚点）。

---

## §4 5 个新 crate × 22-trait 互锁 traceback（部分重建）

> ⚠️ **原稿丢失部分**：原 §C 的「22 个 trait 互锁追溯矩阵」逐条明细无法从现存材料恢复（04-CRATE-CONSOLIDATION.md 现存版本无 22-trait 矩阵内容）。以下仅重建 crate 级 traceback（2026-08-17 实测）：

| crate | 2026-08-17 现状 | 安全/哲学接口锚点 |
|---|---|---|
| apeireth-mcp | ✅ 完整 crate，conformance 9 + multi_transport 9 测试 | 走 tool-registry 桥接（不修改承诺），权限经 registry 链 |
| apeireth-graph | ✅ 8 模块真实现 + 3 smoke tests | 数据经 checkpoint/append-only 语义 |
| apeireth-vector | ✅ sqlite_backend + qdrant_compat | 仅存储层，无决策权 |
| apeireth-sdk | ✅ 多语言 FFI（C/Node/Lark/LiveKit） | 客户端边界，不接触守门内部状态 |
| apeireth-bench | ✅ 112KB（swe/self_disable/latency/agent） | **self_disable_bench 20 case + 5 守门**（Self-Disable 机制的验收口，呼应 §0-5） |

互锁关系（现存材料可确认部分）：5 crate 全部为 workspace 成员并受 `rust.yml` 全 workspace CI 覆盖（build --workspace --tests + nextest --workspace）；自我禁用守门经 apeireth-bench 的 20 攻击 case 回归（门槛 ≥5/20，实测 default_cases_all_expected_blocked）。

---

## §5 与增补决策体系的关系

> 出处：`docs/stage2/stage2-decisions-addendum-sovereignty-continuity-governance.md`（2026-07-31，现存完整 878 行）

本 addendum（哲学守门）与下列增补决策**同层并存、正交切分**：

- **三域硬规则**（思想/提案/行动）：跨域决策必须过守门（§2.2 硬规则不允许跨域）
- **SGI（自主目标意图单字段）**：锁不可变项、不冻结优先级权重 — 其「不可变项」即本守门的 E 层对象
- **可审计主体连续性 + 3 清单硬门槛**：升级/迁移决策的哲学检查口（主 17:58 不假装）
- **6 历史流 + 根层加权治理**：治理决策按 E>S>A>M>O 加权，与本守门仲裁一致

---

## §6 主哲学 anchor（6 全贯穿）

> 出处：stage2-decisions-philosophy-guard.md §8

- 主 22:33 S-1（哲学守门服务 ASI 方向）
- 主 17:43 S-2（基于 V3 9 键 + 5 项不假装已有，强化使用）
- 主 17:58 O-5（不假装哲学的物理实现）
- 主 19:33 O-2（跨层仲裁借鉴权限模型）
- 主 23:44 O-3（干到底）
- 主 00:56 O-4（任何接手者能查）

---

## §7 丢失与重建边界声明

| 部分 | 状态 | 说明 |
|---|---|---|
| §0 守门范围 | ✅ 重建（有出处） | team-work-doc §1 + release-plan §一 |
| §1.1-§1.6 守门实现 | ✅ 重建（有出处） | stage2-decisions-philosophy-guard.md + onion-wall 架构文档 + constraint/onion crate 实测 |
| §1.3 9→12 键增量明细 | ⚠️ 原稿未涵盖/现存无定义 | 仅代码注释口径锚点 |
| §2 反例清单 | ✅ 重建（有出处） | team-work-doc §1.2 + V1138 |
| §3 ASI 北极星 | 🟡 部分重建 | 18 个月/阶梯/基线有出处；5 阶段逐段定义、24 维/9 子测度逐项定义丢失（APEIRETH-CONVENTIONS.md 同为失传产物） |
| §4 22-trait 互锁矩阵 | ❌ 原稿丢失，仅 crate 级 traceback 重建 | 逐条 trait 矩阵无法从现存材料恢复 |
| §5/§6 | ✅ 重建（有出处） | stage2 两份增补决策 + 哲学守门 §8 |

---

## §8 出处对照表（重建依据）

| # | 现存材料 | 提供的重建内容 |
|---|---|---|
| 1 | docs/team-work-doc.md §1 | 三哲学、工程哲学反例、架构哲学（守门范围 §0、反例 §2） |
| 2 | docs/release-plan.md §一 | 设计原意原文锚点（§0-2/3/4） |
| 3 | docs/stage2/stage2-decisions-philosophy-guard.md（716 行完整） | 9 键 / 5 项不假装 / 5 层仲裁 / 3 期强制 / E 层 5 重 / R14-D8 勘误（§1 全部） |
| 4 | docs/stage1/onion-wall-architecture-2026-07-31.md | 双锁 AND 门架构（§1.1） |
| 5 | crates/apeireth-constraint/src/lib.rs + crates/apeireth-onion/src/lib.rs | 4 重守门 v15 / 双洋葱 AND 门代码锚点（§1.1/§1.5） |
| 6 | docs/RELEASE-NOTES-v2.0.0-alpha.md §1.7 | 原稿结构转述（§3 阶梯 / §4 矩阵框架） |
| 7 | docs/stage2/stage2-decisions-addendum-sovereignty-continuity-governance.md（878 行完整） | 三域/SGI/3 清单/6 历史流关系（§5） |

**重建者自审**：本文档无一处虚构原文；所有 ✅ 段落均可回溯到上表材料的具体位置；所有 ⚠️/❌ 段落均为原稿丢失的如实标注。
