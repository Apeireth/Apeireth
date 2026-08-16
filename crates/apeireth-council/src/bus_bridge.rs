//! R111: Council deliberation event → bus 真接 (tokio broadcast, 0 改现有 types)
//!
//! **目标**: Council 跑 deliberation 时, 每个事件 (started/round/completed) 自动
//! 发布到 bus, 任意订阅者 (TUI dashboard / MCP / logging) 都能消费.
//!
//! **Apeireth 真接 (本 module)**:
//! - `DeliberationEvent` enum — 3 类事件: Started / RoundCompleted / Completed
//! - `CouncilBusBridge` — 包装 `tokio::sync::broadcast::Sender<DeliberationEvent>`
//!   - `publish_started(member, role, topic)` — 开始
//!   - `publish_round_completed(round, verdicts)` — 一轮结束
//!   - `publish_completed(final_verdict, total_rounds, duration_ms)` — 全部完成
//!   - `subscribe() -> broadcast::Receiver<DeliberationEvent>` — 订阅
//! - `CouncilBusBridge::disabled()` — 空实现 (caller 不需要 bus 时用, 0 panic)
//! - `CouncilBusBridge::default_capacity(cap)` — 自定义容量 (默认 64)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `CouncilMember` / `MultiRoundVerdict` / `StressReport` (R33-4 / R33-4-1 / R68 LOCKED)
//! - 0 改 `deliberation` / `advisor` / `persona` / `mock_llm` / `llm_backend` (0 业务漂移)
//! - 0 引入 apeireth-bus (避免 tonic/prost/async-tungstenite 重 deps, 留 migration path)
//!
//! **借鉴锚 (S-9)**:
//! - tokio::sync::broadcast (L0 bus inproc 范式)
//! - VCP vcptoolbox event hook (任意模块可订阅)
//! - AutoGen GroupChatManager.run_group_chat() 事件流 (start / speaker / message / end)

use std::time::Duration;
use tokio::sync::broadcast;

use crate::council_member::CouncilMember;

// ============================================================
// DeliberationEvent
// ============================================================

/// **Council deliberation 事件** (per spec: started / round_completed / completed)
#[derive(Debug, Clone, PartialEq)]
pub enum DeliberationEvent {
    /// Deliberation 启动
    Started {
        /// Trace ID (链路追踪, 复用 bus::next_trace_id 语义; 这里用 caller 传, 0 强制 bus)
        trace_id: u64,
        /// 启动时间戳 (epoch millis)
        started_at_ms: i64,
        /// 参与的 member 角色列表
        members: Vec<String>,
        /// 议题 (deliberation 的 prompt / 问题)
        topic: String,
    },
    /// 一轮完成
    RoundCompleted {
        /// Trace ID (跟 Started.trace_id 一致, 给关联用)
        trace_id: u64,
        /// Round index (1-based)
        round: u32,
        /// 该 round 持续时间
        duration: Duration,
        /// 该 round 各 member 的 verdict (member role → verdict string, e.g. "approve")
        verdicts: Vec<(String, String)>,
    },
    /// Deliberation 全部完成
    Completed {
        /// Trace ID
        trace_id: u64,
        /// 总 rounds
        total_rounds: u32,
        /// 总 duration
        total_duration: Duration,
        /// 最终 verdict (e.g. "approved" / "rejected" / "needs_more_discussion")
        final_verdict: String,
    },
}

impl DeliberationEvent {
    /// **kind_str** (per event type)
    pub fn kind_str(&self) -> &'static str {
        match self {
            DeliberationEvent::Started { .. } => "started",
            DeliberationEvent::RoundCompleted { .. } => "round_completed",
            DeliberationEvent::Completed { .. } => "completed",
        }
    }

    /// **trace_id** (per event)
    pub fn trace_id(&self) -> u64 {
        match self {
            DeliberationEvent::Started { trace_id, .. }
            | DeliberationEvent::RoundCompleted { trace_id, .. }
            | DeliberationEvent::Completed { trace_id, .. } => *trace_id,
        }
    }
}

// ============================================================
// CouncilBusBridge
// ============================================================

/// **Council deliberation event 总线桥** (tokio broadcast 包装)
///
/// 用法:
/// ```ignore
/// let bridge = CouncilBusBridge::default();
/// let mut rx = bridge.subscribe();
/// tokio::spawn(async move {
///     while let Ok(event) = rx.recv().await {
///         println!("event: {:?}", event);
///     }
/// });
/// bridge.publish_started(1, members, topic);
/// ```
pub struct CouncilBusBridge {
    tx: broadcast::Sender<DeliberationEvent>,
    /// 是否启用 (disabled 模式下 tx 是 dummy, 0 真发, 0 panic)
    enabled: bool,
}

impl std::fmt::Debug for CouncilBusBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CouncilBusBridge")
            .field("enabled", &self.enabled)
            .field("receiver_count", &self.tx.receiver_count())
            .finish()
    }
}

impl Default for CouncilBusBridge {
    fn default() -> Self {
        Self::with_capacity(64)
    }
}

impl CouncilBusBridge {
    /// **默认容量 64**
    pub fn new() -> Self {
        Self::default()
    }

    /// **自定义容量** (broadcast channel 大小)
    pub fn with_capacity(cap: usize) -> Self {
        let (tx, _) = broadcast::channel(cap.max(1));
        Self { tx, enabled: true }
    }

    /// **disabled 模式** — 0 事件真发, 0 panic (caller 不需要 bus 时)
    pub fn disabled() -> Self {
        // 用容量 1 创 dummy tx; 所有 send 都成功 (但 0 receiver, 消息 drop)
        let (tx, _) = broadcast::channel(1);
        Self { tx, enabled: false }
    }

    /// **是否启用**
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// **当前 receiver 数** (订阅者数)
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// **订阅** (caller 拿 Receiver 自己 recv)
    pub fn subscribe(&self) -> broadcast::Receiver<DeliberationEvent> {
        self.tx.subscribe()
    }

    // ----- publish helpers -----

    /// **publish Started 事件** (member list 从 CouncilMember list 抽 role)
    pub fn publish_started(
        &self,
        trace_id: u64,
        members: &[CouncilMember],
        topic: impl Into<String>,
    ) -> usize {
        if !self.enabled {
            return 0;
        }
        let event = DeliberationEvent::Started {
            trace_id,
            started_at_ms: now_ms(),
            members: members.iter().map(|m| m.role.clone()).collect(),
            topic: topic.into(),
        };
        self.send(event)
    }

    /// **publish RoundCompleted 事件** (verdicts 配对: (role, verdict))
    pub fn publish_round_completed(
        &self,
        trace_id: u64,
        round: u32,
        duration: Duration,
        verdicts: &[(String, String)],
    ) -> usize {
        if !self.enabled {
            return 0;
        }
        let event = DeliberationEvent::RoundCompleted {
            trace_id,
            round,
            duration,
            verdicts: verdicts.to_vec(),
        };
        self.send(event)
    }

    /// **publish Completed 事件** (final verdict + 统计)
    pub fn publish_completed(
        &self,
        trace_id: u64,
        total_rounds: u32,
        total_duration: Duration,
        final_verdict: impl Into<String>,
    ) -> usize {
        if !self.enabled {
            return 0;
        }
        let event = DeliberationEvent::Completed {
            trace_id,
            total_rounds,
            total_duration,
            final_verdict: final_verdict.into(),
        };
        self.send(event)
    }

    /// **底层 send** (返 实际收件人数, 0 时 drop 计数)
    fn send(&self, event: DeliberationEvent) -> usize {
        match self.tx.send(event) {
            Ok(n) => n,
            // 0 receiver 是合法的 (caller 主动 drop 了), 静默返回
            Err(_) => 0,
        }
    }
}

// ============================================================
// 工具
// ============================================================

/// **now epoch millis** (避 chrono 依赖)
fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_member(role: &str) -> CouncilMember {
        CouncilMember {
            role: role.to_string(),
            goal: format!("goal of {}", role),
            backstory: format!("backstory of {}", role),
            provider: "test".to_string(),
        }
    }

    #[test]
    fn deliberation_event_kind_str() {
        let s = DeliberationEvent::Started {
            trace_id: 1,
            started_at_ms: 0,
            members: vec![],
            topic: "t".into(),
        };
        let r = DeliberationEvent::RoundCompleted {
            trace_id: 1,
            round: 1,
            duration: Duration::from_secs(1),
            verdicts: vec![],
        };
        let c = DeliberationEvent::Completed {
            trace_id: 1,
            total_rounds: 1,
            total_duration: Duration::from_secs(1),
            final_verdict: "ok".into(),
        };
        assert_eq!(s.kind_str(), "started");
        assert_eq!(r.kind_str(), "round_completed");
        assert_eq!(c.kind_str(), "completed");
    }

    #[test]
    fn deliberation_event_trace_id() {
        let s = DeliberationEvent::Started {
            trace_id: 42,
            started_at_ms: 0,
            members: vec![],
            topic: "t".into(),
        };
        let r = DeliberationEvent::RoundCompleted {
            trace_id: 42,
            round: 1,
            duration: Duration::from_secs(1),
            verdicts: vec![],
        };
        let c = DeliberationEvent::Completed {
            trace_id: 42,
            total_rounds: 1,
            total_duration: Duration::from_secs(1),
            final_verdict: "ok".into(),
        };
        assert_eq!(s.trace_id(), 42);
        assert_eq!(r.trace_id(), 42);
        assert_eq!(c.trace_id(), 42);
    }

    #[test]
    fn bridge_default_creates_enabled() {
        let b = CouncilBusBridge::default();
        assert!(b.is_enabled());
        assert_eq!(b.receiver_count(), 0);
    }

    #[test]
    fn bridge_with_capacity() {
        let b = CouncilBusBridge::with_capacity(128);
        assert!(b.is_enabled());
    }

    #[test]
    fn bridge_disabled_mode() {
        let b = CouncilBusBridge::disabled();
        assert!(!b.is_enabled());
        let n = b.publish_started(1, &[], "topic");
        assert_eq!(n, 0); // 0 真发
    }

    #[test]
    fn publish_started_with_no_receivers_returns_zero() {
        let b = CouncilBusBridge::default();
        let n = b.publish_started(1, &[make_member("architect")], "test topic");
        // 0 receiver: send 返 Err, 我们 return 0
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn publish_started_with_receiver_delivers() {
        let b = CouncilBusBridge::default();
        let mut rx = b.subscribe();
        let members = vec![make_member("architect"), make_member("security_reviewer")];
        let n = b.publish_started(123, &members, "test topic");
        assert_eq!(n, 1);
        let event = rx.recv().await.unwrap();
        match event {
            DeliberationEvent::Started {
                trace_id,
                members,
                topic,
                ..
            } => {
                assert_eq!(trace_id, 123);
                assert_eq!(topic, "test topic");
                assert_eq!(members.len(), 2);
                assert!(members.contains(&"architect".to_string()));
                assert!(members.contains(&"security_reviewer".to_string()));
            }
            _ => panic!("expected Started event"),
        }
    }

    #[tokio::test]
    async fn publish_round_completed_with_receiver_delivers() {
        let b = CouncilBusBridge::default();
        let mut rx = b.subscribe();
        let verdicts = vec![
            ("architect".to_string(), "approve".to_string()),
            ("reviewer".to_string(), "approve".to_string()),
        ];
        let n = b.publish_round_completed(7, 1, Duration::from_millis(500), &verdicts);
        assert_eq!(n, 1);
        let event = rx.recv().await.unwrap();
        match event {
            DeliberationEvent::RoundCompleted {
                trace_id,
                round,
                duration,
                verdicts,
            } => {
                assert_eq!(trace_id, 7);
                assert_eq!(round, 1);
                assert_eq!(duration, Duration::from_millis(500));
                assert_eq!(verdicts.len(), 2);
            }
            _ => panic!("expected RoundCompleted"),
        }
    }

    #[tokio::test]
    async fn publish_completed_with_receiver_delivers() {
        let b = CouncilBusBridge::default();
        let mut rx = b.subscribe();
        let n = b.publish_completed(99, 3, Duration::from_secs(10), "approved");
        assert_eq!(n, 1);
        let event = rx.recv().await.unwrap();
        match event {
            DeliberationEvent::Completed {
                trace_id,
                total_rounds,
                total_duration,
                final_verdict,
            } => {
                assert_eq!(trace_id, 99);
                assert_eq!(total_rounds, 3);
                assert_eq!(total_duration, Duration::from_secs(10));
                assert_eq!(final_verdict, "approved");
            }
            _ => panic!("expected Completed"),
        }
    }

    #[tokio::test]
    async fn multiple_receivers_get_all_events() {
        let b = CouncilBusBridge::default();
        let mut rx1 = b.subscribe();
        let mut rx2 = b.subscribe();
        assert_eq!(b.receiver_count(), 2);

        let n = b.publish_started(1, &[], "t");
        assert_eq!(n, 2); // 2 个 receiver 都收到

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert_eq!(e1.kind_str(), "started");
        assert_eq!(e2.kind_str(), "started");
    }

    #[tokio::test]
    async fn full_deliberation_lifecycle() {
        let b = CouncilBusBridge::default();
        let mut rx = b.subscribe();
        let members = vec![make_member("a"), make_member("b")];

        // 1. Started
        b.publish_started(1, &members, "topic");
        // 2. Round 1
        b.publish_round_completed(
            1,
            1,
            Duration::from_millis(100),
            &[("a".into(), "approve".into())],
        );
        // 3. Round 2
        b.publish_round_completed(
            1,
            2,
            Duration::from_millis(200),
            &[
                ("a".into(), "approve".into()),
                ("b".into(), "approve".into()),
            ],
        );
        // 4. Completed
        b.publish_completed(1, 2, Duration::from_millis(300), "approved");

        let mut events = Vec::new();
        for _ in 0..4 {
            events.push(rx.recv().await.unwrap());
        }
        assert_eq!(events[0].kind_str(), "started");
        assert_eq!(events[1].kind_str(), "round_completed");
        assert_eq!(events[2].kind_str(), "round_completed");
        assert_eq!(events[3].kind_str(), "completed");
    }

    #[test]
    fn debug_impl_works() {
        let b = CouncilBusBridge::default();
        let s = format!("{:?}", b);
        assert!(s.contains("CouncilBusBridge"));
        assert!(s.contains("enabled"));
    }
}
