//! apeireth-bus v2 — L0 + pattern + event_log 协同集成测试 (v1 抄录保 v1 pub API 表面).
//!
//! 验证 v1 三模块协同 (L0 publish/subscribe + pattern wildcard + event_log replay)
//! 在 v2 架构下的真实端到端工作流.
//!
//! 场景:
//! 1. L0Bus 启用 event_log
//! 2. publish 5 条 agent.* + system.* 消息
//! 3. 注册 pattern "agent.*" + "system.#"
//! 4. verify event_log replay_topic / replay_pattern
//! 5. verify pattern subscriber 收到对应消息

use apeireth_bus::event_log::LoggedEvent;
use apeireth_bus::{BusMessage, L0Bus};
use futures_util::StreamExt;
use std::time::Duration;

fn fresh_bus() -> L0Bus<String> {
    L0Bus::<String>::with_capacity_and_policy(64, apeireth_bus::BackpressurePolicy::Block)
        .with_event_log()
}

#[tokio::test]
async fn v2_l0_event_log_records_published_messages() {
    let bus = fresh_bus();
    let log = bus.event_log().cloned().expect("event_log enabled");
    assert!(log.is_empty());

    bus.publish("agent.bob", BusMessage::new("bob-payload".into()))
        .await
        .expect("publish agent.bob");
    bus.publish("agent.alice", BusMessage::new("alice-payload".into()))
        .await
        .expect("publish agent.alice");
    bus.publish("system.cpu.high", BusMessage::new("cpu-warning".into()))
        .await
        .expect("publish system.cpu.high");

    assert_eq!(log.len(), 3);

    let agent_events = log.replay_topic("agent.bob");
    assert_eq!(agent_events.len(), 1);
    assert_eq!(agent_events[0].message.payload, "bob-payload");
}

#[tokio::test]
async fn v2_l0_event_log_pattern_replay_with_wildcard() {
    let bus = fresh_bus();
    let log = bus.event_log().cloned().expect("event_log enabled");

    bus.publish("agent.bob", BusMessage::new("a".into())).await.unwrap();
    bus.publish("agent.alice", BusMessage::new("b".into())).await.unwrap();
    bus.publish("agent.team.lead", BusMessage::new("c".into())).await.unwrap();
    bus.publish("system.cpu", BusMessage::new("d".into())).await.unwrap();

    // agent.* 匹配单段 (agent.bob, agent.alice)
    let single = log.replay_pattern("agent.*");
    assert_eq!(single.len(), 2);

    // agent.# 匹配多段 (agent.bob, agent.alice, agent.team.lead)
    let multi = log.replay_pattern("agent.#");
    assert_eq!(multi.len(), 3);

    // system.# 匹配 system.cpu
    let sys = log.replay_pattern("system.#");
    assert_eq!(sys.len(), 1);

    // # 匹配全部
    let all = log.replay_pattern("#");
    assert_eq!(all.len(), 4);
}

#[tokio::test]
async fn v2_l0_pattern_subscriber_receives_matching_publishes() {
    let bus = fresh_bus();
    let mut sub = bus.subscribe_pattern("agent.*").await.expect("subscribe pattern");

    bus.publish("agent.bob", BusMessage::new("b-payload".into())).await.unwrap();
    let got = tokio::time::timeout(Duration::from_millis(300), sub.next())
        .await
        .expect("no timeout")
        .expect("stream item")
        .expect("ok");
    assert_eq!(got.payload, "b-payload");
}

#[tokio::test]
async fn v2_l0_pattern_unsubscribe_removes_fanout() {
    let bus = fresh_bus();
    let mut sub = bus.subscribe_pattern("test.*").await.expect("subscribe");

    // 先发一条, 确保 pattern 注册了
    bus.publish("test.a", BusMessage::new("first".into())).await.unwrap();
    let _ = tokio::time::timeout(Duration::from_millis(200), sub.next())
        .await
        .expect("first message")
        .expect("stream item")
        .expect("ok");

    // 注销 pattern
    let removed = bus.unsubscribe_pattern("test.*").await;
    assert!(removed, "pattern 应被注销");
    assert_eq!(bus.pattern_count().await, 0);
}

#[tokio::test]
async fn v2_l0_subscribe_and_publish_round_trip() {
    let bus = fresh_bus();
    let mut rx = bus.subscribe("hello.world").await.expect("subscribe");

    bus.publish("hello.world", BusMessage::new("hi".into())).await.unwrap();
    let m = tokio::time::timeout(Duration::from_millis(200), rx.next())
        .await
        .expect("no timeout")
        .expect("stream item")
        .expect("ok");
    assert_eq!(m.payload, "hi");
    assert!(m.trace_id > 0, "trace_id 必须 > 0");
}

#[tokio::test]
async fn v2_l0_event_log_last_n_returns_newest_first() {
    let bus = fresh_bus();
    let log = bus.event_log().cloned().unwrap();

    for i in 0..5 {
        bus.publish("t", BusMessage::new(format!("msg-{i}"))).await.unwrap();
    }

    let last_2 = log.last_n(2);
    assert_eq!(last_2.len(), 2);
    // LIFO: 最新在前
    assert_eq!(last_2[0].message.payload, "msg-4");
    assert_eq!(last_2[1].message.payload, "msg-3");
}

#[tokio::test]
async fn v2_l0_event_log_capacity_overflow_evicts_oldest() {
    let bus: L0Bus<u32> = L0Bus::<u32>::with_capacity_and_policy(
        64,
        apeireth_bus::BackpressurePolicy::Block,
    )
    .with_event_log_capacity(3);
    let log = bus.event_log().cloned().unwrap();
    assert_eq!(log.capacity(), 3);

    for i in 0..5 {
        bus.publish("t", BusMessage::new(i)).await.unwrap();
    }

    assert_eq!(log.len(), 3, "满了应保持 capacity 大小");
    let all: Vec<LoggedEvent<u32>> = log.all();
    assert_eq!(all[0].message.payload, 2);
    assert_eq!(all[2].message.payload, 4);
}

#[tokio::test]
async fn v2_l0_stats_count_publish_attempts() {
    let bus = fresh_bus();
    let s_before = bus.stats();
    assert_eq!(s_before.sent, 0);

    bus.publish("stats.t", BusMessage::new("x".into())).await.unwrap();
    bus.publish("stats.t", BusMessage::new("y".into())).await.unwrap();

    let s_after = bus.stats();
    assert!(s_after.sent >= 2, "sent 应 ≥ 2, got {}", s_after.sent);
}
