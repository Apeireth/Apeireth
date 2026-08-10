# R32-2: Pipeline tool loop — LangGraph 借鉴

**日期**: 2026-08-09
**作者**: Mavis
**状态**: ✅ 完成
**ROI**: ★★★★★ (借鉴 LangGraph StateGraph + conditional edge, 抽 R30 硬编码循环, 给未来 conditional branching 留口子)

---

## 1. 目标

R30 在 `apeireth-tui/src/backend.rs` 写了两份 tool-loop 循环 (`chat_with_tool_loop` 非流式 + `chat_with_tool_loop_streaming` 流式),
**硬编码** `const MAX_TOOL_TURNS: usize = 3` + 重复的循环骨架. 业务要换循环上限 / 改停止条件 / 引入 conditional branching
(例如 "tool error 立即停", "首轮无 tool 立即停", "max_turns 走配置而非 hardcode") 都要在 TUI 改, 跨 crate 复用 0.

R32-2 借鉴 **LangGraph** 的 `StateGraph` + `add_conditional_edges` 抽象, 把"agent 循环"显式化成:
- **State** (`ToolLoopState`) — 显式 state (input / history / last_reply / turn / max_turns / error)
- **Conditional edge** (`should_continue(state) -> bool`) — 看 state 决定下一节点
- **Runner** (`run_tool_loop<F>(init, step: F) -> State`) — 状态机循环, F 注入 LLM step

业务侧 (TUI / 未来 council / 未来 eval harness) 只需"接 state / 调 LLM / 解析 dispatch" 3 步,
**循环控制权归 pipeline** (`apeireth-pipeline::tool_loop`), 跨 crate 复用 + 跨业务一致.

---

## 2. 设计

### 2.1 `ToolLoopState` (借鉴 LangGraph 节点 state)

```rust
pub struct ToolLoopState {
    pub input: String,
    pub history: Vec<ToolLoopMessage>,
    pub last_reply: String,
    pub turn: usize,
    pub max_turns: usize,
    pub error: Option<String>,
}
```

### 2.2 `should_continue` (LangGraph conditional edge 借鉴)

```rust
pub fn should_continue(state: &ToolLoopState) -> bool {
    if state.error.is_some() { return false; }                  // 错停
    if state.last_reply.is_empty() { return true; }             // 首轮还没跑 → 继续
    if state.turn >= state.max_turns { return false; }          // 超限停
    state.last_reply.contains("<<<[TOOL_REQUEST]>>>")          // 有 tool call 继续, 无停
}
```

### 2.3 `LlmStepResult` (调用方注入 step 闭包)

```rust
pub struct LlmStepResult {
    pub reply_text: String,
    pub has_tool_call: bool,
    pub tool_results: String,
    pub error: Option<String>,
}
```

提供 3 个工厂方法:
- `final_answer(reply)` — 没 tool call 的最终答复
- `err(err, fallback)` — LLM 错
- `with_tool_call(reply, results)` — 有 tool call, 把 results 拼到 history

### 2.4 `run_tool_loop` (state machine runner)

```rust
pub fn run_tool_loop<F>(mut state: ToolLoopState, mut step: F) -> ToolLoopState
where F: FnMut(&mut ToolLoopState) -> LlmStepResult,
{
    loop {
        if !should_continue(&state) { break; }
        let result = step(&mut state);
        state.error = result.error;
        state.last_reply = result.reply_text.clone();
        state.turn += 1;
        if result.has_tool_call && !result.tool_results.is_empty() {
            state.history.push(ToolLoopMessage {
                role: "user".to_string(),
                content: format!(
                    "你的上一轮回复:\n\"\"\"\n{}\n\"\"\"\n工具调用结果:\n{}",
                    result.reply_text, result.tool_results
                ),
            });
            state.input = "继续基于上面的工具结果回答用户最初的问题.".to_string();
        }
    }
    state
}
```

---

## 3. 改动

### 3.1 新增 `crates/apeireth-pipeline/src/tool_loop.rs` (340 LOC)

- 公开 API: `ToolLoopState`, `ToolLoopMessage`, `LlmStepResult`, `should_continue`, `run_tool_loop`, `DEFAULT_MAX_TOOL_TURNS`
- 11 unit test (tool_loop_tests mod)

### 3.2 `crates/apeireth-pipeline/src/lib.rs`

- 加 `pub mod tool_loop;` + 6 个 re-export

### 3.3 `crates/apeireth-tui/src/backend.rs`

- `chat_with_tool_loop` (非流式) 重写: 循环控制权交 `run_tool_loop`, 业务侧只注入 LLM step 闭包
- `chat_with_tool_loop_streaming` (流式) 同样重写
- 两份都 0 业务漂移, 行为完全一致 (DEFAULT_MAX_TOOL_TURNS = 3 保持 R30 兼容)

### 3.4 `crates/apeireth-tui/Cargo.toml`

- 加 `apeireth-pipeline = { path = "../apeireth-pipeline" }` (TUI 第一次直接依赖 pipeline)

---

## 4. 测试

### 4.1 11 个新 unit test 全过 (apeireth-pipeline)

```
test tool_loop::tool_loop_tests::default_max_turns_is_3_for_r30_compat ... ok
test tool_loop::tool_loop_tests::should_continue_first_turn_empty_reply_returns_true ... ok
test tool_loop::tool_loop_tests::should_continue_tool_call_returns_true ... ok
test tool_loop::tool_loop_tests::should_continue_no_tool_call_returns_false ... ok
test tool_loop::tool_loop_tests::should_continue_exceeds_max_turns_returns_false ... ok
test tool_loop::tool_loop_tests::should_continue_error_set_returns_false ... ok
test tool_loop::tool_loop_tests::run_tool_loop_two_turns_tool_then_final ... ok
test tool_loop::tool_loop_tests::run_tool_loop_error_immediately_stops ... ok
test tool_loop::tool_loop_tests::run_tool_loop_zero_max_turns_first_step_runs_then_stops ... ok
test tool_loop::tool_loop_tests::llm_step_result_constructors ... ok
test tool_loop::tool_loop_tests::tool_loop_message_roles ... ok

test result: ok. 11 passed; 0 failed
```

### 4.2 回归 (apeireth-tui 全 workspace)

- TUI 398/398 unit test pass (跟 R31 末态对齐, 0 退化)
- 0 fail

---

## 5. 不漂移 (主哲学锚 #1)

- `tool_loop` **不知道** 具体 LLM / dispatch 是啥, 全部由 F 闭包注入 (LangGraph "图结构" 思路)
- TUI 业务侧 `call_llm_stream_sync` + `parse_and_dispatch_tools_with_evt` 0 改动, 0 漂移
- DEFAULT_MAX_TOOL_TURNS = 3 保持 R30 兼容 (R19 战役 0 7 阶段不改, R30 行为不变)
- 借鉴是真借鉴 (StateGraph + conditional edge 真抽象), 不是抄字面 (LangGraph 是 Python, 我们落地为 Rust state machine)

---

## 6. 顺手给后续 R 留的口子

- **max_turns 走配置**: 现在 hardcode `DEFAULT_MAX_TOOL_TURNS = 3`, 未来 TUI 接 config.json 改 `state.max_turns` 即可
- **conditional branching 拓展**: `should_continue` 现在只看 tool call + turn, 未来加 "tool error count > 2 停" / "总 token > budget 停" / "无 tool 立即停" 都集中在 `should_continue` 一处
- **R33-5 (LangGraph conditional 实战)**: 后续可在 pipeline 拓 `should_branch(state) -> BranchKind` 走 conditional routing (e.g. "tool call" → 工具节点, "纯文本" → 直接结束, "用户打断" → 取消节点)

---

## 7. 后续路线

- ✅ R32-2 完成
- ⏭ R33-1 (Aider conventions scanner, 1d) — 给 LLM 注入项目 dep / edition / lints 上下文
- ⏭ R32-3 (eval smoke test, 2d) — 借 `run_tool_loop` 给 apeireth-eval 真接 1 个 task
- ⏭ R36 (91→40 瘦身, 5d) — 5 老 provider crate 真删
- ⏭ R37-1 (ProtocolRouter 砍 1 层, 1d)
- ⏭ R37-2 (9 organ 部分合并, 3-5d)
- ⏭ R33-3 (MCP resources, 2d)
- ⏭ R33-4 (AutoGen council, 2d)
- ⏭ R33-5 (LangGraph conditional 实战 — 跟 R32-2 后续一起)

---

**Total LOC**: 1 new file (340) + 2 modify (lib.rs re-export + backend.rs 2 function 重构) + 1 Cargo.toml 依赖加, 全 workspace build + test pass.
