//! HTTP contract tests use a recording port. Durable CAS and replay are tested
//! by the shared service's own store tests and the external real-gateway probe.

use std::sync::Mutex;

use apeireth_governance::{UserRecord, UserTable};
use apeireth_protocol::artifacts::{
    Artifact, ArtifactCapabilities, ArtifactDocument, ArtifactHistoryPage, ArtifactListPage,
    ArtifactRevision, ArtifactRevisionSummary, ARTIFACT_SCHEMA, MAX_ARTIFACTS,
    MAX_ARTIFACT_RECEIPTS, SINGLE_OPERATOR_NAMESPACE,
};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tower::ServiceExt;

use super::*;
use crate::canonical_entry::{build_gateway_state_with_services, canonical_router_with_state};
use crate::client_tokens::UserAccess;
use crate::panels::GatewayServices;
use crate::GatewayTokenGate;

const TOKEN: &str = "local-c2-http-contract-placeholder";
const STORE: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86001";
const ARTIFACT: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86002";
const REVISION: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86003";
const OLD_REVISION: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86004";
const OPERATION: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86005";
const CONFLICT_OPERATION: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86006";
const MISMATCH_OPERATION: &str = "5f7595d5-a208-4124-bcb7-c2d4eae86007";
const OLD_CONTENT: &str = "# 原稿😀\r\n第一段。\n末行\r\n";
const CONTENT: &str = "# 人工修改稿😀\n下一段。\n";

fn digest(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

fn access_value(access: &ArtifactAccess) -> Value {
    json!({"namespace":access.namespace,"author":access.author,"origin":access.origin})
}

fn artifact() -> Artifact {
    Artifact {
        artifact_id: ARTIFACT.into(),
        work_id: "local-work-2".into(),
        conversation_session_id: None,
        title: "项目方案".into(),
        media_type: "text/markdown".into(),
        head_revision: REVISION.into(),
        sequence: 2,
        content_sha256: digest(CONTENT),
        content_bytes: CONTENT.len() as u64,
        created_at: 100,
        updated_at: 200,
    }
}

fn revision(old: bool) -> ArtifactRevision {
    let content = if old { OLD_CONTENT } else { CONTENT };
    ArtifactRevision {
        revision_id: if old { OLD_REVISION } else { REVISION }.into(),
        artifact_id: ARTIFACT.into(),
        parent_revision: (!old).then(|| OLD_REVISION.into()),
        sequence: if old { 1 } else { 2 },
        content: content.into(),
        content_sha256: digest(content),
        content_bytes: content.len() as u64,
        created_at: if old { 100 } else { 200 },
        author: ArtifactAccess::local_human().author,
        origin: None,
        restored_from: None,
    }
}

fn response_receipt(operation: &str, conflict: bool) -> ArtifactReceipt {
    ArtifactReceipt {
        schema: ARTIFACT_SCHEMA,
        store_id: STORE.into(),
        operation_id: operation.into(),
        request_sha256: "ab".repeat(32),
        outcome: if conflict { "conflict" } else { "committed" }.into(),
        changed: !conflict,
        artifact: artifact(),
        revision: (!conflict).then(|| revision(false)),
        expected_head: conflict.then(|| OLD_REVISION.into()),
    }
}

#[derive(Default)]
struct RecordingService {
    calls: Mutex<Vec<Value>>,
    failure: Mutex<Option<ArtifactError>>,
}

impl RecordingService {
    fn record<T: serde::Serialize>(
        &self,
        method: &str,
        access: ArtifactAccess,
        input: T,
    ) -> Result<(), ArtifactError> {
        self.calls
            .lock()
            .unwrap()
            .push(json!({"method":method,"access":access_value(&access),"input":input}));
        if let Some(error) = self.failure.lock().unwrap().clone() {
            return Err(error);
        }
        Ok(())
    }

    fn store(&self, store: &str) -> Result<(), ArtifactError> {
        if store == STORE {
            Ok(())
        } else {
            Err(ArtifactError::StoreMismatch)
        }
    }

    fn calls(&self) -> Vec<Value> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl ArtifactService for RecordingService {
    async fn capabilities(
        &self,
        access: ArtifactAccess,
    ) -> Result<ArtifactCapabilities, ArtifactError> {
        self.record("capabilities", access, ())?;
        Ok(ArtifactCapabilities {
            schema: ARTIFACT_SCHEMA,
            store_id: STORE.into(),
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
        self.store(&query.store_id)?;
        self.record("list", access, &query)?;
        Ok(ArtifactListPage {
            schema: ARTIFACT_SCHEMA,
            store_id: STORE.into(),
            work_id: query.work_id,
            artifacts: vec![artifact()],
            next_cursor: None,
        })
    }

    async fn read(
        &self,
        access: ArtifactAccess,
        store_id: String,
        artifact_id: String,
        revision_id: Option<String>,
    ) -> Result<ArtifactDocument, ArtifactError> {
        self.store(&store_id)?;
        self.record(
            "read",
            access,
            json!({"store_id":store_id,"artifact_id":artifact_id,"revision_id":revision_id}),
        )?;
        if artifact_id != ARTIFACT {
            return Err(ArtifactError::NotFound);
        }
        Ok(ArtifactDocument {
            schema: ARTIFACT_SCHEMA,
            store_id: STORE.into(),
            artifact: artifact(),
            revision: revision(revision_id.as_deref() == Some(OLD_REVISION)),
        })
    }

    async fn history(
        &self,
        access: ArtifactAccess,
        artifact_id: String,
        query: ArtifactHistoryQuery,
    ) -> Result<ArtifactHistoryPage, ArtifactError> {
        self.store(&query.store_id)?;
        self.record(
            "history",
            access,
            json!({"artifact_id":artifact_id,"query":query}),
        )?;
        Ok(ArtifactHistoryPage {
            schema: ARTIFACT_SCHEMA,
            store_id: STORE.into(),
            artifact_id,
            revisions: vec![ArtifactRevisionSummary::from(&revision(true))],
            next_cursor: Some(1),
        })
    }

    async fn operation(
        &self,
        access: ArtifactAccess,
        store_id: String,
        operation_id: String,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.store(&store_id)?;
        self.record("operation", access, &operation_id)?;
        Ok(response_receipt(
            &operation_id,
            operation_id == CONFLICT_OPERATION,
        ))
    }

    async fn create(
        &self,
        access: ArtifactAccess,
        request: CreateArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.store(&request.store_id)?;
        self.record("create", access, &request)?;
        Ok(response_receipt(&request.operation_id, false))
    }

    async fn commit(
        &self,
        access: ArtifactAccess,
        artifact_id: String,
        request: CommitArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.store(&request.store_id)?;
        self.record(
            "commit",
            access,
            json!({"artifact_id":artifact_id,"request":request}),
        )?;
        if request.operation_id == MISMATCH_OPERATION {
            return Err(ArtifactError::OperationMismatch);
        }
        Ok(response_receipt(
            &request.operation_id,
            request.expected_head == OLD_REVISION,
        ))
    }

    async fn restore(
        &self,
        access: ArtifactAccess,
        artifact_id: String,
        request: RestoreArtifactRequest,
    ) -> Result<ArtifactReceipt, ArtifactError> {
        self.store(&request.store_id)?;
        self.record(
            "restore",
            access,
            json!({"artifact_id":artifact_id,"request":request}),
        )?;
        Ok(response_receipt(&request.operation_id, false))
    }
}

async fn state(service: Option<Arc<RecordingService>>) -> GatewayState {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let services = GatewayServices {
        artifacts: service.map(|service| service as Arc<dyn ArtifactService>),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(runtime, services);
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.into());
    state
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    bytes: Vec<u8>,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.bytes).unwrap()
    }

    fn protected(&self) {
        assert_eq!(self.headers[header::CACHE_CONTROL], "no-store");
        assert_eq!(self.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    }
}

async fn send(
    app: &Router,
    method: &str,
    path: &str,
    body: &str,
    token: Option<&str>,
    headers: &[(&str, &str)],
) -> Reply {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    Reply {
        status,
        headers,
        bytes,
    }
}

fn create_request() -> Value {
    json!({"store_id":STORE,"operation_id":OPERATION,"work_id":"local-work-2","title":"方案😀","media_type":"text/markdown","content":CONTENT,"conversation_session_id":null})
}

fn commit_request(operation: &str, expected: &str) -> Value {
    json!({"store_id":STORE,"operation_id":operation,"expected_head":expected,"content":CONTENT})
}

#[tokio::test]
async fn all_artifact_routes_stay_inside_auth_and_sensitive_early_refusals_are_not_cacheable() {
    let service = Arc::new(RecordingService::default());
    let app = canonical_router_with_state(state(Some(service.clone())).await);
    let paths = [
        ("GET", "/v1/artifacts/capabilities".into()),
        (
            "GET",
            format!("/v1/artifacts?store_id={STORE}&work_id=local-work-2"),
        ),
        ("POST", "/v1/artifacts".into()),
        ("GET", format!("/v1/artifacts/{ARTIFACT}?store_id={STORE}")),
        (
            "GET",
            format!("/v1/artifacts/{ARTIFACT}/revisions?store_id={STORE}"),
        ),
        ("POST", format!("/v1/artifacts/{ARTIFACT}/revisions")),
        ("POST", format!("/v1/artifacts/{ARTIFACT}/restore")),
        (
            "GET",
            format!("/v1/artifacts/{ARTIFACT}/revisions/{OLD_REVISION}/export?store_id={STORE}"),
        ),
        (
            "GET",
            format!("/v1/artifacts/operations/{OPERATION}?store_id={STORE}"),
        ),
    ];
    for (method, path) in paths {
        for token in [None, Some("wrong-local-placeholder")] {
            let response = send(&app, method, &path, "{}", token, &[]).await;
            assert_eq!(response.status, StatusCode::UNAUTHORIZED, "{method} {path}");
            response.protected();
            assert!(response.json()["error"]["code"]
                .as_str()
                .unwrap()
                .starts_with("auth_"));
        }
    }
    assert!(service.calls().is_empty());
}

#[tokio::test]
async fn no_configured_service_is_explicitly_unsupported_not_an_empty_artifact_list() {
    let app = canonical_router_with_state(state(None).await);
    let response = send(
        &app,
        "GET",
        "/v1/artifacts/capabilities",
        "",
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(response.status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(
        response.json()["error"]["details"]["artifact_error"],
        "unsupported"
    );
    response.protected();
}

#[tokio::test]
async fn human_mutations_use_the_shared_port_with_trusted_actor_and_exact_captured_body() {
    let service = Arc::new(RecordingService::default());
    let app = canonical_router_with_state(state(Some(service.clone())).await);
    let create = create_request();
    let created = send(
        &app,
        "POST",
        "/v1/artifacts",
        &create.to_string(),
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(created.status, StatusCode::OK);
    created.protected();
    assert_eq!(
        created.json(),
        serde_json::to_value(response_receipt(OPERATION, false)).unwrap()
    );
    let committed = send(
        &app,
        "POST",
        &format!("/v1/artifacts/{ARTIFACT}/revisions"),
        &commit_request(OPERATION, REVISION).to_string(),
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(committed.status, StatusCode::OK);
    let restore = json!({"store_id":STORE,"operation_id":OPERATION,"expected_head":REVISION,"revision_id":OLD_REVISION});
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/v1/artifacts/{ARTIFACT}/restore"),
            &restore.to_string(),
            Some(TOKEN),
            &[]
        )
        .await
        .status,
        StatusCode::OK
    );
    let calls = service.calls();
    assert_eq!(calls[0]["input"], create);
    assert_eq!(calls[1]["input"]["artifact_id"], ARTIFACT);
    assert_eq!(
        calls[1]["input"]["request"],
        commit_request(OPERATION, REVISION)
    );
    assert_eq!(calls[2]["input"]["request"], restore);
    for call in calls {
        assert_eq!(call["access"], access_value(&ArtifactAccess::local_human()));
    }
}

#[tokio::test]
async fn strict_queries_bodies_paths_and_bounds_do_not_reach_the_service() {
    let service = Arc::new(RecordingService::default());
    let app = canonical_router_with_state(state(Some(service.clone())).await);
    let cases = [
        format!("/v1/artifacts/{ARTIFACT}"),
        format!("/v1/artifacts/{ARTIFACT}?store_id={STORE}&path=C%3A%2Fprivate"),
        format!("/v1/artifacts/{ARTIFACT}?store_id={STORE}&store_id={STORE}"),
        format!("/v1/artifacts/not-a-uuid?store_id={STORE}"),
        format!("/v1/artifacts/{ARTIFACT}?store_id={}", STORE.to_uppercase()),
        format!("/v1/artifacts?store_id={STORE}&work_id=local-work-2&limit=0"),
        format!("/v1/artifacts?store_id={STORE}&work_id=local-work-2&limit=33"),
        format!("/v1/artifacts?store_id={STORE}&work_id=local-work-2&after=bad"),
        format!("/v1/artifacts/{ARTIFACT}/revisions?store_id={STORE}&after=-1"),
        format!(
            "/v1/artifacts/{ARTIFACT}/revisions/{OLD_REVISION}/export?store_id={STORE}&format=docx"
        ),
        "/v1/artifacts/capabilities?owner=other".into(),
    ];
    for route in cases {
        let response = send(&app, "GET", &route, "", Some(TOKEN), &[]).await;
        assert_eq!(response.status, StatusCode::BAD_REQUEST, "{route}");
        response.protected();
    }
    for key in ["owner", "author", "namespace", "actor", "path"] {
        let mut value = create_request();
        value[key] = json!("forged");
        let response = send(
            &app,
            "POST",
            "/v1/artifacts",
            &value.to_string(),
            Some(TOKEN),
            &[],
        )
        .await;
        assert_eq!(response.status, StatusCode::BAD_REQUEST, "{key}");
    }
    for value in [
        json!({"store_id":STORE,"operation_id":"bad","expected_head":REVISION,"content":"稿"}),
        json!({"store_id":STORE,"operation_id":OPERATION,"expected_head":"00000000-0000-0000-0000-000000000000","content":"稿"}),
        json!({"store_id":STORE,"operation_id":OPERATION,"expected_head":REVISION,"content":"x".repeat(MAX_ARTIFACT_CONTENT_BYTES+1)}),
    ] {
        let response = send(
            &app,
            "POST",
            &format!("/v1/artifacts/{ARTIFACT}/revisions"),
            &value.to_string(),
            Some(TOKEN),
            &[],
        )
        .await;
        assert_eq!(response.status, StatusCode::BAD_REQUEST);
    }
    assert!(service.calls().is_empty());
}

#[tokio::test]
async fn escaped_json_transport_bound_is_distinct_from_utf8_content_bound() {
    let service = Arc::new(RecordingService::default());
    let app = canonical_router_with_state(state(Some(service.clone())).await);
    let mut request = create_request();
    request["content"] = json!("\u{0001}".repeat(MAX_ARTIFACT_CONTENT_BYTES));
    let encoded = request.to_string();
    assert!(encoded.len() > 6 * MAX_ARTIFACT_CONTENT_BYTES);
    assert!(encoded.len() < JSON_BODY_LIMIT);
    let response = send(&app, "POST", "/v1/artifacts", &encoded, Some(TOKEN), &[]).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(service.calls()[0]["input"]["content"], request["content"]);
    let oversized = " ".repeat(JSON_BODY_LIMIT + 1);
    let response = send(&app, "POST", "/v1/artifacts", &oversized, Some(TOKEN), &[]).await;
    assert_eq!(response.status, StatusCode::PAYLOAD_TOO_LARGE);
    response.protected();
    assert_eq!(service.calls().len(), 1);
}

#[tokio::test]
async fn conflict_is_a_durable_receipt_and_lookup_returns_it_without_a_second_mutation() {
    let service = Arc::new(RecordingService::default());
    let app = canonical_router_with_state(state(Some(service.clone())).await);
    let failed = send(
        &app,
        "POST",
        &format!("/v1/artifacts/{ARTIFACT}/revisions"),
        &commit_request(CONFLICT_OPERATION, OLD_REVISION).to_string(),
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(failed.status, StatusCode::CONFLICT);
    let receipt = failed.json()["error"]["details"]["receipt"].clone();
    assert_eq!(
        receipt,
        serde_json::to_value(response_receipt(CONFLICT_OPERATION, true)).unwrap()
    );
    let lookup = send(
        &app,
        "GET",
        &format!("/v1/artifacts/operations/{CONFLICT_OPERATION}?store_id={STORE}"),
        "",
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(lookup.status, StatusCode::OK);
    assert_eq!(lookup.json(), receipt);
    assert_eq!(
        service
            .calls()
            .iter()
            .map(|v| v["method"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["commit", "operation"]
    );
}

#[tokio::test]
async fn operation_payload_conflicts_and_wrong_store_are_not_success_receipts() {
    let service = Arc::new(RecordingService::default());
    let app = canonical_router_with_state(state(Some(service.clone())).await);
    let mismatch = send(
        &app,
        "POST",
        &format!("/v1/artifacts/{ARTIFACT}/revisions"),
        &commit_request(MISMATCH_OPERATION, REVISION).to_string(),
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(mismatch.status, StatusCode::CONFLICT);
    assert_eq!(
        mismatch.json()["error"]["details"]["artifact_error"],
        "operation_mismatch"
    );
    let wrong = send(
        &app,
        "GET",
        &format!("/v1/artifacts/{ARTIFACT}?store_id={OPERATION}"),
        "",
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(wrong.status, StatusCode::CONFLICT);
    assert_eq!(
        wrong.json()["error"]["details"]["artifact_error"],
        "store_mismatch"
    );
    let missing = send(
        &app,
        "GET",
        &format!("/v1/artifacts/{OPERATION}?store_id={STORE}"),
        "",
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn read_history_export_and_operation_are_exact_version_read_only_calls() {
    let service = Arc::new(RecordingService::default());
    let app = canonical_router_with_state(state(Some(service.clone())).await);
    let cap = send(
        &app,
        "GET",
        "/v1/artifacts/capabilities",
        "",
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(cap.json()["store_id"], STORE);
    assert_eq!(cap.json()["scope"], SINGLE_OPERATOR_NAMESPACE);
    let list = send(
        &app,
        "GET",
        &format!("/v1/artifacts?store_id={STORE}&work_id=local-work-2&limit=32"),
        "",
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(list.status, StatusCode::OK);
    let read = send(
        &app,
        "GET",
        &format!("/v1/artifacts/{ARTIFACT}/revisions/{OLD_REVISION}?store_id={STORE}"),
        "",
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(read.json()["revision"]["revision_id"], OLD_REVISION);
    assert_eq!(read.json()["revision"]["content"], OLD_CONTENT);
    assert_eq!(read.json()["artifact"]["head_revision"], REVISION);
    let history = send(
        &app,
        "GET",
        &format!("/v1/artifacts/{ARTIFACT}/revisions?store_id={STORE}&after=0&limit=1"),
        "",
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(history.json()["next_cursor"], 1);
    assert!(history.json()["revisions"][0].get("content").is_none());
    let export = send(
        &app,
        "GET",
        &format!("/v1/artifacts/{ARTIFACT}/revisions/{OLD_REVISION}/export?store_id={STORE}"),
        "",
        Some(TOKEN),
        &[],
    )
    .await;
    assert_eq!(export.status, StatusCode::OK);
    assert_eq!(export.bytes, OLD_CONTENT.as_bytes());
    assert_eq!(
        export.headers[header::CONTENT_TYPE],
        "text/markdown; charset=utf-8"
    );
    assert_eq!(
        export.headers[header::ETAG].to_str().unwrap(),
        format!("\"{}\"", digest(OLD_CONTENT))
    );
    assert!(export.headers[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .contains(ARTIFACT));
    export.protected();
    assert!(service.calls().iter().all(|v| matches!(
        v["method"].as_str(),
        Some("capabilities" | "list" | "read" | "history")
    )));
    let before = service.calls().len();
    for (name, value) in [("range", "bytes=0-10"), ("if-range", "fake-etag")] {
        assert_eq!(
            send(
                &app,
                "GET",
                &format!(
                    "/v1/artifacts/{ARTIFACT}/revisions/{OLD_REVISION}/export?store_id={STORE}"
                ),
                "",
                Some(TOKEN),
                &[(name, value)]
            )
            .await
            .status,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(service.calls().len(), before);
}

#[tokio::test]
async fn unsupported_user_scope_and_forged_domain_never_reach_the_local_store() {
    let service = Arc::new(RecordingService::default());
    let mut configured = state(Some(service.clone())).await;
    configured.user_access = UserAccess::from_table(UserTable::new(vec![UserRecord {
        user_id: "owner-one".into(),
        role: "owner".into(),
        client_names: vec![GatewayTokenGate::ENV_FACE_CLIENT_NAME.into()],
        domain: None,
    }]));
    let app = canonical_router_with_state(configured);
    for (method, path, body) in [
        (
            "GET",
            "/v1/artifacts/capabilities".to_owned(),
            "".to_owned(),
        ),
        (
            "POST",
            "/v1/artifacts".to_owned(),
            create_request().to_string(),
        ),
    ] {
        let response = send(&app, method, &path, &body, Some(TOKEN), &[]).await;
        assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
        response.protected();
        assert_eq!(
            response.json()["error"]["details"]["artifact_error"],
            "scope_unavailable"
        );
    }
    assert!(service.calls().is_empty());
    let local = canonical_router_with_state(state(Some(service.clone())).await);
    let mut frames = Vec::new();
    for claimed in ["foreign-existing", "foreign-nonexistent"] {
        let response = send(
            &local,
            "GET",
            "/v1/artifacts/capabilities",
            "",
            Some(TOKEN),
            &[("x-apeireth-domain", claimed)],
        )
        .await;
        assert_eq!(response.status, StatusCode::FORBIDDEN);
        response.protected();
        frames.push(response.json());
    }
    assert_eq!(frames[0], frames[1]);
    assert!(service.calls().is_empty());
}

#[tokio::test]
async fn service_error_frames_do_not_disclose_storage_paths_or_raw_invalid_content() {
    let service = Arc::new(RecordingService::default());
    let app = canonical_router_with_state(state(Some(service.clone())).await);
    for (failure, status, kind) in [
        (
            ArtifactError::Invalid("private prompt bytes".into()),
            StatusCode::BAD_REQUEST,
            "invalid_request",
        ),
        (
            ArtifactError::ScopeUnavailable,
            StatusCode::SERVICE_UNAVAILABLE,
            "scope_unavailable",
        ),
        (
            ArtifactError::Capacity,
            StatusCode::SERVICE_UNAVAILABLE,
            "capacity",
        ),
        (
            ArtifactError::Storage("C:\\private\\database.sqlite3".into()),
            StatusCode::INTERNAL_SERVER_ERROR,
            "storage",
        ),
    ] {
        *service.failure.lock().unwrap() = Some(failure);
        let response = send(
            &app,
            "GET",
            "/v1/artifacts/capabilities",
            "",
            Some(TOKEN),
            &[],
        )
        .await;
        assert_eq!(response.status, status);
        response.protected();
        assert_eq!(response.json()["error"]["details"]["artifact_error"], kind);
        assert!(!String::from_utf8_lossy(&response.bytes).contains("private"));
    }
}
