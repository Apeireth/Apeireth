# 阶段 2 决策：持久化 (2026-07-30)

> **范围**: R14 Rust 重写持久化决策 (阶段 2 第六项) — apeireth-data 抽象层具体设计
> **触发**: 用户指示 "A" (我给推荐)
> **依据**: 阶段 2 §1e (多种 DB 协同 + 抽象层) + §13 协调统一 4 原则 + 巨型基地哲学
> **配套文档**: `stage2-decisions-tech-stack.md` §2e + `stage2-decisions-architecture.md` + `stage2-decisions-crate-split.md`

---

## 0. 元信息

| 字段 | 值 |
|------|-----|
| **文档路径** | `Apeireth-rust/docs/stage2-decisions-persistence.md` |
| **生成时间 (UTC)** | 2026-07-30 |
| **阶段** | 2 / 6 (子项 6/12) |
| **决策** | **6 DB 协同 + DataBackend trait + Coordinator + Migration + Saga 事务** |
| **候选 crate** | `apeireth-data` (阶段 2 §3 已列) |

---

## 1. 决策总览

```
6 DB 协同:
  MUST (Phase 1):  SQLite (结构化) + sled (KV, 替代 RocksDB 简化部署)
  SHOULD (Phase 2): Qdrant (向量) + Tantivy (全文)
  COULD (Phase 3):  VCP 浪潮 (联想网络, 自研) + 时序 (wallclock/telemetry)

统一抽象: DataBackend trait (1 接口)
协调: DataCoordinator (路由策略)
演进: Migration (schema 版本管理)
事务: 单 backend 强事务 + 跨 backend Saga (最终一致)
```

---

## 2. DB 选型矩阵 (按优先级)

### 2.1 MUST — Phase 1 实现

| DB | 用途 | 为什么 | crate |
|----|------|--------|-------|
| **SQLite** | 结构化 + 事务 (IdentityCard / AuditLog / Config / IdentityStore) | 嵌入式 + ACID + 生态成熟 | `rusqlite = "0.32"` (已有) |
| **sled** | KV (Memory index / Patch Archive / 黑板) | 纯 Rust + 嵌入式 + 高性能 | `sled = "0.34"` |

**为什么 sled 而不是 RocksDB**:
- ✅ 纯 Rust (RocksDB 是 C++, 编译复杂)
- ✅ 嵌入式 (RocksDB 需要额外的配置)
- ✅ 性能足够 (R11 V1130 验证)
- ⚠️ RocksDB 更成熟, 巨型基地 Phase 3 可换

### 2.2 SHOULD — Phase 2 实现

| DB | 用途 | 为什么 | crate |
|----|------|--------|-------|
| **Qdrant** | 向量检索 (A 层联想 / 经验向量) | 高性能 + Rust client + 云原生 | `qdrant-client = "1.7"` |
| **Tantivy** | 全文检索 (memory 文本 / audit log 搜索) | 纯 Rust + Lucene 替代 + 高性能 | `tantivy = "0.22"` |

### 2.3 COULD — Phase 3 实现

| DB | 用途 | 为什么 | crate |
|----|------|--------|-------|
| **VCP 浪潮** ⭐ | 联想网络 (A 层联想 / 知识图谱) | 自研, 完全适配 Apeireth | 自研 |
| **时序** | wallclock / telemetry | 时序数据天然适合 | `influxdb-client` 或自研 sled-based |

**VCP 浪潮自研理由**:
- VCP 联想网络是 apeireth 独有特性 (灵感 §12.3)
- 第三方图数据库 (Neo4j) 不适合联想网络
- 河道能量 + 神经网络信号传播 = 自研最合适

---

## 3. DataBackend trait 抽象

### 3.1 核心 trait

```rust
// apeireth-data/src/lib.rs

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key(pub Vec<u8>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub key: Key,
    pub value: Vec<u8>,
    pub metadata: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub enum Query {
    Prefix(Vec<u8>),          // 前缀扫描
    Range { start: Key, end: Key },
    Vector(Vec<f32>, usize), // 向量检索 (Qdrant 专用)
    FullText(String),        // 全文检索 (Tantivy 专用)
    Graph { from: Key, depth: usize }, // 图遍历 (VCP 浪潮专用)
    Sql(String),             // SQL 查询 (SQLite 专用)
}

#[async_trait]
pub trait DataBackend: Send + Sync {
    async fn put(&self, key: &Key, value: &[u8]) -> Result<(), DataError>;
    async fn get(&self, key: &Key) -> Result<Option<Vec<u8>>, DataError>;
    async fn delete(&self, key: &Key) -> Result<(), DataError>;
    async fn exists(&self, key: &Key) -> Result<bool, DataError>;
    async fn list(&self, prefix: &[u8]) -> Result<Vec<Record>, DataError>;
    async fn query(&self, q: &Query) -> Result<Vec<Record>, DataError>;
    
    /// 后端类型 (用于路由)
    fn backend_type(&self) -> BackendType;
    
    /// 事务 (可选, 不支持返回 NotSupported)
    async fn transaction<F, R>(&self, f: F) -> Result<R, DataError>
    where
        F: FnOnce(&dyn Transaction) -> BoxFuture<'_, Result<R, DataError>> + Send;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Sqlite,
    Sled,
    Qdrant,
    Tantivy,
    Wave,        // VCP 浪潮
    TimeSeries,
}

#[derive(Debug, Clone)]
pub enum BackendHint {
    Transactional,    // SQLite
    KeyValue,         // sled
    Vector,           // Qdrant
    FullText,         // Tantivy
    Graph,            // VCP 浪潮
    TimeSeries,       // 时序
}
```

### 3.2 6 种 backend 实现

```rust
// apeireth-data/src/sqlite.rs
pub struct SqliteBackend {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

#[async_trait]
impl DataBackend for SqliteBackend { /* SQLite 实现 */ }

// apeireth-data/src/sled_backend.rs
pub struct SledBackend {
    db: sled::Db,
}

#[async_trait]
impl DataBackend for SledBackend { /* sled 实现 */ }

// apeireth-data/src/qdrant_backend.rs
pub struct QdrantBackend {
    client: qdrant_client::client::QdrantClient,
    collection: String,
}

#[async_trait]
impl DataBackend for QdrantBackend { /* Qdrant 实现 */ }

// apeireth-data/src/tantivy_backend.rs
pub struct TantivyBackend {
    index: Arc<Mutex<tantivy::Index>>,
}

#[async_trait]
impl DataBackend for TantivyBackend { /* Tantivy 实现 */ }

// apeireth-data/src/wave_backend.rs (自研, Phase 3)
pub struct WaveBackend {
    // VCP 浪潮网络: 河道能量 + 联想网络
    nodes: Arc<RwLock<HashMap<Key, WaveNode>>>,
    rivers: Arc<RwLock<HashMap<Key, Vec<River>>>>,
}

#[async_trait]
impl DataBackend for WaveBackend { /* 浪潮实现 */ }

// apeireth-data/src/timeseries.rs
pub struct TimeSeriesBackend { /* 时序 */ }
```

---

## 4. DataCoordinator 路由

```rust
// apeireth-data/src/coordinator.rs

pub struct DataCoordinator {
    backends: HashMap<BackendType, Arc<dyn DataBackend>>,
    routing_policy: RoutingPolicy,
    default_backend: BackendType,
}

#[derive(Debug, Clone)]
pub enum RoutingPolicy {
    Explicit,      // 调用者指定 BackendType
    Hint,          // 按 BackendHint 自动推断
    Default,       // 用默认 backend
}

impl DataCoordinator {
    pub async fn put(
        &self,
        key: &Key,
        value: &[u8],
        hint: BackendHint,
    ) -> Result<(), DataError> {
        let backend_type = self.route(hint);
        let backend = self.backends.get(&backend_type).unwrap();
        backend.put(key, value).await
    }
    
    pub async fn get(
        &self,
        key: &Key,
        hint: BackendHint,
    ) -> Result<Option<Vec<u8>>, DataError> {
        let backend_type = self.route(hint);
        self.backends.get(&backend_type).unwrap().get(key).await
    }
    
    pub async fn query(
        &self,
        q: &Query,
        hint: BackendHint,
    ) -> Result<Vec<Record>, DataError> {
        let backend_type = self.route(hint);
        self.backends.get(&backend_type).unwrap().query(q).await
    }
    
    fn route(&self, hint: BackendHint) -> BackendType {
        match self.routing_policy {
            RoutingPolicy::Explicit => BackendType::from(hint),
            RoutingPolicy::Hint => BackendType::from(hint),
            RoutingPolicy::Default => self.default_backend,
        }
    }
}
```

### 4.1 上层使用示例

```rust
// apeireth-memory 使用
let coord = DataCoordinator::new()
    .with(SqliteBackend::open("memory.db")?, BackendType::Sqlite)
    .with(SledBackend::open("memory.kv")?, BackendType::Sled)
    .build();

// 写 A 层经验 (KV)
coord.put(
    &key("experience/123"),
    &serialize(&exp)?,
    BackendHint::KeyValue,
).await?;

// 全文检索 (Tantivy)
let results = coord.query(
    &Query::FullText("VCP 联想网络".into()),
    BackendHint::FullText,
).await?;
```

---

## 5. Migration 机制 (schema 演进)

```rust
// apeireth-data/src/migration.rs

#[async_trait]
pub trait Migration: Send + Sync {
    fn version(&self) -> u64;
    fn name(&self) -> &str;
    async fn up(&self, conn: &dyn DataBackend) -> Result<(), MigrationError>;
    async fn down(&self, conn: &dyn DataBackend) -> Result<(), MigrationError>;
}

pub struct MigrationRunner {
    migrations: Vec<Arc<dyn Migration>>,
}

impl MigrationRunner {
    /// 跑所有未执行的 migration
    pub async fn run(&self, backend: &dyn DataBackend) -> Result<(), MigrationError> {
        let current = self.current_version(backend).await?;
        for m in &self.migrations {
            if m.version() > current {
                tracing::info!(version = m.version(), name = m.name(), "running migration");
                m.up(backend).await?;
                self.set_version(backend, m.version()).await?;
            }
        }
        Ok(())
    }
    
    /// 回滚到指定版本
    pub async fn rollback(&self, backend: &dyn DataBackend, target: u64) -> Result<(), MigrationError> {
        let current = self.current_version(backend).await?;
        let mut to_rollback: Vec<_> = self.migrations
            .iter()
            .filter(|m| m.version() > target && m.version() <= current)
            .collect();
        to_rollback.sort_by_key(|m| std::cmp::Reverse(m.version()));
        for m in to_rollback {
            m.down(backend).await?;
            self.set_version(backend, m.version() - 1).await?;
        }
        Ok(())
    }
}

/// Migration 示例: V1 → V2 (加 IdentityCard.notes 字段)
pub struct V1ToV2Migration;

#[async_trait]
impl Migration for V1ToV2Migration {
    fn version(&self) -> u64 { 2 }
    fn name(&self) -> &str { "add_identitycard_notes" }
    
    async fn up(&self, conn: &dyn DataBackend) -> Result<(), MigrationError> {
        // SQLite: ALTER TABLE
        conn.execute("ALTER TABLE identity_card ADD COLUMN notes TEXT").await?;
        Ok(())
    }
    
    async fn down(&self, conn: &dyn DataBackend) -> Result<(), MigrationError> {
        // SQLite: 不支持 DROP COLUMN, 用重建表
        Ok(())
    }
}
```

**Migration 注册表**:
```rust
pub fn all_migrations() -> Vec<Arc<dyn Migration>> {
    vec![
        Arc::new(V1InitialMigration),
        Arc::new(V1ToV2Migration),
        Arc::new(V2ToV3Migration),
        // ...
    ]
}
```

---

## 6. 事务边界 (跨 backend)

### 6.1 单 backend 强事务

```rust
// SQLite / sled 自带事务
backend.transaction(|tx| async move {
    tx.put(&key1, &val1).await?;
    tx.put(&key2, &val2).await?;
    Ok(())
}).await?;
```

### 6.2 跨 backend — Saga pattern (最终一致)

**为什么不用分布式事务 (2PC / 3PC)**:
- ❌ 巨型基地哲学反对"为了完美一致性牺牲可用性"
- ❌ 跨进程 2PC 实现复杂, 性能差
- ❌ Saga 更简单, 最终一致足够

```rust
// apeireth-data/src/saga.rs

pub struct SagaStep {
    pub name: String,
    pub action: Action,           // 正向操作
    pub compensation: Compensation, // 补偿操作
}

pub enum Action {
    PutSqlite { key: Key, value: Vec<u8> },
    PutSled { key: Key, value: Vec<u8> },
    PutQdrant { ... },
}

pub enum Compensation {
    DeleteSqlite { key: Key },
    DeleteSled { key: Key },
    DeleteQdrant { ... },
}

pub struct Saga {
    steps: Vec<SagaStep>,
}

impl Saga {
    /// 执行 saga, 任一步骤失败则回滚前面的步骤
    pub async fn execute(&self, coord: &DataCoordinator) -> Result<(), SagaError> {
        let mut completed: Vec<&SagaStep> = vec![];
        
        for step in &self.steps {
            match self.run_action(&step.action, coord).await {
                Ok(()) => completed.push(step),
                Err(e) => {
                    // 回滚
                    tracing::warn!("saga step {} failed, rolling back", step.name);
                    for completed_step in completed.iter().rev() {
                        if let Err(rollback_err) = self.run_compensation(&completed_step.compensation, coord).await {
                            tracing::error!("rollback failed: {}", rollback_err);
                        }
                    }
                    return Err(e);
                }
            }
        }
        Ok(())
    }
    
    async fn run_action(&self, action: &Action, coord: &DataCoordinator) -> Result<(), SagaError> {
        match action {
            Action::PutSqlite { key, value } => {
                coord.put(key, value, BackendHint::Transactional).await?;
            }
            // ...
        }
        Ok(())
    }
    
    async fn run_compensation(&self, comp: &Compensation, coord: &DataCoordinator) -> Result<(), SagaError> {
        match comp {
            Compensation::DeleteSqlite { key } => {
                coord.delete(key, BackendHint::Transactional).await?;
            }
            // ...
        }
        Ok(())
    }
}
```

### 6.3 事务策略决策表

| 业务场景 | 策略 | 理由 |
|---------|------|------|
| 配置文件加载 | 单 SQLite 事务 | 强一致 |
| AuditLog 写入 | 单 SQLite 事务 | 强一致 |
| IdentityCard 更新 | 单 SQLite 事务 | 强一致 |
| Memory index 写 + 向量写 | Saga | 最终一致可接受 |
| A 层经验沉淀 (联想网络) | 不需要事务 | 联想是异步的 |
| 跨进程 plugin 状态 | 不需要事务 | eventual consistency |
| 升级意图 (manifest) | 单 SQLite 事务 | 强一致 (E-3 守门) |

---

> **[TODO-WAVE-REPOSITION 阶段 3+ 启动前]** — Wave 从 Data backend 候选移动为 retrieval pipeline 的 Association Engine (依据 `research-vcp-rerun-2026-07-31.md` §4.7)。Wave 不再是 apeireth-data 的 backend 实现, 而是 apeireth-memory 的 retrieval pipeline 组件 (联想引擎, 与 VCP 引力式信息流对齐)。**不删原文不动原措辞**, 修订 = §4 DataBackend trait + §10 决策对比表 同步标注 Wave 重定位 + 跨引 §4.7 research-vcp-rerun。**[TODO-OWNER]** architect + backend_engineer + database_engineer。**[TODO-STAGE]** 阶段 3+ (画图纸后) 启动前。

## 7. apeireth-data crate 结构

```
apeireth-data/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # 公共 trait + re-exports
│   ├── key.rs                    # Key 类型
│   ├── record.rs                 # Record 类型
│   ├── query.rs                  # Query 枚举
│   ├── backend.rs                # DataBackend trait + BackendType
│   ├── coordinator.rs            # DataCoordinator 路由
│   ├── migration.rs              # Migration trait + Runner
│   ├── saga.rs                   # Saga pattern 跨 backend
│   ├── backends/
│   │   ├── mod.rs
│   │   ├── sqlite.rs             # SQLite 实现
│   │   ├── sled_backend.rs       # sled 实现
│   │   ├── qdrant.rs             # Qdrant 实现
│   │   ├── tantivy.rs            # Tantivy 实现
│   │   ├── wave.rs               # VCP 浪潮 (Phase 3 自研)
│   │   └── timeseries.rs         # 时序
│   └── error.rs                  # DataError 错误类型
├── tests/
│   ├── integration_sqlite.rs
│   ├── integration_sled.rs
│   ├── integration_qdrant.rs
│   ├── coordinator_test.rs
│   └── saga_test.rs
└── benches/
    └── benchmark.rs
```

---

## 8. Cargo.toml 增量

```toml
[workspace.dependencies]
# 已有
rusqlite = { version = "0.32", features = ["bundled"] }

# 新增 (Phase 1)
sled = "0.34"

# 新增 (Phase 2)
qdrant-client = "1.7"
tantivy = "0.22"

# 新增 (Phase 3 自研)
# VCP 浪潮: 在 apeireth-data/src/backends/wave.rs 自研
# 时序: influxdb-client 或自研 sled-based

[dependencies.async-trait]
version = "0.1"
```

---

## 9. 阶段 2 第六项收尾判定

持久化已沉淀: **6 DB 协同 + DataBackend trait + Coordinator + Migration + Saga 事务**。

**关键设计**:
- ✅ Phase 1: SQLite + sled (MUST)
- ✅ Phase 2: Qdrant + Tantivy (SHOULD)
- ✅ Phase 3: VCP 浪潮 (自研) + 时序 (COULD)
- ✅ DataBackend trait 统一接口
- ✅ DataCoordinator 路由 (Explicit/Hint/Default)
- ✅ Migration 机制 (up/down + version 管理)
- ✅ 单 backend 强事务 + 跨 backend Saga (最终一致)

**R14 增量**:
- 新增 `apeireth-data` crate (阶段 2 §3 已列)
- 必须实现: SqliteBackend + SledBackend (Phase 1)
- 应实现: QdrantBackend + TantivyBackend (Phase 2)
- 可实现: WaveBackend (自研, Phase 3)

**主哲学 anchor (6 全贯穿)**:
- 主 22:33 S-1 (持久化服务 ASI 方向)
- 主 17:43 S-2 (基于真实需求, Phase 1-3 渐进)
- 主 17:58 O-5 (强事务用于 E-3 守门)
- 主 19:33 O-2 (Saga 是成熟模式)
- 主 23:44 O-3 (干到底)
- 主 00:56 O-4 (任何接手者能查)

**下一步**: 阶段 2 第七项 — **LLM 集成**

---

## 10. 决策对比表

| 方案 | 一致性 | 复杂度 | 性能 | 推荐 |
|------|--------|--------|------|------|
| 单 DB (只 SQLite) | ✅ 强 | 低 | 中 | ❌ 不够灵活 |
| 多 DB + 手动管理 | ⚠️ 中 | 中 | 高 | ⚠️ 容易乱 |
| **多 DB + DataBackend trait + Coordinator** | ✅ | 中 | 高 | ✅✅ |
| 多 DB + 分布式事务 (2PC) | ✅ 强 | 高 | 低 | ❌ 太重 |

**Apeireth 选多 DB + DataBackend + Coordinator**:
- 强一致用单 backend 事务
- 最终一致用 Saga
- 不引入分布式事务

---

_主哲学 anchor 6 个全贯穿. 持久化已沉淀. 下一步等用户确认进入阶段 2 第七项 (LLM 集成)._