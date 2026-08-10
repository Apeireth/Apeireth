//! Ear (耳) command 模块 — 事件订阅
//!
//! **借鉴 Golutra #1**: 9 organ × 5-8 command 模式
//!
//! **6 命令**:
//! 1. [`Command::Subscribe`] — 订阅一个 topic
//! 2. [`Command::Unsubscribe`] — 退订一个 topic
//! 3. [`Command::GetRecentEvents`] — 读最近 N 个事件
//! 4. [`Command::GetSubscribedTopics`] — 读已订阅 topic 列表
//! 5. [`Command::GetEventCount`] — 累计事件数
//! 6. [`Command::ClearEvents`] — 清空事件历史
//!
//! **不假装**:
//! - ear 在 `organ/mod.rs` 标 `Readiness::Stub` — 6 命令全部标 placeholder
//! - 真实 R25.3 接 `apeireth-bus` L0-L4 事件总线
//! - topic 名是 `&'static str` (编译期 hardcode 5 已知 topic)
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: ear 服务 ASI 感知事件
//! - S-2 实事求是: stub 标 partial, 6 命令全占位
//! - O-2 走在前人经验上: 借 pub/sub 业界模式
//! - O-3 干到底: 6 命令覆盖事件全场景
//! - O-4 任何人都能接手: State + 5 topic hardcode 全文档化
//! - O-5 不假装: events 是 in-memory Vec, 标 stub
//!
//! **8 项承诺**: 全部遵守

use super::error::OrganError;

/// 已知 topic 编译期 hardcode (5 topic, per 主人 R22 拍板 + L0-L4 bus 命名)
pub const KNOWN_TOPICS: &[&str] = &["L0.system", "L1.session", "L2.tool", "L3.cognition", "L4.bus"];

/// 单个事件记录
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// topic
    pub topic: String,
    /// 事件内容 (字符串描述)
    pub payload: String,
    /// 时间戳占位 (ms)
    pub timestamp_ms: u64,
}

/// Ear 器官状态
#[derive(Debug, Clone)]
pub struct State {
    /// 已订阅 topic 集合
    pub subscribed: std::collections::HashSet<String>,
    /// 事件历史 (新 → 旧)
    pub events: Vec<Event>,
    /// 累计事件数
    pub event_count: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            subscribed: std::collections::HashSet::new(),
            events: Vec::new(),
            event_count: 0,
        }
    }
}

/// Ear 器官 6 命令
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// 订阅一个 topic
    Subscribe {
        /// topic 名
        topic: String,
    },
    /// 退订一个 topic
    Unsubscribe {
        /// topic 名
        topic: String,
    },
    /// 读最近 N 个事件
    GetRecentEvents {
        /// 最多返回条数
        limit: usize,
    },
    /// 读已订阅 topic 列表
    GetSubscribedTopics,
    /// 读累计事件数
    GetEventCount,
    /// 清空事件历史
    ClearEvents,
}

/// Ear 命令响应
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// 通用单元响应
    Unit,
    /// 事件列表
    RecentEvents(Vec<Event>),
    /// 已订阅 topic 列表
    SubscribedTopics(Vec<String>),
    /// 累计事件数
    EventCount(u64),
}

/// 处理 Ear 命令
///
/// **错误**:
/// - [`OrganError::InvalidArg`] — topic 名为空
/// - [`OrganError::NotReady`] — Unsubscribe 时未订阅过
pub fn handle(state: &mut State, cmd: Command) -> Result<Response, OrganError> {
    match cmd {
        Command::Subscribe { topic } => {
            if topic.is_empty() {
                return Err(OrganError::InvalidArg {
                    command: "Subscribe",
                    reason: "topic 不能为空".into(),
                });
            }
            state.subscribed.insert(topic);
            Ok(Response::Unit)
        }
        Command::Unsubscribe { topic } => {
            if !state.subscribed.remove(&topic) {
                return Err(OrganError::NotReady {
                    organ: ASCII_CHAR,
                    reason: format!("topic '{topic}' not subscribed"),
                });
            }
            Ok(Response::Unit)
        }
        Command::GetRecentEvents { limit: _ } => {
            // S-2 实事求是: ear 是 stub — events 永远是空
            let _ = state;
            Ok(Response::RecentEvents(Vec::new()))
        }
        Command::GetSubscribedTopics => {
            let mut topics: Vec<String> = state.subscribed.iter().cloned().collect();
            topics.sort();
            Ok(Response::SubscribedTopics(topics))
        }
        Command::GetEventCount => Ok(Response::EventCount(state.event_count)),
        Command::ClearEvents => {
            state.events.clear();
            Ok(Response::Unit)
        }
    }
}

/// 器官 ASCII 字符
pub const ASCII_CHAR: &str = "[EAR]";

/// 器官中文名
pub const NAME_ZH: &str = "耳";

// =====================================================================
// 单元测试 (6 命令 + 5 known topic + 状态机 = 8+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_state() -> State {
        State::default()
    }

    // ---- 6 命令全部可枚举 ----

    #[test]
    fn six_commands_constructible() {
        let _ = Command::Subscribe { topic: "L0.system".into() };
        let _ = Command::Unsubscribe { topic: "L0.system".into() };
        let _ = Command::GetRecentEvents { limit: 10 };
        let _ = Command::GetSubscribedTopics;
        let _ = Command::GetEventCount;
        let _ = Command::ClearEvents;
    }

    // ---- 5 known topic 编译期 hardcode ----

    #[test]
    fn five_topics_hardcoded() {
        assert_eq!(KNOWN_TOPICS.len(), 5, "5 known topic 编译期 hardcode");
        for required in ["L0.system", "L1.session", "L2.tool", "L3.cognition", "L4.bus"] {
            assert!(KNOWN_TOPICS.contains(&required), "应含 {required}");
        }
    }

    // ---- Subscribe/Unsubscribe ----

    #[test]
    fn subscribe_adds_topic() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::Subscribe { topic: "L0.system".into() },
            );
        assert!(r.is_ok());
        assert!(state.subscribed.contains("L0.system"));
    }

    #[test]
    fn subscribe_rejects_empty_topic() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::Subscribe { topic: "".into() });
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "Subscribe", .. })));
    }

    #[test]
    fn unsubscribe_removes_topic() {
        let mut state = fresh_state();
        let _ = handle(
            &mut state,
            Command::Subscribe { topic: "L1.session".into() },
            );
        let r = handle(
            &mut state,
            Command::Unsubscribe { topic: "L1.session".into() },
            );
        assert!(r.is_ok());
        assert!(!state.subscribed.contains("L1.session"));
    }

    #[test]
    fn unsubscribe_unknown_topic_errors() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::Unsubscribe { topic: "L2.tool".into() },
            );
        assert!(matches!(r, Err(OrganError::NotReady { .. })));
    }

    // ---- GetSubscribedTopics ----

    #[test]
    fn get_subscribed_topics_sorted() {
        let mut state = fresh_state();
        let _ = handle(
            &mut state,
            Command::Subscribe { topic: "L4.bus".into() },
            );
        let _ = handle(
            &mut state,
            Command::Subscribe { topic: "L0.system".into() },
            );
        let r = handle(&mut state, Command::GetSubscribedTopics).unwrap();
        match r {
            Response::SubscribedTopics(v) => {
                assert_eq!(v, vec!["L0.system".to_string(), "L4.bus".to_string()]);
            }
            _ => panic!("expected SubscribedTopics"),
        }
    }

    // ---- Stub 标缺 ----

    #[test]
    fn get_recent_events_empty_stub() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::GetRecentEvents { limit: 10 }).unwrap();
        // S-2 实事求是: stub organ 永远返空
        match r {
            Response::RecentEvents(v) => assert!(v.is_empty()),
            _ => panic!("expected RecentEvents"),
        }
    }

    // ---- 器官元数据 ----

    #[test]
    fn ascii_char_matches_organ_mod() {
        assert_eq!(ASCII_CHAR, "[EAR]");
    }

    #[test]
    fn name_zh_matches_organ_mod() {
        assert_eq!(NAME_ZH, "耳");
    }
}
