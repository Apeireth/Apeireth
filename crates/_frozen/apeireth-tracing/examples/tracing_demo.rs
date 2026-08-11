//! # apeireth-tracing 演示示例
//!
//! 构造一个 5-span trace (parent → child → grandchild → http client → http server),
//! 用 W3C TraceContext propagation 跨"服务"传播.
//!
//! ## 用法
//!
//! ```bash
//! cargo run --example tracing_demo -p apeireth-tracing
//! ```
//!
//! ## 行为
//!
//! 1. 构造 stdout exporter
//! 2. 启动 root span (http.server)
//! 3. 启动 child span (auth middleware)
//! 4. 启动 grandchild span (business logic)
//! 5. 启动 4th span (http.client) 调外部服务, inject context
//! 6. 模拟接收外部响应, extract context, 启动 5th span (http.server external)
//! 7. 逐个 end span, 全部 export 到 stdout

use apeireth_tracing::{
    exporter::ExporterKind, inject_context, sampler::AlwaysOnSampler, span::SpanKind, trace::Trace,
    ExporterConfig, Propagator, ResourceConfig, SamplerConfig, ServiceConfig, TracingConfig,
    W3CTraceContextPropagator,
};
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1) 构造 stdout config
    let config = TracingConfig {
        service: ServiceConfig {
            name: "apeireth-demo".into(),
            version: "0.1.0".into(),
            environment: "dev".into(),
        },
        resource: ResourceConfig::default(),
        sampler: SamplerConfig {
            kind: apeireth_tracing::sampler::SamplerKind::AlwaysOn,
            ratio: 1.0,
        },
        exporter: ExporterConfig {
            kind: ExporterKind::Stdout,
            endpoint: "".into(),
            output_path: "./traces.jsonl".into(),
            batch_size: 512,
        },
    };

    // 2) 构造 trace
    let sampler = Arc::new(AlwaysOnSampler);
    let mut trace = Trace::new(config, sampler).await?;

    // 3) 启动 root: http.server
    let http_server = trace
        .start_root("http.server /orders", SpanKind::Server)
        .await?;
    println!("[demo] root span started: {}", http_server.context);
    let http_server_clone = http_server.clone();

    // 4) 启动 child: business logic (server kind)
    let business = trace
        .start_child(
            "business.orders.create",
            SpanKind::Server,
            &http_server_clone,
        )
        .await?;
    println!("[demo] business span started: {}", business.context);
    let business_clone = business.clone();

    // 5) 启动 4th: http.client (调外部服务), inject context
    let http_client = trace
        .start_child("http.client POST /payments", SpanKind::Client, &business_clone)
        .await?;
    println!("[demo] http.client span started: {}", http_client.context);

    let mut headers = HashMap::new();
    inject_context(&http_client.context, &mut headers).await;
    println!("[demo] injected headers: {:?}", headers);

    // 6) 模拟外部服务: extract context, 构造外部 5th span (http.server external)
    let external_ctx = W3CTraceContextPropagator
        .extract(&headers)
        .await
        .ok_or("failed to extract")?;
    let external_root_ctx = external_ctx.child("cccccccccccccccc".into());
    // 直接 new Span 演示提取 (外部服务独立 Trace, 但 trace_id 相同)
    let external_span = apeireth_tracing::span::Span::new(
        "http.server external /payments",
        SpanKind::Server,
        external_root_ctx,
    )?;
    println!(
        "[demo] external span: {} (name: {})",
        external_span.context, external_span.name
    );

    // 7) 结束所有 span
    trace.end_span("http.client POST /payments").await?;
    trace
        .end_span("business.orders.create")
        .await?;
    trace
        .end_span("http.server /orders")
        .await?;
    trace.flush().await?;
    trace.shutdown().await?;

    println!("[demo] done");
    Ok(())
}
