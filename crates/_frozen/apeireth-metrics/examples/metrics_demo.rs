//! # apeireth-metrics demo
//!
//! 构造 10 metric (5 counter + 3 gauge + 2 histogram) + 1 summary,
//! 用 Prometheus exporter 暴露到 stdout.
//!
//! 运行: `cargo run --example metrics_demo -p apeireth-metrics`

use std::collections::HashMap;
use std::sync::Arc;

use apeireth_metrics::{
    Counter, Gauge, Histogram, MetricsRegistry, PrometheusExporter, StdoutExporter, Summary,
    Exporter,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== apeireth-metrics demo (R20 阶段 6 skeleton) ===\n");

    // 1) 构造 registry
    let registry = MetricsRegistry::new();

    // 2) 注册 5 counter (模拟 HTTP 请求计数)
    println!("[1/4] 注册 5 counter...");
    for i in 0..5 {
        let mut labels = HashMap::new();
        labels.insert("endpoint".to_string(), format!("/api/v1/users/{i}"));
        labels.insert("method".to_string(), "GET".to_string());
        let c = Arc::new(
            Counter::new(
                format!("http_requests_total_{i}"),
                "Total HTTP requests",
                labels,
            )
            .unwrap(),
        );
        c.inc_by((i as u64 + 1) * 100);
        registry.register_counter(c).unwrap();
    }

    // 3) 注册 3 gauge (模拟内存 / 队列长度)
    println!("[2/4] 注册 3 gauge...");
    let g1 = Arc::new(Gauge::new("memory_bytes", "Memory used", HashMap::new()).unwrap());
    g1.set(1.5e9);
    registry.register_gauge(g1).unwrap();

    let g2 = Arc::new(Gauge::new("active_connections", "Active conns", HashMap::new()).unwrap());
    g2.set(42.0);
    registry.register_gauge(g2).unwrap();

    let g3 = Arc::new(Gauge::new("queue_depth", "Queue depth", HashMap::new()).unwrap());
    g3.set(7.0);
    registry.register_gauge(g3).unwrap();

    // 4) 注册 2 histogram (模拟请求延迟)
    println!("[3/4] 注册 2 histogram + 1 summary...");
    let h1 = Arc::new(
        Histogram::new(
            "request_duration_seconds",
            "Request duration",
            HashMap::new(),
        )
        .unwrap(),
    );
    for v in [0.003, 0.012, 0.05, 0.2, 1.5] {
        h1.observe(v);
    }
    registry.register_histogram(h1).unwrap();

    let h2 = Arc::new(
        Histogram::new(
            "db_query_duration_seconds",
            "DB query duration",
            HashMap::new(),
        )
        .unwrap(),
    );
    for v in [0.001, 0.005, 0.02, 0.1] {
        h2.observe(v);
    }
    registry.register_histogram(h2).unwrap();

    // 5) 注册 1 summary (模拟 RPC 分位数)
    let s = Arc::new(
        Summary::new("rpc_duration_seconds", "RPC duration", HashMap::new()).unwrap(),
    );
    for i in 1..=100 {
        s.observe(i as f64 * 0.001);
    }
    registry.register_summary(s).unwrap();

    // 6) Prometheus exporter 导出到字符串
    println!("[4/4] Prometheus exporter 导出 (apeireth_agent_*):\n");
    let p = PrometheusExporter::new("apeireth", "agent");
    let body = p.export(&registry).await?;
    print!("{body}");

    // 7) Stdout exporter (也走 Prometheus format, 但同时 println)
    println!("\n--- Stdout exporter (println 到 stdout) ---\n");
    let so = StdoutExporter::new();
    let _ = so.export(&registry).await?;

    println!("\n=== demo 完成 ===");
    Ok(())
}
