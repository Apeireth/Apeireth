//! apeireth-bus L0 demo (round15-03 minimal stub)
//!
//! 目的:
//! - 让 `cargo test --workspace` 在 workspace 默认 examples 扫描下编译通过
//! - 演示 L0 inproc 总线 `L0Bus<String>` 的最小闭环 + 全局 trace_id 链路
//!
//! 7 项不修改承诺守口: 本 demo 仅依赖 `apeireth-bus` 公开 API,
//! 不修改任何 LOCKED 文档 / baseline / Cargo workspace 顶层结构。
//!
//! 完整的 5 层总线 (L1 UDS / L2 pipe / L3 gRPC / L4 WebSocket) 真实物理接入
//! 属于 V28.x-2 后续深化项 (见 HANDS-ON-MANUAL.md)。

use apeireth_bus::{now_ms, BackpressurePolicy, BusMessage, L0Bus};
use futures_util::stream::StreamExt;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== apeireth-bus L0 demo (round15-03 minimal stub) ===\n");

    let bus: L0Bus<String> = L0Bus::with_capacity_and_policy(16, BackpressurePolicy::Block);

    // [1] 订阅 + 接收 1 条消息 (Pub-Sub)
    let mut receiver = bus.subscribe("hello.world").await.expect("subscribe");
    let trace_id_root = apeireth_bus::next_trace_id();
    println!("[1] root trace_id = {}", trace_id_root);

    // [2] 发布 3 条 hello.* 消息
    for i in 0..3 {
        let payload = format!("hello-{}", i);
        let msg = BusMessage::with_trace_id(trace_id_root + i, payload);
        bus.publish("hello.world", msg).await.expect("publish");
        println!("[2] publish #{} trace_id={}", i, trace_id_root + i);
    }

    // [3] 接收 1 条并打印 trace_id
    if let Some(Ok(msg)) = receiver.next().await {
        println!(
            "[3] received: trace_id={} payload={} created_at_ms={}",
            msg.trace_id, msg.payload, msg.created_at_ms
        );
    }

    // [4] 主题数 + 统计
    let topics = bus.topic_count().await;
    let stats = bus.stats();
    println!("[4] topics={} bus_stats={:?}", topics, stats);

    println!("\n=== demo done (started at ms {}) ===", now_ms());
}
