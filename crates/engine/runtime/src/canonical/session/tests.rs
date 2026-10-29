use super::*;
use apeireth_core::kernel::VirtualClock;

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

#[tokio::test]
async fn a_missing_session_is_created_rather_than_erroring() {
    let manager = SessionManager::new(Arc::new(InMemorySessionStore::new()), clock());
    let id = SessionId::new();

    let session = manager.load_or_create(id).await.unwrap();
    assert_eq!(session.id, id);
    assert!(session.is_empty());
}

#[tokio::test]
async fn a_saved_session_round_trips_with_its_transcript() {
    let store = Arc::new(InMemorySessionStore::new());
    let manager = SessionManager::new(store.clone(), clock());
    let id = SessionId::new();

    let mut session = manager.load_or_create(id).await.unwrap();
    session.append(NormalizedMessage::user("hello"), clock().as_ref());
    session.append(NormalizedMessage::assistant("hi"), clock().as_ref());
    manager.save(&session).await.unwrap();

    let reloaded = manager.load_or_create(id).await.unwrap();
    assert_eq!(reloaded.len(), 2);
    assert!(
        matches!(
            &reloaded.messages[0].content[0],
            apeireth_protocol::canonical::ContentPart::Text { text } if text == "hello"
        ),
        "the transcript must survive the round trip intact"
    );
    assert_eq!(store.len().await, 1);
}

#[tokio::test]
async fn saving_twice_replaces_rather_than_duplicating() {
    let store = Arc::new(InMemorySessionStore::new());
    let manager = SessionManager::new(store.clone(), clock());
    let id = SessionId::new();

    let mut session = manager.load_or_create(id).await.unwrap();
    session.append(NormalizedMessage::user("one"), clock().as_ref());
    manager.save(&session).await.unwrap();
    session.append(NormalizedMessage::user("two"), clock().as_ref());
    manager.save(&session).await.unwrap();

    assert_eq!(store.len().await, 1);
    assert_eq!(manager.load_or_create(id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn messages_and_events_advance_one_monotonic_revision() {
    let clock = clock();
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    let request = RequestId::new();
    let trace = TraceId::new();

    session.append(NormalizedMessage::user("attempt"), clock.as_ref());
    session.record(
        request,
        trace,
        SessionEventKind::ProviderFailed {
            error: "offline".into(),
            round: 1,
        },
        clock.as_ref(),
    );

    assert_eq!(session.revision, 2);
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.events.len(), 1);
    assert_eq!(session.events[0].request, request);
    assert_eq!(session.events[0].trace, trace);
}

#[tokio::test]
async fn appending_advances_the_modification_time_from_the_injected_clock() {
    let virtual_clock = VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    );
    let clock: Arc<dyn Clock> = Arc::new(virtual_clock.clone());
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    let created = session.created_at;

    virtual_clock.advance(chrono::Duration::seconds(30));
    session.append(NormalizedMessage::user("later"), clock.as_ref());

    assert_eq!(session.created_at, created, "creation time must not move");
    assert_eq!(
        session.updated_at.epoch_millis() - created.epoch_millis(),
        30_000
    );
}

#[tokio::test]
async fn two_sessions_do_not_share_a_transcript() {
    let store = Arc::new(InMemorySessionStore::new());
    let manager = SessionManager::new(store.clone(), clock());

    let mut a = manager.load_or_create(SessionId::new()).await.unwrap();
    let b = manager.load_or_create(SessionId::new()).await.unwrap();
    a.append(NormalizedMessage::user("only in a"), clock().as_ref());
    manager.save(&a).await.unwrap();
    manager.save(&b).await.unwrap();

    assert_eq!(manager.load_or_create(a.id).await.unwrap().len(), 1);
    assert_eq!(manager.load_or_create(b.id).await.unwrap().len(), 0);
    assert_eq!(store.len().await, 2);
}

#[test]
fn legacy_session_json_without_settings_migrates_with_defaults_and_no_loss() {
    // A session persisted before the `settings` field existed: serialize a
    // real session, strip the new `settings` key, and reload it. The
    // transcript must survive and settings must fall back to defaults.
    let clock = clock();
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    session.append(NormalizedMessage::user("hello"), clock.as_ref());

    let mut legacy = serde_json::to_value(&session).unwrap();
    let object = legacy
        .as_object_mut()
        .expect("session serializes as object");
    assert!(object.remove("settings").is_some());

    let migrated: Session = serde_json::from_value(legacy).expect("legacy session must load");

    assert_eq!(migrated.settings, SessionSettings::default());
    assert_eq!(migrated.settings.model, None);
    assert_eq!(
        migrated.settings.permission_preset,
        PermissionPreset::Standard
    );
    assert_eq!(
        migrated.messages.len(),
        1,
        "transcript must survive migration"
    );
    assert_eq!(migrated.revision, 1);
}

#[test]
fn session_settings_without_approval_remember_migrates_to_false_without_loss() {
    // A settings blob persisted before `approval_remember` existed must
    // deserialize with `approval_remember == false` and keep every other
    // field intact.
    let settings = SessionSettings {
        model: Some("some/model".into()),
        permission_preset: PermissionPreset::Full,
        approval_remember: true,
    };

    let mut json = serde_json::to_value(&settings).unwrap();
    let object = json.as_object_mut().expect("settings serializes as object");
    assert!(object.remove("approval_remember").is_some());

    let migrated: SessionSettings = serde_json::from_value(json).unwrap();
    assert!(!migrated.approval_remember);
    assert_eq!(migrated.model, Some("some/model".into()));
    assert_eq!(migrated.permission_preset, PermissionPreset::Full);
}

#[test]
fn session_settings_survive_json_roundtrip_for_each_preset() {
    // Select a tier → serialize → deserialize: a populated session (with a
    // transcript and a recorded fact) must read its settings back as the
    // same tier, with the model override and the remember flag untouched —
    // for every tier and every switch between tiers.
    let clock = clock();
    let mut session = Session::new(SessionId::new(), clock.as_ref());
    session.append(NormalizedMessage::user("hello"), clock.as_ref());
    session.record(
        RequestId::new(),
        TraceId::new(),
        SessionEventKind::TurnStarted,
        clock.as_ref(),
    );
    session.settings.model = Some("some/model".into());

    for (preset, remember) in [
        (PermissionPreset::ReadOnly, true),
        (PermissionPreset::Standard, false),
        (PermissionPreset::Full, true),
    ] {
        session.settings.permission_preset = preset;
        session.settings.approval_remember = remember;

        let json = serde_json::to_string(&session).unwrap();
        let reloaded: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(reloaded.settings.permission_preset, preset);
        assert_eq!(reloaded.settings.approval_remember, remember);
        assert_eq!(reloaded.settings.model.as_deref(), Some("some/model"));
    }
}

#[tokio::test]
async fn list_returns_sessions_most_recently_updated_first() {
    let virtual_clock = VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    );
    let clock: Arc<dyn Clock> = Arc::new(virtual_clock.clone());
    let store = InMemorySessionStore::new();

    let older = Session::new(SessionId::new(), clock.as_ref());
    virtual_clock.advance(chrono::Duration::seconds(30));
    let newer = Session::new(SessionId::new(), clock.as_ref());

    store.save(&older).await.unwrap();
    store.save(&newer).await.unwrap();

    let listed = store.list().await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, newer.id, "newest updated first");
    assert_eq!(listed[1].id, older.id);
}

#[tokio::test]
async fn delete_removes_the_stored_session_and_reports_it() {
    let store = InMemorySessionStore::new();
    let id = SessionId::new();
    store
        .save(&Session::new(id, clock().as_ref()))
        .await
        .unwrap();
    assert_eq!(store.len().await, 1);

    assert!(store.delete(&id).await.unwrap(), "record was present");
    assert_eq!(store.len().await, 0);
    assert!(store.load(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn deleting_an_unknown_session_reports_nothing_removed() {
    let store = InMemorySessionStore::new();

    assert!(!store.delete(&SessionId::new()).await.unwrap());
}

// ------------------------------------------------------------------
// W2-③ 会话写闸: 同会话写串行 (排队事实) / 会话间并发 / 读改写原子
// ------------------------------------------------------------------

#[tokio::test]
async fn write_permits_serialize_one_session_and_stay_concurrent_across_sessions() {
    let gate = Arc::new(SessionWriteGate::default());
    let a = SessionId::new();
    let b = SessionId::new();

    let held = gate.admit(a).await;
    assert_eq!(held.admission().queue_position, 0, "首位写者不排队");
    assert!(!held.admission().was_queued());

    // 同会话第二位写者: 排队 (不拒绝、不交叠) —— 排队事实确定性。
    let waiter_gate = Arc::clone(&gate);
    let waiter = tokio::spawn(async move { waiter_gate.admit(a).await });
    for _ in 0..100 {
        if gate.writers_in_flight(&a).await == 2 {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(
        gate.writers_in_flight(&a).await,
        2,
        "第二位写者在场排队, 不是被拒 (排队语义钉死)"
    );

    // 不同会话不共车道: 直接持闸, 不受会话 a 的在场写者影响。
    let other = gate.admit(b).await;
    assert!(!other.admission().was_queued(), "会话之间互不阻塞");

    drop(held);
    let queued = waiter.await.unwrap();
    assert!(queued.admission().was_queued(), "排队事实必须留痕");
    assert_eq!(queued.admission().queue_position, 1, "排在原持闸者之后");

    drop(queued);
    drop(other);
    assert_eq!(gate.writers_in_flight(&a).await, 0, "退场即清计数");
    assert_eq!(gate.writers_in_flight(&b).await, 0);
}

#[tokio::test]
async fn a_queued_writer_really_waits_until_the_holder_releases() {
    let gate = Arc::new(SessionWriteGate::default());
    let session = SessionId::new();

    let held = gate.admit(session).await;
    let waiter_gate = Arc::clone(&gate);
    let waiter = tokio::spawn(async move { waiter_gate.admit(session).await });

    // 持闸者不放行, 排队者绝拿不到闸 (串行不交叠)。
    for _ in 0..100 {
        assert!(!waiter.is_finished(), "持闸期间排队者不得入场 (回合串行化)");
        tokio::task::yield_now().await;
    }

    drop(held);
    let admitted = waiter.await.unwrap();
    assert!(admitted.admission().was_queued());
}

#[tokio::test]
async fn concurrent_updates_never_lose_a_write_on_one_session() {
    // 对抗压测: 16 个写者并发读-改-写同一会话, 每人追加一条消息 ——
    // 交叠写会丢更新, 全量到齐即证零交叠。
    let clock = clock();
    let manager = Arc::new(SessionManager::new(
        Arc::new(InMemorySessionStore::new()),
        Arc::clone(&clock),
    ));
    let id = SessionId::new();
    manager
        .save(&manager.load_or_create(id).await.unwrap())
        .await
        .unwrap();

    let mut writers = Vec::new();
    for n in 0..16u32 {
        let manager = Arc::clone(&manager);
        let clock = Arc::clone(&clock);
        writers.push(tokio::spawn(async move {
            manager
                .update(id, |session| {
                    session.append(NormalizedMessage::user(format!("msg-{n}")), clock.as_ref());
                })
                .await
                .unwrap()
                .expect("session exists")
        }));
    }
    for writer in writers {
        let (_, admission) = writer.await.unwrap();
        assert!(admission.queue_position < 16, "排队事实有界");
    }

    let session = manager.load_or_create(id).await.unwrap();
    assert_eq!(session.len(), 16, "16 次追加一条不丢 (零交叠写)");
    let appended = session
        .log
        .iter()
        .filter(|entry| matches!(entry.event, SessionEventKind::MessageAppended { .. }))
        .count();
    assert_eq!(appended, 16, "账本与转写 lockstep: 每条消息恰有一笔日志");
}

#[tokio::test]
async fn update_reports_missing_sessions_without_creating_them() {
    let manager = SessionManager::new(Arc::new(InMemorySessionStore::new()), clock());
    let id = SessionId::new();

    let outcome = manager.update(id, |_| ()).await.unwrap();
    assert!(outcome.is_none(), "不存在的会话不因读改写而凭空出现");
}

#[tokio::test]
async fn a_stored_session_list_is_one_coherent_snapshot_under_concurrent_writes() {
    // 面板读快照语义 (W2-③): `list()` 一次读 = 一个自洽快照 —— 写者并发
    // 期间, 每个快照行必须是某个时刻的完整会话 (转写与账本永不撕裂)。
    let clock = clock();
    let store = Arc::new(InMemorySessionStore::new());
    let manager = Arc::new(SessionManager::new(
        Arc::clone(&store) as Arc<dyn SessionStore>,
        Arc::clone(&clock),
    ));
    let ids: Vec<SessionId> = (0..4).map(|_| SessionId::new()).collect();
    for id in &ids {
        manager
            .save(&manager.load_or_create(*id).await.unwrap())
            .await
            .unwrap();
    }

    let mut writers = Vec::new();
    for id in &ids {
        for n in 0..8usize {
            let manager = Arc::clone(&manager);
            let clock = Arc::clone(&clock);
            let id = *id;
            writers.push(tokio::spawn(async move {
                manager
                    .update(id, |session| {
                        session.append(NormalizedMessage::user(format!("w-{n}")), clock.as_ref());
                    })
                    .await
                    .unwrap();
            }));
        }
    }

    let mut snapshots = Vec::new();
    for _ in 0..32 {
        snapshots.push(store.list().await.unwrap());
        tokio::task::yield_now().await;
    }
    for writer in writers {
        writer.await.unwrap();
    }
    snapshots.push(store.list().await.unwrap());

    for snapshot in &snapshots {
        assert_eq!(snapshot.len(), ids.len(), "快照不缺行不重行");
        for session in snapshot {
            let appended = session
                .log
                .iter()
                .filter(|entry| matches!(entry.event, SessionEventKind::MessageAppended { .. }))
                .count();
            assert_eq!(
                appended,
                session.messages.len(),
                "每行自洽: 账本追加记录与转写同刻 (无撕裂读)"
            );
            assert!(session.revision >= session.messages.len() as u64);
        }
    }
    // 写者收尾后的最终快照: 每会话 8 位写者 × 1 条全到齐。
    let final_snapshot = snapshots.last().unwrap();
    for session in final_snapshot {
        assert_eq!(session.messages.len(), 8, "零丢失更新");
    }
}
