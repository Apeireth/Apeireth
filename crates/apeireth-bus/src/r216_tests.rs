//! R216 bus 三套通知系统测试覆盖 (接续 R148).
//!
//! **动机**: R148 已实现 3 channel (Ai/Human/Both) + 4 BackpressurePolicy
//! + L0/L1/L2 5 层. 但单元测试只覆盖基础 channel 隔离, 没有:
//! 1. 4 BackpressurePolicy 全部行为测试
//! 2. 3 channel 端到端 trace_id 链路追踪
//! 3. ChannelSet fan-out 多路广播
//!
//! **R216 范围**: 在 bus crate 内部补充 14 测试, 覆盖上面 3 类缺口.
//!
//! **0 触碰**: channel.rs / l0.rs / l1.rs / l2.rs 0 改, 仅新增测试 mod.

#![cfg(test)]
#![allow(missing_docs)] // R216 additive

use futures_util::StreamExt;
use std::time::Duration;

use crate::channel::{Channel, ChannelSet, ChanneledBus};
use crate::{BackpressurePolicy, BusMessage, BusStats, L0Bus};

// ============================================================================
// 1. 4 BackpressurePolicy 行为测试 (4 cases)
// ============================================================================

#[tokio::test]
async fn r216_01_block_policy_no_receiver() {
    // 没有 receiver 时 publish 不应 hang (按 policy 决定: Block 仍然 Ok)
    let bus: L0Bus<String> = L0Bus::with_capacity_and_policy(2, BackpressurePolicy::Block);
    let r = bus.publish("topic", BusMessage::new("m1".to_string())).await;
    assert!(r.is_ok());
}

#[tokio::test]
async fn r216_02_drop_oldest_policy() {
    // DropOldest: broadcast 不真"满", 但 stats 应有正确 sent
    let bus: L0Bus<String> = L0Bus::with_capacity_and_policy(2, BackpressurePolicy::DropOldest);
    let _rx = bus.subscribe("topic").await.unwrap();
    bus.publish("topic", BusMessage::new("m1".to_string())).await.unwrap();
    bus.publish("topic", BusMessage::new("m2".to_string())).await.unwrap();
    bus.publish("topic", BusMessage::new("m3".to_string())).await.unwrap();
    let s = bus.stats();
    assert!(s.sent >= 3, "DropOldest 仍计 sent");
}

#[tokio::test]
async fn r216_03_drop_newest_policy() {
    let bus: L0Bus<String> = L0Bus::with_capacity_and_policy(2, BackpressurePolicy::DropNewest);
    let _rx = bus.subscribe("topic").await.unwrap();
    bus.publish("topic", BusMessage::new("m1".to_string())).await.unwrap();
    bus.publish("topic", BusMessage::new("m2".to_string())).await.unwrap();
    bus.publish("topic", BusMessage::new("m3".to_string())).await.unwrap();
    let s = bus.stats();
    assert!(s.sent >= 3);
}

#[tokio::test]
async fn r216_04_drop_policy() {
    // Drop: 没有 receiver 时直接丢
    let bus: L0Bus<String> = L0Bus::with_capacity_and_policy(2, BackpressurePolicy::Drop);
    bus.publish("topic", BusMessage::new("m1".to_string())).await.unwrap();
    let s = bus.stats();
    assert_eq!(s.dropped, 1, "Drop 策略: 无 receiver 应计 dropped");
}

// ============================================================================
// 2. 3 channel 端到端 trace_id 链路追踪 (4 cases)
// ============================================================================

#[tokio::test]
async fn r216_05_ai_channel_isolation() {
    let bus = ChanneledBus::<String>::new();
    let mut ai_rx = bus.subscribe(Channel::Ai, "topic").await.unwrap();
    let mut human_rx = bus.subscribe(Channel::Human, "topic").await.unwrap();
    bus.publish(Channel::Ai, "topic", BusMessage::new("ai_only".to_string())).await.unwrap();
    // AI 收到
    let msg = tokio::time::timeout(Duration::from_millis(100), ai_rx.next()).await;
    assert!(msg.is_ok() && msg.unwrap().is_some());
    // Human 不应收到 AI 的消息
    let no_msg = tokio::time::timeout(Duration::from_millis(50), human_rx.next()).await;
    assert!(no_msg.is_err(), "Human channel 不应收 AI 消息");
}

#[tokio::test]
async fn r216_06_human_channel_isolation() {
    let bus = ChanneledBus::<String>::new();
    let mut ai_rx = bus.subscribe(Channel::Ai, "topic").await.unwrap();
    let mut human_rx = bus.subscribe(Channel::Human, "topic").await.unwrap();
    bus.publish(Channel::Human, "topic", BusMessage::new("h".to_string())).await.unwrap();
    let msg = tokio::time::timeout(Duration::from_millis(100), human_rx.next()).await;
    assert!(msg.is_ok() && msg.unwrap().is_some());
    let no_msg = tokio::time::timeout(Duration::from_millis(50), ai_rx.next()).await;
    assert!(no_msg.is_err());
}

#[tokio::test]
async fn r216_07_both_channel_visible_to_subscribers() {
    // Both 单独订阅方能收到 (而非"AI + Human 自动收到 Both")
    let bus = ChanneledBus::<String>::new();
    let mut both_rx = bus.subscribe(Channel::Both, "topic").await.unwrap();
    bus.publish(Channel::Both, "topic", BusMessage::new("everyone".to_string())).await.unwrap();
    let msg = tokio::time::timeout(Duration::from_millis(100), both_rx.next()).await;
    assert!(msg.is_ok() && msg.unwrap().is_some());
}

#[tokio::test]
async fn r216_08_trace_id_preserved_through_channel() {
    let bus = ChanneledBus::<String>::new();
    let mut rx = bus.subscribe(Channel::Ai, "topic").await.unwrap();
    let custom_msg = BusMessage::with_trace_id(12345, "trace_test".to_string());
    bus.publish(Channel::Ai, "topic", custom_msg).await.unwrap();
    let received = tokio::time::timeout(Duration::from_millis(100), rx.next()).await.unwrap().unwrap().unwrap();
    assert_eq!(received.trace_id, 12345, "trace_id 应跨 channel 保留");
}

// ============================================================================
// 3. ChannelSet fan-out 多路广播 (3 cases)
// ============================================================================

#[tokio::test]
async fn r216_09_channelset_ai_human_fanout() {
    // ChannelSet(Ai | Human) — 发到 2 个 channel
    let bus = ChanneledBus::<String>::new();
    let mut ai_rx = bus.subscribe(Channel::Ai, "t").await.unwrap();
    let mut human_rx = bus.subscribe(Channel::Human, "t").await.unwrap();
    let set = { let mut s = ChannelSet::empty(); s.insert(Channel::Ai); s.insert(Channel::Human); s };
    let n = bus.publish_multi(set, "t", BusMessage::new("both".to_string())).await.unwrap();
    assert_eq!(n, 2);
    let m1 = tokio::time::timeout(Duration::from_millis(100), ai_rx.next()).await;
    let m2 = tokio::time::timeout(Duration::from_millis(100), human_rx.next()).await;
    assert!(m1.is_ok() && m1.unwrap().is_some());
    assert!(m2.is_ok() && m2.unwrap().is_some());
}

#[tokio::test]
async fn r216_10_channelset_all_3() {
    let bus = ChanneledBus::<String>::new();
    let mut ai_rx = bus.subscribe(Channel::Ai, "t").await.unwrap();
    let mut human_rx = bus.subscribe(Channel::Human, "t").await.unwrap();
    let mut both_rx = bus.subscribe(Channel::Both, "t").await.unwrap();
    let n = bus.publish_multi(ChannelSet::ALL, "t", BusMessage::new("triple".to_string())).await.unwrap();
    assert_eq!(n, 3);
    let m1 = tokio::time::timeout(Duration::from_millis(100), ai_rx.next()).await;
    let m2 = tokio::time::timeout(Duration::from_millis(100), human_rx.next()).await;
    let m3 = tokio::time::timeout(Duration::from_millis(100), both_rx.next()).await;
    assert!(m1.is_ok() && m1.unwrap().is_some());
    assert!(m2.is_ok() && m2.unwrap().is_some());
    assert!(m3.is_ok() && m3.unwrap().is_some());
}

#[tokio::test]
async fn r216_11_channelset_partial() {
    let bus = ChanneledBus::<String>::new();
    let mut human_rx = bus.subscribe(Channel::Human, "t").await.unwrap();
    let set = ChannelSet::from_channel(Channel::Ai);
    bus.publish_multi(set, "t", BusMessage::new("ai_only".to_string())).await.unwrap();
    let no_msg = tokio::time::timeout(Duration::from_millis(50), human_rx.next()).await;
    assert!(no_msg.is_err());
}

// ============================================================================
// 4. BusStats + 综合 (3 cases)
// ============================================================================

#[tokio::test]
async fn r216_12_bus_stats_track() {
    let bus: L0Bus<u32> = L0Bus::new();
    let mut rx = bus.subscribe("s").await.unwrap();
    bus.publish("s", BusMessage::new(1)).await.unwrap();
    bus.publish("s", BusMessage::new(2)).await.unwrap();
    bus.publish("s", BusMessage::new(3)).await.unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(50), rx.next()).await;
    let _ = tokio::time::timeout(Duration::from_millis(50), rx.next()).await;
    let _ = tokio::time::timeout(Duration::from_millis(50), rx.next()).await;
    let s = bus.stats();
    assert_eq!(s.sent, 3);
    assert!(s.received >= 1, "至少收 1 条 (可能 lag)");
}

#[tokio::test]
async fn r216_13_bus_stats_shared_arc() {
    let stats = BusStats::shared();
    stats.sent.fetch_add(5, std::sync::atomic::Ordering::Relaxed);
    let snap = stats.snapshot();
    assert_eq!(snap.sent, 5);
}

#[tokio::test]
async fn r216_14_channel_count_constant() {
    assert_eq!(Channel::COUNT, 3);
    assert_eq!(Channel::ALL.len(), 3);
}


// ============================================================
// R226 — BackpressurePolicy 补全 (Coalesce + Adaptive) — 7 cases
// ============================================================

#[test]
fn r226_01_policy_name_block() {
    assert_eq!(BackpressurePolicy::Block.name(), "Block");
}

#[test]
fn r226_02_policy_name_drop_oldest() {
    assert_eq!(BackpressurePolicy::DropOldest.name(), "DropOldest");
}

#[test]
fn r226_03_policy_name_drop_newest() {
    assert_eq!(BackpressurePolicy::DropNewest.name(), "DropNewest");
}

#[test]
fn r226_04_policy_name_drop() {
    assert_eq!(BackpressurePolicy::Drop.name(), "Drop");
}

#[test]
fn r226_05_policy_name_coalesce() {
    assert_eq!(
        BackpressurePolicy::Coalesce { ttl_ms: 100 }.name(),
        "Coalesce"
    );
}

#[test]
fn r226_06_policy_name_adaptive() {
    assert_eq!(
        BackpressurePolicy::Adaptive {
            initial: Box::new(BackpressurePolicy::Block),
            drop_threshold: 0.5,
        }
        .name(),
        "Adaptive"
    );
}

#[tokio::test]
async fn r226_07_coalesce_policy_publishes() {
    // Coalesce 走 Block 行为 (intent-only)
    let bus: L0Bus<u32> = L0Bus::with_capacity_and_policy(8, BackpressurePolicy::Coalesce { ttl_ms: 100 });
    let _ = bus.publish("topic", BusMessage::new(42)).await;
    let s = bus.stats();
    assert_eq!(s.sent, 1, "Coalesce 应记 1 sent");
}

#[tokio::test]
async fn r226_08_adaptive_policy_publishes() {
    // Adaptive 走 Block 行为 (intent-only)
    let bus: L0Bus<u32> = L0Bus::with_capacity_and_policy(8, BackpressurePolicy::Adaptive {
        initial: Box::new(BackpressurePolicy::DropOldest),
        drop_threshold: 0.3,
    });
    let _ = bus.publish("topic", BusMessage::new(99)).await;
    let s = bus.stats();
    assert_eq!(s.sent, 1, "Adaptive 应记 1 sent");
}


// ============================================================
// R228 — subscribe_pattern 集成 (8 cases)
// ============================================================

#[tokio::test]
async fn r228_01_subscribe_pattern_receives_matching() {
    let bus: L0Bus<String> = L0Bus::new();
    let mut stream = bus.subscribe_pattern("agent.*").await.unwrap();
    bus.publish("agent.bob", BusMessage::new("hello".to_string())).await.unwrap();
    let msg = stream.next().await;
    let item = msg.expect("expected Some(msg)").expect("expected Ok");
    assert_eq!(item.payload, "hello");
}

#[tokio::test]
async fn r228_02_subscribe_pattern_no_match_doesnt_receive() {
    let bus: L0Bus<String> = L0Bus::new();
    let mut stream = bus.subscribe_pattern("agent.*").await.unwrap();
    bus.publish("system.cpu", BusMessage::new("x".to_string())).await.unwrap();
    // 用 timeout 验证 100ms 内不收到
    let res = tokio::time::timeout(std::time::Duration::from_millis(100), stream.next()).await;
    assert!(res.is_err(), "agent.* 不应匹配 system.cpu");
}

#[tokio::test]
async fn r228_03_multi_wildcard_matches_multi_segments() {
    let bus: L0Bus<String> = L0Bus::new();
    let mut stream = bus.subscribe_pattern("agent.#").await.unwrap();
    bus.publish("agent.team.lead", BusMessage::new("y".to_string())).await.unwrap();
    let msg = stream.next().await.expect("Some").expect("Ok");
    assert_eq!(msg.payload, "y");
}

#[tokio::test]
async fn r228_04_pure_multi_wildcard_receives_all() {
    let bus: L0Bus<String> = L0Bus::new();
    let mut stream = bus.subscribe_pattern("#").await.unwrap();
    bus.publish("foo", BusMessage::new("a".to_string())).await.unwrap();
    let msg = stream.next().await.expect("Some").expect("Ok");
    assert_eq!(msg.payload, "a");
    bus.publish("foo.bar", BusMessage::new("b".to_string())).await.unwrap();
    let msg = stream.next().await.expect("Some").expect("Ok");
    assert_eq!(msg.payload, "b");
}

#[tokio::test]
async fn r228_05_publish_fans_out_to_multiple_patterns() {
    let bus: L0Bus<String> = L0Bus::new();
    let mut s1 = bus.subscribe_pattern("agent.*").await.unwrap();
    let mut s2 = bus.subscribe_pattern("*.bob").await.unwrap();
    bus.publish("agent.bob", BusMessage::new("z".to_string())).await.unwrap();
    // s1 (agent.*) 应收到
    let m1 = s1.next().await.expect("s1 Some").expect("s1 Ok");
    assert_eq!(m1.payload, "z");
    // s2 (*.bob) 应收到
    let m2 = s2.next().await.expect("s2 Some").expect("s2 Ok");
    assert_eq!(m2.payload, "z");
}

#[tokio::test]
async fn r228_06_pattern_count_tracks() {
    let bus: L0Bus<String> = L0Bus::new();
    assert_eq!(bus.pattern_count().await, 0);
    let _s1 = bus.subscribe_pattern("a.*").await.unwrap();
    assert_eq!(bus.pattern_count().await, 1);
    let _s2 = bus.subscribe_pattern("b.#").await.unwrap();
    assert_eq!(bus.pattern_count().await, 2);
    assert!(bus.unsubscribe_pattern("a.*").await);
    assert_eq!(bus.pattern_count().await, 1);
    assert!(!bus.unsubscribe_pattern("nonexistent").await);
}

#[tokio::test]
async fn r228_07_unsubscribe_pattern_stops_delivery() {
    let bus: L0Bus<String> = L0Bus::new();
    let mut stream = bus.subscribe_pattern("x.*").await.unwrap();
    bus.publish("x.foo", BusMessage::new("1".to_string())).await.unwrap();
    let m1 = stream.next().await.expect("first Some").expect("first Ok");
    assert_eq!(m1.payload, "1");
    assert!(bus.unsubscribe_pattern("x.*").await);
    // 再 publish — stream 应已 close (Sender 被 drop, Receiver 收到 Closed)
    bus.publish("x.bar", BusMessage::new("2".to_string())).await.unwrap();
    // 用短 timeout 验证 stream 已结束 (None) 而非阻塞
    let res = tokio::time::timeout(std::time::Duration::from_millis(100), stream.next()).await;
    match res {
        Ok(None) => {} // 预期: stream 已 close, next() 立即返 None
        Ok(Some(_)) => panic!("unsubscribe 后 stream 不应再产生消息"),
        Err(_) => panic!("unsubscribe 后 stream 应立即返 None, 不应 timeout"),
    }
}

#[tokio::test]
async fn r228_08_exact_and_pattern_both_receive() {
    // 同一 topic 既被 exact subscribe 又被 pattern subscribe — 都应收到
    let bus: L0Bus<String> = L0Bus::new();
    let mut exact = bus.subscribe("agent.bob").await.unwrap();
    let mut pat = bus.subscribe_pattern("agent.*").await.unwrap();
    bus.publish("agent.bob", BusMessage::new("dual".to_string())).await.unwrap();
    let m1 = exact.next().await.expect("exact Some").expect("exact Ok");
    let m2 = pat.next().await.expect("pat Some").expect("pat Ok");
    assert_eq!(m1.payload, "dual");
    assert_eq!(m2.payload, "dual");
}
