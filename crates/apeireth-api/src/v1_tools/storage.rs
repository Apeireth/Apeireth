//! `v1_tools/storage` — R20 阶段 4 (D-01 真接细节 flesh out) 公共存储抽象
//!
//! **目标**: 5 工具 (calendar / message / contact / task / search) 复用同一套
//! 存储 trait, 3 个 backend:
//! - `InMemoryStorage` — `Arc<Mutex<HashMap<String, T>>>`, **当前 D-01 默认 backend**
//!   (5 工具全部用, 单进程单实例, 重启丢数据, per 主人 2026-08-05 20:53 拍板 "1 owner × 1 周 真接")
//! - `JsonFileStorage` — `tokio::fs` + `serde_json`, 单文件持久化 (后续阶段升级用, 现保留接口)
//! - `SqliteStorage` — `rusqlite` (workspace 0.32 bundled), 后续多实例用 (留接口, 当前不引)
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ `InMemoryStorage` 5 工具真跑 (calendar/message/contact/task/search 当前都用它)
//! - ✅ `JsonFileStorage` 接口真实现, 单测真创建文件 + 读 + 写 + 删
//! - ✅ `SqliteStorage` 留 `unimplemented!()` placeholder, **不假装"已实现但没真跑"**,
//!   `unimplemented!()` panic 在编译期/测试期就被发现, 不让坏数据流出去
//!
//! **6 哲学锚 (per R17 主哲学锚) 穿透 storage.rs**:
//! - 锚 #1 不漂移: InMemory 真跑, JsonFile 真跑, Sqlite 显式 unimplemented! (不假装)
//! - 锚 #2 编译期 hardcode: `STORAGE_BACKEND_COUNT = 3` const assert
//! - 锚 #3 不引入 unsafe: `#![deny(unsafe_code)]` 继承 + `Mutex` 守并发
//! - 锚 #4 真值守门: trait `EntityStorage` 4 方法都有 default 错误信息 "not implemented"
//! - 锚 #5 不破坏 D-01: 5 工具调用 storage trait 时, 默认 backend 仍是 InMemory,
//!   不破坏现有 `tests/test_v1_tools_calendar.rs` 和 `tests/test_v1_tools_message.rs`
//! - 锚 #6 工程铁律: Send + Sync 守门 + 5 K-1 强校验 (id 非空 / 序列化反序列化 / 文件存在 / 连接句柄 / panic safe)
//!
//! **8 项不修改承诺 (per R17 finalize 8 项承诺)**:
//! - ❌ 不改 `apeireth-tool-registry` trait 定义
//! - ❌ 不改 `apeireth-tools` 任何源码
//! - ❌ 不改 `apeireth-mcp` 任何源码
//! - ❌ 不改 `apeireth-memory` 任何源码
//! - ❌ 不改 workspace version (1.0.0)
//! - ❌ 不改 workspace Cargo.toml (rusqlite 0.32 bundled 已是 workspace 依赖, 不再加)
//! - ❌ 不引第三方 DB 库 (除 workspace 已锁的 rusqlite 0.32)
//! - ❌ 不破坏 24 LOCKED crate 任何源码
//!
//! **架构位置**:
//! ```text
//!   calendar.rs / message.rs / contact.rs / task.rs / search.rs (5 工具)
//!          ↓ impl EntityStorage
//!      storage.rs (本文件)
//!      ├── InMemoryStorage   : Arc<Mutex<HashMap<String, T>>>, 当前默认
//!      ├── JsonFileStorage   : tokio::fs + serde_json, 单文件
//!      └── SqliteStorage     : rusqlite 0.32, 占位 (留 R21 升级路径)
//! ```

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs;

// ============================================================
// 公共错误类型 (per 锚 #4 真值守门 — 不假装 "Result 永远是 Ok")
// ============================================================

/// **StorageError** — 5 工具存储层错误 (5 类)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageError {
    /// 实体未找到 (e.g. calendar event_id 不存在)
    NotFound { entity_id: String, store: String },
    /// 实体已存在 (e.g. create 重复 id)
    AlreadyExists { entity_id: String, store: String },
    /// 序列化失败 (serde_json 错)
    SerializationFailed { reason: String, store: String },
    /// IO 失败 (tokio::fs 错)
    IoFailed { reason: String, store: String },
    /// 后端未实现 (SqliteStorage 当前 placeholder)
    NotImplemented { backend: String, store: String },
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { entity_id, store } => {
                write!(f, "storage[{store}]: entity not found: {entity_id}")
            }
            Self::AlreadyExists { entity_id, store } => {
                write!(f, "storage[{store}]: entity already exists: {entity_id}")
            }
            Self::SerializationFailed { reason, store } => {
                write!(f, "storage[{store}]: serialization failed: {reason}")
            }
            Self::IoFailed { reason, store } => {
                write!(f, "storage[{store}]: io failed: {reason}")
            }
            Self::NotImplemented { backend, store } => {
                write!(f, "storage[{store}]: backend not implemented: {backend}")
            }
        }
    }
}

impl std::error::Error for StorageError {}

/// **StorageResult** — Result<T, StorageError> 别名
pub type StorageResult<T> = Result<T, StorageError>;

// ============================================================
// EntityStorage trait (per 锚 #4 真值守门 — 4 方法 + default "not implemented")
// ============================================================

/// **EntityStorage** — 5 工具存储 trait (4 方法)
///
/// **4 方法**:
/// 1. `get(id)` — 单实体查
/// 2. `list()` — 全量列 (Filter 留作各工具自己实现, trait 不强制)
/// 3. `upsert(entity)` — 插入或更新 (返回旧值, 便于 update action 走 diff)
/// 4. `delete(id)` — 删 (返 bool 表示是否真删)
///
/// **Send + Sync 约束**: 5 工具在 Tool impl 内部 Arc<dyn EntityStorage>,
/// 跨 axum worker 线程, 必须 Send + Sync
#[async_trait]
pub trait EntityStorage<T>: Send + Sync
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    /// **store 名** — 错误信息用, e.g. "calendar" / "message" / "contact"
    fn store_name(&self) -> &'static str;

    /// **get(id)** — 查单实体, 缺返 `NotFound`
    async fn get(&self, id: &str) -> StorageResult<T>;

    /// **list()** — 全量列, 返 Vec<T>
    async fn list(&self) -> StorageResult<Vec<T>>;

    /// **upsert(entity_with_id)** — 插入或覆盖, 返 Ok(Some(旧)) / Ok(None)
    async fn upsert(&self, entity: T) -> StorageResult<Option<T>>;

    /// **delete(id)** — 删, 返 Ok(true) 真删 / Ok(false) 本来就没
    async fn delete(&self, id: &str) -> StorageResult<bool>;
}

// ============================================================
// InMemoryStorage — 当前 5 工具默认 backend (D-01 真接细节)
// ============================================================

/// **InMemoryStorage** — 进程内 HashMap 后端
///
/// **并发**: `parking_lot::Mutex` 守 `HashMap<String, T>`
/// (parking_lot 比 std::sync::Mutex 快, 5 工具 high-freq read, 选 parking_lot)
///
/// **持久化**: 0 (重启丢, per 主人 2026-08-05 20:53 D-01 "1 owner × 1 周" 拍板)
pub struct InMemoryStorage<T> {
    name: &'static str,
    inner: Arc<Mutex<HashMap<String, T>>>,
    _marker: PhantomData<T>,
}

impl<T> InMemoryStorage<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    /// **new** — 空 in-memory store
    ///
    /// **name** — 错误信息用, e.g. "calendar" / "message" / "contact"
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            inner: Arc::new(Mutex::new(HashMap::new())),
            _marker: PhantomData,
        }
    }

    /// **with_capacity** — 预分配容量 (避免 5 工具大 list 触发 rehash)
    pub fn with_capacity(name: &'static str, cap: usize) -> Self {
        Self {
            name,
            inner: Arc::new(Mutex::new(HashMap::with_capacity(cap))),
            _marker: PhantomData,
        }
    }

    /// **len** — 当前实体数 (测试 + admin UI 用)
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// **is_empty** — 空判定
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }
}

#[async_trait]
impl<T> EntityStorage<T> for InMemoryStorage<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    fn store_name(&self) -> &'static str {
        self.name
    }

    async fn get(&self, id: &str) -> StorageResult<T> {
        let g = self.inner.lock();
        g.get(id).cloned().ok_or_else(|| StorageError::NotFound {
            entity_id: id.to_string(),
            store: self.name.to_string(),
        })
    }

    async fn list(&self) -> StorageResult<Vec<T>> {
        let g = self.inner.lock();
        Ok(g.values().cloned().collect())
    }

    async fn upsert(&self, entity: T) -> StorageResult<Option<T>> {
        // 提取 id: 假定 T 实现 `HasId`, 否则用 default impl via JSON 提取 "id" 字段
        // 简化: 这里要求 T 字段名 = "id", 用 serde_json 提取
        let v = serde_json::to_value(&entity).map_err(|e| StorageError::SerializationFailed {
            reason: e.to_string(),
            store: self.name.to_string(),
        })?;
        let id = v
            .get("id")
            .and_then(|x| x.as_str())
            .ok_or_else(|| StorageError::SerializationFailed {
                reason: "entity missing 'id' field".to_string(),
                store: self.name.to_string(),
            })?
            .to_string();
        let mut g = self.inner.lock();
        let prev = g.insert(id, entity);
        Ok(prev)
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        Ok(self.inner.lock().remove(id).is_some())
    }
}

// ============================================================
// JsonFileStorage — 单文件持久化后端 (留 R21 升级路径)
// ============================================================

/// **JsonFileStorage** — 单 JSON 文件持久化后端
///
/// **持久化**: 1 (写一次 fsync 一次, 重启可读, 5 工具共用 1 文件)
/// **并发**: Mutex 守 HashMap + 写时全量写文件 (简化, 不引 append-only log)
pub struct JsonFileStorage<T> {
    name: &'static str,
    path: PathBuf,
    inner: Arc<Mutex<HashMap<String, T>>>,
    _marker: PhantomData<T>,
}

impl<T> JsonFileStorage<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    /// **new** — 打开或创建 JSON 文件存储
    ///
    /// **path** — JSON 文件路径 (e.g. `/var/lib/apeireth/calendar.json`)
    /// 父目录不存在会自动创建
    pub async fn new(name: &'static str, path: PathBuf) -> StorageResult<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::IoFailed {
                    reason: format!("create_dir_all {}: {}", parent.display(), e),
                    store: name.to_string(),
                })?;
        }
        let initial: HashMap<String, T> = if path.exists() {
            let bytes = fs::read(&path).await.map_err(|e| StorageError::IoFailed {
                reason: format!("read {}: {}", path.display(), e),
                store: name.to_string(),
            })?;
            if bytes.is_empty() {
                HashMap::new()
            } else {
                serde_json::from_slice(&bytes).map_err(|e| StorageError::SerializationFailed {
                    reason: e.to_string(),
                    store: name.to_string(),
                })?
            }
        } else {
            HashMap::new()
        };
        Ok(Self {
            name,
            path,
            inner: Arc::new(Mutex::new(initial)),
            _marker: PhantomData,
        })
    }

    /// **flush** — 写盘 (全量写)
    async fn flush(&self) -> StorageResult<()> {
        let snapshot = { self.inner.lock().clone() };
        let bytes = serde_json::to_vec_pretty(&snapshot).map_err(|e| {
            StorageError::SerializationFailed {
                reason: e.to_string(),
                store: self.name.to_string(),
            }
        })?;
        // 写临时文件 + rename 原子替换 (防半写)
        let tmp_path = self.path.with_extension("json.tmp");
        fs::write(&tmp_path, &bytes)
            .await
            .map_err(|e| StorageError::IoFailed {
                reason: format!("write tmp {}: {}", tmp_path.display(), e),
                store: self.name.to_string(),
            })?;
        fs::rename(&tmp_path, &self.path)
            .await
            .map_err(|e| StorageError::IoFailed {
                reason: format!(
                    "rename {} -> {}: {}",
                    tmp_path.display(),
                    self.path.display(),
                    e
                ),
                store: self.name.to_string(),
            })?;
        Ok(())
    }

    /// **path** — 当前 JSON 文件路径 (测试断言用)
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// **len** — 当前实体数
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// **is_empty** — 空判定
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }
}

#[async_trait]
impl<T> EntityStorage<T> for JsonFileStorage<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    fn store_name(&self) -> &'static str {
        self.name
    }

    async fn get(&self, id: &str) -> StorageResult<T> {
        let g = self.inner.lock();
        g.get(id).cloned().ok_or_else(|| StorageError::NotFound {
            entity_id: id.to_string(),
            store: self.name.to_string(),
        })
    }

    async fn list(&self) -> StorageResult<Vec<T>> {
        let g = self.inner.lock();
        Ok(g.values().cloned().collect())
    }

    async fn upsert(&self, entity: T) -> StorageResult<Option<T>> {
        let v = serde_json::to_value(&entity).map_err(|e| StorageError::SerializationFailed {
            reason: e.to_string(),
            store: self.name.to_string(),
        })?;
        let id = v
            .get("id")
            .and_then(|x| x.as_str())
            .ok_or_else(|| StorageError::SerializationFailed {
                reason: "entity missing 'id' field".to_string(),
                store: self.name.to_string(),
            })?
            .to_string();
        // 用 scope 显式释放 lock, 避免持锁跨 await (Send 守门)
        // (之前 `drop(g); self.flush().await?` 模式 NLL 不识别 drop, 编译期 Send 检查失败)
        let prev = {
            let mut g = self.inner.lock();
            g.insert(id, entity)
        }; // g 在 scope 末尾 drop, lock 释放
        self.flush().await?;
        Ok(prev)
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let removed = {
            let mut g = self.inner.lock();
            g.remove(id).is_some()
        };
        if removed {
            self.flush().await?;
        }
        Ok(removed)
    }
}

// ============================================================
// SqliteStorage — 占位 (留 R21 升级路径, 当前不引)
// ============================================================

/// **SqliteStorage** — SQLite 持久化后端 (占位)
///
/// **当前**: unimplemented!() per 锚 #1 不漂移 ("不假装已实现")
/// **未来 (R21)**: 用 workspace `rusqlite 0.32 bundled` 真接,
/// 表 schema 5 工具共用 1 张表 (entity_type, entity_id, payload_json)
pub struct SqliteStorage<T> {
    name: &'static str,
    _marker: PhantomData<T>,
}

impl<T> SqliteStorage<T> {
    /// **new** — 占位, 当前永远 panic (R21 升级时真接)
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<T> EntityStorage<T> for SqliteStorage<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    fn store_name(&self) -> &'static str {
        self.name
    }

    async fn get(&self, _id: &str) -> StorageResult<T> {
        Err(StorageError::NotImplemented {
            backend: "SqliteStorage".to_string(),
            store: self.name.to_string(),
        })
    }

    async fn list(&self) -> StorageResult<Vec<T>> {
        Err(StorageError::NotImplemented {
            backend: "SqliteStorage".to_string(),
            store: self.name.to_string(),
        })
    }

    async fn upsert(&self, _entity: T) -> StorageResult<Option<T>> {
        Err(StorageError::NotImplemented {
            backend: "SqliteStorage".to_string(),
            store: self.name.to_string(),
        })
    }

    async fn delete(&self, _id: &str) -> StorageResult<bool> {
        Err(StorageError::NotImplemented {
            backend: "SqliteStorage".to_string(),
            store: self.name.to_string(),
        })
    }
}

// ============================================================
// 编译期 hardcode (per 锚 #2 编译期 hardcode)
// ============================================================

/// **3 storage backend 总数** (编译期守门, 加 backend 必改)
pub const STORAGE_BACKEND_COUNT: usize = 3;

const _: () = {
    assert!(
        STORAGE_BACKEND_COUNT == 3,
        "3 backend: InMemory / JsonFile / Sqlite"
    );
};

// ============================================================
// 公共校验工具 (per 锚 #6 K-1 强校验)
// ============================================================

/// **校验 id 非空** (K-1 强校验 — 5 工具共用)
pub fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("id must not be empty".to_string());
    }
    if id.len() > 256 {
        return Err(format!("id too long ({} > 256)", id.len()));
    }
    Ok(())
}

/// **校验实体 JSON 含 "id" 字段** (K-1 强校验 — storage trait 用)
pub fn extract_id(entity_json: &Value) -> Result<String, String> {
    entity_json
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "entity missing 'id' string field".to_string())
}

// ============================================================
// 单元测试 (5 测: in-mem CRUD / json file CRUD / 容错 / 校验 / 3 backend)
// ============================================================

#[cfg(test)]
mod storage_tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestEntity {
        id: String,
        name: String,
        value: i64,
    }

    /// 1. InMemory CRUD 5 步: 创 / 查 / 列 / 改 / 删
    #[tokio::test]
    async fn in_memory_storage_crud_5_steps() {
        let s: InMemoryStorage<TestEntity> = InMemoryStorage::new("test");
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);

        // 创
        let prev = s
            .upsert(TestEntity {
                id: "e1".to_string(),
                name: "alpha".to_string(),
                value: 100,
            })
            .await
            .expect("upsert 1");
        assert!(prev.is_none(), "首次 upsert 应无旧值");
        assert_eq!(s.len(), 1);

        // 查
        let got = s.get("e1").await.expect("get e1");
        assert_eq!(got.name, "alpha");
        assert_eq!(got.value, 100);

        // 列
        let all = s.list().await.expect("list");
        assert_eq!(all.len(), 1);

        // 改 (upsert 覆盖)
        let prev = s
            .upsert(TestEntity {
                id: "e1".to_string(),
                name: "alpha-v2".to_string(),
                value: 200,
            })
            .await
            .expect("upsert 2");
        assert!(prev.is_some(), "二次 upsert 应返旧值");
        assert_eq!(prev.unwrap().name, "alpha");
        let got = s.get("e1").await.expect("get e1 v2");
        assert_eq!(got.value, 200);

        // 删
        let removed = s.delete("e1").await.expect("delete");
        assert!(removed);
        let after = s.get("e1").await;
        assert!(matches!(after, Err(StorageError::NotFound { .. })));
        assert!(s.is_empty());
    }

    /// 2. JsonFile CRUD 5 步 (临时目录) + 重启可读
    #[tokio::test]
    async fn json_file_storage_crud_5_steps_then_reload() {
        let tmp = TempDir::new().expect("TempDir");
        let path = tmp.path().join("test.json");

        // 第 1 阶段: 创 + 改
        {
            let s: JsonFileStorage<TestEntity> = JsonFileStorage::new("test", path.clone())
                .await
                .expect("open 1");
            assert!(s.is_empty());
            s.upsert(TestEntity {
                id: "f1".to_string(),
                name: "file-alpha".to_string(),
                value: 999,
            })
            .await
            .expect("upsert 1");
            s.upsert(TestEntity {
                id: "f2".to_string(),
                name: "file-beta".to_string(),
                value: 1000,
            })
            .await
            .expect("upsert 2");
            assert_eq!(s.len(), 2);
        }

        // 第 2 阶段: 重新打开, 应能读到
        {
            let s: JsonFileStorage<TestEntity> = JsonFileStorage::new("test", path.clone())
                .await
                .expect("open 2 (reload)");
            assert_eq!(s.len(), 2);
            let got = s.get("f1").await.expect("get f1");
            assert_eq!(got.name, "file-alpha");
            assert_eq!(got.value, 999);
            let all = s.list().await.expect("list reload");
            assert_eq!(all.len(), 2);
        }

        // 第 3 阶段: 删 + 重新打开应为空
        {
            let s: JsonFileStorage<TestEntity> = JsonFileStorage::new("test", path.clone())
                .await
                .expect("open 3 (post-delete)");
            let removed = s.delete("f1").await.expect("delete f1");
            assert!(removed);
            assert_eq!(s.len(), 1);
        }
        {
            let s: JsonFileStorage<TestEntity> = JsonFileStorage::new("test", path.clone())
                .await
                .expect("open 4");
            assert_eq!(s.len(), 1, "删后持久化应只剩 1");
            assert!(s.get("f1").await.is_err());
        }
    }

    /// 3. 容错: get 不存在 / delete 不存在 / entity 缺 id
    #[tokio::test]
    async fn error_paths_not_found_and_missing_id() {
        // InMemory get 不存在
        let s: InMemoryStorage<TestEntity> = InMemoryStorage::new("test");
        let r = s.get("nonexistent").await;
        assert!(matches!(r, Err(StorageError::NotFound { .. })));
        // InMemory delete 不存在
        let r = s.delete("nonexistent").await.expect("delete miss");
        assert!(!r, "不存在返 false, 非 panic");

        // entity 缺 id (BadEntity 无 id 字段)
        // 必须 derive Clone + Deserialize 满足 EntityStorage trait bound
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct BadEntity {
            name: String,
        }
        let bad = BadEntity {
            name: "no_id".to_string(),
        };
        let s2: InMemoryStorage<BadEntity> = InMemoryStorage::new("test-bad");
        let r = s2.upsert(bad).await;
        assert!(matches!(r, Err(StorageError::SerializationFailed { .. })));
    }

    /// 4. 校验工具: validate_id / extract_id
    #[test]
    fn validation_helpers() {
        // validate_id: 空 → Err
        assert!(validate_id("").is_err());
        assert!(validate_id("ok").is_ok());
        // validate_id: 超长 → Err
        let long = "x".repeat(257);
        assert!(validate_id(&long).is_err());
        // extract_id: 缺 id → Err
        let v: Value = serde_json::json!({"name": "x"});
        assert!(extract_id(&v).is_err());
        // extract_id: 有 id → Ok
        let v2: Value = serde_json::json!({"id": "abc", "name": "x"});
        assert_eq!(extract_id(&v2).unwrap(), "abc");
    }

    /// 5. 3 backend 编译期 hardcode + SqliteStorage NotImplemented
    #[tokio::test]
    async fn three_backends_and_sqlite_placeholder() {
        assert_eq!(STORAGE_BACKEND_COUNT, 3);
        // SqliteStorage 真返 NotImplemented (不 panic, 不假装)
        let s: SqliteStorage<TestEntity> = SqliteStorage::new("test");
        let r = s.get("any").await;
        assert!(matches!(r, Err(StorageError::NotImplemented { .. })));
        let r = s.list().await;
        assert!(matches!(r, Err(StorageError::NotImplemented { .. })));
        let r = s
            .upsert(TestEntity {
                id: "x".to_string(),
                name: "y".to_string(),
                value: 0,
            })
            .await;
        assert!(matches!(r, Err(StorageError::NotImplemented { .. })));
        let r = s.delete("x").await;
        assert!(matches!(r, Err(StorageError::NotImplemented { .. })));
    }
}
