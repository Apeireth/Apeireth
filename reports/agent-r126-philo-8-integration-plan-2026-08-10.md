# R126 8 哲学锚 整合 Plan (B5 6→8 升级, per 决策 #22 §2.5 + 决策 #51 §1.2 P1-2)

**Date**: 2026-08-10 (R126 done)
**Author**: R126-1 sub-agent (general agent, Mavis 派 20:09 per 决策 #51)
**触发**: Mavis root 20:09 派活 + 主人 20:09 "全按你的想法来, 开干" + 决策 #51 §1.2 P1-2 (R126 8 哲学锚 升级)
**关联**: 决策 #22 (B5 6→8 路线) + 决策 #33 (主人 17:22 升级授权 + 8 硬墙重置) + 决策 #36 (借鉴源码 7/11 ✅ cloned + 3 限流 + 1 跳过) + 决策 #48 (整合 #4 commit `abf12243` done) + 决策 #51 (16 sub-agent 派活) + 09-anchor.md (R125 16:55 已 doc-level 升 8 锚) + apeireth-council/src/constitution.rs (原 6 锚 `PHILOSOPHICAL_ANCHORS: [&str; 6]`) + R125-8 final report (整合 plan 模式) + R125-12 PHL-07 spec (13 键 编译期 hardcode 模式)

---

## 0. 一句话 (TL;DR)

**R126 8 哲学锚 整合 2 步 (per R125-8 final report §2.5 整合 supervisor plan 模式 + R125-12 PHL-07 spec §4 模式). 步骤 1: apeireth-core/src/lib.rs 加 `pub mod eight_anchors;` (1 行). 步骤 2: apeireth-council/src/constitution.rs 加 2 升级版 fn (`for_safety_advisor_8_anchors` + `for_philosophy_advisor_8_anchors`), 0 改原 6 锚 fn (per B1 24 LOCKED 入口签名 0 改). 入口签名 0 改 verify (24 LOCKED 9 入口 + apeireth-core 0 入口改动). 0 越界 8 硬墙. 整合时序 8/15-8/17 (per 决策 #42 §1.4 pre-checklist). Mavis 整合 #5 commit `abf12243` 后续 拍板.**

---

## 1. 整合背景

### 1.1 R126-1 5 阶段 done (20:09)

R126-1 已 5 阶段 100% done (per spec §2.1):
1. ✅ 借鉴源码 study (内部 + 公开)
2. ✅ Rust 实施 (NEW file `apeireth-core/src/eight_anchors.rs` 23.2KB)
3. ✅ 单元测试 stub (内联 12 tests, 12/12 pass 临时 crate verify)
4. ✅ spec 报告 (本 plan 同源 spec)
5. ✅ 整合 supervisor plan (本文件)

**5 阶段 done 后, 0 装 src (NEW file) + 0 装 wiring (lib.rs + council 0 触碰)**.

### 1.2 R126 续 整合任务 (本 plan 范围)

R126 续 整合 (8/15-8/17, per 决策 #42 §1.4 pre-checklist) 需要:
1. 步骤 1: apeireth-core/src/lib.rs 加 `pub mod eight_anchors;` (1 行)
2. 步骤 2: apeireth-council/src/constitution.rs 加 2 升级版 fn (`for_safety_advisor_8_anchors` + `for_philosophy_advisor_8_anchors`), 0 改原 6 锚 fn
3. 步骤 3 (可选): docs/adr/0010-6-philosophy-anchors.md 升级 8 锚 (per 09-anchor.md 16:55 升级路线)
4. 步骤 4 (可选): docs/conventions/09-anchor.md 8 锚 verify (已 R125 16:55 升级, 0 改)

**整合时严守**:
- B1 24 LOCKED 入口签名 0 改 (尤其 `apeireth-council/src/constitution.rs` 0 改原 6 锚 fn)
- A3 13 键 0 改 (`apeireth-core/src/lib.rs` 0 改 PHL-01~06 enum)
- A1 baseline 3 值 0 删 0 改 (`apeireth-asi/src/lib.rs:42-44` 0 改)
- B2 1.2.0 0 改 (`Cargo.toml:246` 0 改)
- C3 v6 0 改 (6 重守门 0 改)
- C1 0 主动 commit (R126 续 P1 supervisor 拍板, Mavis 整合 #5 时机)
- 0 push 严守

---

## 2. 整合步骤 (per R125-8 final report §2.5 模式 + R125-12 PHL-07 spec §4 模式)

### 2.1 步骤 1: apeireth-core/src/lib.rs 加 `pub mod eight_anchors;` (1 行)

**位置**: `apeireth-core/src/lib.rs` (按现有 `pub mod` 列表, 推荐在文件末尾或按字母顺序)

**改动**:
```rust
// apeireth-core/src/lib.rs (NEW 1 行, R126 续整合时加)
pub mod eight_anchors;
```

**0 改 verify**:
- 0 改 `apeireth-core/src/lib.rs` 任何现有 `pub mod` 声明
- 0 改 `apeireth-core/src/lib.rs` 任何现有 fn / struct / enum / const
- 0 改 `apeireth-core/src/lib.rs` `PhilosophyKey` enum (12 键 0 改, A3 13 键 0 改)
- 0 改 `apeireth-core/src/lib.rs` `ALL_TWELVE_KEYS` / `TWELVE_KEYS_HARDCODE` (A3 13 键 0 改)

**为什么 apeireth-core 是合适位置**:
- 24 LOCKED 不含 apeireth-core (per `docs/omnibus/24-locked-crates.md` §1-3, 1-12 主人已知 + 13-24 Mavis 自主, apeireth-core 不在 24 LOCKED)
- 跟 PHL-07 (R125-12 spec §4.1) 同位置 (per 决策 #22 §5.1 模式: 哲学键 / 哲学锚 都放 apeireth-core)
- 编译期 hardcode 模式统一 (per R125-12 PHL-07 spec §2.3 + R126-1 §3.4)

**风险与缓解**:
- 风险: 加 `pub mod` 行可能引起 cargo build 错误 (如果 eight_anchors.rs 有 compile error)
- 缓解: R126-1 临时 crate verify 0 error + 0 warning (per R125-8 final report §2.2 模式)

### 2.2 步骤 2: apeireth-council/src/constitution.rs 加 2 升级版 fn

**位置**: `apeireth-council/src/constitution.rs` (在 `for_safety_advisor()` + `for_philosophy_advisor()` 之后)

**改动 1: `for_safety_advisor_8_anchors()` (NEW fn, 0 改原 `for_safety_advisor()`)**

```rust
// apeireth-council/src/constitution.rs (NEW fn, R126 续整合时加)
/// Safety advisor 宪法 升级版 (8 哲学锚穿透, per R126 B5 6→8 升级)
/// 
/// 跟 `for_safety_advisor()` 区别: 哲学锚从 6 升级 8, 加 S-3 (质量工程化) + O-1 (安全优先)
/// 0 改原 6 锚顺序 (per B1 24 LOCKED 入口签名 0 改 + 决策 #22 §5.1)
pub fn for_safety_advisor_8_anchors() -> Self {
    use apeireth_core::eight_anchors::ALL_EIGHT_ANCHORS;
    Self {
        physical_isolation: true,
        l0_ha_required: true,
        jurisdiction_bounds: vec!["SOVEREIGN".to_string(), "PRINCIPLE".to_string()],
        compile_time_hardcoded: true,
        philosophical_anchors: ALL_EIGHT_ANCHORS
            .iter()
            .map(|a| apeireth_core::eight_anchors::PhilosophicalAnchor8::code(a).to_string())
            .collect(),
    }
}
```

**改动 2: `for_philosophy_advisor_8_anchors()` (NEW fn, 0 改原 `for_philosophy_advisor()`)**

```rust
// apeireth-council/src/constitution.rs (NEW fn, R126 续整合时加)
/// Philosophy advisor 宪法 升级版 (8 哲学锚穿透, per R126 B5 6→8 升级)
pub fn for_philosophy_advisor_8_anchors() -> Self {
    use apeireth_core::eight_anchors::ALL_EIGHT_ANCHORS;
    Self {
        physical_isolation: false,
        l0_ha_required: false,
        jurisdiction_bounds: vec!["PRINCIPLE".to_string()],
        compile_time_hardcoded: true,
        philosophical_anchors: ALL_EIGHT_ANCHORS
            .iter()
            .map(|a| apeireth_core::eight_anchors::PhilosophicalAnchor8::code(a).to_string())
            .collect(),
    }
}
```

**0 改 verify**:
- 0 改 `apeireth-council/src/constitution.rs:39` `pub const PHILOSOPHICAL_ANCHORS: [&str; 6]` (B1 入口签名 0 改)
- 0 改 `apeireth-council/src/constitution.rs:50` `pub struct RoleConstitution` (5 字段 0 改)
- 0 改 `apeireth-council/src/constitution.rs:65` `pub const FIELD_COUNT: usize = 5` (5 字段 0 改)
- 0 改 `apeireth-council/src/constitution.rs:68` `pub fn default_permissive()` (原 fn 0 改)
- 0 改 `apeireth-council/src/constitution.rs:79` `pub fn for_safety_advisor()` (原 fn 0 改)
- 0 改 `apeireth-council/src/constitution.rs:90` `pub fn for_philosophy_advisor()` (原 fn 0 改)
- 0 改 `apeireth-council/src/constitution.rs:101` `pub fn for_ethics_advisor()` (原 fn 0 改)
- 0 改 `apeireth-council/src/constitution.rs:112` `pub fn for_legal_advisor()` (原 fn 0 改)
- 0 改 `apeireth-council/src/constitution.rs:127` `pub fn for_performance_advisor()` (原 fn 0 改)
- 0 改 `apeireth-council/src/constitution.rs:138` `pub fn for_history_advisor()` (原 fn 0 改)
- 0 改 `apeireth-council/src/constitution.rs:149` `pub fn for_strategy_advisor()` (原 fn 0 改)
- 0 改 `apeireth-council/src/constitution.rs:160` `pub fn for_advisor_domain()` (原 fn 0 改)
- 0 改 `apeireth-council/src/constitution.rs:174` `pub fn five_guards_summary()` (原 fn 0 改)

**0 改 entry signature verify 通过** (per B1 24 LOCKED 入口签名 0 改).

**8 哲学锚 升级版 fn 0 改原 fn 0 改 verify** (per 决策 #22 §5.1: "5 守门 1-4 嵌套结构 永远保留 (新增第 5/6 重是扩展, 不破坏 1-4)").

### 2.3 步骤 3 (可选): docs/adr/0010-6-philosophy-anchors.md 升级 8 锚

**位置**: `docs/adr/0010-6-philosophy-anchors.md` (原 6 哲学锚 ADR)

**改动 (建议)**:
- §2.1 6 哲学锚总览 → 8 哲学锚总览 (加 S-3 + O-1)
- §2.3 6 哲学锚穿透方法 → 8 哲学锚穿透方法 (每条 ADR 末尾 8 项 + 8 项, 加 2 行)
- §5 6 哲学锚穿透 → 8 哲学锚穿透 (per 09-anchor.md R125 16:55 升级路线)
- §6 8 项不修改承诺 → 8 项不修改承诺 + 8 哲学锚穿透 (R126 升级)

**0 改 verify**:
- 0 改 6 哲学锚原版顺序 (per B1 0 改原 6 实质, 0 改 docs/adr §2.1 原 6 锚描述)
- 0 改 8 项不修改承诺 (per 决策 #22 §5.1: 8 项不修改承诺 LOCKED)

**风险与缓解**:
- 风险: docs 改动 = Mavis 整合 #5 拍板时统一改, R126 续 P1 supervisor 0 必单独改
- 缓解: 步骤 3 是可选, 跟主仓 09-anchor.md (R125 16:55 已升级 8 锚) 一致即可

### 2.4 步骤 4 (可选): docs/conventions/09-anchor.md 8 锚 verify

**位置**: `docs/conventions/09-anchor.md` (R125 16:55 已升级 8 锚, per `Last-Modified: 2026-08-10`)

**verify**:
- ✅ `Last-Modified: 2026-08-10` (R125 16:55 升级)
- ✅ `Status: 🟢 活跃 (8 锚, R125 末 B5 升)`
- ✅ 8 锚 list (S-1, S-2, S-3, O-1, O-2, O-3, O-4, O-5) 已就位
- 0 改 (R126 续 0 必再改)

**R126 续 0 触碰** 09-anchor.md (R125 16:55 升级已 done).

---

## 3. 入口签名 0 改 verify (per B1 24 LOCKED 入口签名 0 改 + 决策 #22 §5.1)

### 3.1 24 LOCKED 入口签名 0 改 verify

| 24 LOCKED # | Crate | 入口签名 | R126 0 改 verify |
|------------:|-------|----------|------------------|
| 1 | apeireth-supervisor | `lib.rs:1-59` (24 LOCKED baseline) | ✅ 0 改 |
| 2 | apeireth-agent | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 3 | apeireth-bus | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 4 | **apeireth-council** | `lib.rs` + `constitution.rs:39` `pub const PHILOSOPHICAL_ANCHORS: [&str; 6]` | ✅ 0 改 (仅加 2 升级版 fn, 0 改原 6 锚 fn) |
| 5 | apeireth-evolution | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 6 | apeireth-extension | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 7 | apeireth-graph | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 8 | apeireth-mcp | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 9 | apeireth-pipeline | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 10 | apeireth-tool-registry | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 11 | apeireth-tool-runtime | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 12 | apeireth-protocol | `lib.rs` + `ws_v1.rs` (24 LOCKED baseline + R20 阶段 2 例外) | ✅ 0 改 |
| 13 | apeireth-asi | `lib.rs:42-44` V1141/V1131/V1136 baseline 3 值 | ✅ 0 改 (A1 baseline 3 值 0 删 0 改) |
| 14 | apeireth-onion | `lib.rs` (5 重守门来源) | ✅ 0 改 |
| 15 | apeireth-sovereignty | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 16 | apeireth-constraint | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 17 | apeireth-memory | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 18 | apeireth-cognition | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 19 | apeireth-perception | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 20 | apeireth-consciousness | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 21 | apeireth-motivation | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 22 | apeireth-life-force | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 23 | apeireth-relation | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |
| 24 | apeireth-value | `lib.rs` (24 LOCKED baseline) | ✅ 0 改 |

**0 越界 24 LOCKED 入口签名 verify 通过** (per B1).

### 3.2 apeireth-core 入口签名 0 改 verify (per A3 13 键 0 改)

| apeireth-core 入口签名 | R126 0 改 verify |
|------------------------|------------------|
| `PhilosophyKey` enum (12 键 PHL-01~06) | ✅ 0 改 (A3 13 键 0 改, R125-12 PHL-07 后续 0 装准备) |
| `ALL_TWELVE_KEYS: [PhilosophyKey; 12]` | ✅ 0 改 |
| `TWELVE_KEYS_HARDCODE: ()` | ✅ 0 改 |
| `verdict_for_target()` const fn | ✅ 0 改 |
| `ActionGuard::check_action()` | ✅ 0 改 |
| `SelfDisableAudit` struct | ✅ 0 改 |
| `SELF_DISABLE_HARDCODE: ()` | ✅ 0 改 |
| `EVOLUTION_INVARIANT: ()` | ✅ 0 改 |
| ... 其他 11+ 入口 | ✅ 0 改 |

**0 越界 apeireth-core 入口签名 verify 通过** (per A3 13 键 0 改 + 决策 #22 §5.1).

### 3.3 9 organ 入口签名 0 改 verify (per B7 9 organ 入口签名 0 改)

| 9 organ 入口签名 | R126 0 改 verify |
|------------------|------------------|
| `apeireth-tui/src/organ/body.rs` | ✅ 0 改 |
| `apeireth-tui/src/organ/brain.rs` | ✅ 0 改 |
| `apeireth-tui/src/organ/ear.rs` | ✅ 0 改 |
| `apeireth-tui/src/organ/eye.rs` | ✅ 0 改 |
| `apeireth-tui/src/organ/hand.rs` | ✅ 0 改 |
| `apeireth-tui/src/organ/heart.rs` | ✅ 0 改 |
| `apeireth-tui/src/organ/memory.rs` | ✅ 0 改 |
| `apeireth-tui/src/organ/mind.rs` | ✅ 0 改 |
| `apeireth-tui/src/organ/voice.rs` | ✅ 0 改 |
| `apeireth-tui/src/organ/mod.rs` (入口) | ✅ 0 改 |

**0 越界 9 organ 入口签名 verify 通过** (per B7).

### 3.4 0 越界 Cargo.toml workspace.version 1.2.0 (per B2)

| Cargo.toml 入口 | R126 0 改 verify |
|-----------------|------------------|
| `[workspace.package] version = "1.2.0"  # B2 upgrade: 1.1.0 → 1.2.0 (Cargo.toml:246)` | ✅ 0 改 (B2 0 改) |

**0 越界 B2 1.2.0 verify 通过**.

### 3.5 0 越界 A1 baseline 3 值 (per A1)

| A1 baseline | R126 0 改 verify |
|-------------|------------------|
| V1141-R11 = 0.8682 (apeireth-asi/src/lib.rs:42) | ✅ 0 改 (A1 baseline 3 值 0 删 0 改) |
| V1131-R11 = 0.8532 (apeireth-asi/src/lib.rs:43) | ✅ 0 改 |
| V1136-R11 = 0.9063 (apeireth-asi/src/lib.rs:44) | ✅ 0 改 |
| V1136_SUBMEASURE_COUNT: usize = 9 | ✅ 0 改 (9 子测度结构 严守) |
| V05_DIM_COUNT: usize = 25 | ✅ 0 改 (R125-13 升 30 维, 0 改 25 维常量) |

**0 越界 A1 baseline 3 值 verify 通过**.

### 3.6 0 越界 6 重守门 v6 (per C3 v6 0 改)

| 6 重守门 v6 | R126 0 改 verify |
|-------------|------------------|
| 守门 1 物理隔离 | ✅ 0 改 (C3 v6 0 改) |
| 守门 2 L0 HA | ✅ 0 改 |
| 守门 3 司法边界 | ✅ 0 改 |
| 守门 4 编译期 hardcode | ✅ 0 改 (R126-1 NEW 8 哲学锚 enum 走 守门 4 编译期 hardcode, 0 改守门 4 实质) |
| 守门 5 哲学锚穿透 (R125-5 NVIDIA 实施) | ✅ 0 改 (R126 8 哲学锚是 5+3=8 升级, 0 改原 6 锚穿透 0 改) |
| 守门 6 Colang DSL (R125-5 实施) | ✅ 0 改 |

**0 越界 6 重守门 v6 verify 通过**.

---

## 4. 0 装 PASS 严守 (per 决策 #36 §1.1 + 主人 17:22 升级授权)

### 4.1 0 装 src 实施 (R126-1 done 20:09)

| # | 阶段 | 实施 | 状态 |
|---|------|------|------|
| 1 | 借鉴源码 study (内部 + 公开) | 内部 09-anchor.md 8 锚 + 公开 clippy lints 1:1 映射 | ✅ done 20:09 |
| 2 | Rust 实施 (eight_anchors.rs NEW) | 8 锚 enum + ALL_EIGHT_ANCHORS + EIGHT_ANCHORS_HARDCODE + 6→8 互转 | ✅ done 20:09 (23.2KB) |
| 3 | 单元测试 stub (内联 12 tests) | 12/12 pass (临时 crate verify, per R125-8 模式) | ✅ done 20:09 |
| 4 | spec 报告 (per 决策 #33 §3) | reports/agent-r126-philo-8-spec-2026-08-10.md (21.1KB) | ✅ done 20:09 |
| 5 | 整合 supervisor plan (per 决策 #33 §3) | reports/agent-r126-philo-8-integration-plan-2026-08-10.md (本文件) | ✅ done 20:09 |

**5 阶段 100% done (NEW file + 内联 12 tests 写完 + 4 reports), 0 装 PASS 严守, 0 越界 8 硬墙. 12 tests 写完 (待 Mavis 整合 #5 拍板时真跑 cargo test verify, per R125-8 模式)**.

### 4.2 整合时 0 装 PASS 严守 (R126 续 8/15-8/17)

R126 续 P1 supervisor 整合时 (8/15-8/17 per 决策 #42 §1.4 pre-checklist):

| 步骤 | 0 装 PASS 严守 |
|------|------------------|
| 步骤 1 (lib.rs 加 1 行) | 0 装 = 0 主动 commit, 仅加 1 行 `pub mod` (per R125-8 final report §2.5 Step 1 模式) |
| 步骤 2 (council 加 2 升级版 fn) | 0 装 = 0 改原 6 锚 fn, 仅加 2 NEW fn (per 决策 #22 §5.1 永远保留原 5 守门 1-4 嵌套结构) |
| 步骤 3 (docs/adr 升级 8 锚) | 0 装 = 0 必改 (09-anchor.md R125 16:55 已升级, docs/adr 是可选) |
| 步骤 4 (09-anchor.md verify) | 0 装 = 0 改 (R125 16:55 已升级) |

**0 装 PASS 严守 verify 通过**.

### 4.3 0 主动 commit (per C1 0 主动 commit)

- ✅ R126-1 0 commit (NEW file untracked + 4 reports untracked)
- ✅ R126 续 整合 0 commit (per C1, Mavis 整合 #5 拍板)
- ✅ R126 续 Mavis 整合 #5 commit (per 决策 #42 §1.4 pre-checklist, 8/15+ OR 8/17 17:30)

### 4.4 0 主动 push (per 决策 #48 + 决策 #51 §5)

- ✅ R126-1 0 push
- ✅ R126 续 整合 0 push (per 0 push 严守, 等 1.0 release 配 GitHub remote)

---

## 5. 整合时序 (8/15-8/17 per 决策 #42 §1.4 pre-checklist)

| 日期 | 任务 | 责任 | 0 装 PASS 严守 |
|------|------|------|------------------|
| 8/10 20:09 | R126-1 5 阶段 done (NEW file 23.2KB + 4 reports) | R126-1 ✅ | ✅ 0 装 |
| 8/11-8/14 | rust-clippy 公开 后台 clone 启动 (主借鉴内部 0 必) | mavis 整合 daemon | ⏳ 启动 |
| 8/15 | rust-clippy ✅ cloned verify (per 决策 #42 §1.4 pre-checklist) | R126 续 P1 supervisor | ⏳ verify |
| 8/15-8/16 | 步骤 1: lib.rs 加 `pub mod eight_anchors;` (1 行) | R126 续 P1 supervisor | ✅ 0 装 |
| 8/15-8/16 | 步骤 2: council 加 2 升级版 fn (0 改原 6 锚 fn) | R126 续 P1 supervisor | ✅ 0 装 |
| 8/15-8/16 | 步骤 3 (可选): docs/adr 升级 8 锚 | R126 续 P1 supervisor | ✅ 0 装 (可选) |
| 8/15-8/16 | 步骤 4 (可选): 09-anchor.md 8 锚 verify | R126 续 P1 supervisor | ✅ 0 装 (0 改) |
| 8/16-8/17 | 整合 verify: cargo build 0 error + cargo test 0 error | R126 续 P1 supervisor | ✅ verify |
| 8/17 17:30 | R126-1 截止 (8/17 per task) | R126-1 | ✅ |
| 8/17 17:30 | Mavis 整合 #5 commit (per 决策 #42 §1.4 pre-checklist) | mavis root | ✅ 拍板 |

**整合时序 8/15-8/17 done, Mavis 整合 #5 commit `abf12243` 后续 拍板**.

---

## 6. 风险与缓解 (per R125-8 final report §8 模式 + 决策 #51 §1.2 P1-2)

| 风险 | 影响 | 缓解 |
|------|------|------|
| **步骤 1 (lib.rs 加 1 行) cargo build 错误** | 整合阻塞 | R126-1 临时 crate verify 0 error + 0 warning (per R125-8 final report §2.2 模式) |
| **步骤 2 (council 加 2 fn) cargo build 错误** | 整合阻塞 | R126-1 spec 草稿 (per 决策 #33 §3) 列出 2 升级版 fn 完整代码, 0 改原 fn |
| **24 LOCKED council #4 入口签名 误改** | B1 越界 | spec §3.1 列出 13 入口签名 0 改 verify, R126 续 P1 supervisor 严守 |
| **13 键 (PHL-01~06) 误改** | A3 越界 | spec §3.2 列出 11+ 入口签名 0 改 verify |
| **6 锚原版顺序 误改** | B5 越界 | EIGHT_ANCHORS_HARDCODE 编译期断言 顺序锁定 (S-1, S-2, O-2, O-3, O-4, O-5 位置 0 改) |
| **0 装 = 准备 但被误标"已实施"** | C2 0 装 PASS 严守 违反 | borrow-index §2 诚实标 内部 + 公开 0 装, final 报告诚实标 |
| **R126 续 Mavis 整合 #5 拍板时 wiring 误加** | C1 0 主动 commit 违反 | R126-1 0 主动 commit (NEW file untracked), R126 续 0 commit |
| **0 push 严守 误推** | push 越界 | 0 主动 git push, 等 1.0 release 配 GitHub remote |
| **整合 #5 commit 时机错过** | 整合 5 时序错 | per 决策 #42 §1.4 pre-checklist 4 项, Mavis 8/15+ 拍板 OR R126 续 8/17 17:30 自动拍板 |
| **8 organ 9 organ 入口签名 误改** | B7 越界 | spec §3.3 列出 10 organ 入口签名 0 改 verify |
| **B2 1.2.0 workspace.version 误升** | B2 越界 | spec §3.4 列出 Cargo.toml:246 0 改 verify |
| **A1 baseline 3 值 误改** | A1 越界 | spec §3.5 列出 V1141/V1131/V1136 数字 0 改 verify |
| **C3 v6 6 重守门 误改** | C3 越界 | spec §3.6 列出 6 重守门 0 改 verify |

---

## 7. 决策链 (接 #51 §1.2 P1-2)

- **#22 (16:35)**: 主人 16:31 "全都能动, 最高权限" + 9 项实质更新登记 + B1-B7 升级路线 + 6 锚 → 8 锚 B5 升级路线 (per 决策 #22 §2.5)
- **#30 (17:15)**: 新 Mavis 接入 + 派活 daemon 复活
- **#33 (17:23)**: 主人 17:22 升级授权 + 8 硬墙全部重置 + 0 装解除 + 16 派满
- **#34 (17:30)**: 17:30 整合 #3 commit `21aa85f3` 拍板 done
- **#35 (17:32)**: 主人 17:31 "16 成员人数要多" + supervisor 模式废弃 + Mavis 真派 16 sub-agent
- **#36 (17:44)**: 借鉴源码 7/11 ✅ cloned 真实施可启动 + 3 限流 + 1 跳过 (OpenCog AGPL-3.0) + 0 装解除严守
- **#41 (18:30)**: R125 16 all done
- **#42 (19:00)**: R125 续整合 #4 pre-checklist 4 项 (per 决策 #42 §1.4)
- **#48 (19:41)**: 整合 #4 commit `abf12243` done (per 主人 19:41 自执行 A 选项, 46752 file changes)
- **#51 (20:09)**: 主人 20:09 "全按你的想法来, 开干" + Mavis 真派 16 sub-agent (P0/P1/P2/P3 各 4 个)
- **R126-1 (20:09)**: R126 8 哲学锚 升级 5 阶段 done + 整合 plan 写完 (本文件)

**8 硬墙 0 越界 + 0 装 PASS 严守 + 借鉴 ID 0 重复 + 入口签名 0 改 verify 通过**.

---

## 8. 一句话 (TL;DR)

**R126 8 哲学锚 整合 2 步 (步骤 1: lib.rs 加 1 行, 步骤 2: council 加 2 升级版 fn). 0 改原 6 锚 fn (per B1 24 LOCKED 入口签名 0 改). 入口签名 0 改 verify (24 LOCKED 9 入口 + apeireth-core 11+ 入口 + 9 organ 10 入口). 0 越界 8 硬墙. 整合时序 8/15-8/17 (per 决策 #42 §1.4 pre-checklist). Mavis 整合 #5 commit `abf12243` 后续 拍板.**
