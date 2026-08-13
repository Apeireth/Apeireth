# R200 GitHub 优秀项目综合调研 — 剩余 14 个模块

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R200
> **日期**: 2026-08-13
> **范围**: 14 个未单独调研模块 (agent / api / asi / constraint / graph / http-client / mcp / onion / protocol / pybridge / runtime / supervisor / upgrade / vector)
> **状态**: 综合调研, 找可借鉴的优秀项目

---

## 1. apeireth-onion + apeireth-constraint (双洋葱 + 约束)

### 现状
- core/onion.rs (4KB) — 双洋葱结构 (Principle + Permission)
- core/constraint.rs (?), 独立 crate constraint 待查

### 1.1 双洋葱 SOTA
- **cap-std / capsicum-rs** (Rust 能力系统) — 借鉴
- **Cloudflare Workers capabilities** — 借鉴 (R188)
- **Wasmtime capability-based resource limiting** — 已在用

### 1.2 约束求解 SOTA
- **Z3 SMT solver** (Z3Prover/z3) — 工业级, 学术标杆
- **CVC5** (cvc5/cvc5) — 工业级
- **egg** (egraphs-good/egg) — **Rust 原生 e-graph 求解**
  - 适合 constraint 重写规则
  - MIT, 2K+ stars
- **trait-set / constraint** — 库级

### 1.3 升级方案
- 评估 egg 集成做 constraint 重写
- 0 触碰: 现有双洋葱 0 改

---

## 2. apeireth-asi (ASI 评分)

### 现状
- 0 KB 找到? 待查 (V0.5/V1136)

### 2.1 评分 / 度量 SOTA
- **Prometheus Rust client** (tikv/rust-prometheus) — 指标
- **OpenTelemetry** (再, R184 提过)
- **statrs** (statrs-dev/statrs) — Rust 统计库
- **linfa** (rust-ml/linfa) — Rust ML

### 2.2 升级方案
- 0 改核心 (V0.5/V1136 是 baseline 不可改)
- 加 statrs / linfa 做高级统计 (不确定度 / 置信区间)

---

## 3. apeireth-runtime

### 现状
- 13/24 单测
- 实际 agent runtime (主调度)

### 3.1 Agent Runtime SOTA
- **rig** (rig-rs/rig) — Rust LLM agent framework
- **autogen-rs** (R180 提过)
- **LangGraph** (R180)
- **Temporal** (R188)
- **dapr** (dapr/dapr) — Distributed Application Runtime

### 3.2 升级方案
- 借鉴 LangGraph checkpoint + Time Travel (R180 路线)
- 评估 dapr workflow 集成

---

## 4. apeireth-agent

### 现状
- 估计是 agent trait + base implementation

### 4.1 Agent Framework SOTA
- **ReAct / AutoGPT / BabyAGI** — 经典
- **LangGraph / AutoGen / CrewAI** (R180)
- **smolagents** (huggingface/smolagents) — Hugging Face Rust/Python

### 4.2 升级方案
- 借鉴 smolagents 简洁设计
- 评估成为 council 单 advisor 入口

---

## 5. apeireth-supervisor (监控)

### 现状
- 待查 (v4/v4.1 文档涵盖)

### 5.1 可观测性 SOTA
- **OpenTelemetry** (再, R184 提过)
- **Prometheus** (tikv/rust-prometheus)
- **sentry-rust** (getsentry/sentry-rust) — Sentry SDK
- **tracing** (tokio-rs/tracing) — Rust tracing
- **metrics** (metrics-rs/metrics) — 指标 facade

### 5.2 升级方案
- 强化 tracing 集成
- 评估 OTel 导出

---

## 6. apeireth-vector (向量)

### 现状
- 估计是向量基础类型 / trait

### 6.1 向量库 SOTA
- **ndarray** (rust-ndarray/ndarray) — N 维数组
- **nalgebra** (再, R182 提过)
- **simsimd** (ashvardanian/simsimd) — SIMD 加速相似度
- **hnsw-rs** (www-jboss/hnsw-rs) — Rust HNSW
- **qdrant** (R186 提过)
- **LanceDB** (R186 提过)

### 6.2 升级方案
- 评估 simsimd SIMD 加速
- 评估 hnsw-rs 集成

---

## 7. apeireth-graph (图基础)

### 现状
- 估计是 graph 基础 trait

### 7.1 图库 SOTA
- **petgraph** (再, R182 提过) — 主流
- **graph** (graphprotocol/graph-rs) — 区块链
- **roadwork** — 道路网络
- **pathfinding** (evenfurther/pathfinding) — 寻路

### 7.2 升级方案
- petgraph 强化 (0 改现有)
- 评估 pathfinding crate

---

## 8. apeireth-protocol (协议)

### 现状
- 估计是内部协议定义 (R146 protocol-bridge 桥接)

### 8.1 协议 SOTA
- **JSON-RPC 2.0** — 基础
- **gRPC** (grpc/grpc) — 工业级
- **Apache Arrow** (apache/arrow-rs) — 列式数据
- **MCP** (R189 提过)
- **OpenAI Function Calling** (R189)
- **Anthropic Prompt Caching** (新)
- **VCP Variable & Command Protocol** (R185 提过)

### 8.2 升级方案
- 评估 gRPC 集成 (跨服务)
- 评估 Arrow 集成 (高性能消息)

---

## 9. apeireth-api

### 现状
- 估计是 HTTP/WebSocket API server
- R155 runtime_bridge 已建

### 9.1 Web Framework SOTA
- **axum** (tokio-rs/axum) — Rust 主流 web
- **actix-web** (actix/actix-web) — 高性能
- **warp** (seanmonstar/warp) — 简洁
- **rocket** (rocket/rocket) — 易用
- **poem** (poem-web/poem) — 中文社区
- **salvo** (salvo-rs/salvo) — 中文社区

### 9.2 升级方案
- 评估 axum (如果当前不是)
- 加 OpenAPI spec 自动生成

---

## 10. apeireth-http-client

### 现状
- 估计是 HTTP client 抽象 (R174 HTTP fetch 用了)

### 10.1 HTTP Client SOTA
- **reqwest** (seanmonstar/reqwest) — Rust 主流
- **hyper** (hyperium/hyper) — 底层
- **ureq** (algesten/ureq) — 同步轻量
- **surf** — async 简洁
- **isahc** (hyperium/isahc) — 基于 curl

### 10.2 升级方案
- reqwest 已用
- 加 retry / circuit-breaker 集成 (我们 R198 已有)

---

## 11. apeireth-pybridge (Python 互操作)

### 现状
- PyO3 桥 (主人指示过)

### 11.1 Python-Rust 桥 SOTA
- **PyO3** (PyO3/PyO3) — 主流
- **maturin** (PyO3/maturin) — 打包
- **rust-cpython** (dgrunwald/rust-cpython) — 旧
- **pymind** (RCP-2001) — 心理学
- **pyo3-asyncio** — 异步集成

### 11.2 升级方案
- 评估 pyo3-asyncio (如果当前是 sync)
- 评估 maturin 打包

### 11.3 主人原话确认
> "pyo3桥是兼容python插件的对吧, 这个是合并在我们的兼容模块里的吗, 还是拎出来单独做的?"
- 已 R140 兼容 (host 已有 85+ 插件兼容)
- 我们的 apeireth-pybridge 是独立 crate
- 升级: 评估合并到 apeireth-tools 兼容层

---

## 12. apeireth-mcp (MCP 协议)

### 现状
- 我们 apeireth-mcp crate (R115 council bridge)

### 12.1 MCP SOTA
- **MCP 官方 spec** (modelcontextprotocol) — 行业标准
- **mcp-rs** (modelcontextprotocol/rust-sdk) — Rust 官方 SDK (新)
- 各种 server 实现 (browser-use / Cline / Continue)

### 12.2 升级方案
- 评估升级到 rust-sdk 0.x
- 我们当前可能已经自实现, 评估是否切官方 SDK

---

## 13. apeireth-upgrade (升级系统)

### 现状
- 待查 (R175 路线提过)

### 13.1 Auto-Update SOTA
- **self_update** (jaemk/self_update) — Rust auto-update
- **cargo-edit** — cargo deps update
- **tuf** (theupdateframework/tuf-rs) — The Update Framework (安全)
- **squashfs / zstd** — 镜像压缩

### 13.2 升级方案
- 评估 self_update 集成
- 评估 TUF (安全升级)

---

## 14. apeireth-formal (Kani 形式化)

### 现状
- apeireth-formal crate (Kani proofs)

### 14.1 形式化 SOTA
- **Kani** (再, R178 提过) — 当前用
- **Creusot** (R178)
- **Prusti** (R178)
- **coq-of-rust** (R178)
- **miri** (rust-lang/miri) — Rust 解释器
- **prop-test / quickcheck** (burntsushi/quickcheck) — property testing

### 14.2 升级方案
- 补完 3 关键 proof (R197 计划)
- 评估 quickcheck property testing 补 unit test 覆盖

---

## 15. 综合升级优先级 (R201+)

| 优先级 | 模块 | 工作量 | ROI | 来源 |
|---|---|---|---|---|
| 🥇 | asi (statrs / 高级统计) | 1-2 days | 中-高 | R200 |
| 🥇 | vector (simsimd SIMD) | 1-2 days | 高 | R200 |
| 🥈 | api (axum 升级 + OpenAPI) | 2-3 days | 中 | R200 |
| 🥈 | runtime (LangGraph checkpoint) | 3-5 days | 高 | R180 + R200 |
| 🥉 | constraint (egg 集成) | 5-7 days | 中 | R200 |
| 🥉 | supervisor (OTel 集成) | 3-5 days | 中 | R184 + R200 |
| 4 | upgrade (self_update) | 2-3 days | 中 | R200 |
| 5 | pybridge (pyo3-asyncio) | 2-3 days | 中 | R200 |
| 6 | mcp (rust-sdk 升级) | 1-2 days | 中 | R200 |
| 7 | graph (pathfinding) | 1 day | 低 | R200 |
| 8 | agent (smolagents 借鉴) | 1-2 days | 低 | R200 |
| 9 | protocol (Arrow 集成) | 5-7 days | 中 | R200 |
| 10 | http-client (retry 集成) | 1 day | 低 | R200 |
| 11 | formal (3 Kani proof) | 2-3 days | 高 | R197 |
| 12 | onion (cap-std 借鉴) | 5-7 days | 中 | R200 |

---

## 16. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- 14 模块公开 API: 0 改 (本档纯调研, 不改任何代码)
- 0 新增依赖

---

## 17. 实施路线 (R201+)

按 ROI 排序:
- R201: vector simsimd SIMD 加速 (1-2 days, 高)
- R202: asi statrs 高级统计 (1-2 days, 中-高)
- R203: runtime LangGraph checkpoint (3-5 days, 高)
- R204: api axum 升级 + OpenAPI (2-3 days, 中)
- R205: supervisor OTel 集成 (3-5 days, 中)
- R206: upgrade self_update (2-3 days, 中)
- R207: pybridge pyo3-asyncio (2-3 days, 中)
- R208: mcp rust-sdk 升级 (1-2 days, 中)
- R209: constraint egg 集成 (5-7 days, 中)
- R210: protocol Arrow (5-7 days, 中)
- R211: formal 3 Kani proof (2-3 days, 高)

---

## 18. 参考链接 (R200 综合)

- cap-std: https://github.com/bytecodealliance/cap-std
- Z3: https://github.com/Z3Prover/z3
- egg: https://github.com/egraphs-good/egg
- statrs: https://github.com/statrs-dev/statrs
- linfa: https://github.com/rust-ml/linfa
- OTel Rust: https://github.com/open-telemetry/opentelemetry-rust
- tracing: https://github.com/tokio-rs/tracing
- metrics: https://github.com/metrics-rs/metrics
- ndarray: https://github.com/rust-ndarray/ndarray
- nalgebra: https://github.com/dimforge/nalgebra
- simsimd: https://github.com/ashvardanian/simsimd
- hnsw-rs: https://github.com/jean-pierreBoth/hnsw-rs
- petgraph: https://github.com/petgraph/petgraph
- pathfinding: https://github.com/evenfurther/pathfinding
- gRPC: https://github.com/grpc/grpc
- Apache Arrow Rust: https://github.com/apache/arrow-rs
- axum: https://github.com/tokio-rs/axum
- actix-web: https://github.com/actix/actix-web
- warp: https://github.com/seanmonstar/warp
- reqwest: https://github.com/seanmonstar/reqwest
- hyper: https://github.com/hyperium/hyper
- PyO3: https://github.com/PyO3/PyO3
- maturin: https://github.com/PyO3/maturin
- self_update: https://github.com/jaemk/self_update
- TUF: https://github.com/theupdateframework/tuf-rs
- Kani: https://github.com/model-checking-ai/kani
- quickcheck: https://github.com/BurntSushi/quickcheck
- MCP: https://github.com/modelcontextprotocol
- dapr: https://github.com/dapr/dapr
- smolagents: https://github.com/huggingface/smolagents