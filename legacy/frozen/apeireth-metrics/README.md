# apeireth-metrics

Apeireth **R20 阶段 6 估缺** — Prometheus 兼容 metrics skeleton, 1:1 翻译 v0.9.21 `@anthropic-ai/metrics` 商业版.

## 模块

- `counter` — Counter (单调递增, e.g. `requests_total`)
- `gauge` — Gauge (任意增减, e.g. `memory_bytes`)
- `histogram` — Histogram (分桶, e.g. `request_duration_seconds`)
- `summary` — Summary (分位数, e.g. p50/p90/p99 latency)
- `registry` — 注册中心 (register / unregister / get / list)
- `label` — Label key-value (K-1 强校验: key 字符 + value 长度 256)
- `encoder` — Prometheus exposition format
- `exporter` — 5 exporter (Prometheus 完整, Pushgateway/OTLP/StatsD stub, Stdout 完整)
- `error` — 8-10 MetricsError variant
- `config` — MetricsConfig (namespace / subsystem / labels / exporter)

## 1:1 翻译映射 (v0.9.21 @anthropic-ai/metrics 商业版)

| apeireth-metrics          | @anthropic-ai/metrics 商业版       | 1:1 |
|--------------------------|------------------------------------|-----|
| `Counter`                | `class Counter`                    | ✅  |
| `Gauge`                  | `class Gauge`                      | ✅  |
| `Histogram`              | `class Histogram`                  | ✅  |
| `Summary`                | `class Summary`                    | ✅  |
| `MetricsRegistry`        | `class MetricsRegistry`            | ✅  |
| `Label`                  | `type Label`                       | ✅  |
| `MetricsConfig`          | `MetricsConfig`                    | ✅  |
| `PrometheusExporter`     | `PrometheusExporter`               | ✅ 完整 |
| `PushgatewayExporter`    | `PushgatewayExporter`              | ✅ stub |
| `OtlpExporter`           | `OtlpExporter`                     | ✅ stub |
| `StatsdExporter`         | `StatsdExporter`                   | ✅ stub |
| `StdoutExporter`         | `StdoutExporter`                   | ✅ 完整 |

## 状态

⚠️ **skeleton** (R20 阶段 6 估缺). 1 exporter 完整 (Prometheus), 1 exporter 完整 (Stdout), 3 exporter stub (Pushgateway/OTLP/StatsD).
