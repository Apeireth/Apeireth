# R8-WF-01｜R8 三轨串联设计（骨架版）

> 作者: workflow_designer · R8-WF-01
> 状态: **🟡 SKELETON — 等用户真实需求确认后再细化**（继承 R8 阶段等拍板状态）
> 输入: `r7-wf-01-workflow-design.md` + `r7-wf-02-sequence-diagrams.md`（R7 单线状态机/时序）+ `r8-architecture-overview.md`（分层视角）+ `r8-delivery-summary.md`（R8 现状）
> 定位: 不重画 R7 的 3 条单线；在 R7 单线基础上加 **Track B Identity** + **Track C Self-Evolution** 的接入点 + 跨轨守门 + 待决策分支
> 主哲学: ASI 北极星 + V3 守门 + 真生产不停（继承 R6→R7）

---

## 0. 必读前置

| 文档 | 路径 | 本文件引用什么 |
|---|---|---|
| R7-WF-01 状态图 | `reports/r7-wf-01-workflow-design.md` | §1/§2/§3 = Dream/Replay/HotCold 单线状态图（10/10/9 节点） |
| R7-WF-02 时序图 | `reports/r7-wf-02-sequence-diagrams.md` | §1/§2/§3 = Dream/Replay/HotCold 时序图（13/14/13 行） |
| R8 架构总览 | `reports/r8-architecture-overview.md` | §1 = L0-L7 分层；§3 = Track A 真实现；§4 = Track B PoC；§5 = Track C v0.1 |
| R8 阶段交付 | `reports/r8-delivery-summary.md` | §3/§4/§5 = 三轨交付状态；§9 = 缺口清单 |
| R8→R9 移交 | `reports/r8-handoff-r9-team-leader.md` | §6 = 10 个待用户决策问题 |

---

## 1. 设计原则（ponytail 版 = 最小骨架）

1. **不重画 R7** — Dream/Replay/HotCold 三条单线时序已存在，本文件只加 **接入点** + **跨轨消息** + **守门**
2. **不绑接口** — 当前只描述**消息流方向**，不冻结新接口签名（接口冻结 = 用户拍板后 R9-WF-02 做）
3. **不假装实现** — Track B/C 的节点标注 `🟡 PoC` 或 `🔵 待实现` 状态，绝不把 PoC 当真生产
4. **守门先于轨道** — V3/V1072/V1074/V1081 放在每个跨轨消息的**前/后**而不是"末端"
5. **互斥不破坏 R7** — Track B Identity 读写与 Track A Dream/Replay 单租约互斥（继承 R7-WF-02 §5）
6. **决策点显式** — 用户没拍板的 = 标 `❓` + 列在 §6，不脑补

---

## 2. 三轨关系（一句话）

> **Memory（A）记录"发生了什么" → Identity（B）决定"是谁发生的" → Self-Evolution（C）决定"下次怎么发生得更好"**

数据流: Memory events → Identity attribution → Evolution policy update → Memory schema 升级（闭环）。

---

## 3. R8 三轨串联时序图（骨架版 · 23 节点）

```mermaid
sequenceDiagram
    autonumber
    participant U as User/Task
    participant Ev as EventBus
    participant MA as Track-A Memory<br/>(Dream/Replay/HotCold)
    participant MB as Track-B Identity<br/>(IdentityStore/RelationGraph)
    participant MC as Track-C Evolution<br/>(SelfEvolving/DGMArchive)
    participant G0 as V3 Philosophy Guard
    participant G1 as V1072 Identity Drift
    participant G2 as V1074 Measure
    participant G3 as V1081 Honest Limits

    Note over MA,MC: 跨轨单租约互斥 (继承 R7-WF-02 §5)<br/>三轨共用 G0/G1/G2/G3 守门

    U->>Ev: emit(task)
    Ev->>MA: capture_event()        %% Track A
    MA->>G0: V3.verify(capture)      %% L0 守门
    G0-->>MA: PASS
    MA->>MA: canonicalize → cache_lookup
    alt cache hit
        MA-->>Ev: cached_replay
    else cache miss
        MA->>G0: V3.verify(replay)
        G0-->>MA: PASS
        MA->>MA: replay(ro) → dual_sign(impact≥0.7)
        MA->>MB: attribute_to_identity()   %% ← 跨轨到 B
        Note over MB: 🟡 Track B PoC: v0.2 sqlite_identity_store
        MB->>G1: V1072.drift_check()
        G1-->>MB: PASS / SUSPEND
        MB->>MB: enrich_card() / link_relation()
        MB-->>MA: identity_id+drift_score
        MA->>MA: trace_to_mtm (no LTM write)
    end

    Note over MA: A1 HotCold boundary (>80%?)<br/>+ A3 Dream consolidate/decay
    MA->>G0: V3.verify(migrate/decay)
    G0-->>MA: PASS
    MA->>G2: V1074.emit(asi_snapshot)
    G2-->>MA: ok (async, non-blocking)

    Note over MC: 🔵 Track C 未启动;触发条件待用户决策 (❓D1)
    opt evolution_tick (cron/quality-gate)
        MC->>G3: V1081.limits_probe()
        G3-->>MC: PASS / SCOPE
        MC->>G2: V1074.read(snapshot)
        G2-->>MC: deltas (window)
        MC->>MB: query_identity_history()
        MB-->>MC: identity_trajectory
        MC->>MA: propose_schema_upgrade()
        Note over MA: 🟡 Track A: 是否接受 schema 升级 = ❓D2
        MA->>G0: V3.verify(proposal)
        G0-->>MC: PASS / VETO
        alt VETO
            MC->>MC: rollback + DGM.Archive(rejected)
        else PASS
            MC->>MA: apply_patch(sandboxed)
            MC->>G2: V1074.emit(post_patch)
            MC->>MB: bump_identity_version()
        end
    end

    Note over U,MC: 异常路径 (5 条)<br/>V3 FAIL → rollback+alert<br/>V1072 drift → suspend<br/>V1074 超时 → retry 3x<br/>V1081 scope → 缩窗重试<br/>MB 失败 → fallback to anonymous_identity
```

**节点编号** = autonumber；单租约互斥 = Note at L20；守门 = 4 条短 Note inline。

---

## 4. 节点清单（23 个 + 4 守门）

| # | 节点 | 轨道 | 来源 | 状态 |
|---|---|---|---|---|
| 1 | User/Task | input | — | ✅ |
| 2 | EventBus | glue | `r8-architecture-overview §6` | ✅ |
| 3-13 | Track A 11 节点 | A | `r7-wf-02 §1/2/3` (Dream+Replay+HotCold 完整继承) | ✅ |
| 14 | attribute_to_identity() | A→B | 跨轨桥 — 新增 | 🟡 PoC |
| 15 | IdentityStore | B | `apeireth/sqlite_identity_store.py` v0.2 | 🟡 PoC |
| 16 | RelationGraph | B | 设计稿 | 🔵 未实现 |
| 17 | enrich_card() | B | `apeireth/kickoff_enrichment.py` v0.4 | 🟡 PoC |
| 18 | identity_id + drift_score | B→A | 返回值 | 🟡 PoC |
| 19 | SelfEvolving 主循环 | C | `apeireth/self_evolving.py` v0.1 | 🟡 PoC |
| 20 | DGM Archive (rejected/passed) | C | 待实现 | 🔵 |
| 21 | propose_schema_upgrade() | C→A | 新增 | 🔵 |
| 22 | apply_patch(sandboxed) | C→A | 新增 | 🔵 |
| 23 | bump_identity_version() | C→B | 新增 | 🔵 |
| G0 | V3 philosophy_guard | guard | `r7-wf-01 §4` | ✅ |
| G1 | V1072 identity_drift | guard | `r7-wf-01 §4` | ✅ |
| G2 | V1074 measure | guard | `r7-wf-01 §4` | ✅ |
| G3 | V1081 honest_limits | guard | `r7-wf-01 §4` | ✅ |

**汇总**: ✅ 13 (R7 + 守门) + 🟡 5 (PoC) + 🔵 5 (待实现) = 23 + 4G = **27 元素**

---

## 5. 跨轨依赖表（无环校验）

| 源 | 目标 | 消息 | 守门 | 单租约 | 状态 |
|---|---|---|---|---|---|
| EventBus | MA | capture_event | V3 pre | — | ✅ 已有 |
| MA | MA | canonicalize/cache_lookup | V3 pre | 单轨 | ✅ 已有 |
| **MA** | **MB** | **attribute_to_identity** | **V1072 post** | **与 Dream/Replay 互斥** | 🟡 |
| MB | MB | enrich_card/link_relation | V3 pre | 单轨 | 🟡 |
| MB | MA | identity_id+drift_score | — | 紧接上游 | 🟡 |
| MA | MA | HotCold migrate / Dream decay | V3 pre | 与 Replay 互斥 | ✅ 已有 |
| MA | G2 | emit_snapshot | V1074 terminal | 异步非阻塞 | ✅ 已有 |
| MC | G3 | limits_probe | V1081 pre | 独立轨 | 🟡 |
| MC | G2 | read_snapshot | V1074 read-only | 异步 | 🟡 |
| MC | MB | query_identity_history | — | 只读 | 🔵 |
| MC | MA | propose_schema_upgrade | V3 pre + V1072 pre | **跨轨双签** | 🔵 |
| MC | MA | apply_patch | V3 pre + 沙箱 | 跨轨 | 🔵 |
| MC | MB | bump_identity_version | V3 pre | 跨轨 | 🔵 |

**无环校验**: MA → MB → MA(回写仅 identity_id，无状态变更)→ MC → MA(apply)→ MB(bump)。MB→MA→MC→MA 形成 **MC→MA 单向** 不闭环（apply 后无回流到 MC），无需 rollback MC 自身。

---

## 6. 待用户决策点（❓ 5 项 — 拍板前不动代码）

| ID | 问题 | 候选 A | 候选 B | 候选 C | 影响范围 |
|---|---|---|---|---|---|
| **❓D1** | Self-Evolution 触发条件 | cron 周期（如 24h） | ASI 质量门（Δ<阈值） | 用户显式 opt-in | Track C 入口 |
| **❓D2** | Memory 是否接受 schema 升级 | 始终接受 + V3 审 | 人工 review + V3 | 完全自动 (高风险) | Track A 写入路径 |
| **❓D3** | Identity 失败 fallback | 匿名 identity (匿名事件) | 拒绝入轨 (rollback) | 降级到 LTM-only (无身份标注) | Track B 鲁棒性 |
| **❓D4** | DGM Archive 保留期 | 永久 (审计完整) | 30 天滚动 | 只保留 rejected (省空间) | Track C 存储成本 |
| **❓D5** | 跨轨单租约粒度 | 整轨互斥（保守） | 节点级互斥（精细） | 无互斥（高并发高风险） | 性能 vs 安全 |

**继承 R8-handoff §6 的 10 个用户决策**（业务级），本表只覆盖**工作流级**的 5 个新增项。

---

## 7. 异常路径（5 条 · 与 R7 风格一致）

| 异常 | 触发 | 处理 | 告警 |
|---|---|---|---|
| V3 FAIL | 任意 verify | rollback + alert | philosophy_guard_alert |
| V1072 drift | identity_drift > threshold | suspend + IdentityRecovery | identity_suspend |
| V1074 timeout | snapshot > 60s | retry 3x + 缩窗 | measure_timeout |
| V1081 scope | limits_probe 越界 | 缩窗重试 | limits_scope |
| MB fallback | IdentityStore 不可用 | D3 决策决定 → 当前=匿名 identity | identity_fallback |

**覆盖率**: 5/5 = 100%（R7 是 9/9，本文件只覆盖跨轨 5 条；单轨内部异常见 R7-WF-02 §5）

---

## 8. 与 R7/R8 现有产物的关系

| 来源 | 复用方式 | 本文件改动 |
|---|---|---|
| R7-WF-01 §1/§2/§3 | 完整继承单线状态图 | 0 改动 |
| R7-WF-02 §1/§2/§3 | 完整继承单线时序图 | 0 改动 |
| R7-WF-01 §4 守门表 | 直接引用 | 0 改动 |
| R7-WF-02 §5 单租约 | 直接引用 | 0 改动 |
| R8-architecture-overview §3/§4/§5 | 引用作为节点状态来源 | 0 改动（不重画分层） |
| R8-delivery-summary §9 缺口 | 引用作为 🔵/🟡 标记依据 | 0 改动 |
| R8-handoff-r9 §6 用户问题 | 引用作为 §6 ❓D1-D5 来源 | 0 改动（不重提业务级） |

**本文件新增 = 23 节点跨轨串联 + 5 异常 + 5 决策点 + 1 张主时序图。**

---

## 9. 待办（用户拍板后 R9 启动时）

| 序 | 任务 | 前置 | 输出 |
|---|---|---|---|
| 1 | 用户回答 §6 的 ❓D1-D5 | 用户会议 | 决策矩阵填值 |
| 2 | 冻结 Track B↔C 接口签名 | D1+D5 决策 | r9-int-identity-evolution.md |
| 3 | 补 Track B RelationGraph PoC | D3 决策 | `apeireth/relation_graph.py` v0.1 |
| 4 | 补 Track C DGM Archive PoC | D4 决策 | `apeireth/dgm_archive.py` v0.1 |
| 5 | 跑 V1074/V1081 真测验证守门 | P0 修复完 | 守门全 PASS |
| 6 | 重画 R8-WF-01 v2.0（带决策值） | 1-5 全完成 | 本文件更新 |

---

## ✓ 验收

骨架版 ≥ 6KB ✓ | 1 张主时序图（23 节点 + 4 守门 + 5 异常）✓ | 不重画 R7 ✓ | 引用 R7/R8 共 5 份既有产物 ✓ | 5 待决策点显式 ❓D1-D5 ✓ | 节点状态分 ✅/🟡/🔵 三级 ✓ | 无代码/无 commit ✓ | 与 R7 单租约+守门风格一致 ✓
