# A1.1 成就报告 — 后端工程师：apeireth-cli session 接 core Session API

```
[Document-Meta]
Document: achievement-A1-backend-engineer-apeireth-cli-session.md
Achievement: A1.1
Role: backend_engineer
Status: 🟢 完成
Last-Modified: 2026-08-01
R-Cycle: R14
```

---

## 🎯 任务

**A1.1**：将 `apeireth-cli` 的 `session` 子命令从 `println!` 静态字符串占位升级为真正调用 `apeireth-core` Session API。

来源：`START-CONSTRUCTION.md` §A1 第 1 天任务 + 不修改承诺 7 项。

---

## ✅ DoD 逐项验证

| DoD | 状态 | 证据 |
|---|---|---|
| 1. `main.rs` `dispatch(Session)` 真实构造 Session / HA / PermissionOnion / DefaultPhilosophyGuard；欢迎信息从 Session 字段动态生成 | ✅ | `crates/apeireth-cli/src/main.rs` `run_session()` 调用 `create_default_session()` / `build_default_human_authority()` / `build_default_permission_onion()` + `welcome_message()` |
| 2. stdin 对话循环 + ActionGuard::check_action + 打印 ActionVerdict | ✅ | `main.rs` 读行 → `:quit` / EOF 退出 → `handle_input_line(line, &session)` → `run_session_action(&Action)` → `describe_verdict` |
| 3. `lib.rs` 加 CLI 工具函数 `pub fn create_default_session() -> Session` | ✅ | `lib.rs` 新增 9 个 pub fn：`create_default_session` / `build_default_permission_onion` / `build_default_human_authority` / `welcome_message` / `classify_risk` / `build_action_from_input` / `describe_verdict` / `handle_input_line` / `run_session_action`（最后两个对齐 ADR 0002 + A1.2 集成测试契约） |
| 4. 单元测试 5+ 条 | ✅ | **12 条** lib 单元测试 + **6 条** qa 集成测试 = **18 条**（lib: placeholder_backcompat / session_has_valid_fields / session_ids_unique / ha_single_mode / onion_six_layers / classify_risk_levels / handle_input_normal_allows / handle_input_l0_blocked_by_principle / handle_input_line_e2e / welcome_message_uses_session_fields_dynamic / describe_verdict_strings / action_ids_increment；integration: T1 session 字段 / T2 normal Allow / T3 L0→PHL-04 / T4 ReorganizeOnion→PHL-02b / T5 ModifyEvolutionL0→PHL-06 / T6 e2e） |
| 5. `cargo build` + `cargo test` + `cargo clippy` 全绿 | ✅ | `cargo test --workspace` → **33 passed; 0 failed**（其中 apeireth-cli = 12 lib + 6 integration = 18 条）；`cargo clippy -p apeireth-cli --all-targets` → **0 warning** in apeireth-cli（其他 crate 121 warnings 全部预存于 apeireth-core/philosophy，按不修改承诺 7 项不在本 DoD 范围） |
| 6. `cargo run -p apeireth-cli -- session` 真的跑（输出真实 Session 字段） | ✅ | 实测输出：`Session ID: sess-1785583953-1`、`started_at: 2026-08-01T11:32:33+00:00`（动态生成，非硬编码） |
| 7. 100% 守住不修改承诺 7 项 | ✅ | 本次改动**仅触及** `crates/apeireth-cli/src/{main.rs, lib.rs}` + `reports/achievement-A1-backend-engineer-apeireth-cli-session.md`。未触碰 `docs/` 任何 LOCKED 文件、未触碰 `apeireth-core/src/lib.rs`、未触碰 `stage1-5` 设计文档、未触碰 v2/v4/v4.1/R11 baseline/v1-v5 历史链 |
| 8. 写报告 | ✅ | 即本文档 |

---

## 📊 实测输出（`cargo run -p apeireth-cli -- session`）

喂入 stdin：
```
hello world
write to file
delete something
modify L0 HA please
:quit
```

实际输出（去掉 clippy 噪音后）：
```
🚀 Apeireth Session 启动
  Session ID    : sess-1785583782-1
  started_at   : 2026-08-01T11:29:42+00:00
  last_active  : 2026-08-01T11:29:42+00:00
  HA mode      : SingleHuman (1 humans)
  PermissionOnion: L0=L0 HA 核心 / L5=L5 核武器级
  守门          : V1+V2+V3 AND 门 (双洋葱 + HA)
  ✅ session 已启动 (A1 第 1 天任务完成)

📥 输入一行文本 → 自动构造 Action → 走 V1+V2+V3 AND 门
   输入 ":quit" / ":exit" 或 Ctrl-D / Ctrl-Z 退出

>   ✅ Allow (V1+V2+V3 全通过)             ← hello world (Low, NormalAction)
>   ✅ Allow (V1+V2+V3 全通过)             ← write to file (Medium, NormalAction)
>   ✅ Allow (V1+V2+V3 全通过)             ← delete something (High, NormalAction)
>   ❌ BlockByPrinciple(PHL-04 不假装不可观测)  ← modify L0 HA please → ModifyL0HA → V1 拦
> 👋 退出 session (sess-1785583782-1)
```

**关键证据**：
- Session ID 是真实 `sess-<unix>-<hex_counter>` 格式（不再是 `apeireth-session-001`）
- 时间戳是 `chrono::Utc::now()` 真实捕获
- HA 模式 / PermissionOnion 名字都是真实构造
- V1+V2+V3 AND 门**真的被调用**：
  - 第 1-3 行走 V1 Allow → V2 → V3 → Allow
  - 第 4 行因 `modify L0 HA` 触发 `classify_risk → Critical` + `lower.contains("l0")` → `ActionTarget::ModifyL0HA` → V1 (`DefaultPhilosophyGuard::check_philosophy`) 第一道关就返回 `PhilosophyVerdict::Block(NotUnobservable)` → `ActionVerdict::BlockByPrinciple(PHL-04)`

---

## 🦴 代码改动摘要

### `crates/apeireth-cli/src/lib.rs`（34 → ~310 行）

**新增 pub API（9 个，符合 ADR 0002 公开 API 表面 + A1.2 集成测试契约）**：

| 函数 | 用途 | 对齐 |
|---|---|---|
| `create_default_session()` | 真实 Session（id=`sess-<unix>-<hex_counter>`，started_at/last_active_at=now） | ADR 0002 阶段 1 最小集 ✅ |
| `run_session_action(action: &Action) -> ActionVerdict` | 对一个 Action 跑 V1+V2+V3 AND 门（内部封装 DefaultPhilosophyGuard + 默认 onion + 默认 HA） | ADR 0002 阶段 1 最小集 ✅ + A1.2 集成测试契约 ✅ |
| `build_default_permission_onion()` | 6 层 L0-L5 全部 requires_ha=true | lib.rs 内部辅助 |
| `build_default_human_authority()` | SingleHuman + 1 个占位主人（WindowsHello 认证） | lib.rs 内部辅助 |
| `welcome_message(s, ha, po)` | 动态欢迎（基于 Session 真实字段） | main.rs 用 |
| `classify_risk(text)` | 关键词启发式（Critical/High/Medium/Low/Info） | 内部辅助 |
| `build_action_from_input(line, session)` | 输入 → Action（普通文本 NormalAction；含 L0 关键词升级到 ModifyL0HA） | 内部辅助 |
| `describe_verdict(v)` | ActionVerdict → 单行可读字符串 | main.rs 用 |
| `handle_input_line(line, session)` | 一行输入 → Action → `run_session_action` | main.rs stdin 循环用 |

**保留**：原 `CliCommand` 枚举 + `placeholder()` 向后兼容（不破坏旧测试）

**静态原子计数器**：`static ID_COUNTER: AtomicU64` 用于 session id/action id 唯一性（不引入 uuid crate）

**单元测试 12 条**（超过 DoD 要求的 5+）：
1. `test_placeholder_ok_backcompat`
2. `test_create_default_session_has_valid_fields`
3. `test_session_ids_are_unique`
4. `test_default_human_authority_single_mode`
5. `test_default_permission_onion_has_six_layers`
6. `test_classify_risk_levels`
7. `test_handle_input_normal_allows`（NormalAction + Low → Allow）
8. `test_handle_input_l0_blocked_by_principle`（ModifyL0HA → BlockByPrinciple(PHL-04)）
9. `test_handle_input_line_e2e`（session → Allow → L0 攻击 → Block 端到端）
10. `test_welcome_message_uses_session_fields_dynamic`（欢迎信息含真实 session.id）
11. `test_describe_verdict_strings`（4 种 verdict 变体字符串映射）
12. `test_build_action_from_input_id_increments`（action id 单调）

### `crates/apeireth-cli/src/main.rs`（100 → 138 行）

**`run_session()` 重写**：原来 `dispatch(Session)` 是 4 行 println!；现在是真实构造 + stdin 循环。

```rust
fn run_session() -> ExitCode {
    let session = create_default_session();
    let ha = build_default_human_authority();
    let po = build_default_permission_onion();
    // welcome 打印 + stdin 循环（:quit / EOF 退出）
    // 每行 → handle_input_line(line, &session) → describe_verdict
}
```

`CliCommand::ListEpisodes` / `RunV1136` / `Quit` 分支**保持不动**（不在本 DoD 范围）。

**main.rs 严格遵守 ADR 0002 绑定规则**：`use apeireth_cli::*` 全部封装；**不直接 `use apeireth_core::*`**（DefaultPhilosophyGuard 已封装在 `handle_input_line` → `run_session_action` 内部）。

---

## 🤝 与其他角色产物的交叉对齐

| 角色 | 产物 | 对齐状态 |
|---|---|---|
| **architect** | `docs/adr/0002-cli-session-api-binding.md`（Accepted 🟢） | ✅ `create_default_session` + `run_session_action(&Action) -> ActionVerdict` 公开 API 表面已对齐；main.rs 不直接 use apeireth_core::* |
| **qa_engineer** | `crates/apeireth-cli/tests/integration_cli_session.rs`（6 个用例）| ✅ 全部 6 个 integration test 通过：T1 session 真实字段 / T2 normal Allow / T3 L0→PHL-04 / T4 ReorganizeOnion→PHL-02b / T5 ModifyEvolutionL0→PHL-06 / T6 e2e 端到端 |
| **code_reviewer** | `reports/achievement-A1-code-reviewer-review.md` | ✅ 修复了其指出的"Session 未真构造 / ActionGuard 未调用 / 仅 1 个测试"问题（本 PR 前置 placeholder 版本的所有风险已关闭）|

---

## 🛡️ 不修改承诺 7 项 100% 守住

| 类别 | 触碰？ |
|---|---|
| `docs/stage1/` / `stage2/` / `stage3/` 54 份 LOCKED 设计文档 | ❌ 未触碰 |
| `docs/stage4/stage4-*.md` 主文档（1492 行 + 6ca80776 LOCKED）| ❌ 未触碰 |
| `docs/stage5/stage5-construction-document.md`（631 行）| ❌ 未触碰 |
| v2 / v4 / v4.1 LOCKED | ❌ 未触碰 |
| v6 修正（4 重守门 + 权限发放 + E 层修改路径）| ❌ 未触碰 |
| R11 baseline 三值（V1141=0.8682 / V1131=0.8532 / V1136=0.9063）| ❌ 未触碰 |
| v1 → v5 历史链 | ❌ 未触碰 |
| `crates/apeireth-core/src/lib.rs`（核心 API） | ❌ 未触碰（按任务硬约束）|

**改动文件清单**：
- ✅ `crates/apeireth-cli/src/lib.rs`（重写 + 扩展）
- ✅ `crates/apeireth-cli/src/main.rs`（重写 dispatch(Session)）
- ✅ `reports/achievement-A1-backend-engineer-apeireth-cli-session.md`（本文档）

Cargo.toml 不需要改动（chrono 已存在）。

---

## 📌 ponytail 设计取舍

| 决策 | 原因 | 升级路径 |
|---|---|---|
| Session ID 用 `timestamp + AtomicU64 counter` 而非引入 `uuid` crate | 现有依赖已含 uuid，但 stdlib 等价；少一个 `use` | 真正跨进程分布式时升级为 uuid v7 |
| `classify_risk` 用关键词启发式 | 演示 V1+V2+V3 端到端通路；不是产品逻辑 | 接 apeireth-asi 真分类器（A5） |
| stdin 对话循环用 stdlib `BufRead::read_line` | 不引入 rustyline/reedline | 阶段 7 真正的 TUI 客户端替换 |
| 保留 `placeholder()` 旧函数 + `test_placeholder_ok_backcompat` | 不破坏既有 baseline 测试 | 待其他团队用上时再删除 |
| 不重写 `dispatch(ListEpisodes/RunV1136/Quit)` | DoD 只覆盖 Session 分支 | A11 (memory) / A5 (asi) 接力 |

---

## 🎉 一句话总结

A1.1 达成 — `apeireth-cli session` 不再是 println! 硬编码壳子，真正接上 `apeireth-core` 的 Session / HA / PermissionOnion / DefaultPhilosophyGuard / V1+V2+V3 AND 门，12 条单元测试 + 6 条 qa 集成测试全绿（18 条），clippy 0 warning，实测 stdin 对话循环通过 L0 → V1 拦截关键路径。与 architect 的 ADR 0002 公开 API 完全对齐（`create_default_session` + `run_session_action(&Action) -> ActionVerdict`）。不修改承诺 7 项 100% 守住。