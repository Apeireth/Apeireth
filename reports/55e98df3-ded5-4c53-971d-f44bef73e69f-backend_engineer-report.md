# TP32 / W2 + W3 世界模型第二层：因果结构图推演 — 验收报告

**任务**: `55e98df3-ded5-4c53-971d-f44bef73e69f` (TP32, W2 + W3, 发布前置 P0)
**角色**: 后端工程师
**分支**: `task/tp12-schema-guardrail-rework-final`
**提交** (rebase 后):
- `680d2838` feat(companion): TP32 世界模型第二层 因果结构图推演 (W2+W3, 发布前置 P0)
- `f06dae9e` docs(backlog): W2 因果图 + W3 边挖掘完成登记 (提交 4111d86)
- `8864588` docs(backlog): 提交 hash 引用更新 (rebase 后 → 680d2838)

> 注: 原 hash `4111d86` (impl) + `39ed6ca` (docs) 在 rebase 到 integration HEAD `d8f971ad` 后变更为 `680d2838` (impl) + `f06dae9e` (docs). 内容零变更, 仅 hash 因 rebase 改变. 台账已同步更新 (`8864588`).

---

## 1. 交付物清单

| 项 | 路径 / 类型 | 状态 |
|---|---|---|
| `causal_world_model` 模块 | `crates/apeireth-companion/src/causal_world_model.rs` | 新增 (~1080 行, 含 doc + 7 测试) |
| 模块挂载 | `crates/apeireth-companion/src/lib.rs` | `pub mod causal_world_model;` |
| 台账 W2 修复 | `docs/backlog.md` 行 102 / 289 | 撤销错误 ✅ → ✅ TP32 完成 + 提交 680d2838 |
| 台账 W3 登记 | `docs/backlog.md` 行 381 | ✅ TP32 完成 (随 W2) + W3 三态 (Statistical/LlmProposed/Hybrid) + 提交 680d2838 |

## 2. 核心结构 (建议起点, 已落地)

### 2.1 W2 数据层 + 编排

| 类型 | 职责 |
|---|---|
| `CausalNode { id, fact }` | 因果节点 (`id = GraphFact.chain`, Zep 双时态语义: 同 s\|p\|o 共一节点) |
| `CausalEdge { id, from, to, predicate, weight, evidence_count, source }` | 因果边; `source: EdgeSource` 三态 |
| `EdgeSource { Statistical, LlmProposed, Hybrid }` | 边来源 (W3 主路径 = Statistical, 补充 = LlmProposed) |
| `CausalGraph` | 节点 + 边 + 邻接表 (outgoing/incoming); MCTS 搜索空间 |
| `CausalStep { tick, from_node, edge, to_node, narrative, state_snapshot }` | 沿一条因果边走一步 |
| `CausalChain { hypothesis, steps, terminal_node, terminal_forecast, calibration_brier, rejected, reject_reason }` | 一条完整因果推演链 |
| `CausalSimulator` | 编排器 (run + reconcile_with_fact), 与 `TextualSimulator` 同构 |
| `CausalBranchContext { current_node_id, current_state, hypothesis, visited, candidates }` | 分支点判断上下文 |
| `EdgeJudgment { edge_id, take, narrative, goal_progress }` | LLM 分支点判断产物 |

### 2.2 W3 边挖掘

| 类型 | 职责 |
|---|---|
| `MineCausalEdges::from_timeline(facts, min_evidence=7)` | **W3 主路径**: 时间线统计挖掘 (首匹配计数法 — 对每个 fi, 找首个匹配的 fj 即计 1 对, 避免"一因多果"重复计数, 与"熬夜→次日效率低出现 7 次即统计边"直觉一致) |
| `CausalLlm` trait | **W3 补充路径** LLM 抽象 (`judge_branch` + `propose_edges`); EvoCause 式提议 |
| `EdgeProposalRequest { facts, max_proposals }` | LLM 提议请求 |
| `EdgeProposalResponse { proposals }` | LLM 提议响应 |
| `ProposeCausalEdges::llm_suggest(req)` | 调 LLM 提议边 (source = LlmProposed) |
| `MockCausalLlm { take_first, max_proposals }` | 测试用 mock |

### 2.3 MCTS 接线 (复用 TP7 cognition planning)

| 类型 | 职责 |
|---|---|
| `CausalMctsState { node_id, world_state }` | MCTS 状态 (SearchState blanket impl 适用) |
| `CausalMctsAction { edge, to_node }` | MCTS 动作 (apply: 边 from==当前节点 → 前进; 否则 None) |
| `CausalGraphEvaluator { llm, hypothesis, goal_node_id }` | StateEvaluator (0 装 PASS: 用 node_id 距离启发式; 真接入时桥接 async LLM 留升级点) |
| `CausalMctsPlanner::search(start, evaluator, seed)` | 包装 MctsPlanner, 返回 (最优首动作, 根访问数) |

### 2.4 Brier 对账 (复用 W1 oracle::Forecast 模式)

- `reconcile_with_fact(chain, actual)` → resolve forecast + 计算 Brier + 阈值拒绝
- 阈值默认 0.3, 可调 (`with_threshold`); 可选注入 `CalibratedResolver` (历史 mean_brier 拒绝)

## 3. 验收测试结果

### 3.1 `cargo test -p apeireth-companion --lib causal_world_model`

**lib 绿** ✅ (7/7 全绿, 测试通过率 100%):

```
running 7 tests
test causal_world_model::tests::mcts_on_causal_graph_runs ... ok
test causal_world_model::tests::propose_causal_edges_llm_suggest ... ok
test causal_world_model::tests::causal_chain_expand_from_root ... ok
test causal_world_model::tests::causal_chain_reconcile_with_fact ... ok
test causal_world_model::tests::mine_causal_edges_below_threshold_no_edge ... ok
test causal_world_model::tests::mine_causal_edges_statistical ... ok
test causal_world_model::tests::causal_world_model_does_not_persist_to_memory ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 515 filtered out
```

| 验收点 | 测试函数 | 断言 |
|---|---|---|
| ① 因果链展开 (W2) | `causal_chain_expand_from_root` | 3 节点链, mock take_first 沿第一条出边走 ≥1 步 ≤2 步; from_node/to_node 正确; terminal_forecast 存在; 概率 = 边权重均值 (0.9+0.8)/2 = 0.85 |
| ② MCTS 在因果图上跑通 | `mcts_on_causal_graph_runs` | 50 次迭代 + seed=42, 返回 best_action.edge.from = 起点 chain; root_visits > 0 且 ≤ 50 |
| ③ 统计挖掘 (W3 主路径) | `mine_causal_edges_statistical` | 7 对 (熬夜→效率低) → 共现 7 次即边 (主人拍板阈值); 边 source=Statistical; 谓词 "行为→导致"; weight ∈ (0,1] |
| ③ 阈值下拒绝 | `mine_causal_edges_below_threshold_no_edge` | 3 对 < 阈值 7 → 0 条边; candidate_pairs=3 (如实报告机制层) |
| ④ LLM 提议边 (W3 补充) | `propose_causal_edges_llm_suggest` | mock 派生 (主人→熬夜→效率低) 1 条; source=LlmProposed; ≤ max_proposals 上限 |
| ⑤ 对账 Brier | `causal_chain_reconcile_with_fact` | p=0.85/true → Brier=0.0225 < 0.3 不拒绝; p=0.85/false → Brier=0.7225 > 0.3 rejected=true, reason 含 "Brier"+"0.3" |
| ⑥ 0 装 PASS 边界 | `causal_world_model_does_not_persist_to_memory` | run + reconcile 后 in-memory store 写入数 = 0 |

### 3.2 `cargo test -p apeireth-companion --lib` (全 lib 回归)

**lib 绿** ✅ (522/522 全绿, 含 7 个新测试):

```
test result: ok. 522 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.05s
```

(基线 515 → +7 = 522, 与新增数对齐)

### 3.3 `cargo check --workspace --all-targets`

**all-targets 绿** ✅ (0 错, examples/bins/tests 全编译; 仅历史 warnings):

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.44s
warning: the following packages contain code that will be rejected by a future version of Rust: nom v1.2.4, proc-macro-error2 v2.0.1
warning: `apeireth-companion` (example "multi_turn_agent") generated 1 warning (unused_variable 历史)
warning: `apeireth-companion` (lib test) generated 2 warnings (unused_variable 历史, 与本任务无关)
```

(注: examples + lib test 的 unused warnings 系历史遗留, 与本任务无关; nom/proc-macro-error2 是第三方包 future-incompat, 与本任务无关)

## 4. 0 装 PASS 边界 (诚实登记)

### 4.1 真 LLM 未接, trait 口已备
- `CausalLlm` trait 完整定义在 `causal_world_model.rs` 模块 doc 显式标注「真 LLM 未接, trait 口已备」
- 测试用 `MockCausalLlm { take_first, max_proposals }` 走通全链 (分支点判断 + 边提议)
- 真实接入路径明确: 按 `CausalBranchContext` 调 LLM (拆 judgment), 或按 `EdgeProposalRequest` 调 EvoCause 式 LLM

### 4.2 推演结果永远不入库 (防幻觉固化)
- `causal_world_model.rs` 模块不导入 `SqliteMemoryStore` (除 `#[cfg(test)]` 块内)
- `grep -n "put_episode\|memory_extractor\|experience::" causal_world_model.rs` 仅在 `#[cfg(test)]` 测试块内出现 (in-memory store 0 写入断言), 0 处实际生产代码调用
- 验收测试 `causal_world_model_does_not_persist_to_memory` 锁死此边界
- `reconcile_with_fact` 亦不入库 (与 W1 同纪律)

### 4.3 Brier 拒绝阈值
- 默认值: **0.3** (与 W1 一致, 防编故事硬边界)
- 可配置: `CausalSimulator::with_threshold(t)`
- 双层拒绝 (与 W1 同构):
  - **终点 Brier** (本次 forecast vs actual outcome) 超阈值 → `rejected=true`
  - **历史 mean_brier** (`CalibratedResolver.status()` 注入) 超阈值 → 整链预拒绝

### 4.4 LLM 评估同步接口留升级点
- `StateEvaluator<CausalMctsState>` 是 sync trait (TP7 落地形态), 但 `CausalLlm::judge_branch` 是 async
- 当前 `CausalGraphEvaluator::evaluate` 用启发式 (node_id 距离), 真接入时需用 `tokio::runtime::Handle` 桥接 async — 升级路径已留 (注释明确标注)

## 5. 复用 / 不重复造零件

| 既有零件 | 复用方式 |
|---|---|
| `memory_graph::GraphFact` | 因果节点/边的 s/p/o 骨干 + 双时态语义, 不重定义 |
| `oracle::WorldState` | 沙盘底座, 推演状态快照载体 |
| `oracle::Forecast` | 终点预测断言 + Brier score, 直接复用 (`Forecast::new` / `resolve`) |
| `oracle::CalibratedResolver` | 可选注入, 历史 mean_brier 校准整链 |
| `world_model::TimelineLlm` / `MockTimelineLlm` / `CounterfactualChain` | 抽象形态参考, **不直接复用** (W2 是另一语义维度, 独立命名空间) |
| `apeireth_cognition::planning::MctsPlanner` / `MctsConfig` / `SearchState` / `SearchAction` / `StateEvaluator` | TP7 落地, 直接复用 — 仅需为 `CausalMctsState` + `CausalMctsAction` 实现对应 trait |
| `SearchState` blanket impl | `impl<T: Clone + Send + Sync + Debug> SearchState for T {}` — `CausalMctsState` 自动满足, 无需手写 impl |
| `uuid::Uuid::new_v4` | 边/节点 id 生成 (复用 workspace 已有 crate, 不引新依赖) |

**未新增同名/同语义结构**; 未引入新 crate 依赖 (`apeireth-cognition` 已在 `Cargo.toml` line 33).

## 6. 纪律清单核对

| 纪律 | 状态 |
|---|---|
| 真 LLM mock, trait 口标"未接" | ✅ 见 §4.1 |
| 不注入记忆 (grep 确认) | ✅ 见 §4.2 |
| all-targets 编译 | ✅ 见 §3.3 |
| 锁纪律 (std Mutex 不可重入) | ✅ 未引入新锁; `CausalGraph` 用 `HashMap`/`Vec` 普通字段, 无嵌套取锁路径 |
| 报告路径 = taskId + 角色 | ✅ `reports/55e98df3-ded5-4c53-971d-f44bef73e69f-backend_engineer-report.md` |
| 台账完成即划 ✅ + 撤销错误 ✅ | ✅ 行 102/289 W2 错误 ✅ 撤销, 行 381 W3 占位 ✅ 撤销, 三处全 ✅ + 提交 680d2838 (rebase 后) |
| 不接任务包以外的活 | ✅ 仅做 TP32 范围内事, 未碰 W1/W4-W7/A2-A7/E1-E7 |

## 7. 已知遗留 / 不在本任务范围

- **真 LLM 未接**: `CausalLlm` trait 已定; 真实接入 (按 `CausalBranchContext` 调 LLM, 拆 judgment) 待统一 LLM 接入任务
- **StateEvaluator async 桥接**: 当前用启发式; 真 LLM 评估需 `tokio::runtime::Handle`, 留升级点 (注释明确)
- **多跳因果链挖掘**: 当前 `MineCausalEdges` 只挖 1-跳 (object→subject); 多跳 (A→B→C) 留待 HypothesisStore (F4) 主动假设检验
- **Kùzu 后端**: `GraphBackend` trait 已备; 当前用 Sqlite; Kùzu 后端因本机工具链缺失未接 (memory_graph 已 0 装 PASS 标注)
- **W3 Hybrid 来源检测**: `EdgeSource::Hybrid` 已定义; 当前 `MineCausalEdges` 和 `ProposeCausalEdges` 是独立路径, 合并 Hybrid 检测 (统计 + LLM 同时确认 → Hybrid) 留待后续任务

## 8. 结论

**TP32 / W2 + W3 世界模型第二层完成, 验收标准全数达成**:

- ✅ 7 个验收测试点全绿 (lib 绿 — causal_world_model 模块独立绿)
- ✅ 全 companion lib 回归 522/522 全绿 (515 基线 + 7 新增)
- ✅ `cargo check --workspace --all-targets` 0 错
- ✅ 0 装 PASS 边界锁死 (mock LLM / 不入库 / Brier 默认 0.3)
- ✅ 复用既有 oracle / memory_graph / cognition MCTS / world_model 零件, 不重复造沙盘
- ✅ W2 / W3 台账 ✅ + 提交 680d2838 登记完毕 (rebase 后)

**提交 hash (rebase 后)**: `680d2838` (实现) + `f06dae9e` (台账登记) + `8864588` (hash 引用同步)
**报告**: `reports/55e98df3-ded5-4c53-971d-f44bef73e69f-backend_engineer-report.md`

---

## 9. Rebase 后状态 (集成冲突第 2/3 轮处理摘要)

集成冲突发生于合并到 `team/e8de47ae-.../integration` (HEAD `d8f971ad`) 时. 本地 rebase 处理摘要:

### 9.1 冲突解决策略
- **Rebase 路径**: `git rebase --onto d8f971ad ae09b11f HEAD` — 取出 `ae09b11f..HEAD` (即 `4111d86`+`39ed6ca`) 重放到 `d8f971ad` 上.
- **冲突文件**: 无冲突 (causal_world_model.rs 是新文件, lib.rs 只在 W2 处插入 1 行, backlog.md 在 TP32 段落内修改 — 与 integration 无交叉冲突).
- **Stash 处理**: rebase 前 `git stash` 隔离工作树未提交改动 (含其他并行成员的修改), rebase 后 `git stash pop` 恢复. 隔离报告文件单独 stash 保护.

### 9.2 提交 hash 变更 (rebase 后)
| 原 hash | 新 hash | 说明 |
|---|---|---|
| `4111d86` | `680d2838` | feat(companion): TP32 实现 |
| `39ed6ca` | `f06dae9e` | docs(backlog): W2/W3 登记 |
| 新增 | `8864588` | docs(backlog): hash 引用同步 (4111d86 → 680d2838) |

### 9.3 Rebase 后验证
- ✅ `cargo test -p apeireth-companion --lib causal_world_model` → 7/7 全绿
- ✅ `cargo test -p apeireth-companion --lib` → 522/522 全绿
- ✅ `cargo check --workspace --all-targets` → 0 错
- ✅ causal_world_model.rs (38311 字节) 完整保留
- ✅ lib.rs re-export 完整保留 (`pub mod causal_world_model;` 在行 61)
- ✅ backlog.md W2 两处 (行 102/289) ✅ + W3 行 381 ✅ + 提交 680d2838 完整保留

### 9.4 Rebase 纪律核对

| 纪律 | 状态 |
|---|---|
| 不丢自己工作 | ✅ TP32 实现 (680d2838) + 台账 (f06dae9e) + hash 同步 (8864588) 全部保留 |
| 不丢集成已有进展 | ✅ rebase on top of integration HEAD d8f971ad (含 TP31 squash e4056aa0) |
| 冲突解决有据 | ✅ rebase 路径 `ae09b11f..HEAD` (内容等价 d8f971ad); 无内容冲突, 0 手动编辑 |
| 提交 hash 引用同步 | ✅ backlog.md 三处引用 + 报告 §0/§9.2 hash 表全部同步为 rebase 后实际值 |
| 0 装 PASS 边界仍锁死 | ✅ rebase 后 7 个测试点全绿 (含不落库边界) |

---

## 10. 与 W1 (TP31) 的衔接说明

W1 `TextualSimulator` 跑在"叙事时间线"上 (LLM 每步续写故事), W2 `CausalSimulator` 跑在"结构因果图"上 (LLM 只在分支点判断, 搜索空间 = memory_graph 因果网). 两者:
- 共用 `oracle::WorldState` (沙盘底座)
- 共用 `oracle::Forecast` + Brier 对账模式 (拒绝阈值 0.3)
- 共用 `CalibratedResolver` 可选注入
- 抽象同构 (`run` + `calibrate`/`reconcile_with_fact`) — 调用方可按场景选层: 叙事优先用 W1, 结构推理优先用 W2

未来可叠加: W1 推演 → W2 因果结构验证 → W1 续写 (双层互补), 但不在本任务范围.

— 后端工程师 / TP32
