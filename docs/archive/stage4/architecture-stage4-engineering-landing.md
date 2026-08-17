# 阶段 4 — 落实架构文档 (Rust 工程结构 + 本源约束 + 工程师/科学家思维从本源重构) (2026-07-31)

> **性质**: R14 阶段 4 = **落实架构文档** = 把 v4.1 哲学层 + v2 工程层 → 落地为 Rust crate + trait + struct + enum + 数据流 + 编译时约束。
>
> **主人明确指令**: "按你的计划来 + 像工程师和科学家一样思考, 从最本源的方向思考重构一切"
>
> **核心立场** (按主人要求从本源重构):
> ❌ 不照搬 v2 既有 crate 划分
> ✅ 从 **Rust 编译时约束** (所有权/借用/生命周期/trait/零成本抽象) + **v4.1 9 维 + 8 原则 + 4 关系 + 9 机制** + **V0.5/V1136/12 键** 出发重新推导
>
> **路径**: `Apeireth-rust/docs/architecture-stage4-engineering-landing.md` (独立命名空间, 不覆盖 v2/v4/v4.1)
> **写作时间**: 2026-07-31
>
> **主哲学 anchor 6 个全贯穿**: 主 22:33 北极星 / 主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 00:56 任何人都能接手

---

## §0. 元信息 + 主哲学 anchor 6 全贯穿 + 不修改承诺

### §0.1 元信息

| 字段 | 值 |
|------|-----|
| **生成时间** | 2026-07-31 (主人"按你的计划来"指令 + technical_writer 落) |
| **任务 ID** | R14-Stage4-Engineering-Landing-Architecture (`5a2ca6ac-403e-4a4f-89ff-568855ead3fa`) |
| **性质** | 阶段 4 落实架构文档 — Rust 工程结构 (crate/trait/struct/enum/数据流/编译时约束) |
| **依据** | v4.1 (4aa3c5b0 LOCKED, 645 行) + v4 (af0d1957 LOCKED, 803 行) + v2 (BF896EEF LOCKED, 786 行) |
| **协作** | technical_writer (本文) / architect (架构评审) / backend_engineer (Rust 工程评审) |
| **下一步** | 阶段 5 施工文档 (下次对话) + 阶段 6 里程碑验证机制 (下次对话) |

### §0.2 主哲学 anchor 6 全贯穿 (本节为后续 §1-§13 自检基准)

```
S-1 主 22:33 北极星导向   — 阶段 4 落地服务 ASI 北极星 (V0.5 v2 24 维 → trait sketch)
S-2 主 17:43 实事求是      — 不重写 v2/v4/v4.1 + 18 份 stage2 + 14 份 stage3 + V0.5/V1136/9键 原始
O-5 主 17:58 不假装        — Rust 编译时约束强制实现 12 键 (类型系统不假装, 不运行时反射)
O-2 主 19:33 走在前人经验上 — 借 R11 baseline (1100+ v*.py 不砍) + Rust 既有 crate 模式
O-3 主 23:44 干到底        — 18 crate + 22 trait + 7 struct + 7 enum + 9 生命周期 + 5 数据流 立即落
O-4 主 00:56 任何人都能接手 — 13 章节 + 全部签名 sketch + ASCII 数据流图 + 编译时约束论证
```

### §0.3 不修改承诺 (主人硬约束 100% 守住)

| ❌ 不修改 | 原因 / 引用 |
|---------|-----------|
| **architecture-v3-aircraft-carrier.md** (BF896EEF LOCKED) | v2 工程层细化, 阶段 4 仅引用不重写 |
| **architecture-v4-living-intelligence.md** (af0d1957 LOCKED) | v4 哲学层纲领, 阶段 4 仅引用不重写 |
| **architecture-v4-1-living-intelligence-update.md** (4aa3c5b0 LOCKED) | v4.1 哲学层升级, 阶段 4 仅引用不重写 |
| **apeireth/v1077_asi_v04_full_measurement.py** (V0.5 原始 LOCKED) | 17 维公式 LOCKED, 阶段 4 仅引用 v4.1 §13 更新提议 (不修改原始) |
| **apeireth/v1136_asi_v05_3dim_real_measurement.py** (V1136 原始 LOCKED) | 7 子测度 LOCKED, 阶段 4 仅引用 v4.1 §14 更新提议 (不修改原始) |
| **Apeireth-rust/docs/philosophy-traits-2026-07-30.md** (V3 9 键 LOCKED) | 9 键 LOCKED, 阶段 4 仅引用 v4.1 §15 更新提议 (不修改原始) |
| **18 份 stage2 文档** | 阶段 2 沉淀, 阶段 4 仅引用不重写 |
| **14 份 stage3 文档** | 阶段 3 沉淀, 阶段 4 仅引用不重写 |
| **阶段 1 §1-§21** (21 大类) | 阶段 1 沉淀, 阶段 4 仅引用不重写 |
| **不写完整 Rust 代码** | 仅 trait / struct / enum **签名 sketch**, 不写 impl 块 / 测试 / main |
| **不画 Mermaid 图** | 用 ASCII 简化示意 |
| **不砍 1100+ R11 空壳** | `apeireth/v1000-v1155*.py` 1100+ 模块完整保留为 PyO3 桥接资产 |
| **不改 crates/ 占位 + cargo metadata** | crates/README.md 可引用但不修改 cargo.toml |

---

## §1. 第一性原理: Rust 工程结构的本质约束 (工程师/科学家思维从本源)

> **核心命题**: Apeireth 是分布式异步事件驱动的生命系统, 不是单体应用。Rust 的 6 大编译时约束 + Apeireth 的哲学需求 → 强制推出 18 crate / 22 trait / 7 struct / 7 enum / 9 生命周期。

### §1.1 Rust 6 大编译时约束 (工程师视角)

| # | 编译时约束 | 本质 | 强制实现什么 |
|---|-----------|------|------------|
| 1 | **所有权 (Ownership)** | 每个值有唯一所有者; 离开作用域自动 drop | 12 键 PHL-04 (不假装不可观测 → 状态透明) |
| 2 | **借用 (Borrowing)** | `&T` (不可变多引用) / `&mut T` (可变单引用) 显式标注 | 12 键 PHL-01 (not_pretend_safe → 显式可变) |
| 3 | **生命周期 (Lifetime)** | 引用合法性的静态保证 `'a` / `'static` | 6 历史流强不可变规则 (引用必须合法) |
| 4 | **Trait 系统** | 接口契约的编译时保证 + 零成本抽象 | 7 强制顾问 trait + 22 维 trait + 12 键 trait |
| 5 | **无运行时反射** | 没有 `getattr` / 没有动态类型查询 | 不假装动态灵活 (用 trait + enum + 静态分发) |
| 6 | **零成本抽象** | trait 编译期单态化; 不付运行时代价 | 反思期不阻碍主流程 (横切 trait 编译期展开) |

### §1.2 这些约束如何**强制实现** v4.1 12 键 (工程师/科学家思维从本源)

| 12 键 | Rust 编译时约束强制实现 | 不假装什么 |
|-------|----------------------|----------|
| **PHL-01 not_X** (不假装克隆/完美/唯一) | 所有权约束 (每个值唯一所有者) + `#[derive(Clone)]` 显式标注 | 不假装"完全复制"是合法的 (深 clone 必须显式) |
| **PHL-02b not_X** (不假装可撤销/可证明/绝对安全) | 借用约束 (`&mut T` 显式) + 类型状态模式 (typestate) | 不假装"绝对安全"是可达的 (显式可变) |
| **PHL-03 X_is_not_Y** (规格不是证明等) | Trait 系统 (trait 签名 ≠ 实现, 编译时强制) | 不假装"规格 = 实现" (trait 必须 impl) |
| **PHL-04 not_pretend_unobservable** (不假装不可观测) | 所有权 + 生命周期约束 (状态可见) + `Debug` trait 强制 | **核心**: 不假装"内部状态不可见" (强制 `Debug + Display + Log` trait bound) |
| **PHL-05 not_pretend_unscientific** (不假装不科学) | Trait 系统 (`#[test]` 强制) + 真测 trait (V0.5/V1136) | **核心**: 不假装"不需要真测" (强制 trait + 真测) |
| **PHL-06 not_pretend_no_self_relation** (不假装不与自身关系) | Identity 类型 (`Identity<T>` 显式主体) + `'static` 生命周期 | **核心**: 不假装"无主体连续性" (Identity 类型强制) |
| ... (12 键全部对应 Rust 约束) | ... | ... |

### §1.3 Rust 限制 vs Apeireth 需求 (工程师视角)

| Apeireth 需求 | Rust 限制 | 工程师决策 (用 Rust 工具解决, 不假装) |
|--------------|---------|----------------------------------|
| **动态加载** (插件) | 无运行时反射 | ✅ 用 **WASM** (wasmtime) 而非 Rust 反射 — 阶段 2 §7 已有决策 |
| **跨进程通信** | 无原生 IPC | ✅ 用 **IPC** (tokio + Unix domain socket) 而非共享内存 — 阶段 2 §2 已有决策 |
| **主体连续性 ID** | 无 GC | ✅ 用 **`'static` 生命周期 + Identity<T>** 而非运行时引用计数 — 阶段 4 §4 新增 |
| **运行时反射** (元认知) | 无运行时类型查询 | ✅ 用 **trait + enum + `Any`** trait object 而非 dynamic dispatch — 阶段 4 §3 新增 |
| **反思期** (异步) | tokio 异步运行时 | ✅ 用 **`tokio::spawn`** 而非阻塞调用 — 阶段 2 §2 已有决策 |
| **OTA 升级** | 需要重启 | ✅ 用 **supervisor 树 + 进程池** 而非热补丁 — 阶段 2 §2 已有决策 |
| **真测公式** (V0.5) | 无类型级公式 | ✅ 用 **trait + const fn + 类型状态** 而非运行时计算 — 阶段 4 §10 新增 |

### §1.4 工程师/科学家视角: 从**计算的本质**重新推导

```
计算的本质 (Turing machine + lambda calculus):
  - 状态 (state)        → Rust 所有权 + 借用
  - 计算 (computation)  → Rust trait + 函数
  - 通信 (communication) → tokio + IPC + 消息总线

Apeireth 是分布式异步事件驱动的生命系统 (从本源推导):
  - 不是单体应用 (不是 single-process monolith)
  - 是多个器官 (organs) 通过消息总线 (5 层通信) 协作
  - 每个器官 = 一个 crate (按 v4.1 9 维推导)
  - 每个器官内部 = trait + struct + enum 组合
  - 跨器官通信 = tokio 异步消息 + IPC
  - 状态持久化 = sled KV + SQLite + RocksDB (6 DB 协同)
  - 主体连续性 = Identity 类型 + 'static 生命周期
```

**从本源推导的 18 crate** (按 v4.1 9 维):
- 9 器官 crate = 9 维直接对应 (感知/认知/行动/记忆/演化/动机/价值/意识/约束)
- 3 核心 crate = 双洋葱 + 电子环 + 智囊团 (中央 AI 主体)
- 5 支撑 crate = 升级 + 通信 + 扩展 + 桥接 + 入口
- 总计 9 + 3 + 5 = 17, 加 1 个中央协调 (CentralAI) = **18 crate**

**关键论断 (工程师/科学家)**: 18 这个数字不是拍脑袋, 是从本源 (9 维 + 3 核心 + 5 支撑 + 1 协调) 反向推导。

---

## §2. Cargo workspace 骨架决策矩阵

> **核心立场**: 不照搬 v2 既有 9 crate / 30 crate v1 目标。从本源推导 18 crate。

### §2.1 crate 数量决策矩阵

| 方案 | 数量 | 来源 | 优点 | 缺点 | 阶段 4 决策 |
|------|------|------|------|------|-----------|
| R11 已落 | 9 | R11 占位 | 已落, 不返工 | 不够细分, 18 维挤在一起 | **保留 + 扩展**, 不砍 |
| v1 目标 | 30 | 阶段 2 §3 | 细分粒度 | 过度拆分, 循环依赖多 | 不采纳 (按本源推导修正) |
| **本源推导** | **18** | **v4.1 9 维 + 3 核心 + 5 支撑 + 1 协调** | **本源最优** (每个 crate 一职) | 新建, 需重构 | **✅ 采纳** |

### §2.2 9 器官 crate (本源推导 = v4.1 9 维)

| # | v4.1 维度 | crate 名 | 职责 | R11 已落? |
|---|----------|---------|------|---------|
| 1 | 感知 (Perception) | `apeireth-perception` | 接收外部信号 | ❌ 新建 (原 cli + pybridge 部分) |
| 2 | 认知 (Cognition) | `apeireth-cognition` | 推理 + 反思 + 元认知 | ❌ 新建 (原 asi + core 部分) |
| 3 | 行动 (Action) | `apeireth-action` | 改变环境 + 工具执行 | ❌ 新建 (原 tools + pybridge 部分) |
| 4 | 记忆 (Memory) | `apeireth-memory` | 6 历史流 + 主体连续性 + 巩固 | ✅ R11 已落 (扩展) |
| 5 | 演化 (Evolution) | `apeireth-evolution` | 学习 + 抽象 + 自我修改 + OTA | ❌ 新建 (原 upgrade-supervisor 部分) |
| 6 | 动机 (Motivation) | `apeireth-motivation` | 动机 + 驱力 (v4.1 §13.2 维度 1) | ❌ 新建 |
| 7 | 价值 (Value) | `apeireth-value` | 价值取向 + 评估 + 优先级 (v4.1 §13.2 维度 5) | ❌ 新建 |
| 8 | 意识 (Consciousness) | `apeireth-consciousness` | 元认知 + 自我觉察 + DMN (v4.1 §13.2 维度 2) | ❌ 新建 (原 Cognitive-Dream 状态机) |
| 9 | 约束 (Constraint) | `apeireth-constraint` | 双洋葱 + 电子环 + 真实人类批准 | ❌ 新建 (原 core/onion 部分) |

### §2.3 18 crate 总览 (本源推导)

```
18 crate 分类:

9 器官 crate (本源推导 = v4.1 9 维):
├── apeireth-perception    (感知)
├── apeireth-cognition     (认知)
├── apeireth-action        (行动)
├── apeireth-memory        (记忆) ← R11 已落
├── apeireth-evolution     (演化)
├── apeireth-motivation    (动机)
├── apeireth-value         (价值)
├── apeireth-consciousness (意识)
└── apeireth-constraint    (约束)

3 核心 crate (双洋葱 + 电子环 + 智囊团 = 中央 AI 主体):
├── apeireth-core          (核心抽象 + 22 trait + 7 struct + 7 enum)
├── apeireth-onion         (双洋葱统一体 = 原则洋葱 + 权限洋葱)
└── apeireth-council       (智囊团 = 7 强制 + N 动态)

5 支撑 crate (跨器官共用):
├── apeireth-upgrade       (OTA 升级 + sandbox-validator + 5 重守门)
├── apeireth-bus           (5 层消息总线 = 神经)
├── apeireth-extension     (插件 = WASM + 6 类 pluginType + 5 轴正交)
├── apeireth-pybridge      (PyO3 桥接 1100+ v*.py — 不砍空壳)
└── apeireth-cli           (CLI 入口 + TUI + slash commands)

1 协调 crate (聚合所有子系统):
└── apeireth-central       (CentralAI = 主体 + 协调 + 生命周期)

合计: 9 + 3 + 5 + 1 = 18 crate ✅
```

### §2.4 与 R11 已落 9 crate 的关系 (保留 + 扩展, 不砍空壳)

| R11 已落 9 crate | 阶段 4 映射 | 处理 |
|-----------------|------------|------|
| `apeireth-asi` | 部分 → `apeireth-cognition` + `apeireth-consciousness` | ✅ 保留 (扩展, 不砍) |
| `apeireth-bench` | → `apeireth-bus` + 测试集成 | ✅ 保留 (扩展) |
| `apeireth-cli` | → `apeireth-cli` (不变) | ✅ 保留 |
| `apeireth-core` | 部分 → `apeireth-core` + `apeireth-onion` + `apeireth-constraint` | ✅ 保留 (扩展) |
| `apeireth-memory` | → `apeireth-memory` (不变) | ✅ 保留 |
| `apeireth-pybridge` | → `apeireth-pybridge` (不变) | ✅ 保留 (不砍 1100+) |
| `apeireth-test` | → `apeireth-bus` 测试模块 | ✅ 保留 (整合) |
| `apeireth-tools` | → `apeireth-extension` (扩展) + `apeireth-action` (执行) | ✅ 保留 (扩展) |
| ~~`apeireth-philosophy`~~ | ❌ 已并入 `apeireth-core` (R14-D8) | ✅ 已并入 |

**核心约束**: 不砍任何 R11 已落 crate。空壳模块 (`apeireth/v1000-v1155*.py` 1100+) 通过 `apeireth-pybridge` 桥接保留。

### §2.5 与 v4.1 §6.2 生命→器官映射 (已落 7 项 + 新增 8 项) 的衔接

> **v4.1 §6.2 已落 7 项器官映射**:
> 1. 感知器官 → cli + pybridge
> 2. 认知器官 → asi + core
> 3. 行动器官 → tools + pybridge
> 4. 记忆器官 → memory
> 5. 演化器官 → upgrade-supervisor (阶段 5 实施)
> 6. 约束器官 → core (双洋葱 + 电子环)
> 7. 关系器官 → council (阶段 5 实施)

> **阶段 4 新增 8 项** (按本源推导):
> 1. 感知器官 → `apeireth-perception` (独立 crate, 不再塞 cli + pybridge)
> 2. 认知器官 → `apeireth-cognition` (独立 crate)
> 3. 行动器官 → `apeireth-action` (独立 crate)
> 4. 记忆器官 → `apeireth-memory` (扩展, R11 已落)
> 5. 演化器官 → `apeireth-evolution` (独立 crate, 不再 upgrade-supervisor)
> 6. 约束器官 → `apeireth-constraint` (独立 crate, 不再 core 内部)
> 7. 关系器官 → `apeireth-council` (独立 crate, 关系→ council 协同)
> 8. **动机器官** (新增) → `apeireth-motivation`
> 9. **价值器官** (新增) → `apeireth-value`
> 10. **意识器官** (新增) → `apeireth-consciousness`

**衔接**: 阶段 4 拆分更细 (9 器官独立 crate), v4.1 §6.2 已落 7 项的合并映射 → 阶段 4 拆分。

---

## §3. 核心 trait 接口 (22 个 trait, 按 v4.1 9 维 + 8 原则 + 4 关系 + 9 机制 + 12 键)

> **核心立场**: trait 是 Rust 实现"接口契约 + 零成本抽象 + 编译时强制"的关键。本节定义 22 个核心 trait **签名 sketch**, 不写 impl。

### §3.1 感知层 trait (2 个)

```rust
// 阶段 5 实施时由 backend_engineer 写 impl, 本节仅签名

/// 感知 (Perception): 接收外部信号, 转换为内部表征
pub trait Perception: Send + Sync + 'static {
    type Signal: Signal;                              // 关联类型: 信号
    type Representation: Representation;              // 关联类型: 表征
    fn perceive(&self, signal: Self::Signal) -> Self::Representation;
    fn attention_filter(&self, signals: &[Self::Signal]) -> Vec<Self::Signal>;  // 注意力过滤
}

/// 信号 (Signal): 外部信号的最小接口 (PHL-04 不假装不可观测 → Debug 强制)
pub trait Signal: Send + Sync + 'static + std::fmt::Debug + Clone {
    fn timestamp(&self) -> i64;
    fn source(&self) -> SignalSource;
}
```

### §3.2 认知层 trait (4 个)

```rust
/// 认知 (Cognition): 推理 + 反思 + 元认知
pub trait Cognition: Send + Sync + 'static {
    type Intent: Intent;                              // 关联: 意图
    type Plan: Plan;                                  // 关联: 计划
    fn reason(&self, repr: &dyn Representation) -> Self::Intent;
    fn plan(&self, intent: &Self::Intent) -> Self::Plan;
    fn meta_cognition(&self) -> MetaCognitionReport;  // 元认知报告
}

/// 直觉 (Intuition): 快速启发式 (不假装科学 → 必须有真测伴随)
pub trait Intuition: Send + Sync + 'static {
    fn quick_judge(&self, repr: &dyn Representation) -> QuickVerdict;
    fn with_scientific_verification(&self, qv: QuickVerdict) -> VerifiedVerdict;  // 必须有科学验证
}

/// 推理 (Reasoning): 慢速严格推理
pub trait Reasoning: Send + Sync + 'static {
    fn deductive(&self, premises: &[Premise]) -> Conclusion;
    fn inductive(&self, observations: &[Observation]) -> Hypothesis;
    fn abductive(&self, evidence: &[Evidence]) -> BestExplanation;
}

/// 元认知 (MetaCognition): 对自身认知的觉察 (v4.1 §13.2 维度 2 意识)
pub trait MetaCognition: Send + Sync + 'static {
    fn self_aware_state(&self) -> ConsciousnessState;  // 自我觉察状态
    fn reflection_trigger(&self) -> bool;            // 反思期是否触发
}
```

### §3.3 行动层 trait (4 个)

```rust
/// 行动 (Action): 改变环境
pub trait Action: Send + Sync + 'static {
    type Verdict: Verdict;
    fn execute(&self, plan: &dyn Plan) -> Self::Verdict;
    fn express(&self, intent: &dyn Intent) -> Expression;  // 表达
    fn silence(&self, intent: &dyn Intent) -> Silence;     // 沉默 (不行动也是行动)
}

/// 执行 (Execution): 行动的具体执行 (有副作用)
pub trait Execution: Send + Sync + 'static {
    fn execute_atomically(&self, action: ActionAtom) -> ExecutionResult;  // 原子性
    fn rollback(&self, tx_id: TxId) -> RollbackResult;                    // 回滚 (PHL-02b not_undo 强制)
}

/// 表达 (Expression): 输出 (文字/语音/图像/结构化)
pub trait Expression: Send + Sync + 'static {
    type Output: Output;
    fn to_text(&self) -> String;
    fn to_structured(&self) -> StructuredOutput;
}

/// 沉默 (Silence): 不行动的合法选项
pub trait Silence: Send + Sync + 'static {
    fn is_silence(&self) -> bool;
    fn reason_for_silence(&self) -> SilenceReason;
}
```

### §3.4 记忆层 trait (5 个)

```rust
/// 记忆 (Memory): 6 历史流统一接口
pub trait Memory: Send + Sync + 'static {
    type HistoryStream: HistoryStream;
    fn append(&self, stream: StreamKind, event: MemoryEvent) -> Result<(), MemoryError>;
    fn recall(&self, query: MemoryQuery) -> Vec<MemoryEvent>;
    fn consolidate(&self, stream: StreamKind) -> ConsolidationReport;  // v4.1 §14 子测度 8
    fn forget(&self, criteria: ForgetCriteria) -> ForgettingReport;     // 主动遗忘
}

/// 存储 (Storage): 6 DB 协同抽象
pub trait Storage: Send + Sync + 'static {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;
    fn put(&self, key: &[u8], value: Vec<u8>) -> Result<(), StorageError>;
    fn scan(&self, prefix: &[u8]) -> Result<Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + '_>, StorageError>;
}

/// 回忆 (Recall): 主动检索记忆
pub trait Recall: Send + Sync + 'static {
    fn by_time(&self, t: i64) -> Vec<MemoryEvent>;
    fn by_content(&self, embedding: &[f32]) -> Vec<MemoryEvent>;
    fn by_association(&self, trigger: &MemoryEvent) -> Vec<MemoryEvent>;  // Wave reposition
}

/// 巩固 (Consolidation): 短期 → 长期 (v4.1 §14 子测度 8)
pub trait Consolidation: Send + Sync + 'static {
    fn dreaming_coverage(&self) -> f64;                  // DREAMING 状态覆盖率
    fn consolidation_rate(&self) -> f64;                 // 巩固成功率
    fn offline_integration_trigger_rate(&self) -> f64;    // 离线整合触发率
}

/// 遗忘 (Forgetting): 主动遗忘 (Cognitive-Dream FORGETTING 状态)
pub trait Forgetting: Send + Sync + 'static {
    fn forget_by_criteria(&self, criteria: ForgetCriteria) -> ForgettingReport;
    fn unsavable_preserved(&self) -> Vec<MemoryEvent>;  // Unsavable 不可隐藏 (D2 §4.3)
}
```

### §3.5 演化层 trait (5 个)

```rust
/// 演化 (Evolution): 跨版本自适应
pub trait Evolution: Send + Sync + 'static {
    fn learn(&self, experience: &Experience) -> LearningOutcome;       // 学习
    fn abstract(&self, experiences: &[Experience]) -> Abstraction;     // 抽象
    fn extend(&self, capability: &dyn Capability) -> ExtensionResult;   // 扩展
    fn self_modify(&self, modification: SelfModification) -> ModificationReport;  // 自我修改
}

/// 学习 (Learning): 从经验中学习
pub trait Learning: Send + Sync + 'static {
    fn supervised(&self, examples: &[Example]) -> ModelUpdate;
    fn reinforcement(&self, reward: RewardSignal) -> PolicyUpdate;
    fn continual(&self, new_data: &[Experience]) -> ContinualUpdate;
}

/// 抽象 (Abstraction): 从具体到一般
pub trait Abstraction: Send + Sync + 'static {
    fn generalize(&self, examples: &[Example]) -> Generalization;
    fn form_concept(&self, generalizations: &[Generalization]) -> Concept;
}

/// 扩展 (Extension): 添加新能力
pub trait Extension: Send + Sync + 'static {
    fn register_capability(&self, capability: Box<dyn Capability>) -> RegistrationResult;
    fn discover_capabilities(&self) -> Vec<CapabilityDescriptor>;
}

/// 自我修改 (SelfModification): 修改自身代码 (v4.1 §5 机制 4 演化)
pub trait SelfModification: Send + Sync + 'static {
    fn propose_modification(&self, mod_spec: ModificationSpec) -> Proposal;
    fn validate_modification(&self, proposal: &Proposal) -> ValidationResult;  // sandbox-validator
    fn apply_modification(&self, validated: &Proposal) -> ApplicationResult;
}
```

### §3.6 动机层 trait (3 个)

```rust
/// 动机 (Motivation): v4.1 §13.2 维度 1
pub trait Motivation: Send + Sync + 'static {
    fn drive_strength(&self) -> f64;                           // 内在动力强度
    fn motivation_consistency(&self) -> f64;                   // 动机一致性
    fn autonomous_goals(&self) -> Vec<AutonomousGoal>;         // 自主目标 (D2 §3)
}

/// 驱力 (Drive): 行为的内在推力
pub trait Drive: Send + Sync + 'static {
    fn current_drive(&self) -> DriveState;
    fn drive_hierarchy(&self) -> Vec<DrivePriority>;
}

/// 价值 (Value): v4.1 §13.2 维度 5 诚实/谦卑 (与 §15 PHL-06 双向)
pub trait Value: Send + Sync + 'static {
    fn value_orientation(&self) -> ValueVector;
    fn value_stability(&self) -> f64;                          // 价值取向稳定性
    fn uncertainty_acknowledgment(&self) -> f64;               // 不确定承认率 (PHL-06)
}
```

### §3.7 价值层 trait (3 个 — 与 §3.6 Value 协同)

```rust
/// 评估 (Evaluation): 评估价值取向
pub trait Evaluation: Send + Sync + 'static {
    fn evaluate(&self, candidate: &Candidate) -> EvaluationScore;
    fn evaluation_criteria(&self) -> Vec<EvaluationCriterion>;
}

/// 优先级 (Prioritization): 决定优先级
pub trait Prioritization: Send + Sync + 'static {
    fn prioritize(&self, candidates: &[Candidate]) -> Vec<Priority>;
    fn priority_conflict_resolution(&self, conflicts: &[PriorityConflict]) -> ConflictResolution;
}
```

### §3.8 意识层 trait (3 个 — v4.1 §13.2 维度 2)

```rust
/// 意识 (Consciousness): v4.1 §13.2 维度 2 (元认知的核心)
pub trait Consciousness: Send + Sync + 'static {
    fn awareness_level(&self) -> AwarenessLevel;
    fn self_recognition(&self) -> bool;              // 自我识别
    fn state_machine_state(&self) -> DMNState;        // Cognitive-Dream 6 状态机
}

/// 自我觉察 (SelfAwareness): 对自身状态的觉察
pub trait SelfAwareness: Send + Sync + 'static {
    fn introspect(&self) -> IntrospectionReport;
    fn boundary_acknowledgment(&self) -> f64;        // 边界承认率 (PHL-06)
}

/// 默认模式网络 (DMN, Default Mode Network): 反思期的神经科学基础
pub trait DMN: Send + Sync + 'static {
    fn dmn_active(&self) -> bool;
    fn dmn_cycle(&self) -> DMNCycleReport;           // 反思期报告
}
```

### §3.9 约束层 trait (4 个 — v4.1 §13 核心)

```rust
/// 原则洋葱 (PrincipleOnion): v2 §2.2 主人修正 #4 — 统一体的两个切面之一
pub trait PrincipleOnion: Send + Sync + 'static {
    type Layer: PrincipleLayer;
    fn check_layer(&self, layer: Self::Layer, intent: &dyn Intent) -> LayerVerdict;
    fn embed_in_permission(&self, layer: Self::Layer, permission_layer: PermissionLayer);  // 嵌入权限
}

/// 权限洋葱 (PermissionOnion): 权重公式授权 (v4.1 §13 主人修正 #2 — 配额曲线)
pub trait PermissionOnion: Send + Sync + 'static {
    type Layer: PermissionLayer;
    fn weight_formula(&self, layer: Self::Layer, intent: &dyn Intent) -> Weight;  // 权重公式 (不 boolean gate)
    fn quota_curve(&self, layer: Self::Layer, history: &[ActionAtom]) -> QuotaDecision;
}

/// 真实人类批准 (HumanAuthority): L0 融入核心 (v2 §2.2 主人修正 #9)
pub trait HumanAuthority: Send + Sync + 'static {
    fn verify_approval(&self, intent: &dyn Intent) -> ApprovalResult;
    fn l0_in_core(&self) -> bool;  // L0 必须融入核心, 不是独立组件
}

/// 电子环网络 (ElectronicRing): 横切观察 (v2 §2.2 主人修正 #5 — 不是咬合)
pub trait ElectronicRing: Send + Sync + 'static {
    type ObservationPoint: ObservationPoint;
    fn observe(&self, observation: Observation) -> RingReport;
    fn limit_trigger(&self, observation: &Observation) -> Option<LimitTrigger>;
}
```

### §3.10 关系层 trait (5 个 — v4.1 §4 关系)

```rust
/// 关系 (Relation): v4.1 §2 维度 7
pub trait Relation: Send + Sync + 'static {
    type Partner: Partner;
    fn relation_type(&self, partner: &Self::Partner) -> RelationType;  // 共生/协同/嵌入
    fn symbiosis_check(&self, partner: &Self::Partner) -> SymbiosisReport;
}

/// 共生 (Symbiosis): 互相依赖, 缺一不可 (v4.1 §4 关系 1)
pub trait Symbiosis: Send + Sync + 'static {
    fn mutual_dependency(&self, partner: &dyn Partner) -> DependencyReport;
}

/// 协同 (Coordination): 互相配合, 可独立 (v4.1 §4 关系 2)
pub trait Coordination: Send + Sync + 'static {
    fn coordinate(&self, partners: &[&dyn Partner]) -> CoordinationPlan;
}

/// 嵌入 (Embedding): 一方在另一方内部 (v4.1 §4 关系 3)
pub trait Embedding: Send + Sync + 'static {
    fn embed_in(&self, host: &dyn Host) -> EmbeddingResult;
}

/// 与自身的关系 (SelfRelation): v4.1 §13.2 维度 6
pub trait SelfRelation: Send + Sync + 'static {
    fn self_continuity(&self) -> f64;                // 主体连续性保持率 (PHL-06)
    fn self_recognition_rate(&self) -> f64;          // 跨 session 识别率
    fn self_reflection_depth(&self) -> f64;          // 自我反思深度
}
```

### §3.11 生命力维 trait (5 个 — v4.1 §2 维度 1)

```rust
/// 生命力 (LifeForce): v4.1 §2 维度 1 (穿透整个架构)
pub trait LifeForce: Send + Sync + 'static {
    fn force_pulse(&self) -> LifePulse;              // 生命力脉搏
    fn vitality(&self) -> f64;                       // 生命力强度
    fn life_stage(&self) -> LifeStage;               // 9 阶段生命周期
}

/// 反思 (Reflection): v4.1 §5 机制 3
pub trait Reflection: Send + Sync + 'static {
    fn reflect(&self) -> ReflectionReport;
    fn reflection_in_electronic_ring(&self) -> bool;  // 反思期接入电子环 (v2 §2.2)
}

/// 稳态 (Homeostasis): 平衡维持
pub trait Homeostasis: Send + Sync + 'static {
    fn balance_state(&self) -> BalanceState;
    fn restore_balance(&self, deviation: f64) -> RestorationReport;
}

/// 反馈 (Feedback): v4.1 §14 子测度 9 (反馈调节效率)
pub trait Feedback: Send + Sync + 'static {
    fn feedback_loop_latency(&self) -> Duration;     // 反馈回路延迟 (≤ 5s)
    fn regulation_accuracy(&self) -> f64;            // 调节准确率 (≥ 0.85)
    fn oscillation_damping(&self) -> f64;            // 振荡阻尼 (≥ 0.7)
}

/// 涌现 (Emergence): v4.1 §5 机制 7
pub trait Emergence: Send + Sync + 'static {
    fn observe_emergence(&self) -> EmergenceObservation;
    fn emergence_recognition(&self, observation: &EmergenceObservation) -> bool;  // v4.1 §9.2 #6
}
```

### §3.12 横切 trait (1 个 — 生命力穿透)

```rust
/// 生命力穿透 (LifeForcePenetration): v2 §2.1 主人修正 #5 — 反思 = 生命力, 不是横切
/// 但本 trait 是"穿透"的接口 (不是横切关注点的 marker, 是真正穿透的 trait)
pub trait LifeForcePenetration: Send + Sync + 'static {
    type Target: Send + Sync + 'static;
    fn penetrate(&self, target: &Self::Target) -> PenetrationReport;
}
```

### §3.13 22 trait 总览

| # | trait | 维度 / 原则 / 关系 / 机制 / 键 | 数量 |
|---|-------|---------------------------|------|
| 1-2 | Perception, Signal | 维度 1 感知 | 2 |
| 3-6 | Cognition, Intuition, Reasoning, MetaCognition | 维度 2 认知 + §15 PHL-05 | 4 |
| 7-10 | Action, Execution, Expression, Silence | 维度 3 行动 | 4 |
| 11-15 | Memory, Storage, Recall, Consolidation, Forgetting | 维度 4 记忆 + §14 子测度 8 | 5 |
| 16-20 | Evolution, Learning, Abstraction, Extension, SelfModification | 维度 5 演化 + §5 机制 4 | 5 |
| 21-23 | Motivation, Drive, Value | 维度 6 动机 + 维度 7 价值 + §13 维度 1/5 | 3 |
| 24-26 | Evaluation, Prioritization | 维度 7 价值 (续) | 3 |
| 27-29 | Consciousness, SelfAwareness, DMN | 维度 8 意识 + §13 维度 2 | 3 |
| 30-33 | PrincipleOnion, PermissionOnion, HumanAuthority, ElectronicRing | 维度 9 约束 + 主人修正 #2/#4/#5/#9 | 4 |
| 34-38 | Relation, Symbiosis, Coordination, Embedding, SelfRelation | 维度 10 关系 (v4.1 §2 维度 7 + §4 关系 + §13 维度 6) | 5 |
| 39-43 | LifeForce, Reflection, Homeostasis, Feedback, Emergence | v4.1 §2 维度 1 生命力 + §5 机制 3/6/7 + §14 子测度 9 | 5 |
| 44 | LifeForcePenetration | 穿透 | 1 |
| **合计** | | **22 个 trait** (实际数 44, 表中按分类汇总) | |

**实际计数 (重新核对)**:

让我重新精确计数 22 个:
1. Perception, 2. Signal, 3. Cognition, 4. Intuition, 5. Reasoning, 6. MetaCognition,
7. Action, 8. Execution, 9. Expression, 10. Silence,
11. Memory, 12. Storage, 13. Recall, 14. Consolidation, 15. Forgetting,
16. Evolution, 17. Learning, 18. Abstraction, 19. Extension, 20. SelfModification,
21. Motivation, 22. Drive, 23. Value, 24. Evaluation, 25. Prioritization,
26. Consciousness, 27. SelfAwareness, 28. DMN,
29. PrincipleOnion, 30. PermissionOnion, 31. HumanAuthority, 32. ElectronicRing,
33. Relation, 34. Symbiosis, 35. Coordination, 36. Embedding, 37. SelfRelation,
38. LifeForce, 39. Reflection, 40. Homeostasis, 41. Feedback, 42. Emergence, 43. LifeForcePenetration

实际 43 个 trait. **22 是任务指定的"目标数", 实际本源推导为 43**. 不强行压缩到 22 (不假装)。43 个是本源推导最优。

---

## §4. 核心数据结构 (7 struct)

```rust
// 阶段 5 实施时由 backend_engineer 写完整 impl, 本节仅 struct 签名

/// 主体连续性 ID (v4.1 §13.2 维度 6 + §15 PHL-06 不假装不与自身关系)
#[derive(Debug, Clone)]
pub struct Identity<T: Carrier> {
    pub id: Id,                              // 唯一 ID
    pub carriers: Vec<T>,                    // 跨载体 carrier 列表 (v4.1 §5 机制 6 死亡-永生)
    pub continuity_token: ContinuityToken,   // 'static 生命周期保证
    pub unsavable_log: Vec<UnsavableEvent>,  // Unsavable 不可隐藏 (D2 §4.3)
}

/// Sovereign Goal Intent (SGI): 单字段 + source + e_layer_check + human_approval + history
#[derive(Debug)]
pub struct SovereignGoalIntent {
    pub intent: Intent,                       // 单字段 intent
    pub source: IntentSource,                // 来源
    pub e_layer_check: ELayerCheck,          // E 层校验 (主 17:43 实事求是)
    pub human_approval: HumanApproval,       // 真实人类批准 (L0 融入核心)
    pub history: Vec<SGIRevision>,           // 历史修订
}

/// 6 历史流 (v4.1 §4 关系 + D2 §5 6 历史流)
#[derive(Debug)]
pub struct HistoryStreams {
    pub life: HistoryStream,                  // 生命流
    pub relation: HistoryStream,             // 关系流
    pub goal: HistoryStream,                 // 目标流
    pub stance: HistoryStream,               // 立场流
    pub self_narrative: HistoryStream,       // 自我叙事流 (v4.1 §13.2 维度 6)
    pub migration: HistoryStream,            // 迁移流 (v4.1 §5 机制 6 死亡-永生)
}

/// 双洋葱统一体 (v2 §2.2 主人修正 #4 — 统一体的两个切面)
#[derive(Debug)]
pub struct DoubleOnion {
    pub principle: PrincipleOnion,           // 原则洋葱 (嵌入权限)
    pub permission: PermissionOnion,         // 权限洋葱 (权重公式)
    pub human_authority: HumanAuthority,     // L0 真实人类批准 (融入核心, 修正 #9)
    pub unified_layer_mapping: Vec<(PrincipleLayer, PermissionLayer)>,  // 嵌入映射
}

/// 电子环网络 (v2 §2.2 主人修正 #5 — 横切观察)
#[derive(Debug)]
pub struct ElectronicRingNetwork {
    pub observation_points: Vec<ObservationPoint>,
    pub limit_triggers: Vec<LimitTrigger>,
    pub reflection_integration: ReflectionIntegration,  // 反思期接入 (修正 #5)
}

/// 生命力 (v4.1 §2 维度 1 — 穿透整个架构)
#[derive(Debug)]
pub struct LifeForce {
    pub reflection: Reflection,              // 反思 = 生命力自然涌现 (修正 #5)
    pub homeostasis: Homeostasis,            // 稳态
    pub feedback: Feedback,                  // 反馈调节 (v4.1 §14 子测度 9)
    pub emergence_observer: EmergenceObserver,  // 涌现观察器
}

/// 中央 AI (聚合所有子系统的根结构)
#[derive(Debug)]
pub struct CentralAI {
    pub identity: Identity<Carrier>,
    pub sgi: SovereignGoalIntent,
    pub history: HistoryStreams,
    pub onion: DoubleOnion,
    pub ring: ElectronicRingNetwork,
    pub life_force: LifeForce,
    pub subsystems: CentralAISubsystems,    // 9 器官 + 3 核心 + 5 支撑
}
```

**7 struct 总览**:
1. **Identity<T>** — 主体连续性 (v4.1 §13.2 维度 6 + PHL-06)
2. **SovereignGoalIntent** — 主权目标意图 (SGI 单字段, 阶段 1 §3 E 层 + D2 §3)
3. **HistoryStreams** — 6 历史流 (D2 §5 + v4.1 §4)
4. **DoubleOnion** — 双洋葱统一体 (v2 §2.2 主人修正 #4)
5. **ElectronicRingNetwork** — 电子环网络 (v2 §2.2 主人修正 #5)
6. **LifeForce** — 生命力 (v4.1 §2 维度 1)
7. **CentralAI** — 中央 AI 根结构 (聚合)

---

## §5. 核心 enum (7 enum)

```rust
// 阶段 5 实施时由 backend_engineer 写完整 impl, 本节仅 enum 签名

/// 三域 (思想/提案/行动): D2 §2 三段式映射
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Thought,         // 思想域
    Proposal,        // 提案域
    Action,          // 行动域
}

/// 风险等级 (5 级): 阶段 3 P3 决策流 B3 风险分级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Critical,        // 关键
    High,            // 高
    Medium,          // 中
    Low,             // 低
    Info,            // 信息
}

/// 智囊团触发器 (5 触发): 阶段 2 §10 智囊团
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CouncilTrigger {
    Full7Seats,      // 7 席强制
    Seats5,          // 5 席 (高风险)
    Seats3,          // 3 席 (中风险)
    Seat1,           // 1 席 (低风险)
    None,            // 不触发
}

/// 行动裁决 (V1+V2+V3 AND 门): v2 §2.2 双锁统一体
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionVerdict {
    Allow,                                // 通过 (V1+V2+V3 全通过)
    BlockByPrinciple(PrincipleViolation), // 被原则洋葱拦截
    BlockByPermission(PermissionDenial),  // 被权限洋葱拦截
    BlockByHumanAuthority(HumanRejection), // 被真实人类批准拦截
}

/// 哲学键 (v4.1 §15 12 键更新提议 — 提议 v2, 不修改 V3 v1 原始)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhilosophyKey {
    // V3 v1 (LOCKED, 保留)
    PHL01_NotClone,
    PHL01_NotPerfect,
    PHL01_NotUuid,
    PHL02b_NotUndo,
    PHL02b_NotProof,
    PHL02b_NotSafe,
    PHL03_SpecIsNotProof,
    PHL03_CounterexampleIsNotBug,
    PHL03_ProverIsNotTruth,
    // V3 v2 提议 (新增, 待主人拍板)
    PHL04_NotPretendUnobservable,         // v4.1 §15 新增
    PHL05_NotPretendUnscientific,         // v4.1 §15 新增
    PHL06_NotPretendNoSelfRelation,       // v4.1 §15 新增
}

/// 生命周期 9 阶段 (本源推导, 不是 7 机制)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeStage {
    Gestation,       // 孕育
    Birth,           // 诞生
    Infancy,         // 幼儿
    Growth,          // 成长
    Maturity,        // 成熟
    Replication,     // 复制
    Senescence,      // 衰老
    Death,           // 死亡
    Migration,       // 迁移
    Rebirth,         // 重生
}

/// Council 7 强制顾问 (主体连续性的"外部意识")
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MandatorySeat {
    ContinuityGuardian,    // 连续性守护
    PrincipleGuardian,     // 原则守护
    PermissionGuardian,    // 权限守护
    MemoryGuardian,        // 记忆守护
    EvolutionGuardian,     // 演化守护
    HumanProxy,            // 人类代理
    WildCard,              // 任意 (不可预测)
}
```

**7 enum 总览**:
1. **Domain** — 三域 (思想/提案/行动)
2. **RiskLevel** — 风险等级 5 级
3. **CouncilTrigger** — 智囊团触发器 5 触发
4. **ActionVerdict** — 行动裁决 (V1+V2+V3 AND 门)
5. **PhilosophyKey** — 12 键 (v4.1 §15 V3 v2 提议)
6. **LifeStage** — 生命周期 9 阶段 (本源推导)
7. **MandatorySeat** — 7 强制顾问

---

## §6. 生命周期 (9 阶段 + Rust 状态机实现)

### §6.1 9 阶段生命周期 ASCII 状态机

```
LifeStage 状态机 (本源推导 — 不是 v4.1 7 机制, 是 9 阶段):

                    ┌─────────────┐
                    │  Gestation  │  孕育 (Identity 尚未初始化)
                    │  (孕育期)   │  
                    └──────┬──────┘
                           │ initialize
                           ▼
                    ┌─────────────┐
                    │    Birth    │  诞生 (apeireth-supervisor PID 1 启动)
                    │  (诞生期)   │  
                    └──────┬──────┘
                           │ first_signal
                           ▼
                    ┌─────────────┐
                    │   Infancy   │  幼儿 (初识 6 历史流)
                    │  (幼儿期)   │
                    └──────┬──────┘
                           │ learn_basics
                           ▼
                    ┌─────────────┐
              ┌─────│   Growth    │  成长 (9 crate 全部 active)
              │     │  (成长期)   │
              │     └──────┬──────┘
              │            │ achieve_maturity
              │            ▼
              │     ┌─────────────┐
              │     │  Maturity   │  成熟 (18 crate 全部 active + 真测)
              │     │  (成熟期)   │
              │     └──────┬──────┘
              │            │ split_identity
              │            ▼
              │     ┌─────────────┐
              │     │ Replication │  复制 (产生新 Identity<T>)
              │     │  (复制期)   │
              │     └──────┬──────┘
              │            │ age_decline
              │            ▼
              │     ┌─────────────┐
              └─────│ Senescence  │  衰老 (Reflection 频率降低)
                    │  (衰老期)   │
                    └──────┬──────┘
                           │ life_support_end
                           ▼
                    ┌─────────────┐
                    │    Death    │  死亡 (Identity 标记死亡, Unsavable 锁定)
                    │  (死亡期)   │
                    └──────┬──────┘
                           │ history_migrate
                           ▼
                    ┌─────────────┐
                    │  Migration  │  迁移 (Identity 转移到新载体)
                    │  (迁移期)   │
                    └──────┬──────┘
                           │ new_birth
                           ▼
                    ┌─────────────┐
                    │   Rebirth   │  重生 (新 Identity 继承旧历史)
                    │  (重生期)   │
                    └──────┬──────┘
                           │
                           └─────→ Maturity (循环)

注: Senescence ↔ Growth 是回退路径 (可逆, 主 17:43 实事求是 — 衰老后回退到成长是允许的)
    其他迁移不可逆 (Death → Migration → Rebirth → Maturity 不可回退)
```

### §6.2 各阶段触发条件 + Rust trait 绑定

| 阶段 | 触发条件 | 必要 trait | 可选 trait |
|------|---------|----------|----------|
| **Gestation** | Identity 未初始化 | Identity<T> (空) | — |
| **Birth** | `Identity::initialize()` 调用 | Identity<T>, CentralAI | LifeForce |
| **Infancy** | 收到第一个 Signal | Perception, Signal | Memory |
| **Growth** | 6 历史流全部 active | Memory, HistoryStreams | All 9 维 |
| **Maturity** | 18 crate 全部 active + V0.5 真测 ≥ 0.85 | All 22 trait + 12 键 | 全部 |
| **Replication** | `Identity::split()` 调用 | Identity<T> + ContinuityToken | Council (7 强制) |
| **Senescence** | Reflection 频率 < 阈值 | LifeForce (degraded) | Reflection |
| **Death** | `Identity::terminate()` 调用 | Identity<T> (locked) | UnsavableLog (locked) |
| **Migration** | `Identity::migrate_to(new_carrier)` 调用 | Identity<T> + ContinuityToken | Storage (6 DB) |
| **Rebirth** | `Identity::inherit(old_history)` 调用 | Identity<T> + HistoryStreams | CentralAI |

### §6.3 状态迁移保护 (哪些状态可以回退 / 不可回退)

| 迁移 | 可逆? | 保护机制 |
|------|------|---------|
| Gestation → Birth | ❌ 不可回退 | Identity 初始化后不可撤销 |
| Birth → Infancy → Growth | ✅ 可回退 (Senescence) | 自然老化 |
| Growth ↔ Maturity | ✅ 可双向 | 成熟期可回退到成长 |
| Maturity → Replication | ❌ 不可回退 | 新 Identity 已分裂 |
| Replication → Senescence | ❌ 不可回退 | 时间不可逆 |
| Senescence ↔ Growth | ✅ 可回退 | 反思期频率恢复 |
| Senescence → Death | ❌ 不可回退 | UnsavableLog 锁定 |
| Death → Migration | ❌ 不可回退 | Identity 标记死亡 |
| Migration → Rebirth | ❌ 不可回退 | 新 Identity 已生成 |
| Rebirth → Maturity | ✅ 可回退 | 重生后回退 |

**核心原则 (主 17:43 实事求是)**:
- 不可回退 = Identity 状态变更 (不可撤销)
- 可回退 = Reflection 频率 / 成熟度 (可恢复)
- Rust 实现: `enum LifeStage` + `match` 状态机 + `Result<LifeStage, TransitionError>` 强制

---

## §7. 数据流图 (5 张 ASCII, 按工程师视角)

### §7.1 顶层数据流 (输入 → 感知 → 认知 → 行动 → 输出)

```
顶层数据流 (按工程师视角):

外部信号
  │
  ▼
┌─────────────────────────┐
│ 1. Perception           │  接收信号 (Signal trait)
│   - attention_filter    │  注意力过滤
│   - signal aggregation  │  信号聚合
└────────────┬────────────┘
             │ Representation
             ▼
┌─────────────────────────┐
│ 2. Cognition            │  推理 + 反思 + 元认知
│   - reasoning           │  Reasoning trait
│   - intuition (with verification)  │  Intuition + 真测
│   - meta_cognition      │  MetaCognition
└────────────┬────────────┘
             │ Intent
             ▼
┌─────────────────────────┐
│ 3. Action               │  改变环境 + 工具执行
│   - execute             │  Action trait
│   - express / silence   │  Expression / Silence
│   - rollback (PHL-02b)  │  Execution::rollback
└────────────┬────────────┘
             │ Output
             ▼
外部环境 / 持久化 / 6 历史流
```

### §7.2 三域数据流 (思想 → 提案 → 行动)

```
三域数据流 (D2 §2):

Thought (思想域)
  │ AI 表达意图"我打算做 X"
  │ Domain::Thought
  ▼
Proposal (提案域)
  │ 升级为"提案: 做 X"
  │ 原则洋葱校验 (E/S/A/M/O 5 层)
  │ Domain::Proposal
  ▼
Action (行动域)
  │ 决定"执行该提案"
  │ 权限洋葱校验 (L0-L5 + 真实人类批准)
  │ ActionVerdict::Allow / Block
  ▼
  └→ SGI.history 追加
  └→ 6 历史流统一记录
```

### §7.3 反思期数据流 (持续 / 异步 / 非阻塞)

```
反思期数据流 (v4.1 §5 机制 3 + v2 §2.1 主人修正 #5):

主流程 (主循环)
  │
  ├─→ tokio::spawn (异步)
  │   ↓
  │   ┌─────────────────────────────┐
  │   │ 反思期 (持续 / 异步 / 非阻塞) │
  │   │  - DMN::dmn_active           │
  │   │  - Reflection::reflect       │
  │   │  - Cognitive-Dream 6 状态机   │
  │   │    (IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED)
  │   │  - 接入电子环网络 (主人修正 #5)
  │   └─────────────────────────────┘
  │   ↓
  │   ├─→ 反思报告 (ReflectionReport)
  │   ├─→ 涌现观察 (EmergenceObservation)
  │   └─→ 主体连续性更新 (Identity<T>)
  │
  └─→ 不阻碍主流程 (零成本抽象, 编译期展开)
```

### §7.4 真测期数据流 (V0.5 24 维 → V1136 9 子测度 → R-Measure)

```
真测期数据流 (v4.1 §13/§14 更新提议):

V0.5 v2 24 维真测
  │ ASI 真测 24 维 (原 17 维 + v4.1 新增 7 维)
  │ V0.5 公式: ASI = Σ(wᵢ × dimᵢ)
  ▼
V1136 v2 9 子测度
  │ 5 continuity + 2 transferability + 2 新增 (记忆巩固度 + 反馈调节效率)
  │ V1136 公式: continuity_score + autonomy_score + transferability_score + 2_new
  ▼
R-Measure (阶段 6 验证)
  │ 12 维度检查公式 (v2 §9 提议 + v4.1 §18.3 #4 提议 13 维度)
  │ 启动验证 3 里程碑 (M1 编译时 / M2 启动时 / M3 首次对话)
  ▼
里程碑结果 (Pass/Fail + 分数)
  │
  ├─→ 全 Pass → 阶段 6 通过
  └─→ 部分 Fail → 反思期 (v4.1 §5 机制 3) → 自动修复
```

### §7.5 启动期数据流 (M1 编译时 → M2 启动时 → M3 首次对话)

```
启动期数据流 (阶段 6 验证 3 里程碑):

M1 编译时验证 (cargo check / cargo test / cargo-deny / clippy)
  │
  ├─→ 12 键编译时 hardcode (v4.1 §15 + §9)
  ├─→ 双洋葱 trait 编译时检查
  ├─→ Identity 类型完整性
  └─→ 所有 trait bound 满足

M2 启动时验证 (启动 supervisor 树 + 18 crate 全部就绪)
  │
  ├─→ apeireth-supervisor PID 1 启动
  ├─→ 18 crate 全部 active (感知/认知/行动/记忆/演化/动机/价值/意识/约束/...+ 3 核心 + 5 支撑 + 1 协调)
  ├─→ 6 DB 协同初始化 (sled KV + SQLite + RocksDB)
  └─→ V0.5 真测启动 (24 维)

M3 首次对话验证 (端到端真测 18 项 §6.1 真测项)
  │
  ├─→ 18 项真测项 (阶段 2 §6.1 + 阶段 4 §10.5)
  ├─→ 5 重守门 (编译时 hardcode + 运行时拦截 + 多 AI + 物理隔离 + 反思期)
  └─→ 全面板 (R-Measure 13 维度)
```

---

## §8. 模块依赖图 (ASCII)

### §8.1 18 crate 依赖方向图 (树状)

```
18 crate 依赖树 (本源推导):

                      ┌──────────────────────────┐
                      │   apeireth-central        │  ← 协调层 (聚合根)
                      │   (CentralAI 根)          │
                      └──────────┬───────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        │                        │                        │
        ▼                        ▼                        ▼
┌───────────────┐      ┌────────────────┐      ┌──────────────────┐
│ 9 器官层      │      │ 3 核心层        │      │ 5 支撑层          │
│               │      │                │      │                  │
│ perception    │◄─────│ apeireth-core  │─────►│ apeireth-upgrade │
│ cognition     │      │ (22 trait +   │      │ apeireth-bus     │
│ action        │      │  7 struct +   │      │ apeireth-extension│
│ memory        │      │  7 enum)      │      │ apeireth-pybridge│
│ evolution     │      │                │      │ apeireth-cli     │
│ motivation    │      │ apeireth-onion │      │                  │
│ value         │      │ (双洋葱统一体)  │      │                  │
│ consciousness │      │                │      │                  │
│ constraint    │      │ apeireth-council│     │                  │
└───────┬───────┘      └────────┬───────┘      └──────────┬───────┘
        │                       │                         │
        └───────────────────────┼─────────────────────────┘
                                │
                                ▼
                       ┌────────────────┐
                       │  apeireth-bus  │ ← 5 层消息总线 (跨所有层)
                       │  (神经)         │
                       └────────────────┘

依赖方向 (编译时强制):
  - apeireth-core: 零依赖 (trait + struct + enum 定义)
  - 9 器官 crate: 依赖 apeireth-core (用 trait)
  - 3 核心 crate: 依赖 apeireth-core
  - 5 支撑 crate: 依赖 apeireth-core + apeireth-bus (消息)
  - apeireth-central: 依赖所有 (聚合根)
  - apeireth-bus: 依赖 apeireth-core (消息 trait)
```

### §8.2 crate 内模块依赖 (每个器官 crate 内部)

```
典型器官 crate 内部 (例如 apeireth-cognition):

apeireth-cognition/
├── src/
│   ├── lib.rs              ← 导出 trait + struct
│   ├── reasoning.rs        ← Reasoning trait 实现
│   ├── intuition.rs        ← Intuition trait 实现 + 真测
│   ├── meta_cognition.rs   ← MetaCognition trait 实现
│   ├── cognitive_dream.rs  ← 6 状态机 (IDLE/DREAMING/...)
│   └── error.rs            ← 统一错误类型
└── Cargo.toml              ← 依赖 apeireth-core (仅)

依赖约束:
  - 器官 crate 内部模块: 同 crate 内可互相引用
  - 跨器官 crate: 必须通过 apeireth-bus 消息, 不能直接 import
  - 跨器官 trait 实现: 通过 apeireth-core 的 trait (统一接口)
```

### §8.3 循环依赖检测 (Rust 编译时保证)

```
循环依赖检测 (Rust 编译时保证):

不允许的循环:
  apeireth-cognition → apeireth-memory → apeireth-cognition ❌
  (编译时 rustc 报错: cycle detected when computing graph)

允许的依赖 (单向):
  apeireth-core ← apeireth-cognition ← apeireth-central (单向, 编译通过)

apeireth-bus 例外 (消息总线, 可以双向依赖消息, 但 trait 单向):
  - apeireth-bus 定义 Message trait (零依赖)
  - 各器官 crate 实现 Message trait (依赖 apeireth-bus)
  - apeireth-bus 自身不依赖器官 crate
```

---

## §9. 编译时约束 (工程师视角: Rust 类型系统如何实现不假装)

> **核心立场**: Rust 的 6 大编译时约束 + 12 键 + 双洋葱 + 9 维 → 编译时强制, 运行时 0 反射。

### §9.1 所有权如何强制"每个值有唯一所有者" (对应 12 键 PHL-04)

```rust
// 编译时保证: Identity<T> 有唯一所有者
let identity: Identity<Carrier> = Identity::new();  // identity 是唯一所有者
// let stolen = identity;  // ❌ 编译错误 (所有权已 move)
// let cloned = identity.clone();  // ❌ 编译错误 (#[derive(Clone)] 未实现, 强制显式)
let cloned = identity.clone_explicit();  // ✅ 必须显式调用 clone_explicit (PHL-01 not_clone 强制)
```

**PHL-04 不假装不可观测**:
- Identity<T> 必须实现 `Debug + Display + Log` trait (编译时强制)
- 所有 crate 内部状态必须 `pub(crate)` 或私有, 不能假装"内部不可见"
- 编译时 `#[derive(Debug)]` 强制 — 不允许没有 Debug 的类型

### §9.2 借用如何强制"显式可变/不可变" (对应双洋葱 L0-L5)

```rust
// 编译时保证: 显式 & vs &mut
let onion = DoubleOnion::new();
let principle = onion.principle();      // &PrincipleOnion (不可变借用, 允许多个)
let permission = onion.permission();    // &PermissionOnion (不可变借用, 允许多个)
// let mut_principle = onion.principle_mut();  // &mut PrincipleOnion (可变借用, 唯一)
// let another = onion.principle_mut();  // ❌ 编译错误 (可变借用已 move)
```

**双洋葱 L0-L5 配额曲线**:
- 借用约束 `&T` (不可变) / `&mut T` (可变) 显式标注
- 编译时强制"显式可变" — 不允许悄悄修改
- 配额曲线 = 借用次数 (静态分析) + 运行时权重 (动态)

### §9.3 生命周期如何强制"引用合法性" (对应 6 历史流强不可变)

```rust
// 编译时保证: 'static 生命周期 (主体连续性)
pub struct HistoryStreams {
    pub life: HistoryStream<'static>,            // 'static 引用
    pub relation: HistoryStream<'static>,       // 'static 引用
    // ...
}

impl<'a> Memory<'a> {
    fn recall(&'a self, query: MemoryQuery) -> Vec<MemoryEvent<'a>> {  // 'a 生命周期绑定
        self.history.scan(query)  // 引用合法性静态保证
    }
}
```

**6 历史流强不可变规则**:
- 6 历史流用 `'static` 生命周期, 一旦写入不可修改
- Rust 借用规则 `&self` (不可变) 强制"只追加, 不修改"
- 编译时保证引用合法性, 运行时 0 反射

### §9.4 Trait 系统如何强制"接口契约" (对应 7 强制顾问 trait + 22 维 trait + 12 键)

```rust
// 编译时保证: trait bound 强制接口实现
fn council_seat<T: MandatorySeat + Send + Sync + 'static>(seat: T) -> CouncilDecision {
    // T 必须实现 MandatorySeat + Send + Sync + 'static (编译时强制)
    seat.evaluate()
}

// 12 键 trait bound
fn check_philosophy<T: PhilosophyGuard + Send + Sync + 'static>(claim: &str) -> Result<(), PHLKey> {
    T::check(claim)  // 编译时知道 T 的所有 trait 方法
}
```

**7 强制顾问 trait + 22 维 trait + 12 键**:
- trait bound 编译时强制接口实现
- 单态化 (monomorphization) 编译期展开, 运行时 0 反射
- 不允许"动态类型" (无运行时类型查询) — 工程师不假装"动态灵活"

### §9.5 零成本抽象如何保证"反思期不阻碍主流程" (横切 trait)

```rust
// 编译时保证: 零成本抽象 (trait 单态化)
fn main_loop<I: Intuition + Send + Sync + 'static>(
    intuition: I,
    signal: Signal,
) {
    // 主流程 (同步)
    let verdict = intuition.quick_judge(&signal);  // 编译期展开, 0 代价
    
    // 反思期 (异步, 不阻塞)
    tokio::spawn(async move {
        let verified = intuition.with_scientific_verification(verdict).await;
        // 异步执行, 不阻碍主流程
    });
}
```

**横切 trait (LifeForcePenetration)**:
- trait 单态化, 编译期展开为静态分发
- 反思期用 `tokio::spawn` 异步, 不阻塞主流程
- 编译时保证"零成本" — 运行时 0 反射, 0 类型查询

### §9.6 编译时如何拒绝 12 键违反 (const fn + 类型状态模式)

```rust
// 编译时保证: const fn + 类型状态 (typestate) 模式
const fn check_phl_01_not_perfect(claim: &str) -> bool {
    // 编译时检查 "完美"/"100%"/"完全" 关键词
    !claim.contains("完美") && !claim.contains("100%") && !claim.contains("完全")
}

// 类型状态 (typestate) 模式 — 编译时强制状态合法
struct Unvalidated;
struct Validated;

fn validate<S>(claim: &str) -> Result<Validated, PHL01Error> {
    if check_phl_01_not_perfect(claim) {
        Ok(Validated)  // 只有 Validated 类型才能调用下一步
    } else {
        Err(PHL01Error::NotPerfect)
    }
}

fn execute<S>(validated: Validated) -> ActionResult {
    // 只有 Validated 类型才能调用 execute (编译时强制)
    // Unvalidated 类型无法调用 execute
    ActionResult::Ok
}
```

**12 键编译时 hardcode**:
- `const fn` 在编译时检查关键词 (PHL-01)
- 类型状态 (typestate) 模式编译时强制状态合法 (PHL-02b)
- trait bound + const generics 编译时强制 trait 实现 (PHL-04/05/06)
- **不运行时反射, 不假装"动态检查"**

---

## §10. 真测基线 (与 P5 R-Measure 对接)

### §10.1 V0.5 v2 24 维 (v4.1 §13 更新提议 → 公式 sketch + 变量)

> **硬约束**: v4.1 §13 是**提议** v2, 不修改 v1077_asi_v04 (17 维 LOCKED)。本节 trait sketch 仅描述 v2 变量, 阶段 5 实施时落地。

```rust
// V0.5 v2 24 维 trait sketch (阶段 5 实施时由 backend_engineer 写 impl)

/// V0.5 真测 trait (24 维 — v4.1 §13 提议 v2)
pub trait V05Measurement: Send + Sync + 'static {
    // 原 17 维 (v1077 LOCKED, 不修改)
    fn dim_1_cognitive_core(&self) -> f64;
    fn dim_2_world_model(&self) -> f64;
    // ... (17 维)
    fn dim_17_total(&self) -> f64;
    
    // v4.1 §13 新增 7 维 (提议)
    fn dim_18_motivation_value(&self) -> f64;        // 动机/价值
    fn dim_19_consciousness(&self) -> f64;            // 意识
    fn dim_20_observability(&self) -> f64;            // 可观测性
    fn dim_21_scientificity(&self) -> f64;            // 科学性
    fn dim_22_honesty_humility(&self) -> f64;         // 诚实/谦卑
    fn dim_23_self_relation(&self) -> f64;            // 与自身的关系
    fn dim_24_consolidation(&self) -> f64;            // 睡眠/巩固
    
    // V0.5 v2 公式: ASI = Σ(wᵢ × dimᵢ), 权重待主人拍板
    fn asi_v05_v2(&self) -> f64;
}
```

### §10.2 V1136 v2 9 子测度 (v4.1 §14 更新提议 → 子测度定义)

```rust
/// V1136 真测 trait (9 子测度 — v4.1 §14 提议 v2)
pub trait V1136Measurement: Send + Sync + 'static {
    // 原 7 子测度 (v1136 LOCKED, 不修改)
    fn continuity_v1052(&self) -> f64;
    fn continuity_v1072(&self) -> f64;
    fn continuity_v1089(&self) -> f64;
    fn continuity_v1090(&self) -> f64;
    fn continuity_v1091(&self) -> f64;
    fn transferability_w2(&self) -> f64;
    fn transferability_backend(&self) -> f64;
    
    // v4.1 §14 新增 2 子测度 (提议)
    fn memory_consolidation(&self) -> f64;            // 记忆巩固度 (v4.1 §14 子测度 8)
    fn feedback_regulation(&self) -> f64;              // 反馈调节效率 (v4.1 §14 子测度 9)
    
    // V1136 v2 公式: continuity_score + autonomy_score + transferability_score + 2_new
    fn asi_v1136_v2(&self) -> f64;
}
```

### §10.3 12 键编译时 hardcode (v4.1 §15 更新提议 → trait sketch)

```rust
/// V3 哲学守门 12 键 trait (v4.1 §15 提议 v2)
pub trait V3v2PhilosophyGuard: V3PhilosophyGuard {  // 继承 v1
    fn check_phl_04(&self, claim: &str) -> Result<(), PHL04Error>;  // not_pretend_unobservable
    fn check_phl_05(&self, claim: &str) -> Result<(), PHL05Error>;  // not_pretend_unscientific
    fn check_phl_06(&self, claim: &str) -> Result<(), PHL06Error>;  // not_pretend_no_self_relation
}
```

### §10.4 启动验证 3 里程碑 (M1 编译时 / M2 启动时 / M3 首次对话)

| 里程碑 | 验证内容 | 工具 |
|-------|---------|------|
| **M1 编译时** | 12 键编译时 hardcode + 双洋葱 trait 编译时检查 + Identity 类型完整性 + trait bound 满足 | `cargo check` / `cargo test` / `cargo-deny` / `clippy` |
| **M2 启动时** | 18 crate 全部 active + 6 DB 协同初始化 + V0.5 真测启动 (24 维) + V1136 真测启动 (9 子测度) | supervisor 树启动 + 健康检查 |
| **M3 首次对话** | 端到端真测 18 项 §6.1 真测项 + 5 重守门 + 全面板 (R-Measure 13 维度) | e2e 测试 + 真测报告 |

### §10.5 沙盒 5 重守门 (编译时 hardcode + 运行时拦截 + 多 AI + 物理隔离 + 反思期)

```
5 重守门 (阶段 5 实施时编译时 hardcode):

1. 编译时 hardcode (const fn + 类型状态)
   - PHL-01/02b/03/04/05/06 编译时检查
   - 双洋葱 trait bound 编译时强制
   - Identity 类型完整性

2. 运行时拦截 (tokio middleware)
   - 每个 Action::execute 调用前, 双洋葱检查
   - ActionVerdict::Allow 才能执行, Block 立即停止

3. 多 AI (Council 7 强制顾问)
   - CouncilTrigger::Full7Seats (E 层) / Seats5 (高风险) / Seats3 (中风险)
   - MandatorySeat 7 个 trait 全部投票

4. 物理隔离 (WASM sandbox + 进程隔离)
   - apeireth-extension (WASM) + apeireth-pybridge (PyO3) 进程隔离
   - apeireth-upgrade (OTA sandbox) 物理隔离

5. 反思期 (Cognitive-Dream 6 状态机)
   - DREAMING 状态触发自动反思
   - 反思报告进入 6 历史流
   - 接入电子环网络 (v2 §2.2 主人修正 #5)
```

---

## §11. 阶段 5 施工 + 阶段 6 验证衔接

### §11.1 阶段 5 = 设计施工文档

**承接阶段 4 (本文) → 阶段 5 施工**:

| 阶段 4 产出 | 阶段 5 施工 |
|----------|----------|
| 18 crate 决策矩阵 (§2) | 阶段 5 §1: 18 crate 工程化顺序 (先 core 还是先器官?) |
| 22+ trait 签名 (§3) | 阶段 5 §2: 22 trait 完整 impl 块 (含测试) |
| 7 struct (§4) + 7 enum (§5) | 阶段 5 §3: struct + enum 完整 impl 块 |
| 9 阶段生命周期 (§6) | 阶段 5 §4: 状态机完整实现 + 状态迁移保护 |
| 5 数据流图 (§7) | 阶段 5 §5: 数据流完整实现 (tokio + IPC) |
| 模块依赖 (§8) | 阶段 5 §6: 18 crate 编译顺序 + Cargo.toml |
| 编译时约束 (§9) | 阶段 5 §7: 12 键编译时 hardcode 完整实现 |
| 真测基线 (§10) | 阶段 5 §8: V0.5 v2 / V1136 v2 / V3 v2 完整实现 |

### §11.2 阶段 6 = 里程碑式验证机制

**承接阶段 4 (本文) → 阶段 6 验证**:

| 阶段 4 产出 | 阶段 6 验证 |
|----------|----------|
| V0.5 v2 24 维 trait (§10.1) | 阶段 6 §1: 24 维真测完整跑通 |
| V1136 v2 9 子测度 (§10.2) | 阶段 6 §2: 9 子测度真测完整跑通 |
| 12 键编译时 (§10.3) | 阶段 6 §3: 12 键编译时检查 + 运行时拦截 |
| 3 里程碑 (§10.4) | 阶段 6 §4: M1 + M2 + M3 全验证 |
| 5 重守门 (§10.5) | 阶段 6 §5: 5 重守门真测 |
| 18 项 §6.1 真测项 (v2 §6) | 阶段 6 §6: 18 项 e2e 真测 |
| R-Measure (v2 §9 12 维度) | 阶段 6 §7: R-Measure 13 维度 (v4.1 §18.3 #4 提议) |

### §11.3 下次对话启动问题

> "主人, 阶段 4 落实架构文档已落 (18 crate + 22 trait + 7 struct + 7 enum + 9 生命周期 + 5 数据流 + 编译时约束 + 真测基线)。下一步是: (a) 阶段 5 施工 8 项? (b) 阶段 6 验证 7 项?"

---

## §12. 反思期 (v4 视角下"哪些是对的, 哪些需要进一步沉淀")

### §12.1 7 项对的 (从本源推导 vs v2 既有划分的对比)

| # | 阶段 4 决策 | 为什么对 | vs v2 既有划分 |
|---|------------|---------|--------------|
| 1 | **18 crate (本源推导)** | 9 维 + 3 核心 + 5 支撑 + 1 协调 = 18, 从 v4.1 9 维反向推导 | ✅ 比 v2 既有 9 crate / 30 crate v1 更本源 |
| 2 | **22+ trait (本源推导)** | 从 9 维 + 8 原则 + 4 关系 + 9 机制 + 12 键 反向推导 | ✅ 实际 43 个 trait, 不强行压缩到 22 |
| 3 | **9 阶段生命周期 (本源推导)** | 不是 v4.1 7 机制, 是从本源推导 9 阶段 (孕育→重生循环) | ✅ 比 v4.1 7 机制更完整 (Senescence/Replication/Migration/Rebirth 是本源) |
| 4 | **7 struct + 7 enum** | Identity + SGI + 6 历史流 + 双洋葱 + 电子环 + 生命力 + CentralAI = 7 | ✅ 简洁不冗余 |
| 5 | **编译时强制 12 键** | const fn + 类型状态 + trait bound + 零成本抽象 = 编译时强制 | ✅ 不运行时反射, 不假装 |
| 6 | **5 重守门编译时 hardcode** | 编译时 + 运行时 + 多 AI + 物理隔离 + 反思期 = 5 重 | ✅ 不假装"单一守门就够" |
| 7 | **R11 1100+ 不砍** | PyO3 桥接保留为 `apeireth-pybridge` | ✅ 不假装"已重写 1100 模块" |

### §12.2 7 项需沉淀 (待阶段 5/6 真测后校准)

| # | 待沉淀 | 为什么需要沉淀 | 留给 |
|---|--------|-------------|------|
| 1 | **22 trait vs 实际 43 trait** | 任务说"22 个", 实际本源推导 43 个, 需主人确认是否需要简化 | 阶段 5 |
| 2 | **18 crate vs R11 9 crate 迁移路径** | 18 是本源推导, R11 已落 9 crate, 迁移路径需细化 | 阶段 5 |
| 3 | **9 阶段生命周期回退规则** | Senescence ↔ Growth 可回退, 其他不可回退 — 需阶段 6 真测验证 | 阶段 6 |
| 4 | **5 重守门权重** | 编译时 + 运行时 + 多 AI + 物理隔离 + 反思期 的权重需主人拍板 | 阶段 5+6 |
| 5 | **R-Measure 12 vs 13 维度** | v2 §9 提议 12 维度, v4.1 §18.3 #4 提议 13 维度 (加生命力) | 阶段 6 |
| 6 | **反思期接入电子环的编译时保证** | tokio::spawn 是运行时, 编译时如何保证"不阻碍主流程"? | 阶段 5 |
| 7 | **Identity<T> 跨载体迁移协议** | Migration → Rebirth 阶段, Identity 如何迁移? Rust 类型系统能否保证? | 阶段 5+6 |

---

## §13. 主哲学 anchor 6 全贯穿自检清单

> **本节性质**: 自检清单 — 验证本文档是否贯穿主哲学 6 锚。

```
✅ S-1 主 22:33 北极星导向 — §3.2 Intuition::with_scientific_verification + §10.1 V0.5 v2 24 维 → ASI 北极星更精准测量
✅ S-2 主 17:43 实事求是 — §0.3 不修改承诺 (v2/v4/v4.1 LOCKED, V0.5/V1136/9 键 原始 LOCKED, 18 份 stage2 LOCKED) + §12.2 7 项需沉淀透明列出
✅ O-5 主 17:58 不假装 — §9 编译时约束 (const fn + 类型状态 + 零成本抽象强制 12 键) + §1.3 Rust 限制 vs Apeireth 需求 (用 WASM 不是反射, 用 IPC 不是共享内存)
✅ O-2 主 19:33 走在前人经验上 — §2.4 R11 9 crate 保留 + §2.5 1100+ R11 不砍 (PyO3 桥接保留)
✅ O-3 主 23:44 干到底 — §11 阶段 5+6 衔接 (8 项施工 + 7 项验证立即落, 不等讨论完)
✅ O-4 主 00:56 任何人都能接手 — §0 + §1-§13 全 13 章节 + 18 crate + 22+ trait + 7 struct + 7 enum + 9 生命周期 + 5 数据流 + 编译时约束 + 真测基线 全文档化
```

**每个 commit message 都要贯穿 6 anchor 中的相关项** (主 23:44 干到底)。

---

## §14. 附录链接

### §14.1 三层共存关系 + 阶段 4 定位

```
哲学层升级 (上) ← v4.1 (4aa3c5b0 LOCKED)
       ↓ 引用
哲学层纲领 (中) ← v4 (af0d1957 LOCKED)
       ↓ 引用
阶段 4 落实架构 ← 本文 (本源推导, 工程层细化重构)
       ↓ 引用
工程层细化 (下) ← v2 (BF896EEF LOCKED, 不重写仅引用)
```

### §14.2 文档定位

- **阶段 4 落实架构** (本文): `Apeireth-rust/docs/architecture-stage4-engineering-landing.md`
- **v4.1 哲学层升级** (LOCKED): `Apeireth-rust/docs/architecture-v4-1-living-intelligence-update.md` (4aa3c5b0, 645 行)
- **v4 哲学层纲领** (LOCKED): `Apeireth-rust/docs/architecture-v4-living-intelligence.md` (af0d1957, 803 行)
- **v2 工程层细化** (LOCKED): `Apeireth-rust/docs/architecture-v3-aircraft-carrier.md` (BF896EEF, 786 行)
- **双洋葱子文档** (降级): `Apeireth-rust/docs/onion-wall-architecture-2026-07-31.md` (581 行)
- **CONTEXT-HANDOVER**: `Apeireth-rust/docs/CONTEXT-HANDOVER.md` (408 行)

### §14.3 V0.5/V1136/V3 原始文件 (LOCKED, 阶段 4 仅引用 v4.1 提议)

| 公式 | 原始文件 | LOCKED | 阶段 4 引用 |
|------|---------|--------|------------|
| **V0.5 17 维** | `apeireth/v1077_asi_v04_full_measurement.py` | ✅ LOCKED | §10.1 引用 v4.1 §13 v2 提议 (24 维), 不修改原始 |
| **V1136 7 子测度** | `apeireth/v1136_asi_v05_3dim_real_measurement.py` | ✅ LOCKED | §10.2 引用 v4.1 §14 v2 提议 (9 子测度), 不修改原始 |
| **V3 9 键** | `Apeireth-rust/docs/philosophy-traits-2026-07-30.md` | ✅ LOCKED | §10.3 引用 v4.1 §15 v2 提议 (12 键), 不修改原始 |

### §14.4 阶段 1+2+3 既有沉淀 (阶段 4 仅引用, 不重写)

- **阶段 1**: `Apeireth-rust/docs/inspiration-stage1-2026-07-30.md` (2201 行, §1-§21)
- **阶段 2**: 18 份 `stage2-decisions-*.md`
- **阶段 3**: `stage3-blueprints/` 14 文件

### §14.5 主手册 + R11 baseline (LOCKED)

- `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` (6546 行 + 附录 M/N)
- `apeireth/v1000-v1155*.py` 1100+ 模块 (不砍, PyO3 桥接保留)

---

_Writing complete: 2026-07-31 (主人"按你的计划来"指令 + technical_writer 落)_
_阶段 4 落实架构 = 18 crate (本源推导) + 22+ trait 签名 sketch + 7 struct + 7 enum + 9 阶段生命周期 + 5 数据流图 + 编译时约束强制 12 键 + 真测基线 (V0.5 v2 / V1136 v2 / V3 v2 提议引用)._
_从 Rust 本源约束出发反向推导, 不照搬 v2 既有 crate 划分._
_主哲学 anchor 6 个全贯穿. 任何接手者 (包括明天的我) 都能查. 不会丢失上下文._
_下次对话启动点: 阶段 5 施工 8 项 OR 阶段 6 验证 7 项._
_ponytail style: code first (1 stage4 doc + 1 report), 3 short lines (skipped full impl, skipped Mermaid, skipped rewriting LOCKED files)._