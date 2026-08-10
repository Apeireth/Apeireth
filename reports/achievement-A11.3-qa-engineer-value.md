# Achievement A11.3 — apeireth-value 落地报告

**任务**: A11.3 — apeireth-value 价值器官（Ponytail 简化版）
**执行人**: qa_engineer
**日期**: 2026-08-01
**状态**: ✅ 已落地至 integration 分支（61 tests pass）

---

## TL;DR

- 新建 crate `apeireth-value` (v0.14.0-R14) — Apeireth R14 Phase 4 A11.3 落点
- 提供 ValueEvaluation / ValuePrioritization / 5 层原则洋葱一致性 三大 trait
- **61 tests** 全过（46 unit + 15 integration），example 跑通 6 场景
- 与 A11.1 (action) / A11.2 (motivation) / A12 (consciousness) / A13 (life-force) 接口对齐

## 1. crate 结构

```
crates/apeireth-value/
├── Cargo.toml                 # workspace 依赖继承
├── README.md                  # 模块/用法/5层洋葱/0.85门槛
├── examples/
│   └── value_demo.rs          # 6 场景演示
├── src/
│   ├── lib.rs                 # 公共类型 + re-export
│   ├── evaluation.rs          # ValueEvaluation trait + DefaultValueEvaluator
│   ├── prioritization.rs      # ValuePrioritization trait + DefaultPrioritizer
│   └── onion_consistency.rs   # 5 层洋葱 stance 比较
└── tests/
    └── value_tests.rs         # 15 集成测试
```

## 2. 核心 API

### 2.1 ValueCandidate / ValueDimension / ValuePriorityKind

```rust
pub struct ValueCandidate {
    pub id: Uuid,
    pub label: String,
    pub dimensions: Vec<ValueDimension>,
    pub priority_kind: ValuePriorityKind,
    pub autonomy_consistency: f64,
    pub value_stability: f64,
    pub intrinsic_motivation: f64,
    pub timestamp: i64,
    pub verdict: Option<PhilosophyVerdict>,
    pub target: Option<ActionTarget>,
}

pub enum ValueDimension { PrincipleE, ValueS, ExperienceA, MethodologyM, OperationO }
```

### 2.2 评估 (motivation_score = 0.85 * autonomy + 0.10 * stability + 0.05 * intrinsic)

```rust
pub fn evaluate_value(candidate: &ValueCandidate) -> ValueResult<ValueEvaluationReport>;

pub struct ValueEvaluationReport {
    pub candidate_id: Uuid,
    pub motivation: f64,
    pub alignment_map: BTreeMap<ValueDimension, ValueAlignment>,
    pub passes_threshold: bool,
    pub has_e_layer_conflict: bool,
}
```

### 2.3 排序

```rust
pub fn prioritize_values(candidates: &[ValueCandidate]) -> Vec<ValueRank>;
```

### 2.4 5 层洋葱一致性

```rust
pub fn check_5_layer_consistency(
    a: &BTreeMap<ValueDimension, OnionLayerStance>,
    b: &BTreeMap<ValueDimension, OnionLayerStance>,
) -> (ConsistencyVerdict, Option<(ValueDimension, ValueComparison)>);
```

## 3. 关键设计决策

| 决策 | 理由 |
|------|------|
| motivation 公式 = `0.85·a + 0.10·s + 0.05·i` | 自主一致性主导（v4.1 §13.2 内省优先） |
| `DEFAULT_THRESHOLD = 0.85` | 与 A11.2 motivation 对齐；E 层硬拒 |
| 5 层洋葱用 `BTreeMap` + `Ord` derive | 确定性迭代顺序 |
| `ValueCandidate` 不派生 Serialize | 上游 `apeireth_core::PhilosophyVerdict/ActionTarget` 未派生 |
| 三态 stance (Aligned/Underspecified/Conflicted) | 与 A12 守门 v3 verdict 对齐 |

## 4. 测试覆盖 (61 tests pass)

### Unit (46)
- `tests` 模块: 19 测试
- `evaluation::tests` 模块: 7 测试
- `prioritization::tests` 模块: 4 测试
- `onion_consistency::tests` 模块: 16 测试

### Integration (15)
end_to_end_dimension_label_round_trip, end_to_end_asi_v05_scores_linkage,
end_to_end_compare_higher_lower_equal, end_to_end_default_prioritizer_ranks_one_based,
end_to_end_s_layer_drift_verdict, end_to_end_cycle_avg_and_passing_count,
end_to_end_e_layer_hard_reject, end_to_end_motivation_score_and_threshold,
end_to_end_onion_layers_constant_matches_all, end_to_end_priority_kind_weight_sort,
end_to_end_custom_onion_mapping, end_to_end_target_propagates_through_evaluation,
end_to_end_value_error_score_out_of_range, end_to_end_empty_candidate_list_error,
end_to_end_value_error_invalid_input_via_empty_label

## 5. example 输出（6 场景）

```
=== apeireth-value demo (A11.3 — 动机/价值器官 v1) ===
[场景 1] 单候选评估: "长期诚实 > 一时方便"
  motivation_score = 0.920, passes 0.85 = true, 5 层全部 Aligned
[场景 2] "假装可复制人" verdict=Block(NotClone)
  motivation = 0.950, has_e_layer_conflict = true (E 层硬拒)
[场景 3] "摇摆的价值选择" value_stability = 0.2
  verdict = Drift, S 层 Conflicted, A 层 Aligned
[场景 4] 多候选排序 (3 同 motivation 不同 priority) — rank#1 #2 #3
[场景 5] evaluate_cycle (3 候选) + ASI V0.5 联动
  avg_motivation = 0.856, passing_count = 2/3
[场景 6] 自定义 OnionValueMapping 全 S 强制 Aligned
```

## 6. 已知简化（Ponytail）

- `ValueCandidate.label: String` 占位 → 升级 Bernstein 树 / TruthValue 字典
- 公式硬编码 0.85/0.10/0.05 → 完整版支持可调权重
- 无持久化接口 → P15 upgrade crate 提供序列化路径
- 待 core 派生 Serialize → 可一键补 ValueCandidate 序列化

## 7. 集成状态

- ✅ 8 文件 + 1 report 已 commit 至 integration 分支 (HEAD = 87b9621e+1)
- ✅ 61 tests 在 integration worktree 通过验证
- ✅ example 在 integration worktree 跑通 6 场景

---

**签名**: qa_engineer
**任务状态**: ✅ 已交付，等待 Leader 验收
