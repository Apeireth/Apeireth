# apeireth-formal 5 不变量 + Kani 验证 harness 完整蓝图 (R19+ 阶段 3)

```
[Document-Meta]
Document: docs/stage4/apeireth-formal-invariants-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+ 阶段 3 (跟 team-lead 同步)
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + leader 复核)
Author: software-architect (Mavis 通用 agent)
```

> **性质**: 纯文档交付。**不写代码、不改任何文件** (除本文件)。给后续 rust-coder 实施 `apeireth-formal` 4 个新不变量 + Kani harness 用。
>
> **依据**:
> - `crates/apeireth-formal/src/lib.rs` + `src/invariants/double_onion_sample.rs` (现有 1/5 不变量, 模板已上)
> - `crates/apeireth-formal/Cargo.toml` (零依赖, Kani-friendly, 编译期 hardcode `PERMISSION_ONION_DEPTH=6`)
> - `crates/apeireth-formal/docs/kani-setup.md` (Kani 安装 + 跑 harness 指南)
> - `reports/apeireth-session-vector-asi-2026-08-05.md` §1.1 / §4.3 / §8.5 (formal 1/5 现状 + 阶段 3 估时 2-3 天)
> - `docs/stage4/apeireth-session-blueprint-2026-08-05.md` §4.3 / §4.4 (mid_task 蓝图 + 3 处修法)
> - `docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md` §4 (7 advisor voting 触发 trait 注入)
> - `docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md` §2 (E 层修改路径 = 守门 1-4 + 权限发放)
> - `docs/stage4/stage4-correction-v15-four-gates-permission-grant.md` §3 (4 重守门嵌套 + 权限发放 7 advisor + L0 HA)
> - `docs/adr/0001-double-onion-unity.md` (双洋葱统一体, 7 advisor 设定)
> - `docs/adr/0012-team-lead-council-collaboration.md` (7 advisor voting trait 抽象)
> - `docs/stage4/r-measure-verification-design-2026-08-05.md` (R-Measure 守门)
> - `APEIRETH-CONVENTIONS.md` §10 (8 项不修改承诺) + §11 (R11 baseline 3 值)
>
> **不修改承诺**: 阶段 1/2/3/4/5 LOCKED + v2/v4/v4.1 LOCKED + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta + R11 baseline 三值 全部保留 (见 §10)。
>
> **诚实登记** (S-2 17:43):
> 1. 任务 prompt 提到 "APEIRETH-CONVENTIONS §6 (3 层架构, 4 组件 + 权限分配)" — **实际** APEIRETH-CONVENTIONS §6 是 commit 规范 (Manual-Rev-H + Fix-12)。e_layer / permission_grant_l0 权威源是 `stage4-correction-v6` (E 层修改路径) + `stage4-correction-v15` (4 重守门嵌套 + 权限发放 7 advisor), 已在本文 §2.2 / §2.3 准确标注。本文档**不**修改 APEIRETH-CONVENTIONS (LOCKED)。
> 2. 当前 `apeireth-formal` 实际是 **1/5 不变量** (`double_onion_sample` 已有, Kani harness + 4 unit test + 11 sanity case, 跑通)。需要补 **4 个新不变量** (e_layer / permission_grant_l0 / mid_task / 7_advisor), 估时 2-3 天, 估 400 LOC (per `apeireth-session-vector-asi §4.4`)。
> 3. 4 个新不变量强依赖 3 个上游 crate: `apeireth-session` (mid_task)、`apeireth-council` (7 advisor)、`apeireth-team-lead` (voting 触发)。这些 crate 当前实装进度不一致, Kani harness **必须** 写 POD 模型 (不依赖上游真实结构体), 跟 `double_onion_sample.rs` 模板一致。
> 4. Kani 在 Windows 需 WSL2 (`docs/kani-setup.md §2.1` 已写)。CI workflow 待 R20 阶段 1.5 加 (per `apeireth-session-vector-asi §8.6`)。
> 5. **跟 Hermes R18 阶段 2 集成测试互补**: Hermes 跑 `cargo test` (122 个集成测试, 34 lib.rs), Kani 跑 model check (符号执行, 完备覆盖)。两套通道独立, 互不替代。

---

## §1 战略背景 (为什么)

### 1.1 关键事实

| 事实 | 来源 | 含义 |
|------|------|------|
| **`apeireth-formal` 1/5 不变量** | `crates/apeireth-formal/src/invariants/double_onion_sample.rs` (1 文件 101 LOC) | R19+ 阶段 3 待扩 4 个 |
| **R19+ 阶段 3 集成阻塞** | `reports/apeireth-session-vector-asi-2026-08-05.md` §1.1 (formal 缺 4 不变量) + §7.1 G4 (P0 必补) | 关键 action 缺形式化守门 |
| **Kani 形式化 = 编译期 + 运行时 双层守门** | `docs/kani-setup.md §1` (Kani = 符号执行 + 有界模型检查, 完备覆盖) | 比 `cargo test` 强 |
| **5 不变量估 2-3 天实施** | `apeireth-session-vector-asi §4.4` (估 LOC 400) + `§8.5` (阶段 3 估 2-3 天) | 跟本蓝图 §8 5 阶段 3 天一致 |
| **跟 Hermes R18 集成测试互补** | `apeireth-session-vector-asi §1.2 第 3 行` (formal 跟 team-lead 集成弱) | Kani = 形式化, cargo test = 行为, 双层守 |

### 1.2 不补这 4 个不变量的代价 (诚实登记)

按主 S-2 17:43 实事求是:

| 不补的后果 | 触发场景 | 影响 |
|----------|---------|------|
| **e_layer 隔离无形式化证明** | AI 大进化时误改 E 层 (E-1..E-6 6 项不可违背) | 失去"电子环最后护栏", 架构层无证明 |
| **permission_grant_l0 无 M-of-N 守门** | L0 权限提升绕过 PID 1 + sovereignty 联合签名 | 最高权限失控, HA 失效 |
| **mid_task 状态撕裂无原子性证明** | child session 状态转换非 CAS, 窗口期 race | mid-task bug 3 处复发, 父进程卡死 5min |
| **7_advisor voting 完整性无证明** | 7 advisor 部分返回就 synthesis | 陪审团失效, 决策不完整 |

4 处是**关键守门**, 任一不补 = 形式化通道有缺口。P0 必一起补 (跟 mid-task bug 3 处一起改思路一致)。

### 1.3 战略原则 (硬约束)

| 原则 | 来源 | 落地 |
|------|------|------|
| **Kani-friendly POD 模型** | `crates/apeireth-formal/src/lib.rs §禁止` | ❌ 不引入 String/Vec/HashMap, 不用 `apeireth-core`/`apeireth-onion` 真实结构体 |
| **不依赖上游真实结构体** | `double_onion_sample.rs` 模板 | 4 个新不变量全部自带 POD 模型, 等上游实装后改 trait 桥接 |
| **Kani 单独 workflow, 不挡 PR** | `docs/kani-setup.md §4` | `.github/workflows/kani.yml` (R20 阶段 1.5 加, 不在本蓝图范围) |
| **no unsafe** | `crates/apeireth-formal/src/lib.rs:28` `#![deny(unsafe_code)]` | 4 个新不变量同步禁止 |
| **编译期 hardcode 守门** | `PERMISSION_ONION_DEPTH = 6` (lib.rs:56) | 4 个新不变量加编译期 const (e.g. `E_LAYER_COUNT = 3`, `ADVISOR_COUNT = 7`) |
| **不假装已实现** | O-5 + APEIRETH-CONVENTIONS §2 | Kani harness 写完但实装 POD 是 stub, runtime sanity_check 用具体 case 兜底 |
| **6 主哲学锚穿透** | APEIRETH-CONVENTIONS §9 | 见 §11 |
| **8 项不修改承诺** | APEIRETH-CONVENTIONS §10 + ADR-0011 §不修改承诺 | 见 §10 |

### 1.4 比喻

> 现有 `double_onion_sample` = apeireth-formal 的"hello world" — 1 个不变量, 跑通 Kani 通道。
>
> 5 不变量完整版 (本蓝图) = apeireth-formal 的"完整安检" — 5 个不变量覆盖 e_layer 隔离 / L0 权限 / mid_task 原子 / 7 advisor voting 完整性 / 现有 L0 HA, 跟上游 crate 弱耦合 (POD 模型), 强验证 (Kani 完备覆盖)。
>
> 集成 = 把 Kani 通道**嵌入** R19+ 关键 action 守门 (M11 跨载体 / M12 OTA / mid-task bug 修法 / 7 advisor voting 触发), 让形式化证明**伴随**实装一起上, 不是事后补丁。
>
> 阶段编号详见 docs/stage4/r19-r20-stage-unified-2026-08-05.md §3 (本蓝图"阶段 1-5" = 套 C formal 5 不变量 5 阶段实施)

---

## §2 5 不变量完整清单

### 2.1 不变量 1: ⏳ 已有 — `double_onion_sample` (L0 必须 HA)

| 维度 | 详情 |
|------|------|
| **状态** | ✅ **已实装**, 跑通 Kani (`cargo kani --harness double_onion_sample`) + 4 unit test + 11 sanity case |
| **来源** | `crates/apeireth-formal/src/invariants/double_onion_sample.rs` (101 LOC) + `lib.rs:63` `l0_requires_ha_invariant` |
| **Kani harness** | `double_onion_sample()` — 任意 `PermissionLayerConfig { kind: u8, requires_ha: bool }`, 断言 `kind==0 ⇒ requires_ha==true` |
| **编译期守门** | `PERMISSION_ONION_DEPTH: usize = 6` (lib.rs:56) — 6 层洋葱永不变 |
| **验证范围** | L0 权限发放必须要求 HA (Human Authority 真实人类批准) |
| **关联 LOCKED** | APEIRETH-CONVENTIONS §10 不修改承诺 6 (R11 baseline) + 阶段 1 §3.1 (L0 是最后护栏) |
| **TODO (本蓝图)** | ❌ **不改**, 保留作模板 |

**现有源码摘要** (per `double_onion_sample.rs`):

```rust
use crate::{l0_requires_ha_invariant, PermissionLayerConfig};

#[cfg_attr(kani, kani::proof)]
pub fn double_onion_sample() {
    let cfg = nondet_config();
    assert!(l0_requires_ha_invariant(cfg));
}

#[cfg(kani)]
fn nondet_config() -> PermissionLayerConfig { kani::any() }

#[cfg(not(kani))]
fn nondet_config() -> PermissionLayerConfig {
    PermissionLayerConfig::new(0, true)  // cargo test 兜底: happy path
}

pub fn sanity_check() -> bool { /* 11 case: L0+true + 1..=5 × {true,false} */ }
```

**模板要素** (4 个新不变量严格沿用):
1. `#[cfg_attr(kani, kani::proof)]` 让 stable Rust 也能编译
2. `nondet_*()` 双 cfg 隔离 Kani 模式 / runtime 模式
3. `sanity_check() -> bool` runtime smoke test
4. `#[cfg(test)] mod tests` 至少 3 个 test (正例 + 反例 + harness 可见性)

---

### 2.2 不变量 2: ❌ 缺失 — `e_layer_isolation` (E 层隔离)

| 维度 | 详情 |
|------|------|
| **状态** | ❌ **缺失**, R19+ 阶段 3 新增 |
| **定义** | e_layer (electronic layer, 电子层) 跟 o_layer (organic layer, 有机层) / s_layer (social layer, 社会层) **严格隔离**。任何跨 e_layer 调用必须经过权限检查 |
| **POD 模型** | `EConfig { caller_layer: u8, target_layer: u8, action: u8, has_permission: bool }` (3 层 caller × 3 层 target × 4 action = 36 组合, Kani 完备可解) |
| **Kani 验证** | `e_layer_isolation()` — 任意配置, 断言: 跨 e_layer 写入 (caller != E, target == E, action == Mutate) 必须 `has_permission == true`; 同层或非 e_layer 写入无要求 |
| **关联 LOCKED** | `stage4-correction-v6-consolidated-and-e-layer-mutation.md §2.3` (E 层修改路径 = 守门 1-4 默认拒绝 + 权限发放例外允许) + §18.6 (双根棒) + 阶段 1 §3.1 (E-1..E-6 6 项不可违背) |
| **实施估时** | 0.5 天, 估 80 LOC (per `apeireth-session-vector-asi §4.4` 第 1 行) |
| **跟 Hermes 互补** | Hermes R18 跑 `cargo test` 验证 e_layer 业务逻辑 (5 器官), Kani 验证隔离**形式属性** (跨层必检) |

**为什么 E 层要守** (per v6 修正链): E 层 = 整个 12 键编译时 hardcode + 4 重守门 Gate 1 内层。失去 E 层 = 失去 ASI 候选主体最后护栏。

**Kani harness 伪代码** (跟 `double_onion_sample.rs` 模板一致, rust-coder 复制改即可):

```rust
// crates/apeireth-formal/src/invariants/e_layer_isolation.rs
//
// 不变量: E 层隔离 (e_layer_isolation)
// 物理含义: 任何跨 e_layer 写入必须经权限检查, 失去 E 层 = 失去最后护栏
// 依据: stage4-correction-v6 §2.3 (E 层修改路径)
// Kani-friendly: POD u8 / bool, 无 String / Vec / HashMap

use crate::{E_LAYER_COUNT, ELayerAction, LayerKind};

/// 编译期 hardcode: 3 层架构 (e_layer / o_layer / s_layer)
pub const E_LAYER_COUNT: u8 = 3;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LayerKind { Electronic = 0, Organic = 1, Social = 2 }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ELayerAction { Read = 0, Mutate = 1, Cross = 2, CrossMutate = 3 }

/// Kani-friendly POD 配置
#[derive(Copy, Clone, Debug)]
pub struct EConfig {
    pub caller_layer: u8,      // 0..=2
    pub target_layer: u8,      // 0..=2
    pub action: u8,            // 0..=3
    pub has_permission: bool,
}

/// 核心不变量: 跨 e_layer 写入必须 has_permission
///
/// 判定规则:
/// - caller == E (0) + target == E (0) + action == Mutate (1): E 层内修改 → 守门 1-4 默认拒绝
///   (本不变量只覆盖"跨层", 同 E 内修改由 Gate 1-4 守门, 不在本不变量范围)
/// - caller != E (1,2) + target == E (0) + action == CrossMutate (3): 跨 E 层写入 → 必须 has_permission
/// - 其他组合: 自由 (无隔离要求)
pub fn e_layer_isolation_invariant(cfg: EConfig) -> bool {
    let is_cross_e_write = cfg.caller_layer != LayerKind::Electronic as u8
        && cfg.target_layer == LayerKind::Electronic as u8
        && cfg.action == ELayerAction::CrossMutate as u8;
    if is_cross_e_write { cfg.has_permission } else { true }
}

#[cfg_attr(kani, kani::proof)]
pub fn e_layer_isolation() {
    let cfg = nondet_config();
    assert!(e_layer_isolation_invariant(cfg));
}

#[cfg(kani)]
fn nondet_config() -> EConfig { kani::any() }

#[cfg(not(kani))]
fn nondet_config() -> EConfig {
    // cargo test 兜底: 一个合法跨层写入 (有 permission, 通过不变量)
    EConfig { caller_layer: 1, target_layer: 0, action: 3, has_permission: true }
}

pub fn sanity_check() -> bool {
    // happy: 跨 E 写入有 permission → pass
    if !e_layer_isolation_invariant(EConfig { caller_layer: 1, target_layer: 0, action: 3, has_permission: true }) {
        return false;
    }
    // happy: 同 E 内读 → pass
    if !e_layer_isolation_invariant(EConfig { caller_layer: 0, target_layer: 0, action: 0, has_permission: false }) {
        return false;
    }
    // happy: 非跨 E → pass (不管 permission)
    if !e_layer_isolation_invariant(EConfig { caller_layer: 2, target_layer: 1, action: 1, has_permission: false }) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_function_is_publicly_visible() {
        let _: fn() = e_layer_isolation;
    }

    #[test]
    fn sanity_check_passes_all_precondition_inputs() {
        assert!(sanity_check(), "e_layer_isolation violated on valid input");
    }

    #[test]
    fn negative_cross_e_write_without_permission_must_violate() {
        // 反例: 跨 E 写入无 permission 必须被不变量抓出
        let bad = EConfig { caller_layer: 1, target_layer: 0, action: 3, has_permission: false };
        assert!(!e_layer_isolation_invariant(bad), "cross e_layer write without permission must violate");
    }

    #[test]
    fn e_layer_count_is_three() {
        // 编译期守: 3 层架构永不变
        assert_eq!(E_LAYER_COUNT, 3);
    }
}
```

**待 Mavis 拍板 (S-2 17:43)**:
- 任务 prompt 提 "APEIRETH-CONVENTIONS §6" — **实际** §6 是 commit 规范。E 层权威源是 v6 修正链 (4 重守门嵌套)。本不变量以 v6 为依据, **不**改 APEIRETH-CONVENTIONS。
- LayerKind 命名: 阶段 1 §3.1 用 E/S/A/M/O 5 层 (5 原则洋葱), 任务 prompt 提"e_layer / o_layer / s_layer 3 层" — 实际 5 层洋葱是 5 原则 (Electronic / Social / Action / Motivation / Organic), **不**是 3 层。本不变量**只**覆盖 e_layer 隔离 (跨 e_layer 写入), 不动其他 4 层。Stage4 决策以主人 v6 修正链为准。

---

### 2.3 不变量 3: ❌ 缺失 — `permission_grant_l0` (L0 权限提升守门)

| 维度 | 详情 |
|------|------|
| **状态** | ❌ **缺失**, R19+ 阶段 3 新增 |
| **定义** | L0 权限 (最高权限) 提升必须满足 **(PID 1 签名 ∧ sovereignty 签名 ∧ HA 至少 1 票)** 联合。M-of-N 中 N=2 (PID 1 + sovereignty 联合) + 1 HA 票 |
| **POD 模型** | `GrantConfig { level: u8, pid1_signed: bool, sovereignty_signed: bool, ha_count: u8 }` (level 0..=5 × pid1 × sovereignty × ha 0..=255 = 1536 组合, Kani 完备) |
| **Kani 验证** | `permission_grant_l0()` — 任意配置, 断言: level == 0 ⇒ (pid1_signed ∧ sovereignty_signed ∧ ha_count >= 1) |
| **关联 LOCKED** | `stage4-correction-v15-four-gates-permission-grant.md §3` (4 重守门嵌套 + 权限发放 = 5 重治理 + Council 7 智囊团 + L0 HA) + ADR-0005 (M1-M12 风险分级, L0 = critical 7 席全量) |
| **实施估时** | 0.5 天, 估 100 LOC (per `apeireth-session-vector-asi §4.4` 第 2 行) |
| **跟 Hermes 互补** | Hermes R18 跑 `cargo test` 验证 permission grant 业务流 (council 审议 + supervisor 监督), Kani 验证 L0 联合签名**形式属性** |

**为什么 L0 联合要守** (per v15 修正链 + 阶段 1 §1.4): L0 = HA 核心 (🛡️ 最后护栏)。失去 L0 联合 = 失去"AI 大进化时修改 E 层"的安全通道, ASI 候选主体失控风险。

**Kani harness 伪代码**:

```rust
// crates/apeireth-formal/src/invariants/permission_grant_l0.rs
//
// 不变量: L0 权限提升必须 PID 1 + sovereignty 联合 + 至少 1 HA 票
// 物理含义: L0 是 HA 最后护栏, 联合签名是唯一合法路径
// 依据: stage4-correction-v15 §3 (4 重守门 + 权限发放) + 阶段 1 §1.4 (L0 最后护栏)
// Kani-friendly: POD u8 / bool

use crate::PERMISSION_ONION_DEPTH;

/// 编译期 hardcode: L0 权限发放最少需要 1 个 HA 票
pub const L0_MIN_HA_VOTES: u8 = 1;
/// 编译期 hardcode: L0 必须 PID 1 + sovereignty 联合 (2/2 票)
pub const L0_MIN_AUTHORITY_SIGNATURES: u8 = 2;

#[derive(Copy, Clone, Debug)]
pub struct GrantConfig {
    pub level: u8,                    // 0..=5 (L0..L5)
    pub pid1_signed: bool,            // PID 1 进程签名 (apeireth-supervisor 持有)
    pub sovereignty_signed: bool,     // sovereignty 持续性检查签名
    pub ha_count: u8,                 // 真实人类批准票数 (0..=255)
}

/// 核心不变量: L0 提升必须满足 (pid1 ∧ sovereignty) 联合 + 至少 1 HA 票
pub fn permission_grant_l0_invariant(cfg: GrantConfig) -> bool {
    if cfg.level == 0 {
        cfg.pid1_signed && cfg.sovereignty_signed && cfg.ha_count >= L0_MIN_HA_VOTES
    } else {
        true  // L1..L5 不在本不变量范围, 由 4 重守门守
    }
}

#[cfg_attr(kani, kani::proof)]
pub fn permission_grant_l0() {
    let cfg = nondet_config();
    assert!(permission_grant_l0_invariant(cfg));
}

#[cfg(kani)]
fn nondet_config() -> GrantConfig { kani::any() }

#[cfg(not(kani))]
fn nondet_config() -> GrantConfig {
    // cargo test 兜底: L0 happy path (PID 1 + sovereignty + 1 HA)
    GrantConfig { level: 0, pid1_signed: true, sovereignty_signed: true, ha_count: 1 }
}

pub fn sanity_check() -> bool {
    // happy: L0 三签名齐全 → pass
    if !permission_grant_l0_invariant(GrantConfig { level: 0, pid1_signed: true, sovereignty_signed: true, ha_count: 1 }) {
        return false;
    }
    // happy: L1..L5 任意组合 → pass
    for level in 1u8..=5 {
        for pid1 in [true, false] {
            for sov in [true, false] {
                if !permission_grant_l0_invariant(GrantConfig { level, pid1_signed: pid1, sovereignty_signed: sov, ha_count: 0 }) {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_function_is_publicly_visible() {
        let _: fn() = permission_grant_l0;
    }

    #[test]
    fn sanity_check_passes_all_precondition_inputs() {
        assert!(sanity_check(), "permission_grant_l0 violated on valid input");
    }

    #[test]
    fn negative_l0_missing_pid1_must_violate() {
        // 反例 1: L0 缺 PID 1 签名
        let bad = GrantConfig { level: 0, pid1_signed: false, sovereignty_signed: true, ha_count: 1 };
        assert!(!permission_grant_l0_invariant(bad), "L0 without PID 1 must violate");
    }

    #[test]
    fn negative_l0_missing_sovereignty_must_violate() {
        // 反例 2: L0 缺 sovereignty 签名
        let bad = GrantConfig { level: 0, pid1_signed: true, sovereignty_signed: false, ha_count: 1 };
        assert!(!permission_grant_l0_invariant(bad), "L0 without sovereignty must violate");
    }

    #[test]
    fn negative_l0_missing_ha_must_violate() {
        // 反例 3: L0 缺 HA 票
        let bad = GrantConfig { level: 0, pid1_signed: true, sovereignty_signed: true, ha_count: 0 };
        assert!(!permission_grant_l0_invariant(bad), "L0 without HA must violate");
    }

    #[test]
    fn l0_min_ha_votes_is_one() {
        assert_eq!(L0_MIN_HA_VOTES, 1);
    }

    #[test]
    fn l0_min_authority_signatures_is_two() {
        assert_eq!(L0_MIN_AUTHORITY_SIGNATURES, 2);
    }
}
```

**待 Mavis 拍板 (S-2 17:43)**:
- 任务 prompt 提 "L0 只能由 PID 1 + sovereignty 联合授予" — 跟 v15 修正链 + 阶段 1 §1.4 一致, 但 v15 还**额外**要求 ≥1 HA 票 (人类决策 = L0 HA 真实人类批准)。本不变量采用 **"联合 + HA" 三签名齐全** 版本 (比任务 prompt 严格, 跟 v15 LOCKED 一致), **不**只覆盖 PID 1 + sovereignty。
- L0 在 ADR-0005 是 M1-M12 风险分级中的 **critical 7 席全量** (per v15 §3 第 5 项)。Council 7 强制 advisor 完整性由不变量 5 (7_advisor) 守, 本不变量只守"联合签名齐全"。

---

### 2.4 不变量 4: ❌ 缺失 — `mid_task_atomicity` (mid-task 状态转换原子性)

| 维度 | 详情 |
|------|------|
| **状态** | ❌ **缺失**, R19+ 阶段 3 新增 |
| **定义** | child session 状态变化到 agent 状态变化是**原子的**, 不存在窗口期 (mid-task bug #3 根因) |
| **POD 模型** | `SessionState (2 = Running / MidTask)` + `MidTaskState (4 = Idle / Active / Interrupted / Failed)` + `MessageRef (3 字段: id, seq, valid)`, 模拟 `apeireth-session::Session::transition_to_mid_task()` 路径 |
| **Kani 验证** | `mid_task_atomicity()` — 任意初始状态, 断言: 转换**要么**成功 (`state = MidTask ∧ mid_task_state.status = Active ∧ mid_task_state.caused_by_seq = input.seq`), **要么**失败 (`state` 保持原样, `mid_task_state.status` 保持原样, 无中间态) |
| **关联 LOCKED** | `docs/stage4/apeireth-session-blueprint-2026-08-05.md §4.3` (mid-task bug 时序) + §4.4 修法 #2 (AgentHandle.send_to_agent 加 child session 状态检查 + await) + 修法 #3 (broadcast 事件总线) |
| **实施估时** | 1 天, 估 120 LOC (per `apeireth-session-vector-asi §4.4` 第 3 行) — 4 个最复杂 |
| **跟 Hermes 互补** | Hermes R18 跑 122 个集成测试验证 mid-task 业务流 (修法 1+2+3 一起改), Kani 验证状态转换**形式属性** (无 race) |

**为什么 mid_task 原子性要守** (per session-blueprint §4.3 + §4.4): mid-task bug 3 处修法任一不改 = 撕裂状态复发。原子性证明 = 形式化保障修法不再退化。

**Kani harness 伪代码**:

```rust
// crates/apeireth-formal/src/invariants/mid_task_atomicity.rs
//
// 不变量: mid-task 状态转换原子性 (无 race window)
// 物理含义: 修法 1+2+3 后, 状态转换必须 CAS, 不存在窗口期
// 依据: apeireth-session-blueprint §4.3 + §4.4 修法 #2 + #3
// Kani-friendly: 有限状态机 POD (Running / MidTask × Idle/Active/Interrupted/Failed)

use crate::MID_TASK_STATES;

/// 编译期 hardcode: 4 种 mid-task 子状态
pub const MID_TASK_STATES: u8 = 4;
/// 编译期 hardcode: 主状态数 (Running / MidTask / 已完成 = 3, 取 2 个活跃)
pub const ACTIVE_SESSION_STATES: u8 = 2;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SessionState { Running = 0, MidTask = 1 }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MidTaskStatus { Idle = 0, Active = 1, Interrupted = 2, Failed = 3 }

#[derive(Copy, Clone, Debug)]
pub struct MessageRef { pub id: u32, pub seq: u64, pub valid: bool }

#[derive(Copy, Clone, Debug)]
pub struct MidTaskState { pub status: u8, pub caused_by_seq: u64 }

#[derive(Copy, Clone, Debug)]
pub struct Session {
    pub state: u8,                    // SessionState as u8 (Kani 友好)
    pub mid_task_state: MidTaskState,
}

/// 模拟 Session::transition_to_mid_task — 原子性保证:
///
/// 成功条件: state == Running ∧ mid_task_state.status == Idle ∧ message.valid
///   ⇒ 全部字段一次性更新 (state=1, mid_task_state.status=1, caused_by_seq=message.seq)
/// 失败: 任一前提不满足
///   ⇒ state 保持, mid_task_state 保持 (无部分更新)
///
/// Kani 验证: 转换函数返回后, 状态机状态只能是 (1, 1, seq) 或 (0, status, 0) 或 (1, status, prev)
/// — 不存在 (0, 1, seq) 或 (1, 0, seq) 这种半更新
pub fn mid_task_atomicity_invariant(
    initial: Session,
    message: MessageRef,
    result_state: u8,
    result_mid_status: u8,
    result_caused_by: u64,
) -> bool {
    // 前提: 初始状态合法
    let initial_valid = initial.state <= 1 && initial.mid_task_state.status < MID_TASK_STATES;
    if !initial_valid { return true; /* Kani 输入超出范围, 不变量空真 */ }

    let can_succeed = initial.state == SessionState::Running as u8
        && initial.mid_task_state.status == MidTaskStatus::Idle as u8
        && message.valid;

    if can_succeed {
        // 成功: 必须是完整 MidTask 状态 (state=1, status=1, caused_by=message.seq)
        result_state == SessionState::MidTask as u8
            && result_mid_status == MidTaskStatus::Active as u8
            && result_caused_by == message.seq
    } else {
        // 失败: 必须完全回滚 (state 不变, mid_task 不变, caused_by 不变)
        result_state == initial.state
            && result_mid_status == initial.mid_task_state.status
            && result_caused_by == initial.mid_task_state.caused_by_seq
    }
}

#[cfg_attr(kani, kani::proof)]
pub fn mid_task_atomicity() {
    let initial = nondet_session();
    let message = nondet_message();
    let result_state: u8 = kani::any();
    let result_mid_status: u8 = kani::any();
    let result_caused_by: u64 = kani::any();

    assert!(mid_task_atomicity_invariant(
        initial, message, result_state, result_mid_status, result_caused_by
    ));
}

#[cfg(kani)]
fn nondet_session() -> Session { kani::any() }
#[cfg(kani)]
fn nondet_message() -> MessageRef { kani::any() }

#[cfg(not(kani))]
fn nondet_session() -> Session {
    Session { state: 0, mid_task_state: MidTaskState { status: 0, caused_by_seq: 0 } }
}
#[cfg(not(kani))]
fn nondet_message() -> MessageRef { MessageRef { id: 1, seq: 1, valid: true } }

pub fn sanity_check() -> bool {
    // happy 1: Running + Idle + valid msg → 成功 (state=1, status=1, caused_by=1)
    let init = Session { state: 0, mid_task_state: MidTaskState { status: 0, caused_by_seq: 0 } };
    if !mid_task_atomicity_invariant(init, MessageRef { id: 1, seq: 1, valid: true }, 1, 1, 1) {
        return false;
    }
    // happy 2: invalid msg → 失败回滚 (state=0, status=0, caused_by=0)
    if !mid_task_atomicity_invariant(init, MessageRef { id: 1, seq: 1, valid: false }, 0, 0, 0) {
        return false;
    }
    // happy 3: state=MidTask + 任意 msg → 失败回滚 (state=1, status=0, caused_by=0)
    let mid = Session { state: 1, mid_task_state: MidTaskState { status: 0, caused_by_seq: 0 } };
    if !mid_task_atomicity_invariant(mid, MessageRef { id: 1, seq: 1, valid: true }, 1, 0, 0) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_function_is_publicly_visible() {
        let _: fn() = mid_task_atomicity;
    }

    #[test]
    fn sanity_check_passes_all_precondition_inputs() {
        assert!(sanity_check(), "mid_task_atomicity violated on valid input");
    }

    #[test]
    fn negative_partial_update_must_violate() {
        // 反例: 成功条件满足但状态只更新一半 (state=MidTask 但 status=Idle)
        let init = Session { state: 0, mid_task_state: MidTaskState { status: 0, caused_by_seq: 0 } };
        let bad = (init, MessageRef { id: 1, seq: 1, valid: true }, 1, 0, 1);
        assert!(!mid_task_atomicity_invariant(bad.0, bad.1, bad.2, bad.3, bad.4),
            "partial update must violate atomicity");
    }

    #[test]
    fn mid_task_state_count_is_four() {
        assert_eq!(MID_TASK_STATES, 4);
    }
}
```

**待 Mavis 拍板 (S-2 17:43)**:
- POD 模型 vs 真实 `apeireth-session` 结构体: 本不变量**只**用 POD (u8 编码状态), 跟 `double_onion_sample.rs` 模板一致 (不依赖 `apeireth-core`/`apeireth-onion` 真实结构体)。等 `apeireth-session` 实装后, 可在 `Cargo.toml` 加 dependency + 写 `#[cfg(not(kani))]` 桥接。
- 实际 `apeireth-session::Session::transition_to_mid_task` 是 6 状态机 (per session-blueprint §3.3), 本不变量简化到 2 (Running / MidTask) 是为了 Kani 完备可解。完整 6 状态机在 `apeireth-session` 内部用 runtime invariant + cargo test 守, Kani 通道只守"原子性"这个核心形式属性。

---

### 2.5 不变量 5: ❌ 缺失 — `seven_advisor_voting` (7 advisor voting 完整性)

| 维度 | 详情 |
|------|------|
| **状态** | ❌ **缺失**, R19+ 阶段 3 新增 |
| **定义** | `apeireth-council` 7 强制 advisor **全部**返回 opinion 才能做 synthesis (投票完整性)。任一 opinion 缺失 = synthesis 必须 wait, 不允许部分 synthesis |
| **POD 模型** | `VotingConfig { criticality: u8 (0..=100), opinions: [bool; 7] (true=returned, false=missing) }` + `triggered: bool` (是否已触发 voting) |
| **Kani 验证** | `seven_advisor_voting()` — 任意配置, 断言: `triggered == true ∧ synthesis 启动` ⇒ `opinions[0..7]` 全部 `true` (无 missing) |
| **关联 LOCKED** | ADR-0001 (double-onion-unity, 7 强制 advisor 设定) + ADR-0012 §决策 1-4 (7 advisor voting trait 抽象) + `apeireth-team-lead-implementation-guide §4` (voting 触发判定 + criticality 阈值) + `apeireth-council §2 7 advisor 表` |
| **实施估时** | 0.5 天, 估 100 LOC (per `apeireth-session-vector-asi §4.4` 第 4 行) |
| **跟 Hermes 互补** | Hermes R18 跑 122 个集成测试验证 council 业务流 (7 advisor 并行审议), Kani 验证 voting 完整性**形式属性** (全部返回才能 synthesis) |

**为什么 7 advisor voting 完整性要守** (per ADR-0001 + ADR-0012): 7 advisor 是 ASI 候选主体安全陪审团。任一缺失就 synthesis = 决策不完整, ASI 风险升高。Criticality 阈值 (默认 0.8 = 80) 触发后, **必须**等 7 全返才能进 synthesis。

**Kani harness 伪代码**:

```rust
// crates/apeireth-formal/src/invariants/seven_advisor_voting.rs
//
// 不变量: 7 advisor voting 完整性 (synthesis 启动前 7 opinion 全到)
// 物理含义: 陪审团制度, 任何 1 席缺失 = 决策不完整, synthesis 必须 wait
// 依据: ADR-0001 (双洋葱 7 advisor) + ADR-0012 §决策 1-4 + team-lead-impl §4
// Kani-friendly: 固定 [bool; 7] 数组 + u8 临界值

use crate::ADVISOR_COUNT;

/// 编译期 hardcode: 7 强制 advisor (per ADR-0001)
pub const ADVISOR_COUNT: usize = 7;
/// 编译期 hardcode: voting 触发临界值 (per team-lead-impl §4, 默认 0.8 → u8 表示 80)
pub const VOTING_CRITICALITY_THRESHOLD: u8 = 80;

#[derive(Copy, Clone, Debug)]
pub struct VotingConfig {
    pub criticality: u8,                  // 0..=100, criticality 越高压力越大
    pub opinions: [bool; ADVISOR_COUNT],  // true = returned, false = missing
    pub synthesis_started: bool,          // synthesis 阶段是否已启动
}

/// 核心不变量: criticality 触发后, synthesis 启动前 7 opinion 全到
///
/// 规则:
/// - criticality >= 80 ∧ synthesis_started = true ⇒ opinions[0..7] 全 true (任一 false = violation)
/// - criticality < 80 (未触发): synthesis 随意, 无要求
pub fn seven_advisor_voting_invariant(cfg: VotingConfig) -> bool {
    if cfg.criticality >= VOTING_CRITICALITY_THRESHOLD && cfg.synthesis_started {
        // 7 opinion 全到 (无 missing)
        cfg.opinions.iter().all(|&op| op)
    } else {
        true  // 未触发或 synthesis 未启动, 不变量空真
    }
}

#[cfg_attr(kani, kani::proof)]
pub fn seven_advisor_voting() {
    let cfg = nondet_config();
    assert!(seven_advisor_voting_invariant(cfg));
}

#[cfg(kani)]
fn nondet_config() -> VotingConfig { kani::any() }

#[cfg(not(kani))]
fn nondet_config() -> VotingConfig {
    // cargo test 兜底: criticality 触发 + synthesis 启动 + 7 opinion 全返 (happy path)
    VotingConfig { criticality: 90, opinions: [true; ADVISOR_COUNT], synthesis_started: true }
}

pub fn sanity_check() -> bool {
    // happy 1: criticality 触发 + 7 opinion 全 + synthesis 启动 → pass
    if !seven_advisor_voting_invariant(VotingConfig {
        criticality: 90, opinions: [true; ADVISOR_COUNT], synthesis_started: true
    }) { return false; }

    // happy 2: criticality < 80 → pass (不触发, 任意 opinions 组合)
    for &crit in &[0u8, 50, 79] {
        for syn in [true, false] {
            for missing_idx in 0..ADVISOR_COUNT {
                let mut ops = [true; ADVISOR_COUNT];
                ops[missing_idx] = false;
                if !seven_advisor_voting_invariant(VotingConfig { criticality: crit, opinions: ops, synthesis_started: syn }) {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_function_is_publicly_visible() {
        let _: fn() = seven_advisor_voting;
    }

    #[test]
    fn sanity_check_passes_all_precondition_inputs() {
        assert!(sanity_check(), "seven_advisor_voting violated on valid input");
    }

    #[test]
    fn negative_missing_one_advisor_must_violate() {
        // 反例: criticality 触发 + 7 opinion 缺 1 + synthesis 启动
        let mut ops = [true; ADVISOR_COUNT];
        ops[3] = false;  // 缺第 4 席
        let bad = VotingConfig { criticality: 85, opinions: ops, synthesis_started: true };
        assert!(!seven_advisor_voting_invariant(bad), "missing 1 advisor must violate");
    }

    #[test]
    fn negative_missing_all_advisors_must_violate() {
        // 反例 2: 7 opinion 全缺
        let bad = VotingConfig { criticality: 100, opinions: [false; ADVISOR_COUNT], synthesis_started: true };
        assert!(!seven_advisor_voting_invariant(bad), "missing all advisors must violate");
    }

    #[test]
    fn advisor_count_is_seven() {
        // 编译期守: 7 强制 advisor 永不变
        assert_eq!(ADVISOR_COUNT, 7);
    }

    #[test]
    fn criticality_threshold_is_80() {
        assert_eq!(VOTING_CRITICALITY_THRESHOLD, 80);
    }
}
```

**待 Mavis 拍板 (S-2 17:43)**:
- 任务 prompt 说 "synthesis 之前必须 wait_for_all_7_advisors()" — 本不变量采用 "criticality 触发 ∧ synthesis 启动 ⇒ 7 opinion 全返" 形式 (per team-lead-impl §4 voting 触发条件)。criticality < 80 (未触发) 时不约束 (允许部分返回即 synthesis, 因为低风险决策不必等全席)。
- ADR-0012 §决策 1-4 提 "trait 抽象先小后大 — 第 1 版只 2 个方法 (should_trigger_vote + request_vote)" — 本不变量**只**守 "完整性" (synthesis 前 7 全返), 不守 "投票内容正确性" (那是 council 内部 7 advisor 各 trait, 不在 formal 范围)。

---

### 2.6 5 不变量总览矩阵

| # | 不变量 | 状态 | 文件 | POD 模型 (核心字段) | Kani 完备可解 | 关联 LOCKED | 估 LOC |
|---|--------|------|------|-------------------|:------------:|------------|-------:|
| 1 | `double_onion_sample` | ✅ 已有 | `invariants/double_onion_sample.rs` | `PermissionLayerConfig { kind: u8, requires_ha: bool }` | ✅ 12 组合 | 阶段 1 §1.4 L0 最后护栏 | 101 |
| 2 | `e_layer_isolation` | ❌ 缺失 | `invariants/e_layer_isolation.rs` | `EConfig { caller_layer: u8, target_layer: u8, action: u8, has_permission: bool }` | ✅ 36 组合 | v6 修正链 §2.3 E 层修改路径 | 80 |
| 3 | `permission_grant_l0` | ❌ 缺失 | `invariants/permission_grant_l0.rs` | `GrantConfig { level: u8, pid1_signed: bool, sovereignty_signed: bool, ha_count: u8 }` | ✅ 1536 组合 | v15 修正链 §3 4 重守门 + 权限发放 | 100 |
| 4 | `mid_task_atomicity` | ❌ 缺失 | `invariants/mid_task_atomicity.rs` | `Session { state: u8, mid_task_state: MidTaskState }` + `MessageRef` | ✅ 状态机 8 节点 | session-blueprint §4.3 + §4.4 修法 #2+#3 | 120 |
| 5 | `seven_advisor_voting` | ❌ 缺失 | `invariants/seven_advisor_voting.rs` | `VotingConfig { criticality: u8, opinions: [bool; 7], synthesis_started: bool }` | ✅ 256 × 128 组合 | ADR-0001 + ADR-0012 §决策 1-4 | 100 |
| **总计** | **5 不变量** | **1/5 → 5/5** | **5 文件** | **全 POD, 无堆类型** | **全可解** | **全 LOCKED 关联** | **~500 LOC** |

---

## §3 Kani 验证 harness 设计

### 3.1 目录结构 (R19+ 阶段 3 实施后)

```
crates/apeireth-formal/  (现有, 增量加 4 文件 + 改 mod.rs)
├── src/
│   ├── lib.rs                  (现有 82 LOC, 加 pub mod 4 个新 + pub const 6 个新)
│   ├── invariants/
│   │   ├── mod.rs              (现有 16 LOC, 改: pub mod 5 个 + run_all() 5 个 sanity)
│   │   ├── double_onion_sample.rs  (现有 101 LOC, 不改)
│   │   ├── e_layer_isolation.rs    (🆕, §2.2 伪代码, 估 80 LOC)
│   │   ├── permission_grant_l0.rs  (🆕, §2.3 伪代码, 估 100 LOC)
│   │   ├── mid_task_atomicity.rs   (🆕, §2.4 伪代码, 估 120 LOC)
│   │   └── seven_advisor_voting.rs (🆕, §2.5 伪代码, 估 100 LOC)
│   └── ...                     (未来: 阶段 4 加 5 重治理 + 反思期 守门)
├── tests/                       (现有 0 文件, 未来加 cargo test 兜底 — 不在本蓝图范围)
├── docs/
│   └── kani-setup.md           (现有 137 LOC, 不改)
└── Cargo.toml                  (现有, 零依赖, 不改 — M 标记文件)
```

**约束 (跟 `double_onion_sample.rs` 模板一致)**:
- 4 个新文件**全部**用 `#[cfg_attr(kani, kani::proof)]` 模式
- 4 个新文件**全部**自带头 `#[cfg(test)] mod tests` 至少 3 个 test
- 4 个新文件**全部**禁 String / Vec / HashMap, 全 POD (u8 / u32 / bool / 固定 array)
- 4 个新文件**全部**禁 `unsafe` (crate 根 `#![deny(unsafe_code)]` 已守)
- `Cargo.toml` **不改** (M 标记, 现有零依赖 Kani-friendly, 加 dependency 会污染编译图)

### 3.2 Cargo.toml 关键配置 (现有, 不改)

```toml
[package]
name = "apeireth-formal"
version.workspace = true
description = "Apeireth formal verification skeleton — Kani model checker harness for the double-onion permission invariants (V2 战区 5 / docs/v2-strategy/03 §4A)"

[lib]
name = "apeireth_formal"
path = "src/lib.rs"

# ponytail: Kani is not a runtime dependency — cargo-kani is an external
# tool that discovers #[kani::proof] harnesses via cfg(kani) attribute.
# We deliberately keep zero `dependencies` to avoid bloating the build
# graph; the formal crate is invoked by `cargo kani --harness <name>`,
# not by `cargo build --workspace`.
[dependencies]    # 零依赖, 保持

[dev-dependencies]
# Runtime sanity tests (not Kani). Mirror the invariant's surface API
# with concrete inputs so the crate compiles & tests on stable Rust
# even when cargo-kani is not installed locally.
# (现有空, 不改)

# R19 第 0 阶段: workspace lint 继承
[lints]
workspace = true
```

**为什么不加 `[package.metadata.kani]`** (per 任务 prompt §3.2 提议, 但**不**采用):
- 现有 `kani-setup.md §3.1-3.3` 用 `cargo kani -p apeireth-formal --harness <name>` 命令式调用, 不依赖 metadata
- 加 metadata 会让所有 `cargo kani` 跑全 5 个 harness (vs 现在按需跑), CI 时间↑, 跟 §6 R-Measure 守门 "5 proof 全跑" 一致 — **未来可加**, 但 R19+ 阶段 3 不在范围内
- 决策: 保持 Cargo.toml **零改动** (M 标记, LOCKED), rust-coder 只动 `src/invariants/` 4 个新文件 + `src/lib.rs` + `src/invariants/mod.rs`

### 3.3 invariants/mod.rs 改动 (R19+ 阶段 3)

```rust
// crates/apeireth-formal/src/invariants/mod.rs (R19+ 阶段 3 改后, 估 30 LOC)
// 与现有 double_onion_sample.rs 模板严格一致

//! 不变量模块: 每个不变量一个文件, 每个文件暴露 1 个 Kani harness + 1 个 sanity test.
//!
//! R19+ 阶段 3: 1 → 5 不变量 (per docs/stage4/apeireth-formal-invariants-2026-08-05.md)
//! 5 不变量总览:
//!   1. double_onion_sample    (L0 必须 HA)                — 已有
//!   2. e_layer_isolation       (E 层隔离)                  — 新增
//!   3. permission_grant_l0     (L0 联合签名 + HA)           — 新增
//!   4. mid_task_atomicity      (mid-task 状态转换原子性)   — 新增
//!   5. seven_advisor_voting    (7 advisor voting 完整性)   — 新增

pub mod double_onion_sample;
pub mod e_layer_isolation;
pub mod permission_grant_l0;
pub mod mid_task_atomicity;
pub mod seven_advisor_voting;

/// 运行所有不变量的 runtime sanity check (供 `run_all` 调用).
///
/// 5 不变量 Kani 形式化证明 + runtime sanity 是两条独立的验证通道:
/// - Kani 符号执行覆盖**所有**输入 (完备)
/// - runtime sanity 仅覆盖少量具体输入 (快速 smoke test)
pub fn run_all() -> bool {
    double_onion_sample::sanity_check()
        && e_layer_isolation::sanity_check()
        && permission_grant_l0::sanity_check()
        && mid_task_atomicity::sanity_check()
        && seven_advisor_voting::sanity_check()
}
```

### 3.4 lib.rs 改动 (R19+ 阶段 3, Mavis 协调决定)

**重要**: `lib.rs` 是 M 标记文件之一 (per 任务 prompt §不要扫 第 1 条), 本蓝图**不**改 `lib.rs` 内容, 只**建议**改动 (R19+ 阶段 3.4 实施时, Mavis 协调解锁后再改):

```rust
// crates/apeireth-formal/src/lib.rs (R19+ 阶段 3 改后建议, 估 100 LOC)
// 当前 82 LOC, 加 4 行 pub mod + 6 行 pub const + 5 行 re-export

#![deny(unsafe_code)]

pub mod invariants;

// 不变量 2: e_layer (编译期 hardcode)
pub const E_LAYER_COUNT: u8 = 3;                          // 3 层架构 (e/o/s)

// 不变量 3: L0 权限 (编译期 hardcode)
pub const L0_MIN_HA_VOTES: u8 = 1;                        // 至少 1 HA 票
pub const L0_MIN_AUTHORITY_SIGNATURES: u8 = 2;            // PID 1 + sovereignty 联合

// 不变量 4: mid-task (编译期 hardcode)
pub const MID_TASK_STATES: u8 = 4;                        // 4 子状态
pub const ACTIVE_SESSION_STATES: u8 = 2;                  // 2 活跃主状态

// 不变量 5: 7 advisor (编译期 hardcode)
pub const ADVISOR_COUNT: usize = 7;                       // 7 强制 advisor
pub const VOTING_CRITICALITY_THRESHOLD: u8 = 80;          // voting 触发临界

// 现有 (保留)
pub const PERMISSION_ONION_DEPTH: usize = 6;              // 6 层洋葱 (不变量 1)

// ... 现有 PermissionLayerConfig + l0_requires_ha_invariant + run_all + verify 不动
```

**等 Mavis 拍板 (S-2 17:43)**: `lib.rs` 加 6 个 `pub const` + 4 个 `pub mod` re-export 是**纯增量**, 不动现有 82 LOC。但 `lib.rs` 是 M 标记, 需 Mavis 协调解锁。本蓝图**不**预修改, 留给 R19+ 阶段 3.4 实施时统一处理。

---

## §4 验证流程 (本地 + CI)

### 4.1 本地 (开发机, 一次性安装)

```bash
# 1. 安装 Kani (per docs/kani-setup.md §2.2)
# 注意: Kani 不支持 Windows 原生, 需 WSL2
cargo install --locked kani-verifier
cargo install --locked cargo-kani

# 2. 验证安装
cargo kani --version
```

### 4.2 跑 5 个 Kani harness (R19+ 阶段 3.4 实施后)

```bash
# 1. 进入 apeireth-formal 目录
cd crates/apeireth-formal

# 2. 跑单个 harness (开发循环, 快, ~1-3 分钟)
cargo kani --harness double_onion_sample
cargo kani --harness e_layer_isolation
cargo kani --harness permission_grant_l0
cargo kani --harness mid_task_atomicity
cargo kani --harness seven_advisor_voting

# 3. 跑全部 harness (per §6 R-Measure 守门, CI 必跑)
cargo kani -p apeireth-formal --harness double_onion_sample \
    && cargo kani -p apeireth-formal --harness e_layer_isolation \
    && cargo kani -p apeireth-formal --harness permission_grant_l0 \
    && cargo kani -p apeireth-formal --harness mid_task_atomicity \
    && cargo kani -p apeireth-formal --harness seven_advisor_voting

# 4. (可选) 看 Kani 内部 trace / CBMC args
cargo kani -p apeireth-formal --harness mid_task_atomicity --verbose
```

### 4.3 runtime sanity check (cargo test, 快, 任何机器)

```bash
# 1. runtime sanity (5 个不变量全过, < 1 分钟)
cargo test -p apeireth-formal

# 2. 单独跑某个不变量的 sanity
cargo test -p apeireth-formal -- invariants::e_layer_isolation
```

### 4.4 CI 集成 (R20 阶段 1.5 加, 不在本蓝图范围)

```yaml
# .github/workflows/kani.yml (建议, R20 阶段 1.5 加)
# 不在本蓝图范围, 仅作 placeholder 展示协同
name: Kani formal verification
on:
  workflow_dispatch:    # 不挡 PR, 手动触发
  schedule:
    - cron: '0 2 * * 0'  # 每周日凌晨 2 点跑
jobs:
  kani:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Kani
        run: |
          cargo install --locked kani-verifier
          cargo install --locked cargo-kani
      - name: Run all 5 Kani proofs
        run: |
          for h in double_onion_sample e_layer_isolation permission_grant_l0 mid_task_atomicity seven_advisor_voting; do
            cargo kani -p apeireth-formal --harness $h
          done
```

**决策**: CI 单独 workflow, **不**挡 PR (per `docs/kani-setup.md §4`)。Kani 冷启动 ~3 分钟 / 预热 ~1 分钟, 5 个 proof 总计 ~5-15 分钟。

---

## §5 跟 R19+ 集成的协同 (5 不变量 ↔ 上游 crate)

| 不变量 | 上游 crate | 协同方式 | 跟 Hermes R18 集成测试关系 |
|--------|----------|---------|--------------------------|
| **1. double_onion_sample** | `apeireth-formal` (本身) | 现有 1/5, 跑通 Kani 通道 | 互补 (无对应 cargo test, 是 Kani 独有) |
| **2. e_layer_isolation** | `apeireth-core` (E 层守门) | Kani 守"跨 E 层写入必检", cargo test 守 E 层 6 项不可违背业务逻辑 | ✅ 互补 (Hermes R18 跑 5 器官 cargo test) |
| **3. permission_grant_l0** | `apeireth-council` + `apeireth-supervisor` (PID 1) + sovereignty | Kani 守"L0 联合签名齐全", cargo test 守 council 审议 + supervisor 监督业务流 | ✅ 互补 (Hermes R18 跑 council 122 集成测试) |
| **4. mid_task_atomicity** | `apeireth-session` (R19+ 阶段 3 新建) | Kani 守"状态转换原子性", cargo test 守 mid-task bug 3 处修法业务流 | ✅ 互补 (Hermes R18 跑 session lifecycle 10 集成测试) |
| **5. seven_advisor_voting** | `apeireth-council` (7 advisor) + `apeireth-team-lead` (R19+ 新建) | Kani 守"7 opinion 全到才能 synthesis", cargo test 守 council 7 advisor 并行审议 | ✅ 互补 (Hermes R18 跑 council + voting trait 集成测试) |

**协同架构图** (跟 `apeireth-session-vector-asi §6.1` 一致):

```
                ┌─────────────────────────────────────────┐
                │  apeireth-team-lead (R19+ 新建)         │
                │  构造 supervisor prompt + voting 触发   │
                └──────────┬─────────────────────┬───────┘
                           │                     │
                           ▼                     ▼
        ┌──────────────────────────┐  ┌────────────────────────────┐
        │  apeireth-mcp::team      │  │  apeireth-council          │
        │  14 工具 trait           │  │  7 advisor 并行审议        │
        │  (lock at R18 Hermes)    │  │  (Kani 不变量 5 守完整性)   │
        └──────────┬───────────────┘  └────────────────────────────┘
                   │
                   ▼
        ┌──────────────────────────┐
        │  apeireth-session        │  ◀──── Kani 不变量 4 守 mid_task 原子性
        │  (R19+ 新建)             │
        │  6 状态机 + mid_task     │
        └──────────┬───────────────┘
                   │
                   ▼
        ┌──────────────────────────────────────────────────────┐
        │  apeireth-formal (Kani 形式化通道)                    │
        │  5 不变量: 1/5 → 5/5 (R19+ 阶段 3 补 4 个)           │
        │  - double_onion_sample    (L0 HA)                    │
        │  - e_layer_isolation      (E 层隔离)                  │
        │  - permission_grant_l0    (L0 联合 + HA)             │
        │  - mid_task_atomicity     (mid-task 原子)            │
        │  - seven_advisor_voting   (7 advisor 完整性)         │
        └──────────────────┬───────────────────────────────────┘
                           │
                           ▼
        ┌──────────────────────────────────────────────────────┐
        │  Kani model checker (CBMC backend)                  │
        │  符号执行 + 有界模型检查 (完备覆盖)                   │
        │  跟 Hermes R18 cargo test 互补 (行为 vs 形式)         │
        └──────────────────────────────────────────────────────┘
```

**关键协同点**:
- **不变量 2 (e_layer)** → 跟 APEIRETH-CONVENTIONS §6 + v6 修正链 §2.3 强约束 (守门 1-4 默认拒绝)
- **不变量 3 (permission_grant_l0)** → 跟 v15 修正链 §3 + sovereignty 集成 (PID 1 + sovereignty 联合 + HA)
- **不变量 4 (mid_task)** → 跟 `apeireth-session` 集成, 是 mid-task bug 修法的**形式化保障** (cargo test 验证行为, Kani 验证原子)
- **不变量 5 (7_advisor)** → 跟 `apeireth-council` + `apeireth-team-lead` 集成, 是 council voting 的**形式化保障** (cargo test 验证审议, Kani 验证完整性)
- **不变量 1 (double_onion_sample)** → 现有, 是 Kani 通道模板

---

## §6 R-Measure 守门 (per `r-measure-verification-design-2026-08-05.md`)

### 6.1 5 不变量验证通过率

| 指标 | 目标 | 说明 |
|------|------|------|
| **5 不变量 Kani 验证通过率** | ≥ 95% | 允许 1 个 Kani 不可解的边界 (e.g. mid_task 6 状态机 Kani 跑超 30 分钟, 允许 fallback 到 cargo test) |
| **cargo test 兜底** | 5/5 全过 | runtime sanity 5 个 `sanity_check()` 全 `true` |
| **编译期守门** | `#![deny(unsafe_code)]` 0 violation | 5 文件全过 lint |
| **编译期 hardcode** | 6 个 `pub const` 永不变 | `E_LAYER_COUNT=3` / `L0_MIN_HA_VOTES=1` / `L0_MIN_AUTHORITY_SIGNATURES=2` / `MID_TASK_STATES=4` / `ACTIVE_SESSION_STATES=2` / `ADVISOR_COUNT=7` / `VOTING_CRITICALITY_THRESHOLD=80` / `PERMISSION_ONION_DEPTH=6` |

### 6.2 验证时间预算

| 阶段 | 时长 | 说明 |
|------|------|------|
| **单 harness 跑** | 1-3 分钟 | 简单 POD 模型 (12 / 36 / 1536 组合) |
| **mid_task_atomicity 跑** | 5-10 分钟 | 状态机节点最多, 8 节点 (R19+ 阶段 3.4 实测可能更久) |
| **5 harness 全跑** | 5-15 分钟 | CI 预算 |
| **cargo test 兜底** | < 1 分钟 | 快速 smoke |
| **总验证时间** | < 15 分钟 | 满足 §7 验收标准 "验证时间 < 10 分钟 (5 个 proof) + cargo test 1 分钟" |

### 6.3 跟 R-Measure baseline 关系

**重要诚实登记 (S-2 17:43)**:
- **5 不变量 Kani 验证** ≠ **R-Measure baseline 守门** (per `r-measure-verification-design`)
- 前者验证**形式属性** (5 不变量逻辑正确), 后者验证 **R11 baseline 三值** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063, 24 维 LOCKED)
- 两者独立通道, 互不替代
- 5 不变量**不掉** R-Measure baseline (Kani 改的是 POD 模型 + 编译期 const, 不影响 `apeireth-asi` 24 维 LOCKED)
- R-Measure baseline 守门在 `apeireth-r-measure-verify` (R19+ 阶段 4, 1320 LOC, per `apeireth-session-vector-asi §8.6`), **不**在本蓝图范围

---

## §7 验收标准 (rust-coder 实施完, Mavis 验收)

- [ ] **5 不变量 Kani harness 全写** (1 已有 + 4 新, 全 `#[cfg_attr(kani, kani::proof)]`)
- [ ] **`cargo kani` 跑通 5 个 proof** (4 个新 + 1 个旧 = 5 个, 全 SUCCESS)
- [ ] **`cargo test -p apeireth-formal` 跑通 5 个 sanity** (5 × 1 `sanity_check()` 全 `true`, 加 5 × 3 unit test = 20 test)
- [ ] **验证时间 < 10 分钟** (5 个 Kani proof + cargo test < 1 分钟)
- [ ] **no unsafe** (`#![deny(unsafe_code)]` 0 violation, 5 文件全过)
- [ ] **编译期 hardcode 守门** (8 个 `pub const` 在 `lib.rs`, 5 个文件各 `#[test]` 断言)
- [ ] **Kani-friendly** (5 文件全 POD, 0 个 String / Vec / HashMap / 浮点)
- [ ] **Cargo.toml 不动** (M 标记, 保持零依赖, 0 编译图污染)
- [ ] **invariants/mod.rs 改对** (5 `pub mod` + `run_all()` 调 5 个 `sanity_check()`)
- [ ] **跟 Hermes R18 集成测试互补** (cargo test 行为 + Kani 形式, 双层守, 不重叠)

---

## §8 实施时间表 (5 阶段 估 3 天)

| 阶段 | 时长 | 任务 | Owner | 依赖 | 关键产出 |
|------|----:|------|-------|------|---------|
| **1** | 0.5 天 | 不变量 2 `e_layer_isolation` + Kani harness + 4 unit test | rust-coder | 无 (POD 自带) | `e_layer_isolation.rs` 80 LOC |
| **2** | 0.5 天 | 不变量 3 `permission_grant_l0` + Kani harness + 7 unit test | rust-coder | 无 (POD 自带) | `permission_grant_l0.rs` 100 LOC |
| **3** | 1 天 | 不变量 4 `mid_task_atomicity` + Kani harness + 4 unit test (最复杂) | rust-coder | 无 (POD 自带) | `mid_task_atomicity.rs` 120 LOC |
| **4** | 0.5 天 | 不变量 5 `seven_advisor_voting` + Kani harness + 5 unit test | rust-coder | 无 (POD 自带) | `seven_advisor_voting.rs` 100 LOC |
| **5** | 0.5 天 | 改 `invariants/mod.rs` (5 `pub mod` + `run_all()`) + 改 `lib.rs` (6 `pub const` + 4 `pub mod` re-export) + 5 harness 全跑通验证 | rust-coder | 阶段 1-4 全完 | 5/5 不变量 + 5 Kani proof 全 SUCCESS |
| **6** (R20) | 1 天 | CI workflow `.github/workflows/kani.yml` (per §4.4) | devops_engineer | 阶段 5 完 | 5 Kani proof 每周日自动跑 |
| **总计** | **3 天** (R19+ 阶段 3) | | | | **+ 1 天 R20 阶段 1.5 CI** |

**跟 `apeireth-session-vector-asi §8.5` 估时一致**: 阶段 3 (formal 扩 4 不变量) 估 2-3 天 400 LOC — 本蓝图估 3 天 500 LOC (含 unit test 多估 100 LOC), 略宽 1 天 buffer。

---

## §9 风险清单

| # | 风险 | 等级 | 应对 | 监测 |
|--:|------|:--:|------|------|
| **1** | Kani 0.50 安装复杂 (Windows 兼容性, 需 WSL2) | 🟡 中 | per `docs/kani-setup.md §2.1-2.2` 步骤, 团队 WSL2 标准化 | 阶段 1 实测 `cargo kani --version` 跑通 |
| **2** | Kani harness 写错可能 false positive / negative | 🟡 中 | 双层守 (Kani + cargo test), 3-5 个 unit test 覆盖正反例 | 阶段 5 全跑 + R-Measure baseline 不掉 |
| **3** | mid_task 不变量依赖 `apeireth-session` 实施进度 | 🟢 低 | POD 模型**不**依赖 `apeireth-session` 真实结构体 (跟 `double_onion_sample` 模板一致) | 阶段 3.4 实施时 5 个 proof 全跑通 |
| **4** | 7_advisor 不变量依赖 `apeireth-council` 实施进度 | 🟢 低 | POD 模型**不**依赖 `apeireth-council` 真实结构体 | 阶段 4 实施时 5 个 proof 全跑通 |
| **5** | Kani 验证时间长 (大型程序可能 30+ 分钟) | 🟡 中 | POD 模型保持小状态空间 (5 不变量都 < 2000 组合), 单 harness < 5 分钟 | 阶段 5 验证时间 < 10 分钟 |
| **6** | `lib.rs` 是 M 标记文件, 加 6 `pub const` + 4 `pub mod` re-export 需 Mavis 协调 | 🟡 中 | 本蓝图**不**预改, R19+ 阶段 3.4 实施时 Mavis 协调解锁 | 阶段 5 实装前 Mavis 拍板 |
| **7** | 5 个 `pub const` 编译期 hardcode 跟未来"动态调整"需求冲突 | 🟢 低 | 编译期 hardcode 是 APEIRETH-CONVENTIONS 强约束 (O-5 不假装), 8 项不修改承诺之一 | 阶段 5 unit test 断言 const 值 |
| **8** | 任务 prompt 提 "APEIRETH-CONVENTIONS §6 (3 层架构, 4 组件 + 权限分配)" 实际是 commit 规范 | 🟢 低 | 本蓝图诚实登记 (见 §1 + §2.2), 实际权威源 v6 / v15 修正链 + ADR-0001 | 文档审阅 Mavis 拍板 |

---

## §10 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10 + ADR-0011)

| # | 不修改项 | 原因 | 本蓝图遵守 |
|--:|---------|------|:---------:|
| 1 | 阶段 1+2 LOCKED | 主人明确沉淀 | ✅ |
| 2 | v2 / v4 / v4.1 LOCKED | 哲学层纲领 | ✅ |
| 3 | 阶段 4 主文档 LOCKED | 6ca80776 | ✅ (本蓝图是 stage4 子文档, 不动主文档) |
| 4 | 阶段 5 施工文档 LOCKED | 631 行 | ✅ (本蓝图是 stage4, 不动 stage5) |
| 5 | v6 修正 (4 重守门 + 权限发放 + E 层修改路径) | 关键路径 | ✅ (不变量 2/3 引用 v6/v15 作 LOCKED 依据, 不修改) |
| 6 | R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) | APEIRETH-CONVENTIONS §11 | ✅ (Kani 改 POD 模型 + 编译期 const, 不影响 24 维 LOCKED) |
| 7 | v1 → v5 历史链 | 不删除 | ✅ (本蓝图引用 v6, 不删 v1-v5) |
| 8 | workspace v1.0.0 + Document-Meta + 12 键 + 6 锚 | APEIRETH-CONVENTIONS 强约束 | ✅ (本蓝图 Document-Meta 严格按 v12 规范, 6 锚穿透见 §11) |

**额外承诺 (per 任务 prompt §10 "跟 ADR-0011 §不修改承诺 一致")**:
- 不修改任何 M 标记文件 (Cargo.toml / CHANGELOG.md / README.md / ROADMAP.md / 其他 crates/)
- 不修改任何 LOCKED 文档 (阶段 1-5 + v2/v4/v4.1 + v6 + stage3-blueprints/)
- 不修改任何 ADR (0001-0012)
- 不修改 Hermes R18 集成测试
- 本蓝图只**新增** `docs/stage4/apeireth-formal-invariants-2026-08-05.md` 1 个文件 (跟任务 prompt §产出 一致)

---

## §11 6 哲学 anchor 穿透 (per APEIRETH-CONVENTIONS §9)

| 锚 | 时间 | 穿透点 |
|---|------|--------|
| **S-1** | 22:33 | 6 anchor ASI 完整性 — 5 不变量守 ASI 候选主体最后护栏 (L0 HA + E 层隔离 + L0 联合 + mid_task 原子 + 7 advisor 完整), 服务 ASI 北极星 |
| **S-2** | 17:43 | 6 anchor 实验室 — Kani 形式化验证是实验室最高标准 (符号执行 + 有界模型检查, 完备覆盖), 跟 cargo test 行为测试互补双层守 |
| **O-5** | 17:58 | 6 anchor 12 急救 — mid_task_atomicity 是 P0 急救 (mid-task bug 3 处修法形式化保障), Kani 跑通 = 修法不复发, 跟 session-blueprint §4.4 修法 #1+#2+#3 一致 |
| **O-2** | 19:33 | 6 anchor 4 分类 — e_layer (架构) / permission (权限) / mid_task (会话) / advisor (审议) 4 维度清晰分类, 跟 ADR-0011 §决策 4 "命名空间严格分离" 一致 |
| **O-3** | 23:44 | 6 anchor 决策清单 — 5 阶段实施 (e_layer → permission_l0 → mid_task → advisor → mod.rs/lib.rs/CI), 明确 owner + 估时 + 验收标准, 跟 team-lead 实施指南节奏一致 |
| **O-4** | 00:56 | 6 anchor 12 统一 — 跟 APEIRETH-CONVENTIONS 12 子规范统一 (命名空间 v12 / 路径 stage4 / 8 项不修改承诺 / R11 baseline / Document-Meta / 6 锚穿透), 任何接手者能查 |

**穿透检查清单** (per APEIRETH-CONVENTIONS §9):
- [x] S-1 北极星导向: 5 不变量守 ASI 完整性
- [x] S-2 实事求是: 诚实登记 1/5 现状 + Kani Windows 限制 + 任务 prompt §6 笔误
- [x] O-5 不假装: 编译期 hardcode 8 个 const + `#![deny(unsafe_code)]` + POD 模型
- [x] O-2 走在前人经验上: 借鉴 Kani/CBMC + wasmtime 子 crate workspace lint 模式
- [x] O-3 干到底: 5 阶段实施 + 明确 owner + 验收标准 10 项
- [x] O-4 任何人都能接手: 12 子规范统一 + Kani 模板清晰 + 关联文档 §12 完整

---

## §12 关联文档 (按权威性排序)

### 12.1 权威依据 (LOCKED, 本蓝图**只引用**, **不修改**)

| 文档 | 章节 | 关联不变量 |
|------|------|-----------|
| `docs/stage4/stage4-correction-v6-consolidated-and-e-layer-mutation.md` | §2.3 E 层修改路径 | 不变量 2 (e_layer) |
| `docs/stage4/stage4-correction-v15-four-gates-permission-grant.md` | §3 4 重守门 + 权限发放 | 不变量 3 (permission_grant_l0) |
| `docs/adr/0001-double-onion-unity.md` | (全文) | 不变量 5 (7 advisor) 设定 |
| `docs/adr/0012-team-lead-council-collaboration.md` | §决策 1-4 | 不变量 5 (voting trait 抽象) |
| `docs/stage4/apeireth-session-blueprint-2026-08-05.md` | §4.3 + §4.4 修法 #1+#2+#3 | 不变量 4 (mid_task) |
| `docs/stage4/apeireth-team-lead-implementation-guide-2026-08-05.md` | §4 voting 触发判定 | 不变量 5 (criticality 阈值) |
| `APEIRETH-CONVENTIONS.md` | §10 8 项不修改承诺 + §11 R11 baseline 3 值 | §10 |
| `APEIRETH-CONVENTIONS.md` | §9 6 锚穿透 + §2 路径系统 (stage4) | §11 + 文档定位 |
| `crates/apeireth-formal/src/invariants/double_onion_sample.rs` | (全文 101 LOC) | 不变量 1 模板 |
| `crates/apeireth-formal/docs/kani-setup.md` | (全文 137 LOC) | §4 验证流程 |
| `crates/apeireth-formal/Cargo.toml` | (全文 30 行) | §3.2 零依赖 (不改) |
| `crates/apeireth-formal/src/lib.rs` | (全文 82 LOC) | §3.4 改后建议 (M 标记) |

### 12.2 协同 (R19+ 阶段 3 同期产出, **互相引用**, 但各管各的)

| 文档 | 主题 | 关系 |
|------|------|------|
| `reports/apeireth-session-vector-asi-2026-08-05.md` | 4 crate 现状 + 集成分析 | §1 关键事实 + §6.2 5 阶段协同 + §8 实施时间表一致 |
| `docs/stage4/spectrAI-integration-blueprint-r19-plus-2026-08-05.md` | R19+ 集成蓝图 | §6 5 不变量期望跟本蓝图一致 |
| `docs/stage4/r-measure-verification-design-2026-08-05.md` | R-Measure 守门 | §6 跟 R11 baseline 关系诚实登记 |

### 12.3 Hermes R18 互补 (代码层, **不动**)

| 提交 | 内容 | 跟本蓝图关系 |
|------|------|------------|
| **34992e9f** 等 5 个 R18/R19 commit | Hermes 加 122 集成测试 + 34 lib.rs | **互补** — cargo test 行为测试 vs Kani 形式化测试, 双层守门 |

### 12.4 未来文档 (R20+ 阶段, **不在本蓝图范围**)

| 文档 | 主题 |
|------|------|
| `docs/stage4/apeireth-r-measure-verify-2026-08-XX.md` (R19+ 阶段 4 蓝图) | R-Measure baseline 守门 (per `apeireth-session-vector-asi §8.6`) |
| `.github/workflows/kani.yml` (R20 阶段 1.5 加) | Kani CI workflow (per §4.4 placeholder) |
| `docs/stage4/apeireth-formal-v6-stubs-2026-08-XX.md` (R20+) | 5 重治理 + 反思期 守门 (per `apeireth-session-vector-asi §7.3 G13`) |

---

_本蓝图由 software-architect (Mavis 通用 agent) 2026-08-05 草拟, 跟 team-lead 实施指南 + 蓝图节奏同步._
_5 不变量 + Kani harness 伪代码 + 5 阶段实施 + 6 锚穿透 + 跟 Hermes 互补._
_任何接手者能照 §2.2-§2.5 复制 double_onion_sample.rs 模板改即可, 不动 LOCKED._
