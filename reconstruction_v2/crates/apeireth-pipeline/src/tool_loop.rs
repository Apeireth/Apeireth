//! Tool loop (LangGraph state machine + conditional edge).

pub const DEFAULT_MAX_TOOL_TURNS: usize = 5;

#[derive(Debug, Clone)]
pub struct ToolLoopMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct ToolLoopState {
    pub messages: Vec<ToolLoopMessage>,
    pub tool_calls_made: usize,
    pub max_turns: usize,
}

#[derive(Debug, Clone)]
pub enum LlmStepResult {
    Text(String),
    ToolCalls(Vec<ToolCall>),
    Final(String),
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Decide whether the tool loop should continue.
pub fn should_continue(state: &ToolLoopState) -> bool {
    state.tool_calls_made < state.max_turns
}

/// Run a tool loop with a step function.
pub async fn run_tool_loop<F, Fut>(
    mut state: ToolLoopState,
    mut step_fn: F,
) -> ToolLoopState
where
    F: FnMut(ToolLoopState) -> Fut,
    Fut: std::future::Future<Output = (ToolLoopState, LlmStepResult)>,
{
    loop {
        if !should_continue(&state) {
            break;
        }
        let s = state.clone();
        let (new_state, result) = step_fn(s).await;
        state = new_state;
        match result {
            LlmStepResult::Text(t) => {
                state.messages.push(ToolLoopMessage { role: "assistant".into(), content: t });
            }
            LlmStepResult::Final(t) => {
                state.messages.push(ToolLoopMessage { role: "assistant".into(), content: t });
                break;
            }
            LlmStepResult::ToolCalls(calls) => {
                state.tool_calls_made += calls.len();
                for c in calls {
                    state.messages.push(ToolLoopMessage {
                        role: "tool".into(),
                        content: format!("{}: {}", c.name, c.arguments),
                    });
                }
            }
        }
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_continue_default() {
        let s = ToolLoopState { max_turns: 3, ..Default::default() };
        assert!(should_continue(&s));
    }

    #[test]
    fn should_continue_after_max() {
        let s = ToolLoopState { max_turns: 1, tool_calls_made: 2, messages: vec![] };
        assert!(!should_continue(&s));
    }

    #[tokio::test]
    async fn run_loop_final_after_1_turn() {
        let s = ToolLoopState { max_turns: 5, ..Default::default() };
        let final_state = run_tool_loop(s, |_| async {
            (ToolLoopState::default(), LlmStepResult::Final("done".into()))
        }).await;
        assert_eq!(final_state.messages.last().unwrap().content, "done");
    }

    #[tokio::test]
    async fn run_loop_terminates_after_max() {
        let s = ToolLoopState { max_turns: 2, ..Default::default() };
        let mut count = 0;
        let final_state = run_tool_loop(s, move |st| {
            count += 1;
            async move {
                (ToolLoopState { max_turns: st.max_turns, tool_calls_made: st.tool_calls_made + 1, ..Default::default() },
                 LlmStepResult::Text(format!("turn {count}")))
            }
        }).await;
        assert_eq!(final_state.tool_calls_made, 2);
    }

    #[test]
    fn constants() {
        assert_eq!(DEFAULT_MAX_TOOL_TURNS, 5);
    }
}
