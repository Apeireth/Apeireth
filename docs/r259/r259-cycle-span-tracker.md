# R259: Cycle Span Tracker (OTel-style 自研轻量级)

**日期**: 2026-08-14
**作者**: 楚零
**目的**: 给 Runtime 加 Span 追踪能力，借鉴 OTel spec 但 0 引外部 dep

> 主人 8 14 拍板: 调研差不多就停。R259 选 _frozen/apeireth-tracing 复活评估外**不复活** (3685 行 vs 现实需求不匹配), 自研轻量级 SpanTracker 更优。

---

## §1 范围

**新增**:
- `apeireth-supervisor::span` 模块: `SpanId` / `SpanStatus` / `SpanEvent` / `SpanTracker`
- `Runtime::span_tracker: Arc<SpanTracker>` 字段
- `CycleReport::spans: Vec<SpanEvent>` 字段 (`#[serde(default)]` 向后兼容)
- `run_one_cycle_inner` 集成 5 spans: cycle_root + task.dispatch + task.complete + search.index + emotion.apply

**不复活**:
- `_frozen/apeireth-tracing` (3685 行, 包含 exporter/OTLP/sampler 等分布式追踪, 单进程场景下过度)
- 评估结论: 自研 ~400 行 (span.rs) + 现有 otel_metrics.rs (398 行) = 完整可观测栈

---

## §2 设计

### 2.1 SpanEvent (OTel Span 字段映射)

| OTel 字段 | SpanEvent 字段 | 备注 |
|---|---|---|
| name | name: String | span 名 |
| span_id | span_id: SpanId(u64) | 全局单调 |
| parent_span_id | parent: Option<SpanId> | None = root |
| start_time_unix_nano | start_unix_ms: u64 | ms 精度足够 |
| end_time_unix_nano | end_unix_ms: u64 | 0 = pending |
| status.code | status: SpanStatus | Ok/Err/Unset |
| attributes | attrs: Vec<(String, String)> | 简单 K-V |

**trace_id 不在 SpanEvent 字段**: 跨 span 关联用 CycleReport.trace_id (一个 cycle 一个 trace)。

### 2.2 SpanTracker API

| API | 语义 |
|---|---|
| `new()` / `with_capacity(pending, completed)` | 构造, 默认 1024 + 4096 |
| `start_span(parent, name) -> Option<SpanId>` | 开 span, 容量满返 None |
| `end_span(id, status, attrs) -> Option<SpanEvent>` | 关 span, 移入 completed |
| `take_completed() -> Vec<SpanEvent>` | 提取+清空 completed |
| `active_count() -> usize` | pending 计数 |
| `completed_len() -> usize` | 已完成未提取 计数 |

### 2.3 Runtime 集成

```
run_one_cycle_inner()
  ├─ start_span(None, "runtime.cycle")  -> cycle_span
  ├─ start_span(cycle_span, "task.dispatch")
  ├─ end_span(task.dispatch, Ok, [task_id])
  ├─ start_span(cycle_span, "task.complete")
  ├─ end_span(task.complete, Ok, [task_id, status, tool])
  ├─ start_span(cycle_span, "search.index")
  ├─ end_span(search.index, Ok, [doc_id])
  ├─ start_span(cycle_span, "emotion.apply")
  ├─ end_span(emotion.apply, Ok, [dominant, intensity])
  ├─ take_completed()  -> child spans
  ├─ end_span(cycle, Ok, [trace_id, elapsed_ms])
  ├─ take_completed()  -> root span (1 个)
  └─ report.spans = [root_span, ...child_spans]  (root 在 index 0)
```

---

## §3 测试 (15 supervisor span + 5 runtime integration = 20 new)

### supervisor/span.rs (15 cases)
- t01 root span default parent
- t02 root span via ROOT const
- t03 child span parent set
- t04 end_span returns event
- t05 end_span unknown returns none
- t06 take_completed clears
- t07 max_pending capacity
- t08 max_completed drops oldest
- t09 attrs extend on end
- t10 span_id display
- t11 default impl
- t12 parent filter drops root
- t13 thread-safe shared tracker (10 threads)
- t14 status default unset
- t15 attrs keyed lookup

### runtime/lib.rs (5 cases)
- r259_01 cycle_report spans populated (5 spans: root + 4 child)
- r259_02 spans form parent-child tree
- r259_03 span attributes record metadata (task.complete.status, emotion.apply.dominant)
- r259_04 cycle_report legacy serde compat (pre-R259 JSON deserialize)
- r259_05 spans have nonzero elapsed when completed

---

## §4 主哲学锚对齐

- **S-1 北极星**: 借鉴 OTel spec 但 0 引外部 dep, 自研轻量级优于 3685 行 apeireth-tracing 复活
- **S-2 实事求是**: 评估 _frozen/apeireth-tracing 实际不需要 (单进程无需 distributed propagation)
- **O-1 安全优先**: spans 不写日志/不发 bus, 仅作为 CycleReport 字段返回给调用方
- **O-2 走在前人**: 0 新外部 dep, 全 std + workspace 已有 serde + tokio
- **O-3 干到底**: 1 模块 (span.rs) + 1 集成 (runtime) + 20 tests 全过
- **O-5 不假装**: 明确标注 self-rolled SpanTracker 非 distributed tracing impl, 真接 OTLP/W3C 时走外部

---

## §5 不复活评估结论

| 选项 | 评估 |
|---|---|
| 复活 _frozen/apeireth-tracing (3685 行) | ✗ 过度工程: exporter/OTLP/sampler 单进程不需要 |
| 引入 `tracing` crate (0.1) | ✗ 增加 dep, 已被 supervisor::otel_metrics 局部替代 |
| 引入 `opentelemetry` crate | ✗ 同上, dep 膨胀 |
| **自研 SpanTracker (400 行)** | ✓ 0 dep, 完整满足 cycle span 需求, 借鉴 OTel spec 但简化 |

**结论**: R259 不复活 apeireth-tracing，自研更契合"一体化优美"理念。
