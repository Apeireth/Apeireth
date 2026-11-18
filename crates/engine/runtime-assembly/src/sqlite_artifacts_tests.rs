use super::*;

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn human() -> ArtifactAccess {
    ArtifactAccess::local_human()
}

async fn fixture() -> (tempfile::TempDir, SqliteArtifactService, String) {
    let directory = tempfile::tempdir().unwrap();
    let service = SqliteArtifactService::open_with_sealer(directory.path(), None)
        .await
        .unwrap();
    let store = service.capabilities(human()).await.unwrap().store_id;
    (directory, service, store)
}

fn create_request(store: &str, content: &str) -> CreateArtifactRequest {
    CreateArtifactRequest {
        store_id: store.into(),
        operation_id: id(),
        work_id: "local-work-2".into(),
        title: "项目方案".into(),
        media_type: "text/markdown".into(),
        content: content.into(),
        conversation_session_id: None,
    }
}

fn commit_request(store: &str, head: &str, content: &str) -> CommitArtifactRequest {
    CommitArtifactRequest {
        store_id: store.into(),
        operation_id: id(),
        expected_head: head.into(),
        content: content.into(),
    }
}

#[tokio::test]
async fn exact_text_noop_and_old_receipt_remain_truthful_after_new_head() {
    let (_directory, service, store) = fixture().await;
    let request = create_request(&store, "# 初稿😀\r\n第二行\n");
    let original = service.create(human(), request.clone()).await.unwrap();
    assert!(original.changed);
    assert_eq!(original.revision.as_ref().unwrap().content, request.content);
    let artifact = original.artifact.artifact_id.clone();
    let change = commit_request(
        &store,
        &original.artifact.head_revision,
        "用户修改的第二版😀\r\n",
    );
    let saved = service
        .commit(human(), artifact.clone(), change)
        .await
        .unwrap();
    assert_eq!(saved.artifact.sequence, 2);
    let no_change = service
        .commit(
            human(),
            artifact.clone(),
            commit_request(
                &store,
                &saved.artifact.head_revision,
                "用户修改的第二版😀\r\n",
            ),
        )
        .await
        .unwrap();
    assert!(!no_change.changed);
    assert_eq!(
        no_change.artifact.head_revision,
        saved.artifact.head_revision
    );
    assert_eq!(
        service.create(human(), request.clone()).await.unwrap(),
        original
    );
    assert_eq!(
        service
            .operation(human(), store.clone(), request.operation_id.clone())
            .await
            .unwrap(),
        original
    );
    let mut conflicting = request;
    conflicting.content = "异载荷不能抢同一操作".into();
    assert_eq!(
        service.create(human(), conflicting).await.unwrap_err(),
        ArtifactError::OperationMismatch
    );
    let current = service
        .read(human(), store.clone(), artifact.clone(), None)
        .await
        .unwrap();
    assert_eq!(current.artifact.head_revision, saved.artifact.head_revision);
    let history = service
        .history(
            human(),
            artifact.clone(),
            ArtifactHistoryQuery {
                store_id: store.clone(),
                after: None,
                limit: Some(1),
            },
        )
        .await
        .unwrap();
    assert_eq!(history.revisions.len(), 1);
    assert_eq!(history.next_cursor, Some(1));
    assert_eq!(
        history.revisions[0].revision_id,
        original.artifact.head_revision
    );
    let old = service
        .read(
            human(),
            store,
            artifact,
            Some(original.artifact.head_revision),
        )
        .await
        .unwrap();
    assert_eq!(old.revision.content, "# 初稿😀\r\n第二行\n");
}

#[tokio::test]
async fn independent_connections_cas_once_and_persist_conflict_and_receipt_across_reopen() {
    let (directory, first, store) = fixture().await;
    let second = SqliteArtifactService::open_with_sealer(directory.path(), None)
        .await
        .unwrap();
    let original = first
        .create(human(), create_request(&store, "第一版"))
        .await
        .unwrap();
    let artifact = original.artifact.artifact_id.clone();
    let left = commit_request(&store, &original.artifact.head_revision, "左边的新稿");
    let right = commit_request(&store, &original.artifact.head_revision, "右边的新稿");
    let (a, b) = tokio::join!(
        first.commit(human(), artifact.clone(), left.clone()),
        second.commit(human(), artifact.clone(), right.clone())
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(
        [&a, &b].iter().filter(|r| r.outcome == "committed").count(),
        1
    );
    assert_eq!(
        [&a, &b].iter().filter(|r| r.outcome == "conflict").count(),
        1
    );
    let (conflict, request) = if a.outcome == "conflict" {
        (a, left)
    } else {
        (b, right)
    };
    let current = first
        .read(human(), store.clone(), artifact.clone(), None)
        .await
        .unwrap();
    assert_eq!(current.artifact.sequence, 2);
    let newest = first
        .commit(
            human(),
            artifact.clone(),
            commit_request(&store, &current.artifact.head_revision, "第三版"),
        )
        .await
        .unwrap();
    assert_eq!(
        first
            .commit(human(), artifact.clone(), request)
            .await
            .unwrap(),
        conflict
    );
    drop(first);
    drop(second);
    let reopened = SqliteArtifactService::open_with_sealer(directory.path(), None)
        .await
        .unwrap();
    assert_eq!(
        reopened.capabilities(human()).await.unwrap().store_id,
        store
    );
    assert_eq!(
        reopened
            .operation(human(), store.clone(), conflict.operation_id.clone())
            .await
            .unwrap(),
        conflict
    );
    assert_eq!(
        reopened
            .read(human(), store.clone(), artifact, None)
            .await
            .unwrap()
            .artifact
            .head_revision,
        newest.artifact.head_revision
    );
    assert_eq!(
        reopened.operation(human(), store, id()).await.unwrap_err(),
        ArtifactError::NotFound
    );
}

#[tokio::test]
async fn restore_creates_new_history_even_for_equal_text_and_rejects_stale_head() {
    let (_directory, service, store) = fixture().await;
    let first = service
        .create(human(), create_request(&store, "A"))
        .await
        .unwrap();
    let artifact = first.artifact.artifact_id.clone();
    let second = service
        .commit(
            human(),
            artifact.clone(),
            commit_request(&store, &first.artifact.head_revision, "B"),
        )
        .await
        .unwrap();
    let restore = RestoreArtifactRequest {
        store_id: store.clone(),
        operation_id: id(),
        expected_head: second.artifact.head_revision,
        revision_id: first.artifact.head_revision.clone(),
    };
    let restored = service
        .restore(human(), artifact.clone(), restore.clone())
        .await
        .unwrap();
    assert!(restored.changed);
    assert_eq!(restored.artifact.sequence, 3);
    assert_ne!(
        restored.artifact.head_revision,
        first.artifact.head_revision
    );
    assert_eq!(
        restored.revision.as_ref().unwrap().restored_from,
        Some(first.artifact.head_revision.clone())
    );
    assert_eq!(restored.revision.as_ref().unwrap().content, "A");
    let mut same = restore.clone();
    same.operation_id = id();
    same.expected_head = restored.artifact.head_revision;
    let same = service
        .restore(human(), artifact.clone(), same)
        .await
        .unwrap();
    assert!(same.changed);
    assert_eq!(same.artifact.sequence, 4);
    let mut stale = restore;
    stale.operation_id = id();
    assert_eq!(
        service
            .restore(human(), artifact.clone(), stale)
            .await
            .unwrap()
            .outcome,
        "conflict"
    );
    assert_eq!(
        service
            .read(human(), store, artifact, Some(first.artifact.head_revision))
            .await
            .unwrap()
            .revision
            .content,
        "A"
    );
}

#[tokio::test]
async fn invalid_scope_store_and_payloads_cannot_mutate_or_probe_receipts() {
    let (directory, service, store) = fixture().await;
    let mut foreign = human();
    foreign.namespace = "another-user-domain".into();
    assert_eq!(
        service.capabilities(foreign.clone()).await.unwrap_err(),
        ArtifactError::ScopeUnavailable
    );
    assert_eq!(
        service
            .create(foreign, create_request(&store, "bad"))
            .await
            .unwrap_err(),
        ArtifactError::ScopeUnavailable
    );
    let mut wrong = create_request(&store, "bad");
    wrong.store_id = id();
    assert_eq!(
        service.create(human(), wrong).await.unwrap_err(),
        ArtifactError::StoreMismatch
    );
    for mut request in [
        create_request(&store, "bad\0text"),
        create_request(&store, &"a".repeat(MAX_ARTIFACT_CONTENT_BYTES + 1)),
        create_request(&store, "bad"),
    ]
    .into_iter()
    .enumerate()
    {
        if request.0 == 2 {
            request.1.work_id = "bad/path".into();
        }
        assert!(matches!(
            service.create(human(), request.1).await,
            Err(ArtifactError::Invalid(_))
        ));
    }
    let empty = service
        .create(human(), create_request(&store, ""))
        .await
        .unwrap();
    assert_eq!(empty.revision.as_ref().unwrap().content, "");
    assert_eq!(
        service
            .list(
                human(),
                ArtifactListQuery {
                    store_id: store.clone(),
                    work_id: "local-work-2".into(),
                    after: None,
                    limit: None
                }
            )
            .await
            .unwrap()
            .artifacts
            .len(),
        1
    );
    std::fs::write(directory.path().join("users.json"), "{}").unwrap();
    assert_eq!(
        service.capabilities(human()).await.unwrap_err(),
        ArtifactError::ScopeUnavailable
    );
    assert_eq!(
        service
            .operation(human(), store, empty.operation_id)
            .await
            .unwrap_err(),
        ArtifactError::ScopeUnavailable
    );
}

#[tokio::test]
async fn revisions_from_other_artifacts_are_not_readable_or_restorable() {
    let (_directory, service, store) = fixture().await;
    let a = service
        .create(human(), create_request(&store, "A"))
        .await
        .unwrap();
    let b = service
        .create(human(), create_request(&store, "B"))
        .await
        .unwrap();
    let wrong = b.artifact.head_revision;
    assert_eq!(
        service
            .read(
                human(),
                store.clone(),
                a.artifact.artifact_id.clone(),
                Some(wrong.clone())
            )
            .await
            .unwrap_err(),
        ArtifactError::NotFound
    );
    assert_eq!(
        service
            .restore(
                human(),
                a.artifact.artifact_id.clone(),
                RestoreArtifactRequest {
                    store_id: store.clone(),
                    operation_id: id(),
                    expected_head: a.artifact.head_revision.clone(),
                    revision_id: wrong
                }
            )
            .await
            .unwrap_err(),
        ArtifactError::NotFound
    );
    assert_eq!(
        service
            .read(human(), store, a.artifact.artifact_id, None)
            .await
            .unwrap()
            .artifact
            .head_revision,
        a.artifact.head_revision
    );
}

#[tokio::test]
async fn canonical_operation_vector_and_malformed_page_are_precise() {
    let (_directory, service, store) = fixture().await;
    let request = CreateArtifactRequest {
        store_id: "11111111-1111-4111-8111-111111111111".into(),
        operation_id: "22222222-2222-4222-8222-222222222222".into(),
        work_id: "local-work-2".into(),
        title: "项目方案".into(),
        media_type: "text/markdown".into(),
        content: "# 初稿😀\r\n第二行\n".into(),
        conversation_session_id: None,
    };
    let canonical = canonical_artifact_request("create", None, &request).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(canonical)),
        "2d3011b41b435f59a8deac469f17be8b9c9b13b0a7a200d1c86ca0f4c3e3ffbd"
    );
    assert!(matches!(
        service
            .list(
                human(),
                ArtifactListQuery {
                    store_id: store,
                    work_id: "local-work-2".into(),
                    after: None,
                    limit: Some(33)
                }
            )
            .await,
        Err(ArtifactError::Invalid(_))
    ));
}

#[tokio::test]
async fn sealing_covers_content_and_receipts_and_refuses_mode_change_or_tampering() {
    use apeireth_storage::crypto::{PartitionId, PartitionKey};
    let directory = tempfile::tempdir().unwrap();
    let sealer = SessionSealer::new(PartitionKey::from_bytes(
        PartitionId::new("test.artifacts").unwrap(),
        [7; 32],
    ));
    let service = SqliteArtifactService::open_with_sealer(directory.path(), Some(sealer.clone()))
        .await
        .unwrap();
    let store = service.capabilities(human()).await.unwrap().store_id;
    let secret = "private artifact body not plaintext at rest";
    let receipt = service
        .create(human(), create_request(&store, secret))
        .await
        .unwrap();
    service
        .pool
        .read(|connection| {
            for table in [
                "managed_artifacts",
                "artifact_revisions",
                "artifact_receipts",
            ] {
                let body: String = connection.query_row(
                    &format!("SELECT data FROM {table} LIMIT 1"),
                    [],
                    |row| row.get(0),
                )?;
                assert!(SessionSealer::is_sealed(&body));
                assert!(!body.contains(secret));
            }
            Ok(())
        })
        .unwrap();
    let wrong_mode = SqliteArtifactService::open_with_sealer(directory.path(), None).await;
    assert!(matches!(wrong_mode, Err(ArtifactError::Storage(_))));
    let artifact = receipt.artifact.artifact_id.clone();
    let reopened = SqliteArtifactService::open_with_sealer(directory.path(), Some(sealer))
        .await
        .unwrap();
    assert_eq!(
        reopened
            .read(human(), store.clone(), artifact.clone(), None)
            .await
            .unwrap()
            .revision
            .content,
        secret
    );
    service
        .pool
        .write(|connection| {
            connection.execute(
                "UPDATE artifact_revisions SET data='plaintext replacement'",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(matches!(
        reopened.read(human(), store, artifact, None).await,
        Err(ArtifactError::Storage(_))
    ));
}
