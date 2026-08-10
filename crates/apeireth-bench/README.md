# apeireth-bench

> **职责**: 性能基准 (V1130 wallclock + 多场景)
> **状态**: R11 占位实现
> **对应文档**: 阶段 2 §5 内存布局 (性能目标) + 阶段 2 §7 LLM (V1130 wallclock)

---

## 设计意图

`apeireth-bench` 是 Apeireth 的"性能仪表"crate, 包含:

1. **V1130 wallclock** — 单次 LLM 调用 < 2.5s (目标)
2. **Memory 检索** — < 50ms (1k 条)
3. **Audit log 追加** — < 5ms
4. **跨进程 memory 共享** — < 10ms (SharedMem)

## 阶段 2 §5 性能目标

| 操作 | 当前 R11 | R14 目标 | 启用机制 |
|------|----------|----------|---------|
| 单次 LLM 调用 | 8.7s | < 2.5s | A + D (零拷贝 token 流) |
| Memory 检索 (1k 条) | 200ms | < 50ms | A + D (SIMD 搜索) |
| Audit log 追加 | 50ms | < 5ms | B (arena) |
| 跨进程 memory 共享 | 100ms | < 10ms | C (SharedMem) |

## Cargo.toml

```toml
criterion = { version = "0.5", features = ["html_reports"] }
```

---

_主哲学 anchor: 主 17:43 实事求是 (基于真实数据) + 主 19:33 走在前人经验上 (Hermes 1,428 tests 标准)._