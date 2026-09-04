# B4 · Phase 2 交付报告：真双时态事实模型（BTFM 五元组）

> 依据 `_research_mem/ra/00-master-plan.md` §5 Phase 2 与 `ra2-bitemporal-algebra-proposal.md` · 2026-09-04
> 基线：`research/baselines/baseline-2026-09-phase0.md`

## 1. 问题定义与假设

- **问题**：`bitemporal_graph.rs` 实为单时态 valid-time 版本链（`valid_at_ms` 恒等于写入时刻），缺 transaction time（信念时间）轴，迟到事实无法表达。
- **假设**：BTFM 五元组 `(fid, φ, V, B, π, θ)` + 版本链语义可无破坏升级；三类切片（facts/beliefs/retrospective）可用确定性规则验证。

## 2. 交付内容

| 交付项 | 位置 | 状态 |
|---|---|---|
| 字段升级：`belief_at_ms` / `belief_until_ms` / `provenance` / `conflict_count`（serde 缺省兼容旧 JSON + `normalize_legacy_belief()` 兜底） | `bitemporal_graph.rs` | ✅ |
| `insert_fact_full`（显式 valid 区间 + 迟到事实入口，A4 闭包旧信念） | 同上 | ✅ |
| `retract_fact`（空 valid 区间 tombstone 追加；整键排除） | 同上 | ✅ |
| 三类查询：`facts_as_of` / `beliefs_as_of` / `retrospective`（版本链语义） | 同上 | ✅ |
| 信任半环：`belief_trust`（w⊗δ⊗κ，Viterbi 半环）+ `active_arbitrated_facts`（⊕=max 仲裁） | 同上 | ✅ |
| RA-2 §4 工作示例（迟到事实 + 更正）逐行测试 | tests | ✅ |
| 旧 API 语义不变（upsert/get_valid_facts_at/get_current_valid_facts，退化等价验证） | tests | ✅ |

**验证**：bitemporal 模块 8/8 绿；memory crate 全量 **653/653 绿**（+6 新测试）；workspace 编译 0 错。

## 3. 语义裁定（0 装，重要）

RA-2 提案 §4 的示意表格与其 §3.2 Datalog 规范存在三处矛盾。本实现**以 §3.2 代数/Datalog 为准**：

1. 采用**版本链语义**：`facts_as_of(t, τ)` = 每键在 `t ∈ V` 且 `b_s ≤ τ` 的版本中取最高 rev（"迟到更正覆盖重叠区间"）；`retrospective(t_ask, t_belief)` 同规则但到达门限用 `t_belief`。
2. **已修正的提案错误**：§4 表格 `retrospective(250,250)=北京` 在任何语义下不成立（250∉V(d1)=[0,200)），实现与测试均返回空集并注释说明。
3. 撤回语义：整键最新已到达版本为 tombstone（空 valid 区间）⇒ 该键从 facts/retrospective 排除；`beliefs_as_of` 如实保留 tombstone 行（审计完整性）。

## 4. 闸门核对

- 等价性门 ✅：旧 API 签名与语义不变；upsert 路径 belief==valid 退化等价有测试锚定。
- 性能门 ✅：新字段/方法纯 additive，旧路径无新增计算。
- 证据门 ⏳：工作示例与单测通过；迟到事实评测集（ra2-benchmark-design.md）接入留评测批。
- 术语门 ✅：trust 参数明示为"工程策略，非数据学习结论"。

## 5. 已知局限（0 装）

1. IC3 按线性信念链处理（多智能体分支信念留后续，RA-2 §8.1）。
2. trust 的 w/λ/κ 未校准（需 benchmark，RA-2 §8.2）。
3. 本升级为内存图（`Vec`）语义层；持久化（SQLite）与仲裁时间戳打通（RA-2 §6.2.5 PR-D）未做。
4. 旧序列化数据需加载路径显式调用 `normalize_legacy_belief()`（本 crate 当前无该加载路径，仅提供 API）。
