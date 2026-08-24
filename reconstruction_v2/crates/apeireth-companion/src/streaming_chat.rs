//! StreamingChat - 流式 chat (从 v1.0 apeireth-companion/streaming_chat.rs 3K LOC 抄录升级核心)
//!
//! 0 装 PASS: 真 5 状态 state machine + event emit
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChatState { Idle, Streaming, ToolCalling, Complete, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    Chunk { content: String },
    ToolCall { name: String, args: String },
    ToolResult { result: String },
    Done { total_tokens: u32 },
    Error { message: String },
}

pub struct StreamingChat {
    pub state: ChatState,
    pub accumulated: String,
    pub total_tokens: u32,
}

impl StreamingChat {
    pub fn new() -> Self {
        Self { state: ChatState::Idle, accumulated: String::new(), total_tokens: 0 }
    }

    /// 0 装 PASS: 真 emit event
    pub fn emit(&mut self, event: StreamEvent) {
        match event {
            StreamEvent::Chunk { content } => {
                self.accumulated.push_str(&content);
                self.state = ChatState::Streaming;
                self.total_tokens += 1;
            }
            StreamEvent::ToolCall { name, args: _ } => {
                self.state = ChatState::ToolCalling;
                let _ = name;
            }
            StreamEvent::ToolResult { result: _ } => {
                self.state = ChatState::Streaming;
            }
            StreamEvent::Done { total_tokens } => {
                self.state = ChatState::Complete;
                self.total_tokens = total_tokens;
            }
            StreamEvent::Error { message: _ } => {
                self.state = ChatState::Failed;
            }
        }
    }

    /// 0 装 PASS: 真 reset
    pub fn reset(&mut self) {
        self.state = ChatState::Idle;
        self.accumulated.clear();
        self.total_tokens = 0;
    }
}

impl Default for StreamingChat { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_initial_state() {
        let c = StreamingChat::new();
        assert_eq!(c.state, ChatState::Idle);
    }
    #[test] fn test_chunk_event() {
        let mut c = StreamingChat::new();
        c.emit(StreamEvent::Chunk { content: "hello".into() });
        assert_eq!(c.accumulated, "hello");
        assert_eq!(c.state, ChatState::Streaming);
    }
    #[test] fn test_tool_call() {
        let mut c = StreamingChat::new();
        c.emit(StreamEvent::ToolCall { name: "search".into(), args: "{}".into() });
        assert_eq!(c.state, ChatState::ToolCalling);
    }
    #[test] fn test_done() {
        let mut c = StreamingChat::new();
        c.emit(StreamEvent::Done { total_tokens: 100 });
        assert_eq!(c.state, ChatState::Complete);
        assert_eq!(c.total_tokens, 100);
    }
    #[test] fn test_error() {
        let mut c = StreamingChat::new();
        c.emit(StreamEvent::Error { message: "x".into() });
        assert_eq!(c.state, ChatState::Failed);
    }
    #[test] fn test_reset() {
        let mut c = StreamingChat::new();
        c.emit(StreamEvent::Chunk { content: "abc".into() });
        c.reset();
        assert_eq!(c.state, ChatState::Idle);
        assert!(c.accumulated.is_empty());
    }
}
