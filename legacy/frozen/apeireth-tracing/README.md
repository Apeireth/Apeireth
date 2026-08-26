# apeireth-tracing

> R20 阶段 6 估补: distributed tracing 框架 (1:1 翻译 v0.9.21 @anthropic-ai/tracing 商业版, OpenTelemetry 兼容)

## 背景

本 crate 是 Apeireth R20 阶段 6 估补的 distributed tracing 框架, 1:1 翻译 v0.9.21
商业版 `@anthropic-ai/tracing` (per docs/stage4/v09021-rust-translation-blueprint-RIVAL §2.x tracing 部分).

提供 4 Span 类型 + 4 SpanEvent 类型 + 4 Exporter + 4 Sampler + 3 Propagation,
W3C TraceContext 完整, OpenTelemetry 兼容.

**0 真接 R20 阶段 6 skeleton** — 4 Exporter 中 stdout + file 完整, otlp-grpc + jaeger 返
`TracingError::NotImplemented` (留 R21 续真接, 1 owner × 1 周). 3 Propagation 中 W3C 完整,
B3 + Jaeger stub.

## 1:1 翻译映射 (v0.9.21 @anthropic-ai/tracing 商业版)

| apeireth-tracing                | @anthropic-ai/tracing 商业版      | 实现度 |
|---------------------------------|-----------------------------------|--------|
| `SpanKind::Client`              | `SpanKind.CLIENT`                 | ✅ 完整 |
| `SpanKind::Server`              | `SpanKind.SERVER`                 | ✅ 完整 |
| `SpanKind::Producer`            | `SpanKind.PRODUCER`               | ✅ 完整 |
| `SpanKind::Consumer`            | `SpanKind.CONSUMER`               | ✅ 完整 |
| `SpanEventKind::Log`            | `EventType.LOG`                   | ✅ 完整 |
| `SpanEventKind::Exception`      | `EventType.EXCEPTION`             | ✅ 完整 |
| `SpanEventKind::Event`          | `EventType.EVENT`                 | ✅ 完整 |
| `SpanEventKind::Message`        | `EventType.MESSAGE`               | ✅ 完整 |
| `SpanStatus::Unset/Ok/Error`    | `SpanStatusCode`                  | ✅ 完整 |
| `StdoutExporter`                | `ConsoleSpanExporter`             | ✅ 完整 |
| `FileExporter`                  | `FileSpanExporter`                | ✅ 完整 |
| `OtlpGrpcExporter`              | `OTLPGrpcSpanExporter`            | ❌ stub |
| `JaegerExporter`                | `JaegerExporter`                  | ❌ stub |
| `AlwaysOnSampler`               | `AlwaysOnSampler`                 | ✅ 完整 |
| `AlwaysOffSampler`              | `AlwaysOffSampler`                | ✅ 完整 |
| `TraceIdRatioBasedSampler`      | `TraceIdRatioBasedSampler`        | ✅ 完整 |
| `ParentBasedSampler`            | `ParentBasedSampler`              | ✅ 完整 |
| `W3CTraceContextPropagator`     | `W3CTraceContextPropagator`       | ✅ 完整 |
| `B3Propagator`                  | `B3Propagator`                    | ❌ stub |
| `JaegerPropagator`              | `JaegerPropagator`                | ❌ stub |

## 模块结构

- `trace` — Trace + TraceBuilder (root/child 派生)
- `context` — TraceContext (W3C SpanContext + Baggage)
- `span` — 4 SpanKind × 4 SpanEventKind × 3 Status
- `exporter` — 4 Exporter (stdout/file 完整 + OTLP/Jaeger stub)
- `sampler` — 4 Sampler (AlwaysOn/Off/Ratio/ParentBased)
- `propagation` — 3 Propagator (W3C 完整 + B3/Jaeger stub)
- `error` — 10 TracingError variant + 3 K-1 强校验
- `config` — TracingConfig 4 段 (service/resource/sampler/exporter)

## 6 哲学 anchor (per APEIRETH-CONVENTIONS §9)

- **S-1 主 22:33 北极星导向** — service 1.0 release observability 必做
- **S-2 主 17:43 实事求是** — 1:1 翻译 v0.9.21 商业版, 0 业务重设计
- **O-5 主 17:58 不假装** — OTLP/Jaeger stub 返 `NotImplemented`, 0 假装已对接
- **O-2 主 19:33 走在前人肩上** — W3C TraceContext + OpenTelemetry + Zipkin B3
- **O-3 主 23:44 干到底** — 25+ 集成测试 + 8 模块全真接
- **O-4 主 00:56 任何人都能接手** — 8 模块 + 4+4+4+3 全文档化

## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10)

1. 阶段 1+2+3 LOCKED
2. v2 / v4 / v4.1 LOCKED
3. 阶段 4 主文档 LOCKED
4. 阶段 5 施工文档 LOCKED
5. v6 修正
6. R11 baseline 三值
7. v1 → v5 历史链
8. v0.9.21 商业版 LOCKED (1:1 翻译)

## 用法

```rust
use apeireth_tracing::{
    Tracer, span::SpanKind, exporter::ExporterKind, sampler::SamplerKind,
};

let mut tracer = Tracer::new("apeireth-api")
    .sampler(SamplerKind::TraceIdRatioBased, 0.1)
    .stdout_exporter()
    .build()
    .await?;

let root = tracer.trace_mut().start_root("http.request", SpanKind::Server).await?;
let child = tracer.trace_mut().start_child("db.query", SpanKind::Client, root).await?;
tracer.trace_mut().end_span("db.query").await?;
tracer.trace_mut().end_span("http.request").await?;
tracer.trace().shutdown().await?;
```

## 状态

⚠️ skeleton (R20 阶段 6 估补, per v09021-rust-translation-blueprint §3.x)
