//! Managed text artifacts shared by governed tools and human transports.
//! Authentication is supplied by the host, never decoded from document input.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub const ARTIFACT_SCHEMA: u8 = 1;
pub const MAX_ARTIFACT_CONTENT_BYTES: usize = 256 * 1024;
pub const MAX_ARTIFACT_TITLE_BYTES: usize = 256;
pub const MAX_ARTIFACT_WORK_BYTES: usize = 128;
pub const MAX_ARTIFACT_PAGE_LIMIT: u8 = 32;
pub const MAX_ARTIFACTS: usize = 1024;
pub const MAX_ARTIFACT_RECEIPTS: usize = 100_000;
pub const SINGLE_OPERATOR_NAMESPACE: &str = "single_operator";

/// Canonical, non-nil UUID spelling. No identity is inferred from its bytes.
pub fn valid_artifact_uuid(value: &str) -> bool {
    value != "00000000-0000-0000-0000-000000000000"
        && value
            .parse::<apeireth_core::kernel::RequestId>()
            .is_ok_and(|id| id.to_string() == value)
}

/// UTF-8 bytes hashed for the durable operation. Object keys are recursively
/// sorted; the strict typed payload includes explicit null option fields.
/// Content itself is never normalized or reparsed.
pub fn canonical_artifact_request<T: Serialize>(
    kind: &str,
    artifact_id: Option<&str>,
    payload: &T,
) -> Result<Vec<u8>, serde_json::Error> {
    fn write(value: &serde_json::Value, out: &mut String) -> Result<(), serde_json::Error> {
        match value {
            serde_json::Value::Object(values) => {
                out.push('{');
                let mut keys = values.keys().collect::<Vec<_>>();
                keys.sort();
                for (index, key) in keys.into_iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(&serde_json::to_string(key)?);
                    out.push(':');
                    write(&values[key], out)?;
                }
                out.push('}');
            }
            serde_json::Value::Array(values) => {
                out.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write(value, out)?;
                }
                out.push(']');
            }
            other => out.push_str(&serde_json::to_string(other)?),
        }
        Ok(())
    }
    let value = serde_json::json!({"kind":kind,"artifact_id":artifact_id,"payload":payload});
    let mut output = String::new();
    write(&value, &mut output)?;
    Ok(output.into_bytes())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactAuthorKind {
    Human,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactAuthor {
    pub kind: ArtifactAuthorKind,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactOrigin {
    pub session_id: String,
    pub request_id: String,
    pub trace_id: String,
}

/// Trusted host context. This deliberately does not implement Deserialize.
#[derive(Clone)]
pub struct ArtifactAccess {
    pub namespace: String,
    pub author: ArtifactAuthor,
    pub origin: Option<ArtifactOrigin>,
    active: Option<std::sync::Arc<dyn Fn() -> bool + Send + Sync>>,
}

impl ArtifactAccess {
    pub fn local_human() -> Self {
        Self {
            namespace: SINGLE_OPERATOR_NAMESPACE.into(),
            author: ArtifactAuthor {
                kind: ArtifactAuthorKind::Human,
                id: "local-operator".into(),
            },
            origin: None,
            active: None,
        }
    }

    pub fn local_agent(agent: String, origin: ArtifactOrigin) -> Self {
        Self {
            namespace: SINGLE_OPERATOR_NAMESPACE.into(),
            author: ArtifactAuthor {
                kind: ArtifactAuthorKind::Agent,
                id: agent,
            },
            origin: Some(origin),
            active: None,
        }
    }

    /// A trusted host may bind cancellation/deadline to queued mutations.
    pub fn with_liveness(mut self, active: std::sync::Arc<dyn Fn() -> bool + Send + Sync>) -> Self {
        self.active = Some(active);
        self
    }

    pub fn is_active(&self) -> bool {
        self.active.as_ref().is_none_or(|active| active())
    }
}

impl std::fmt::Debug for ArtifactAccess {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArtifactAccess")
            .field("namespace", &self.namespace)
            .field("author", &self.author)
            .field("origin", &self.origin)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactCapabilities {
    pub schema: u8,
    pub store_id: String,
    pub scope: String,
    pub media_types: Vec<String>,
    pub max_content_bytes: usize,
    pub max_title_bytes: usize,
    pub max_page_limit: u8,
    pub max_artifacts: usize,
    pub max_receipts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub artifact_id: String,
    /// Local grouping only. It is not a backend project or an authorization.
    pub work_id: String,
    /// Attribution only; never an ownership or permission credential.
    pub conversation_session_id: Option<String>,
    pub title: String,
    pub media_type: String,
    pub head_revision: String,
    pub sequence: u64,
    pub content_sha256: String,
    pub content_bytes: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRevision {
    pub revision_id: String,
    pub artifact_id: String,
    pub parent_revision: Option<String>,
    pub sequence: u64,
    pub content: String,
    pub content_sha256: String,
    pub content_bytes: u64,
    pub created_at: i64,
    pub author: ArtifactAuthor,
    pub origin: Option<ArtifactOrigin>,
    pub restored_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRevisionSummary {
    pub revision_id: String,
    pub artifact_id: String,
    pub parent_revision: Option<String>,
    pub sequence: u64,
    pub content_sha256: String,
    pub content_bytes: u64,
    pub created_at: i64,
    pub author: ArtifactAuthor,
    pub origin: Option<ArtifactOrigin>,
    pub restored_from: Option<String>,
}

impl From<&ArtifactRevision> for ArtifactRevisionSummary {
    fn from(revision: &ArtifactRevision) -> Self {
        Self {
            revision_id: revision.revision_id.clone(),
            artifact_id: revision.artifact_id.clone(),
            parent_revision: revision.parent_revision.clone(),
            sequence: revision.sequence,
            content_sha256: revision.content_sha256.clone(),
            content_bytes: revision.content_bytes,
            created_at: revision.created_at,
            author: revision.author.clone(),
            origin: revision.origin.clone(),
            restored_from: revision.restored_from.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDocument {
    pub schema: u8,
    pub store_id: String,
    pub artifact: Artifact,
    pub revision: ArtifactRevision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactListQuery {
    pub store_id: String,
    pub work_id: String,
    pub after: Option<String>,
    pub limit: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactListPage {
    pub schema: u8,
    pub store_id: String,
    pub work_id: String,
    pub artifacts: Vec<Artifact>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactHistoryQuery {
    pub store_id: String,
    pub after: Option<u64>,
    pub limit: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactHistoryPage {
    pub schema: u8,
    pub store_id: String,
    pub artifact_id: String,
    pub revisions: Vec<ArtifactRevisionSummary>,
    pub next_cursor: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateArtifactRequest {
    pub store_id: String,
    pub operation_id: String,
    pub work_id: String,
    pub title: String,
    pub media_type: String,
    pub content: String,
    pub conversation_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitArtifactRequest {
    pub store_id: String,
    pub operation_id: String,
    pub expected_head: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreArtifactRequest {
    pub store_id: String,
    pub operation_id: String,
    pub expected_head: String,
    pub revision_id: String,
}

/// This immutable outcome is persisted atomically with a successful version or
/// a head conflict. Replaying it does not project a later artifact head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactReceipt {
    pub schema: u8,
    pub store_id: String,
    pub operation_id: String,
    pub request_sha256: String,
    pub outcome: String,
    pub changed: bool,
    pub artifact: Artifact,
    pub revision: Option<ArtifactRevision>,
    pub expected_head: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArtifactError {
    #[error("invalid artifact request: {0}")]
    Invalid(String),
    #[error("artifact store identity mismatch")]
    StoreMismatch,
    #[error("artifact resource not found")]
    NotFound,
    #[error("managed artifacts require the configured single-operator scope")]
    ScopeUnavailable,
    #[error("managed artifact capacity reached")]
    Capacity,
    #[error("operation identity was already bound to a different request")]
    OperationMismatch,
    #[error("artifact storage failure: {0}")]
    Storage(String),
}

#[async_trait]
pub trait ArtifactService: Send + Sync {
    async fn capabilities(
        &self,
        access: ArtifactAccess,
    ) -> Result<ArtifactCapabilities, ArtifactError>;
    async fn list(
        &self,
        access: ArtifactAccess,
        query: ArtifactListQuery,
    ) -> Result<ArtifactListPage, ArtifactError>;
    async fn read(
        &self,
        access: ArtifactAccess,
        store_id: String,
        artifact_id: String,
        revision_id: Option<String>,
    ) -> Result<ArtifactDocument, ArtifactError>;
    async fn history(
        &self,
        access: ArtifactAccess,
        artifact_id: String,
        query: ArtifactHistoryQuery,
    ) -> Result<ArtifactHistoryPage, ArtifactError>;
    async fn operation(
        &self,
        access: ArtifactAccess,
        store_id: String,
        operation_id: String,
    ) -> Result<ArtifactReceipt, ArtifactError>;
    async fn create(
        &self,
        access: ArtifactAccess,
        request: CreateArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError>;
    async fn commit(
        &self,
        access: ArtifactAccess,
        artifact_id: String,
        request: CommitArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError>;
    async fn restore(
        &self,
        access: ArtifactAccess,
        artifact_id: String,
        request: RestoreArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_identity_is_canonical_and_non_nil() {
        assert!(valid_artifact_uuid("dbe9b3f5-74aa-4c73-939c-3903b885bbcb"));
        for invalid in [
            "",
            "local-work-2",
            "dbe9b3f574aa4c73939c3903b885bbcb",
            "DBE9B3F5-74AA-4C73-939C-3903B885BBCB",
            "00000000-0000-0000-0000-000000000000",
        ] {
            assert!(!valid_artifact_uuid(invalid));
        }
    }

    #[test]
    fn content_preserves_unicode_and_line_endings_and_author_cannot_be_injected() {
        let value = serde_json::json!({"store_id":"s","operation_id":"o","work_id":"local-work-1","title":"稿","media_type":"text/plain","content":"首行😀\r\n二行\n","conversation_session_id":null});
        let request: CreateArtifactRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(request.content, "首行😀\r\n二行\n");
        let mut injected = value;
        injected["author"] = serde_json::json!({"kind":"human","id":"owner"});
        assert!(serde_json::from_value::<CreateArtifactRequest>(injected).is_err());
    }
}
