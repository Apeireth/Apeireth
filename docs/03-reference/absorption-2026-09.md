# 吸收对照登记：2026 追兵论文工程方案 → Apeireth 落地 (2026-09)

> 依据：`_research_mem/ra/ra14-engineering-absorption-report.md` + `ra15-engineering-dispatch-package.md`。
> 归属纪律（O-2 锚）：吸收的代码在注释中标注来源论文与 arXiv 号；本文是逐行对照登记。
> 状态约定：✅ 已落地（默认关闭/并列，不替换生产路径）｜📋 仅登记（后置路线）。

---

## P0-C · betti_hole_detector 4-环过滤 bug 修复（自研 bug，非吸收）

- 文件：`crates/engine/memory/src/betti_hole_detector.rs`
- 修复内容（2026-09-06）：
  1. `find_candidate_cycles` 拆分为 3-环/4-环双阈值；4-环候选加 **尺度上界** `four_cycle_epsilon · max_dist`（默认 0.8，clamp [0.05,1.0]，ε=1 显式选择旧行为）。
  2. 4-环候选加 **出生存活条件**：两条对角线都必须长于最长边界边（否则该环在出生尺度已被填充，不可能生成持续 β₁）。
  3. **退化守卫**：`max_dist ≤ 1e-6`（全同/近全同嵌入）直接跳过环搜索，返回 β₀ 报告。
  4. **持久化地板移除**：`compute_cycle_persistence` 删除 `max(birth+0.1)`（4+ 环）与 `max(0.15)`（3 环）地板——地板使任何 `min_persistence_threshold ≤ 0.1` 恒通过，是"threshold 恒真"的根因。现在死亡尺度 = 真实几何量（对角/面积项），寿命 ≤ 0 即过滤。
- 验收：`--max-nodes 150` 完成（确定性 150 节点 8D 云回归测试 `n150_deterministic_cloud_completes`；修复前该场景物化全部 C(150,4)≈2.03×10⁷ 环 OOM）。memory crate 666→680 tests 全绿。
- **学术线须知（B1 漂移基线）**：
  1. B1 的 vendor 副本需同步本修复，漂移基线随持久化地板移除而变（死亡尺度语义变化，三角形/方形检测测试仍绿）。
  2. 残留事实（0 装）：阈值 0.05 低于高维噪声地板时，随机云仍产出 ~3% of C(n,4) 的 4-环（实测 619,707 @ n=150）与大量 3-环——建议 B1 按噪声地板调阈值或 top-k 策展。3-环刻意不受 ε 界（C(n,3) 可控 + 三角检测语义基线锁定）。

## P0-A · 执行态遗忘闭包（吸收自 arXiv:2609.04875 "execution-state unlearning"）

- 论文原文要点：forget 只删明文记录即停；形式化"遗忘后行为=从未观察目标"；pre-target 前缀免费共享、post-target 后缀不可约污染（精确去学习 ≥ T−τ+1 次重算转移）；Provenance-Guided Selective Replay = 血缘定位注入点 + KV 裁剪 + 消毒重放。
- 落地（`crates/engine/memory/src/research_derived_memory.rs` + `migrations.rs` V9）：
  - `ExecutionStateKind`（SessionTokenSpan / PendingToolPlan / InFlightSummary / PromptCacheFragment）——外部 API 无 KV 句柄，会话 KV 段以"注入步 τ 之后的后缀"登记为等价物（0 装口径写在 `ExecutionStateInventory.unobservable_note`）。
  - `research_record_execution_state`（幂等 UPSERT 进新表 `research_execution_state`，V9 migration，append-only 注册）。
  - `research_forget_closure` 输出增加 `execution_state` 节；`research_audit_forgotten_leaks` 输出增加"执行态泄漏"节（仍只报告不删除）。
  - `execution_state_prefix_crop(τ, end)`：论文前缀等价结论的纯函数工程版（τ=0 ⇒ 整段污染）。
- 验收（回归测试）：`p0a_execution_state_inventory_lists_tainted_items` / `p0a_prefix_crop_matches_paper_semantics` / `p0a_forget_then_crop_behaves_as_never_observed`（遗忘 + 裁剪前缀 + 血缘过滤召回三者组合 = 从未观察）+ `v9_migration_applied_on_fresh_db`。
- 未做（诚实）：生产 `forget_episode` 的"明文+派生面+执行态一次性清"升级留待人类批准流程设计后接入（ra14 的"将来升级版"口径）；runtime 侧会话跨度的自动登记留部署层。

## P0-B · 屏障优先级联修复执行器（吸收自 arXiv:2605.07242 MEMOREPAIR）

- 论文原文要点：修复事件 = 从失效后代状态到验证后继状态的受控迁移；受影响后代先撤回 → 用保留支撑+已修复前驱重建 → 仅放行前驱闭包已验证的后继；修复选择 = 最大权前驱闭包，单次 s-t 最小割精确解；ToolBench/MemoryArena 上失效暴露 69.8–94.3%→0%，省 24–43% 算子成本。
- 落地（`crates/engine/memory/src/derived_repair.rs`，新文件）：
  - `RepairExecutor::affected_closure`（taint 同语义 BFS）。
  - `RepairExecutor::select_repair_set`：最大权前驱闭包 → s-t 最小割；**自写 Dinic**（i64 整数化容量保证确定性，0 新外部依赖，Cargo.lock 0 行 diff）。
  - `RepairExecutor::execute`：屏障三段（撤回全部受影响 → 拓扑序阶段重建 → 验证"父支撑 ∈ 保留 ∪ 已修复 ∪ 根[上游已更正]"→ 仅放行通过者）；根不重建（内容由上游人工更正，0 装）；`planner_only = true` 显式标注持久化留部署层。
- 验收（回归测试）：级联闭包 + 前驱闭合选择、最小割最优性小实例（负利润独立节点排除 / 闭包连带负利润前驱）、屏障执行三段账本、放行顺序前驱先行、根撤回不重建。
- 与学术线论文关系：审计是修复的前置层，本执行器是其下游（ra14 §2 口径）。

## P1-A · 记忆准入控制（吸收自 arXiv:2603.04549 A-MAC）

- 论文原文要点：准入 = 结构化决策；价值分解为五因子（未来效用、事实置信、语义新颖、时间新近、内容类型先验）；规则特征 + 单次 LLM 效用评估；内容类型先验是消融里最可靠因子。
- 落地（`crates/engine/memory/src/admission_gate.rs`，新文件）：
  - `ResearchAdmissionGate::adjudicate`：五因子加权 + 冲突惩罚；类型先验复用既有 `bitemporal_graph::TrustWeights`（RA-2 §5.2 w 表，`w()` 改 pub）；类型先验 × 置信按 A-MAC 结论相乘。
  - 决定 `Admit / PendingReview` 带分因子 `breakdown`（可解释审计）。
  - `record_decision` 写 append-only `research_lineage_events`（审计链；不幂等覆盖，0 装标注）。
  - LLM 效用评估留 `trait` 口（`ConflictScorer` 供冲突度；效用由调用方供给）；`KeywordConflictScorer` 确定性 stub。
- 与 StackPin 分层：准入管"该不该记"，StackPin 管"记了之后带哪些进考场"（P1 论文 related work 差异点）。
- 未做（诚实）：未挂生产 `put_note`/提取写入路径（默认关闭）；策略参数未做 A-MAC 式交叉验证优化（需要 B1 数据）。

## P1-B · 统一内存成本账本（吸收自 arXiv:2607.08032 rate-distortion 统一）

- 论文原文要点：KV 逐出/提示压缩/架构状态/智能体记忆是同一个 rate-distortion 决策；单一压缩目标 + 层不可知下界 + 七轴分类；每层信号以同样方式失败（查询未知前丢弃、不可撤销）。
- 落地（`crates/foundation/orchestration/src/research_cost_ledger.rs`，新文件，Research 前缀默认关闭）：
  - 四层（Context / Summary / Cache / Vault）× 成本三元组（tokens, latency_ms, distortion）统一记账。
  - `utility_cost_curve`（按预算档聚合，面板"8k→2k 损失多少效用省多少成本"后端）+ `marginal_distortion_per_token`（每省 1 token 的失真代价）+ `recommend_compaction_order`（rate-distortion 贪心，无法估计的层排最后）。
  - 适配器：`record_context_rot`（腐烂分均值→失真）、`record_prompt_cache`（1−命中率）、`record_vault_retention`（1−保留命中率）、`record_summary`（调用方给失真）。
- 未做（诚实）：KV 层本身不在我们手里（外部 API），KV 三篇吸收挂 P2 本地推理路线。

## P2 · KV 层三篇（仅登记，后置）

| arXiv | 标题要点 | 状态 |
|---|---|---|
| 2607.10582 | MemDecay 区域感知逐出 | 📋 登记，本地推理（便携 U 盘 SLM）路线启动后吸收 |
| 2608.00528 | S4R 采样+子空间+稀疏重建 | 📋 同上 |
| 2601.18999 | 随机化逐出 + 学习路由 | 📋 同上 |

---

_本文由工程线 2026-09-06 写入；对手论文原文由工程线自行从 arXiv 下载精读（摘要存档于 %TEMP%\apx-arxiv，不入库）。学术线引用本文时，P0/P1 各项状态以"✅ 已落地"为准，可在论文 related work 写"已在工程侧实现"。_
