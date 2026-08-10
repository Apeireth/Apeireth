# apeireth-pybridge

> **职责**: PyO3 兼容桥 (现有 1100+ Python 模块)
> **状态**: R11 占位实现 (LOCKED, 不砍)
> **对应文档**: 阶段 2 §8 模块化 + §3 兼容组件层

---

## 设计意图

`apeireth-pybridge` 是 Apeireth 的"Python 兼容桥"crate, **核心 LOCKED**:

1. **PyO3 绑定** — `pyo3 = "0.22"` (已有)
2. **现有 1100+ Python 模块** — R11 已落, R14 不砍
3. **子进程隔离** — Python GIL thread (阶段 2 §4)
4. **manifest.toml 注册** — 阶段 2 §8 plugin 系统

## 关键决策 (R11 LOCKED)

- ❌ **不砍 1100 空壳模块** (R14 重写时清理, 现在不动)
- ❌ **不重写 Python 兼容层** (PyO3 已落)
- ✅ **可以**通过 PyO3 bridge 接入
- ✅ **可以**作为 plugin subprocess 启动

## Cargo.toml

```toml
pyo3 = { version = "0.22", features = ["auto-initialize"] }
```

## Python → Rust 接入模式

```rust
use apeireth_pybridge::*;

// 启动 Python 子进程
let py = apeireth_pybridge::Python::new();
let module = py.import("apeireth.memory.store")?;

// 调用 Python 函数
let result = module.call("retrieve", (query,))?;
```

## 阶段 2 增强

- 增强 PyO3 GIL thread (阶段 2 §4 进程/线程分工)
- 增强 plugin manifest (阶段 2 §8 模块化)
- 增强 subprocess 通信 (阶段 2 §9 L4 WebSocket)

---

_主哲学 anchor: 主 17:43 实事求是 (R11 LOCKED 不动) + 主 19:33 走在前人经验上 (PyO3 生态)._