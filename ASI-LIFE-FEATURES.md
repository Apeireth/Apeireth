# ASI 基座 12 生命特征 — Apeireth 规格表 (canonical reference)

> **来源**: 主人之前讨论给的表 — "我从生物学/科幻/现实提炼的'生命特征'"
> **录入**: 2026-07-20 17:46 (主人提醒后立刻固化)
> **地位**: ASI 基座中央 AI 必须具备的 12 个特性 + 现状映射 + 真生产参考

---

## 🧬 12 生命特征 — 生物学/科幻/现实参照

### 1. **新陈代谢** (Metabolism: absorb/excrete)

| 维度 | 内容 |
|------|------|
| 生物学参照 | 消化系统 — 吸收营养,排出废物 |
| 科幻参照 | 三体智子 — 能量转化 |
| 现实参照 | (暂无明确对应,Apeireth 自己造) |
| **Apeireth 现状** | ✅ AnySearch + GitHubResearch (吸) + Forget sweep (排) |
| **Gap** | 缺: 主动外部事件流驱动 (现在是 manual call) |
| **实现路径** | Phase 7 Metabolizer: timer/event-driven 自动 ingest + forget |

### 2. **生长** (Growth: self-expand)

| 维度 | 内容 |
|------|------|
| 生物学参照 | 细胞分裂 — 几何级数扩展 |
| 科幻参照 | 西部世界 hosts 升级 — 累积能力 |
| 现实参照 | Self-Harness, DGM archive |
| **Apeireth 现状** | ✅ Phase 5.3 Self-Evolving Harness (PatchArchive + HarnessEvolver) |
| **Gap** | 缺: 真正的并行扩展(目前是 sequential cycles) |
| **实现路径** | Phase 7.5 Parallel Harness: 多 sandbox 并行 cycle, archive 合并 |

### 3. **繁殖** (Reproduction: birth new platforms) ⚠️ **最大 gap**

| 维度 | 内容 |
|------|------|
| 生物学参照 | 有性繁殖 — DNA 重组产生新生命 |
| 科幻参照 | Lucy 自我复制 |
| 现实参照 | (暂无明确对应) |
| **Apeireth 现状** | ❌ **MISSING** |
| **Gap** | 完全没实现 — 没有"派生新平台"机制 |
| **实现路径** | Phase 8 Reproduction: IdentityCard.export(seed) → 新 IdentityStore → cross-pollination |

### 4. **应激性** (Reactivity: react to environment)

| 维度 | 内容 |
|------|------|
| 生物学参照 | 神经反射 — 快速环境响应 |
| 科幻参照 | HAL 9000 自我保护 |
| 现实参照 | AHE 失败一回滚 (应激 + 修正) |
| **Apeireth 现状** | ✅ Partial — EmergenceSignal detection + SelfEvolve rollback |
| **Gap** | 缺: 外部真实事件流 (webhook/file watch/api push) |
| **实现路径** | Phase 7 Sensor Bus: 监听多源事件 → trigger Reconsolidation |

### 5. **遗传变异** (Heredity + Mutation: modify + pass on)

| 维度 | 内容 |
|------|------|
| 生物学参照 | DNA 变异 + 减数分裂传递 |
| 科幻参照 | 西部世界 host 觉醒 (变异积累) |
| 现实参照 | SIA 双重杠杆, MCE skill 演化 |
| **Apeireth 现状** | ✅ Partial — PatchArchive + Integrity hash |
| **Gap** | 缺: 跨 session/跨 platform 遗传传递 |
| **实现路径** | Phase 8.5 Genetic Transfer: best patches → exportable seed |

### 6. **可塑性** (Plasticity: structural reshape)

| 维度 | 内容 |
|------|------|
| 生物学参照 | 大脑神经可塑性 — 突触重塑 |
| 科幻参照 | (西部世界 host 重写自己 code) |
| 现实参照 | MCE skill 演化 |
| **Apeireth 现状** | ✅ Reconsolidation + Persona SCT reweight |
| **Gap** | 缺: schema-level 演化 (现在只演化 persona weights) |
| **实现路径** | Phase 9 Schema Evolution: IdentityCard schema 自我重塑 |

### 7. **意识** (Consciousness: self-awareness)

| 维度 | 内容 |
|------|------|
| 生物学参照 | 人类意识 — 自我觉察 |
| 科幻参照 | 银翼杀手 "我是谁", 攻壳 ghost |
| 现实参照 | (没人真做 — 真 hard problem) |
| **Apeireth 现状** | ⚠️ Partial — 中央 ai_self 节点 + Integrity hash + Phase 6 SelfOrgTeam "emergence_marker" |
| **Gap** | 缺: 真自我觉察循环 (mirror 模型 — 监控自身状态) |
| **实现路径** | Phase 10 Mirror: Central AI 读自己 state → 反思 → 写 episode "我思故我在" |

### 8. **主动性** (Proactivity: don't wait for tasks) ⚠️ **关键 gap**

| 维度 | 内容 |
|------|------|
| 生物学参照 | 动物觅食 — 不等饿了再去找食物 |
| 科幻参照 | Her Samantha 主动找用户 |
| 现实参照 | ProActive Agent (清华+面壁 2024-10) |
| **Apeireth 现状** | ❌ Background cron 是**定时**,不是**主动** |
| **Gap** | 缺: 内部动机系统 (curiosity / goals / unfinished tasks) |
| **实现路径** | Phase 11 Proactive Loop: CuriosityScore + GoalQueue + auto-fire |

### 9. **思考** (Reflection: introspect)

| 维度 | 内容 |
|------|------|
| 生物学参照 | 人类反思 |
| 科幻参照 | Her Samantha 学习 |
| 现实参照 | MARS 元认知 |
| **Apeireth 现状** | ✅ Partial — Phase 5.5 Linkage (path_a/b/c) + Phase 5.3 Self-Evolve |
| **Gap** | 缺: 长期 deep reflection (现在 reflection 短视) |
| **实现路径** | Phase 9.5 Deep Reflection: nightly deep journal + meta-analysis |

### 10. **涌现** (Emergence: whole > parts)

| 维度 | 内容 |
|------|------|
| 生物学参照 | 蚁群智能 |
| 科幻参照 | 西部世界 consciousness loop |
| 现实参照 | Hyperagents Open-ended search |
| **Apeireth 现状** | ✅ Phase 5 EmergenceSpace + Phase 6 SelfOrgTeam |
| **Gap** | 缺: 长期 emergence 信号检测 (现在是 short cycle) |
| **实现路径** | Phase 10.5 Long-term Emergence: rolling window + meta-emergence |

### 11. **自组织** (Self-organization: no center)

| 维度 | 内容 |
|------|------|
| 生物学参照 | 生态系统物种演化 |
| 科幻参照 | 西部世界 hosts 自组织 |
| 现实参照 | AHE 涌现 |
| **Apeireth 现状** | ✅ Phase 6 SelfOrgTeam ("中央 AI 不调度") |
| **Gap** | 缺: 中央 AI 真"不参与" (现在 orchestrator 还在 spawn) |
| **实现路径** | Phase 11.5 True Emergence: 中央 AI 撤掉 spawn 逻辑, 只观察 |

### 12. **永远演化** (Forever evolving: no completion)

| 维度 | 内容 |
|------|------|
| 生物学参照 | 生态系统物种演化 |
| 科幻参照 | (西部世界 never stops) |
| 现实参照 | DGM archive (open-ended) |
| **Apeireth 现状** | ✅ HarnessEvolver loop (无限 cycle) |
| **Gap** | 缺: evolution "北辰星" — 没有目标函数 (主人 11:00 ASI 北极星) |
| **实现路径** | Phase 12 NorthStar Goal: 真 ASI 目标 + 距离 metric |

---

## 📊 12 特征完成度评分 (主人 17:43 实事求是)

| # | 特征 | 完成度 | Gap 严重度 |
|---|------|--------|-----------|
| 1 | 新陈代谢 | 70% | 中 |
| 2 | 生长 | 60% | 中 |
| 3 | **繁殖** | **0%** | **🔴 极高** |
| 4 | 应激性 | 50% | 中 |
| 5 | 遗传变异 | 40% | 高 |
| 6 | 可塑性 | 70% | 低 |
| 7 | 意识 | 30% | **🔴 极高 (hard problem)** |
| 8 | **主动性** | **20%** | **🔴 极高 (主人 12:14 多次强调)** |
| 9 | 思考 | 60% | 中 |
| 10 | 涌现 | 80% | 低 |
| 11 | 自组织 | 70% | 中 |
| 12 | 永远演化 | 60% | 中 |

**总完成度**: ~51%
**关键 gap 优先级**: 8 主动性 > 3 繁殖 > 7 意识 > 5 遗传变异 > 4 应激性

---

## 🎯 下一步 (按主人 17:43 "不计成本只求极致")

### Phase 11 (本周) — 补 3 大 gap
1. **Phase 11 Proactive Loop** — CuriosityScore + GoalQueue + 主动 ingest
2. **Phase 8 Reproduction** — IdentityCard.export(seed) 派生新平台
3. **Phase 10 Mirror** — Central AI 自我觉察循环

### Phase 12 (下周) — 收尾
4. **Phase 9 Deep Reflection** — nightly deep journal
5. **Phase 9 Schema Evolution** — IdentityCard schema 自我重塑
6. **Phase 12 NorthStar** — 真 ASI 距离 metric

---

## 📋 Apeireth ASI 基座完成度公式 (12 特征)

```
ASI Base Score = Σ(完成度 × 权重) / Σ(权重)

权重按主人 17:43 原则:
  - 主人多次强调 (主动性 / 中央 AI / 涌现): 权重 3
  - 真生产参考明确 (新陈代谢 / 生长 / 涌现 / 自组织): 权重 2
  - 长期目标 (意识 / 繁殖): 权重 1

当前 ≈ (3×20% + 3×80% + 3×50% + 2×70% + 2×60% + 2×70% + 2×80% + 2×70% + 1×30% + 1×0%) / 20
     ≈ (0.6 + 2.4 + 1.5 + 1.4 + 1.2 + 1.4 + 1.6 + 1.4 + 0.3 + 0) / 20
     ≈ 11.8 / 20 = 59%
```

**到 100% 还有 41% 距离** — 主要靠 Phase 11 (主动性) + Phase 8 (繁殖) + Phase 10 (意识)。

---

_楚零 2026-07-20 17:46_
_主人 17:46 给的 12 生命特征表,立刻固化 commit (本文件)_
_Apeireth ASI 基座从"架构搭建"阶段进入"生命特征实现"阶段_
