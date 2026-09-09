# 三段式协调遗忘：P0-A 生产接线设计 (2026-09-08)

> 依据：`_research_mem/ra/ra14` §1 + `ra15` P0-A；吸收自 arXiv:2609.04875
> （execution-state unlearning）。状态：设计定案，实现见
> `crates/engine/memory/src/forget_coordinator.rs`（opt-in，默认关闭）。

## 1. 问题与目标

现状（0 装）：
- `forget_episode` 只软删根 episode（governance sidecar）；
- `research_forget_closure` / `research_execution_state_inventory` 只审计不删除；
- `GovernedRecall` 的遗忘集是**调用方显式传入**的，进程重启即失。

目标（对手语义的工程版）：**人工批准后，明文 + 派生面 + 执行态一次性清**，
且**遗忘后行为等价于从未观察**（回归断言，不复刻证明）。

## 2. 三段式语义

| 阶段 | 内容 | 实现 |
|---|---|---|
| P1 明文 | 根与闭包中 store 拥有的明文面 | episode → 既有 `forget_episode`；所有闭包节点 → 新持久化遗忘集（V10 表） |
| P2 派生面 | diary/wiki/chronicle/cache 等派生 | 持久化遗忘集标记 + 缓存代际联动（调用方经既有 `research_invalidate_cache_on_forget` 推进）+ 可选 `RepairExecutor` 修复计划（P0-B 集成） |
| P3 执行态 | 会话 τ 后缀 / 挂起计划 / 在途 summary / 缓存片段 | 删除 `research_execution_state` 注册行，并把被删条目**报告给调用方**清 runtime 活体工件（runtime 侧清理留部署层） |

## 3. 关键设计决策

1. **审批工件引用而非内建审批流**：`CoordinatedForget.approval_id` 由上游
   approval 生命周期解析后传入；协调器只执行 + 把 approval_id 写进审计链。
   拒绝方案：在 memory crate 内建第二审批权威——违反
   "无第二审批权威"架构不变量（canonical_architecture_invariants）。
2. **持久化遗忘集（V10 `research_forgotten_artifacts`）**：治理语义的遗忘集
   从"调用方内存集"升级为"store 内持久事实"。`GovernedRecall` 增加
   `from_store_persisted` 构造器；旧 `with_filter` 保留（等价性门）。
3. **执行态删除边界**：store 只删自己的注册表行；runtime 活体工件
   （工具计划/摘要/缓存片段）由报告清单驱动调用方清理——store 不越权。
4. **行为契约断言内建**：协调器执行后自验"从未观察"三条件
   （执行态清单空 + 根被召回排除 + 持久遗忘集覆盖闭包），`verified` 字段
   如实返回，不假装。

## 4. 边界（0 装）

- 旧 `forget_episode` 语义不变（等价性门）；协调器为 opt-in 显式调用。
- 派生内容的**实际重算**（LLM/规则）是部署层回调——store 只管标记与清标记
  （与 `derived_repair::planner_only` 口径一致）。
- 缓存代际推进由调用方执行（store 不持有 GenerationCache）。

## 5. 验收

- `coordinated_forget` 三段式回归：根召回排除 + V10 标记 + 执行态注册行清空 +
  审计事件 + `verified=true`。
- 持久化召回：`GovernedRecall::from_store_persisted` 过滤派生项。
- P0-B 集成：`research_apply_repair_outcome` 标记/清标记。
- 5 重守门全绿。
