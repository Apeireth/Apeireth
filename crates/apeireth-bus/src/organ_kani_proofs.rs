//! R177 bus organ Kani proofs (W4)

#![allow(missing_docs)]

use crate::{BackpressurePolicy, BusMessage, MessagePriority};

#[test]
fn r177_bus_01_message_priority_3() {
    let p1 = MessagePriority::High;
    let p2 = MessagePriority::Normal;
    let p3 = MessagePriority::Low;
    assert_ne!(p1, p2);
    assert_ne!(p2, p3);
    assert_ne!(p1, p3);
}

#[test]
fn r177_bus_02_priority_default_normal() {
    let p: MessagePriority = Default::default();
    assert_eq!(p, MessagePriority::Normal);
}

#[test]
fn r177_bus_03_bus_message_new() {
    let m: BusMessage<String> = BusMessage::new("hello".to_string());
    assert_eq!(m.payload, "hello");
    assert_eq!(m.priority, MessagePriority::Normal);
    assert!(m.trace_id > 0);
}

#[test]
fn r177_bus_04_bus_message_with_trace_id() {
    let m: BusMessage<i32> = BusMessage::with_trace_id(42, 100);
    assert_eq!(m.trace_id, 42);
    assert_eq!(m.payload, 100);
}

#[test]
fn r177_bus_05_bus_message_map_preserves_trace() {
    let m: BusMessage<i32> = BusMessage::with_trace_id(42, 100);
    let m2 = m.map(|x| x * 2);
    assert_eq!(m2.trace_id, 42, "map 应保留 trace_id");
    assert_eq!(m2.payload, 200);
}

#[test]
fn r177_bus_06_bus_message_with_priority() {
    let m: BusMessage<()> = BusMessage::new(()).with_priority(MessagePriority::High);
    assert_eq!(m.priority, MessagePriority::High);
}

#[test]
fn r177_bus_07_backpressure_policy_default_block() {
    let p: BackpressurePolicy = Default::default();
    assert_eq!(p, BackpressurePolicy::Block);
}

#[test]
fn r177_bus_08_backpressure_policy_variants_6() {
    assert_eq!(BackpressurePolicy::VARIANT_COUNT, 6);
}

#[test]
fn r177_bus_09_backpressure_policy_names_distinct() {
    let policies = vec![
        BackpressurePolicy::Block,
        BackpressurePolicy::DropOldest,
        BackpressurePolicy::DropNewest,
        BackpressurePolicy::Drop,
        BackpressurePolicy::Coalesce { ttl_ms: 1000 },
        BackpressurePolicy::Adaptive {
            initial: Box::new(BackpressurePolicy::Block),
            drop_threshold: 0.5,
        },
    ];
    let names: Vec<&str> = policies.iter().map(|p| p.name()).collect();
    let mut seen = std::collections::HashSet::new();
    for n in &names {
        assert!(seen.insert(*n), "policy name 重复: {}", n);
    }
    assert_eq!(seen.len(), 6);
}

#[test]
fn r177_bus_10_bus_message_clone_preserves() {
    let m: BusMessage<String> = BusMessage::new("hi".into()).with_priority(MessagePriority::Low);
    let m2 = m.clone();
    assert_eq!(m.trace_id, m2.trace_id);
    assert_eq!(m.payload, m2.payload);
    assert_eq!(m.priority, m2.priority);
}

#[cfg(kani)]
#[kani::proof]
fn r177_bus_kani_01_priority_distinct() {
    let p1 = MessagePriority::High;
    let p2 = MessagePriority::Normal;
    let p3 = MessagePriority::Low;
    assert_ne!(p1, p2);
    assert_ne!(p2, p3);
    assert_ne!(p1, p3);
}

#[cfg(kani)]
#[kani::proof]
fn r177_bus_kani_02_policy_default_block() {
    let p: BackpressurePolicy = Default::default();
    assert_eq!(p, BackpressurePolicy::Block);
}
