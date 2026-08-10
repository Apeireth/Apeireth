# 阶段 2 决策：技术栈选型 (2026-07-30)

> **范围**: R14 Rust 重写技术栈选型决策 (阶段 2 第一项, 子项 a-e)
> **触发**: 用户最新指示 "你的推荐不错，沉淀吧"
> **依据**: R11 现状 (Cargo.toml 已配) + 调研 (Hermes Agent Rust / OpenClaw / VCP / claude-mem 等) + 阶段 1 灵感
> **不修改承诺**: ❌ 不修改 crates/ 下已有代码 / ❌ 不修改 Cargo.toml / ❌ 不删除任何文件

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-tech-stack.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 1/12) |
| **依据** | R11 现状 + Hermes Agent Rust 110K+ Rust / OpenClaw / VCP / claude-mem 等调研 |
| **配套文档** | `inspiration-stage1-2026-07-30.md` (5 层原则洋葱 + 灵感产物) |
| **后续** | 阶段 2 子项 2: 核心架构形态 |

---

## 1. 5 项决策总览

| # | 项目 | 决策 | 状态 |
|---|------|------|------|
| a | Rust 版本 | **1.80 stable + edition 2021** | ✅ R11 已配, 保持 |
| b | 工具链 | **必装 + 加 cargo-deny/cargo-audit/cargo-nextest** | ⚠️ 部分增量 |
| c | 异步运行时 | **tokio 1.40 multi-thread** | ✅ R11 已配, 保持 |
| d | 序列化 | **配置 TOML / API JSON / 内部 bincode / 跨语言 rmp** | ⚠️ 部分新增 |
| e | 数据库 | **多样协同 + 设立 apeireth-data 抽象层** | ⚠️ 新增 crate |

---

## 2. 详细决策

### 2a. Rust 版本: 1.80 stable + edition 2021

**决策理由**:
- ✅ R11 已落 (`Cargo.toml` rust-version = "1.80")
- ✅ 生态成熟 (crates.io 上 95% 库已支持 1.80+)
- ✅ edition 2021 是当前 Rust 主流量
- ⚠️ edition 2024 (Rust 1.85+) 还在测试, 不冒险

**不冒险理由**:
- edition 2024 主要改进: `if let` 链式 + RPIT (return position impl trait) 改进 + 新的 trait 解析
- 我们是巨型基地, 不为新特性牺牲稳定性
- 未来 2026 Q4 edition 2024 稳定后, 可作为 R14 Phase 4+ 升级点

**Cargo.toml 配置**:
```toml
[workspace.package]
rust-version = "1.80"
edition = "2021"
```

**rust-toolchain.toml 配置**:
```toml
[toolchain]
channel = "stable"      # 自动跟随 1.80+ stable
components = ["rustfmt", "clippy", "rust-src"]
profile = "minimal"
```

### 2b. 工具链增量: 必装 + 加 3 个

**已有 (R11 已配)**:
- ✅ rustup (版本管理)
- ✅ cargo (构建/包管理)
- ✅ rustc (编译器)
- ✅ rustfmt (代码格式化)
- ✅ clippy (代码 lint)

**新增 (阶段 2 决策)**:
| 工具 | 干啥 | 为什么加 |
|------|------|---------|
| **cargo-deny** | 依赖检查 (许可证 + 安全漏洞 + 重复依赖) | 巨型基地需要严格依赖治理 |
| **cargo-audit** | 安全审计 (CVE 数据库) | E-3 不创造毁灭能力, 需审查依赖 |
| **cargo-nextest** | 更快测试运行 (并行 + 增量) | 巨型基地测试多, 需要速度 |

**可选 (团队自选)**:
- cargo-watch (自动重编译, 开发体验)
- miri (未定义行为检查)
- cargo-flamegraph (性能火焰图)

**安装命令** (Phase 0 准备阶段执行):
```bash
# 必装 (CI 必备)
cargo install cargo-deny --locked
cargo install cargo-audit --locked
cargo install cargo-nextest --locked

# .cargo/config.toml 配置 nextest
[target.x86_64-unknown-linux-gnu]
runner = "cargo-nextest run"
```

### 2c. 异步运行时: tokio 1.40 multi-thread

**决策理由**:
- ✅ R11 已配 (`tokio = "1.40"`, features = `["full"]`)
- ✅ 工业级 Rust 异步标准 (Hermes / OpenClaw / 大部分项目都用)
- ✅ 生态最大 (95% Rust 异步库基于 tokio)
- ✅ 多线程 runtime 适合巨型基地多 crate 并发

**features 选择** (`["full"]` 已包含):
- rt-multi-thread (多线程 runtime)
- macros (#[tokio::main])
- io-util / io-std
- sync (Mutex/RwLock/mpsc/broadcast/watch)
- time (timeout/interval)
- signal (Unix signal 处理)
- fs (异步文件 IO)
- net (TCP/UDP/Unix domain socket)

**巨型基地多线程策略**:
```
主线程          → 主 AI 主循环 (Sovereignty trait)
worker threads  → 智囊团 7 顾问 (Council trait)
IO threads      → 文件/网络 IO
timer threads   → cron / heartbeat / OTA check
block threads   → 阻塞调用 (PyO3 / FFI)
```

**不选其他 runtimes**:
- ❌ async-std: 已停更 (2025 年)
- ❌ smol: 生态小, 不适合巨型基地
- ❌ monoio: io_uring 优化是特定场景, 暂不需要

### 2d. 序列化分层: 按数据用途选

**决策**: 4 种格式按场景分工

| 用途 | 格式 | crate | 理由 |
|------|------|-------|------|
| **配置文件** | TOML | `toml = "0.8"` | 人类可读, Rust 生态标准 |
| **对外 API** | JSON | `serde_json` (已有) | 跨语言, 调试友好 |
| **内部 RPC** | bincode | `bincode = "1.3"` | 仅 Rust, 最快, 无 schema 开销 |
| **跨语言模块通信** | MessagePack / rmp | `rmp-serde = "1.3"` | 跨语言 + 二进制高效 |
| **持久化存储** | bincode (默认) / rmp (跨语言) | 同上 | 按用途切换 |
| **网络协议** | protobuf (可选) | `prost = "0.13"` | 强 schema, 跨语言 (未来用) |

**Cargo.toml 增量**:
```toml
[workspace.dependencies]
# 新增
toml = "0.8"
bincode = "1.3"
rmp-serde = "1.3"
# 已有
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**trait 接口设计** (R14 设计草案):
```rust
pub trait Serializer: Send + Sync {
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, SerializeError>;
    fn deserialize<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, DeserializeError>;
}

// 4 种实现
pub struct TomlSerializer;
pub struct JsonSerializer;
pub struct BincodeSerializer;
pub struct MessagePackSerializer;
```

**为什么不只用 JSON**:
- JSON 慢 (~10x bincode) + 大 (~3x bincode)
- 巨型基地内部 RPC 高频, JSON 不够快
- 持久化存储量大, JSON 浪费 IO

### 2e. 数据库: 多样协同 + apeireth-data 抽象层

**决策**: 多 DB 协同 + 设立 `apeireth-data` 候选 crate

**DB 选型矩阵**:

| 数据类型 | 推荐 DB | 候选 | 用途 |
|---------|---------|------|------|
| 结构化 + 事务 | **SQLite** | PostgreSQL | IdentityCard / AuditLog / Config |
| KV 高性能 | **RocksDB** | sled / LMDB | Memory index / Patch Archive |
| 语义检索 | **Qdrant** | Milvus / zvec | A 层联想 / 经验向量检索 |
| 全文检索 | **Tantivy** | Meilisearch | 知识图谱全文检索 |
| 知识图谱 | **VCP 浪潮网络** | Neo4j / SurrealDB | A 层联想 + 知识结构化 |
| 时序数据 | **InfluxDB / sled** | TimescaleDB | wallclock / telemetry 时序 |

**候选 crate `apeireth-data`** (新增, 阶段 2 第 3 项细化):

```
apeireth-data/
├── src/
│   ├── lib.rs                    # 统一 trait 抽象
│   ├── sqlite_backend.rs         # SQLite (结构化)
│   ├── rocksdb_backend.rs        # RocksDB (KV)
│   ├── qdrant_backend.rs         # Qdrant (向量)
│   ├── tantivy_backend.rs        # Tantivy (全文)
│   ├── wave_backend.rs           # VCP 浪潮 (知识图谱)
│   ├── timeseries_backend.rs     # 时序
│   ├── coordinator.rs            # 多 backend 协调
│   └── migration.rs              # 数据迁移
└── tests/
    ├── integration_tests.rs
    └── benchmark.rs
```

**统一 trait 设计**:
```rust
#[async_trait]
pub trait DataBackend: Send + Sync {
    async fn put(&self, key: &Key, value: &[u8]) -> Result<(), DataError>;
    async fn get(&self, key: &Key) -> Result<Option<Vec<u8>>, DataError>;
    async fn delete(&self, key: &Key) -> Result<(), DataError>;
    async fn query(&self, q: &Query) -> Result<Vec<Record>, DataError>;
    fn backend_type(&self) -> BackendType;
}

// 上层不绑死, 通过 coordinator 选择
pub struct DataCoordinator {
    backends: HashMap<BackendType, Arc<dyn DataBackend>>,
    routing: RoutingPolicy,
}

impl DataCoordinator {
    pub async fn store(&self, key: &Key, value: &[u8], hint: BackendHint) -> Result<(), DataError> {
        let backend = self.routing.route(hint);
        backend.put(key, value).await
    }
}
```

**与"协调统一 4 原则"的关系**:
- ✅ 多源共存: 多 DB 并存, 各自最优场景
- ✅ 抽象隔离: 通过 `DataBackend` trait, 上层不感知后端
- ✅ 可插拔: Phase 1 先 1-2 个 DB (SQLite + RocksDB), Phase 2+ 按需扩展
- ⚠️ 自研优先: VCP 浪潮网络可能部分自研 (阶段 2 决定)

**Cargo.toml 增量**:
```toml
[workspace.dependencies]
# 已有
rusqlite = { version = "0.32", features = ["bundled"] }
# 新增 (Phase 1 至少加 sled 作为 KV)
sled = "0.34"
# 新增 (Phase 2+ 加 Qdrant client)
qdrant-client = "1.7"
# 新增 (Phase 2+ 加 Tantivy)
tantivy = "0.22"
```

---

## 3. R11 现状 vs R14 增量

```
R11 现状:
  - Rust 1.80 + edition 2021 ✅
  - tokio 1.40 (full) ✅
  - serde + serde_json + anyhow + thiserror ✅
  - rusqlite + chrono + uuid ✅
  - pyo3 0.22 ✅
  - criterion + proptest ✅

R14 增量 (阶段 2 决策):
  - 加 cargo-deny + cargo-audit + cargo-nextest
  - 加 toml + bincode + rmp-serde (3 个序列化 crate)
  - 加 sled (Phase 1 KV)
  - 候选加 qdrant-client + tantivy (Phase 2+)
  - 新增候选 crate: apeireth-data (阶段 2 第 3 项细化)

主哲学 anchor (6 全贯穿):
  - 主 22:33 S-1 北极星导向 (技术栈服务于 ASI 方向)
  - 主 17:43 S-2 实事求是 (基于 R11 现状, 不重写)
  - 主 17:58 O-5 不假装 (cargo-deny/audit 防假依赖)
  - 主 19:33 O-2 走在前人经验上 (调研 Hermes/OpenClaw/VCP)
  - 主 23:44 O-3 干到底 (决策立刻沉淀)
  - 主 00:56 O-4 任何人都能接手 (决策可追溯)
```

---

## 4. 阶段 2 第一项收尾判定

技术栈 5 项决策已沉淀。R14 增量清晰, 不破坏 R11 现状。

**下一步**: 阶段 2 第二项 — **核心架构形态**

候选项 (单进程多线程 / 多进程 supervisor / 微服务 / actor / 异构):
- 阶段 1 已定: **多进程 + supervisor + actor + 异构** (灵感 §1.3)
- 但具体怎么组合? 单体进程 + 多线程? 多进程?

**待你确认**: 进入阶段 2 第二项 (核心架构形态)?

---

_主哲学 anchor 6 个全贯穿: 主 22:33 (S-1 技术栈服务 ASI 方向) + 主 17:43 (S-2 基于 R11 现状) + 主 17:58 (O-5 不假装, cargo-deny 防假依赖) + 主 19:33 (O-2 走在前人经验上) + 主 23:44 (O-3 干到底) + 主 00:56 (O-4 任何人都能接手)._
_技术栈 5 项决策已沉淀, R14 增量清晰. 下一步等用户确认进入阶段 2 第二项._