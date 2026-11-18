//! Durable managed text versions. Human and governed-tool exits share this
//! service; SQLite transactions own CAS and durable operation outcomes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use apeireth_protocol::artifacts::*;
use apeireth_storage::{SqliteConfig, SqliteConnectionPool, StorageError, SynchronousMode};
use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

use crate::canonical::session_seal::SessionSealer;

mod mutations;

pub const ARTIFACT_DATABASE_FILE: &str = "artifacts.sqlite3";

#[derive(Clone)]
pub struct SqliteArtifactService {
    pool: Arc<SqliteConnectionPool>,
    store_id: String,
    data_root: PathBuf,
    sealer: Option<SessionSealer>,
}

struct MutationPlan {
    kind: &'static str,
    artifact_id: Option<String>,
    content: Option<String>,
    expected: Option<String>,
    create: Option<CreateArtifactRequest>,
    restored_from: Option<String>,
}

fn storage(error: impl std::fmt::Display) -> ArtifactError {
    ArtifactError::Storage(error.to_string())
}

fn serialized(error: impl std::fmt::Display) -> StorageError {
    StorageError::Serialization(error.to_string())
}

pub fn artifact_digest(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

fn uuid(value: &str) -> Result<(), ArtifactError> {
    if valid_artifact_uuid(value) {
        Ok(())
    } else {
        Err(ArtifactError::Invalid(
            "canonical non-nil UUID required".into(),
        ))
    }
}

fn work(value: &str) -> Result<(), ArtifactError> {
    if !value.is_empty()
        && value.len() <= MAX_ARTIFACT_WORK_BYTES
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.' | b':'))
    {
        Ok(())
    } else {
        Err(ArtifactError::Invalid("invalid local work grouping".into()))
    }
}

fn text(value: &str) -> Result<(), ArtifactError> {
    if value.len() > MAX_ARTIFACT_CONTENT_BYTES || value.contains('\0') {
        Err(ArtifactError::Invalid(
            "invalid text content or content limit exceeded".into(),
        ))
    } else {
        Ok(())
    }
}

fn limit(value: Option<u8>) -> Result<usize, ArtifactError> {
    match value.unwrap_or(16) {
        value @ 1..=MAX_ARTIFACT_PAGE_LIMIT => Ok(value as usize),
        _ => Err(ArtifactError::Invalid(
            "page limit must be 1 through 32".into(),
        )),
    }
}

impl SqliteArtifactService {
    /// Fixed managed data root, supplied only by the composition root.
    pub async fn open(data_root: impl AsRef<Path>) -> Result<Self, ArtifactError> {
        let sealer = crate::canonical::session_seal::sealer_from_env().map_err(storage)?;
        Self::open_with_sealer(data_root, sealer).await
    }

    /// Explicit key injection for embedded hosts and persistence tests.
    pub async fn open_with_sealer(
        data_root: impl AsRef<Path>,
        sealer: Option<SessionSealer>,
    ) -> Result<Self, ArtifactError> {
        let root = data_root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root).map_err(storage)?;
        let pool = Arc::new(
            SqliteConnectionPool::open_with_config(
                root.join(ARTIFACT_DATABASE_FILE),
                SqliteConfig::default().with_synchronous(SynchronousMode::Full),
            )
            .await
            .map_err(storage)?,
        );
        let mode = if sealer.is_some() {
            "session_seal_v1"
        } else {
            "plain_v1"
        };
        let store_id = pool.write(move |connection| {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch("CREATE TABLE IF NOT EXISTS artifact_store_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), schema INTEGER NOT NULL, store_id TEXT NOT NULL, encryption_mode TEXT NOT NULL);")?;
            tx.execute("INSERT OR IGNORE INTO artifact_store_meta(singleton,schema,store_id,encryption_mode) VALUES(1,1,?1,?2)", params![uuid::Uuid::new_v4().to_string(), mode])?;
            let (schema, id, current_mode): (i64, String, String) = tx.query_row("SELECT schema,store_id,encryption_mode FROM artifact_store_meta WHERE singleton=1", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)))?;
            if schema != 1 || !valid_artifact_uuid(&id) || current_mode != mode {
                return Err(serialized("artifact schema or encryption configuration mismatch; explicit migration required"));
            }
            tx.execute_batch("CREATE TABLE IF NOT EXISTS managed_artifacts (id TEXT PRIMARY KEY, namespace TEXT NOT NULL, work_id TEXT NOT NULL, data TEXT NOT NULL);
                CREATE INDEX IF NOT EXISTS managed_artifacts_work ON managed_artifacts(namespace,work_id,id);
                CREATE TABLE IF NOT EXISTS artifact_revisions (id TEXT PRIMARY KEY, artifact_id TEXT NOT NULL REFERENCES managed_artifacts(id), namespace TEXT NOT NULL, sequence INTEGER NOT NULL, data TEXT NOT NULL, UNIQUE(artifact_id,sequence));
                CREATE TABLE IF NOT EXISTS artifact_receipts (namespace TEXT NOT NULL, operation_id TEXT NOT NULL, request_sha256 TEXT NOT NULL, data TEXT NOT NULL, PRIMARY KEY(namespace,operation_id));")?;
            tx.commit()?;
            Ok(id)
        }).await.map_err(storage)?;
        Ok(Self {
            pool,
            store_id,
            data_root: root,
            sealer,
        })
    }

    /// A newly configured user table revokes the local-only face immediately.
    /// No content or owner is read from that file by this service.
    fn authorize(&self, access: &ArtifactAccess) -> Result<(), ArtifactError> {
        if access.namespace != SINGLE_OPERATOR_NAMESPACE || !access.is_active() {
            return Err(ArtifactError::ScopeUnavailable);
        }
        match std::fs::symlink_metadata(self.data_root.join("users.json")) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err(ArtifactError::ScopeUnavailable),
        }
        match (&access.author.kind, &access.origin) {
            (ArtifactAuthorKind::Human, None) if access.author.id == "local-operator" => Ok(()),
            (ArtifactAuthorKind::Agent, Some(origin))
                if !access.author.id.is_empty()
                    && valid_artifact_uuid(&origin.session_id)
                    && valid_artifact_uuid(&origin.request_id)
                    && valid_artifact_uuid(&origin.trace_id) =>
            {
                Ok(())
            }
            _ => Err(ArtifactError::ScopeUnavailable),
        }
    }

    fn check_store(&self, value: &str) -> Result<(), ArtifactError> {
        uuid(value)?;
        if value == self.store_id {
            Ok(())
        } else {
            Err(ArtifactError::StoreMismatch)
        }
    }

    fn encode<T: Serialize>(&self, value: &T) -> Result<String, StorageError> {
        let bytes = serde_json::to_vec(value).map_err(serialized)?;
        match &self.sealer {
            Some(sealer) => sealer.seal_to_text(&bytes).map_err(serialized),
            None => String::from_utf8(bytes).map_err(serialized),
        }
    }

    fn decode<T: DeserializeOwned>(&self, value: &str) -> Result<T, StorageError> {
        let bytes = match &self.sealer {
            Some(sealer) => {
                if !SessionSealer::is_sealed(value) {
                    return Err(serialized("invalid sealed artifact record"));
                }
                sealer.open_text(value).map_err(serialized)?
            }
            None => value.as_bytes().to_vec(),
        };
        serde_json::from_slice(&bytes).map_err(serialized)
    }

    fn load_artifact(
        &self,
        connection: &Connection,
        namespace: &str,
        id: &str,
    ) -> Result<Option<Artifact>, StorageError> {
        let value: Option<String> = connection
            .query_row(
                "SELECT data FROM managed_artifacts WHERE namespace=?1 AND id=?2",
                params![namespace, id],
                |row| row.get(0),
            )
            .optional()?;
        let artifact = value.map(|v| self.decode::<Artifact>(&v)).transpose()?;
        if artifact.as_ref().is_some_and(|a| {
            a.artifact_id != id
                || !valid_artifact_uuid(&a.head_revision)
                || a.sequence == 0
                || a.content_bytes > MAX_ARTIFACT_CONTENT_BYTES as u64
        }) {
            return Err(serialized(
                "artifact identity does not match its durable key",
            ));
        }
        Ok(artifact)
    }

    fn load_revision(
        &self,
        connection: &Connection,
        namespace: &str,
        artifact: &str,
        id: &str,
    ) -> Result<Option<ArtifactRevision>, StorageError> {
        let value: Option<String> = connection.query_row("SELECT data FROM artifact_revisions WHERE namespace=?1 AND artifact_id=?2 AND id=?3",params![namespace,artifact,id],|row| row.get(0)).optional()?;
        let revision = value
            .map(|v| self.decode::<ArtifactRevision>(&v))
            .transpose()?;
        if revision.as_ref().is_some_and(|r| {
            r.revision_id != id
                || r.artifact_id != artifact
                || r.content.len() as u64 != r.content_bytes
                || r.content_sha256 != artifact_digest(&r.content)
                || r.content_bytes > MAX_ARTIFACT_CONTENT_BYTES as u64
                || r.sequence == 0
                || r.content.contains('\0')
        }) {
            return Err(serialized("artifact revision identity or digest mismatch"));
        }
        Ok(revision)
    }

    fn load_receipt(
        &self,
        connection: &Connection,
        namespace: &str,
        id: &str,
    ) -> Result<Option<ArtifactReceipt>, StorageError> {
        let value: Option<(String,String)> = connection.query_row("SELECT request_sha256,data FROM artifact_receipts WHERE namespace=?1 AND operation_id=?2",params![namespace,id],|row| Ok((row.get(0)?,row.get(1)?))).optional()?;
        value
            .map(|(digest, data)| {
                let receipt: ArtifactReceipt = self.decode(&data)?;
                if receipt.operation_id != id
                    || receipt.store_id != self.store_id
                    || receipt.request_sha256 != digest
                {
                    return Err(serialized("artifact receipt identity mismatch"));
                }
                Ok(receipt)
            })
            .transpose()
    }

    async fn read_snapshot<T, F>(&self, access: ArtifactAccess, f: F) -> Result<T, ArtifactError>
    where
        T: Send + 'static,
        F: FnOnce(&Self, &Connection, &ArtifactAccess) -> Result<T, ArtifactError> + Send + 'static,
    {
        self.authorize(&access)?;
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            this.authorize(&access)?;
            this.pool
                .read(|connection| {
                    let tx = connection.unchecked_transaction()?;
                    // Keep the domain result separate from the persistence error.
                    Ok(f(&this, &tx, &access))
                })
                .map_err(storage)?
        })
        .await
        .map_err(storage)?
    }
}

#[async_trait]
impl ArtifactService for SqliteArtifactService {
    async fn capabilities(
        &self,
        access: ArtifactAccess,
    ) -> Result<ArtifactCapabilities, ArtifactError> {
        self.authorize(&access)?;
        Ok(ArtifactCapabilities {
            schema: ARTIFACT_SCHEMA,
            store_id: self.store_id.clone(),
            scope: SINGLE_OPERATOR_NAMESPACE.into(),
            media_types: vec!["text/plain".into(), "text/markdown".into()],
            max_content_bytes: MAX_ARTIFACT_CONTENT_BYTES,
            max_title_bytes: MAX_ARTIFACT_TITLE_BYTES,
            max_page_limit: MAX_ARTIFACT_PAGE_LIMIT,
            max_artifacts: MAX_ARTIFACTS,
            max_receipts: MAX_ARTIFACT_RECEIPTS,
        })
    }
    async fn list(
        &self,
        access: ArtifactAccess,
        query: ArtifactListQuery,
    ) -> Result<ArtifactListPage, ArtifactError> {
        self.authorize(&access)?;
        self.check_store(&query.store_id)?;
        work(&query.work_id)?;
        let size = limit(query.limit)?;
        if let Some(id) = &query.after {
            uuid(id)?;
        }
        self.read_snapshot(access,move|this,connection,access| {
            let mut statement=connection.prepare("SELECT data FROM managed_artifacts WHERE namespace=?1 AND work_id=?2 AND id>?3 ORDER BY id LIMIT ?4").map_err(storage)?;
            let rows=statement.query_map(params![access.namespace,query.work_id,query.after.unwrap_or_default(),size as i64+1],|row| row.get::<_,String>(0)).map_err(storage)?;
            let mut artifacts=rows.map(|value|this.decode::<Artifact>(&value.map_err(storage)?).map_err(storage)).collect::<Result<Vec<_>,ArtifactError>>()?;
            let more=artifacts.len()>size;artifacts.truncate(size);
            let next_cursor=if more {artifacts.last().map(|a|a.artifact_id.clone())} else {None};
            Ok(ArtifactListPage {schema:ARTIFACT_SCHEMA,store_id:this.store_id.clone(),work_id:query.work_id,artifacts,next_cursor})
        }).await
    }
    async fn read(
        &self,
        access: ArtifactAccess,
        store_id: String,
        artifact_id: String,
        revision_id: Option<String>,
    ) -> Result<ArtifactDocument, ArtifactError> {
        self.authorize(&access)?;
        self.check_store(&store_id)?;
        uuid(&artifact_id)?;
        if let Some(id) = &revision_id {
            uuid(id)?;
        }
        self.read_snapshot(access, move |this, connection, access| {
            let artifact = this
                .load_artifact(connection, &access.namespace, &artifact_id)
                .map_err(storage)?
                .ok_or(ArtifactError::NotFound)?;
            let revision = this
                .load_revision(
                    connection,
                    &access.namespace,
                    &artifact_id,
                    revision_id.as_deref().unwrap_or(&artifact.head_revision),
                )
                .map_err(storage)?
                .ok_or(ArtifactError::NotFound)?;
            if revision_id.is_none()
                && (revision.sequence != artifact.sequence
                    || revision.content_sha256 != artifact.content_sha256
                    || revision.content_bytes != artifact.content_bytes)
            {
                return Err(storage("inconsistent artifact head"));
            }
            Ok(ArtifactDocument {
                schema: ARTIFACT_SCHEMA,
                store_id: this.store_id.clone(),
                artifact,
                revision,
            })
        })
        .await
    }
    async fn history(
        &self,
        access: ArtifactAccess,
        artifact_id: String,
        query: ArtifactHistoryQuery,
    ) -> Result<ArtifactHistoryPage, ArtifactError> {
        self.authorize(&access)?;
        self.check_store(&query.store_id)?;
        uuid(&artifact_id)?;
        let size = limit(query.limit)?;
        let after = query.after.unwrap_or(0);
        if after > i64::MAX as u64 {
            return Err(ArtifactError::Invalid("invalid revision cursor".into()));
        }
        self.read_snapshot(access,move|this,connection,access| {
            let artifact=this.load_artifact(connection,&access.namespace,&artifact_id).map_err(storage)?.ok_or(ArtifactError::NotFound)?;
            if after>artifact.sequence {return Err(ArtifactError::Invalid("revision cursor exceeds the history".into()));}
            let mut statement=connection.prepare("SELECT data FROM artifact_revisions WHERE namespace=?1 AND artifact_id=?2 AND sequence>?3 ORDER BY sequence LIMIT ?4").map_err(storage)?;
            let rows=statement.query_map(params![access.namespace,artifact_id,after as i64,size as i64+1],|row|row.get::<_,String>(0)).map_err(storage)?;
            let mut revisions=rows.map(|value|{let revision=this.decode::<ArtifactRevision>(&value.map_err(storage)?).map_err(storage)?;Ok(ArtifactRevisionSummary::from(&revision))}).collect::<Result<Vec<_>,ArtifactError>>()?;
            let more=revisions.len()>size;revisions.truncate(size);let next_cursor=if more {revisions.last().map(|r|r.sequence)} else {None};
            Ok(ArtifactHistoryPage {schema:ARTIFACT_SCHEMA,store_id:this.store_id.clone(),artifact_id,revisions,next_cursor})
        }).await
    }
    async fn operation(
        &self,
        access: ArtifactAccess,
        store_id: String,
        operation_id: String,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.authorize(&access)?;
        self.check_store(&store_id)?;
        uuid(&operation_id)?;
        self.read_snapshot(access, move |this, connection, access| {
            this.load_receipt(connection, &access.namespace, &operation_id)
                .map_err(storage)?
                .ok_or(ArtifactError::NotFound)
        })
        .await
    }
    async fn create(
        &self,
        access: ArtifactAccess,
        request: CreateArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.authorize(&access)?;
        self.check_store(&request.store_id)?;
        uuid(&request.operation_id)?;
        work(&request.work_id)?;
        text(&request.content)?;
        if request.title.trim().is_empty()
            || request.title.len() > MAX_ARTIFACT_TITLE_BYTES
            || request.title.chars().any(char::is_control)
            || !matches!(request.media_type.as_str(), "text/plain" | "text/markdown")
        {
            return Err(ArtifactError::Invalid(
                "invalid artifact title or media type".into(),
            ));
        }
        if let Some(id) = &request.conversation_session_id {
            uuid(id)?;
        }
        self.mutate(
            access,
            request.operation_id.clone(),
            request.clone(),
            MutationPlan {
                kind: "create",
                artifact_id: None,
                content: Some(request.content.clone()),
                expected: None,
                create: Some(request),
                restored_from: None,
            },
        )
        .await
    }
    async fn commit(
        &self,
        access: ArtifactAccess,
        artifact_id: String,
        request: CommitArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.authorize(&access)?;
        self.check_store(&request.store_id)?;
        uuid(&artifact_id)?;
        uuid(&request.operation_id)?;
        uuid(&request.expected_head)?;
        text(&request.content)?;
        self.mutate(
            access,
            request.operation_id.clone(),
            request.clone(),
            MutationPlan {
                kind: "commit",
                artifact_id: Some(artifact_id),
                content: Some(request.content),
                expected: Some(request.expected_head),
                create: None,
                restored_from: None,
            },
        )
        .await
    }
    async fn restore(
        &self,
        access: ArtifactAccess,
        artifact_id: String,
        request: RestoreArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.authorize(&access)?;
        self.check_store(&request.store_id)?;
        uuid(&artifact_id)?;
        uuid(&request.operation_id)?;
        uuid(&request.expected_head)?;
        uuid(&request.revision_id)?;
        self.mutate(
            access,
            request.operation_id.clone(),
            request.clone(),
            MutationPlan {
                kind: "restore",
                artifact_id: Some(artifact_id),
                content: None,
                expected: Some(request.expected_head),
                create: None,
                restored_from: Some(request.revision_id),
            },
        )
        .await
    }
}

#[cfg(test)]
#[path = "sqlite_artifacts_tests.rs"]
mod tests;
