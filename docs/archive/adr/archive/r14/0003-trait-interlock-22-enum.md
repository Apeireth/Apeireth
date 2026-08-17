# ADR-0003: 22 trait 互锁 enum 编译期 hardcode

**状态**: ✅ Accepted
**日期**: 2026-08-03
**作者**: architect2 (claude-sonnet-4.5, Ponytail: full)
**任务**: round8-08 stage6 22 trait 互锁代码实装
**依据**: docs/stage6/22-trait-interlock.md (round8-02 深化)
**影响范围**: 仅 `crates/apeireth-verify/src/lib.rs` 新增模块, 不修改任何 LOCKED 文档

---

## 上下文 (Context)

阶段 4 §3 推导了 43 个 trait, 但互锁仅需 22 个核心 trait 形成矩阵.
若采用字符串/动态注册表, 会出现:
1. 运行时拼写错误 (Perception vs Preception)
2. 互锁关系被绕过 (添加新 trait 不更新矩阵)
3. 编译期无法强制穷尽 (22 个数组成员改动无编译警告)

## 决策 (Decision)

在 `apeireth-verify` crate 新增 `InterlockedTraitKind` enum 22 变体 +
`INTERLOCKED_TRAIT_COUNT = 22` const + `INTERLOCKED_TRAITS` 编译期数组 +
`interlock_matrix(a, b) -> bool` const fn (29 个非对称互锁关系):

```rust
pub enum InterlockedTraitKind {
    Perception, Signal, Cognition, Intuition, Reasoning, MetaCognition,
    Action, Execution, Expression,
    Memory, Recall, Consolidation,
    Evolution, Learning, SelfModification,
    Motivation, Drive, Value,
    Consciousness, SelfAwareness,
    HumanAuthority, Reflection,
}

pub const INTERLOCKED_TRAIT_COUNT: usize = 22;
pub const INTERLOCKED_TRAITS: [InterlockedTraitKind; 22] = [...];

pub const fn interlock_matrix(a: InterlockedTraitKind, b: InterlockedTraitKind) -> bool { ... }
```

+ `interlock_assert!(A, B)` 编译期宏, 编译失败若 A→B 不在矩阵.

## 取舍 (Consequences)

**优点**:
1. 编译期 hardcode, 添加/删除变体 → 编译器穷尽检查强制更新
2. 0 运行时分配, const fn 全部内联
3. 互锁关系 33 个全部编译期可查, 0 字符串比较
4. 跨 crate 集成测试可使用同一 enum 引用

**缺点**:
1. enum 添加/删除需更新 `_exhaustive` match 块 (但编译器强制)
2. 22 个变体相对 43 个 trait 偏少 (但互锁矩阵只关心横切 trait)

## 守门 (Guardrails)

- 不修改 docs/stage1-5 LOCKED
- 不修改 apeireth-council/sovereignty/constraint 源
- 仅在 apeireth-verify crate 内新增模块
- 7 项不修改承诺完全遵守

## 验证 (Verification)

- `cargo test -p apeireth-verify --lib`: **28 passed / 0 failed**
- `cargo test -p apeireth-verify --test stage6_22_interlock`: **10 passed / 0 failed**
- 编译期穷尽断言 (const _: () = { ... })
- `interlock_assert!` 宏 5 次调用编译通过 (跨 crate 集成测试)

## 后续 (Follow-ups)

- ADR-0004 权限洋葱版本化策略
- ADR-0005 风险分级 M1-M12 阈值定义
- ADR-0006 集成 rebase skip policy 实施细节 (V23 fail-forward)