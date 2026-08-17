# R238 -- runtime OTel metrics integration

## Problem
\peireth-supervisor::otel_metrics\ has full Counter/Gauge/Histogram/Registry/Prometheus-text
export (10 tests pass). But runtime only USES supervisor's HeartbeatScheduler -- it never
EXPOSES metrics to any observability stack.

TUI/dashboard/debug tools cannot see:
- how many cycles ran / how many failed
- duration distribution
- task store total
- emotion decay emit frequency

## Solution: plug MetricsRegistry into Runtime

### 1. \Runtime\ adds 5 default metrics
- cycle_total (Counter)
- cycle_failures_total (Counter)
- decay_emit_total (Counter)
- cycle_duration_ms (Histogram)
- pending_tasks (Gauge, renamed runtime_total_tasks for accuracy)

### 2. run_one_cycle integration
head: cycle_total.inc()
on decay publish: decay_emit_total.inc()
tail: cycle_duration_ms.observe(elapsed_ms); pending_tasks.set(task_store.len().await)

### 3. metrics_text() exposes Prometheus format

## Tests (4 pass)
- r238_01..r238_04 (registry init, increment, text export, Arc ptr_eq sharing)

## Followup
- failure path counter inc
- HTTP /v1/runtime/metrics endpoint (via apeireth-api)

cumulative: ~6314 tests pass.