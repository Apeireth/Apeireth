//! apeireth-bus v2 — 公开 API 类型 serde round-trip 测试 (v1 抄录保 v1 pub API 表面).
//!
//! v1 pub API surface 保:
//! - `BusMessage<T>` (trace_id / payload / created_at_ms / priority)
//! - `MessagePriority` (High / Normal / Low)
//! - `BackpressurePolicy` (Block / DropOldest / DropNewest / Drop / Coalesce / Adaptive)
//! - `BusStats` + `BusStatsSnapshot`
//! - `Channel` (Ai / Human / Both) + `ChannelSet` (u8 bitset)
//! - `LifecycleEvent` + `LifecycleContext`
//!
//! v2 增项:
//! - serde_json round-trip (v1 用 bincode / json, v2 仅验 JSON)
//! - bincode 2.x round-trip (L1 用)

use apeireth_bus::{
    next_trace_id, BackpressurePolicy, BusMessage, BusStats, Channel, ChannelSet,
    LifecycleContext, LifecycleEvent, LifecycleMessage, MessagePriority,
};

#[test]
fn v2_bus_message_serde_json_round_trip() {
    let m: BusMessage<String> = BusMessage::with_trace_id(42u64, "hello".into())
        .with_priority(MessagePriority::High);
    let json = serde_json::to_string(&m).expect("serialize");
    let parsed: BusMessage<String> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.trace_id, 42);
    assert_eq!(parsed.payload, "hello");
    assert_eq!(parsed.priority, MessagePriority::High);
}

#[test]
fn v2_bus_message_bincode_round_trip() {
    let m: BusMessage<String> = BusMessage::with_trace_id(123u64, "abc".into());
    let bytes = bincode::serde::encode_to_vec(&m, bincode::config::standard()).unwrap();
    let (parsed, _): (BusMessage<String>, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
    assert_eq!(parsed.trace_id, 123);
    assert_eq!(parsed.payload, "abc");
}

#[test]
fn v2_message_priority_default_is_normal() {
    let m: BusMessage<u32> = BusMessage::new(1);
    assert_eq!(m.priority, MessagePriority::Normal);
    let json = serde_json::to_string(&m).unwrap();
    let parsed: BusMessage<u32> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.priority, MessagePriority::Normal);
}

#[test]
fn v2_backpressure_policy_all_variants_serde() {
    let variants = vec![
        BackpressurePolicy::Block,
        BackpressurePolicy::DropOldest,
        BackpressurePolicy::DropNewest,
        BackpressurePolicy::Drop,
        BackpressurePolicy::Coalesce { ttl_ms: 1500 },
        BackpressurePolicy::Adaptive {
            initial: Box::new(BackpressurePolicy::DropOldest),
            drop_threshold: 0.75,
        },
    ];
    for pol in variants {
        let json = serde_json::to_string(&pol).expect("serialize");
        let back: BackpressurePolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, pol);
        assert_eq!(back.name(), pol.name());
    }
}

#[test]
fn v2_backpressure_policy_variant_count_hardcoded_6() {
    assert_eq!(BackpressurePolicy::VARIANT_COUNT, 6);
}

#[test]
fn v2_channel_serde_round_trip() {
    for ch in [Channel::Ai, Channel::Human, Channel::Both] {
        let json = serde_json::to_string(&ch).unwrap();
        let back: Channel = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ch);
        assert_eq!(back.as_legacy_str(), ch.as_legacy_str());
        assert_eq!(back.topic_prefix(), ch.topic_prefix());
    }
}

#[test]
fn v2_channel_set_bits_serde_round_trip() {
    let mut s = ChannelSet::empty();
    s.insert(Channel::Ai);
    s.insert(Channel::Human);
    let json = serde_json::to_string(&s).unwrap();
    let back: ChannelSet = serde_json::from_str(&json).unwrap();
    assert_eq!(back.bits(), s.bits());
    assert!(back.contains(Channel::Ai));
    assert!(back.contains(Channel::Human));
    assert!(!back.contains(Channel::Both));
}

#[test]
fn v2_channel_set_to_vec_round_trip() {
    // R148 fix: BOTH = Ai | Human fan-out, 不是 Both 自己
    assert_eq!(ChannelSet::AI.to_vec(), vec![Channel::Ai]);
    assert_eq!(ChannelSet::BOTH.to_vec(), vec![Channel::Ai, Channel::Human]);
    assert_eq!(
        ChannelSet::ALL.to_vec(),
        vec![Channel::Ai, Channel::Human, Channel::Both]
    );
}

#[test]
fn v2_bus_stats_snapshot_serde_round_trip() {
    use std::sync::atomic::Ordering;
    let s = BusStats::new();
    s.sent.fetch_add(7, Ordering::Relaxed);
    s.dropped.fetch_add(3, Ordering::Relaxed);
    s.received.fetch_add(11, Ordering::Relaxed);
    s.high_priority.fetch_add(2, Ordering::Relaxed);
    s.normal_priority.fetch_add(3, Ordering::Relaxed);
    s.low_priority.fetch_add(2, Ordering::Relaxed);
    let snap = s.snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let back: apeireth_bus::BusStatsSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(back.sent, 7);
    assert_eq!(back.dropped, 3);
    assert_eq!(back.received, 11);
    assert_eq!(back.high_priority, 2);
    assert_eq!(back.normal_priority, 3);
    assert_eq!(back.low_priority, 2);
}

#[test]
fn v2_lifecycle_event_serde_round_trip() {
    for ev in [
        LifecycleEvent::UserPromptSubmit,
        LifecycleEvent::SessionStart,
        LifecycleEvent::SessionEnd,
        LifecycleEvent::PostToolUse,
        LifecycleEvent::Stop,
    ] {
        let json = serde_json::to_string(&ev).unwrap();
        let back: LifecycleEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ev);
        assert_eq!(back.as_str(), ev.as_str());
    }
}

#[test]
fn v2_lifecycle_context_serde_round_trip() {
    let ctx = LifecycleContext::new("session-x").with_detail("user-query-text");
    let json = serde_json::to_string(&ctx).unwrap();
    let back: LifecycleContext = serde_json::from_str(&json).unwrap();
    assert_eq!(back.session_id.as_deref(), Some("session-x"));
    assert_eq!(back.detail.as_deref(), Some("user-query-text"));
}

#[test]
fn v2_lifecycle_message_serde_round_trip() {
    let lm = LifecycleMessage {
        event: LifecycleEvent::SessionStart,
        ctx: LifecycleContext::new("session-y").with_detail("starting"),
    };
    let json = serde_json::to_string(&lm).unwrap();
    let back: LifecycleMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(back.event, LifecycleEvent::SessionStart);
    assert_eq!(back.ctx.session_id.as_deref(), Some("session-y"));
}

#[test]
fn v2_trace_id_is_monotonically_increasing() {
    let a = next_trace_id();
    let b = next_trace_id();
    let c = next_trace_id();
    assert!(a < b && b < c, "trace_ids must be monotonically increasing");
}

#[test]
fn v2_bus_message_map_preserves_trace_and_priority() {
    let m: BusMessage<u32> = BusMessage::with_trace_id(99u64, 5u32)
        .with_priority(MessagePriority::Low);
    let mapped: BusMessage<String> = m.map(|x| format!("v={x}"));
    assert_eq!(mapped.trace_id, 99);
    assert_eq!(mapped.payload, "v=5");
    assert_eq!(mapped.priority, MessagePriority::Low);
}
