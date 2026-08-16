//! B2: Web 端 tool_loop 适配 (per R32-2 + R35 follow-up)
//!
//! **复用 TUI 同模式**: 把 TUI `chat_with_tool_loop_streaming` 的循环控制权交
//! `apeireth_pipeline::tool_loop::run_tool_loop`, web 端只注入"调 LLM + 解析 dispatch" 2 步.
//!
//! **不漂移 (主哲学锚 #1)**: Web 端不重写循环状态机, 跟 TUI 共享同一份 `should_continue`
//! 规则 (错误停 / 超上限停 / 有 tool call 继续 / 无 tool call 停), 0 业务漂移.
//!
//! **集成点**:
//! - LLM 调用: web 端走 `apeireth-api::llm::*` (AnthropicCompatible + ApeirethApi 协议)
//! - Tool dispatch: 跟 TUI 同协议 `<<<[TOOL_REQUEST]>>> / <<<[END_TOOL_REQUEST]>>>`
//!   (复用 `apeireth-tui::backend::parse_and_dispatch_tools` 或在 web 端平移实现)
//!
//! **R25 报告节奏**: 跟 TUI 同款 1.1 升级, 0 改 TUI 行为, 0 改 pipeline.tool_loop.

#![allow(dead_code)]

use apeireth_pipeline::tool_loop::{
    run_tool_loop, LlmStepResult, ToolLoopMessage, ToolLoopState, DEFAULT_MAX_TOOL_TURNS,
};

/// B2: Web 端 chat_with_tool_loop 适配.
//
// 输入: 用户 input + history (Vec<(role, content)>)
// 输出: LLM 最终 reply 文本
// 内部: run_tool_loop 状态机驱动, 每轮调 `call_llm_fn` 拿 reply, 然后 `parse_and_dispatch_fn` 抽 tool call.
//
// 0 重写循环控制, 0 改 TUI 行为.
pub fn chat_with_tool_loop_web<L, P>(
    input: &str,
    history: &[(String, String)],
    mut call_llm_fn: L,
    mut parse_and_dispatch_fn: P,
) -> String
where
    L: FnMut(&str, &[(String, String)]) -> String,
    P: FnMut(&str) -> (Vec<String>, String),
{
    let initial_history: Vec<ToolLoopMessage> = history
        .iter()
        .map(|(r, c)| ToolLoopMessage {
            role: r.clone(),
            content: c.clone(),
        })
        .collect();
    let state = ToolLoopState::new(input, initial_history, DEFAULT_MAX_TOOL_TURNS);
    let final_state = run_tool_loop(state, |s| {
        let history_pairs: Vec<(String, String)> = s
            .history
            .iter()
            .map(|m| (m.role.clone(), m.content.clone()))
            .collect();
        let reply_text = call_llm_fn(&s.input, &history_pairs);
        let (_names, results) = parse_and_dispatch_fn(&reply_text);
        if results.is_empty() {
            LlmStepResult::final_answer(reply_text)
        } else {
            LlmStepResult::with_tool_call(reply_text, results)
        }
    });
    final_state.last_reply
}

// ============================================================================
// §Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: 无 tool call 直接 LLM 答案 → 1 轮停.
    #[test]
    fn chat_with_tool_loop_web_no_tool_call_returns_immediate() {
        let reply = chat_with_tool_loop_web(
            "hi",
            &[],
            |_input, _h| "hi there!".to_string(),
            |_reply| (vec![], String::new()),
        );
        assert_eq!(reply, "hi there!");
    }

    /// 守门 #2: 有 tool call → 2 轮, 第 2 轮 LLM 给最终答案.
    #[test]
    fn chat_with_tool_loop_web_with_tool_call_two_turns() {
        let mut turn = 0;
        let reply = chat_with_tool_loop_web(
            "call tool",
            &[],
            |_input, _h| {
                turn += 1;
                if turn == 1 {
                    "<<<[TOOL_REQUEST]>>>\ntool_name: <<<echo>>>\nmsg: <<<hi>>>\n<<<[END_TOOL_REQUEST]>>>".to_string()
                } else {
                    "tool result processed: hi".to_string()
                }
            },
            |reply| {
                if reply.contains("<<<[TOOL_REQUEST]>>>") {
                    (vec!["echo".to_string()], "echo: hi".to_string())
                } else {
                    (vec![], String::new())
                }
            },
        );
        assert_eq!(reply, "tool result processed: hi");
        assert_eq!(turn, 2);
    }

    /// 守门 #3: MAX_TOOL_TURNS 上限守门 (3 轮) — 防止 tool call 死循环.
    #[test]
    fn chat_with_tool_loop_web_max_turns_caps_loop() {
        let mut turn_count = 0;
        let _reply = chat_with_tool_loop_web(
            "loop forever",
            &[],
            |_input, _h| {
                turn_count += 1;
                "<<<[TOOL_REQUEST]>>>\ntool_name: <<<n>>>\n<<<[END_TOOL_REQUEST]>>>".to_string()
            },
            |_reply| (vec!["n".to_string()], "loop".to_string()),
        );
        // 0 改 DEFAULT_MAX_TOOL_TURNS = 3 行为: 3 轮后停
        assert!(
            turn_count <= DEFAULT_MAX_TOOL_TURNS,
            "should cap at MAX_TOOL_TURNS"
        );
    }
}
