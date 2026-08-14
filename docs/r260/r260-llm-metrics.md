# R260: LlmMetrics (OTel latency + counters for LlmWorker dispatch)

**日期**: 2026-08-14
**作者**: 楚零
**目的**: 给 LlmWorker dispatch 加 OTel 可观测性 (latency histogram + 4 counters)

---

## §1 范围

**新增**:
- `LlmMetrics` struct: 自包含 4 个 OTel metric + 独立 `MetricsRegistry`
- `Runtime.llm_metrics: LlmMetrics` 字段
- `metrics_text()` 合并 runtime + llm metrics 输出
- `dispatch_llm_task` 改造: spawn detached task 在 task 完成后 record metrics

**不修改**:
- `LlmWorker` (向后兼容, 0 触碰)
- `AsyncWorker` trait
- 现有 53 个 runtime tests (R255/r241/r242/r238/r246/r247/r250)

---

## §2 LlmMetrics 设计

```rust
pub struct LlmMetrics {
    pub registry: Arc<MetricsRegistry>,  // 独立 registry, 自包含
    pub requests_total: Arc<Counter>,    // 全 dispatch 调用次数 (含 errors)
    pub errors_total: Arc<Counter>,      // 返回 Err 的次数
    pub latency_ms: Arc<Histogram>,      // 端到端 wallclock 分布
}
```

**自包含 MetricsRegistry** — 不依赖 Runtime 主 registry。这样 LlmMetrics 可以独立 export、单独 subscribe、未来可挂载到独立 exporter。

### 2.1 记录 API

```rust
impl LlmMetrics {
    pub fn record_success(&self, latency_ms: f64) {
        self.requests_total.inc();
        self.latency_ms.observe(latency_ms);
    }
    pub fn record_error(&self, latency_ms: f64) {
        self.requests_total.inc();
        self.errors_total.inc();
        self.latency_ms.observe(latency_ms);
    }
}
```

---

## §3 dispatch_llm_task 集成

**改造**: dispatch_llm_task 不再同步 wait_for_completion，而是返回 TaskId 后让 caller 继续。**detached tokio::spawn** 在后台 wait_for_completion + record metrics:

```rust
pub async fn dispatch_llm_task(...) -> TaskId {
    let started = std::time::Instant::now();
    // ... construct worker + dispatch ...
    let task_id = self.dispatch_async_task_with_worker(worker, ...).await;
    let store = self.task_store.clone();
    let metrics = self.llm_metrics.clone();
    tokio::spawn(async move {
        let result = store.wait_for_completion(task_id, Duration::from_secs(120)).await;
        let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
        match result {
            Ok(rec) if rec.status == TaskStatus::Failed => metrics.record_error(latency_ms),
            Ok(_) => metrics.record_success(latency_ms),
            Err(_) => metrics.record_error(latency_ms),
        }
    });
    task_id
}
```

**decoupled design** — caller 拿到 TaskId 后立刻继续；metrics recording 在后台完成。延迟包含完整 request->response 周期。

---

## §4 测试 (6 cases)

- r260_01 llm_metrics initial zero (3 fields)
- r260_02 metrics_text includes llm counters (3 metric names appear)
- r260_03 record_success increments request + observes latency
- r260_04 record_error increments both counters (requests + errors)
- r260_05 llm_metrics clone shares state (Arc 共享)
- r260_06 dispatch_llm_task records after completion (端到端 + sleep 等待 detached task)

**64 tests pass total** (53 original + 5 R259 + 6 R260).

---

## §5 主哲学锚对齐

- **S-1 北极星**: 自包含 MetricsRegistry 不依赖 Runtime, 借鉴 OTel metric spec 但轻量级
- **S-2 实事求是**: detached spawn 后台 record, 不阻塞 caller
- **O-1 安全优先**: 不修改 LlmWorker (向后兼容, 0 触碰 async trait)
- **O-2 走在前人**: 0 新外部 dep, 复用 supervisor::otel_metrics
- **O-3 干到底**: 1 struct + 1 dispatch 改造 + 6 tests 全过
- **O-5 不假装**: tokens 字段不假装已知 (chat() 返回 content string 而非 usage), 仅记录 requests/errors/latency
