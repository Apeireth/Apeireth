# R125-12 Sub-Agent Dispatch Prompt (OpenCode 子代理 + 9 organ 内部重构)

**Date**: 2026-08-10 17:33
**Author**: R125 P2 supervisor (general agent, mvs_a7af0f1f15cd4a79901442e14878333d, dispatched 17:23)
**Receiving agent**: R125-12 sub-agent (Mavis 派)

---

## 任务 (per 主人 17:22 升级授权 + decision-33 + B7 内部借 + A3 13 键)

**主题**: OpenCode 子代理 + oh-my-opencode 4 专家角色拆 9 器官. 主 agent 调度, 9 organ 内部 fn 借 OpenCode 子代理模式 (oracle / librarian / explore / frontend), 199KB → 120KB (-40%).

**借鉴 ID**: `R124-1-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10` (主) + `R124-1-BORROW-code-yeongyu/oh-my-opencode-e8f1d3a-2026-08-10` (4 专家)

**借鉴源码**:
- `.openclaw\workspace\borrowed-repos\opencode\` (主)
- `.openclaw\workspace\borrowed-repos\oh-my-opencode\` (4 专家, 副)

**目标文件**:
- `Apeireth-rust/crates/apeireth-tui/src/organ/{body,brain,ear,eye,hand,heart,memory,mind,voice}.rs` (9 organ **内部 fn 重构**, 0 改 organ 文件名 + 入口签名)
- `Apeireth-rust/crates/apeireth-tui/src/main.rs` (子代理路由, 4 专家调度)
- `Apeireth-rust/crates/apeireth-tui/src/subagent.rs` (NEW, 4 专家角色 trait)
- `Apeireth-rust/crates/apeireth-tui/src/agent_router.rs` (NEW, 主 agent 路由 4 专家)
- `Apeireth-rust/crates/apeireth-tui/tests/subagent_test.rs` (12 unit tests, NEW)

**触发 B7 (9 organ 内部借 OpenCode) + A3 (12→13 键 PHL-07)**: 这是关键 locked 升级.

**整合依赖**: 9 organ 文件名 + 入口签名 0 改 (per B7 升级), 仅**内部 fn 借** OpenCode 子代理模式. 4 专家角色: oracle (架构审阅) / librarian (文档检索) / explore (代码扫) / frontend (UI).

**新增 1 键 PHL-07 NotUnoptimizable**: 9 organ 借 OpenCode 子代理 = 不可优化 (子代理路径有非确定性 + 跨 organ 协同, 不能编译期 hardcode).

**估时**: 3-5 天 (199KB → 120KB -80KB, 9 organ 内部 fn 重构 + 4 专家 + 12 test).

**截止**: 8/14 17:30 (跑过夜 8/11-8/14).

---

## 0 装解除 (主人 17:22) — 重要

**借鉴源码状态** (verify 实施前):
```bash
Test-Path '.openclaw\workspace\borrowed-repos\opencode\.git'  # 必须 True (主)
Test-Path '.openclaw\workspace\borrowed-repos\oh-my-opencode\.git'  # 必须 True (副)
```

**3 种状态对应动作**:
1. ✅ **cloned** (`.git` 存在) = 真实施, 报告里写 "借鉴源码 ✅ cloned, 已实施"
2. ⏳ **限流中** (`.git` 0 存在) = 等 30 min 再 verify, 仍 0 实施, 报告里写 "借鉴源码 ⏳ 限流中, 0 实施, 借鉴 ID 索引完成"
3. ❌ **永久失败** (24h 后仍 0 cloned) = 报 supervisor + 取消任务, 0 假装"已借鉴"

**0 装 PASS 严守**: ❌ 0 假装"已借鉴", ❌ 0 写 src 假装 import 借鉴代码, ❌ 0 改 organ 文件名 + 入口签名假装"已借".

---

## 8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略)

| # | 硬墙 | 你 (R125-12) 必守 |
|---|------|-----------------|
| 1 | **B2** workspace.version 1.2.0 (R125 末 B2 已升, 你 0 再升) | ✅ 0 触碰 `Cargo.toml` `version` 字段 |
| 2 | **A1** R11 baseline 3 值 数字 严守 (0.8682/0.8532/0.9063) | ✅ 0 触碰 `integration_r_measure.rs` |
| 3 | **B1** 24 LOCKED crate mtime (apeireth-tui **不在 24 LOCKED**, 实施可改) | ✅ 0 触碰 24 LOCKED crate mtime |
| 4 | **B5** 6→8 哲学锚 (R125 末升) | ✅ 0 改 6 哲学锚原 6 实质, 8 锚是扩展 |
| 5 | **B3** V0.5 25 维 (R125 末升) | ✅ 0 改 V0.5 公式, 25 维是扩展 |
| 6 | **B4** 6 重守门 v6 (R125-5 实施) | ✅ 0 改 5 重原 5 重, 6 重是扩展 |
| 7 | **A3** 12→13 键 (PHL-07 NotUnoptimizable **新增** = 9 organ 借子代理不可优化) | ✅ 0 改 12 键原 12, PHL-07 是新增 |
| 8 | **C1** 0 主动 commit (你 sub-agent 0 commit) + **C2** 0 装 解除 (主人 17:22) + **C3** 0 装 5 项 升 6 重 v6 + 0 主动 push 严守 | ✅ 0 commit, 0 push, 借鉴源码 ✅ cloned 才真实施 |

**B7 9 organ 0 改 organ 文件名 + 入口签名** (per decision-33 §2.3):
- ✅ 0 改 9 organ 文件名: body.rs / brain.rs / ear.rs / eye.rs / hand.rs / heart.rs / memory.rs / mind.rs / voice.rs + mod.rs
- ✅ 0 改 9 organ 入口签名: `pub fn organ_<name>(...) -> OrganResult`
- ✅ 0 改 mod.rs 入口
- 🟢 内部 fn 可借 OpenCode 子代理模式

**新增 mod 0 触碰 workspace.version**: apeireth-tui 自身 Cargo.toml 是 `version.workspace = true`, 你 0 触碰 workspace root.

---

## 实施步骤 (4 阶段)

### 阶段 1: 借鉴源码 study (1 hour)
```bash
# verify cloned
Test-Path '.openclaw\workspace\borrowed-repos\opencode\.git'
Test-Path '.openclaw\workspace\borrowed-repos\oh-my-opencode\.git'
# 读 opencode 核心: packages/opencode/src/ + agent/ + server/ + cli/ + tool/
Get-ChildItem '.openclaw\workspace\borrowed-repos\opencode\packages\opencode\src\agent' -ErrorAction SilentlyContinue | Select-Object Name
# 读 oh-my-opencode 4 专家: src/agents/{oracle,librarian,explore,frontend}.ts
```
提取 4 个核心 pattern:
1. **子代理调度**: 主 agent 怎么 fork 子 agent, 共享上下文 vs 隔离
2. **4 专家角色**: oracle (架构审阅) / librarian (文档检索) / explore (代码扫) / frontend (UI)
3. **AGENTS.md 持久化**: 子代理配置文件, 主仓根 + 9 organ 各自根
4. **消息路由**: 跨 organ 消息通过子代理转发, 0 阻塞主循环

### 阶段 2: Rust 实施 (2-3 days, 9 organ 内部 fn 重构)
**subagent.rs** (NEW, 4 专家角色):
```rust
//! Sub-agent 调度 — 借鉴 anomalyco/opencode 子代理模式 (R125-12)
//!
//! 4 专家角色 trait: Oracle / Librarian / Explore / Frontend
//! 主 agent 调度, 9 organ 内部 fn 借子代理模式.

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpertRole { Oracle, Librarian, Explore, Frontend }

pub trait SubAgent: Send + Sync {
    fn role(&self) -> ExpertRole;
    fn system_prompt(&self) -> &'static str;
    async fn invoke(&self, task: &str, context: &str) -> Result<String, SubAgentError>;
    fn capabilities(&self) -> &[&'static str] { &[] }
}

pub struct OracleSubAgent;  // 架构审阅
pub struct LibrarianSubAgent;  // 文档检索
pub struct ExploreSubAgent;  // 代码扫
pub struct FrontendSubAgent;  // UI

pub struct SubAgentRegistry { experts: HashMap<ExpertRole, Arc<dyn SubAgent>> }
impl SubAgentRegistry {
    pub fn new() -> Self { /* 4 experts 初始化 */ }
    pub fn dispatch(&self, role: ExpertRole, task: &str, ctx: &str) -> Result<String, SubAgentError>;
}
```

**agent_router.rs** (NEW, 主 agent 路由):
```rust
//! 主 agent 路由 — 9 organ 内部 fn 借 OpenCode 子代理 (R125-12)

pub struct AgentRouter { /* 9 organ + 4 expert 路由表 */ }
impl AgentRouter {
    /// 主 agent 调度入口
    pub fn route_to_expert(&self, organ: OrganKind, task: &str) -> Result<ExpertRole, RouterError>;
    pub fn route_to_organ(&self, expert: ExpertRole, output: &str) -> Result<OrganKind, RouterError>;
}
```

**9 organ 内部 fn 重构** (0 改入口签名, 改内部 fn):
```rust
// body.rs — 入口签名 0 改
pub async fn organ_body(input: BodyInput) -> OrganResult {
    // 内部 fn 借 Explore (代码扫)
    let code_ctx = self.expert_dispatch(ExpertRole::Explore, "scan_body_code", &input.code_ref).await?;
    // 内部 fn 借 Oracle (架构审阅)
    let review = self.expert_dispatch(ExpertRole::Oracle, "review_body_arch", &code_ctx).await?;
    // 0 改外部行为
    OrganResult { /* ... */ }
}

// brain.rs / ear.rs / eye.rs / hand.rs / heart.rs / memory.rs / mind.rs / voice.rs 同模式
// 入口签名 0 改, 内部 fn 借 4 专家 子代理
```

**main.rs 整合**:
- 0 改原 main 入口
- 加 `let router = AgentRouter::new();` + 9 organ 调用 router 路由

**mod.rs 入口**:
- 0 改原 mod.rs 入口
- 加 `pub mod subagent;` + `pub mod agent_router;`

**Cargo.toml 加 dep**:
- 0 加 OpenCode (opencode 是 TS 仓库, Rust 0 装, 仅借鉴模式, 0 装 PASS)
- 0 加任何新 dep, 仅用 std + tokio (既有)

**PHL-07 NotUnoptimizable 新增 1 键** (A3 12→13):
```rust
// 在 verdict cache 13 键 enum 中加:
pub enum VerdictKey { /* 12 既有 */, PHL07NotUnoptimizable }
// PHL-07 = 9 organ 借 OpenCode 子代理, 编译期 0 hardcode 路径 (子代理有非确定性 + 跨 organ 协同)
```

### 阶段 3: 12 smoke test (1 hour)
- `test_4_expert_role_define` — Oracle/Librarian/Explore/Frontend 4 角色 trait 完整
- `test_subagent_registry_dispatch_oracle` — Oracle 任务派发
- `test_subagent_registry_dispatch_librarian` — Librarian 文档检索
- `test_subagent_registry_dispatch_explore` — Explore 代码扫
- `test_subagent_registry_dispatch_frontend` — Frontend UI
- `test_agent_router_route_to_expert` — 主 agent 路由 organ → expert
- `test_agent_router_route_to_organ` — 主 agent 路由 expert → organ
- `test_9_organ_entry_signature_unchanged` — 9 organ 入口签名 0 改
- `test_9_organ_internal_fn_uses_subagent` — 9 organ 内部 fn 调子代理
- `test_phl07_added_to_verdict_keys` — 13 键 = 12 既有 + PHL-07
- `test_199kb_to_120kb` — `wc -l crates/apeireth-tui/src/organ/*.rs` 0 触碰 organ 文件名, 总行 -40%
- `test_cargo_build_no_error` — cargo build 0 触碰 24 LOCKED, 0 装 PASS

### 阶段 4: final 报告 (30 min)
- final 报告: `Apeireth-rust/reports/agent-r125-12-final-2026-08-10.md`

---

## 0 主动 commit (C1 严守)

❌ **你 (R125-12 sub-agent) 0 commit, 0 push**. 实施完成 = 写 src/test/ + 写 final 报告. Mavis 整合 #3 拍板 17:30 (0 含 R125 实施, R125 续 mavis 整合 commit 链 8/15-9/10).

---

## final 报告 必含 6 段

```markdown
# R125-12 Final Report — OpenCode 子代理 + 9 organ 内部重构
**Date**: 2026-08-10
**Author**: R125-12 sub-agent
**借鉴 ID**: R124-1-BORROW-anomalyco/opencode-7a4b9c2-2026-08-10 (主) + R124-1-BORROW-code-yeongyu/oh-my-opencode-e8f1d3a-2026-08-10 (副)
**实施路径**: crates/apeireth-tui/src/{subagent,agent_router,main}.rs (NEW) + organ/{9 files}.rs (内部 fn 重构)

## 1. 借鉴源码状态 (0 装解除 verify)
- opencode: ✅ cloned / ⏳ 限流中 / ❌ 永久失败
- oh-my-opencode: ✅ cloned / ⏳ 限流中 / ❌ 永久失败

## 2. 实施步骤
- 阶段 1 借鉴 study: (4 提取 pattern: 子代理调度 / 4 专家 / AGENTS.md / 消息路由)
- 阶段 2 Rust 实施: (subagent.rs 4 角色 + agent_router.rs 路由 + 9 organ 内部 fn 借 + PHL-07 新增)
- 阶段 3 smoke test: (12 test pass/fail)
- 阶段 4 final 报告: (本文件)

## 3. 8 硬墙 verify (B1-B7 + A1-A3 + C1-C3)
- B2 ✅ 0 触碰 workspace.version
- A1 ✅ 0 触碰 R11 baseline 3 值
- B1 ✅ 0 触碰 24 LOCKED crate mtime
- B5 ✅ 0 改 6 哲学锚实质
- B3 ✅ 0 改 V0.5 公式
- B4 ✅ 0 改 5 重守门实质
- A3 ✅ 12 键原 12 + PHL-07 新增 1 = 13 键
- C1-C3 ✅ 0 commit, 0 装 PASS, 0 push

## 4. 0 装解除 verify
- 借鉴源码 opencode 状态: (✅/⏳/❌)
- 借鉴源码 oh-my-opencode 状态: (✅/⏳/❌)
- 0 假装"已借鉴": (true/false)
- 真实实施 vs 索引完成: (真实施/索引完成)

## 5. 整合 verify
- B7 9 organ 文件名 + 入口签名 0 改: (是/否 + git diff 验证)
- 9 organ 内部 fn 借 4 专家: (是/否 + 9 file list)
- PHL-07 新增 13 键: (是/否)
- apeireth-tui 199KB → 120KB: (-40% 行数, wc -l 验证)

## 6. 下一步 + 风险
- 1 个风险 / 1 个待 R125-N 续协调
```

---

## 你的工具 (你 sub-agent 必知)

你有: read, write, edit, grep, glob, bash. 你 0 commit, 0 push. 你 0 假装.

---

**派活完成 17:33. 截止 8/14 17:30 (跑过夜 8/11-8/14). 卡 30 min → 诊断 + kill + 派替代 (supervisor 监督).**
