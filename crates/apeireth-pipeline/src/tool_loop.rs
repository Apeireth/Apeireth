//! R32-2: 通用 tool-call 循环状态机 + 条件边
//!
//! **借鉴 LangGraph** (`langgraph` StateGraph + conditional edge):
//! LangGraph 把"agent 循环"建模成 `StateGraph` 节点 + `add_conditional_edges` 看 state 决定
//! 下一节点 (continue / end). 我们借鉴这个抽象, 把 R30 硬编码的 `MAX_TOOL_TURNS = 3`
//! 循环抽成显式 state machine:
//! - `ToolLoopState` — 显式 state (input / history / last_reply / turn / max_turns / error)
//! - `should_continue(&state) -> bool` — 条件边 (LangGraph `add_conditional_edges`)
//! - `run_tool_loop<F>(init, step: F) -> ToolLoopState` — 状态机 runner, F 由调用方注入
//!
//! **不漂移 (主哲学锚 #1)**:
//! - pipeline.tool_loop **不知道** 具体 LLM / dispatch 是啥. 全部由 `F: FnMut(&mut State) -> LlmStepResult` 注入.
//! - TUI 把 `call_llm_stream_sync + parse_and_dispatch_tools_with_evt` 装成闭包, 业务代码 0 漂移.
//! - `should_continue` 规则可测: 单测覆盖 "有 tool call 继续 / 无停 / 超上限停 / 错停".
//!
//! **不引用但已承接的 R30 行为**:
//! - R30 P0 `chat_with_tool_loop` (非流式) 和 P4 `chat_with_tool_loop_streaming` 都换走 `run_tool_loop`,
//!   循环控制权统一到 pipeline, TUI 只剩"接 state / 调 LLM / 解析 dispatch" 3 步.
#![allow(clippy::result_large_err)]

/// R30 硬编码 `MAX_TOOL_TURNS = 3` 的中央化常量 (TUI 行为兼容, 不漂移)
pub const DEFAULT_MAX_TOOL_TURNS: usize = 3;

/// LangGraph 风格: tool loop 的 state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolLoopState {
    /// 当前 LLM input (多轮会被自动改成 "继续基于工具结果回答")
    pub input: String,
    /// 历史 messages (role + content)
    pub history: Vec<ToolLoopMessage>,
    /// 上一轮 LLM 完整 reply (空 = 首轮还没跑)
    pub last_reply: String,
    /// 已跑轮数 (从 0 开始, 跑一次 step 加 1)
    pub turn: usize,
    /// 最大轮数 (LangGraph recursion_limit 借鉴)
    pub max_turns: usize,
    /// 错误信息 (有 = 立即停)
    pub error: Option<String>,
}

impl ToolLoopState {
    /// 构造初始 state
    pub fn new(input: impl Into<String>, history: Vec<ToolLoopMessage>, max_turns: usize) -> Self {
        Self {
            input: input.into(),
            history,
            last_reply: String::new(),
            turn: 0,
            max_turns,
            error: None,
        }
    }
}

/// 单条历史消息 (role + content, 通用版, 不绑 TUI ChatMessage)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolLoopMessage {
    pub role: String,
    pub content: String,
}

impl ToolLoopMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".to_string(), content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".to_string(), content: content.into() }
    }
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".to_string(), content: content.into() }
    }
}

/// LangGraph 条件边: 看 state 决定下一节点.
pub fn should_continue(state: &ToolLoopState) -> bool {
    // 错误停
    if state.error.is_some() {
        return false;
    }
    // 首轮还没跑 (last_reply 空) — 继续
    if state.last_reply.is_empty() {
        return true;
    }
    // 超上限停
    if state.turn >= state.max_turns {
        return false;
    }
    // 有 tool call 继续, 无停
    state.last_reply.contains("<<<[TOOL_REQUEST]>>>")
}

/// LLM step 返回 — 由调用方注入
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmStepResult {
    /// LLM 完整 reply 文本
    pub reply_text: String,
    /// 这一轮里是否真有 tool call (有 → 拼进 history 进下一轮)
    pub has_tool_call: bool,
    /// tool dispatch 结果文本 (会拼到 history 末尾, R30 风格)
    pub tool_results: String,
    /// 错误 (有 → should_continue 立刻返 false, run_tool_loop 停)
    pub error: Option<String>,
}

impl LlmStepResult {
    /// 便捷: 没 tool call 的最终答复
    pub fn final_answer(reply: impl Into<String>) -> Self {
        Self {
            reply_text: reply.into(),
            has_tool_call: false,
            tool_results: String::new(),
            error: None,
        }
    }
    /// 便捷: LLM 错
    pub fn err(err: impl Into<String>, fallback_reply: impl Into<String>) -> Self {
        Self {
            reply_text: fallback_reply.into(),
            has_tool_call: false,
            tool_results: String::new(),
            error: Some(err.into()),
        }
    }
    /// 便捷: 有 tool call, 结果文本
    pub fn with_tool_call(reply: impl Into<String>, results: impl Into<String>) -> Self {
        Self {
            reply_text: reply.into(),
            has_tool_call: true,
            tool_results: results.into(),
            error: None,
        }
    }
}

/// 跑 tool loop state machine. 循环终止条件 = `should_continue` 返 false.
pub fn run_tool_loop<F>(mut state: ToolLoopState, mut step: F) -> ToolLoopState
where
    F: FnMut(&mut ToolLoopState) -> LlmStepResult,
{
    loop {
        if !should_continue(&state) {
            break;
        }
        let result = step(&mut state);
        state.error = result.error;
        state.last_reply = result.reply_text.clone();
        state.turn += 1;
        if result.has_tool_call && !result.tool_results.is_empty() {
            // R30 风格: 拼 "assistant 原文 + tool 结果" 进 history
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

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod tool_loop_tests {
    use super::*;

    /// should_continue: 首轮空 last_reply → 继续
    #[test]
    fn should_continue_first_turn_empty_reply_returns_true() {
        let s = ToolLoopState::new("hi", vec![], DEFAULT_MAX_TOOL_TURNS);
        assert!(should_continue(&s));
    }

    /// should_continue: 有 tool call → 继续
    #[test]
    fn should_continue_tool_call_returns_true() {
        let mut s = ToolLoopState::new("hi", vec![], DEFAULT_MAX_TOOL_TURNS);
        s.last_reply = "好的, 我查一下\n<<<[TOOL_REQUEST]>>>\ntool_name: <<<search>>>\n<<<[END_TOOL_REQUEST]>>>".to_string();
        s.turn = 1;
        assert!(should_continue(&s));
    }

    /// should_continue: 没 tool call (纯文本) → 停
    #[test]
    fn should_continue_no_tool_call_returns_false() {
        let mut s = ToolLoopState::new("hi", vec![], DEFAULT_MAX_TOOL_TURNS);
        s.last_reply = "这是答案, 没工具.".to_string();
        s.turn = 1;
        assert!(!should_continue(&s));
    }

    /// should_continue: 超 max_turns → 停
    #[test]
    fn should_continue_exceeds_max_turns_returns_false() {
        let mut s = ToolLoopState::new("hi", vec![], DEFAULT_MAX_TOOL_TURNS);
        s.last_reply = "<<<[TOOL_REQUEST]>>>tool: <<<x>>>".to_string();
        s.turn = DEFAULT_MAX_TOOL_TURNS; // 3 == 3, 条件 turn >= max → 停
        assert!(!should_continue(&s));
    }

    /// should_continue: error 置位 → 停
    #[test]
    fn should_continue_error_set_returns_false() {
        let mut s = ToolLoopState::new("hi", vec![], DEFAULT_MAX_TOOL_TURNS);
        s.last_reply = "anything".to_string();
        s.turn = 0;
        s.error = Some("LLM 503".to_string());
        assert!(!should_continue(&s));
    }

    /// run_tool_loop: 2 轮, 第一轮 tool, 第二轮 final — 跑完
    #[test]
    fn run_tool_loop_two_turns_tool_then_final() {
        let state = ToolLoopState::new("搜下天气", vec![], DEFAULT_MAX_TOOL_TURNS);
        let final_state = run_tool_loop(state, |s| {
            if s.turn == 0 {
                // 第一轮: 假 LLM 返 tool call
                s.history.push(ToolLoopMessage::user("搜下天气"));
                LlmStepResult::with_tool_call(
                    "我帮你查\n<<<[TOOL_REQUEST]>>>\ntool_name: <<<weather>>>\n<<<[END_TOOL_REQUEST]>>>",
                    "[weather OK]\n晴天 25 度\n---",
                )
            } else {
                // 第二轮: 假 LLM 返最终答复
                LlmStepResult::final_answer("今天晴天 25 度")
            }
        });
        assert_eq!(final_state.turn, 2);
        assert_eq!(final_state.last_reply, "今天晴天 25 度");
        assert!(final_state.error.is_none());
        // history 应该多 1 条 (tool 结果)
        assert!(final_state.history.iter().any(|m| m.content.contains("工具调用结果")));
    }

    /// run_tool_loop: LLM 首轮就错 → 立即停, 不继续
    #[test]
    fn run_tool_loop_error_immediately_stops() {
        let state = ToolLoopState::new("hi", vec![], DEFAULT_MAX_TOOL_TURNS);
        let final_state = run_tool_loop(state, |_| {
            LlmStepResult::err("network 503", "(LLM 调用失败: network 503)")
        });
        assert_eq!(final_state.turn, 1);
        assert!(final_state.error.is_some());
        assert!(final_state.last_reply.contains("LLM 调用失败"));
    }

    /// run_tool_loop: 0 轮 (空 LLM) — 不应崩, 直接返
    #[test]
    fn run_tool_loop_zero_max_turns_first_step_runs_then_stops() {
        let state = ToolLoopState::new("hi", vec![], 0);
        let final_state = run_tool_loop(state, |_s| {
            // 第一轮: last_reply 空, should_continue 返 true (因为首轮没跑)
            // step 跑完后 turn=1 >= max=0, 下次 should_continue 停
            LlmStepResult::final_answer("立即停")
        });
        assert_eq!(final_state.turn, 1);
        assert_eq!(final_state.last_reply, "立即停");
    }

    /// LlmStepResult 工厂方法正确性
    #[test]
    fn llm_step_result_constructors() {
        let f = LlmStepResult::final_answer("ok");
        assert!(!f.has_tool_call);
        assert!(f.error.is_none());

        let e = LlmStepResult::err("boom", "fallback");
        assert!(e.error.is_some());
        assert_eq!(e.reply_text, "fallback");

        let t = LlmStepResult::with_tool_call("call", "results");
        assert!(t.has_tool_call);
        assert_eq!(t.tool_results, "results");
    }

    /// ToolLoopMessage 工厂: user/assistant/system
    #[test]
    fn tool_loop_message_roles() {
        assert_eq!(ToolLoopMessage::user("u").role, "user");
        assert_eq!(ToolLoopMessage::assistant("a").role, "assistant");
        assert_eq!(ToolLoopMessage::system("s").role, "system");
    }

    /// DEFAULT_MAX_TOOL_TURNS = 3 (R30 兼容)
    #[test]
    fn default_max_turns_is_3_for_r30_compat() {
        assert_eq!(DEFAULT_MAX_TOOL_TURNS, 3);
    }
}
