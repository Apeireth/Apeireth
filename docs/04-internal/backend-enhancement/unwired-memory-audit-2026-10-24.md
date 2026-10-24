# `engine/memory` 孤儿 API 核查报告（2026-10-24）

> 背景：扩大审计覆盖面后（PR #77），`engine/memory` 独占 **33 条**「生产零引用」，
> 是待判组（G5）里最多的一块。本页把这 33 条逐条查完，给出结论。
>
> **一句话结论**：33 条里**没有一条是「用户能感知的功能缺失」**——
> 核心功能都有主路径在用，零调用的都是**并行入口 / 便利包装 / 重复 API**。
> 但这次核查**不是白做**：它验证了一个重要判断（见 §4）。

---

## 1. 最重要的判断：这批不是「没接电」，是「多个入口」

`audit_unwired.py` 报的是「生产代码零引用」。但**零引用 ≠ 功能缺失** ——
一个能力可以有多个入口，生产只用主路径：

```
        ┌─ append()          ← 主路径（真实现）
append ─┼─ insert()          ← 兼容别名（有 doc 说明）
        └─ append_fact()     ← 无 doc 的再一层重复  ← 真冗余（已删）
```

所以这 33 条必须**逐条查主路径是否存在**，不能批量接线（接错了就是给库加冗余）。

## 2. 逐条结论

### 2.1 冗余入口 / 便利包装 → 归 G6（13 条）

**已核实「有主路径在用」**：

| 符号 | 主路径 | 证据 |
|---|---|---|
| `query_as_of` / `query_facts` / `query_recent` | `query()` | `typed_recall.rs:168` 用 `store.query(&TemporalGraphQuery...)` |
| `traverse_from` | `traverse()` | 同文件主方法 |
| `select_candidates` / `recall_with_policy` | `recall()` | `coordinator.rs:854` 用 `ProactiveRecallService::recall()` |
| `recall_into_context` | `retrieval_pipeline` | `build_l2_retrieval_disclosure` 在 pipeline 里被真实调用 |
| `consolidate_governed` | `consolidate` | 同文件主方法 |
| `events_with_ids` | `events` | 同文件主方法 |
| `get_for_subject` / `list_active` | 其他承诺查询 | commitments 有其他入口 |
| `get_history` | `history` | 同文件主方法 |
| `get_page` | `compile_page` | wiki 编译走主路径 |
| `get_persona` | persona 的 sweep/compress/consolidate | 五维 persona 有多条处理路径 |

### 2.2 实测「功能是生效的」（虚惊，3 条）

这三条最容易误判成安全/隐私缺口，逐条证伪了：

| 符号 | 担心什么 | 实际 |
|---|---|---|
| `fetch_governed`（公开版） | 治理读路径被绕过？ | **否**。内部走 `fetch_governed_on_conn`（288/305/323/339 行都在用）；公开版只是包装 |
| `is_episode_forgotten` | 遗忘不生效 ⇒ 已删记忆还被检索出来？ | **否**。`coordinator.rs` 在 5 处调 `get_governed(&ep.id)` 查 status，遗忘会被过滤 |
| `ensure_governance` / `governance_cas_failure` | 治理初始化/CAS 失败处理缺失？ | 走 `MemoryGovernanceStore` 的其他入口，非缺口 |

### 2.3 真冗余：已删（1 条）

| 符号 | 为什么删 |
|---|---|
| `append_fact` | ①零调用；②**无 doc**（对比 `insert` 有「Compatibility spelling」说明）；③实现与 `insert` **逐字相同**（都是 `self.append(fact).await`）⇒ 三层重复里的第三层 |

### 2.4 其余（16 条）→ 留在 G5 待判

`bottom` / `clear_working` / `default_min_count` / `is_verbatim_zone` / `links_for_diary` /
`materialize_to_sink` / `put_episode_full` / `query_with_time_range` / `retract_relation` /
`retrieve_with_reranker` / `scalar` / `set_quiet_threshold` / `streaks` /
`upsert_hallway` / `without_as_of` / `coalesce` 类内部工具。

这些**没有明显主路径可对**，需要设计意图判断（例如 `set_quiet_threshold` 是睡眠周期的
可配置入口，可能本就该由宿主调）。**不硬接、不乱删** —— 留待有真实需求时处理。

## 3. 净结果

| | 数量 |
|---|---|
| 核查前 `engine/memory` 在 G5 的条数 | 33 |
| 归 G6（已证实有主路径） | 13 |
| 证实功能生效（虚惊） | 3 |
| **真删** | **1**（`append_fact`） |
| 留在 G5 待判 | 16 |

**基线 164 → 163**（真减 1）。G5 从 90 → 77。

## 4. 这次核查的真正价值：修正了对「未接线」的理解

之前把「未接线」当成一个**统一的缺陷类别**。这 33 条的核查说明它有**至少两种形态**：

| 形态 | 特征 | 处置 |
|---|---|---|
| **A. 功能缺失**：能力有了但没人调 | 有主路径**不存在**，且功能对用户有意义 | **接线**（这是真债） |
| **B. 入口冗余**：同一能力多个入口 | 主路径**存在**且在用 | **删冗余或归组**（不是债） |

**判据**：拿到一条孤儿，先问「**这个能力有主路径吗？**」
- 有 ⇒ 形态 B，别接线（接了就变两套逻辑，更糟）；
- 没有 ⇒ 形态 A，看功能是否有用户价值。

这条判据已写进 `unwired-debt-taxonomy-2026-10-24.md`。

## 5. 下一批建议

G5 还剩 77 条，按「**有主路径吗**」这个判据继续判。优先看：
1. `capabilities/tools`（9 条）—— 工具面是用户直接接触的；
2. `engine/guard`（6 条）—— 安全面，零调用可能意味着护栏没生效（**值得优先**）；
3. `foundation/core`（6 条）。
