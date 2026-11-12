//! Deterministic lifecycle checks for Chinese conversational memory.
//!
//! These exercise the production SQLite/coordinator boundary. Corrections are
//! explicit governed edits, not automatic natural-language fact reconciliation.
//! Raw source text remains auditable; forgotten records are hidden, not purged.

use std::path::Path;
use std::sync::Arc;

use apeireth_core::kernel::Episode;
use apeireth_memory::{
    BoundedMemoryInput, EpisodeStore, MemoryCoordinator, MemoryExtractionMessage,
    MemoryExtractionResult, MemoryGovernanceError, MemoryGovernanceStore, MemoryLayerKind,
    MemoryMaterializer, MemoryProvenance, MemoryRecallQuery, MemoryScope, SqliteMemoryStore,
};
use tempfile::tempdir;

const SESSION: &str = "conversation-owner";
const NOW_SECS: i64 = 1_800_000_000;
const OLD_FACT: &str = "我住在武汉，晨星项目的联络城市是武汉。";
const NEW_FACT: &str = "我住在上海，晨星项目的联络城市是上海。";

fn open(path: &Path) -> (Arc<SqliteMemoryStore>, MemoryCoordinator) {
    let store = Arc::new(SqliteMemoryStore::open(path).unwrap());
    let coordinator =
        MemoryCoordinator::new(store.clone(), store.clone()).with_scoped_backend(store.clone());
    (store, coordinator)
}

fn write(
    coordinator: &MemoryCoordinator,
    id: &str,
    source_session: &str,
    scope: MemoryScope,
    content: &str,
) -> Episode {
    let episode = Episode {
        id: id.into(),
        timestamp: NOW_SECS,
        role: "user".into(),
        content: content.into(),
        session_id: source_session.into(),
    };
    coordinator
        .writeback_scoped_episode(
            &episode,
            &scope,
            &MemoryProvenance {
                source: "conversation-quality".into(),
                source_session: Some(source_session.into()),
                source_trace: Some(format!("trace-{id}")),
                source_request: Some(format!("request-{id}")),
            },
        )
        .unwrap();
    episode
}

fn query(session: &str, text: &str) -> MemoryRecallQuery {
    // Keep the production k and character budget; pin time instead of sleeping.
    MemoryRecallQuery::new(session, text).with_as_of_ms(NOW_SECS * 1_000)
}

fn assert_current(coordinator: &MemoryCoordinator, id: &str, layers: Vec<MemoryLayerKind>) {
    let query = query(SESSION, "晨星 项目 联络 城市").with_layers(layers);
    let recalled = coordinator.recall(&query).unwrap();
    let item = recalled.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(
        item.content, NEW_FACT,
        "every requested layer must honor the edit"
    );
    assert!(!recalled
        .items
        .iter()
        .any(|item| item.content.contains("武汉")));
    let selected = coordinator
        .compile_prompt_overlay_with_selected_access(&query)
        .unwrap()
        .unwrap();
    assert!(selected
        .selected_candidate_ids
        .iter()
        .any(|item| item == id));
    assert!(selected.overlay.contains(NEW_FACT));
    assert!(!selected.overlay.contains("武汉"));
    assert!(selected.overlay.chars().count() <= query.max_chars);
}

#[test]
fn governed_chinese_correction_reaches_every_layer_and_survives_restart() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("correction.sqlite3");
    let (store, coordinator) = open(&path);
    let episode = write(
        &coordinator,
        "location-fact",
        SESSION,
        MemoryScope::session(SESSION),
        OLD_FACT,
    );
    let original = coordinator
        .recall(&query(SESSION, "晨星 联络城市"))
        .unwrap();
    assert!(original.items.iter().any(|item| item.content == OLD_FACT));

    let edited = coordinator
        .update_episode_content(&episode.id, NEW_FACT, Some("owner"), 0)
        .unwrap();
    assert_eq!(edited.revision, 1);
    assert_eq!(edited.updated_by.as_deref(), Some("owner"));
    assert!(matches!(
        coordinator.update_episode_content(&episode.id, "我住在广州。", Some("stale"), 0),
        Err(MemoryGovernanceError::Conflict {
            expected: 0,
            actual: 1,
            ..
        })
    ));

    assert_current(&coordinator, &episode.id, vec![MemoryLayerKind::Working]);
    assert_current(&coordinator, &episode.id, vec![MemoryLayerKind::Episodic]);
    assert_current(
        &coordinator,
        &episode.id,
        vec![MemoryLayerKind::Working, MemoryLayerKind::Episodic],
    );
    let raw = EpisodeStore::get_episode(store.as_ref(), &episode.id)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&raw).unwrap(),
        serde_json::to_value(&episode).unwrap(),
        "a governed edit must preserve the original evidence"
    );
    let before = coordinator.episode_scope_metadata(&raw);
    assert_eq!(before.0, MemoryScope::session(SESSION));
    assert_eq!(
        before.1.source_request.as_deref(),
        Some("request-location-fact")
    );

    drop(coordinator);
    drop(store);
    let (store, coordinator) = open(&path);
    assert_current(&coordinator, &episode.id, vec![MemoryLayerKind::Episodic]);
    let raw = EpisodeStore::get_episode(store.as_ref(), &episode.id)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&raw).unwrap(),
        serde_json::to_value(&episode).unwrap()
    );
    assert_eq!(coordinator.episode_scope_metadata(&raw), before);
    assert_eq!(
        store.get_governed(&episode.id).unwrap().unwrap().revision,
        1
    );
}

#[test]
fn forgotten_chinese_fact_stays_hidden_after_restart_and_other_evidence_remains() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("forget.sqlite3");
    let (store, coordinator) = open(&path);
    let forgotten = write(
        &coordinator,
        "forget-location",
        SESSION,
        MemoryScope::session(SESSION),
        OLD_FACT,
    );
    let kept = write(
        &coordinator,
        "keep-contact",
        SESSION,
        MemoryScope::session(SESSION),
        "晨星项目的联络方式是站内留言。",
    );
    assert!(coordinator
        .recall(&query(SESSION, "晨星 联络"))
        .unwrap()
        .items
        .iter()
        .any(|item| item.id == forgotten.id));
    coordinator.protect_episode(&forgotten.id, 0).unwrap();
    assert!(matches!(
        coordinator.forget_episode(&forgotten.id, Some("owner request"), 1),
        Err(MemoryGovernanceError::Protected(_))
    ));
    coordinator.unprotect_episode(&forgotten.id, 1).unwrap();
    coordinator
        .forget_episode(&forgotten.id, Some("owner request"), 2)
        .unwrap();

    for layers in [
        vec![MemoryLayerKind::Working],
        vec![MemoryLayerKind::Episodic],
    ] {
        let query = query(SESSION, "晨星 联络").with_layers(layers);
        let recalled = coordinator.recall(&query).unwrap();
        assert!(!recalled.items.iter().any(|item| item.id == forgotten.id));
        assert!(recalled.items.iter().any(|item| item.id == kept.id));
        let selected = coordinator
            .compile_prompt_overlay_with_selected_access(&query)
            .unwrap()
            .unwrap();
        assert!(!selected.selected_candidate_ids.contains(&forgotten.id));
        assert!(!selected.overlay.contains("武汉"));
        assert!(selected.overlay.contains("站内留言"));
    }
    drop(coordinator);
    drop(store);
    let (store, coordinator) = open(&path);
    let recalled = coordinator.recall(&query(SESSION, "晨星 联络")).unwrap();
    assert!(!recalled.items.iter().any(|item| item.id == forgotten.id));
    assert!(recalled.items.iter().any(|item| item.id == kept.id));
    let selected = coordinator
        .compile_prompt_overlay_with_selected_access(&query(SESSION, "晨星 联络"))
        .unwrap()
        .unwrap();
    assert!(!selected.overlay.contains("武汉"));
    assert!(selected.overlay.contains("站内留言"));
    assert_eq!(
        store
            .get_governed(&forgotten.id)
            .unwrap()
            .unwrap()
            .status
            .as_str(),
        "forgotten"
    );
    assert_eq!(
        serde_json::to_value(
            EpisodeStore::get_episode(store.as_ref(), &forgotten.id)
                .unwrap()
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(forgotten).unwrap()
    );
}

#[test]
fn chinese_competing_facts_require_exact_subject_persona_and_session_scopes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("scopes.sqlite3");
    let (store, coordinator) = open(&path);
    let user_scope = MemoryScope::User {
        user_id: "user-a".into(),
    };
    let persona_scope = MemoryScope::Persona {
        persona_id: "persona-a".into(),
        user_id: "user-a".into(),
    };
    let records = [
        (
            "user-a-fact",
            "source-a",
            user_scope.clone(),
            "晨星项目的联络城市是上海。",
        ),
        (
            "user-b-fact",
            "source-b",
            MemoryScope::User {
                user_id: "user-b".into(),
            },
            "晨星项目的联络城市是深圳。",
        ),
        (
            "persona-a-fact",
            "source-a",
            persona_scope.clone(),
            "晨星项目的联络人是林青。",
        ),
        (
            "persona-b-fact",
            "source-a",
            MemoryScope::Persona {
                persona_id: "persona-b".into(),
                user_id: "user-a".into(),
            },
            "晨星项目的联络人是周宁。",
        ),
        (
            "other-session",
            "source-b",
            MemoryScope::session("source-b"),
            "晨星项目的联络方式是电话。",
        ),
        (
            "global-fact",
            "source-global",
            MemoryScope::Global,
            "晨星项目的联络时间是深夜。",
        ),
    ];
    for (id, session, scope, text) in records {
        write(&coordinator, id, session, scope, text);
    }
    drop(coordinator);
    drop(store);
    let (_, coordinator) = open(&path);
    let query = query("new-session", "晨星 项目 联络")
        .with_layers(vec![MemoryLayerKind::Episodic])
        .with_visible_scopes(vec![user_scope.clone(), persona_scope.clone()]);
    let recalled = coordinator.recall(&query).unwrap();
    let ids = recalled
        .items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        ids,
        std::collections::BTreeSet::from(["user-a-fact", "persona-a-fact"])
    );
    let selected = coordinator
        .compile_prompt_overlay_with_selected_access(&query)
        .unwrap()
        .unwrap();
    assert!(selected.overlay.contains("上海"));
    assert!(selected.overlay.contains("林青"));
    for excluded in ["深圳", "周宁", "电话", "深夜"] {
        assert!(
            !selected.overlay.contains(excluded),
            "scope leaked {excluded}"
        );
    }
    let raw = coordinator
        .backend()
        .get_episode("user-a-fact")
        .unwrap()
        .unwrap();
    let (scope, provenance) = coordinator.episode_scope_metadata(&raw);
    assert_eq!(scope, user_scope);
    assert_eq!(provenance.source_session.as_deref(), Some("source-a"));
    assert_eq!(
        provenance.source_trace.as_deref(),
        Some("trace-user-a-fact")
    );
    assert_eq!(
        provenance.source_request.as_deref(),
        Some("request-user-a-fact")
    );
    assert!(selected.overlay.chars().count() <= query.max_chars);
}

#[test]
fn repeated_chinese_evidence_is_deduplicated_at_recall_without_erasing_sources() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("duplicates.sqlite3");
    let (store, coordinator) = open(&path);
    for id in ["repeat-one", "repeat-two", "repeat-three"] {
        write(
            &coordinator,
            id,
            SESSION,
            MemoryScope::session(SESSION),
            "我喜欢简明的中文说明。",
        );
    }
    assert_eq!(
        EpisodeStore::count_by_session(store.as_ref(), SESSION).unwrap(),
        3
    );
    let query = query(SESSION, "中文 说明");
    let recalled = coordinator.recall(&query).unwrap();
    assert_eq!(
        recalled.items.len(),
        1,
        "duplicates must not occupy the recall budget"
    );
    let selected = coordinator
        .compile_prompt_overlay_with_selected_access(&query)
        .unwrap()
        .unwrap();
    assert_eq!(selected.selected_candidate_ids.len(), 1);
    assert_eq!(
        selected.overlay.matches("我喜欢简明的中文说明。").count(),
        1
    );
}

#[tokio::test]
async fn chinese_user_assertions_are_not_inferred_from_assistant_or_tool_text() {
    for role in ["user", "assistant", "tool", "system"] {
        let mut input = BoundedMemoryInput::new(
            MemoryScope::session(SESSION),
            vec![MemoryExtractionMessage {
                role: role.into(),
                content: "我喜欢简明的中文说明。".into(),
            }],
        );
        input.source_session = Some(SESSION.into());
        input.source_request = Some("role-check".into());
        let extracted = MemoryMaterializer::default().extract(input).await.unwrap();
        if role == "user" {
            assert_eq!(extracted.preferences.len(), 1);
            assert_eq!(
                extracted.preferences[0].scope,
                MemoryScope::session(SESSION)
            );
            assert_eq!(
                extracted.preferences[0]
                    .provenance
                    .source_request
                    .as_deref(),
                Some("role-check")
            );
        } else {
            assert_eq!(
                extracted,
                MemoryExtractionResult::default(),
                "{role} is source text, not a user assertion"
            );
        }
    }
}
