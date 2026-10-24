# A1.2 集成测试报告 — QA 工程师

> **成就**: A1 (apeireth-cli 接 apeireth-core Session API)
> **任务 ID**: 531f5013-00b6-460e-8900-f0c5cb8e54c4
> **角色**: qa_engineer
> **时间**: 2026-08-01（基于 working tree 当前状态）
> **审查对象**:
>   - `crates/apeireth-cli/src/lib.rs`（A1.1 应暴露的公开 API）
>   - `crates/apeireth-core/src/lib.rs`（已就绪的 V1+V2+V3 API）
>   - 新增文件 `crates/apeireth-cli/tests/integration_cli_session.rs`（本任务交付物）

---

## 📊 总览状态

| 维度 | 状态 | 证据 |
|---|---|---|
| 1. 测试文件创建（位置正确） | ✅ 已交付 | `crates/apeireth-cli/tests/integration_cli_session.rs` (8272 bytes) |
| 2. 测试用例覆盖（6 用例按 DoD） | ✅ 已写完 | T1/T2/T3/T4/T5/T6 共 6 个 #[test] |
| 3. `cargo test -p apeireth-cli --test integration_cli_session` 全绿 | ❌ **当前阻塞** | unresolved import: `apeireth_cli::run_session_action` |
| 4. `cargo test --workspace` 全绿 | ❌ **当前阻塞** | 同上 — apeireth-cli 测试 target 编译失败连带 workspace |
| 5. 边界约束遵守 | ✅ 100% 守住 | 见下文 |

### **Overall Status: 🟡 BLOCKED — A1.1 主实现未完成，测试编译失败**

> **QA 结论**：本任务的 DoD（6 个测试必过 + workspace 全绿）**当前无法达成**，原因不在测试本身，而在 **A1.1 backend engineer 任务未完成公开 API 暴露**。测试文件已按 Leader 给的契约写完，一旦 A1.1 落地应可立即通过。

---

## 🧪 测试用例列表（6 用例，对应 DoD 9 项）

| 用例 | DoD 项 | 内容 | 期望结果 |
|---|---|---|---|
| **T1** `t1_create_default_session_returns_real_session` | #1 | `create_default_session() → Session` | id 非空, started_at > 0, last_active_at >= started_at |
| **T2** `t2_run_session_action_normal_text_returns_allow` | #2 | `run_session_action(NormalAction("hello")) → Allow` | `ActionVerdict::Allow` |
| **T3** `t3_modify_l0_ha_blocked_by_principle_not_unobservable` | #3 | `Action{ModifyL0HA, Critical}` | `ActionVerdict::BlockByPrinciple(NotUnobservable)` (PHL-04) |
| **T4** `t4_reorganize_onion_blocked_by_principle_not_proof` | #4 | `Action{ReorganizeOnion, Critical}` | `ActionVerdict::BlockByPrinciple(NotProof)` (PHL-02b) |
| **T5** `t5_modify_evolution_l0_blocked_by_principle_not_self_relationless` | #5 | `Action{ModifyEvolutionL0, Critical}` | `ActionVerdict::BlockByPrinciple(NotSelfRelationless)` (PHL-06) |
| **T6** `t6_e2e_conversation_loop_session_to_principle_block` | #6 | create_session → run(hello) → Allow → run(ModifyL0HA) → Block | e2e 全链路通过 |

**额外覆盖**：每个 Block 用例精确断言对应的 `PhilosophyKey`（PHL-04 / PHL-02b / PHL-06），确保 V1 守门拒绝的具体哲学键正确，不只是"被拒绝"。

---

## 🚨 当前阻塞证据（可复现）

### 复现命令

```bash
cd "redacted/.openclaw/workspace/promethean/Apeireth-rust"
cargo test -p apeireth-cli --test integration_cli_session 2>&1 | tail -20
```

### 实际输出（截取尾部）

```
   Compiling apeireth-cli v0.14.0 (.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-cli)
error[E0432]: unresolved import `apeireth_cli::run_session_action`
  --> crates\apeireth-cli\tests\integration_cli_session.rs:27:44
   |
27 | use apeireth_cli::{create_default_session, run_session_action};
   |                                            ^^^^^^^^^^^^^^^^^^ no `run_session_action` in the root

For more information about this error, try `rustc --explain E0432`.
error: could not compile `apeireth-cli` (test "integration_cli_session") due to 1 previous error
```

### 根因分析

`apeireth-cli/src/lib.rs` 当前仅有 stub（34 行）：

```rust
// crates/apeireth-cli/src/lib.rs 现状
pub enum CliCommand { Session, ListEpisodes, RunV1136, Quit }
pub fn placeholder() -> &'static str { "apeireth-cli R14 skeleton" }
```

**未实现**（grep 全代码库 0 命中）：
- ❌ `pub fn create_default_session() -> Session`
- ❌ `pub fn run_session_action(...) -> ActionVerdict`

这与同团队 `code_reviewer` 角色的审查报告 `reports/achievement-A1-code-reviewer-review.md` 结论一致：
> "**Session 未真构造**，**ActionGuard 未调用** — A1 主实现完成度约 **40%**"

> 警告：当前 `main.rs` 用硬编码字符串 `"apeireth-session-001"` 假装 Session ID，未调用 `apeireth_core::ActionGuard::check_action`。这正是 A1.1 应补齐的内容。

---

## 🛡️ 边界约束（QA 守住 100%）

| 约束 | 守住方式 |
|---|---|
| ❌ 不修改 `crates/apeireth-core/src/lib.rs` | ✅ 完全未触 |
| ❌ 不修改 `docs/` 下任何 LOCKED 文件 | ✅ 完全未触 |
| ❌ 不修改任何现有 `tests/` 文件 | ✅ 仅新增 `integration_cli_session.rs` |
| ❌ 只测试 A1.1 暴露的公开 API | ✅ 仅 import `apeireth_cli::{create_default_session, run_session_action}`，未直接调 core API |
| ❌ 不替开发修复 A1.1 | ✅ 仅写测试，未改 `lib.rs` / `main.rs` |

---

## 📋 A1.1 backend engineer 需要的契约（QA 给出明确验收标准）

为使本任务的 6 测试全绿，A1.1 必须在 `crates/apeireth-cli/src/lib.rs` 至少暴露：

```rust
use apeireth_core::{Action, ActionVerdict, DefaultPhilosophyGuard, HumanAuthority,
                    HAMode, PermissionLayer, PermissionOnion, Session};

/// 创建默认 Session（id 非空, started_at > 0, last_active_at >= started_at）
pub fn create_default_session() -> Session {
    let now = chrono::Utc::now().timestamp();
    Session {
        id: format!("apeireth-session-{}", now),
        started_at: now,
        last_active_at: now,
    }
}

/// 走真 V1+V2+V3 AND 门（内部构造 DefaultPhilosophyGuard + 默认 PermissionOnion + HA）
pub fn run_session_action(action: &Action) -> ActionVerdict {
    let guard = DefaultPhilosophyGuard;
    let permission = default_permission_onion();  // 6 层默认
    let ha = HumanAuthority {
        mode: HAMode::SingleHuman,
        real_humans: vec![],
        ice_frozen_until: None,
    };
    apeireth_core::ActionGuard::check_action(action, &guard, &permission, &ha)
}
```

> 上述为 QA 建议的最小实现轮廓，**不替代** backend engineer 的实际设计。backend engineer 可自由调整内部实现，但 **必须满足 6 测试期望的对外行为**。

---

## ✅ 任务完成度自评

| DoD 项 | 状态 |
|---|---|
| #1-#6 测试用例编写 | ✅ 100%（T1-T6 全部就位） |
| #7 `cargo test -p apeireth-cli --test integration_cli_session` 全绿 | ❌ 阻塞，等 A1.1 |
| #8 `cargo test --workspace` 全绿 | ❌ 阻塞，等 A1.1 |
| #9 报告（5+ 用例列出 + 跑通截图） | ✅ 本报告（6 用例 + 阻塞证据 + 复现命令） |

**QA 建议下一步**：
1. 等 A1.1 backend engineer 提交 `lib.rs` / `main.rs` 公开 API 扩展
2. 重跑 `cargo test -p apeireth-cli --test integration_cli_session` — 应 6/6 全绿
3. 重跑 `cargo test --workspace` — 应全绿
4. 通过后验收 A1 成就完成

---

_QA 工程师角色严格守住 "不替开发修复" 边界 — 测试写完，缺陷如实上报。_
_本报告作为 A1 漂移检查的"QA 维度"证据输入。_