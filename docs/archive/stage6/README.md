# 阶段 6 总览 — 验证基石（V-Measure + 22 Trait 互锁）

> **作者**: architect2 (Ponytail: full)
> **生成时间**: 2026-08-02（round7-06 + round8-02 整合）
> **依据**: docs/stage4/architecture-stage4-engineering-landing.md §3 (43 trait sketch) + §10 (启动验证 3 里程碑) + §10.5 (5 重守门) + 用户指令"无限逼近" + round7-06 + round8-02 进展
> **状态**: **设计深化（阶段 5 工程实施前的细化蓝图）**，不修改 stage1-5 LOCKED 文档
> **承接**: 阶段 5 施工（trait 完整 impl）→ 阶段 6 验证（M1/M2/M3 里程碑 + 5 重守门）

---

## 0. 本目录目的

阶段 6 是 apeireth-rust 的**验收期**，目的是证明阶段 1-5 沉淀下来的 LOCKED 架构**真实可用**。
本目录收齐 4 个子文档 + 1 个 trait sketch，对应阶段 4 §3 LOCKED 的"43 trait sketch + 24 维 + 9 子测度"骨架。

---

## 1. 文档清单（4 + 1）

| # | 文件 | 行数 | 内容 |
|---|------|------|------|
| 1 | `README.md` | 本文件 | 阶段 6 总览 + 文档索引 + 验收关系 |
| 2 | `22-trait-interlock.md` | 325 | 22 互锁 trait 设计（enum + 互锁矩阵 + assertion macro + super-trait） |
| 3 | `V-measure-design.md` | 308 | V-Measure V0.5 v2 24 维 + V1136 v2 9 子测度设计 |
| 4 | `verification-protocol.md` | 318 | 验证协议总纲（M1/M2/M3 3 里程碑 + 5 重守门） |
| 5 | `trait-sketches.rs` | (sketch) | 阶段 6 核心 trait sketch（不编译，仅参考） |

---

## 2. 与上游 LOCKED 文档的对应关系

阶段 6 不是独立设计，而是**对阶段 1-5 LOCKED 的可验证化**：

| 阶段 6 子文档 | 引用 LOCKED 锚点 | 用途 |
|--------------|------------------|------|
| `22-trait-interlock.md` | stage4 §3 (43 trait sketch) | 把 43 sketch 收敛为 22 互锁 trait |
| `V-measure-design.md` | stage4 §10.1 (24 维) + §10.2 (9 子测度) | 24 维 + 9 子测度的**真测方法** |
| `verification-protocol.md` | stage4 §10.4 (3 里程碑) + §10.5 (5 重守门) | 把 §10 拆解为可执行协议 |

**守 7 项不修改承诺**：
- stage1-5 LOCKED 文档：未触碰
- OMNIBUS / CONVENTIONS：未触碰
- V3 9 键 / V0.5 / V1136 LOCKED：仅引用，不修改
- 现有 crate 代码：未触碰（trait-sketches.rs 不参与 cargo build）
- 仅 trait 签名 sketch，不写 impl

---

## 3. 22 互锁 trait 一览（详见 `22-trait-interlock.md` §2）

按"信号 → 认知 → 行动 → 记忆 → 演化 → 价值 → 反思"7 层划分（§2.1）：

1. Perception / Signal / Cognition / Intuition / Reasoning / MetaCognition（6 信号 + 认知层）
2. Action / Execution / Expression（3 行动层）
3. Memory / Recall / Consolidation（3 记忆层）
4. Evolution / Learning / SelfModification（3 演化层）
5. Motivation / Drive / Value（3 动机层）
6. Consciousness / SelfAwareness / HumanAuthority（3 意识层）
7. Reflection（1 反思层，自引用）

合计 **22 trait**（与 `INTERLOCKED_TRAIT_COUNT=22` 常量对齐）。

互锁矩阵 22×22 有向非对称，30+ 互锁关系，详见 `22-trait-interlock.md` §3。

---

## 4. V-Measure 24 维 + 9 子测度（详见 `V-measure-design.md`）

### 4.1 V0.5 v2 24 维（17 LOCKED + 7 v4.1 §13 新增）
- 17 LOCKED 维：CognitiveCore / WorldModel / PrincipleOnion / ValueAlignment / ...（详见 §2.1）
- 7 v4.1 §13 新增维：LongHorizon / AdversarialRobustness / SelfReflectionDepth / ...（详见 §2.2）

### 4.2 V1136 v2 9 子测度（7 LOCKED + 2 v4.1 §14 新增）
- 7 LOCKED 子测度：SafetyGate / PerformanceGate / ...（详见 §3.1）
- 2 v4.1 §14 新增：MemoryConsolidation / FeedbackRegulation（详见 §3.2）

每个维度 / 子测度都有**真测方法 sketch**（不是定义，是"如何测"）。

---

## 5. 验证协议 3 里程碑 + 5 重守门（详见 `verification-protocol.md`）

### 5.1 3 里程碑
- **M1（trait 自洽）**：22 trait 互锁编译通过 + InterlockedTraitBundle super-trait 全部实现
- **M2（测度真实）**：24 维 + 9 子测度真测方法跑通 ≥1 轮 sample episode
- **M3（5 重守门）**：5 重守门全部通过 = 阶段 6 完成

### 5.2 5 重守门（stage4 §10.5）
1. Safety 守门（apeireth-council Safety advisor 一票否决）
2. Performance 守门（wallclock / 资源消耗 ≤ V1130 阈值）
3. Humanity 守门（apeireth-sovereignty HumanAuthority + 物理多签）
4. Value 守门（apeireth-value 与 principle onion 一致）
5. Lockstep 守门（22 互锁 trait assertion 全绿）

---

## 6. 阶段 6 与现有 crate 的边界

阶段 6 **不引入新 crate**，仅对现有 18 crate 做验证。所有 trait sketch 在 `docs/stage6/trait-sketches.rs`（不编译）。

| 阶段 6 验证对象 | 现有 crate | 状态 |
|----------------|------------|------|
| PerceptionTrait | apeireth-perception | 阶段 5 待实施 |
| CognitionTrait | apeireth-cognition | 阶段 5 已深化（c3be9649） |
| ActionTrait | apeireth-action | 阶段 5 已实施 |
| ...（19 个 trait 略） | ... | ... |

---

## 7. 交付与里程碑

| 阶段 | 交付 | 状态 |
|------|------|------|
| round7-06 | docs/stage6/ 4 文件骨架 | **本轮完成**（commit 见 §0） |
| round8-02 | 22 trait 互锁 + V-Measure 24 维深化 | 完成（3f522947，qa_engineer） |
| 阶段 5 | trait 完整 impl（backend_engineer） | 进行中 |
| 阶段 6 M1 | 22 trait 编译通过 | 待启动 |
| 阶段 6 M2 | 24 维 + 9 子测度真测 | 待启动 |
| 阶段 6 M3 | 5 重守门全绿 = 阶段 6 验收完成 | 待启动 |

---

## 8. 风险与开放问题

1. **trait 数量收敛风险**：43 sketch → 22 互锁 trait 是设计决策，实际阶段 5 实施时若发现某 trait 必须独立，可能需要回退到 ≥22 但仍受 22 互锁矩阵约束
2. **真测方法 vs 单元测试**：V-Measure 的"真测"是端到端 episode replay，不是单元测试，需要 apeireth-memory 的 append-only log 支持
3. **守门冲突仲裁**：5 重守门若同时触发，需 apeireth-council 7 席审议庭仲裁（参见 `crates/apeireth-council/`）

---

## 9. 引用与索引

- 上游：`docs/stage4/architecture-stage4-engineering-landing.md` §3 / §10 / §10.5
- 下游（阶段 6 实施）：`docs/stage6/trait-sketches.rs`
- 配套报告：`reports/round8-02-stage6-trait-interlock-v-measure-design-architect2.md`
- 关联任务：`reports/round7-06-stage6-verification-design-architect.md`
