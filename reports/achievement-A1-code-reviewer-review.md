# A1 代码审查报告 — 代码审查角色

> **成就**: A1 (apeireth-cli 接 apeireth-core Session API)
> **审查对象**: `crates/apeireth-cli/src/main.rs` (100 行) + `crates/apeireth-cli/src/lib.rs` (34 行)
> **审查者**: code_reviewer
> **审查时间**: 2026-08-01（基于当前 working tree，commit 尚未落地）
> **DoD**: A1.3 任务 (0b019661-5248-4990-a798-8f29ed1fde6f)

---

## 📊 总览评分

| 维度 | 评分 | 状态 |
|---|---|---|
| 1. 正确性（Session 真构造 / ActionGuard 真调用） | ⚠️ 部分 | Session **未真构造**，ActionGuard **未调用** |
| 2. 测试覆盖（5+ 单元测试覆盖关键路径） | ❌ 严重不足 | 仅 1 个 test，覆盖率 ~12.5% (1/8 路径) |
| 3. 错误处理（stdin EOF / 解析错误 / clippy warning） | ⚠️ 部分 | 解析错误已处理，stdin EOF 与 clippy 边界未触发 |
| 4. 不修改承诺 7 项 | ✅ 100% 守住 | 7 项全部打勾，0 触动 |
| 5. Self-Disable 边界 | ✅ 守住 | 无违规 API，仅装饰性 print |
| 6. V1+V2+V3 AND 门 真接入 | ❌ 未接入 | 仅 `println!` 标签，无 `ActionGuard::check_action` 调用 |
| 7. Commit 规范（`crate:apeireth-cli` scope） | ⚠️ 待 commit | 当前 working tree 未 commit，无法核查 |

### **Overall Risk Level: 🟡 MEDIUM**（部分核心需求为 placeholder，未真集成）

> **Verdict**: A1 任务"扩展 apeireth-cli 接 apeireth-core Session API" 完成度约 **40%** — CLI 协议层与参数解析层完整，但 **Session API 真接入** 与 **V1+V2+V3 守门真接入** 两块核心均未实现，仅以 placeholder 字符串冒充。建议补 commit 前先补齐这两项。

---

## 🔬 维度 1：正确性（Session 字段真构造、ActionGuard 真调用）

### 1.1 Session 字段真构造 — ❌ **未真构造**

**证据**：`apeireth-core/src/lib.rs:49-53`
```rust
pub struct Session {
    pub id: String,
    pub started_at: i64,
    pub last_active_at: i64,
}
```

**关键事实**：apeireth-core 当前 **未提供 `Session::new()` 或任何 `impl Session { ... }` 构造函数**。`grep "impl Session"` 在 core lib 中返回 0 命中。Session 仅是 3 个 pub 字段的 plain data struct。

**A1.1 实现**：main.rs:55-62 `CliCommand::Session` 分支：
```rust
CliCommand::Session => {
    println!("🚀 Apeireth session 启动...");
    println!("   Session ID: apeireth-session-001");  // ← 硬编码字符串
    println!("   主 AI: 中央 AI 主体");
    println!("   守门: V1+V2+V3 AND 门 (双洋葱 + HA)");
    println!("   ✅ session 已启动 (A1 第 1 天任务完成)");
    ExitCode::SUCCESS
}
```

**问题**：
- "Session ID: apeireth-session-001" 是 **hardcoded 字符串**，未通过 `Session { id: ..., started_at: ..., last_active_at: ... }` 构造
- A1.1 **未导入** `apeireth_core::Session`
- A1.1 **未调用** 任何 Session 相关方法

**修复建议**（不动代码，仅指出方向）：
- 选项 A（推荐）：在 `apeireth-core/src/lib.rs` 加 `impl Session { pub fn new(id: impl Into<String>) -> Self { ... } }`，cli 调用之
- 选项 B：cli 直接用 struct literal `Session { id: format!("apeireth-session-{}", now), started_at: now, last_active_at: now }`，至少把字段真造出来

### 1.2 ActionGuard 真调用 — ❌ **未调用**

**证据**：grep "ActionGuard" 在 `crates/apeireth-cli/src/*.rs` 中 **0 命中**。

**核心 lib 已有**（`apeireth-core/src/lib.rs:292-368`）：
- `pub fn check_action(...)` 接收 `(action, guard, permission, ha)` → `ActionVerdict`
- 6 种 verdict：Allow / BlockByPrinciple / BlockByPermission / BlockByHA / DeferToHuman / PendingReflection

**A1.1 缺失**：
- main.rs **未导入** `ActionGuard` / `ActionVerdict` / `RiskLevel` / `ActionTarget` / `PhilosophyKey`
- main.rs **未构造** `DefaultPhilosophyGuard` / `PermissionOnion` / `HumanAuthority`
- "V1+V2+V3 AND 门 (双洋葱 + HA)" 仅是 println! 标签，不是真调用

**修复建议**：在 `dispatch(Session)` 分支前先构造守门 stack：
```rust
let guard = DefaultPhilosophyGuard;
let permission = PermissionOnion { l0: ..., ... };  // 真构造 6 层
let ha = HumanAuthority { mode: HAMode::SingleHuman, ... };
let action = Action { id: "session.start".into(), risk_level: RiskLevel::Low, target: ActionTarget::SessionStart };
let verdict = ActionGuard::check_action(&action, &guard, &permission, &ha);
```

### 1.3 参数解析层正确性 — ✅ **正确**

`parse_args` 处理 8 个分支全部正确：
- 默认（无参数）→ Session
- session / list-episodes / run-v1136 / quit → 4 个枚举
- --help / -h → Quit（main 中单独处理）
- --version / -V → println + exit(0)
- unknown → Err + help 提示

**小瑕疵**（不阻断）：
- `parse_args` 中 "session" / "list-episodes" 等命令在 lib 中是 **私有 fn in main.rs**，外部 crate 无法复用 — 但本任务范围是 binary，不算问题

---

## 🧪 维度 2：测试覆盖（5+ 单元测试覆盖关键路径）

### 现状统计

| 位置 | `#[test]` 数量 |
|---|---|
| `crates/apeireth-cli/src/main.rs` | **0** |
| `crates/apeireth-cli/src/lib.rs` | **1** (`placeholder_ok`) |
| `crates/apeireth-cli/tests/` | 0（目录不存在） |
| **合计** | **1** |

**任务要求**：5+ 单元测试覆盖关键路径 — **❌ 未达标**。

### 应覆盖的 8 条关键路径

| # | 路径 | 当前覆盖 |
|---|---|---|
| 1 | `parse_args([])` → `Session`（默认） | ❌ |
| 2 | `parse_args(["apeireth-cli", "session"])` → `Session` | ❌ |
| 3 | `parse_args(["apeireth-cli", "list-episodes"])` → `ListEpisodes` | ❌ |
| 4 | `parse_args(["apeireth-cli", "run-v1136"])` → `RunV1136` | ❌ |
| 5 | `parse_args(["apeireth-cli", "quit"])` → `Quit` | ❌ |
| 6 | `parse_args(["apeireth-cli", "--version"])` → exit(0)（难测，跳过） | ⚠️ |
| 7 | `parse_args(["apeireth-cli", "unknown"])` → `Err(String)` | ❌ |
| 8 | `dispatch(Quit)` 等 4 个 variant | ❌ |

### 修复建议（最低限度 6 个 test）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_default_to_session() {
        assert!(matches!(parse_args(&argv(&["apeireth-cli"])), Ok(CliCommand::Session)));
    }

    #[test]
    fn parse_known_commands() {
        for cmd in ["session", "list-episodes", "run-v1136", "quit", "exit"] {
            let r = parse_args(&argv(&["apeireth-cli", cmd]));
            assert!(r.is_ok(), "cmd {cmd} should parse");
        }
    }

    #[test]
    fn parse_unknown_command_err() {
        let r = parse_args(&argv(&["apeireth-cli", "rm-rf"]));
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("未知命令"));
    }

    #[test]
    fn dispatch_quit_returns_success() {
        assert_eq!(dispatch(CliCommand::Quit), ExitCode::SUCCESS);
    }

    #[test]
    fn dispatch_list_episodes_placeholder() {
        assert_eq!(dispatch(CliCommand::ListEpisodes), ExitCode::SUCCESS);
    }

    #[test]
    fn dispatch_run_v1136_mentions_baseline() {
        // placeholder 必须显示 R11 baseline 0.9063
        // 此测需 capture stdout；如不便可省略
    }
}
```

**位置建议**：`main.rs` 中的 `parse_args` / `dispatch` 是 `fn`（私有）— 测试需放 main.rs（binary 测试）或将其移到 lib.rs 改为 `pub`。**Ponytail lazy 建议**：直接放 main.rs 的 `#[cfg(test)] mod tests`，最小改动。

---

## 🛡️ 维度 3：错误处理（stdin EOF / 解析错误 / clippy warning）

### 3.1 解析错误 — ✅ **已处理**

- `parse_args` 返回 `Result<CliCommand, String>`
- main 中 `Err(e)` → `eprintln!` + `print_help()` + `ExitCode::FAILURE`
- ✅ 错误信息含可用命令列表（user-friendly）

### 3.2 stdin EOF — ⚠️ **不适用（当前未实现 stdin 读取）**

A1 第 1 天任务描述中提到：
> 主交互 = 对话流（输入文本 → 走 V1+V2+V3 → 返回）

但 A1.1 **未实现 stdin 读取**（grep "stdin\|Stdin\|io::stdin" 0 命中）。当前 `Session` 分支仅 println 后立即 ExitCode::SUCCESS，**没有对话循环**。

**评估**：
- 若 A1.1 范围**仅是"启动 session 并打印欢迎"** → stdin 不适用，不扣分
- 若 A1.1 范围**包含"对话流"** → ❌ 缺失

**建议**：与 Leader 确认 A1 范围边界。若含对话流 → 需补 `BufReader::new(io::stdin()).lines()` + EOF 处理（`Err(stdin.read_line...)` 或 `Ok(0)`）。

### 3.3 clippy warning — ✅ **0 warning 在 apeireth-cli 中**

```
Checking apeireth-cli v0.14.0
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.97s
```

`cargo clippy -p apeireth-cli --no-deps` 输出仅 "Finished"，无 warning。

（顺带：apeireth-core 有 120 warning — `missing_docs` for `Interrupted` variant 等 — 但 **不属于本 PR 范围**，不在 A1.3 审查对象内。）

### 3.4 其他 — ✅

- `env!("CARGO_PKG_VERSION")` 安全（编译期展开）
- `ExitCode::SUCCESS/FAILURE` 正确使用
- `process::exit(0)` 仅在 `--version` 处使用（合理）
- 无 panic-prone 代码（unwrap/expect 均 0 命中）

---

## 🛡️ 维度 4：不修改承诺 7 项（每个项打勾确认）

> 来源：START-CONSTRUCTION.md §87-96 + §738-746

| # | 不修改承诺 | 是否触动 | 证据 |
|---|---|---|---|
| 1 | **阶段 1+2+3 LOCKED**（54 份设计文档） | ✅ 未触动 | A1.1 只动 `crates/apeireth-cli/` + `reports/`，未碰 docs/ 设计文档 |
| 2 | **v2 / v4 / v4.1 LOCKED**（哲学层纲领） | ✅ 未触动 | 同上 |
| 3 | **阶段 4 主文档 LOCKED**（1492 行，6ca80776） | ✅ 未触动 | 未编辑 stage4 主文档 |
| 4 | **阶段 5 施工文档 LOCKED**（631 行） | ✅ 未触动 | 未编辑 stage5 施工文档 |
| 5 | **v6 修正（4 重守门 + 权限发放 + E 层修改路径）** | ✅ 未破坏 | v6 在 apeireth-core 中完整保留；A1.1 仅 print 一句"守门: V1+V2+V3"，未改 v6 逻辑 |
| 6 | **R11 baseline 三值**（V1141=0.8682 / V1131=0.8532 / V1136=0.9063） | ✅ 未修改 | main.rs:72 println `"baseline = 0.9063"` 是 placeholder **显示**，不是修改基线值。符合"展示允许、修改禁止" |
| 7 | **v1 → v5 历史链 LOCKED**（保留，不删除） | ✅ 保留 | git log 显示 v1-v5 历史 commit 完整保留；A1.1 工作分支未删任何历史 |

**结论**：7 项 LOCKED **100% 守住**。✅

---

## 🔒 维度 5：Self-Disable 边界（stage4-external-feedback-and-revisions.md §3）

> 来源：START-CONSTRUCTION.md §487-495

| 禁令 | 是否违反 | 证据 |
|---|---|---|
| ❌ 不得修改 L0 HA 相关 trait | ✅ 未违反 | A1.1 不涉及 trait 定义或修改 |
| ❌ 不得添加能绕过 V1+V2+V3 AND 门的代码 | ✅ 未违反 | A1.1 未写任何守门 bypass 代码（实际上连守门本身都没写） |
| ❌ 不得添加"询问是否需要 L0 HA"的元问题 API | ✅ 未违反 | CliCommand 仅 Session/ListEpisodes/RunV1136/Quit，无元问题 API |
| ✅ 必须让反思期白名单生效 | ⚠️ N/A | 反思期未在本 PR 实现 |
| ✅ 必须保持 HA 在权限洋葱核心 L0（不可变） | ✅ 守住 | A1.1 未尝试移动 HA 位置 |

**关键风险点（潜在但未触发）**：
- main.rs:59 写 `"守门: V1+V2+V3 AND 门 (双洋葱 + HA)"` 是 **label print** 而非真实接入 → 虽然字面提到 "HA"，但**不构成 Self-Disable 违规**（因为只是字符串）
- 但 **风险**：未来若有人看到这段代码以为"已接入守门"，可能误判 Self-Disable 状态 → 建议在 print 文案加注释 `// TODO(A1+): 接入真 ActionGuard::check_action`

**结论**：Self-Disable 边界 **守住**，但留有 1 个 TODO 注释位。

---

## 🛡️ 维度 6：4 重守门（V1+V2+V3 AND 门）真接入

> v6 设计：V1 编译时 hardcode + V2 运行时拦截 + V3 Multi-AI consensus → AND 门真接入

### A1.1 接入情况

| 层 | 当前状态 |
|---|---|
| V1 编译时 hardcode（12 键 trait + 双洋葱 + 电子环） | ❌ 0 调用 |
| V2 运行时拦截（`ActionGuard::check_action`） | ❌ 0 调用 |
| V3 Multi-AI consensus（未来 A5 apeireth-asi） | N/A（未到 A5） |
| AND 门真接入 | ❌ 未实现 |

**唯一提及**："守门: V1+V2+V3 AND 门 (双洋葱 + HA)" 是 **println! 标签字符串**（main.rs:59），**不是真接入**。

### 修复建议

按 v6 设计真接入需 3 步：
1. **构造守门 stack**（cli main）：
   ```rust
   let guard = DefaultPhilosophyGuard;
   let permission = PermissionOnion::default();  // 需 core 提供 Default
   let ha = HumanAuthority::default();
   ```
2. **调用 ActionGuard**：
   ```rust
   let action = Action { id: "session.start".into(), risk_level: RiskLevel::Low, target: ActionTarget::SessionStart };
   match ActionGuard::check_action(&action, &guard, &permission, &ha) {
       ActionVerdict::Allow => /* 启动 */,
       ActionVerdict::DeferToHuman => /* 退出 + 提示 */,
       ActionVerdict::BlockByPrinciple(k) => eprintln!("Blocked by {:?}", k),
       _ => return ExitCode::FAILURE,
   }
   ```
3. **physics HA 联动**（L0 永远 requires_ha=true）

> **Ponytail lazy 提醒**：补 V2 拦截即可，V1/V3 在后续成就完成。当前成就目标是"接 Session API"，守门接入可放 A1+ 或单独 PR。

---

## 📝 维度 7：Commit 规范（`crate:apeireth-cli` scope）

### 当前 git 状态

```
On branch rebase/d7d8-into-integration
Changes not staged for commit:
    modified:   ../.spectrai-worktrees/r10-ao-retry2 ...
    deleted:    START-HERE-FOR-CONSTRUCTION-LEADER.md

Untracked files:
    ../.spectrai-worktrees/integrations/e8de47ae-...
```

**关键事实**：
- A1.1 的代码改动 **尚未 commit**（仅 working tree 状态）
- `git diff HEAD -- crates/apeireth-cli/` 返回空（changes already committed? 或实际 working tree 与 HEAD 一致？）

**待 commit 的预期格式**（按 START-CONSTRUCTION.md §459-468）：
```
crate:apeireth-cli <subject>  (≤ 72 字符)
```

**预期 subject 候选**：
- `crate:apeireth-cli A1 session + 4 command protocol skeleton`（推荐）
- `crate:apeireth-cli CliCommand enum + dispatch + parse_args`

**建议**：
- ✅ scope 必须是 `crate:apeireth-cli`（per APEIRETH-CONVENTIONS.md Commit 规范）
- ✅ subject 描述动作 + 范围，≤ 72 字符
- ⚠️ 当前 working tree 同时删除了 `START-HERE-FOR-CONSTRUCTION-LEADER.md` — **这与 A1.1 无关**（应在其他 commit 处理，避免 commit 范围污染）

---

## 🎯 Risk Level 与修复建议

### Risk Level: 🟡 **MEDIUM**

| 项 | 严重度 | 说明 |
|---|---|---|
| Session 未真构造 | 🟡 中 | 任务核心需求未满足，但不影响 build |
| ActionGuard 未调用 | 🟡 中 | 守门接入未完成，但当前 placeholder 阶段可接受 |
| 测试覆盖不足（1 vs 5+） | 🟡 中 | 不阻断 merge 但违反 §480-485 测试要求 |
| stdin EOF / 对话流 | 🟢 低 | 范围边界问题，与 Leader 确认即可 |
| Commit 未落地 | 🟢 低 | 时序问题，commit 时按规范即可 |
| Self-Disable 边界 | 🟢 无 | 完全守住 |
| 不修改承诺 7 项 | 🟢 无 | 100% 守住 |

### 修复建议（按优先级）

1. **🔴 P0 — 补 Session 真构造**：在 apeireth-core 加 `impl Session { pub fn new(id: impl Into<String>) -> Self }`，cli 调 `Session::new("apeireth-session-001")` 替代字符串硬编码（1 行 core + 1 行 cli）
2. **🟡 P1 — 补 5+ 单元测试**：在 main.rs `#[cfg(test)] mod tests` 加 `parse_default / parse_known_commands / parse_unknown_err / dispatch_*` 至少 5 个（30 行）
3. **🟡 P1 — 补 V2 守门接入**：构造 `DefaultPhilosophyGuard + PermissionOnion + HumanAuthority` + 调用 `ActionGuard::check_action`（~20 行 cli）
4. **🟢 P2 — 与 Leader 确认 A1 范围**：stdin 对话流是否在 A1 范围内？若是 → 补对话循环 + EOF 处理
5. **🟢 P2 — commit 规范**：用 `crate:apeireth-cli <subject>` 格式 commit

### 不建议做的事（守住边界）

- ❌ **不要**触碰不修改承诺 7 项（已守住，继续守）
- ❌ **不要**添加"询问是否需要 L0 HA"等元问题 API
- ❌ **不要**为 A1 添加新依赖（serde/tokio/anyhow/thiserror/chrono 已在 Cargo.toml 足够）
- ❌ **不要**实现 V3 Multi-AI consensus（属于 A5 apeireth-asi）

---

## 📋 审查结论（一句话）

**A1 当前实现是"协议层 skeleton"（占位完整 + 协议正确 + 守门标签齐全），但"真接入"层缺位（Session 未构造、ActionGuard 未调用、测试 1/8）—— 建议在 commit 前补 P0 + P1 两项以满足 A1 DoD，否则 risk level 维持 MEDIUM。**

---

> **审查者**: code_reviewer (A1.3 任务 0b019661-5248-4990-a798-8f29ed1fde6f)
> **状态**: 已完成审查，待 Leader 决定是否要求 A1.1 补齐后再 commit
> **下一步**: 等待 Leader 反馈；如要求修复则进入 A1.4 修复 PR；如接受现状则进入 A2 成就