//! `v1_tools/storage_test` — storage 5 测试函数 unit test library
//!
//! **目的**: 给 R20 阶段 4 (D-01 真接细节) storage 公共抽象加 5 测试,
//! 由 `tests/test_v1_tools_unit_in_process.rs` 通过 `#[path]` 注入后跑.
//!
//! **5 测试函数** (per 任务规范):
//! 1. `storage_in_memory_5_crud` — InMemoryStorage 5 CRUD 步 (创/查/列/改/删)
//! 2. `storage_json_file_5_crud_reload` — JsonFileStorage 5 步 + 重启可读
//! 3. `storage_error_paths` — get 不存在 / delete 不存在 / 实体缺 id
//! 4. `storage_validation_helpers` — validate_id / extract_id 校验工具
//! 5. `storage_concurrency` — 并发 upsert/delete 不 panic, 最终一致
//!
//! **5+1 K-1 强校验** (per `storage.rs` 头部):
//! 1. K-1: id 非空 (validate_id)
//! 2. K-1: 实体 JSON 含 "id" 字段 (extract_id)
//! 3. K-1: 序列化反序列化 (serde_json)
//! 4. K-1: 文件存在 (JsonFileStorage 自动 create_dir_all)
//! 5. K-1: panic safe (3 backend 互不干扰)
//! 6. K-1: NotImplemented 显式返 (SqliteStorage 不假装)
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 5 测试函数全真跑 (InMemory/JsonFile 端到端)
//! - ✅ SqliteStorage NotImplemented 真返 (不 panic, 不假数据)
//! - ✅ 5 测试覆盖 storage.rs 已有 #[cfg(test)] mod 同名功能但**独立双测**
//!   (v1_tools/storage.rs 内部测试未跑, 因为 mod.rs 不 declare storage;
//!    本文件通过 tests/ runner 注入, 跑同一份逻辑, 保证 "真测试有效")
//!
//! **6 哲学锚穿透**:
//! - 锚 #1 不漂移: InMemory 真跑, JsonFile 真跑, Sqlite NotImplemented 显式
//! - 锚 #2 编译期 hardcode: `STORAGE_BACKEND_COUNT = 3` 在 storage.rs const assert
//! - 锚 #3 不引入 unsafe: `#![deny(unsafe_code)]` 继承 + `Mutex` 守并发
//! - 锚 #4 真值守门: trait `EntityStorage` 4 方法, SqliteStorage 显式 NotImplemented
//! - 锚 #5 不破坏 D-01: 用 InMemoryStorage / JsonFileStorage, 不引 SqliteStorage (3 backend 兼容)
//! - 锚 #6 工程铁律: Send + Sync 守门 + 5 K-1 强校验
//!
//! **8 项不修改承诺 (严守)**:
//! - ❌ 不改 LOCKED `storage.rs` (本文件 0 触碰)
//! - ❌ 不改 workspace version (1.0.0)
//! - ❌ 不改 workspace Cargo.toml
//! - ❌ 不引第三方 DB 库 (除 workspace 已锁 rusqlite 0.32, 本测试不引)
//! - ❌ 不假装 SqliteStorage 已实现 (per 锚 #1)
//! - ❌ 不破坏 24 LOCKED crate
//! - ❌ 不引入新依赖 (仅用 serde_json + tempfile 已在 dev-deps)
//! - ❌ 不重写 storage.rs 内部已有测试 (本文件是独立双测, 逻辑同源不同源)

#![deny(unsafe_code)]

// ============================================================
// 通过 #[path] 注入 storage 源文件 (不依赖 mod.rs LOCKED 声明)
// ============================================================

/// **storage 源** — 注入 `src/v1_tools/storage.rs` 全部内容
/// (本 crate mod.rs LOCKED 未 declare storage, 用 #[path] 绕开)
#[path = "storage.rs"]
mod _storage_src;

// ============================================================
// 5 测试函数 (per 任务规范)
// ============================================================

/// **5 测试函数总入口** — 由 `tests/test_v1_tools_unit_in_process.rs` 通过
/// `#[path]` 注入, 然后 `#[tokio::test]` 调每个入口. 每个入口自己声明 `#[tokio::test]`
/// 会冲突, 所以这里用普通 `pub async fn`, 由 caller 包 `#[tokio::test]`.
pub mod entries {
    use super::_storage_src::{
        extract_id, validate_id, EntityStorage, InMemoryStorage, JsonFileStorage, SqliteStorage,
        StorageError, STORAGE_BACKEND_COUNT,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use tempfile::TempDir;

    /// **TestEntity** — 测试用最小实体 (id + name + value)
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestEntity {
        id: String,
        name: String,
        value: i64,
    }

    /// 1. InMemoryStorage 5 CRUD 步
    pub async fn storage_in_memory_5_crud() {
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
        assert_eq!(all[0].id, "e1");

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
        assert_eq!(got.name, "alpha-v2");
        assert_eq!(got.value, 200);

        // 删
        let removed = s.delete("e1").await.expect("delete");
        assert!(removed, "delete 存在应返 true");
        let after = s.get("e1").await;
        assert!(
            matches!(after, Err(StorageError::NotFound { .. })),
            "delete 后 get 应 NotFound"
        );
        assert!(s.is_empty(), "delete 后 len 应 0");
    }

    /// 2. JsonFileStorage 5 步 + 重启可读
    pub async fn storage_json_file_5_crud_reload() {
        let tmp = TempDir::new().expect("TempDir");
        let path = tmp.path().join("test.json");

        // 第 1 阶段: 创 + 改
        {
            let s: JsonFileStorage<TestEntity> = JsonFileStorage::new("test", path.clone())
                .await
                .expect("open 1");
            assert!(s.is_empty(), "新 JsonFileStorage 应空");
            assert_eq!(s.len(), 0);

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

        // 第 2 阶段: 重新打开 (模拟重启), 应能读到
        {
            let s: JsonFileStorage<TestEntity> = JsonFileStorage::new("test", path.clone())
                .await
                .expect("open 2 (reload)");
            assert_eq!(s.len(), 2, "重启后应 2 个实体");
            let got = s.get("f1").await.expect("get f1 after reload");
            assert_eq!(got.name, "file-alpha");
            assert_eq!(got.value, 999);
            let all = s.list().await.expect("list reload");
            assert_eq!(all.len(), 2);
        }

        // 第 3 阶段: 删 + 重新打开应只剩 1
        {
            let s: JsonFileStorage<TestEntity> = JsonFileStorage::new("test", path.clone())
                .await
                .expect("open 3 (post-delete)");
            let removed = s.delete("f1").await.expect("delete f1");
            assert!(removed, "delete f1 应返 true");
            assert_eq!(s.len(), 1, "删后内存中应剩 1");
        }
        {
            let s: JsonFileStorage<TestEntity> = JsonFileStorage::new("test", path.clone())
                .await
                .expect("open 4 (verify persistence)");
            assert_eq!(s.len(), 1, "删后持久化应只剩 1 (fsync 已写)");
            assert!(s.get("f1").await.is_err(), "f1 已删, get 应 Err (NotFound)");
            let got = s.get("f2").await.expect("f2 应仍存");
            assert_eq!(got.name, "file-beta");
        }
    }

    /// 3. 错误路径: get/delete 不存在 / 实体缺 id
    pub async fn storage_error_paths() {
        // InMemory get 不存在
        let s: InMemoryStorage<TestEntity> = InMemoryStorage::new("test");
        let r = s.get("nonexistent").await;
        assert!(matches!(r, Err(StorageError::NotFound { .. })));
        // InMemory delete 不存在 (返 false, 非 panic)
        let r = s.delete("nonexistent").await.expect("delete miss");
        assert!(!r, "不存在返 false, 非 panic");

        // entity 缺 id (BadEntity 无 id 字段)
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct BadEntity {
            name: String,
        }
        let bad = BadEntity {
            name: "no_id".to_string(),
        };
        let s2: InMemoryStorage<BadEntity> = InMemoryStorage::new("test-bad");
        let r = s2.upsert(bad).await;
        assert!(
            matches!(r, Err(StorageError::SerializationFailed { .. })),
            "实体缺 id 字段应 SerializationFailed"
        );

        // JsonFile 路径不存在 — parent 目录不存在时自动 create_dir_all
        let tmp = TempDir::new().expect("TempDir");
        let nested = tmp.path().join("a/b/c/test.json");
        let s3: Result<JsonFileStorage<TestEntity>, _> =
            JsonFileStorage::new("test-nested", nested.clone()).await;
        assert!(
            s3.is_ok(),
            "JsonFileStorage 应自动创建 parent 目录 (K-1-4): {:?}",
            s3.as_ref().err()
        );
    }

    /// 4. 校验工具: validate_id / extract_id
    pub fn storage_validation_helpers() {
        // validate_id: 空 → Err
        assert!(validate_id("").is_err(), "空 id 应 Err");
        assert!(validate_id("ok").is_ok(), "正常 id 应 Ok");
        // validate_id: 超长 → Err
        let long = "x".repeat(257);
        assert!(validate_id(&long).is_err(), "超长 id 应 Err (> 256)");
        // validate_id: 边界 256 通过
        let boundary = "x".repeat(256);
        assert!(validate_id(&boundary).is_ok(), "边界 256 应 Ok");

        // extract_id: 缺 id → Err
        let v: Value = serde_json::json!({"name": "x"});
        assert!(extract_id(&v).is_err(), "缺 id 应 Err");

        // extract_id: 有 id → Ok
        let v2: Value = serde_json::json!({"id": "abc", "name": "x"});
        assert_eq!(extract_id(&v2).unwrap(), "abc");

        // extract_id: id 非字符串 → Err
        let v3: Value = serde_json::json!({"id": 123});
        assert!(extract_id(&v3).is_err(), "id 非字符串应 Err");
    }

    /// 5. 并发 upsert/delete 不 panic, 最终一致
    pub async fn storage_concurrency() {
        use std::sync::Arc;

        let s: Arc<InMemoryStorage<TestEntity>> = Arc::new(InMemoryStorage::new("concurrent"));

        // 8 个并发 task 各自 upsert 100 个不同 id
        let mut handles = Vec::new();
        for worker in 0..8 {
            let s_clone = s.clone();
            handles.push(tokio::spawn(async move {
                for i in 0..100 {
                    let id = format!("w{worker}-i{i}");
                    s_clone
                        .upsert(TestEntity {
                            id: id.clone(),
                            name: format!("name-{worker}-{i}"),
                            value: i64::from(worker * 100 + i),
                        })
                        .await
                        .expect("concurrent upsert");
                }
            }));
        }
        for h in handles {
            h.await.expect("join");
        }

        // 验证最终一致: 8 worker × 100 = 800 个不同 id
        let all = s.list().await.expect("list concurrent");
        assert_eq!(
            all.len(),
            8 * 100,
            "并发 upsert 800 次, 最终应 800 个实体 (无丢失)"
        );

        // 并发 delete 一半
        let mut handles = Vec::new();
        for worker in 0..8 {
            let s_clone = s.clone();
            handles.push(tokio::spawn(async move {
                for i in 0..50 {
                    let id = format!("w{worker}-i{i}");
                    let _ = s_clone.delete(&id).await;
                }
            }));
        }
        for h in handles {
            h.await.expect("join");
        }
        let all = s.list().await.expect("list post-delete");
        assert_eq!(all.len(), 8 * 50, "8 worker × 50 = 400 个剩余 (无残留)");
    }

    /// **附: 3 backend 编译期 hardcode + SqliteStorage NotImplemented**
    /// (用户说 5 测试函数, 这里把 5 个测的子集单独 expose 给 integration)
    pub async fn storage_three_backends() {
        assert_eq!(STORAGE_BACKEND_COUNT, 3, "3 backend 总数");
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
