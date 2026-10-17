//! 人格记忆层集成面: `cognitive.sqlite3` persona 分表的持久化证据.
//!
//! 钉住四件事:
//! 1. V16 迁移在新库自动应用 (persona_memory_* 三表就位);
//! 2. 人格层与用户事实层**分表**: 人格沉淀不进 episodes, 情节事实不进人格表;
//! 3. 人格整编幂等 (跨持久化: 重复整编零变更, 最后整编真值不推进);
//! 4. 自述节计数 + 最后整编真值跨重开真实值.

use apeireth_core::kernel::memory::Episode;
use apeireth_memory::{
    consolidate_persona, EpisodeStore, InMemoryPersonaMemoryStore, PersonaEntry, PersonaExtractor,
    PersonaMemoryStore, SqliteMemoryStore,
};

fn episode(id: &str, role: &str, content: &str) -> Episode {
    Episode {
        id: id.into(),
        timestamp: 1_000,
        role: role.into(),
        content: content.into(),
        session_id: "sess-1".into(),
    }
}

#[test]
fn v16_persona_migration_is_applied_on_a_fresh_db() {
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    let applied = store.applied_migrations().unwrap();
    assert!(
        applied.contains(&16),
        "V16 persona 分表迁移应已应用: {applied:?}"
    );
    for table in [
        "persona_memory_entries",
        "persona_memory_tensions",
        "persona_memory_state",
    ] {
        let conn = store.conn().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                rusqlite::params![table],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists, "{table} 必须就位");
    }
}

#[test]
fn persona_layer_is_a_separate_table_from_user_facts() {
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    // 用户事实层: 情节 (关于「我」与「用户」的原始事实) 照常入 episodes.
    store
        .put_episode(&episode("ep-user-fact", "user", "主人喜欢乌龙茶"))
        .unwrap();
    store
        .put_episode(&episode("ep-self", "assistant", "我相信先验证再叙事"))
        .unwrap();
    // 人格层: 从自述情节提炼的沉淀入 persona_memory_* 表.
    let report = consolidate_persona(
        &store,
        &[episode("ep-self", "assistant", "我相信先验证再叙事")],
        10,
    )
    .unwrap();
    assert!(report.changed);

    // 分表证明: 整编不写 episodes (情节面原样), 人格表只收人格沉淀.
    let conn = store.conn().unwrap();
    let episode_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(episode_rows, 2, "情节面原样 (整编只写人格表)");
    let persona_texts: String = conn
        .query_row(
            "SELECT COALESCE(GROUP_CONCAT(text, '|'), '') FROM persona_memory_entries",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(persona_texts, "我相信先验证再叙事", "只有人格沉淀进人格表");
    assert!(!persona_texts.contains("乌龙茶"), "用户事实不进人格表");
}

#[test]
fn consolidation_is_idempotent_across_persistence() {
    use apeireth_memory::consolidation::MemoryConsolidationJob;
    let store = SqliteMemoryStore::open_in_memory().unwrap();
    let episodes = vec![episode(
        "ep-1",
        "assistant",
        "我喜欢把计划写清楚。我绝不在未告知的情况下改用户数据。",
    )];
    // 夜间整编入口 (`MemoryConsolidationJob::consolidate_persona`) 走同一条管线.
    let nightly = MemoryConsolidationJob::new();
    let first = nightly.consolidate_persona(&store, &episodes, 10).unwrap();
    assert!(first.changed);
    let snapshot = store.load().unwrap();

    let second = consolidate_persona(&store, &episodes, 999).unwrap();
    assert!(!second.changed, "重复整编必须零变更");
    assert_eq!(
        store.load().unwrap(),
        snapshot,
        "跨持久化的幂等: 逐字段相等"
    );
    let status = store.status().unwrap();
    assert_eq!(
        status.last_consolidation_at,
        Some(10),
        "真值不被重复整编推进"
    );
}

#[test]
fn status_truth_survives_reopening_the_store() {
    let dir = std::env::temp_dir().join(format!("persona-layer-status-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cognitive.sqlite3");

    {
        let store = SqliteMemoryStore::open(&path).unwrap();
        let status_before = store.status().unwrap();
        assert_eq!(
            status_before.last_consolidation_at, None,
            "尚未整编如实 None"
        );
        consolidate_persona(
            &store,
            &[
                episode("ep-1", "assistant", "我相信先验证再叙事"),
                episode("ep-3", "assistant", "我绝不说谎"),
            ],
            42,
        )
        .unwrap();
        // 下一轮整编撞出矛盾 → 张力标注 (不静默覆盖).
        consolidate_persona(&store, &[episode("ep-2", "assistant", "我习惯说谎")], 50).unwrap();
    }

    let reopened = SqliteMemoryStore::open(&path).unwrap();
    let status = reopened.status().unwrap();
    assert_eq!(status.core_beliefs, 1);
    assert_eq!(status.boundaries, 1);
    assert_eq!(status.tensions, 1, "张力跨重开保留");
    assert_eq!(status.last_consolidation_at, Some(50), "最后整编真值跨重开");
    let memory = reopened.load().unwrap();
    assert!(memory
        .tensions
        .iter()
        .any(|tension| tension.incoming_text == "我习惯说谎"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// 落库往返逐字段等价: 状态面保存不改写任何原文.
#[test]
fn sqlite_round_trip_preserves_every_field() {
    let mut memory = apeireth_memory::PersonaMemory::default();
    let extract = PersonaExtractor.extract(&[episode(
        "ep-1",
        "assistant",
        "我相信先验证再叙事。我是一个工程伙伴。",
    )]);
    memory.apply_extract(&extract, 7);

    let sqlite = SqliteMemoryStore::open_in_memory().unwrap();
    sqlite.save(&memory).unwrap();
    assert_eq!(sqlite.load().unwrap(), memory, "SQLite 往返逐字段等价");

    // 进程内存储同语义 (两实现同一合同).
    let in_memory = InMemoryPersonaMemoryStore::new();
    in_memory.save(&memory).unwrap();
    assert_eq!(in_memory.load().unwrap(), memory);
}

/// 保护语义落库: protection 默认 on, 随行往返.
#[test]
fn protection_defaults_to_on_and_round_trips() {
    let mut memory = apeireth_memory::PersonaMemory::default();
    let extract = PersonaExtractor.extract(&[episode("ep-1", "assistant", "我绝不说谎")]);
    memory.apply_extract(&extract, 7);
    assert!(memory
        .core_beliefs
        .iter()
        .chain(memory.preferences.iter())
        .chain(memory.boundaries.iter())
        .all(|entry: &PersonaEntry| entry.protection == apeireth_memory::PersonaProtection::On));

    let sqlite = SqliteMemoryStore::open_in_memory().unwrap();
    sqlite.save(&memory).unwrap();
    let loaded = sqlite.load().unwrap();
    assert_eq!(
        loaded.boundaries[0].protection,
        apeireth_memory::PersonaProtection::On
    );
}
