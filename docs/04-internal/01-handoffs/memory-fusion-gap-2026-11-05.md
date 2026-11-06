[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-fusion-gap-2026-11-05.md
Class: DECISION
Status: 🟡 待裁决
Last-Modified: 2026-11-05
Author: deputy

# 「自研融合」的真缺口 ⇒ **融合链缺零件**（不是缺融合面）

> **背景**：owner 问「每个模块都是一个算法呗，我们就不能自研融合吗」
> ⇒ ⇒ 本文件回答：**融合已经在做**，**入口已经收敛**，**缺的是链上的零件**。
> **上游**：[`memory-super-engine-verdict-2026-11-05.md`](./memory-super-engine-verdict-2026-11-05.md)（含一次自我更正）
> · [`memory-module-atlas-2026-11-05.md`](./memory-module-atlas-2026-11-05.md)（实测图谱）

## 一、**先纠正两个前提**（**都是实测**）

### ① 「每个模块都是一个算法」⇒ ⇒ **一半对**，而且这决定了做法
- **多数不是「竞争算法」，是「链条环节」**：
  治理域 7 个候选 = `admission_gate`（准不准进）→ `memory_governance`（写面治理）→
  `access_history`+`forget_access_guard`（先记访问史）→ `universal_forget`（按主体全量遗忘）→
  `retention`（保留）→ `forgetting_metrics`（量）→ `forget_coordinator`（协调）
  ⇒ ⇒ **是一条链的 7 环，不是 7 个算法抢一个位置** ⇒ ⇒ **融合 = 串链，不是择一**；
- **真竞争只有少数**：图谱族（`amem_graph` / `bitemporal_graph` / `temporal_graph_store` / `graph_algo`）·
  召回族（`hybrid_search` / `retrieval_pipeline` / `memory_rank` / `proactive_recall`）·
  注入对照（`golden_injection` 一轮 vs `golden_injection2` 二轮 ⇒ ⇒ **第二轮是实验**，不是候选）。

### ② ⭐ **「融合面 10 个入口」是我判据错造成的误判 ⇒ ⇒ 实测：生产入口只有 1 个**
- 能**发起一次召回**的公开入口共 **12** 个，但**生产真在用的只有 1**：
  `MemoryCoordinator::recall(&query)`（`runtime-assembly/canonical/memory_module/execution.rs:93`）
  ⇒ ⇒ 内部走 `retrieval_pipeline`；
- ⚠️ 那三个「生产调用」里，**前两个是同名不同物**：`search` 命中的是 **`crates/engine/image/algo/**` 的图像算法**，
  `retrieve` 命中的是 README；
- ⇒ ⇒ ⇒ **融合面已经收敛** ⇒ ⇒ **「统一召回面」不是缺口**。

## 二、⭐ **那「自研融合」到底已经做到什么程度**（实测）

| 融合要素 | 现状 | 证据 |
|---|---|---|
| **融合配方对象** | ✅ **已有**：`MemoryRankingConfig`（9 维权重 + `diversity_lambda`）⇒ ⇒ 且注释明写「**No retrieval stage should own magic weights**」 | `scope.rs:168` |
| **多路召回融合** | ✅ **已有**：BM25（词法）+ 向量 + 静态向量 | `retrieval_pipeline.rs` |
| **多信号融合打分** | ✅ **已有**：`ScoreComponents::weighted(&config)` = 9 维加权和 | `scope.rs:149` |
| **权重自演化** | ✅ **已有**：`RetrievalTuningEngine::observe_batch` 按使用证据改写权重 + 会话桶隔离 | `retrieval_tuning.rs:416` |
| **不许悄悄劣化** | ✅ **已有**：`QualityDriftMonitor`（EMA 基线 + 2σ 连击告警） | `quality_drift.rs` |
| **不许掉分** | ✅ **已有**：`golden_set::check_no_regression`（CI 闸门，相对劣化 >1% 拦下） | `scripts/check_golden_baseline.py` |

⇒ ⇒ ⇒ **结论：「自研融合」不是「要不要做」，而是「已经做了、而且是这条线做得最实的部分」**。

## 三、⭐ **真缺口：融合链上缺零件**

**实测**：能**发起召回**的能力里，以下**建好了但没进生产链**（⇒ ⇒ 融合里根本没有它们）：

| 零件 | 行数 | 现状 | 它能给融合带来什么 | 接它的成本 |
|---|---|---|---|---|
| **`bitemporal_graph`** | 781 | 零消费（内部被 `admission_gate`/`amem_graph` 用） | ⭐ **「当时知道什么」**（时间旅行记忆）⇒ ⇒ 双时态事实可进融合 ⇒ ⇒ 答「为什么当时这么答」 | **低**（内部已有 2 处用法）⇒ ⇒ 下游消费点已存在 |
| **`amem_graph`** | 606 | 零消费（内部被 `community` 用） | ⭐ **A-MEM 图算法** ⇒ ⇒ 联想式召回（不靠字面相似） | **低**（内部 1 处用法） |
| **`graph_algo`** | 639 | 零消费（内部被 `community` 用） | 图遍历 ⇒ ⇒ 融合的图维度算子 | **低** |
| **`signal_sources`** | 1108 | 零消费（内部被 `anchor_injection` 用） | ⭐ **非答案侧的题面信号** ⇒ ⇒ 召回质量的关键输入（**「问了什么」比「答了什么」更早**） | **低**（内部 1 处用法） |
| **`calibration`** | 498 | 零消费（内部被 `calibration_critic`/`ensemble` 用） | ⭐ **校准**（Brier/ECE）⇒ ⇒ 让「自信度」这一维**可信** | **低**（内部 2 处用法） |
| **`reflexion`** | 977 | 零消费（内部被 `calibration_critic` 用） | ⭐ **教训回路** ⇒ ⇒ 失败轨迹回喂 ⇒ ⇒ 权重自演化的**信号来源** | **中**（要接行为面） |
| **`dreaming`** | 305 | 零消费（内部被 `dream_consolidation`/`dream_wiring` 用） | ⭐ **离线做梦 / 长程重组** ⇒ ⇒ 融合的「深层记忆」 | **中** |
| **`meta_thinking`** | 485 | 零消费（内部被 `dream_wiring`/`dreaming` 用） | 元思考 ⇒ ⇒ 反思与整编的中间层 | **中** |
| **`persistent_vector`** | 584 | 零消费（内部被 `vector_rebuild` 用） | 持久向量索引 ⇒ ⇒ **向量维度的持久化后端** | **中** |
| **`vector_rebuild`** | 460 | 零消费（内部零消费） | 派生索引重建 | **中**（要新建消费点） |
| **`vector_distance`** | 200 | 零消费（内部零消费） | 向量距离工具 | 低 |
| **`offline_embed`** | 253 | 零消费（内部零消费） | **纯本地嵌入** ⇒ ⇒ 无外部依赖的向量来源 | 中 |
| **`ensemble`** | 756 | 零消费（内部零消费） | 集成预测聚合 + Hanson LMSR | 中 |
| **`session_lifecycle`** | 796 | 零消费（内部零消费） | 会话生命周期 ⇒ ⇒ **跨轮融合的载体** | 中 |
| **`principles`** | 871 | 零消费（内部零消费） | 动态原则层 ⇒ ⇒ 长期人格/规则的沉淀面 | 中 |
| **`admission_gate`** | 291 | 零消费（内部零消费） | ⭐ **准入控制**（什么记忆准进来）⇒ ⇒ **安全面** | 中（安全面 ⇒ 需裁决） |
| **`identity`** | 499 | 零消费（内部零消费） | 身份卡跨载体存储 ⇒ ⇒ 主体连续性 | 中 |

## 四、📌 **给 owner / 总指挥的选项**

### 选项 A：**先接「成本最低 + 融合收益最高」的 5 个**（**我建议**）
⇒ ⇒ `bitemporal_graph` · `amem_graph` · `graph_algo` · `signal_sources` · `calibration`
⇒ ⇒ **共同点**：**内部已有用法 ⇒ ⇒ 不需要新建消费点** ⇒ ⇒ **成本最低**
⇒ ⇒ **融合收益**：图维度算子（3 个）+ 题面信号（1 个）+ 置信度可信（1 个）
⇒ ⇒ ⇒ **这 5 个进链 ⇒ ⇒ 融合从「词法+向量」变成「词法+向量+图+信号+校准」**

### 选项 B：**A + 测量「融合配方 A vs 配方 B」**（**A 的必要配套**）
⇒ ⇒ 现有闸门只有「**不许变差**」（相对劣化 >1% 拦下）
⇒ ⇒ ⇒ 加一条「**不许不涨**」：**新零件接入后，融合效果必须**提升**才留**
⇒ ⇒ ⇒ **没有这条 ⇒ ⇒ 融合就是「越加越多，说不清谁有用」** ⇒ ⇒ **这正是「糊成一团」的成因**

### 选项 C：**A + B + 教训回路**（`reflexion` 接行为面）
⇒ ⇒ **权重自演化的信号来源** ⇒ ⇒ 融合能自我改进得更狠
⇒ ⇒ 代价：要接行为面（跨区）⇒ ⇒ 需协调

### 选项 D：**等「记忆引擎 v3 设计」一起做**
⇒ ⇒ 若 v3 要重设计 ⇒ ⇒ A/B 都可不做（直接 v3 里一次解决）

### 我的建议
**A + B**（⇒ ⇒ 5 个零件进链 + **给融合装上「不许不涨」的尺子**）
⇒ ⇒ ⚠️ **B 不能省** ⇒ ⇒ 因为**没有度量就没有融合治理**
⇒ ⇒ ⇒ 且 B 的成本极低（`golden_set` 上已有全部量具 ⇒ ⇒ 只需一条新判定）

## 五、⛔ **我明确反对的一件事**

### 反对：**「把所有模块都塞进融合链」**
- 依据：多数模块是**链条环节**不是算子 ⇒ ⇒ 塞进去只会让**链路更长、延迟更高、可解释性更差**；
- ⭐ **实测反例**：`admission_gate`（准入）是**安全面** ⇒ ⇒ 它该在**入口**拦，而不是**在融合里参与打分**
  ⇒ ⇒ 「塞进融合链」会把**安全边界**变成**打分项** ⇒ ⇒ **边界一旦可调，就等于没有边界**；
- ⇒ ⇒ **替代**：**按「它是算子 / 环节 / 边界」三分** ⇒ ⇒ 算子进融合 · 环节进链 · **边界留在门口**。

## 六、我这次**没有做**什么

| 没做 | 原因 |
|---|---|
| **没接那 5 个零件** | 跨区（`runtime-assembly` 是他的区）⇒ ⇒ 需先拍板 |
| **没写「不许不涨」的判定** | 要动 `golden_set` / CI 闸门 ⇒ ⇒ 属**验收口径** ⇒ ⇒ 需拍板 |
| **没合并任何模块** | 融合 ≠ 合并（见 §一 ①）⇒ ⇒ **124/132 真不同** ⇒ ⇒ 合并是倒退 |
| **没声称「融合完成」** | ⚠️ 融合链**仍缺 17 个零件**（§三）⇒ ⇒ 现在说完成 = 虚假 |

## 七、复现命令

```sh
cd /home/bai/Projects/apeireth-debt94-wt
grep -n "pub struct MemoryRankingConfig" -A 14 crates/engine/memory/src/scope.rs   # 融合配方对象
grep -n "pub fn weighted" -A 10 crates/engine/memory/src/scope.rs                   # 9 维加权和
grep -n "pub fn observe_batch" crates/engine/memory/src/retrieval_tuning.rs         # 权重自演化
grep -rn "c.recall(&query)" crates/engine/runtime-assembly/src/canonical/memory_module/execution.rs  # 唯一生产入口
```

## 相关文件

- 判断书（含自我更正）：[`memory-super-engine-verdict-2026-11-05.md`](./memory-super-engine-verdict-2026-11-05.md)
- 模块图谱：[`memory-module-atlas-2026-11-05.md`](./memory-module-atlas-2026-11-05.md)
- 收敛方案：[`memory-convergence-plan-2026-11-05.md`](./memory-convergence-plan-2026-11-05.md)
