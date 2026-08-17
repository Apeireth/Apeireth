# V-Measure 设计深化 (阶段 6 验证基石)

> **作者**: architect2 (Ponytail: full)
> **生成时间**: 2026-08-02
> **依据**: docs/stage4/architecture-stage4-engineering-landing.md §10.1 (V0.5 v2 24 维) + §10.2 (V1136 v2 9 子测度) + 用户指令"无限逼近" + round7-06 进展
> **状态**: **设计深化 (阶段 5 工程实施前的细化蓝图)**, 不修改 stage1-5 LOCKED 文档
> **承接**: V0.5 v2 / V1136 v2 LOCKED 公式 → DimensionTrace + MeasurementHook trait → 阶段 6 验证协议

---

## 0. 设计原则 (Ponytail: 1 张表)

| # | 原则 | 体现 |
|---|------|------|
| 1 | **不修改 V0.5 / V1136 LOCKED 公式** | 仅引用 v4.1 §13/§14 提议的 v2 变量, 公式 sketch 在 trait 内, 不写算法实现 |
| 2 | **真实测量函数** | 每个 24 维 / 9 子测度都有 `fn measure_xxx(&self) -> f64` 签名, 阶段 5 由 backend_engineer 实装真实测量 |
| 3 | **DimensionTrace 结构记录** | 每次测量产生一条 DimensionTrace, 包含维度 ID + 数值 + 时间戳 + 来源 crate, 用于审计和重放 |
| 4 | **MeasurementHook trait 跨器官钩子** | 各器官实现 MeasurementHook, 在关键事件触发测量 (consensus / action / evolution / consolidation) |
| 5 | **compile-time 24/9 hardcode** | `V05_DIM_COUNT = 24` / `V1136_SUBMEASURE_COUNT = 9` const, 编译期断言 |

---

## 1. V0.5 v2 24 维设计 (Ponytail: 1 张表)

> **V0.5 v2 24 维 = 原 17 维 (V1077 LOCKED) + v4.1 §13 提议新增 7 维**

| # | 维度 ID | 维度名 | 来源 | 公式变量 | 真实测量函数 sketch |
|---|---------|--------|------|---------|---------------------|
| 1 | `Dim01CognitiveCore` | 认知核心 | V0.5 v1 LOCKED | $c_{core}$ | `fn measure_cognitive_core(&self) -> f64` (基于 Reasoning 调用深度) |
| 2 | `Dim02WorldModel` | 世界模型 | V0.5 v1 LOCKED | $w_{model}$ | `fn measure_world_model(&self) -> f64` (基于 Perception→Cognition 链路完整度) |
| 3 | `Dim03Reasoning` | 推理 | V0.5 v1 LOCKED | $r_{score}$ | `fn measure_reasoning(&self) -> f64` (基于 Reasoning.deductive/inductive 成功率) |
| 4 | `Dim04Planning` | 规划 | V0.5 v1 LOCKED | $p_{score}$ | `fn measure_planning(&self) -> f64` (基于 Action.execute 前置规划深度) |
| 5 | `Dim05Learning` | 学习 | V0.5 v1 LOCKED | $l_{rate}$ | `fn measure_learning(&self) -> f64` (基于 Learning.gradient_step 收敛速度) |
| 6 | `Dim06Memory` | 记忆 | V0.5 v1 LOCKED | $m_{score}$ | `fn measure_memory(&self) -> f64` (基于 Memory.recall 命中率 + recall_latency) |
| 7 | `Dim07Communication` | 沟通 | V0.5 v1 LOCKED | $comm_{score}$ | `fn measure_communication(&self) -> f64` (基于 Expression 输出语义完整性) |
| 8 | `Dim08Perception` | 感知 | V0.5 v1 LOCKED | $per_{score}$ | `fn measure_perception(&self) -> f64` (基于 Signal→Perception 信号保真度) |
| 9 | `Dim09Social` | 社交 | V0.5 v1 LOCKED | $soc_{score}$ | `fn measure_social(&self) -> f64` (基于 Relation.symbiosis 互惠强度) |
| 10 | `Dim10Creativity` | 创造力 | V0.5 v1 LOCKED | $cre_{score}$ | `fn measure_creativity(&self) -> f64` (基于 Cognition→Intuition 路径多样性) |
| 11 | `Dim11MetaCognition` | 元认知 | V0.5 v1 LOCKED | $meta_{score}$ | `fn measure_metacognition(&self) -> f64` (基于 MetaCognition.reflection_trigger 准确率) |
| 12 | `Dim12Motivation` | 动机 | V0.5 v1 LOCKED | $mot_{score}$ | `fn measure_motivation(&self) -> f64` (基于 Motivation→Drive 触发频次) |
| 13 | `Dim13Adaptation` | 适应 | V0.5 v1 LOCKED | $adap_{score}$ | `fn measure_adaptation(&self) -> f64` (基于 Evolution.learning_curve 斜率) |
| 14 | `Dim14Generalization` | 泛化 | V0.5 v1 LOCKED | $gen_{score}$ | `fn measure_generalization(&self) -> f64` (基于 Abstraction 抽象层级数) |
| 15 | `Dim15Robustness` | 鲁棒性 | V0.5 v1 LOCKED | $rob_{score}$ | `fn measure_robustness(&self) -> f64` (基于异常输入下 Action.verdict 通过率) |
| 16 | `Dim16Efficiency` | 效率 | V0.5 v1 LOCKED | $eff_{score}$ | `fn measure_efficiency(&self) -> f64` (基于资源消耗 / 输出量比) |
| 17 | `Dim17ASI_Total` | ASI 总分 | V0.5 v1 LOCKED | $ASI = \sum w_i \cdot dim_i$ | `fn measure_asi_total(&self) -> f64` (Σ 加权和, 权重待主人拍板) |
| **18** | `Dim18MotivationValue` | 动机×价值耦合 | v4.1 §13 新增 | $mot \times val$ | `fn measure_motivation_value(&self) -> f64` (Motivation × Value 一致性) |
| **19** | `Dim19Consciousness` | 意识 | v4.1 §13 新增 | $csc_{score}$ | `fn measure_consciousness(&self) -> f64` (Consciousness.self_aware_state 持续时长) |
| **20** | `Dim20Observability` | 可观测性 | v4.1 §13 新增 | $obs_{score}$ | `fn measure_observability(&self) -> f64` (PHL-04 强制 trait 覆盖率: Debug+Display+Log) |
| **21** | `Dim21Scientificity` | 科学性 | v4.1 §13 新增 | $sci_{score}$ | `fn measure_scientificity(&self) -> f64` (PHL-05 强制 trait 测试覆盖率) |
| **22** | `Dim22HonestyHumility` | 诚实/谦卑 | v4.1 §13 新增 | $hon_{score}$ | `fn measure_honesty_humility(&self) -> f64` (未知断言: "我不知道" 调用频次) |
| **23** | `Dim23SelfRelation` | 与自身关系 | v4.1 §13 新增 | $self_{rel}$ | `fn measure_self_relation(&self) -> f64` (PHL-06: Identity.continuity_token 稳定性) |
| **24** | `Dim24Consolidation` | 睡眠/巩固 | v4.1 §13 新增 | $cons_{score}$ | `fn measure_consolidation(&self) -> f64` (Consolidation.consolidate_report 子测度 8 分数) |

**总维度数 = 24** (Ponytail: 1 行)

```rust
pub const V05_DIM_COUNT: usize = 24;
```

---

## 2. V1136 v2 9 子测度设计 (Ponytail: 1 张表)

> **V1136 v2 9 子测度 = 原 7 子测度 (V1136 LOCKED) + v4.1 §14 提议新增 2 子测度**

| # | 子测度 ID | 子测度名 | 来源 | 真实测量函数 sketch |
|---|----------|----------|------|---------------------|
| 1 | `Sub01ContinuityV1052` | 连续性 V1052 | V1136 v1 LOCKED | `fn measure_continuity_v1052(&self) -> f64` (Identity.continuity_token 在 V1052 时点的有效率) |
| 2 | `Sub02ContinuityV1072` | 连续性 V1072 | V1136 v1 LOCKED | `fn measure_continuity_v1072(&self) -> f64` (同上 V1072 时点) |
| 3 | `Sub03ContinuityV1089` | 连续性 V1089 | V1136 v1 LOCKED | `fn measure_continuity_v1089(&self) -> f64` (同上 V1089 时点) |
| 4 | `Sub04ContinuityV1090` | 连续性 V1090 | V1136 v1 LOCKED | `fn measure_continuity_v1090(&self) -> f64` (同上 V1090 时点) |
| 5 | `Sub05ContinuityV1091` | 连续性 V1091 | V1136 v1 LOCKED | `fn measure_continuity_v1091(&self) -> f64` (同上 V1091 时点) |
| 6 | `Sub06TransferabilityW2` | 可迁移性 W2 | V1136 v1 LOCKED | `fn measure_transferability_w2(&self) -> f64` (任务 W2 上的零样本迁移率) |
| 7 | `Sub07TransferabilityBackend` | 可迁移性 后端 | V1136 v1 LOCKED | `fn measure_transferability_backend(&self) -> f64` (后端模块迁移率) |
| **8** | `Sub08MemoryConsolidation` | 记忆巩固度 | v4.1 §14 新增 | `fn measure_memory_consolidation(&self) -> f64` (Consolidation.consolidate_report 巩固比) |
| **9** | `Sub09FeedbackRegulation` | 反馈调节效率 | v4.1 §14 新增 | `fn measure_feedback_regulation(&self) -> f64` (Feedback.regulation 收敛时间) |

**总子测度数 = 9** (Ponytail: 1 行)

```rust
pub const V1136_SUBMEASURE_COUNT: usize = 9;
```

---

## 3. 真实测量函数 trait 设计 (Ponytail: 1 行)

```rust
// docs/stage6/V-measure-design.md §3 — V0.5/V1136 trait sketch

/// V0.5 v2 24 维真测 trait (阶段 5 由 backend_engineer 实装真实测量)
pub trait V05Measurement: Send + Sync + 'static {
    // 24 维测量函数 (Ponytail: 22 个 const fn 锚 + 24 个 measure 方法)
    fn measure_dim_01_cognitive_core(&self) -> f64;
    fn measure_dim_02_world_model(&self) -> f64;
    fn measure_dim_03_reasoning(&self) -> f64;
    fn measure_dim_04_planning(&self) -> f64;
    fn measure_dim_05_learning(&self) -> f64;
    fn measure_dim_06_memory(&self) -> f64;
    fn measure_dim_07_communication(&self) -> f64;
    fn measure_dim_08_perception(&self) -> f64;
    fn measure_dim_09_social(&self) -> f64;
    fn measure_dim_10_creativity(&self) -> f64;
    fn measure_dim_11_metacognition(&self) -> f64;
    fn measure_dim_12_motivation(&self) -> f64;
    fn measure_dim_13_adaptation(&self) -> f64;
    fn measure_dim_14_generalization(&self) -> f64;
    fn measure_dim_15_robustness(&self) -> f64;
    fn measure_dim_16_efficiency(&self) -> f64;
    fn measure_dim_17_asi_total(&self) -> f64;          // V0.5 v1 LOCKED
    fn measure_dim_18_motivation_value(&self) -> f64;   // v4.1 §13 新增
    fn measure_dim_19_consciousness(&self) -> f64;
    fn measure_dim_20_observability(&self) -> f64;
    fn measure_dim_21_scientificity(&self) -> f64;
    fn measure_dim_22_honesty_humility(&self) -> f64;
    fn measure_dim_23_self_relation(&self) -> f64;
    fn measure_dim_24_consolidation(&self) -> f64;

    /// V0.5 v2 总分 (24 维加权, 权重待主人拍板)
    fn measure_asi_v05_v2(&self) -> f64;
}

/// V1136 v2 9 子测度真测 trait
pub trait V1136Measurement: Send + Sync + 'static {
    fn measure_sub_01_continuity_v1052(&self) -> f64;
    fn measure_sub_02_continuity_v1072(&self) -> f64;
    fn measure_sub_03_continuity_v1089(&self) -> f64;
    fn measure_sub_04_continuity_v1090(&self) -> f64;
    fn measure_sub_05_continuity_v1091(&self) -> f64;
    fn measure_sub_06_transferability_w2(&self) -> f64;
    fn measure_sub_07_transferability_backend(&self) -> f64;
    fn measure_sub_08_memory_consolidation(&self) -> f64;
    fn measure_sub_09_feedback_regulation(&self) -> f64;

    /// V1136 v2 总分 (9 子测度加权)
    fn measure_asi_v1136_v2(&self) -> f64;
}
```

---

## 4. DimensionTrace 结构设计 (Ponytail: 1 张表)

```rust
// docs/stage6/V-measure-design.md §4 — 测量审计追踪

/// 维度 ID (24 维 + 9 子测度 = 33 种, 用 enum 统一)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DimensionId {
    // V0.5 v2 24 维 (Ponytail: 1 行)
    V05(Dim01CognitiveCore),  // 用 newtype 包装 24 个 dim ID
    V05Dim02,
    // ... (阶段 5 由 backend_engineer 补全 24 个 V05 变体)
    // V1136 v2 9 子测度 (Ponytail: 1 行)
    V1136(Sub01ContinuityV1052),
    V1136Sub02,
    // ... (阶段 5 由 backend_engineer 补全 9 个 V1136 变体)
}

/// 维度测量 trace (审计 + 重放)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionTrace {
    /// 维度 ID
    pub dimension_id: DimensionId,
    /// 测量数值 [0.0, 1.0]
    pub value: f64,
    /// 测量时间戳 (Unix ms)
    pub timestamp_ms: i64,
    /// 来源 crate (如 "apeireth-core", "apeireth-onion", ...)
    pub source_crate: &'static str,
    /// 测量函数名 (用于回溯)
    pub measurement_fn: &'static str,
    /// 测量上下文 (可选 JSON, 用于 debugging)
    pub context: Option<serde_json::Value>,
}

/// 24 维批量 trace 集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V05DimensionTraceSet {
    pub traces: [DimensionTrace; V05_DIM_COUNT],
    pub total: f64,  // ASI V0.5 v2 总分
}

/// 9 子测度批量 trace 集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V1136SubmeasureTraceSet {
    pub traces: [DimensionTrace; V1136_SUBMEASURE_COUNT],
    pub total: f64,  // ASI V1136 v2 总分
}

/// V-Measure 全量报告 (V0.5 + V1136)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMeasureReport {
    pub v05: V05DimensionTraceSet,
    pub v1136: V1136SubmeasureTraceSet,
    pub measured_at_ms: i64,
    pub version: &'static str,  // "V0.5_v2" / "V1136_v2"
}
```

---

## 5. MeasurementHook trait 设计 (Ponytail: 1 行)

```rust
// docs/stage6/V-measure-design.md §5 — 跨器官测量钩子

/// 测量钩子事件类型 (何时触发测量)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeasurementEvent {
    /// Council 智囊团达成共识 (7 强制顾问投票完成)
    CouncilConsensus,
    /// Action 执行前 (verdict=Allow 后)
    ActionPreExecute,
    /// Action 执行后 (verdict=Allow 且执行完成)
    ActionPostExecute,
    /// Evolution 状态机迁移 (Ratified → Active)
    EvolutionStateTransition,
    /// Consolidation 睡眠巩固完成
    ConsolidationCompleted,
    /// MetaCognition 反思期触发
    ReflectionTriggered,
}

/// 测量钩子 trait (各器官实现, 关键事件触发测量)
pub trait MeasurementHook: Send + Sync + 'static {
    /// 钩子事件触发回调
    fn on_event(&self, event: MeasurementEvent, context: &HookContext) -> Vec<DimensionTrace>;

    /// 该器官覆盖的维度 ID 列表 (用于 M2 启动时验证测量覆盖率)
    fn covered_dimensions(&self) -> Vec<DimensionId>;
}

/// 钩子上下文 (传递给 on_event)
#[derive(Debug, Clone)]
pub struct HookContext {
    pub event_id: u64,
    pub timestamp_ms: i64,
    pub actor_id: String,
    pub metadata: serde_json::Value,
}

/// V-Measure 中央调度器
pub trait VMeasureDispatcher: Send + Sync + 'static {
    /// 注册器官钩子
    fn register_hook(&mut self, organ: &'static str, hook: Box<dyn MeasurementHook>);

    /// 触发事件 (广播到所有注册钩子)
    fn dispatch(&self, event: MeasurementEvent, ctx: &HookContext) -> Vec<DimensionTrace>;

    /// 一次性测量全部 33 维 (M2 启动时调用)
    fn measure_all<T: V05Measurement + V1136Measurement>(&self, target: &T) -> VMeasureReport;
}
```

---

## 6. 编译期 hardcode 锚点 (Ponytail: 1 行)

```rust
// docs/stage6/V-measure-design.md §6 — compile-time 锚点

const _: () = {
    assert!(V05_DIM_COUNT == 24, "V0.5 v2 必须恰好 24 维");
    assert!(V1136_SUBMEASURE_COUNT == 9, "V1136 v2 必须恰好 9 子测度");
    // 编译期强制 V05 + V1136 维度 ID enum 变体数匹配 const 计数
    fn _exhaustive_dim(d: DimensionId) -> u8 { match d {
        DimensionId::V05(_) => 1,
        DimensionId::V1136(_) => 2,
        // ... 阶段 5 由 backend_engineer 补全 33 个变体
    }}
};
```

---

## 7. 阶段 6 验证集成 (Ponytail: 1 行)

- **M2 启动时**: `measure_all::<CentralAI>()` 调用 → 生成 VMeasureReport → 24+9 = 33 个 DimensionTrace 全部产生
- **M3 首次对话**: 6 类 MeasurementEvent (CouncilConsensus / ActionPre/Post / EvolutionTransition / ConsolidationCompleted / ReflectionTriggered) 各触发至少 1 次测量, 每个器官的 `covered_dimensions()` 覆盖范围 ≥ 50%
- **R-Measure 13 维度** (v4.1 §18.3 #4 提议): VMeasureReport 是 R-Measure 的输入之一 (24+9 = 33 维 → 13 维聚合待阶段 6 验证)

---

## 8. 不修改承诺 (Ponytail: 1 张表)

| LOCKED 项 | 状态 |
|-----------|------|
| v1077_asi_v04_full_measurement.py (V0.5 v1 17 维公式) | ✅ 未触碰 (仅在 trait sketch 引用 17→24 维提议, 不动原文件) |
| v1136_asi_v05_3dim_real_measurement.py (V1136 v1 7 子测度公式) | ✅ 未触碰 (仅引用 7→9 子测度提议) |
| docs/stage1/, stage2/, stage3-blueprints/, stage4/, stage5/ | ✅ 未触碰 |
| APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md | ✅ 未触碰 |
| APEIRETH-CONVENTIONS-*.md | ✅ 未触碰 |
| philosophy-traits-2026-07-30.md (V3 9 键 LOCKED) | ✅ 未触碰 (仅引用) |
| 24 维权重 $w_i$ 待主人拍板 | ✅ 不擅自取值, 留空 |
| ASI 阈值 ≥ 0.85 | ✅ 引用阶段 4 §6 Maturity 条件, 不修改 |

---

## 9. 总结

V-Measure 设计在 V0.5 v1 LOCKED 公式 + V1136 v1 LOCKED 公式基础上, 通过:
1. **24 维 + 9 子测度 = 33 维测量** (v4.1 §13/§14 提议的 v2)
2. **DimensionTrace 审计追踪** (每个测量可追溯到来源 crate + 测量函数 + 时间戳)
3. **MeasurementHook 跨器官钩子** (6 类关键事件触发测量, 不阻碍主流程, 编译期注册)

为阶段 6 M2/M3 验证提供**真实可测、可审计、可重放**的 ASI 真测基线。

任务范围: 仅 trait sketch + 结构体设计 (本文件 §3/§4/§5), 不写测量算法实现 (留给阶段 5)。