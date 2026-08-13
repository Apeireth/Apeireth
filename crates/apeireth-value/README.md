# apeireth-value

**Apeireth 价值器官 — R14 Phase 4 A11.3 落点**

> "价值不是一句话，而是 5 层洋葱的一致性 + 一个 0.85 的门槛。"
> —— qa_engineer, 2026-08-01

## 职责

apeireth-value 是 Apeireth R14 双洋葱架构中的 **价值/动机评估器官**，提供：

1. **ValueEvaluation** — 单候选动机评分（motivation_score = 0.85 * autonomy + 0.10 * stability + 0.05 * intrinsic）
2. **ValuePrioritization** — 多候选排序（基于 ValuePriorityKind 权重）
3. **5 层原则洋葱一致性** — E/S/A/M/O 五层 stance 比较
4. **ASI V0.5 联动** — 接收 asi-asi 评分作为评估输入
5. **A12 守门联动** — 接收 `apeireth_core::PhilosophyVerdict` 作为评估准入
6. **Self-Disable 防护** — E 层 Conflict 自动触发 HasELayerConflict=true 阻止通过

## 5 层洋葱 (Principle Onion)

| Layer | 中文 | 含义 |
|-------|------|------|
| **E** | Principle (原则) | 最高层 — 不可被覆盖 |
| **S** | Value (价值观) | 长期价值取向 |
| **A** | Experience (经验) | 已验证经验 |
| **M** | Methodology (方法论) | 流程与决策模式 |
| **O** | Operation (操作) | 当前行动 |

5 层全部 `Aligned` → 通过；E 层 `Conflicted` → 硬拒。

## 0.85 门槛

```
motivation_score >= 0.85 → passes_threshold = true
motivation_score <  0.85 → passes_threshold = false
E 层 Conflicted → passes_threshold = false (硬拒)
```

## 用法

```rust
use apeireth_value::{evaluate_value, prioritize_values, check_5_layer_consistency};

let report = evaluate_value(&candidate)?;
let ranks = prioritize_values(&candidates);
let (verdict, diff) = check_5_layer_consistency(&stance_map_a, &stance_map_b);
```

## 测试

```
cargo test -p apeireth-value   # 61 tests: 46 unit + 15 integration
```

## 任务来源

A11.3 — R14 Phase 4 落地（qa_engineer, 2026-08-01）
- 上游: A11.1 (apeireth-action), A11.2 (apeireth-motivation), A12 (apeireth-consciousness)
- 下游: A13 (apeireth-life-force), A20 (apeireth-central)

v0.14.0-R14 — Apeireth R14 启动版

## R163 lint cleanup

2 -> 0 warnings. unreachable statement removed (onion_consistency.rs:193), redundant cmp binding restructured.
