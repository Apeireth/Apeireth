//! One transaction owns an immutable revision, its head and replay outcome.
use super::*;

impl SqliteArtifactService {
    pub(super) async fn mutate<T: Serialize + Send + 'static>(
        &self,
        access: ArtifactAccess,
        operation_id: String,
        request: T,
        plan: MutationPlan,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.authorize(&access)?;
        let bytes = canonical_artifact_request(plan.kind, plan.artifact_id.as_deref(), &request)
            .map_err(storage)?;
        let fingerprint = format!("{:x}", Sha256::digest(bytes));
        let this = self.clone();
        self.pool
            .write(move |connection| {
                let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
                match this.apply_mutation(&tx, access, operation_id, fingerprint, plan) {
                    Ok(receipt) => {
                        tx.commit()?;
                        Ok(Ok(receipt))
                    }
                    Err(error) => Ok(Err(error)),
                }
            })
            .await
            .map_err(storage)?
    }

    fn apply_mutation(
        &self,
        tx: &Connection,
        access: ArtifactAccess,
        operation_id: String,
        fingerprint: String,
        plan: MutationPlan,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.authorize(&access)?;
        if let Some(previous) = self
            .load_receipt(tx, &access.namespace, &operation_id)
            .map_err(storage)?
        {
            return if previous.request_sha256 == fingerprint {
                Ok(previous)
            } else {
                Err(ArtifactError::OperationMismatch)
            };
        }
        let count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM artifact_receipts WHERE namespace=?1",
                [&access.namespace],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if count >= MAX_ARTIFACT_RECEIPTS as i64 {
            return Err(ArtifactError::Capacity);
        }
        let now = chrono::Utc::now().timestamp_millis();
        let artifact = if let Some(create) = plan.create {
            self.new_artifact(tx, &access.namespace, create, now)?
        } else {
            self.load_artifact(
                tx,
                &access.namespace,
                plan.artifact_id
                    .as_deref()
                    .expect("validated mutation identity"),
            )
            .map_err(storage)?
            .ok_or(ArtifactError::NotFound)?
        };
        let conflict = plan
            .expected
            .as_ref()
            .is_some_and(|head| head != &artifact.head_revision);
        let mut receipt = ArtifactReceipt {
            schema: ARTIFACT_SCHEMA,
            store_id: self.store_id.clone(),
            operation_id: operation_id.clone(),
            request_sha256: fingerprint.clone(),
            outcome: if conflict { "conflict" } else { "committed" }.into(),
            changed: false,
            artifact,
            revision: None,
            expected_head: plan.expected,
        };
        if !conflict {
            let content = match plan.restored_from.as_deref() {
                Some(id) => {
                    self.load_revision(tx, &access.namespace, &receipt.artifact.artifact_id, id)
                        .map_err(storage)?
                        .ok_or(ArtifactError::NotFound)?
                        .content
                }
                None => plan.content.expect("validated mutation content"),
            };
            self.advance(tx, &access, &mut receipt, content, plan.restored_from, now)?;
        }
        // A revoked local scope or cancelled queued call never commits.
        self.authorize(&access)?;
        tx.execute(
            "INSERT INTO artifact_receipts(namespace,operation_id,request_sha256,data) VALUES(?1,?2,?3,?4)",
            params![access.namespace, operation_id, fingerprint, self.encode(&receipt).map_err(storage)?],
        ).map_err(storage)?;
        Ok(receipt)
    }

    fn new_artifact(
        &self,
        tx: &Connection,
        namespace: &str,
        create: CreateArtifactRequest,
        now: i64,
    ) -> Result<Artifact, ArtifactError> {
        let count: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM managed_artifacts WHERE namespace=?1",
                [namespace],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if count >= MAX_ARTIFACTS as i64 {
            return Err(ArtifactError::Capacity);
        }
        Ok(Artifact {
            artifact_id: uuid::Uuid::new_v4().to_string(),
            work_id: create.work_id,
            conversation_session_id: create.conversation_session_id,
            title: create.title,
            media_type: create.media_type,
            head_revision: String::new(),
            sequence: 0,
            content_sha256: String::new(),
            content_bytes: 0,
            created_at: now,
            updated_at: now,
        })
    }

    fn advance(
        &self,
        tx: &Connection,
        access: &ArtifactAccess,
        receipt: &mut ArtifactReceipt,
        content: String,
        restored_from: Option<String>,
        now: i64,
    ) -> Result<(), ArtifactError> {
        let artifact = &mut receipt.artifact;
        let current = if artifact.sequence == 0 {
            None
        } else {
            Some(
                self.load_revision(
                    tx,
                    &access.namespace,
                    &artifact.artifact_id,
                    &artifact.head_revision,
                )
                .map_err(storage)?
                .ok_or_else(|| storage("artifact head revision is missing"))?,
            )
        };
        // Restore always records a new historical fact, including equal text.
        let changed =
            restored_from.is_some() || current.as_ref().is_none_or(|r| r.content != content);
        let revision = if changed {
            let revision = ArtifactRevision {
                revision_id: uuid::Uuid::new_v4().to_string(),
                artifact_id: artifact.artifact_id.clone(),
                parent_revision: current.as_ref().map(|r| r.revision_id.clone()),
                sequence: artifact
                    .sequence
                    .checked_add(1)
                    .ok_or(ArtifactError::Capacity)?,
                content_sha256: artifact_digest(&content),
                content_bytes: content.len() as u64,
                content,
                created_at: now,
                author: access.author.clone(),
                origin: access.origin.clone(),
                restored_from,
            };
            artifact.head_revision = revision.revision_id.clone();
            artifact.sequence = revision.sequence;
            artifact.content_sha256 = revision.content_sha256.clone();
            artifact.content_bytes = revision.content_bytes;
            artifact.updated_at = now;
            tx.execute(
                "INSERT INTO managed_artifacts(id,namespace,work_id,data) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
                params![artifact.artifact_id,access.namespace,artifact.work_id,self.encode(&artifact).map_err(storage)?],
            ).map_err(storage)?;
            tx.execute(
                "INSERT INTO artifact_revisions(id,artifact_id,namespace,sequence,data) VALUES(?1,?2,?3,?4,?5)",
                params![revision.revision_id,revision.artifact_id,access.namespace,revision.sequence as i64,self.encode(&revision).map_err(storage)?],
            ).map_err(storage)?;
            revision
        } else {
            current.expect("unchanged revision exists")
        };
        receipt.changed = changed;
        receipt.revision = Some(revision);
        Ok(())
    }
}
