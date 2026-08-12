//! B2: Tauri (desktop) 端 tool_loop 适配 (per R32-2 + R35 follow-up)
//!
//! 跟 web 端同款, 0 重写循环控制, 复用 `apeireth_pipeline::tool_loop` 状态机.
//!
//! **R19 战役待实现**: 真 Tauri 集成后, 替换 `call_llm_fn` 为真 LLM 调用.
//! 当前 stub 阶段 (R19_DESKTOP_STUB = true), 适配器先落地, 等 R19 战役时再接真 LLM.

#![allow(dead_code)]

use apeireth_pipeline::tool_loop::{
    run_tool_loop, LlmStepResult, ToolLoopMessage, ToolLoopState, DEFAULT_MAX_TOOL_TURNS,
};

/// B2: Tauri/desktop 端 chat_with_tool_loop 适配.
pub fn chat_with_tool_loop_desktop<L, P>(
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
        .map(|(r, c)| ToolLoopMessage { role: r.clone(), content: c.clone() })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_with_tool_loop_desktop_no_tool_call() {
        let reply = chat_with_tool_loop_desktop(
            "hi",
            &[],
            |_i, _h| "hello from desktop".to_string(),
            |_r| (vec![], String::new()),
        );
        assert_eq!(reply, "hello from desktop");
    }

    #[test]
    fn chat_with_tool_loop_desktop_with_tool_call() {
        let mut turn = 0;
        let reply = chat_with_tool_loop_desktop(
            "call tool",
            &[],
            |_i, _h| {
                turn += 1;
                if turn == 1 {
                    "<<<[TOOL_REQUEST]>>>\ntool_name: <<<ping>>>\n<<<[END_TOOL_REQUEST]>>>".to_string()
                } else {
                    "pong".to_string()
                }
            },
            |r| {
                if r.contains("<<<[TOOL_REQUEST]>>>") {
                    (vec!["ping".to_string()], "ping: pong".to_string())
                } else {
                    (vec![], String::new())
                }
            },
        );
        assert_eq!(reply, "pong");
        assert_eq!(turn, 2);
    }
}