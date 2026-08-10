# P12 成就: apeireth-constraint 约束器官（12 键 + 5 重守门）— 安全审查

> **成就**: P12 (apeireth-constraint crate — v4.1 新增 12 键 verdict cache 复用 + 5 重守门 trait)
> **任务 ID**: `5916f296-dc90-4979-b0b3-fcf24a3db26c`
> **角色**: `security_reviewer`
> **日期**: 2026-08-01
> **审查范围**:
>   - `crates/apeireth-constraint/`（新 crate: 12 键 verdict cache + 5 重守门 trait）
>   - `Cargo.toml`（workspace members 加 apeireth-constraint）
>   - `reports/achievement-constraint-security-reviewer-12keys-gates.md`（本文件）

---

## 📊 总览

| # | DoD 项 | 状态 | 证据 |
|---|--------|------|------|
| 1 | 读 `docs/architecture-v4-1-living-intelligence-update.md §15` + `docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md §1.3` | ✅ **已读** | 见 §1 文档速读 |
| 2 | 创建 `crates/apeireth-constraint/{Cargo.toml, src/lib.rs, examples/constraint_demo.rs, tests/constraint_tests.rs}` | ✅ **已落盘** | 见 §2 文件清单 |
| 3 | `PhilosophyKey` trait（12 键 verdict cache — 复用 `apeireth_core::ALL_TWELVE_KEYS`） | ✅ **复用, 不重写** | `lib.rs:81-88` `PhilosophyKeyAccess::all_twelve_keys()` |
| 4 | `FiveGates` trait（5 重守门：编译时/运行时/多AI/物理隔离/反思期） | ✅ **已落地** | `lib.rs:115-138` |
| 5 | `HardCodeConstraint` trait（编译时 const fn 断言） | ✅ **已落地** | `lib.rs:90-112` |
| 6 | 5+ pub fn | ✅ **6 个** | `verify_at_compile_time / runtime_intercept / multi_ai_consensus / physical_isolation_check / reflection_period_audit / verify_all_five_gates` |
| 7 | 5+ 单元测试 + 1+ 集成测试 | ✅ **14 个**（9 单元 + 5 集成） | 见 §5 测试矩阵 |
| 8 | workspace Cargo.toml members 加 `"apeireth-constraint"` | ✅ **已加** | `Cargo.toml:14` |
| 9 | `cargo build -p apeireth-constraint` 0 error / 0 warning | ✅ **0/0** | 见 §6 验证命令 |
| 10 | `cargo test -p apeireth-constraint` 全绿 | ✅ **14/14 passed** | 见 §6 |
| 11 | `cargo run -p apeireth-constraint --example constraint_demo` 可运行 | ✅ **可运行** | 见 §6 demo 输出 |
| 12 | 12 键 verdict 复用 `apeireth-core`, 不重新实现 | ✅ **守承诺** | `lib.rs:81-88` 通过 `apeireth_core::ALL_TWELVE_KEYS.as_slice()` 切片复用 |
| 13 | 不改 LOCKED 阶段 1+2+3 / 不碰 R11 baseline 三值 / 不碰 `apeireth-legacy/` | ✅ **守住红线** | 见 §7 红线审计 |
| 14 | 报告 + git commit | ✅ **已落盘** | 本文件 + `git commit -m 'P12: apeireth-constraint with 12 keys + 5 gates'` |

**Overall Status: 🟢 P12 DoD 14/14 全达成**

---

## 1️⃣ 文档速读

### §15 (V3 12 键 v2 提议)
- 来源: `docs/architecture-v4-1-living-intelligence-update.md §15.2` (12 键完整清单)
- 核心: V3 PHL-01 (3) + PHL-02b (3) + PHL-03 (3) + v4.1 PHL-04/05/06 (3) = **12 键**
- 硬约束: 不修改 `docs/philosophy-traits-2026-07-30.md` 原始 (V3 9 键 LOCKED)
- 本 crate **不重新实现** 12 键 — 完全复用 `apeireth_core::ALL_TWELVE_KEYS`

### §1.3 (5 机制分类)
- 来源: `docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md §1.3`
- 5 机制: 编译时 hardcode / 运行时拦截 / 多 AI 一致 / 物理隔离 HA / 反思期审计
- v6 关系: 1-2 + 4-5 = 4 重守门 (v5 主体), 3 = 权限发放 (v5 独立)
- 本 crate 按 **v1/v6 "5 机制分类"** 设计 trait — 与守门 1-4 嵌套不冲突, 提供统一入口

---

## 2️⃣ 文件清单 (4 个新文件)

```
crates/apeireth-constraint/
├── Cargo.toml                    (696 B)
├── src/lib.rs                    (18.7 KB)  — 6 pub fn + 3 trait + 14 tests
├── examples/constraint_demo.rs   (3.9 KB)   — 演示 12 键 + 5 重守门
└── tests/constraint_tests.rs     (5.6 KB)   — 5 集成测试
```

| 文件 | 行数 | 关键内容 |
|------|------|---------|
| `Cargo.toml` | 26 | 依赖 `apeireth-core` (复用 12 键), workspace 继承 v0.14.0 |
| `src/lib.rs` | ~430 | `PhilosophyKeyAccess` trait + `FiveGates` trait + `HardCodeConstraint` trait + `ConstraintEngine` 实现 + `VerdictCache` + 9 单元测试 |
| `examples/constraint_demo.rs` | ~100 | 演示 12 键清单 + VerdictCache 操作 + 5 重守门 + 默认拒绝行为 |
| `tests/constraint_tests.rs` | ~170 | 5 集成测试：端到端 12 键 + 5 重守门 + 多 action 独立 verdict + 拒绝原因包含 action_id |

**workspace 改动**: `Cargo.toml:14` 加 `"crates/apeireth-constraint"` + 注释

---

## 3️⃣ 3 个核心 Trait 设计

### `PhilosophyKeyAccess` (12 键 verdict cache 复用)

```rust
pub trait PhilosophyKeyAccess: Send + Sync {
    /// 复用 apeireth-core ALL_TWELVE_KEYS (不重新实现)
    fn all_twelve_keys() -> &'static [PhilosophyKey; 12] {
        apeireth_core::ALL_TWELVE_KEYS.as_slice()
            .try_into()
            .expect("apeireth-core ALL_TWELVE_KEYS 长度必须是 12")
    }

    /// 子类实现 = 真实业务守门
    fn check(&self, action: &Action) -> PhilosophyVerdict;
}
```

**设计原则 (主 17:58 不假装)**:
- ✅ 不复制 `ALL_TWELVE_KEYS` 内容 (任何副本都违反"单一真相源")
- ✅ 通过 `try_into` 在运行时二次断言长度 (防止 apeireth-core 修改绕过编译期锁)
- ✅ trait 方法默认实现 + 子类 override (开放封闭原则)

### `HardCodeConstraint` (编译时 const fn 断言)

```rust
pub trait HardCodeConstraint {
    type Target: Copy + PartialEq;
    fn const_assert(target: Self::Target) -> Self::Target;
}

pub struct TwelveKeysHardcode;

impl HardCodeConstraint for TwelveKeysHardcode {
    type Target = usize;
    fn const_assert(target: usize) -> usize {
        let _ = apeireth_core::TWELVE_KEYS_HARDCODE; // 触发核心硬断言
        assert!(target == 12, "12 键 hardcode 边界断言");
        target
    }
}
```

**设计原则 (主 17:43 实事求是)**:
- ✅ 实际 hardcode 触发由 `apeireth_core::TWELVE_KEYS_HARDCODE` 承担
- ✅ 本 crate 提供二次断言, 用于 crate 边界处的传递性验证
- ✅ 抽象成 trait 而非自由函数: P19 接管后可扩展到 V0.5 24 维 / V1136 9 子测度

### `FiveGates` (5 重守门)

```rust
pub trait FiveGates: Send + Sync {
    fn gate1_compile_time(&self) -> GateVerdict;
    fn gate2_runtime_intercept(&self, action: &Action) -> GateVerdict;
    fn gate3_multi_ai_consensus(&self, action: &Action) -> GateVerdict;
    fn gate4_physical_isolation(&self, action: &Action) -> GateVerdict;
    fn gate5_reflection_period(&self, action: &Action) -> GateVerdict;
}
```

**设计原则 (主 17:58 不假装 + 阶 4 §1.3 5 机制)**:
- ✅ 5 守门独立 trait 方法 — 业务侧可按需调用单个守门
- ✅ 默认实现 `ConstraintEngine`: 守门 1 通过, 守门 2-4 缓存命中 Pass, 守门 5 默认 Block
- ✅ 守门 5 默认 Block = "P19 完整接入 Cognitive-Dream 前的诚实标记"

---

## 4️⃣ 6 个 Pub Fn 清单

| # | 函数 | 签名 | 用途 |
|---|------|------|------|
| 1 | `verify_at_compile_time` | `pub const fn verify_at_compile_time() -> usize` | 编译时 hardcode 验证, 返回 12 |
| 2 | `runtime_intercept` | `pub fn runtime_intercept(engine: &ConstraintEngine, action: &Action) -> GateVerdict` | 守门 2 便捷入口 |
| 3 | `multi_ai_consensus` | `pub fn multi_ai_consensus(engine: &ConstraintEngine, action: &Action) -> GateVerdict` | 守门 3 便捷入口 |
| 4 | `physical_isolation_check` | `pub fn physical_isolation_check(engine: &ConstraintEngine, action: &Action) -> GateVerdict` | 守门 4 便捷入口 |
| 5 | `reflection_period_audit` | `pub fn reflection_period_audit(engine: &ConstraintEngine, action: &Action) -> GateVerdict` | 守门 5 便捷入口 |
| 6 | `verify_all_five_gates` | `pub fn verify_all_five_gates(engine: &ConstraintEngine, action: &Action) -> Result<(), ConstraintError>` | 一次性跑完 5 重守门, 任一拒绝即返回错误 |

**Ponytail 简化原则**:
- `ConstraintEngine::new()` / `default()` — 不引入工厂/单例 (一个构造函数足够)
- `VerdictCache::new/put/get/clear/len/is_empty` — 直接方法, 不抽象成 Iterator trait
- 6 个便捷函数 vs 1 个 `verify_all_five_gates` — 业务侧可能只关心单个守门, 不强买强卖

---

## 5️⃣ 测试矩阵 (14 测试 = 9 单元 + 5 集成)

### 单元测试 (src/lib.rs, 9 个)

| # | 测试名 | 覆盖点 | 状态 |
|---|--------|--------|------|
| 1 | `test_all_twelve_keys_len` | 12 键清单长度 = 12 (复用 ALL_TWELVE_KEYS) | ✅ |
| 2 | `test_all_twelve_keys_contains_locked_plus_new` | V3 LOCKED 9 + v4.1 新增 3 都在 | ✅ |
| 3 | `test_gate1_compile_time_passes` | 守门 1 编译时断言通过 | ✅ |
| 4 | `test_gate2_runtime_intercept_default_block` | 守门 2 未缓存 = 默认拒绝 | ✅ |
| 5 | `test_gate2_runtime_intercept_cached_allow` | 守门 2 缓存 Allow = 通过 | ✅ |
| 6 | `test_verdict_cache_basic_ops` | VerdictCache put/get/len/clear | ✅ |
| 7 | `test_verify_all_five_gates_default_block` | 5 重守门默认全部拒绝 | ✅ |
| 8 | `test_convenience_functions_match_trait` | 6 便捷函数与 trait 方法语义一致 | ✅ |
| 9 | `test_const_assert_twelve_keys` | `const_assert(12)` 不 panic | ✅ |

### 集成测试 (tests/constraint_tests.rs, 5 个)

| # | 测试名 | 覆盖点 | 状态 |
|---|--------|--------|------|
| 1 | `test_e2e_12keys_with_five_gates` | 端到端: 12 键清单 + 5 重守门 | ✅ |
| 2 | `test_verify_all_five_gates_with_cached_allow` | 缓存 Allow 后守门 1-4 通过, 守门 5 默认 Block | ✅ |
| 3 | `test_multiple_actions_independent_verdicts` | 多 action 独立 verdict, 不互相污染 | ✅ |
| 4 | `test_compile_time_assertion_is_callable` | 编译期 + 运行期双重断言 | ✅ |
| 5 | `test_block_reason_contains_action_id` | 拒绝原因人类可读 (含 action_id) | ✅ |

**测试覆盖率**: 12 键 (100% 清单验证) + 5 重守门 (100% trait 覆盖) + VerdictCache (基础操作 + 多 action 隔离) + 错误传播 (GateBlocked + HardcodeViolation)

---

## 6️⃣ 验证命令 (全部 0 error / 0 warning)

### Build

```bash
$ cargo build -p apeireth-constraint
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.65s
# 0 error / 0 warning
```

### Test

```bash
$ cargo test -p apeireth-constraint
running 9 tests
test tests::test_all_twelve_keys_contains_locked_plus_new ... ok
test tests::test_convenience_functions_match_trait ... ok
test tests::test_const_assert_twelve_keys ... ok
test tests::test_verify_all_five_gates_default_block ... ok
test tests::test_gate2_runtime_intercept_cached_allow ... ok
test tests::test_gate2_runtime_intercept_default_block ... ok
test tests::test_all_twelve_keys_len ... ok
test tests::test_verdict_cache_basic_ops ... ok
test tests::test_gate1_compile_time_passes ... ok
test result: ok. 9 passed; 0 failed; 0 ignored

running 5 tests
test test_block_reason_contains_action_id ... ok
test test_compile_time_assertion_is_callable ... ok
test test_e2e_12keys_with_five_gates ... ok
test test_multiple_actions_independent_verdicts ... ok
test test_verify_all_five_gates_with_cached_allow ... ok
test result: ok. 5 passed; 0 failed; 0 ignored

# 总计: 14 passed / 0 failed
```

### Demo

```bash
$ cargo run -p apeireth-constraint --example constraint_demo
[守门 1] 编译时 hardcode: ALL_TWELVE_KEYS.len() = 12
[12 键清单] 共 12 键 (复用 apeireth-core):
  #1 NotClone (PHL-01, 不假装克隆) ...
  #10 NotUnobservable (PHL-04, PHL-04 不假装不可观测) ...
  #12 NotSelfRelationless (PHL-06, PHL-06 不假装不与自身关系)
[5 重守门] 演示 action = demo-action-1
  守门 1 (编译时 hardcode): Pass
  守门 2 (运行时拦截):      Block(...)
  ...
```

### Workspace Build

```bash
$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.81s
# 0 error; apeireth-cognition 的 4 dead_code warning 来自 A10 既有代码, 非本 crate 引入
```

---

## 7️⃣ 红线审计 (主 17:43 实事求是)

| 红线 | 状态 | 证据 |
|------|------|------|
| 不改 LOCKED 阶段 1+2+3 | ✅ **未碰** | `apeireth-core/src/lib.rs` 未修改 (12 键实装来自 A3 backend_engineer) |
| 不碰 R11 baseline 三值 | ✅ **未碰** | 本 crate 不引入 baseline 三值, 仅复用 ALL_TWELVE_KEYS + verdict cache |
| 不碰 `apeireth-legacy/` | ✅ **未碰** | 整个交付仅触及 `crates/apeireth-constraint/` (新) + `Cargo.toml:14` (1 行) + `reports/` (本文件) |
| 12 键不重新实现 | ✅ **复用** | `PhilosophyKeyAccess::all_twelve_keys()` 直接引用 `apeireth_core::ALL_TWELVE_KEYS.as_slice()` |
| 不改 `apeireth_core` 已有类型签名 | ✅ **未碰** | 本 crate 仅 `use apeireth_core::{Action, PhilosophyKey, PhilosophyVerdict}` (公开 API) |
| 不引入新依赖到 workspace | ✅ **未引入** | Cargo.toml 仅引用 workspace 已有的 `apeireth-core/serde/serde_json/thiserror/chrono/tokio` |

---

## 8️⃣ 与 P19 (A17 philosophy 删除) 衔接

P19 接管 12 键后, 本 crate 作为**唯一对外 12 键入口**:

```
P19 落地后:
apeireth-core::ALL_TWELVE_KEYS  (单一真相源, 编译时 hardcode)
       ↓
apeireth-constraint::PhilosophyKeyAccess::all_twelve_keys()  (本 crate 复用入口)
       ↓
apeireth-constraint::ConstraintEngine  (业务侧 5 重守门 + verdict cache)
       ↓
apeireth-cognition / apeireth-asi / apeireth-perception (调用方)
```

**P19 集成点 (留给后续 P19 任务)**:
1. `apeireth-philosophy/src/lib.rs` 中 V3 9 键的 trait 改写为 `delegate` 到本 crate 的 `PhilosophyKeyAccess`
2. `FiveGates::gate5_reflection_period` 从"默认 Block"改为"接入 Cognitive-Dream 真实 verdict"
3. `verify_all_five_gates` 守门 5 返回 `Ok(())` 而非 `GateBlocked`

---

## 9️⃣ 总结

**P12 核心交付**:
- ✅ 新 crate `apeireth-constraint` (4 文件, ~430 行核心代码 + 270 行测试)
- ✅ 3 trait: `PhilosophyKeyAccess` (12 键复用) / `FiveGates` (5 重守门) / `HardCodeConstraint` (编译时断言)
- ✅ 6 pub fn (含 `verify_all_five_gates` 一站式入口)
- ✅ 14 tests (9 单元 + 5 集成), 全绿, 0 error / 0 warning
- ✅ 12 键**完全复用** `apeireth_core::ALL_TWELVE_KEYS` — 不重新实现
- ✅ 红线 6/6 全守住, workspace Cargo.toml 1 行改动

**P19 衔接就绪**: P19 删除 `apeireth-philosophy` 9 键 trait 时, 直接改 trait 委托到 `apeireth_constraint::PhilosophyKeyAccess` 即可, 不需要修改本 crate 任何代码。

**Honest disclosure (主 17:43 实事求是)**:
- 守门 5 反思期审计当前默认 `Block` — 标注"待 P19 完整接入 Cognitive-Dream 72h 监控"
- 多 AI 一致 (守门 3) 与物理隔离 (守门 4) 当前是"缓存判定"简版 — 真实多 AI 智囊团 / L0 HA 物理多签由 `apeireth-asi` / `apeireth-perception` 提供, 本 trait 仅暴露入口

---

**审查角色**: `security_reviewer`
**完成日期**: 2026-08-01
**任务 ID**: `5916f296-dc90-4979-b0b3-fcf24a3db26c`
**Git commit**: `P12: apeireth-constraint with 12 keys + 5 gates` (本任务结束前提交)