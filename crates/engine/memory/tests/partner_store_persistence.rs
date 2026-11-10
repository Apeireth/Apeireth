//! partner 羁绊存储的**跨重启持久化**（2026-10-24 接上 SQLite 后端）。
//!
//! # 为什么要有这条测试
//!
//! 生产装配此前用 `InMemoryPartnerStore`（进程内，重启即散）——
//! 羁绊本义是**跨会话**的东西，那种存法等于每次重启都要重新「认识」一遍。
//! 换成 `SqlitePartnerStore` 后，必须钉住「**换个 store 实例还能读到**」，
//! 否则「换了持久后端」可能只是换了个壳。

use apeireth_memory::partner::{Partner, PartnerId, PartnerPreferences, PartnerStore};
use apeireth_storage::SqliteConnectionPool;
use tempfile::tempdir;

#[tokio::test]
async fn partner_survives_a_new_store_instance() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("partner.sqlite");
    let pool = SqliteConnectionPool::open(&path).await.unwrap();

    let id = PartnerId::new();
    {
        let store =
            apeireth_memory::SqlitePartnerStore::from_arc(std::sync::Arc::new(pool.clone()));
        let partner = Partner::new(id.clone(), "小林", PartnerPreferences::default(), 1_000);
        store.save_partner(&partner).expect("save partner");
    }

    // **新实例**（模拟进程重启后重新打开同一份库）
    let reopened = apeireth_memory::SqlitePartnerStore::from_arc(std::sync::Arc::new(pool));
    let loaded = reopened.get_partner(&id).expect("get partner");
    let loaded = loaded.expect("partner 必须还在（跨重启持久）");
    assert_eq!(loaded.id, id);
    assert_eq!(loaded.display_name, "小林");
    assert_eq!(loaded.created_at_epoch_ms, 1_000);
    assert_eq!(loaded.last_seen_epoch_ms, 1_000);
}

#[tokio::test]
async fn missing_partner_reads_as_none() {
    let pool = SqliteConnectionPool::in_memory().await.unwrap();
    let store = apeireth_memory::SqlitePartnerStore::from_arc(std::sync::Arc::new(pool));
    let loaded = store.get_partner(&PartnerId::new()).expect("get partner");
    assert!(loaded.is_none(), "不存在的 partner 应返回 None 而不是报错");
}
