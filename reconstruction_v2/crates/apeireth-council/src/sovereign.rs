//! Sovereignty Hook 接口 — 智囊团事件 → 主权仲裁器 (v2 自洽)
//!
//! **设计** (对齐 v1 sovereignty.rs intent):
//! - `CouncilEvent` 枚举: DeliberationStarted / OpinionIssued / HoldTriggered / SovereigntyAdjudicated / DeliberationCompleted
//! - `SovereigntyHook` trait: `on_council_event(&mut self, event) -> Ack` — 监听 council 事件流
//! - `NoopSovereigntyHook`: 默认实现 (不做事, 仅 ack)
//! - `BroadcastHook`: 收事件到 Vec (in-memory 测试 / 单元测试 / 多监听器扇出)
//! - `Ack` 枚举: Acked / Throttled / Rejected(reason)
//!
//! **不抄 v1 FFI/HTTP/SQL**: trait + 内存 Vec 即可.

use crate::persona::BondCharacter;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 智囊团事件 (5 类型).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CouncilEvent {
    /// 审议开始
    DeliberationStarted {
        session_id: String,
        query_id: String,
        started_at_ms: i64,
    },
    /// 单个 opinion 产出
    OpinionIssued {
        session_id: String,
        opinion_id: String,
        author_id: String,
        author_character: BondCharacter,
        /// -1.0..=+1.0
        stance_score: f64,
        /// 0.0..=1.0
        confidence: f64,
    },
    /// 按住触发 (智囊要求主权限暂停决策)
    HoldTriggered {
        session_id: String,
        reason: String,
        triggered_at_ms: i64,
    },
    /// 主权仲裁结果 (按住解除)
    SovereigntyAdjudicated {
        session_id: String,
        decision: String,
        adjudicated_at_ms: i64,
    },
    /// 审议完成
    DeliberationCompleted {
        session_id: String,
        verdict: String,
        completed_at_ms: i64,
    },
}

/// Hook 确认返回.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Ack {
    /// 已被接收
    Acked,
    /// 被节流 (频率过高)
    Throttled,
    /// 被拒绝 (含原因)
    Rejected(String),
}

/// Sovereignty Hook trait — 监听 council 事件流.
pub trait SovereigntyHook: Send {
    /// 收到一个事件, 返回 ack.
    fn on_council_event(&mut self, event: &CouncilEvent) -> Ack;
    /// hook 名字 (用于日志 / 多 hook 路由).
    fn name(&self) -> &str;
}

/// 默认 no-op 实现.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopSovereigntyHook;

impl NoopSovereigntyHook {
    pub fn new() -> Self {
        Self
    }
}

impl SovereigntyHook for NoopSovereigntyHook {
    fn on_council_event(&mut self, _event: &CouncilEvent) -> Ack {
        Ack::Acked
    }
    fn name(&self) -> &str {
        "noop"
    }
}

/// Broadcast hook — 收事件到 in-memory Vec (用于测试 / 调试 / 扇出).
#[derive(Debug, Default, Clone)]
pub struct BroadcastHook {
    events: Vec<CouncilEvent>,
    capacity: usize,
}

impl BroadcastHook {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: Vec::new(),
            capacity,
        }
    }

    /// 取所有事件副本.
    pub fn events(&self) -> Vec<CouncilEvent> {
        self.events.clone()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl SovereigntyHook for BroadcastHook {
    fn on_council_event(&mut self, event: &CouncilEvent) -> Ack {
        if self.events.len() >= self.capacity {
            return Ack::Throttled;
        }
        self.events.push(event.clone());
        Ack::Acked
    }
    fn name(&self) -> &str {
        "broadcast"
    }
}

/// 多 hook 扇出 — 同时调用多个 hook, 任意 Rejected 即视为整体 Rejected.
pub struct FanOutHook {
    hooks: Vec<Box<dyn SovereigntyHook>>,
}

impl fmt::Debug for FanOutHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FanOutHook")
            .field("count", &self.hooks.len())
            .finish()
    }
}

impl FanOutHook {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }
    pub fn push(mut self, hook: Box<dyn SovereigntyHook>) -> Self {
        self.hooks.push(hook);
        self
    }
    pub fn len(&self) -> usize {
        self.hooks.len()
    }
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}

impl Default for FanOutHook {
    fn default() -> Self {
        Self::new()
    }
}

impl SovereigntyHook for FanOutHook {
    fn on_council_event(&mut self, event: &CouncilEvent) -> Ack {
        for h in &mut self.hooks {
            match h.on_council_event(event) {
                Ack::Acked | Ack::Throttled => continue,
                Ack::Rejected(reason) => return Ack::Rejected(reason),
            }
        }
        Ack::Acked
    }
    fn name(&self) -> &str {
        "fanout"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> CouncilEvent {
        CouncilEvent::OpinionIssued {
            session_id: "s1".into(),
            opinion_id: "op1".into(),
            author_id: "safety-001".into(),
            author_character: BondCharacter::Guardian,
            stance_score: 0.5,
            confidence: 0.9,
        }
    }

    #[test]
    fn t01_noop_acks() {
        let mut h = NoopSovereigntyHook::new();
        assert_eq!(h.on_council_event(&sample_event()), Ack::Acked);
        assert_eq!(h.name(), "noop");
    }

    #[test]
    fn t02_broadcast_records_and_capacity_throttles() {
        let mut h = BroadcastHook::new(2);
        assert_eq!(h.on_council_event(&sample_event()), Ack::Acked);
        assert_eq!(h.on_council_event(&sample_event()), Ack::Acked);
        assert_eq!(h.on_council_event(&sample_event()), Ack::Throttled);
        assert_eq!(h.len(), 2);
        assert_eq!(h.events().len(), 2);
    }

    #[test]
    fn t03_broadcast_clear() {
        let mut h = BroadcastHook::new(10);
        h.on_council_event(&sample_event());
        h.clear();
        assert!(h.is_empty());
    }

    #[test]
    fn t04_fanout_rejects_on_first_reject() {
        struct Rejecting;
        impl SovereigntyHook for Rejecting {
            fn on_council_event(&mut self, _: &CouncilEvent) -> Ack {
                Ack::Rejected("test".into())
            }
            fn name(&self) -> &str { "r" }
        }
        let mut fan = FanOutHook::new()
            .push(Box::new(BroadcastHook::new(10)))
            .push(Box::new(Rejecting));
        match fan.on_council_event(&sample_event()) {
            Ack::Rejected(r) => assert_eq!(r, "test"),
            other => panic!("expected Rejected, got {:?}", other),
        }
    }

    #[test]
    fn t05_event_serde_round_trip() {
        let e = CouncilEvent::DeliberationStarted {
            session_id: "s".into(),
            query_id: "q".into(),
            started_at_ms: 42,
        };
        let s = serde_json::to_string(&e).unwrap();
        let d: CouncilEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(e, d);
    }
}
