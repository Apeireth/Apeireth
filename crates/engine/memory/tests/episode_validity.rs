//! `episode` 的**有效期**写入与读取路径（2026-10-27）。
//!
//! # 为什么有这批测试
//!
//! 设计稿 §6.1 的 **L1 时间正确率**要测「检索返回的是否**已失效**」，
//! 但实测发现：**全仓没有任何写入路径会写 `episodes.valid_until_ms`**
//! ⇒ 真实库里**没有一条记录会过期**
//! ⇒ 「时间正确率 = 1.0」是**构造性**的 ⇒ 该指标**没有区分力**。
//!
//! ⇒ 本批补上**带失效时间的写入路径** ⇒ 让该指标**真的有区分力**。
//!
//! # 三条纪律（测试逐条钉住）
//!
//! 1. **不覆盖既有路径**（`put_episode` 语义不变 ⇒ 既有调用方零改动）；
//! 2. **入链**（D0 哈希链）⇒ 链摘要含 `valid_until_ms` ⇒ 校验侧同口径；
//! 3. **拒绝「永远无效」的记录**（`valid_until_ms <= valid_from_ms` ⇒ 硬拒）。

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::{EpisodeStore, MemoryError, SqliteMemoryStore};

fn store() -> SqliteMemoryStore {
    SqliteMemoryStore::open_in_memory().expect("in-memory store")
}

fn episode(id: &str, timestamp: i64) -> Episode {
    Episode {
        id: id.to_string(),
        timestamp,
        role: "user".to_string(),
        content: format!("内容 {id}"),
        session_id: "s1".to_string(),
    }
}

// ============================================================
// 写入：带失效时间
// ============================================================

#[test]
fn writes_and_reads_back_the_validity_window() {
    let store = store();
    store
        .put_episode_with_validity(&episode("e1", 100), "subject-a", 1_000, Some(5_000))
        .expect("写入带失效时间的记录");
    let validity = store
        .get_episode_validity("e1")
        .expect("读有效期")
        .expect("记录存在");
    assert_eq!(validity.0, Some(1_000));
    assert_eq!(validity.1, Some(5_000));
}

#[test]
fn permanent_validity_is_written_as_null_until() {
    let store = store();
    store
        .put_episode_with_validity(&episode("e2", 100), "subject-a", 1_000, None)
        .expect("写入永久有效的记录");
    let validity = store
        .get_episode_validity("e2")
        .expect("读有效期")
        .expect("记录存在");
    assert_eq!(validity.0, Some(1_000));
    assert_eq!(validity.1, None, "永久有效 ⇒ until 为 NULL");
}

#[test]
fn rejects_a_record_that_is_never_valid() {
    // ⚠️ **永远无效的记忆** = 插进去就是垃圾数据 ⇒ 硬拒。
    let store = store();
    let result =
        store.put_episode_with_validity(&episode("e3", 100), "subject-a", 5_000, Some(1_000));
    assert!(result.is_err(), "失效时间不晚于生效时间 ⇒ 必须拒");
}

#[test]
fn rejects_equal_validity_bounds() {
    let store = store();
    let result =
        store.put_episode_with_validity(&episode("e4", 100), "subject-a", 1_000, Some(1_000));
    assert!(result.is_err(), "起止相等 ⇒ 瞬间失效 ⇒ 必须拒");
}

#[test]
fn rejects_empty_continuity_id() {
    let store = store();
    let result = store.put_episode_with_validity(&episode("e5", 100), "  ", 1_000, None);
    assert!(
        result.is_err(),
        "空 continuity_id ⇒ 必须拒（不许悄悄落成 default）"
    );
}

#[test]
fn rejects_empty_episode_id() {
    let store = store();
    let result = store.put_episode_with_validity(&episode("", 100), "subject-a", 1_000, None);
    assert!(result.is_err(), "空 id ⇒ 必须拒");
}

#[test]
fn reading_validity_of_a_missing_record_is_none() {
    let store = store();
    assert_eq!(
        store.get_episode_validity("nope").expect("读不存在的记录"),
        None
    );
}

#[test]
fn reading_validity_with_empty_id_is_an_error() {
    let store = store();
    assert!(matches!(
        store.get_episode_validity("  "),
        Err(MemoryError::Invalid(_))
    ));
}

// ============================================================
// 兼容性：既有写入路径**语义不变**
// ============================================================

#[test]
fn the_existing_path_still_writes_permanent_validity() {
    // ⚠️ **不覆盖既有路径** ⇒ `put_episode` 仍写「永久有效」。
    let store = store();
    store.put_episode(&episode("e6", 100)).expect("既有路径");
    let validity = store
        .get_episode_validity("e6")
        .expect("读有效期")
        .expect("记录存在");
    assert_eq!(validity.1, None, "既有路径仍写永久有效");
}

// ============================================================
// D0 哈希链：带失效时间的记录**也要入链**
// ============================================================

#[test]
fn a_record_with_validity_still_carries_chain_fields() {
    // ⚠️ 若不入链 ⇒ 体检会把它算成「未纳管」⇒ 而调用方会以为数据受保护。
    let store = store();
    store
        .put_episode_with_validity(&episode("e7", 100), "subject-a", 1_000, Some(5_000))
        .expect("写入");
    let conn = store.conn().expect("conn");
    let chain_hash: Option<String> = conn
        .query_row(
            "SELECT chain_hash FROM episodes WHERE id = ?1",
            ["e7"],
            |row| row.get(0),
        )
        .expect("读 chain_hash");
    assert!(
        chain_hash.as_deref().is_some_and(|value| !value.is_empty()),
        "带失效时间的记录**必须**带链摘要（否则体检算成未纳管）"
    );
}
