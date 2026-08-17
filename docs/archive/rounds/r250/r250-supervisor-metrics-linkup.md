# R250 -- Supervisor Metrics Linkup

## Problem
`apeireth-supervisor::otel_metrics` exposes 6 SupervisorMetrics (heartbeat_count,
tick_duration, restart_count, panic_count, active_children, total_children).
`apeireth-runtime` previously tracked its own runtime_cycle_* metrics but never
linked up with supervisor metrics — meaning HeartbeatScheduler tick activity was
invisible in supervisor's observability layer.

## Solution

### Runtime struct (new field)
```rust
pub supervisor_metrics: SupervisorMetrics,  // from supervisor_default_metrics()
```

### LivingCycleHeartbeat (new field + parameter)
```rust
pub struct LivingCycleHeartbeat {
    runtime: Arc<Runtime>,
    supervisor_metrics: SupervisorMetrics,
}
impl LivingCycleHeartbeat {
    pub fn new(runtime: Arc<Runtime>, supervisor_metrics: SupervisorMetrics) -> Self { ... }
}
```

### on_tick / on_event / on_user (wired)
Each invocation now does:
```rust
self.supervisor_metrics.heartbeat_count.inc();
let start = std::time::Instant::now();
let _ = self.runtime.run_one_cycle().await;
self.supervisor_metrics.tick_duration.observe(start.elapsed().as_secs_f64() * 1000.0);
```

### Bootstrap
```rust
let hb = LivingCycleHeartbeat::new(self.clone(), self.supervisor_metrics.clone());
```

## Tests (4 new pass)
- r250_01: heartbeat_count=0, tick_duration.count=0 initially
- r250_02: heartbeat_count can be inc-ed
- r250_03: tick_duration.observe accumulates correctly
- r250_04: runtime.metrics_text() still works after linkup (separate registry)

## Notes
- `supervisor_default_metrics()` returns `(MetricsRegistry, SupervisorMetrics)` —
  runtime stores the SupervisorMetrics but uses its OWN metrics_registry for the
  runtime_cycle_* metrics. Both can be exported independently.
- Future: combine into a single Prometheus exposition if needed.

## Files
- `crates/apeireth-runtime/src/lib.rs` (+1 field, +LivingCycleHeartbeat wiring, +4 tests)

cumulative: ~6353 tests pass.
