# B3 · Phase 3 交付报告：上下文在线保留决策（StackPin + ShadowLogger + 离线 replay）

> 依据 `_research_mem/ra/00-master-plan.md` §5 Phase 3 与 `ra3-formal-model-and-algorithms.md` · 2026-09-04
> 基线：`research/baselines/baseline-2026-09-phase0.md`

## 1. 问题定义与假设

- **问题**：每轮在 token 预算内对上下文段做 Retain/Compress/Fold/Drop/Protect 决策；现状 rot_score 是逐段贪心启发式（无竞争比定义、无 OPT 参照、无切换代价）。
- **假设**：等尺寸分桶 + core 钉住 + recency 栈（StackPin）落在经典 paging 抽象内，继承 LRU 的 k-competitive 上界（Sleator-Tarjan 1985，护栏命题，非新定理）；shadow 记录为价值估计提供 logged 数据。

## 2. 交付内容（全部 Research 前缀，默认关闭）

| 交付项 | 位置 | 状态 |
|---|---|---|
| `ResearchContextPolicy` trait（五动作决策接口） | `orchestration/src/research_context_policy.rs` | ✅ |
| `ResearchStackPinPolicy`（Proposal A：分桶等尺寸化 H1 + Retain/Drop H2 + core 钉住 H3 + touch 观测 H4 + Fold 叠加） | 同上 | ✅ |
| `ResearchShadowLogger`（Proposal C：shadow_entry JSONL，schema 对齐 `research/logs/README.md`） | 同上 | ✅ |
| 离线 replay：确定性合成请求生成（xorshift64*）+ Belady OPT + LRU(StackPin paging) miss 计数 + 竞争比测量 | 同上 | ✅ |
| 单测 9 项（栈属性嵌套 / core 永不被 Drop / touch 提升 recency / Fold 叠加 / **竞争比 ≤ k 于 4 组种子** / JSONL schema / 分桶上取整 / 成本守预算 / 决策纯函数） | 同上 | ✅ |

## 3. 竞争比护栏验证（RA-3 §2.3 路线）

- 合成序列：universe=40、hot=8、p_hot=0.7、n=400、种子 {1,7,42,99}；k=8。
- 断言：`online_misses ≤ k × opt_misses`（Belady OPT 为离线参照）——4 组种子全过。
- 口径声明：此为**等尺寸 paging 抽象**（H1–H4）内的护栏验证；真实段尺寸不等/含 Compress/切换代价时，该上界退化为工程护栏而非端到端保证（RA-3 §2.2 L2/L3 分层）。

## 4. 闸门核对

- 等价性门 ✅：`context_rot` / `context_budget` 零改动；本模块不挂任何生产装配路径（默认关闭）。
- 性能门 ✅（默认路径）：生产路径零新增计算。
- 证据门 ⏳：合成序列竞争比已验证；LoCoMo/LongMemEval 效用-成本曲线留 Phase 3 评测批（需数据集接入）。
- 术语门 ✅：文档无"已证明/SOTA"表述；竞争比表述为"继承经典定理的护栏"。

## 5. 已知局限（0 装）

1. 竞争比只覆盖等尺寸 paging 抽象；VaultLRU/FTRL（Proposal B，O(√T) 后悔界）未实现，留后续批次（4–6 人周量级）。
2. ShadowLogger 为内存/JSONL 导出，未接生产管线（生产路径零改动的设计要求）。
3. 切换代价按 tail 桶数粗估（RA-3 §1.4 近似），精确版需 provider 实测 prompt_tokens。
4. 评测集（LoCoMo/LongMemEval）未接入；效用-成本曲线与 7 策略矩阵实验待评测批。
