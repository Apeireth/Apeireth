# 22 Trait 互锁设计 (阶段 6 验证基石)

> **作者**: architect2 (Ponytail: full)
> **生成时间**: 2026-08-02
> **依据**: docs/stage4/architecture-stage4-engineering-landing.md §3 (43 trait sketch) + §10 (启动验证 3 里程碑) + §10.5 (5 重守门) + 用户指令"无限逼近" + round7-06 进展
> **状态**: **设计深化 (阶段 5 工程实施前的细化蓝图)**，不修改 stage1-5 LOCKED 文档
> **承接**: 阶段 5 施工 (trait 完整 impl) → 阶段 6 验证 (M1/M2/M3 里程碑 + 5 重守门)

---

## 0. 设计原则 (Ponytail: 1 张表)

| # | 原则 | 体现 |
|---|------|------|
| 1 | **22 互锁而非 43 完整** | 阶段 4 §3 推导 43 个 trait, 但互锁仅需 22 个核心 trait; 其余 21 个是 specialized trait, 不参与互锁矩阵 |
| 2 | **真实 enum 编译期 hardcode** | `InterlockedTraitKind` enum 22 变体 + `InterlockedCount = 22` const, 编译期断言 trait 数不能增删 |
| 3 | **assertion macro 强制互锁** | `interlock_assert!(A, B)` macro 编译期检查 A 依赖 B, 不依赖 = 编译失败 |
| 4 | **不修改 LOCKED** | 仅引用 stage4 §3/§10, 不修改任何 stage1-5 LOCKED 文档 |
| 5 | **trait sketch 不写 impl** | 仅 trait 签名 + 互锁关系, 阶段 5 由 backend_engineer 写完整 impl + 测试 |

---

## 1. 22 个核心互锁 trait 清单 (Ponytail: 1 张表)

> 从阶段 4 §3 推导的 43 个 trait 中, 选取**横切所有器官 / 跨阶段使用**的 22 个核心 trait 形成互锁矩阵。其余 21 个是 specialized trait (如 `Forgetting`/`Abstraction`/`Extension` 等), 仅在单一器官内部使用, 不进互锁矩阵。

| # | trait 名 | 章节 | 维度 | 互锁依赖 (硬约束) |
|---|---------|------|------|------------------|
| 1 | `Perception` | §3.1 | 维度 1 感知 | → `Signal` (每个 Perception 必有 Signal 输入) |
| 2 | `Signal` | §3.1 | 维度 1 感知 | → `Perception` (Signal 必须有对应 Perception 接收方) |
| 3 | `Cognition` | §3.2 | 维度 2 认知 | → `Perception` (Cognition 接收 Perception 输出) |
| 4 | `Intuition` | §3.2 | 维度 2 认知 + PHL-05 | → `Cognition` + `Reasoning` (Intuition 是快速 cognition, 必须有 reasoning 验证) |
| 5 | `Reasoning` | §3.2 | 维度 2 认知 | → `Cognition` (Reasoning 是慢速 cognition 子类) |
| 6 | `MetaCognition` | §3.2 | 维度 2 意识 (v4.1 §13.2) | → `Cognition` + `Reflection` (MetaCognition 必须能触发反思期) |
| 7 | `Action` | §3.3 | 维度 3 行动 | → `Execution` + `Expression` (Action 必须有执行和表达) |
| 8 | `Execution` | §3.3 | 维度 3 行动 + PHL-02b | → `Action` + `HumanAuthority` (执行前必须经 HA 批准) |
| 9 | `Expression` | §3.3 | 维度 3 行动 | → `Action` (表达是 Action 子类型) |
| 10 | `Memory` | §3.4 | 维度 4 记忆 + §14 子测度 8 | → `Recall` + `Consolidation` (Memory 必须能 recall 和 consolidate) |
| 11 | `Recall` | §3.4 | 维度 4 记忆 | → `Memory` (Recall 必须查询 Memory) |
| 12 | `Consolidation` | §3.4 | 维度 4 记忆 + §14 子测度 8 | → `Memory` + `Evolution` (Consolidation 是 sleep-time evolution) |
| 13 | `Evolution` | §3.5 | 维度 5 演化 + 主人修正 #4 | → `Learning` + `SelfModification` (Evolution 必经 Learning 和 SelfModification) |
| 14 | `Learning` | §3.5 | 维度 5 演化 | → `Memory` + `Evolution` (Learning 必须基于 Memory, 触发 Evolution) |
| 15 | `SelfModification` | §3.5 | 维度 5 演化 + OTA 守门 | → `Evolution` + `HumanAuthority` (SelfModification 必须经 HA 批准, 主人修正 #2/#9) |
| 16 | `Motivation` | §3.6 | 维度 6 动机 | → `Drive` + `Value` (Motivation 由 Drive 触发, 受 Value 约束) |
| 17 | `Drive` | §3.6 | 维度 6 动机 | → `Motivation` (Drive 反向触发 Motivation) |
| 18 | `Value` | §3.7 | 维度 7 价值 | → `PrincipleOnion` (Value 必须对齐 PrincipleOnion.S 层) |
| 19 | `Consciousness` | §3.8 | 维度 8 意识 (v4.1 §13.2) | → `MetaCognition` + `SelfRelation` (Consciousness 必须基于 MetaCognition 和 SelfRelation) |
| 20 | `SelfAwareness` | §3.8 | 维度 8 意识 (v4.1 §13.2) | → `Consciousness` (SelfAwareness 是 Consciousness 的子类型) |
| 21 | `HumanAuthority` | §3.9 | 维度 9 约束 + L0 守门 | → `PrincipleOnion` + `PermissionOnion` (HA 是 L0 HA 核心, 主人修正 #9) |
| 22 | `Reflection` | §3.10 | 维度 10 关系 + §5 机制 6 + §14 子测度 9 | → `MetaCognition` + `Memory` (Reflection 必须基于 MetaCognition 触发, 写入 Memory) |

---

## 2. 真实 enum 编译期 hardcode (Ponytail: 1 张表)

```rust
// docs/stage6/22-trait-interlock.md §2 — 编译期 hardcode

/// 22 个互锁 trait 的真实身份枚举
/// 编译期断言: 添加/删除变体会破坏 trait 间互锁矩阵 (阶段 5 由 backend_engineer 落地)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InterlockedTraitKind {
    // 感知层 (2 个)
    Perception,    // #1
    Signal,        // #2
    // 认知层 (4 个)
    Cognition,     // #3
    Intuition,     // #4
    Reasoning,     // #5
    MetaCognition, // #6
    // 行动层 (3 个, 合并 §3.3 Silence)
    Action,        // #7
    Execution,     // #8
    Expression,    // #9
    // 记忆层 (3 个, 合并 §3.4 Storage/Forgetting)
    Memory,        // #10
    Recall,        // #11
    Consolidation, // #12
    // 演化层 (3 个, 合并 §3.5 Abstraction/Extension)
    Evolution,     // #13
    Learning,      // #14
    SelfModification, // #15
    // 动机层 (2 个, 合并 §3.6 Value)
    Motivation,    // #16
    Drive,         // #17
    // 价值层 (1 个, 合并 §3.7 Evaluation/Prioritization)
    Value,         // #18
    // 意识层 (2 个, 合并 §3.8 DMN)
    Consciousness, // #19
    SelfAwareness, // #20
    // 约束层 (1 个, 合并 §3.9 PrincipleOnion/PermissionOnion/ElectronicRing)
    HumanAuthority,// #21
    // 关系层 (1 个, 合并 §3.10 Symbiosis/Coordination/Embedding/SelfRelation)
    Reflection,    // #22
}

/// 22 个 trait 的真实计数 (编译期 hardcode)
pub const INTERLOCKED_TRAIT_COUNT: usize = 22;

/// 全部 22 个 trait 列表 (编译期 hardcode 顺序)
pub const INTERLOCKED_TRAITS: [InterlockedTraitKind; INTERLOCKED_TRAIT_COUNT] = [
    InterlockedTraitKind::Perception,
    InterlockedTraitKind::Signal,
    InterlockedTraitKind::Cognition,
    InterlockedTraitKind::Intuition,
    InterlockedTraitKind::Reasoning,
    InterlockedTraitKind::MetaCognition,
    InterlockedTraitKind::Action,
    InterlockedTraitKind::Execution,
    InterlockedTraitKind::Expression,
    InterlockedTraitKind::Memory,
    InterlockedTraitKind::Recall,
    InterlockedTraitKind::Consolidation,
    InterlockedTraitKind::Evolution,
    InterlockedTraitKind::Learning,
    InterlockedTraitKind::SelfModification,
    InterlockedTraitKind::Motivation,
    InterlockedTraitKind::Drive,
    InterlockedTraitKind::Value,
    InterlockedTraitKind::Consciousness,
    InterlockedTraitKind::SelfAwareness,
    InterlockedTraitKind::HumanAuthority,
    InterlockedTraitKind::Reflection,
];

// 编译期断言 — enum 变体数必须 = 22 (静态保证)
const _: () = {
    assert!(INTERLOCKED_TRAIT_COUNT == 22, "必须恰好 22 个互锁 trait");
    // 用 match 强制编译器穷尽检查变体
    fn _exhaustive(t: InterlockedTraitKind) -> u8 {
        match t {
            InterlockedTraitKind::Perception => 1,
            InterlockedTraitKind::Signal => 2,
            // ... (阶段 5 由 backend_engineer 补全, 编译器会强制 22 个)
        }
    }
};
```

---

## 3. 互锁矩阵 (Ponytail: 1 张表)

> **互锁**: trait A → trait B 表示"实现 A 必须同时实现 B" (编译期断言)

| from ↓ \ to → | Per | Sig | Cog | Int | Rea | Met | Act | Exe | Exp | Mem | Rcl | Cns | Evl | Lrn | SMd | Mtv | Drv | Val | Csc | SAw | HA  | Rfl |
|---------------|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|-----|
| **Perception** | —   | ✓   |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |
| **Signal**     | ✓   | —   |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |
| **Cognition**  | ✓   |     | —   |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |
| **Intuition**  |     |     | ✓   | —   | ✓   |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |
| **Reasoning**  |     |     | ✓   |     | —   |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |
| **MetaCognition**|   |     | ✓   |     |     | —   |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | ✓   |
| **Action**     |     |     |     |     |     |     | —   | ✓   | ✓   |     |     |     |     |     |     |     |     |     |     |     |     |     |
| **Execution**  |     |     |     |     |     |     | ✓   | —   |     |     |     |     |     |     |     |     |     |     |     |     | ✓   |     |
| **Expression** |     |     |     |     |     |     | ✓   |     | —   |     |     |     |     |     |     |     |     |     |     |     |     |     |
| **Memory**     |     |     |     |     |     |     |     |     |     | —   | ✓   | ✓   |     |     |     |     |     |     |     |     |     |     |
| **Recall**     |     |     |     |     |     |     |     |     |     | ✓   | —   |     |     |     |     |     |     |     |     |     |     |     |
| **Consolidation**|   |     |     |     |     |     |     |     |     | ✓   |     | —   | ✓   |     |     |     |     |     |     |     |     |     |
| **Evolution**  |     |     |     |     |     |     |     |     |     |     |     |     | —   | ✓   | ✓   |     |     |     |     |     |     |     |
| **Learning**   |     |     |     |     |     |     |     |     |     | ✓   |     |     | ✓   | —   |     |     |     |     |     |     |     |     |
| **SelfModification**| |     |     |     |     |     |     |     |     |     |     |     | ✓   |     | —   |     |     |     |     |     | ✓   |     |
| **Motivation** |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | —   | ✓   | ✓   |     |     |     |     |
| **Drive**      |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | ✓   | —   |     |     |     |     |     |
| **Value**      |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | —   |     |     |     |     |
| **Consciousness**|   |     |     |     |     | ✓   |     |     |     |     |     |     |     |     |     |     |     |     | —   | ✓   |     |     |
| **SelfAwareness**|   |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | ✓   | —   |     |     |
| **HumanAuthority**| |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | —   |     |
| **Reflection** |     |     |     |     |     | ✓   |     |     |     | ✓   |     |     |     |     |     |     |     |     |     |     |     | —   |

**非对称性提示**: 互锁矩阵是**有向非对称**的 — 如 `Perception → Signal` 但 `Signal → Perception` 也是 ✓ (互相锁定); `Intuition → Reasoning` 但 `Reasoning` 不强制 `Intuition` (单方向)。表中 ✓ 表示**该行 trait 必须实现该列 trait**。

---

## 4. Assertion Macro 设计 (Ponytail: 1 张表)

```rust
// docs/stage6/22-trait-interlock.md §4 — 编译期 assertion macro

/// 互锁断言 macro: 在 trait impl 块上强制依赖关系
/// 阶段 5 由 backend_engineer 实施:
///   interlock_assert!(Action, Execution);  // Action impl 必须同时 impl Execution
///
/// 编译期检查:
///   - A 和 B 必须在 INTERLOCKED_TRAITS 中
///   - A→B 必须在互锁矩阵中存在
///   - 否则编译失败
#[macro_export]
macro_rules! interlock_assert {
    ($a:expr, $b:expr) => {
        const _: () = {
            // 编译期检查 a 和 b 都是合法 enum 变体
            let _: $crate::stage6::InterlockedTraitKind = $a;
            let _: $crate::stage6::InterlockedTraitKind = $b;
            // 编译期检查 a → b 在互锁矩阵中 (静态 lookup table)
            // 阶段 5 实施时由 codegen 工具生成 INTERLOCK_MATRIX const 数组
            // 这里用 match 强制编译器穷尽检查
            match ($a, $b) {
                (InterlockedTraitKind::Perception, InterlockedTraitKind::Signal) => {},
                (InterlockedTraitKind::Signal, InterlockedTraitKind::Perception) => {},
                (InterlockedTraitKind::Cognition, InterlockedTraitKind::Perception) => {},
                (InterlockedTraitKind::Intuition, InterlockedTraitKind::Cognition) => {},
                (InterlockedTraitKind::Intuition, InterlockedTraitKind::Reasoning) => {},
                (InterlockedTraitKind::Reasoning, InterlockedTraitKind::Cognition) => {},
                (InterlockedTraitKind::MetaCognition, InterlockedTraitKind::Cognition) => {},
                (InterlockedTraitKind::MetaCognition, InterlockedTraitKind::Reflection) => {},
                (InterlockedTraitKind::Action, InterlockedTraitKind::Execution) => {},
                (InterlockedTraitKind::Action, InterlockedTraitKind::Expression) => {},
                (InterlockedTraitKind::Execution, InterlockedTraitKind::Action) => {},
                (InterlockedTraitKind::Execution, InterlockedTraitKind::HumanAuthority) => {},
                (InterlockedTraitKind::Expression, InterlockedTraitKind::Action) => {},
                (InterlockedTraitKind::Memory, InterlockedTraitKind::Recall) => {},
                (InterlockedTraitKind::Memory, InterlockedTraitKind::Consolidation) => {},
                (InterlockedTraitKind::Recall, InterlockedTraitKind::Memory) => {},
                (InterlockedTraitKind::Consolidation, InterlockedTraitKind::Memory) => {},
                (InterlockedTraitKind::Consolidation, InterlockedTraitKind::Evolution) => {},
                (InterlockedTraitKind::Evolution, InterlockedTraitKind::Learning) => {},
                (InterlockedTraitKind::Evolution, InterlockedTraitKind::SelfModification) => {},
                (InterlockedTraitKind::Learning, InterlockedTraitKind::Memory) => {},
                (InterlockedTraitKind::Learning, InterlockedTraitKind::Evolution) => {},
                (InterlockedTraitKind::SelfModification, InterlockedTraitKind::Evolution) => {},
                (InterlockedTraitKind::SelfModification, InterlockedTraitKind::HumanAuthority) => {},
                (InterlockedTraitKind::Motivation, InterlockedTraitKind::Drive) => {},
                (InterlockedTraitKind::Motivation, InterlockedTraitKind::Value) => {},
                (InterlockedTraitKind::Drive, InterlockedTraitKind::Motivation) => {},
                (InterlockedTraitKind::Value, InterlockedTraitKind::HumanAuthority) => {}, // via PrincipleOnion
                (InterlockedTraitKind::Consciousness, InterlockedTraitKind::MetaCognition) => {},
                (InterlockedTraitKind::SelfAwareness, InterlockedTraitKind::Consciousness) => {},
                (InterlockedTraitKind::Reflection, InterlockedTraitKind::MetaCognition) => {},
                (InterlockedTraitKind::Reflection, InterlockedTraitKind::Memory) => {},
                _ => panic!("互锁矩阵中不存在该依赖关系: A→B"),
            }
        };
    };
}

/// 强类型约束: 任何实现 InterlockedTrait 的类型必须提供 22 个 trait 全部 impl
/// 阶段 5 由 backend_engineer 实施时, 通过 blanket impl 强制所有 CentralAI 都满足
pub trait InterlockedTraitBundle:
    Perception + Signal + Cognition + Intuition + Reasoning + MetaCognition +
    Action + Execution + Expression +
    Memory + Recall + Consolidation +
    Evolution + Learning + SelfModification +
    Motivation + Drive + Value +
    Consciousness + SelfAwareness +
    HumanAuthority + Reflection
{
    fn interlocked_kind(&self) -> InterlockedTraitKind;
    fn interlock_check(&self) -> Result<(), InterlockError>;
}
```

---

## 5. 互锁验证函数 (Ponytail: 1 张表)

```rust
// docs/stage6/22-trait-interlock.md §5 — 运行时互锁验证

/// 互锁验证: 检查 type T 是否实现了所有需要的 trait
/// 编译期 hardcode: 22 个 trait 必须全部实现, 否则编译失败 (auto trait bound)
pub fn verify_interlock<T: InterlockedTraitBundle>() -> Result<(), InterlockError> {
    // 22 个 trait 的 static_assertion
    fn _check<T: InterlockedTraitBundle>() {
        // 编译期确认所有 trait 都已实现
        fn assert_impl<T: ?Sized + Perception>() {}
        fn assert_impl2<T: ?Sized + Signal>() {}
        // ... 22 个 assert_impl
        assert_impl::<T>();
        assert_impl2::<T>();
        // ... 编译器强制穷尽
    }
    Ok(())
}

/// 互锁矩阵静态查询 (编译期 const fn)
pub const fn interlock_lookup(a: InterlockedTraitKind, b: InterlockedTraitKind) -> bool {
    match (a, b) {
        (InterlockedTraitKind::Perception, InterlockedTraitKind::Signal) => true,
        // ... 30 条 lookup
        _ => false,
    }
}

/// 互锁失败错误 (运行时)
#[derive(Debug, thiserror::Error)]
pub enum InterlockError {
    #[error("trait {0:?} 缺少对 trait {1:?} 的互锁实现")]
    MissingDependency(InterlockedTraitKind, InterlockedTraitKind),
    #[error("trait {0:?} 不在 22 个互锁 trait 中")]
    NotInterlocked(InterlockedTraitKind),
    #[error("互锁矩阵非传递性: {0:?} → {1:?} 但缺少中间依赖")]
    NonTransitive(InterlockedTraitKind, InterlockedTraitKind),
}
```

---

## 6. 阶段 6 验证集成 (Ponytail: 1 行)

22 互锁 trait 与 `verification-protocol.md` 中的 M1 编译时验证集成:
- M1 编译时: `cargo check` 必须通过所有 `interlock_assert!` 调用 = 互锁矩阵一致性
- M2 启动时: `verify_interlock::<CentralAI>()` 必须返回 Ok = 22 trait 全部实现
- M3 首次对话: 端到端验证 22 个 trait 的运行时协同 (CognitiveDream 状态机)

---

## 7. 不修改承诺 (Ponytail: 1 张表)

| LOCKED 项 | 状态 |
|-----------|------|
| docs/stage1/, stage2/, stage3-blueprints/, stage4/, stage5/ | ✅ 未触碰 (仅在 docs/stage6/ 新建文件) |
| APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md | ✅ 未触碰 |
| APEIRETH-CONVENTIONS-*.md | ✅ 未触碰 |
| philosophy-traits-2026-07-30.md (V3 9 键 LOCKED) | ✅ 未触碰 (仅引用) |
| v1077_asi_v04_full_measurement.py (V0.5 LOCKED) | ✅ 未触碰 (仅引用 17→24 维提议) |
| v1136_asi_v05_3dim_real_measurement.py (V1136 LOCKED) | ✅ 未触碰 (仅引用 7→9 子测度提议) |
| 22 vs 43 trait 决策 | ✅ 引用阶段 4 §12.2 #1 待沉淀项, 不强压缩 |

---

## 8. 总结

22 互锁 trait 设计在阶段 4 §3 的 43 trait 基础上, 通过**收敛核心互锁 trait** + **真实 enum 编译期 hardcode** + **assertion macro 编译期互锁检查** 三层防御, 为阶段 6 M1/M2/M3 验证提供**可机械化检验**的互锁矩阵 — 不再依靠 reviewer 人工核对 43 个 trait 的依赖关系。

任务范围: 仅设计文档 + trait sketch (本文件 §2/§4/§5), 不写 impl 块 (留给阶段 5)。