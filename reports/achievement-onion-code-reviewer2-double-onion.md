# P16 成就报告 — apeireth-onion 双洋葱统一体 trait 抽象层

- **任务 ID**: P16
- **角色**: code_reviewer2
- **日期**: 2026-08-01
- **状态**: ✅ 完成（10 单元测试 + 2 集成测试 + 1 example 全部通过）

---

## 一、交付物

```
crates/apeireth-onion/
├── Cargo.toml              (依赖: apeireth-core + serde + serde_json + thiserror, 0 新增依赖)
├── src/
│   └── lib.rs              (684 行, 含 10 单元测试)
├── examples/
│   └── onion_demo.rs       (双洋葱 + AND 门 + 仲裁 demo)
└── tests/
    └── onion_tests.rs      (2 集成测试: e2e + offline HA)
```

Workspace 注册：`Cargo.toml` `members` 数组新增 `"crates/apeireth-onion"`。

---

## 二、设计原则

### 2.1 比喻 vs 架构分离（§19.4）

```text
   比喻 (双洋葱)         ← §19.4 对外心智模型
        ↓
   架构 (本 trait 层)    ← PREREQ-2 §4 6 组件的"接口骨架"
        ↓
   实现 (apeireth-core)  ← struct data + serde + verdict cache
```

**关键决策**：`apeireth-onion` 不重复定义 `PrincipleOnion` / `PermissionOnion` struct（避免与
`apeireth-core` 双重事实来源），而是通过 **trait 抽象** 提供统一接口。`apeireth-core` 的
struct 通过 blanket impl 适配为 trait 实现者。

### 2.2 编译时 hardcode（5 + 6 = 11）

```rust
pub const PRINCIPLE_LAYERS_OUTER_IN: [PrincipleLayerKind; 5] = [E, S, A, M, O];
pub const PERMISSION_LAYERS_OUTER_IN: [PermissionLayerKind; 6] = [L0..L5];
pub const ELECTRONIC_RING_LEN: usize = 11;

const _: () = {
    assert!(PRINCIPLE_LAYERS_OUTER_IN.len() == 5);
    assert!(PERMISSION_LAYERS_OUTER_IN.len() == 6);
    assert!(... == ELECTRONIC_RING_LEN);
};
```

零依赖编译期断言（无 `static_assertions` crate，符合"不引入新依赖"约束）。

---

## 三、Trait 契约

| Trait | 职责 | 实现者 |
|-------|------|--------|
| `PrincipleSlice` | 5 原则层切片 | `apeireth_core::PrincipleLayer` |
| `PermissionSlice` | 6 权限层切片 | `apeireth_core::PermissionLayer` |
| `PrincipleOnion` | 5 层枚举 + 仲裁 | `apeireth_core::PrincipleOnion` + `DefaultDoubleOnion` |
| `PermissionOnion` | 6 层枚举 + HA 查询 | `apeireth_core::PermissionOnion` + `DefaultDoubleOnion` |
| `DoubleOnionUnification` | V1+V2+V3 AND 门 | `DefaultDoubleOnion` |
| `ElectronicRingNetwork` | 11 节点电子环视图 | `DefaultDoubleOnion` |

### 3.1 AND 门语义（ADR-0001）

```rust
fn unify_check(&self, action: &OnionAction) -> OnionVerdict {
    // V3 先检查（HA 物理隔离最强约束 — 离线模式直接拒绝）
    // V1 原则检查（L5 由 E 层兜底拒绝）
    // V2 权限检查
    // AND 门全通过 → Allow
}
```

任何一票反对 = 独立拒绝。

### 3.2 跨层冲突仲裁（§3.6）

```
E > S > A > M > O
硬编码层永不输（实现者可 override 但必须保持"硬编码永不输"语义）
```

---

## 四、验证结果

```
running 10 tests (unit)
test tests::t1_principle_layers_hardcoded_count_is_5 ... ok
test tests::t2_permission_layers_hardcoded_count_is_6 ... ok
test tests::t3_electronic_ring_capacity_is_11 ... ok
test tests::t4_arbitrate_e_always_wins ... ok
test tests::t5_unify_check_l5_blocked_by_e_layer ... ok
test tests::t6_unify_check_l1_normal_allows ... ok
test tests::t7_l0_requires_ha_is_always_true ... ok
test tests::t8_electronic_ring_node_partition ... ok
test tests::t9_default_impl_composes_core_structs ... ok
test tests::t10_electronic_ring_overflow_panics ... ok

running 2 tests (integration)
test integration_double_onion_unity_e2e ... ok
test integration_offline_ha_rejects_critical ... ok

test result: ok. 12 passed; 0 failed
```

### Demo 输出

```
[3/5] 11 节点电子环
  - 节点总数: 11 (complete: true)
  - 序列: Operational → Methodology → Accumulation → Spirit → Existence → L5 → L4 → L3 → L2 → L1 → L0

[4/5] V1+V2+V3 AND 门
  ✓ 4.1 日常读 (L1)                   Allow (11 层)
  ✓ 4.2 关键操作 (L3, HA SingleHuman) Allow (11 层)
  ✗ 4.3 核武器级 (L5) — E 层兜底      BlockByPrinciple(Existence)
```

---

## 五、禁止项遵守

| 约束 | 状态 |
|------|------|
| 不修改 `apeireth-core` 已实装 struct | ✅ 仅 blanket impl 适配 |
| 不修改 LOCKED 阶段 1/2/3 文件 | ✅ 0 个 LOCKED 文件被触碰 |
| 不引入新依赖 | ✅ 仅 workspace.dependencies（apeireth-core + serde + serde_json + thiserror） |
| 不引入 I/O | ✅ 全 crate 0 个文件 I/O 调用 |
| 不引入 unsafe | ✅ `#![deny(unsafe_code)]` + 0 个 unsafe 块 |
| 5+ 单元测试 | ✅ 10 个 |
| 1+ 集成测试 | ✅ 2 个 |
| 1+ example | ✅ 1 个 |

---

## 六、协作集成提示

### 6.1 给其他 crate 接入的接口

```rust
use apeireth_onion::{
    DoubleOnionUnification, ElectronicRingNetwork, OnionAction, OnionVerdict,
    PrincipleLayerKind, PermissionLayerKind,
};

// 1. 调用统一判定 API
let verdict = my_double_onion.unify_check(&action);

// 2. 检查电子环完整性
assert!(my_double_onion.ring_is_complete());

// 3. 实现自己的 DoubleOnionUnification（替代比喻如"航空母舰 + 机库"）
struct MyCarrierOnion { ... }
impl DoubleOnionUnification for MyCarrierOnion { ... }
```

### 6.2 未来扩展点（不动）

- §19.4 候选比喻替换（"双根 + 枝叶" / "航空母舰 + 机库"）：本 trait 层即为此预留接口
- Cognitive-Dream 6 状态机接入电子环（trait `ElectronicRingNetwork` 可扩展状态）

---

## 七、诚实登记

### 7.1 多 agent 并发冲突（已知）

本次执行期间 `Cargo.toml` workspace 多次被其他 P 任务并发修改（含 rebase 冲突标记），
最终导致 `apeireth-onion` 目录一度被外部操作删除（已重建）。最终本地 `cargo build -p
apeireth-onion --examples --tests` 全绿验证已完成；最终 workspace 一致性需 integration
阶段协调。

### 7.2 V1+V2+V3 简化实现

为最小可用落地（5+ pub fn + 5+ tests），`DefaultDoubleOnion::unify_check` 的 V1/V2/V3 检查
为简化版（演示原则嵌入权限 + AND 门 + 仲裁三核心）。完整 12 键 verdict cache + 5 重守门 +
verdict chain 见 `apeireth-constraint` crate（P12 task）。

### 7.3 §19.4 候选比喻替换

本 trait 层已为比喻替换预留接口，但未实现具体替代 impl（"航空母舰 + 机库" / "双根 + 枝叶"
等）。待 阶段 5+ 由其 owner 落地。

---

## 八、签名

- 角色: code_reviewer2
- 任务: P16 双洋葱统一体 trait 抽象层
- 文件: 4 个新文件 + 1 行 workspace Cargo.toml
- 测试: 12 passed (10 unit + 2 integration)
- 依赖: 0 新增（仅 workspace 已有）
- 状态: ✅ 完成
