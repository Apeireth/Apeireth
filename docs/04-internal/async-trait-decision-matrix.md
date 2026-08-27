# async-trait vs Native async fn — 决策矩阵 (O-6 锚 #5 留项)

> **现状 (2026-08-27)**: v2 5 个 plugin 文件 (`crates/foundation/plugin/src/{manager, plugin, provider, registry, tool}.rs`) 用 `#[async_trait]` 宏.
> Rust toolchain = **1.97.1** (per `rust-toolchain.toml`), 已支持原生 `async fn` in traits (stabilized in 1.75).
> 决策待 v2.0.0-rc 阶段做 (per ROADMAP §4 P-arch 留项), 0 装诚实标注: 本文档只列矩阵, 不预先拍板.

```
[Document-Meta]
Document:        docs/04-internal/async-trait-decision-matrix.md
Version:         Decision-1.0
Last-Modified:   2026-08-27
Status:          🟡 待 v2.0.0-rc 阶段拍板
```

---

## 1. 决策维度

| # | 维度 | 当前 (`async_trait`) | 候选 A (native `async fn`) | 候选 B (`trait_variant`) |
|---|---|---|---|---|
| 1 | 工具链要求 | stable | stable (1.75+) | stable + extra dep |
| 2 | `Send` bounds | 自动添加 (impl `Send` Box) | 手动 `+ Send` / `+ !Send` | 自动 |
| 3 | 宏依赖 | `async-trait = workspace` | 0 宏依赖 | `trait-variant = workspace` |
| 4 | Cargo.toml 依赖 | 1 行 (workspace true) | 0 行 (可删 `async-trait`) | 1 行 (加 `trait-variant`) |
| 5 | trait method `Box<dyn Future>` 开销 | 有 (每次 dispatch 装箱) | 0 (直接 future) | 0 |
| 6 | trait 可以 `dyn Trait` | ❌ (native `async fn` in trait 也是 dyn-incompatible, 但 `async_trait` 提供 workaround via `Box<dyn Future>`) | ❌ (除非用 RPITIT + boxed return type, 1.75+) | ✅ (`trait_variant::make` 生成 sync + async 兼容 trait) |
| 7 | compile time | 慢 (宏展开) | 快 | 快 |
| 8 | binary size | 大 | 小 | 中 |
| 9 | test 友好度 (#[tokio::test]) | 容易 | 容易 (1.75+) | 容易 |
| 10 | ecosystem 兼容 (LLM SDK crate) | 标准 (claude-rs, anthropic-rs 都用) | 越来越多 | 少 |

---

## 2. v2 实际场景分析

### 2.1 plugin crate 5 个文件用了 `#[async_trait]`

| 文件 | trait | methods |
|---|---|---|
| `manager.rs` | `PluginManager` | `register` / `start` / `stop` / `dispatch` / `unregister` (async) |
| `plugin.rs` | `Plugin` trait | `start` / `stop` (async) |
| `provider.rs` | `ProviderCapability` | `complete` / `stream` (async) |
| `registry.rs` | `CapabilityRegistry` | `dispatch` (async) |
| `tool.rs` | `ToolCapability` | `invoke` / `invoke_frozen` (async) |

### 2.2 是否需要 `dyn Trait`

- `PluginManager` 用 `Arc<dyn PluginManager>` (`Arc<dyn ...>` 装箱) — 需要 dyn
- `CapabilityRegistry` 持 `HashMap<CapabilityId, Arc<dyn ToolCapability>>` — 需要 dyn
- runtime dispatch 走 `registry.dispatch(capability_id)` — 需要 dyn

**所以必须支持 `dyn Trait`**. 这是核心约束。

### 2.3 native `async fn` + `dyn` 的限制

- 原生 `async fn` in trait: 1.75+ 在 trait 里**可以**写 `async fn`, 但 trait 本身**仍 dyn-incompatible** (除非 RPITIT 配合 `Box<dyn Future>`)
- workaround: `trait Foo { fn method(&self) -> impl Future<...> + Send; }` — 但 `dyn Foo` 仍不行
- 真正的 dyn-compatible 方案: 用 `trait_variant::make` 生成 `trait FooSync { fn method(&self) -> Box<dyn Future<...> + Send + Unpin>; }`

### 2.4 候选 B (`trait_variant`) 评估

- 优点: dyn-compatible + 0 装箱开销 + 0 宏依赖
- 缺点: 多 1 个 dep (`trait-variant` ~5KB), 需要适配 trait method 签名 (`BoxFuture<'_, T>`)
- 适配工作: 5 个文件改 trait def + 改 impl + 改测试 (5 文件 × 平均 5 method = 25 处)

---

## 3. 三种路径 + 工作量 + 风险

| 路径 | 描述 | 工作量 | 风险 |
|---|---|---|---|
| **A. 保持 `async_trait`** (现状) | 0 改, 等 future 升级 | 0 | 0 |
| **B. native `async fn` in trait** (但 trait 不再 dyn-compatible) | 5 文件改, 需重写 registry 不用 `Arc<dyn ...>` | 1 周 (涉及 runtime 重构) | **极高** (破坏 100+ consumer) |
| **C. `trait_variant`** | 5 文件改 trait/impl + 测试 + 加 dep | 2-3 天 | **低** (dyn-compatible) |

---

## 4. v2.0.0-rc 拍板推荐

**推荐: 路径 A (保持 `async_trait`)**.

理由:
1. async-trait 是 Rust 生态事实标准 (tokio / hyper / axum / reqwest / sqlx 等都用)
2. v2 rc 阶段重点是**接真 backend** (RC-1 到 RC-10), 不是**优化 dispatch 开销**
3. dyn-compatible + 0 破坏 + 0 重构 + ecosystem 兼容
4. perf 优化路径 (路径 C `trait_variant`) 留给 v2.1 性能优化阶段 (有 perf baseline 后做)

**不推荐**: 路径 B (破坏 dyn, runtime 重构) — 任何收益都被破坏 100+ consumer 抵消.

**将来**: 如果 dispatch 真的成为瓶颈 (per benchmark), 路径 C `trait_variant` 是**渐进式**优化路径 (一个 trait 一个 trait 迁, 不破现状).

---

## 5. rc 阶段拍板流程 (v2.0.0-rc 启动时)

1. 跑 `cargo bench` 建立 dispatch 延迟 baseline (用 `crates/engine/runtime/benches/`)
2. 如果 baseline < 100μs/turn (L1 主观): 保持路径 A
3. 如果 baseline > 1ms/turn: 切路径 C (`trait_variant`) 优化
4. 任何时候不切路径 B (破坏太大)

---

## 6. 决策延迟的代价

按 O-6 锚 #9 "等以后做是借口" — **这条**决策**不**适用, 因为:

- 当前 `async_trait` 路径 perf 影响**未量化** (没 baseline)
- 真 baseline 在 rc 接 backend 后才能跑 (没 backend 谈 perf = 空对空)
- 这条决策的**前置条件**(backend 接通) 不在 alpha 阶段

**所以**: rc 启动后再拍, 是**正确**时机, 不是借口.

---

## 7. 一句话总结

保持 `async_trait` (现状), rc 接 backend 后跑 perf baseline 再决定. 路径 C `trait_variant` 是将来 perf 优化备选. 路径 B (破坏 dyn) **永不**采纳.

---

_本文 O-6 锚 #5 决策矩阵 v1 (2026-08-27): 5 plugin 文件用 async_trait 是现状; 切路径 C 需 2-3 天, 切路径 B 需 1 周且破坏 dyn. 拍板延迟到 v2.0.0-rc 是**正确**时机, 不是借口, 因为 perf baseline 依赖 backend 接通. 哲学锚 #9 "等以后做是借口"**不**适用本项: 不是借口, 是真前置条件._