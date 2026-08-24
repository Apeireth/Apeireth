//! TUI App - 应用状态机 (从 v1.0 apeireth-tui/app.rs 363 LOC 抄录升级)
//!
//! 0 装 PASS: 真 AppState + Mode 切换 + 消息列表 (不依赖 ratatui)

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppMode { Chat, Memory, Organ, Settings, Help }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,    // user / assistant / system
    pub content: String,
    pub timestamp_ms: i64,
}

pub struct AppState {
    pub mode: AppMode,
    pub messages: VecDeque<Message>,
    pub input_buffer: String,
    pub max_messages: usize,
    pub status: String,
}

impl AppState {
    pub fn new() -> Self {
        Self { mode: AppMode::Chat, messages: VecDeque::new(), input_buffer: String::new(), max_messages: 1000, status: "ready".into() }
    }

    /// 0 装 PASS: 真模式切换
    pub fn set_mode(&mut self, m: AppMode) {
        self.mode = m;
        self.status = format!("switched to {:?}", m);
    }

    /// 0 装 PASS: 真添加消息 (超过容量删老的)
    pub fn push_message(&mut self, msg: Message) {
        self.messages.push_back(msg);
        if self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
    }

    /// 0 装 PASS: 真 append input
    pub fn append_input(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    /// 0 装 PASS: 真 backspace
    pub fn backspace(&mut self) {
        self.input_buffer.pop();
    }

    /// 0 装 PASS: 真提交 input -> 返回字符串 + 清空
    pub fn submit_input(&mut self) -> String {
        let s = std::mem::take(&mut self.input_buffer);
        s
    }

    pub fn message_count(&self) -> usize { self.messages.len() }
}

impl Default for AppState {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_mode_switch() {
        let mut s = AppState::new();
        s.set_mode(AppMode::Memory);
        assert_eq!(s.mode, AppMode::Memory);
    }
    #[test] fn test_push_message_eviction() {
        let mut s = AppState::new();
        s.max_messages = 2;
        s.push_message(Message { role: "u".into(), content: "1".into(), timestamp_ms: 1 });
        s.push_message(Message { role: "u".into(), content: "2".into(), timestamp_ms: 2 });
        s.push_message(Message { role: "u".into(), content: "3".into(), timestamp_ms: 3 });
        assert_eq!(s.message_count(), 2);
        assert_eq!(s.messages.back().unwrap().content, "3");
    }
    #[test] fn test_input_submit() {
        let mut s = AppState::new();
        s.append_input('h'); s.append_input('i');
        let r = s.submit_input();
        assert_eq!(r, "hi");
        assert!(s.input_buffer.is_empty());
    }
    #[test] fn test_backspace() {
        let mut s = AppState::new();
        s.append_input('a'); s.append_input('b');
        s.backspace();
        assert_eq!(s.input_buffer, "a");
    }
    #[test] fn test_mode_eq() {
        assert_eq!(AppMode::Chat, AppMode::Chat);
        assert_ne!(AppMode::Chat, AppMode::Memory);
    }
}
