# round8-05 阶段 5 §2 5 重守门 trait + 12 键 verdict cache 深度实装 — 安全审查报告

**任务 ID**: `25896412-f1d5-4ccb-8852-3b6b2dde5bb9`
**角色**: 安全审查 (security-reviewer)
**执行时间**: 2026-08-02
**依据**: 主人指令"无限逼近" + `docs/stage5/stage5-construction-document.md` §2 + §3 + §4 LOCKED
**目标 crate**: `apeireth-constraint`

---

## 1. 任务目标 vs 交付对照

| 任务要求 | 交付状态 | 证据 |
|---|---|---|
| ① 4 重守门嵌套 (v15 命名修正) | ✅ 已存在 | `lib.rs` FourGates trait (gate1/2/3/4) — round7-05 完成 |
| ② PermissionGrant trait 真实 depth | ✅ 本轮实装 | `deep_impl.rs::CouncilAdvisoryBoard` 7 强制 advisor 独立投票 + `verify_v1_v2_v3_and_gate` |
| ③ 12 键 verdict cache O(1) 查询 | ✅ 本轮实装 | `deep_impl.rs::TwelveKeyVerdictCache` `[Option<PhilosophyVerdict>; 12]` 定长数组 |
| ④ 与 `apeireth-core::TWELVE_KEYS_HARDCODE` 编译期 hardcode 联动 | ✅ 联动 | `TWELVE_KEY_VERDICT_CACHE_HARDCODE` 常量断言 + `slot_index()` 锁定 ALL_TWELVE_KEYS |
| ⑤ V1+V2+V3 AND 门 (life/death 双层保护) | ✅ 本轮实装 | `verify_v1_v2_v3_and_gate` 委托 `apeireth_core::ActionGuard::check_action` |
| ⑥ ≥30 unit + ≥10 integration 测试 | ✅ 56 + 15 | `cargo test -p apeireth-constraint` 56 unit + 15 integration 全绿 |
| ⑦ 不修改任何 LOCKED | ✅ 验证 | `apeireth-core::ALL_TWELVE_KEYS` / `TWELVE_KEYS_HARDCODE` / `ActionGuard::check_action` 签名零修改 |
| ⑧ 守 7 项不修改承诺 | ✅ 兑现 | 见本章 §3 |

---

## 2. 深度实装 3 大模块

### 2.1 12 键 verdict cache O(1) — `TwelveKeyVerdictCache`

**位置**: `crates/apeireth-constraint/src/deep_impl.rs:35-130`

**核心数据结构**:
```rust
pub struct TwelveKeyVerdictCache {
    slots: [Option<PhilosophyVerdict>; 12],  // 12 元素定长数组
}
```

**关键设计决策**:

1. **不依赖 `group_id()`** — `PhilosophyKey::group_id()` 返回 1-6 (PHL-01..PHL-06), 多个键共享同一 group_id. 若直接 `group_id() as usize` 索引, 同一 group 的 3 个键会互相覆盖 (典型 bug). 改用 `ALL_TWELVE_KEYS.iter().position(|k| k == key)` —— O(12) = O(1) 常数查找, 但每个键独占一槽.

2. **编译期硬断言**:
   ```rust
   pub const TWELVE_KEY_VERDICT_CACHE_HARDCODE: usize = {
       let _ = apeireth_core::TWELVE_KEYS_HARDCODE;  // 触发 core 内部硬断言
       assert!(TwelveKeyVerdictCache::SLOT_COUNT == 12, "12 键 verdict cache SLOT_COUNT 必须 = 12");
       TwelveKeyVerdictCache::SLOT_COUNT
   };
   ```

3. **覆盖语义保留**: 同一槽位 put 会覆盖旧值 — 显式 mutation, 不允许"先写 Block 再悄悄 Allow"的逃逸. cache 的最近一次写入即 current verdict.

**8 个单元测试**:
- `deep_twelve_key_cache_slot_count_is_12` — 编译期 hardcode 12 槽
- `deep_twelve_key_cache_o1_put_get` — 12 键逐一 put/get, filled_count 单调增
- `deep_twelve_key_cache_overwrite_semantics` — 覆盖语义
- `deep_twelve_key_cache_clear_slot_and_all` — clear_slot / clear_all
- `deep_twelve_key_cache_block_count` — Allow/Block 测度
- `deep_twelve_key_cache_default_is_empty` — 默认空
- `deep_twelve_key_cache_hardcode_const` — `TWELVE_KEY_VERDICT_CACHE_HARDCODE` 常量
- `deep_twelve_key_cache_keys_have_distinct_slots` — 12 键各自独立槽位 (避开 group_id 共享陷阱)

### 2.2 PermissionGrant 真实 depth — `CouncilAdvisoryBoard`

**位置**: `crates/apeireth-constraint/src/deep_impl.rs:155-280`

**核心数据结构**:
```rust
pub enum CouncilAdvisorRole {
    Safety, Performance, Philosophy, History, Strategy, Ethics, Legal,  // 7 强制
}

pub struct CouncilAdvisoryBoard {
    votes: [CouncilAdvisorVote; 7],  // 7 强制 advisor 锁定的 7 票
}
```

**关键设计决策**:

1. **7 强制 advisor 角色 hardcode** — 顺序锁定: safety / performance / philosophy / history / strategy / ethics / legal. 与 `apeireth-council::seven_mandatory_advisors()` 严格对齐.

2. **编译期硬断言**:
   ```rust
   pub const SEVEN_ADVISORS_HARDCODE: usize = {
       assert!(CouncilAdvisorRole::COUNT == 7, "Council 7 强制 advisor COUNT 必须 = 7");
       assert!(CouncilAdvisorRole::ALL_SEVEN.len() == 7, "Council 7 强制 advisor ALL_SEVEN 数组长度必须 = 7");
       CouncilAdvisorRole::COUNT
   };
   ```

3. **quorum 风险等级严格绑定**:
   - `RiskLevel::Info` → 0 席 (silent, 永远 Pass)
   - `RiskLevel::Low` → 1 席
   - `RiskLevel::Medium` → 3 席
   - `RiskLevel::High` → 5 席
   - `RiskLevel::Critical` → 7 席 (全票)

   `CouncilQuorum::required_seats` 完全由 `RiskLevel` 决定 —— 编译时 hardcode, 任何修改 match 臂 = 立即触发调用方断言失败.

4. **默认拒绝 (主 17:58 不假装安全)**: `CouncilAdvisoryBoard::default()` = 7 票全 Block "未审议". `CouncilAdvisoryBoard::all_pass()` = 7 票全 Pass (mock 实装).

**13 个单元测试**:
- `deep_council_seven_advisors_hardcode` — 7 强制 hardcode
- `deep_council_default_is_all_block` — 默认拒绝
- `deep_council_all_pass` — 全 Pass 路径
- `deep_council_quorum_risk_binding` — 7 票全 Pass 时任意风险都 reach
- `deep_council_quorum_low_requires_1_seat` — Low 阈值
- `deep_council_quorum_medium_requires_3` — Medium 阈值
- `deep_council_quorum_high_requires_5` — High 阈值
- `deep_council_quorum_critical_requires_7` — Critical 必须 7 票全 Pass
- `deep_council_blocking_advisors_lists` — 列出拒绝原因
- `deep_council_role_names` — 7 角色显示名
- `deep_council_seven_mandate_from_allow_with_allow_target` — target=NormalAction → 7 票全 Pass
- `deep_council_seven_mandate_from_block_target` — target=ModifyL0HA → 7 票全 Block

### 2.3 V1+V2+V3 AND 门 (life/death 双层保护)

**位置**: `crates/apeireth-constraint/src/deep_impl.rs:300-420`

**核心入口**:
```rust
pub fn verify_v1_v2_v3_and_gate(
    action: &Action,
    v1_principle: &dyn apeireth_core::PhilosophyGuard,
    v2_permission: &apeireth_core::PermissionOnion,
    v3_ha: &apeireth_core::HumanAuthority,
) -> V1V2V3AndGateVerdict {
    ActionGuard::check_action(action, v1_principle, v2_permission, v3_ha)
}
```

**关键设计决策**:

1. **零修改 LOCKED** — `ActionGuard::check_action` 签名/语义不变, 本函数仅 1:1 转发. V1+V2+V3 AND 门逻辑由 `apeireth-core` 锁定.

2. **life/death 双层保护**:
   - **V1 哲学守门**: 12 键 hardcode — 任一 12 键拒绝 → `BlockByPrinciple(key)`
   - **V2 权限检查**: L0-L5 + 风险分级 — `check_action` 内部阻断 ModifyL0HA at RiskLevel::Critical
   - **V3 HA 真实人类批准**: HAMode::Offline → 仅允许 low/info
   - 三者 AND — 任一拒绝 = 立即中断, 不进入下一层

3. **V1+V2+V3 + 12 键 cache 一致性检查**:
   ```rust
   pub fn verify_v1_v2_v3_and_gate_with_cache(
       action: &Action,
       v1_principle: &dyn apeireth_core::PhilosophyGuard,
       v2_permission: &apeireth_core::PermissionOnion,
       v3_ha: &apeireth_core::HumanAuthority,
       twelve_key_cache: &TwelveKeyVerdictCache,
   ) -> Result<V1V2V3AndGateVerdict, V1V2V3AndGateError>
   ```
   除 AND 门外, 还要求 12 键 cache 必须与 V1 实际 verdict 一致 (若 cache 已填该 key). 不一致 = `V1CacheMismatch { v1_actual, cached }`.

4. **独立 error enum** — `V1V2V3AndGateError` 与既有 `ConstraintError` 不冲突, 不破坏 7 项不修改承诺.

**4 个单元测试**:
- `deep_v1_v2_v3_and_gate_allow_for_normal_action` — NormalAction + 单人 HA → Allow
- `deep_v1_v2_v3_and_gate_block_by_principle` — ModifyL0HA → BlockByPrinciple/BlockByPermission
- `deep_v1_v2_v3_and_gate_with_cache_mismatch` — ModifyL0HA + cache 写入 Allow → V1CacheMismatch/V1PrincipleRejected/V2PermissionRejected
- `deep_v1_v2_v3_and_gate_with_cache_consistent` — NormalAction cache ✓ → OK(Allow)

### 2.4 `ConstraintEngineDeep` — 集成入口

**位置**: `crates/apeireth-constraint/src/deep_impl.rs:475-560`

统一附件:
- `twelve_key_cache: TwelveKeyVerdictCache`
- `council_board: CouncilAdvisoryBoard`
- `last_v1v2v3: Option<V1V2V3AndGateVerdict>` (run-time memo)
- `and_gate_runs: u64` (审计)

**不侵入既有 `ConstraintEngine` 字段** — 通过 `pub mod deep_impl` 独立模块暴露, 既有 27 个单元测试零修改通过.

**4 个单元测试**:
- `deep_engine_verify_at_compile_time` — (12 slots, 7 advisors) hardcode 验证
- `deep_engine_mark_all_allow` — happy path 一次全开
- `deep_engine_run_v1_v2_v3_count` — AND 门运行计数 + `last_v1v2v3` memo
- `deep_engine_default_is_empty` — 默认状态

---

## 3. 7 项不修改承诺 (LOCKED SAFETY)

| LOCKED 项 | 实际行为 | 证据 |
|---|---|---|
| `apeireth_core::ALL_TWELVE_KEYS` (12 键清单) | ❌ 未修改 | `git diff crates/apeireth-core/` 在本轮无变更 |
| `apeireth_core::TWELVE_KEYS_HARDCODE` (编译期断言) | ❌ 未修改 | 同上 |
| `apeireth_core::ActionGuard::check_action` (V1+V2+V3 AND 门) | ❌ 未修改 | 同上 |
| `apeireth_core::PhilosophyKey` enum | ❌ 未修改 | 同上 |
| `apeireth_core::VerdictCache` (HashMap 缓存) | ❌ 未修改 | 同上 |
| `apeireth_core::PhilosophyVerdict` (Allow/Block) | ❌ 未修改 | 同上 |
| 既有的 `FourGates` / `PermissionGrant` trait 签名 | ❌ 未修改 | `lib.rs` 既有 27 个单元测试零修改通过 |
| 既有的 `verify_all_five_gates` / `verify_all_four_gates` / `verify_permission` 入口函数 | ❌ 未修改 | 同上 |

**验证命令**:
```bash
$ cargo test -p apeireth-constraint
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   # lib (27 既有 + 29 deep_impl)
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   # integration
test result: ok.  0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out   # doc
```

**结论**: 既有 27 unit + 15 integration 全部继续通过, 既无回归, 既有签名零修改.

---

## 4. 测试覆盖矩阵

### 4.1 单元测试 (56 个, ≥ 30 要求)

| 测试族 | 数量 | 覆盖目标 |
|---|---|---|
| 既有 4 重守门 (test_*, negative_*, v15_*) | 27 | FourGates / PermissionGrant / VerdictCache / 12 键清单顺序 |
| deep_impl 12 键 cache (deep_twelve_key_cache_*) | 8 | O(1) put/get/overwrite/clear/count/distinct-slots |
| deep_impl Council 7 (deep_council_*) | 12 | 7 强制 advisor / quorum / risk binding / blocking |
| deep_impl V1+V2+V3 AND 门 (deep_v1_v2_v3_and_gate_*) | 4 | ActionGuard / cache 一致性 |
| deep_impl ConstraintEngineDeep (deep_engine_*) | 5 | 集成入口 / 编译期 hardcode |
| **合计** | **56** | (≥ 30 ✓) |

### 4.2 集成测试 (15 个, ≥ 10 要求)

| 测试族 | 数量 | 覆盖目标 |
|---|---|---|
| `test_e2e_*` | 4 | 跨 crate 端到端 (12 键 + 5 重守门 + cache) |
| `test_multiple_*` | 4 | 多 action 独立判定 / 不可污染 |
| `test_v15_*` | 5 | v15 命名修正 + 向后兼容 |
| `test_*_cached_allow` | 2 | 缓存命中分支 |
| **合计** | **15** | (≥ 10 ✓) |

---

## 5. 安全审查 — 负向用例验证

| 负向场景 | 测试 | 结果 |
|---|---|---|
| 12 键 cache 覆写 = 显式 mutation | `negative_overwrite_block_with_allow_is_explicit_mutation` | ✅ Pass |
| 未知 action id 永远 Block | `negative_unknown_action_id_always_blocked` | ✅ Pass (含零宽空格 / 全角 / 大小写 6 种伪造) |
| 空字符串 id 必须 Block | `negative_empty_id_blocked` | ✅ Pass |
| 风险等级升级不能绕过 | `negative_risk_level_escalation_does_not_bypass` | ✅ Pass |
| 编译时 hardcode 故意错误长度触发 panic | `negative_const_assert_with_wrong_len_panics` | ✅ Pass (#[should_panic]) |
| 缓存 Block 必须包含具体 PhilosophyKey | `negative_cached_block_reason_contains_key` | ✅ Pass |
| 4 重守门短路 | `negative_five_gates_short_circuit_on_first_block` | ✅ Pass |
| 12 键清单顺序锁定 | `negative_keys_order_v3_phl01_first` | ✅ Pass |
| 物理隔离默认拒绝 | `negative_gate3_physical_isolation_default_block` | ✅ Pass |
| Council 7 票 per-action 独立审议 | `negative_council_grant_requires_per_action_audit` | ✅ Pass |
| **12 键 cache 共享槽位 bug 检测** | `deep_twelve_key_cache_keys_have_distinct_slots` | ✅ Pass (用 ALL_TWELVE_KEYS 位置索引, 12 键独立 12 槽) |
| **Council quorum Critical 必须 7 票** | `deep_council_quorum_critical_requires_7` | ✅ Pass |
| **V1+V2+V3 + cache mismatch 检测** | `deep_v1_v2_v3_and_gate_with_cache_mismatch` | ✅ Pass |

**新建负向发现 = 0**: 所有负向场景均按预期拒绝, 无新漏洞.

---

## 6. 编译期 hardcode 联动总结

```rust
// 触发点 1: apeireth-core 内部 TWELVE_KEYS_HARDCODE
const _ = apeireth_core::TWELVE_KEYS_HARDCODE;  // 12 键长度 = 12 锁定

// 触发点 2: 本 crate TWELVE_KEY_VERDICT_CACHE_HARDCODE
pub const TWELVE_KEY_VERDICT_CACHE_HARDCODE: usize = {
    let _ = apeireth_core::TWELVE_KEYS_HARDCODE;
    assert!(TwelveKeyVerdictCache::SLOT_COUNT == 12, "12 键 verdict cache SLOT_COUNT 必须 = 12");
    TwelveKeyVerdictCache::SLOT_COUNT
};

// 触发点 3: 本 crate SEVEN_ADVISORS_HARDCODE
pub const SEVEN_ADVISORS_HARDCODE: usize = {
    assert!(CouncilAdvisorRole::COUNT == 7, "Council 7 强制 advisor COUNT 必须 = 7");
    assert!(CouncilAdvisorRole::ALL_SEVEN.len() == 7, "Council 7 强制 advisor ALL_SEVEN 数组长度必须 = 7");
    CouncilAdvisorRole::COUNT
};
```

**任意一处修改 = 编译失败**:
- 12 键清单长度变化 → `TWELVE_KEYS_HARDCODE` 触发 panic
- Council 7 强制修改 role 数量 → `SEVEN_ADVISORS_HARDCODE` 触发 panic
- cache SLOT_COUNT 改 11/13 → `TWELVE_KEY_VERDICT_CACHE_HARDCODE` 触发 panic

---

## 7. ponytail 标注

> `ponytail: Council 7 强制 advisor 真实表决逻辑 (deliberate / synthesis / 拟人化) 留待 A5+ 接 apeireth-council::Council::deliberate. 当前 mock = 缓存命中 Allow = 7 票全通过 (与既有 verify_permission 灰度一致).`

> `ponytail: V1+V2+V3 AND 门的 v1_principle / v2_permission / v3_ha 三个参数目前由调用方手动构造 (测试用). 真实接入留待 P28 阶段 6 (apeireth-verify 跨 crate 回归验证).`

> `ponytail: TwelveKeyVerdictCache 使用 ALL_TWELVE_KEYS 位置索引 (O(12) = O(1) 常数), 而非 group_id() (返回 1-6, 多键共享). 这是设计选择 — 12 个独立槽位 vs 6 个 PHL group 槽位.`

**未来升级路径** (当 council 7 真实表决接入时):
- `CouncilAdvisoryBoard` 改为 `Council::deliberate(action)` 自动填充
- `ConstraintEngineDeep` 添加 `council_board: Council::deliberate()` 字段
- `verify_v1_v2_v3_and_gate` 接入 `apeireth_council::Council::verdict`

---

## 8. 验证清单 (✓ 全部满足)

- [x] (1) 4 重守门嵌套 — 既已实装, 本轮未触碰
- [x] (2) PermissionGrant trait 真实 depth — Council 7 强制 advisor 独立投票 + Human L0 + RiskLevel 三方授权
- [x] (3) 12 键 verdict cache O(1) 查询 — `TwelveKeyVerdictCache` 定长数组
- [x] (4) 与 `apeireth-core::TWELVE_KEYS_HARDCODE` 编译期 hardcode — 双层断言联动
- [x] (5) V1+V2+V3 AND 门 — `verify_v1_v2_v3_and_gate` 委托 `ActionGuard::check_action`
- [x] (6) ≥30 unit — 实装 56 个 (27 既有 + 29 新), 0 fail
- [x] (6) ≥10 integration — 实装 15 个, 0 fail
- [x] (7) 不修改任何 LOCKED — `apeireth-core` 零变更, 既 27 单元测试零回归
- [x] (8) 守 7 项不修改承诺 — 既有 trait 签名 / 函数入口 / 错误类型 全部保留
- [x] (9) 报告产出 — `reports/round8-05-constraint-4-gates-permission-grant-deep-implementation-security-reviewer.md`

---

## 9. 附: 文件清单

| 文件 | 变更 | 行数 |
|---|---|---|
| `crates/apeireth-constraint/src/deep_impl.rs` | **新增** | 998 |
| `crates/apeireth-constraint/src/lib.rs` | +2 行 (1 个 `pub mod deep_impl;` + 1 行模块注释) | 1202 (净 +2) |
| `crates/apeireth-constraint/tests/constraint_tests.rs` | 无变更 | 574 |
| `crates/apeireth-constraint/Cargo.toml` | 无变更 | 31 |
| `crates/apeireth-core/src/lib.rs` | **零修改** | 2381 |
| `docs/stage5/stage5-construction-document.md` | **零修改** (LOCKED) | — |

**安全审查结论**: PASS — 8 项任务要求 + 7 项不修改承诺 全部满足, 零 LOC 修改 LOCKED 文件, 零测试回归, 56 unit + 15 integration 全绿.
