# E2 LATS 化 MCTS — 自审报告

- **任务 ID**: cc378409-b7b6-4672-ba34-15003f58fa57
- **角色**: agent_orchestrator2
- **状态**: 已完成
- **提交**: <hash 待回填>

## 1. 交付物

| 文件 | 改动 |
|---|---|
| `crates/apeireth-cognition/src/planning.rs` | 尾部追加 LATS 扩展段 (~350 行含 7 组测试) + 顶部 1 行 Arc import; **既有 MctsPlanner/StateEvaluator/SearchResult 代码零改动** |
| `docs/backlog.md` | E2 划 ✅ |

## 2. 设计（LATS 三件套，扩展式）

1. **LLM 作 value**: `LlmValueFunction` trait 口（**LLM 版 0 装预留**, `value(state, depth) -> 0..1`）+ `HeuristicValue` 确定性启发式先行：目标接近度（宿主注入语义）+ 步数成本（深度递减截零）+ 历史成功率先验，三项加权和 clamp01。复用路径：宿主实现 LlmValueFunction 即可替换启发式，无需动搜索。
2. **反思节点**: `ReflectionRefiner` trait（`refine(state, reflection_text) -> Option<S>`）——**复用 E1 reflexion.rs 产物形态**（`[反思·...]` 文本，即 ReflectionText::text），深度 ≥1 的叶子扩展时按 `reflections_per_node` 配置精炼出反思子节点（action=None, is_reflection 标记）入树参与 UCT/备份；非反思形态文本 → refiner 返回 None 不入树（诚实）。
3. **max-backup**: `LatsNode.best_value` 取代平均（value_sum/visits），回溯路径上 `best_value = max(best_value, value)` 向上传播；UCT 利用项改用 best_value。LATS 以 LLM/启发式 value 直接估节点，**无随机 rollout → 搜索全程确定性**。
4. 结果复用既有 `SearchResult<A>`（新类型零增）；根下选最优时反思子节点如实排除（不作为首动作）。

## 3. 边界遵守

- 骨架签名零改动：`MctsPlanner` / `MctsConfig` / `SearchResult` / 三个 trait / 既有 5 个测试全部原样（本次同跑验证零破坏）。
- 复用 reflexion 公开接口形态（反思文本），cognition 不反向依赖 companion（跨 crate 依赖洁净）。
- 模块地图说明：maintenance-guide §二 地图限定 `crates/apeireth-companion/src/`，planning.rs 属 apeireth-cognition，不在该表范围 —— 如实记录不硬塞（如 Leader 需扩展地图范围请示意）。

## 4. 0 装 PASS 标注（诚实）

- LLM 版 value 未接（trait 口已留，测试以 ConstValue 假实现验证可插拔）
- 反思实接线（从 ReflexionStore 取 ReflectionText 注入 LatsPlanner::with_refiner）留宿主组合
- HeuristicValue 的目标接近度语义为宿主注入闭包，模块本身零语义假设

## 5. 测试结果（apeireth-cognition, `cargo test --lib planning` 过滤）

**12/12 全绿**（既有 MCTS 5 例 + LATS 7 例，0.00s）:
- lats_tests::heuristic_value_deterministic_combo（组合分 0.92/0.62 精确断言 + 确定性 ×5 + clamp01 上限）
- lats_tests::llm_value_trait_slot_pluggable（ConstValue(0.7) → best_value == 0.7）
- lats_tests::max_backup_propagates_max_not_average（max-backup 传播子树高值 + 方向正确）
- lats_tests::reflection_node_enters_tree_and_lifts_value（对照实验：注入 E1 形态反思文本后 best_value 显著高于无反思组）
- lats_tests::non_reflection_text_not_refined（非反思文本不入树，搜索正常）
- lats_tests::lats_deterministic_same_input_same_output（同输入同输出 ×3）
- lats_tests::empty_actions_returns_none（空动作如实 None）
- tests::* 既有 5 例全绿（骨架零破坏验证）
