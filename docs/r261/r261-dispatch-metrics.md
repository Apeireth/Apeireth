# R261: DispatchMetrics (通用 dispatch counter + latency histogram)

**日期**: 2026-08-14
**作者**: 楚零
**目的**: 给所有 AsyncWorker dispatch 加 OTel 通用 metrics (覆盖 SimulatedWorker + LlmWorker + custom)

---

## §1 范围

**新增**:
- `DispatchMetrics` struct: counter + errors counter + latency histogram
- `Runtime.dispatch_metrics: DispatchMetrics` 字段
- `dispatch_async_task_with_worker` 集成 metrics record
- `metrics_text()` 合并 dispatch metrics 输出

**不修改**:
- `dispatch_async_task` (R147 主循环 inline spawn, 不调 _with_worker) — 留给后续 R 接入
- 现有 64 个 runtime tests (R255/r241/r242/r238/r246/r247/r250/R259/R260)

---

## §2 DispatchMetrics vs LlmMetrics

| 维度 | LlmMetrics (R260) | DispatchMetrics (R261) |
|---|---|---|
| Counter | requests_total + errors_total | dispatch_total + dispatch_errors_total |
| Histogram | latency_ms (chat API wallclock) | dispatch_latency_ms (worker.execute wallclock) |
| 覆盖范围 | 仅 LlmWorker | 所有 AsyncWorker (LlmWorker / SimulatedWorker / custom) |
| 集成点 | dispatch_llm_task (detached spawn) | dispatch_async_task_with_worker (inline spawn) |

**两者互补**: LlmMetrics 跟踪 LLM API 调用的端到端 latency (含网络), DispatchMetrics 跟踪 worker.execute 的 wallclock latency (含业务逻辑).

---

## §3 dispatch_async_task_with_worker 集成

```rust
pub async fn dispatch_async_task_with_worker(...) -> TaskId {
    let started = std::time::Instant::now();
    self.dispatch_metrics.dispatch_total.inc();
    // ... existing register + mark_running + clone handles ...
    let metrics = self.dispatch_metrics.clone();
    tokio::spawn(async move {
        let result = worker.execute(task_id, params_owned).await;
        let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
        match &result {
            Ok(json) => {
                metrics.record_success(latency_ms);
                // ... existing store.complete + bus.publish ...
            }
            Err(_) => {
                metrics.record_error(latency_ms);
                let _ = store.fail(task_id, err.clone()).await;
            }
        }
    });
    task_id
}
```

**关键设计**: metrics record 在 spawn 的 worker.execute 完成后立即进行 (Ok/Err 两路都 record), 不依赖 caller 的 wait_for_completion. 这样 metrics 是 fire-and-forget 的, 不阻塞 caller.

---

## §4 测试 (5 cases)

- r261_01 dispatch_metrics initial zero (3 fields 0)
- r261_02 metrics_text includes dispatch counters (3 metric names appear)
- r261_03 dispatch_async_task_with_worker records (端到端: dispatch_total inc + latency observed)
- r261_04 record_success increments total (1 counter)
- r261_05 record_error increments both counters (requests + errors)

**69 tests pass total** (53 original + 5 R259 + 6 R260 + 5 R261).

---

## §5 主哲学锚对齐

- **S-1 北极星**: 借鉴 OTel metric spec, 自包含 MetricsRegistry, 跟 R260 LlmMetrics 风格一致
- **S-2 实事求是**: 0 引外部 dep, fire-and-forget metrics record (不阻塞 caller)
- **O-1 安全优先**: 不修改 AsyncWorker trait, 通过 dispatch_async_task_with_worker 包装接入
- **O-2 走在前人**: 复用 supervisor::otel_metrics
- **O-3 干到底**: 1 struct + 1 dispatch 改造 + 5 tests 全过
- **O-5 不假装**: metrics 是真实 worker.execute 时间, 不是 estimated/predicted
