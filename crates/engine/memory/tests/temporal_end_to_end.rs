//! 「时间正确率」的**端到端**验证：真写库 ⇒ 真读有效期 ⇒ 真判 ⇒ **真的有区分力**。
//!
//! # 为什么有这批测试
//!
//! 设计稿 §6.1 的 **L1 时间正确率**要测「检索返回的是否**已失效**」。
//! ⚠️ 但 2026-10-27 的真实语料报告暴露：这个 1.0 是「**构造性**」的
//! ⇒ 因为**全仓没有任何写入路径会写 `episodes.valid_until_ms`**
//! ⇒ 真实库里**没有一条记录会过期** ⇒ 那个 1.0 **不是**「检索很棒」的证据。
//!
//! ⇒ 现在有了**带失效时间的写入路径**（`put_episode_with_validity`）
//! ⇒ ⇒ 本批证明：**当库里真的有失效记录时，该指标会真的降下来**
//! ⇒ ⇒ 指标**不是恒等于 1 的装饰品**。

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::quality_metrics::{measure_temporal, Validity};
use apeireth_memory::SqliteMemoryStore;
use std::collections::BTreeMap;

fn episode(id: &str, timestamp: i64) -> Episode {
    Episode {
        id: id.to_string(),
        timestamp,
        role: "user".to_string(),
        content: format!("内容 {id}"),
        session_id: "s1".to_string(),
    }
}

/// 从**真实库**读有效期，建出指标要的映射。
fn validity_map(store: &SqliteMemoryStore, ids: &[&str]) -> BTreeMap<String, Validity> {
    let mut map = BTreeMap::new();
    for id in ids {
        let validity = store
            .get_episode_validity(id)
            .expect("读有效期")
            .expect("记录存在");
        map.insert(
            (*id).to_string(),
            Validity {
                valid_from_ms: validity.0,
                valid_until_ms: validity.1,
            },
        );
    }
    map
}

#[test]
fn the_metric_drops_when_the_library_really_has_an_expired_record() {
    // ⚠️ **核心断言**：库里**真的有**一条失效记录 ⇒ 时间正确率**必须 < 1.0**
    // ⇒ 之前它是恒等于 1.0 的 ⇒ 那是「构造性」的、**没有区分力**。
    let store = SqliteMemoryStore::open_in_memory().expect("in-memory store");
    store
        .put_episode_with_validity(&episode("alive", 100), "s1", 1_000, None)
        .expect("写永久有效");
    store
        .put_episode_with_validity(&episode("expired", 100), "s1", 1_000, Some(2_000))
        .expect("写已失效");

    let validity = validity_map(&store, &["alive", "expired"]);
    let as_of = 3_000;
    let verdict = measure_temporal(
        &["alive".to_string(), "expired".to_string()],
        &validity,
        as_of,
    );

    assert_eq!(verdict.checked, 2, "两条都查到了有效期");
    assert_eq!(verdict.stale_returned, 1, "过期那条被返回了");
    let rate = verdict.correct_rate.expect("有分母");
    assert!(
        rate < 1.0,
        "⚠️ 库里真有过期记录时，时间正确率**必须**降下来（实际 {rate}）"
    );
    assert!((rate - 0.5).abs() < 1e-9, "两条里错一条 ⇒ 0.5，实际 {rate}");
}

#[test]
fn the_metric_is_one_when_no_record_has_expired() {
    let store = SqliteMemoryStore::open_in_memory().expect("in-memory store");
    store
        .put_episode_with_validity(&episode("a", 100), "s1", 1_000, None)
        .expect("写永久有效");
    store
        .put_episode_with_validity(&episode("b", 100), "s1", 1_000, Some(9_000))
        .expect("写尚未失效");

    let validity = validity_map(&store, &["a", "b"]);
    let verdict = measure_temporal(&["a".to_string(), "b".to_string()], &validity, 3_000);
    assert_eq!(verdict.stale_returned, 0);
    assert_eq!(verdict.correct_rate, Some(1.0));
}

#[test]
fn a_record_that_is_not_yet_effective_is_not_counted_as_an_error() {
    // ⚠️ 「**尚未生效**」**不是**「返回了失效的记忆」⇒ 不算错。
    let store = SqliteMemoryStore::open_in_memory().expect("in-memory store");
    store
        .put_episode_with_validity(&episode("future", 100), "s1", 5_000, None)
        .expect("写尚未生效");

    let validity = validity_map(&store, &["future"]);
    let verdict = measure_temporal(&["future".to_string()], &validity, 1_000);
    assert_eq!(verdict.stale_returned, 0);
    assert_eq!(verdict.correct_rate, Some(1.0));
}

#[test]
fn the_expiry_boundary_is_exclusive_and_matches_the_canonical_semantics() {
    // ⚠️ `as_of == valid_until` ⇒ **已失效**（与 `canonical::domain` 同口径）。
    let store = SqliteMemoryStore::open_in_memory().expect("in-memory store");
    store
        .put_episode_with_validity(&episode("e", 100), "s1", 1_000, Some(2_000))
        .expect("写失效时间");

    let validity = validity_map(&store, &["e"]);
    let at_boundary = measure_temporal(&["e".to_string()], &validity, 2_000);
    assert_eq!(
        at_boundary.stale_returned, 1,
        "到期那一刻即已失效 ⇒ 半开区间 [from, until)"
    );
    let just_before = measure_temporal(&["e".to_string()], &validity, 1_999);
    assert_eq!(just_before.stale_returned, 0, "还没到期 ⇒ 仍有效");
}

#[test]
fn a_record_without_validity_information_is_not_counted_as_an_error() {
    // ⚠️ **查不到有效期 ⇒ 不计入分母**（无信息不算错）⇒ 但**单独计数**。
    let store = SqliteMemoryStore::open_in_memory().expect("in-memory store");
    store
        .put_episode_with_validity(&episode("known", 100), "s1", 1_000, None)
        .expect("写");

    let mut validity = validity_map(&store, &["known"]);
    validity.insert(
        "unknown".to_string(),
        Validity {
            valid_from_ms: None,
            valid_until_ms: None,
        },
    );
    let verdict = measure_temporal(
        &["known".to_string(), "unknown".to_string()],
        &validity,
        2_000,
    );
    assert_eq!(verdict.checked, 2);
    assert_eq!(verdict.stale_returned, 0);
    assert_eq!(verdict.correct_rate, Some(1.0));
}
