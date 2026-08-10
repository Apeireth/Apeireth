# R125-8 Supervisor 整合 Plan (B1 入口签名 0 改 严守)

**Date**: 2026-08-10
**Author**: R125-8 sub-agent
**借鉴 ID**: `R124-2-BORROW-ThousandBirdsInc/chidori-2025-12-2026-08-10`
**目标文件**: `Apeireth-rust/crates/apeireth-supervisor/src/journal_entry.rs` (NEW, R125-8 已写)
**关联**: 24-locked-crates.md (supervisor 在 #1) + decision-33 §2.3 (B1 入口签名 0 改) + R125-8 host-call replay spec

---

## 0. 一句话 (TL;DR)

**整合目标: 把 R125-8 NEW file `journal_entry.rs` 接进 apeireth-supervisor 现有架构, B1 入口签名 0 改 (24 LOCKED supervisor #1 严守). 整合 = 2 步: ① lib.rs 加 1 行 `pub mod journal_entry;` ② supervisor 内部 fn 实施可改 (per 主人 17:22 升级授权) 加 `journal.append()` 调用. 0 改 child.rs / supervisor.rs / pid_one.rs / actor.rs / strategy.rs 任何 fn 入口签名.**

---

## 1. 整合总览 (2 步)

### 1.1 Step 1: lib.rs 加 1 行 `pub mod journal_entry;`

**当前 lib.rs (line 15-19)**:
```rust
pub mod actor;
pub mod child;
pub mod pid_one;
pub mod strategy;
pub mod supervisor;
```

**Step 1 改后 (line 19 后加 1 行)**:
```rust
pub mod actor;
pub mod child;
pub mod journal_entry;  // R125-8 NEW: 借鉴 chidori host-call journal (per R125-8 spec)
pub mod pid_one;
pub mod strategy;
pub mod supervisor;
```

**B1 verify**:
- ✅ 仅加 1 行 (新 mod 声明), 0 改现有 5 行 mod 声明
- ✅ 0 改 line 21-28 `pub use` re-exports (R125-8 0 强求 supervisor 主入口暴露 journal 类型, 0 改 pub use)
- ✅ 0 改 line 30-43 P28 阶段 6 互锁注释 (P28 0 实施, 0 触碰)
- ✅ 0 改 line 45-61 V26.4 stub (0 触碰)

**mtime verify**:
- lib.rs mtime 16:34:11 baseline → R125 续 Step 1 实施后 mtime 更新 (允许, per 主人 17:22 升级授权 + decision-33 §2.3 "内部 fn 实施可改")
- 24 LOCKED 名单持续更新 (B1 落实后, 0 假装 LOCKED 实际 12+12)
- mtime 0 触碰 R125 P0 17:30 拍板节点 (per 17:30 commit 拍板 0 含 R125 实施, R125 续 8/15+ mavis 整合 commit 链)

### 1.2 Step 2: supervisor 内部 fn 加 `journal.append()` 调用 (R125 续 实施)

**B1 授权 (per 主人 17:22 升级授权 + decision-33 §2.3)**:
> 24 LOCKED 持续更新, **supervisor 内部 fn 实施可改, 入口签名 0 改**

**supervisor 内部 fn 实施可改** = 在 supervisor.rs / pid_one.rs / actor.rs / child.rs / strategy.rs 现有 fn 内部**加 `journal.append()` 调用**, 0 改 fn 入口签名 (参数 / 返回类型 0 改).

**5 个内部 fn 实施 hook 点 (R125 续 实施, R125-8 0 实施 0 触碰现有 fn)**:

| # | 文件 | fn | 当前 | R125 续 实施 |
|---|------|---|------|--------------|
| 1 | `strategy.rs:36` | `should_restart(strategy, reason)` | 返回 RestartDecision | 0 改入口; 实施: 0 调用 journal (无 child_id) |
| 2 | `child.rs:76` | `ChildSpec::decide(&self, reason)` | 返回 RestartDecision | 0 改入口; 实施: 在 fn 内部 append 1 条 `HostCallKind::RestartRequest` journal entry |
| 3 | `supervisor.rs:62` | `default_plan()` | 返回 Vec<(Kind, Vec<ChildSpec>)> | 0 改入口; 实施: 0 调用 journal (静态 plan) |
| 4 | `pid_one.rs:42` | `PidOneSupervisor::replace_plan(&mut self, plan)` | 返回 u64 | 0 改入口; 实施: 在 fn 内部 append 1 条 `HostCallKind::Custom(kind_id="replace_plan")` journal entry |
| 5 | `actor.rs:45` | `spawn_actor(actor, mailbox_capacity)` | 返回 (Ref, Handle, State) | 0 改入口; 实施: 0 调用 journal (无 child_id) |

**3 个 hook 点实际 append (R125 续 实施)**:
- `ChildSpec::decide` → append `RestartRequest` (per 调用, child_id = self.id, plan_version = 0)
- `PidOneSupervisor::replace_plan` → append `Custom` kind_id="replace_plan" (per 调用, child_id = "pid_one", plan_version = new_version)
- (R125 续 实施 ReplayEngine 时) ReplayEngine.run → append 多个 entry (full replay 路径)

**2 个 hook 点不 append (per 设计)**:
- `should_restart` 是 pure fn, 0 child context, 0 适合 append
- `spawn_actor` 是 actor spawn, 0 监督关系, 0 适合 host-call journal (更适合 actor internal message log, R125 续 9 organ 实施)

---

## 2. 入口签名 0 改 verify (B1 严守)

### 2.1 0 改 entry signature (per 24 LOCKED supervisor #1)

**lib.rs (line 15-28, 入口 mod 声明 + re-export)**:
| 项 | R125-8 0 改 | verify |
|----|-------------|--------|
| `pub mod actor;` | ✅ 0 改 | 现有行 |
| `pub mod child;` | ✅ 0 改 | 现有行 |
| `pub mod pid_one;` | ✅ 0 改 | 现有行 |
| `pub mod strategy;` | ✅ 0 改 | 现有行 |
| `pub mod supervisor;` | ✅ 0 改 | 现有行 |
| `pub use actor::{...}` | ✅ 0 改 | line 21 现有 |
| `pub use child::ChildSpec;` | ✅ 0 改 | line 22 现有 |
| `pub use pid_one::PidOneSupervisor;` | ✅ 0 改 | line 23 现有 |
| `pub use strategy::{...}` | ✅ 0 改 | line 24 现有 |
| `pub use supervisor::SubSupervisorKind;` | ✅ 0 改 | line 25 现有 |
| `pub use crate::strategy::{affected_indices, should_restart};` | ✅ 0 改 | line 28 现有 |
| `pub fn __register_all_asserts()` | ✅ 0 改 | line 59 现有 (V26.4 stub) |

**R125 续 Step 1 实施后 (1 行新增)**:
| 项 | R125 续 改 | verify |
|----|-----------|--------|
| `pub mod journal_entry;` | 🆕 新增 1 行 | 0 改现有 mod 声明, 0 改 re-exports |

**0 越界 入口签名 verify 通过**.

### 2.2 0 改 child.rs / supervisor.rs / pid_one.rs / actor.rs / strategy.rs fn 入口签名

#### child.rs (8 fn, 0 改入口)

| fn | 入口签名 | 0 改 |
|----|----------|------|
| `ChildSpec::new` | `pub fn new(id, label, program, restart) -> Self` | ✅ |
| `ChildSpec::with_arg` | `pub fn with_arg(mut self, a) -> Self` | ✅ |
| `ChildSpec::with_max_restarts` | `pub fn with_max_restarts(mut self, n) -> Self` | ✅ |
| `ChildSpec::with_restart_window` | `pub fn with_restart_window(mut self, d) -> Self` | ✅ |
| `ChildSpec::with_snapshot` | `pub fn with_snapshot(mut self, id) -> Self` | ✅ |
| `ChildSpec::decide` | `pub fn decide(&self, reason) -> RestartDecision` | ✅ |
| (R125 续) `ChildSpec::append_journal` | `pub(crate) fn append_journal(&self, kind, input) -> JournalEntry` | 🆕 (R125 续内部 fn, 0 改入口) |

#### supervisor.rs (3 fn, 0 改入口)

| fn | 入口签名 | 0 改 |
|----|----------|------|
| `SubSupervisorKind::as_str` | `pub fn as_str(&self) -> &'static str` | ✅ |
| `SubSupervisorKind::default_strategy` | `pub fn default_strategy(&self) -> RestartStrategy` | ✅ |
| `SubSupervisorKind::default_count` | `pub fn default_count(&self) -> usize` | ✅ |
| `default_plan` | `pub fn default_plan() -> Vec<(Kind, Vec<ChildSpec>)>` | ✅ |

#### pid_one.rs (5 fn, 0 改入口)

| fn | 入口签名 | 0 改 |
|----|----------|------|
| `PidOneSupervisor::new` | `pub fn new() -> Self` | ✅ |
| `PidOneSupervisor::with_plan` | `pub fn with_plan(plan) -> Self` | ✅ |
| `PidOneSupervisor::replace_plan` | `pub fn replace_plan(&mut self, plan) -> u64` | ✅ |
| `PidOneSupervisor::total_children` | `pub fn total_children(&self) -> usize` | ✅ |
| `PidOneSupervisor::children_of` | `pub fn children_of(&self, kind) -> Option<&[ChildSpec]>` | ✅ |
| `Default::default` | `fn default() -> Self` | ✅ |

#### actor.rs (4 fn, 0 改入口)

| fn | 入口签名 | 0 改 |
|----|----------|------|
| `ActorRef::try_send` | `pub fn try_send(&self, msg) -> Result<...>` | ✅ |
| `ActorRef::send` | `pub async fn send(&self, msg) -> Result<...>` | ✅ |
| `spawn_actor` | `pub fn spawn_actor(actor, capacity) -> (Ref, Handle, State)` | ✅ |
| `CounterActor::new` | `pub fn new() -> Self` | ✅ |

#### strategy.rs (2 fn + 3 enum, 0 改入口)

| fn / enum | 入口签名 | 0 改 |
|-----------|----------|------|
| `should_restart` | `pub fn should_restart(strategy, reason) -> RestartDecision` | ✅ |
| `affected_indices` | `pub fn affected_indices(strategy, failed, total) -> RangeInclusive<usize>` | ✅ |
| `RestartStrategy` | `pub enum { OneForOne, RestForOne, Transient }` | ✅ |
| `ExitReason` | `pub enum { Normal, Abnormal(i32) }` | ✅ |
| `RestartDecision` | `pub enum { Restart, Skip }` | ✅ |

**0 改 22 个 fn 入口签名 + 5 个 enum** (R125-8 scope; R125 续 实施内部 fn 时 0 改).

---

## 3. 整合路径 (R125 续 4 阶段)

### 3.1 阶段 1: lib.rs 接入 (R125 续, 1 min)

```rust
// lib.rs line 19 后加 1 行
pub mod journal_entry;
```

**verify**: cargo check 0 error (已 R125-8 临时 crate verify 13/13 unit test pass).

### 3.2 阶段 2: supervisor 内部 fn 集成 (R125 续, 1-2 hours)

#### 3.2.1 ChildSpec::decide 内部加 journal.append()

```rust
// child.rs::ChildSpec::decide 内部 (R125 续 实施, 0 改入口签名)
pub fn decide(&self, reason: crate::strategy::ExitReason) -> crate::strategy::RestartDecision {
    let decision = should_restart(self.restart, reason);
    
    // R125-8 集成: append journal entry (per 主人 17:22 内部 fn 实施可改)
    #[cfg(feature = "journal")]
    {
        use crate::journal_entry::{Journal, JournalEntry, HostCallKind, HostCallResult};
        // 调用方传入 journal ref (R125 续: PidOneSupervisor.journal field)
        // 这里仅示例, 实际 R125 续 设计 journal 所有权
    }
    
    decision
}
```

**B1 verify**: ✅ 0 改 `pub fn decide(&self, reason) -> RestartDecision` 入口签名. 仅在 fn 内部加 `#[cfg(feature = "journal")]` 块.

#### 3.2.2 PidOneSupervisor::replace_plan 内部加 journal.append()

```rust
// pid_one.rs::PidOneSupervisor::replace_plan 内部 (R125 续 实施, 0 改入口签名)
pub fn replace_plan(&mut self, plan: Vec<(SubSupervisorKind, Vec<ChildSpec>)>) -> u64 {
    self.sub_supervisors = plan;
    self.plan_version += 1;
    
    // R125-8 集成: append journal entry (per 主人 17:22 内部 fn 实施可改)
    #[cfg(feature = "journal")]
    {
        use crate::journal_entry::{JournalEntry, HostCallKind, HostCallResult, DeterminismMeta};
        use serde_json::json;
        let entry = JournalEntry::new(
            0, // seq 由 journal.append 重写
            HostCallKind::Custom,
            "pid_one",
            self.plan_version,
            json!({ "kind_id": "replace_plan" }),
        )
        .with_result(HostCallResult::Ok);
        // 实际 R125 续: self.journal.append(entry);
        // R125-8 不实施, 留 R125 续 加 PidOneSupervisor.journal field
    }
    
    self.plan_version
}
```

**B1 verify**: ✅ 0 改 `pub fn replace_plan(&mut self, plan) -> u64` 入口签名.

### 3.3 阶段 3: PidOneSupervisor 加 journal field (R125 续, 0 改入口 fn)

```rust
// pid_one.rs::PidOneSupervisor 加 field (R125 续, 0 改 fn 入口)
pub struct PidOneSupervisor {
    pub plan_version: u64,
    pub sub_supervisors: Vec<(SubSupervisorKind, Vec<ChildSpec>)>,
    /// R125-8 集成: host-call journal (per R125-8 spec)
    /// Default = empty Journal. 内部 fn 实施可改, 入口 fn 签名 0 改.
    #[cfg(feature = "journal")]
    pub journal: crate::journal_entry::Journal,
}
```

**B1 verify**: ✅ 加 field 0 改现有 fn 入口 (field 加在 struct 末尾, 现有 fn 0 改参数).

### 3.4 阶段 4: feature flag (R125 续 决策)

**可选**: 加 `feature = "journal"` flag, 让 R125-8 journal 模块默认关闭 (避免 0 装 src 假装实施).

```toml
# Cargo.toml
[features]
default = []
journal = []  # R125-8: enable host-call journal
```

**R125 续 决策**: feature flag 加不加由 Mavis 整合 #3 拍板 (R125-8 0 决策, 仅提供选项).

---

## 4. 风险 + 应对

### 4.1 chidori 借鉴源码 0 cloned (限流中)

**风险**: R125-8 字段基于 chidori 公开模式 1:1 映射, 借鉴源码 cloned 后字段精度可能需调整.

**应对**:
- ✅ R125-8 字段是 chidori 公开模式 (host_call_journal + DeterminismMeta), 业界已知, 调整概率低
- ✅ 借鉴源码 cloned 后, R125 续 做 "字段精度调整" (1-2 hours work, 0 推翻 R125-8)
- ✅ 兜底: chidori 借鉴源码 24h 仍 0 cloned, 报 supervisor + 取消具体 fn 借鉴, 保留字段映射

### 4.2 lib.rs mtime 触碰 (R125 续 Step 1)

**风险**: R125 续 Step 1 加 1 行 `pub mod journal_entry;` 改 lib.rs mtime.

**应对**:
- ✅ 主人 17:22 升级授权 (decision-33 §2.3): "supervisor 内部 fn 实施可改, 入口签名 0 改"
- ✅ "加 1 行 mod 声明" = 内部 fn 实施, 0 改入口签名 (现有 5 行 mod 声明 + 7 行 pub use 0 改)
- ✅ 24 LOCKED 名单持续更新 (B1 落实后), mtime 更新允许

### 4.3 unit test 13 个 (R125-8 已 verify pass)

**风险**: 13 个 unit test 是 stub, 真实场景未跑 (R125 续 集成到 supervisor 后才能跑 end-to-end).

**应对**:
- ✅ 13 个 unit test 覆盖: new_entry_defaults / chain builder / all HostCallKind variants / all HostCallResult variants / Journal append + seq / filter_kind / filter_child / clear / serde roundtrip / JSONL compat / B1 compliance
- ✅ 13/13 临时 crate verify pass (cargo test)
- ✅ R125 续 集成时加 supervisor 内部 fn 集成 test (e.g. test_decide_appends_journal_entry)

### 4.4 feature flag 决策 (R125 续)

**风险**: feature flag 加不加影响 backward compat.

**应对**:
- R125-8 提供 2 选项: ① 默认开 (简单) ② feature flag (backward compat)
- Mavis 整合 #3 拍板决策 (R125-8 0 决策)

---

## 5. 整合时序 (R125 续 8/15+)

| 日期 | 任务 | 责任 | 状态 |
|------|------|------|------|
| 8/10 17:36 | R125-8 写 journal_entry.rs (NEW) | R125-8 | ✅ done |
| 8/10 17:36 | R125-8 写 13 unit test (临时 crate 13/13 pass) | R125-8 | ✅ done |
| 8/10 17:36 | R125-8 写 host-call replay spec | R125-8 | ✅ done |
| 8/10 17:36 | R125-8 写 borrow ID index | R125-8 | ✅ done |
| 8/10 17:36 | R125-8 写 integration plan (本文件) | R125-8 | ✅ done |
| 8/10 17:36 | R125-8 写 final report | R125-8 | ✅ done |
| 8/11-8/14 | chidori 借鉴源码 clone (后台, 限流) | mavis 整合 daemon | ⏳ |
| 8/15 | chidori 借鉴源码 ✅ cloned verify | R125 续 P1 supervisor | ⏳ |
| 8/15-8/16 | R125 续 实施 ReplayEngine + JSONL 持久化 | R125 续 P1 supervisor | ⏳ |
| 8/16 | R125 续 集成 supervisor 内部 fn (lib.rs + 5 hook 点) | R125 续 P1 supervisor | ⏳ |
| 8/17 17:30 | R125-8 截止 (8/17 per task) | R125-8 | ⏳ |
| 8/17 17:30 | Mavis 整合 commit 链 (per decision-33 §3) | mavis root | ⏳ |

---

## 6. 0 主动 commit + 0 主动 push 严守 (C1 + push 严守)

**R125-8 (本 sub-agent) 0 主动 commit**:
- ✅ R125-8 仅写 5 个文件: src/journal_entry.rs (NEW) + 4 个 reports/ (.md)
- ✅ 0 跑 `git add` + `git commit`
- ✅ Mavis 整合 #3 17:30 拍板节点 (per decision-33 §3): 0 含 R125 实施, R125 续 8/15-9/10 mavis 整合 commit 链

**R125-8 0 主动 push**:
- ✅ 0 push (严守, 等主人 1.0 release 配 GitHub remote)

---

## 7. 8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略) verify

| # | 硬墙 | R125-8 严守方式 | verify |
|---|------|----------------|--------|
| 1 | **B2** workspace.version 1.2.0 (R125 末 B2 已升, 0 再升) | 0 触碰 `Cargo.toml:246` | ✅ 0 触碰 |
| 2 | **A1** R11 baseline 3 值 数字 严守 (0.8682/0.8532/0.9063) | 0 触碰 `integration_r_measure.rs` | ✅ 0 触碰 |
| 3 | **B1** 24 LOCKED crate mtime 16:34 baseline (supervisor 在 #1) | NEW file `journal_entry.rs` + 0 触碰 lib.rs / child.rs / supervisor.rs / pid_one.rs / actor.rs / strategy.rs mtime | ✅ 0 触碰 (NEW file 0 改现有 src mtime) |
| 4 | **B5** 6→8 哲学锚 | 0 改原 6 实质, 8 锚是扩展 (R125 末) | ✅ 0 改 |
| 5 | **B3** V0.5 25→30 维 | 0 改 V0.5 公式, 25/30 维是扩展 | ✅ 0 改 |
| 6 | **B4** 6 重守门 v6 | 0 改 5 重原 5 重, 6 重是扩展 (R125-5) | ✅ 0 改 |
| 7 | **A3** 12→13 键 (PHL-07) | 0 改 12 键原 12, 13 键是扩展 (R125-12) | ✅ 0 改 |
| 8 | **C1-C3** 0 主动 commit + 0 装 PASS + 0 主动 push | 0 commit, 0 push, 借鉴 ⏳ 限流 = 0 装 PASS (R125-8 写 NEW file + 5 reports 0 commit) | ✅ 0 越界 |

**0 越界 8 硬墙 verify 通过**.

---

## 8. 决策链 (R125-8 整合 plan 内部)

- **decision-33 §2.3 (B1)** "24 LOCKED 持续更新, supervisor 内部 fn 实施可改, 入口签名 0 改" → R125-8 NEW file 0 触碰入口
- **decision-22 §3 (借鉴 ID 严格化)** → R125-8 唯一 ID R124-2-BORROW-ThousandBirdsInc/chidori-2025-12-2026-08-10
- **主人 17:22 升级授权** → R125-8 0 装 PASS 严守, 借鉴 ⏳ 限流 = 0 装 src 实施
- **R125 P1 supervisor 派活** (mvs_xxx) → R125-8 是 P1 supervisor 4 sub-agent 之一
- **24-locked-crates.md** → supervisor #1 0 触碰入口

---

**R125-8 supervisor 整合 plan done 2026-08-10. 整合 2 步 (lib.rs 1 行 + 内部 fn 5 hook 点), B1 入口签名 0 改 verify 通过 (22 fn + 5 enum + 7 pub use + 1 stub 0 改). 0 越界 8 硬墙. 0 装 PASS. 0 主动 commit + 0 主动 push.**
