//! A4 集成测试: 6 历史流 + IdentityCard + Episode + Session + Note 端到端
//!
//! 覆盖场景:
//! 1. 6 流独立 schema + 各自的 append/list/tombstone
//! 2. IdentityCard 跨载体唯一约束 + record_migration
//! 3. Episode 按 session / time range / continuity_id 查询
//! 4. Session upsert + close; Note 完整 CRUD
//! 5. 一键导出所有 6 流 (D2 §5.3 #4)
//! 6. A1 兼容 trait (ContinuitySnapshotStore) 仍可用
//!
//! 这些测试放在 `tests/` 目录, 编译为独立 binary, 验证 public API.

use apeireth_core::{Episode, IdentityCard, Migration, Note, Session};
use apeireth_memory::{
    ActionStream, ContinuitySnapshotStore, EpisodeQuery, EpisodeStore, EvolutionStream, GoalStream,
    HistoryEntry, HistoryStream, IdentityCardStore, MigrationStream, NoteQuery, NoteStore,
    ProposalStream, ReflectionStream, RelationStream, SessionStore, SqliteMemoryStore,
    StanceStream, StreamKind, ThoughtStream,
};
use serde_json::json;
use std::sync::Arc;

fn fresh() -> SqliteMemoryStore {
    SqliteMemoryStore::open_in_memory().expect("open in-memory store")
}

fn entry(
    id: &str,
    subject: &str,
    rev: i64,
    at: i64,
    session: &str,
    payload: serde_json::Value,
) -> HistoryEntry {
    HistoryEntry {
        id: id.into(),
        subject_id: subject.into(),
        subject_rev: rev,
        session_id: Some(session.into()),
        created_at: at,
        payload,
        source: "integration".into(),
        tags: vec!["it".into()],
        tombstoned_at: None,
    }
}

#[test]
fn end_to_end_six_streams_independent() {
    let store = fresh();
    let conn = store.conn().unwrap();
    // 6 流各写一条
    ThoughtStream::new(&conn)
        .append(&entry("t-1", "subj", 1, 100, "s1", json!({"text": "思考"})))
        .unwrap();
    ProposalStream::new(&conn)
        .append(&entry("p-1", "subj", 1, 200, "s1", json!({"text": "提案"})))
        .unwrap();
    ActionStream::new(&conn)
        .append(&entry("a-1", "subj", 1, 300, "s1", json!({"text": "行动"})))
        .unwrap();
    RelationStream::new(&conn)
        .append(&entry("r-1", "subj", 1, 400, "s1", json!({"text": "关系"})))
        .unwrap();
    EvolutionStream::new(&conn)
        .append(&entry("e-1", "subj", 1, 500, "s1", json!({"text": "演化"})))
        .unwrap();
    ReflectionStream::new(&conn)
        .append(&entry(
            "rf-1",
            "subj",
            1,
            600,
            "s1",
            json!({"text": "反思"}),
        ))
        .unwrap();

    // 每条都能按 subject 查到
    for kind in StreamKind::ALL {
        let count = match kind {
            StreamKind::Thought => ThoughtStream::new(&conn)
                .list_for_subject("subj", None, None, false)
                .unwrap()
                .len(),
            StreamKind::Proposal => ProposalStream::new(&conn)
                .list_for_subject("subj", None, None, false)
                .unwrap()
                .len(),
            StreamKind::Action => ActionStream::new(&conn)
                .list_for_subject("subj", None, None, false)
                .unwrap()
                .len(),
            StreamKind::Relation => RelationStream::new(&conn)
                .list_for_subject("subj", None, None, false)
                .unwrap()
                .len(),
            StreamKind::Evolution => EvolutionStream::new(&conn)
                .list_for_subject("subj", None, None, false)
                .unwrap()
                .len(),
            StreamKind::Reflection => ReflectionStream::new(&conn)
                .list_for_subject("subj", None, None, false)
                .unwrap()
                .len(),
        };
        assert_eq!(count, 1, "stream {kind:?} should have 1 entry");
    }
}

#[test]
fn end_to_end_semantic_aliases() {
    // GoalStream = ThoughtStream, StanceStream = ProposalStream, LifeStream / MigrationStream = ActionStream
    let store = fresh();
    let conn = store.conn().unwrap();
    GoalStream::new(&conn)
        .append(&entry("g-1", "subj", 1, 100, "s", json!({})))
        .unwrap();
    StanceStream::new(&conn)
        .append(&entry("st-1", "subj", 1, 100, "s", json!({})))
        .unwrap();
    MigrationStream::new(&conn)
        .append(&entry("mg-1", "subj", 1, 100, "s", json!({})))
        .unwrap();
    // 都能从底层查到
    assert_eq!(
        ThoughtStream::new(&conn)
            .list_for_subject("subj", None, None, false)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        ProposalStream::new(&conn)
            .list_for_subject("subj", None, None, false)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        ActionStream::new(&conn)
            .list_for_subject("subj", None, None, false)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn end_to_end_identity_card_with_migration() {
    let store = fresh();
    let card = IdentityCard {
        continuity_id: "cid-A".into(),
        birth_time: 1_700_000_000,
        carriers: vec!["aws-1".into()],
        migration_history: vec![],
    };
    let rec = <SqliteMemoryStore as IdentityCardStore>::create(&store, &card).unwrap();
    assert_eq!(rec.subject_rev, 0);

    // 跨载体迁移 1 次
    let m = Migration {
        from_carrier: "aws-1".into(),
        to_carrier: "azure-1".into(),
        timestamp: 1_700_000_500,
    };
    let rec2 =
        <SqliteMemoryStore as IdentityCardStore>::record_migration(&store, "cid-A", &m).unwrap();
    assert_eq!(rec2.subject_rev, 1);
    assert_eq!(rec2.carriers.len(), 2);

    // UNIQUE 冲突
    let dup_err = <SqliteMemoryStore as IdentityCardStore>::create(&store, &card).unwrap_err();
    let msg = format!("{dup_err}");
    assert!(msg.contains("continuity_id") || msg.contains("UNIQUE"));

    // 按 carrier 查找
    let azure_cards =
        <SqliteMemoryStore as IdentityCardStore>::list_by_carrier(&store, "azure-1").unwrap();
    assert_eq!(azure_cards.len(), 1);
}

#[test]
fn end_to_end_episode_query_by_session_time_subject() {
    let store = fresh();
    for i in 0..6 {
        let ep = Episode {
            id: format!("e{i}"),
            timestamp: 1000 + i * 100,
            role: if i % 2 == 0 {
                "user".into()
            } else {
                "assistant".into()
            },
            content: format!("msg {i}"),
            session_id: if i < 4 { "s1".into() } else { "s2".into() },
        };
        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &ep).unwrap();
    }
    // 按 session
    let s1 =
        <SqliteMemoryStore as EpisodeStore>::query(&store, &EpisodeQuery::new().for_session("s1"))
            .unwrap();
    assert_eq!(s1.len(), 4);

    // 按 time range
    let r = <SqliteMemoryStore as EpisodeStore>::query(
        &store,
        &EpisodeQuery::new().in_range(Some(1200), Some(1500)),
    )
    .unwrap();
    assert_eq!(r.len(), 4); // e2, e3, e4, e5

    // 按 role
    let users =
        <SqliteMemoryStore as EpisodeStore>::query(&store, &EpisodeQuery::new().with_role("user"))
            .unwrap();
    assert_eq!(users.len(), 3);

    // 复合: session + time + role
    let r = <SqliteMemoryStore as EpisodeStore>::query(
        &store,
        &EpisodeQuery::new()
            .for_session("s1")
            .in_range(Some(1000), Some(1300))
            .with_role("user"),
    )
    .unwrap();
    assert_eq!(r.len(), 2); // e0, e2
}

#[test]
fn end_to_end_session_lifecycle() {
    let store = fresh();
    let s = Session {
        id: "s-lifecycle".into(),
        started_at: 100,
        last_active_at: 100,
    };
    <SqliteMemoryStore as SessionStore>::upsert_session(&store, &s).unwrap();
    <SqliteMemoryStore as SessionStore>::close_session(&store, "s-lifecycle", 500).unwrap();
    let got = <SqliteMemoryStore as SessionStore>::get_session(&store, "s-lifecycle")
        .unwrap()
        .unwrap();
    assert_eq!(got.closed_at, Some(500));
    let open = <SqliteMemoryStore as SessionStore>::list_open_sessions(&store).unwrap();
    assert!(open.is_empty());
}

#[test]
fn end_to_end_note_crud_and_query() {
    let store = fresh();
    for i in 0..5 {
        let n = Note {
            id: format!("n{i}"),
            timestamp: 1000 + i,
            content: format!("note {i}"),
            source_episode_ids: vec![format!("e{i}")],
            confidence: 0.2 + (i as f64) * 0.2,
            tags: if i % 2 == 0 {
                vec!["even".into()]
            } else {
                vec!["odd".into()]
            },
        };
        <SqliteMemoryStore as NoteStore>::put_note(&store, &n).unwrap();
    }
    // 按 confidence 过滤
    use apeireth_memory::NoteQuery;
    let confident =
        <SqliteMemoryStore as NoteStore>::query(&store, &NoteQuery::new().min_confidence(0.6))
            .unwrap();
    // n2(0.6), n3(0.8), n4(1.0) 全部 >= 0.6, 故 3 条.
    assert_eq!(confident.len(), 3);

    // 更新 content
    <SqliteMemoryStore as NoteStore>::update_note_content(&store, "n0", "updated").unwrap();
    let got = <SqliteMemoryStore as NoteStore>::get_note(&store, "n0")
        .unwrap()
        .unwrap();
    assert_eq!(got.content, "updated");

    // 物理删除
    <SqliteMemoryStore as NoteStore>::delete_note(&store, "n4").unwrap();
    let got = <SqliteMemoryStore as NoteStore>::get_note(&store, "n4").unwrap();
    assert!(got.is_none());
}

#[test]
fn end_to_end_export_streams() {
    let store = fresh();
    // 在 scoped 块内拿连接, 块结束自动释放锁, 防止死锁.
    {
        let conn = store.conn().unwrap();
        for kind in StreamKind::ALL {
            let id = format!("{kind:?}-x");
            let e = entry(
                &id,
                "export-subj",
                1,
                100,
                "sess-export",
                json!({"kind": format!("{kind:?}")}),
            );
            match kind {
                StreamKind::Thought => ThoughtStream::new(&conn).append(&e).unwrap(),
                StreamKind::Proposal => ProposalStream::new(&conn).append(&e).unwrap(),
                StreamKind::Action => ActionStream::new(&conn).append(&e).unwrap(),
                StreamKind::Relation => RelationStream::new(&conn).append(&e).unwrap(),
                StreamKind::Evolution => EvolutionStream::new(&conn).append(&e).unwrap(),
                StreamKind::Reflection => ReflectionStream::new(&conn).append(&e).unwrap(),
            }
        }
    } // conn dropped here
    let all = store.export_streams_jsonl().unwrap();
    assert_eq!(all.len(), 6);
}

#[test]
fn a1_compat_trait_still_works() {
    // 验证 A1 阶段 CLI 引用的 ContinuitySnapshotStore trait 仍可用
    let store = fresh();
    let ep = Episode {
        id: "a1-ep".into(),
        timestamp: 1_700_000_000,
        role: "user".into(),
        content: "compat".into(),
        session_id: "a1-sess".into(),
    };
    ContinuitySnapshotStore::put_episode(&store, &ep).unwrap();
    let n = Note {
        id: "a1-n".into(),
        timestamp: 1_700_000_100,
        content: "a1 compat note".into(),
        source_episode_ids: vec!["a1-ep".into()],
        confidence: 0.5,
        tags: vec![],
    };
    ContinuitySnapshotStore::put_note(&store, &n).unwrap();
    let recent = ContinuitySnapshotStore::recent_episodes(&store, "a1-sess", 5).unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].id, "a1-ep");
}

#[test]
fn store_is_send_sync_via_mutex() {
    // SqliteMemoryStore 通过 Mutex<Connection> 获得 Sync, A4 阶段要求.
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SqliteMemoryStore>();
    let store = Arc::new(fresh());
    let s2 = Arc::clone(&store);
    let h = std::thread::spawn(move || {
        // 在另一个线程上 put_episode
        let ep = Episode {
            id: "th-ep".into(),
            timestamp: 1,
            role: "user".into(),
            content: "threaded".into(),
            session_id: "th-sess".into(),
        };
        ContinuitySnapshotStore::put_episode(&*s2, &ep).unwrap();
    });
    h.join().unwrap();
    let recent = ContinuitySnapshotStore::recent_episodes(&*store, "th-sess", 5).unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].id, "th-ep");
}
