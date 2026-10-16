//! 记忆账本计数的元数据面 (自省通道): 会话数 / 记忆件数 / 保护件数.
//!
//! 合同: 计数准确且**只回计数** —— 返回结构里没有内容原文字段。

use apeireth_core::Episode;
use apeireth_memory::{EpisodeStore, MemoryGovernanceStore, SqliteMemoryStore};

fn episode(id: &str, session: &str) -> Episode {
    Episode {
        id: id.to_string(),
        timestamp: 1_700_000_000,
        role: "user".to_string(),
        content: format!("内容原文-{id}"),
        session_id: session.to_string(),
    }
}

#[test]
fn empty_store_counts_zero_without_failing() {
    let store = SqliteMemoryStore::open_in_memory().expect("store");
    let counts = store.ledger_counts().expect("counts");
    assert_eq!(counts.sessions, 0);
    assert_eq!(counts.memories, 0);
    assert_eq!(counts.protected, 0);
}

#[test]
fn ledger_counts_are_exact_metadata_counts() {
    let store = SqliteMemoryStore::open_in_memory().expect("store");
    for (id, session) in [("e1", "s1"), ("e2", "s1"), ("e3", "s2")] {
        store.put_episode(&episode(id, session)).expect("episode");
    }
    store.protect_episode("e1", 0).expect("protect");
    store.protect_episode("e3", 0).expect("protect");

    let counts = store.ledger_counts().expect("counts");
    assert_eq!(counts.sessions, 2, "不同 session_id 计数");
    assert_eq!(counts.memories, 3, "episode 行计数");
    assert_eq!(counts.protected, 2, "protected = 1 行计数");
}

#[test]
fn ledger_counts_stay_counts_when_memories_grow() {
    let store = SqliteMemoryStore::open_in_memory().expect("store");
    for index in 0..10 {
        store
            .put_episode(&episode(&format!("e{index}"), "s1"))
            .expect("episode");
    }
    let counts = store.ledger_counts().expect("counts");
    assert_eq!(counts.memories, 10);
    assert_eq!(counts.sessions, 1);
    assert_eq!(counts.protected, 0);
}
