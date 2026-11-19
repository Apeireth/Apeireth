use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::system_clock;
use apeireth_core::user_domain::{DomainRegistry, DomainRole, DomainUserId};
use apeireth_governance::{UserRecord, UserTable};
use apeireth_runtime::canonical::{
    DomainSessionPlane, InMemorySessionStore, Runtime, SessionManager,
};
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use super::tests::{fixture, HELLO_HASH};
use super::*;
use crate::build_gateway_state_with_services;
use crate::canonical_entry::{canonical_router_with_state, GatewayTokenGate};
use crate::client_tokens::{ClientTokenConfig, ClientTokenSpec, ClientTokenTable, UserAccess};
use crate::panels::GatewayServices;

const TOKEN: &str = "turn-read-gateway-token";

mod history;

fn uri(session: SessionId) -> String {
    format!("/v1/sessions/{session}/turn-result?input_sha256={HELLO_HASH}")
}

fn target(session: SessionId, history: bool) -> String {
    if history {
        format!("/v1/sessions/{session}/turns")
    } else {
        uri(session)
    }
}

async fn state() -> GatewayState {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let mut state = build_gateway_state_with_services(runtime, GatewayServices::default());
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.into());
    state
}

async fn get(
    state: &GatewayState,
    method: &str,
    uri: &str,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = canonical_router_with_state(state.clone())
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn managed_single_operator_reads_with_the_enforced_env_token() {
    let mut state = state().await;
    state.user_access = crate::model_connection::managed_operator_access();
    let (session, request, trace) = fixture();
    state.runtime.sessions().save(&session).await.unwrap();
    for history in [false, true] {
        let (status, body) = get(&state, "GET", &target(session.id, history), Some(TOKEN)).await;
        assert_eq!(status, StatusCode::OK);
        if history {
            assert_eq!(body["turns"][0]["request"], request.to_string());
            assert_eq!(body["turns"][0]["trace_id"], trace.to_string());
        } else {
            assert_eq!(body["state"], "completed");
        }
    }
}

#[tokio::test]
async fn turn_results_route_is_read_only_bound_and_behind_the_token_gate() {
    let state = state().await;
    let (session, request, trace) = fixture();
    state.runtime.sessions().save(&session).await.unwrap();
    let before =
        serde_json::to_value(state.runtime.sessions().load(&session.id).await.unwrap()).unwrap();
    let target = uri(session.id);
    assert_eq!(
        get(&state, "GET", &target, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get(&state, "GET", &target, Some("not-the-configured-token"))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    for _ in 0..2 {
        let (status, body) = get(&state, "GET", &target, Some(TOKEN)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["state"], "completed");
        assert_eq!(body["request"], request.to_string());
        assert_eq!(body["trace_id"], trace.to_string());
        assert_eq!(body["text"], "真实结果 😀\n保留末尾空格 ");
        assert!(body["usage"].is_null());
    }
    let after =
        serde_json::to_value(state.runtime.sessions().load(&session.id).await.unwrap()).unwrap();
    assert_eq!(before, after, "queries do not change the durable record");
    assert_eq!(
        get(&state, "POST", &target, Some(TOKEN)).await.0,
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(
        get(
            &state,
            "GET",
            &format!("{target}&request={}", RequestId::new()),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        get(&state, "GET", &format!("{target}&request=bad"), Some(TOKEN))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get(
            &state,
            "GET",
            &format!("{target}&unexpected=1"),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get(
            &state,
            "GET",
            &format!("/v1/sessions/{}/turn-result", session.id),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get(&state, "GET", &uri(SessionId::new()), Some(TOKEN))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn turn_results_do_not_wait_for_the_turn_write_permit() {
    let state = state().await;
    let (session, _, _) = fixture();
    state.runtime.sessions().save(&session).await.unwrap();
    let _executing = state.runtime.sessions().write_permit(session.id).await;
    let response = tokio::time::timeout(
        Duration::from_secs(1),
        get(&state, "GET", &uri(session.id), Some(TOKEN)),
    )
    .await;
    assert_eq!(
        response
            .expect("read must not queue behind an executing turn")
            .0,
        StatusCode::OK
    );
}

fn bind_users(state: &mut GatewayState, users: &[(&str, &str)]) {
    state.client_tokens = ClientTokenTable::from_config(ClientTokenConfig::new(
        users
            .iter()
            .map(|(id, _)| ClientTokenSpec {
                name: (*id).into(),
                token: format!("{id}-turn-result-token"),
                user_id: Some((*id).into()),
            })
            .chain(std::iter::once(ClientTokenSpec {
                name: "unbound".into(),
                token: "unbound-turn-result-token".into(),
                user_id: None,
            }))
            .collect(),
    ))
    .unwrap();
    state.user_access = UserAccess::from_table(UserTable::new(
        users
            .iter()
            .map(|(id, role)| UserRecord {
                user_id: (*id).into(),
                role: (*role).into(),
                client_names: vec![(*id).into()],
                domain: None,
            })
            .collect(),
    ));
}

#[tokio::test]
async fn turn_results_refuse_cross_user_and_unbound_reads() {
    for history in [false, true] {
        let mut state = state().await;
        bind_users(
            &mut state,
            &[
                ("owner", "owner"),
                ("alice", "member"),
                ("bob", "member"),
                ("guest", "guest"),
            ],
        );
        let (session, _, _) = fixture();
        state.runtime.sessions().save(&session).await.unwrap();
        state.user_access.bind_session_user(session.id, "alice");
        assert_eq!(
            get(
                &state,
                "GET",
                &target(session.id, history),
                Some("alice-turn-result-token")
            )
            .await
            .0,
            StatusCode::OK
        );
        assert_eq!(
            get(
                &state,
                "GET",
                &target(session.id, history),
                Some("owner-turn-result-token")
            )
            .await
            .0,
            StatusCode::OK
        );
        let cross = get(
            &state,
            "GET",
            &target(session.id, history),
            Some("bob-turn-result-token"),
        )
        .await;
        let absent = get(
            &state,
            "GET",
            &target(SessionId::new(), history),
            Some("bob-turn-result-token"),
        )
        .await;
        assert_eq!(cross.0, StatusCode::NOT_FOUND);
        assert_eq!(
            cross, absent,
            "cross-user and missing resources share the same frame"
        );
        assert_eq!(
            get(
                &state,
                "GET",
                &target(session.id, history),
                Some("unbound-turn-result-token")
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            get(&state, "GET", &target(session.id, history), Some(TOKEN))
                .await
                .0,
            StatusCode::FORBIDDEN,
            "unbound env credential does not bypass an active user table"
        );
        let (guest, _, _) = fixture();
        state.runtime.sessions().save(&guest).await.unwrap();
        state.user_access.bind_session_user(guest.id, "guest");
        assert_eq!(
            get(
                &state,
                "GET",
                &target(guest.id, history),
                Some("guest-turn-result-token")
            )
            .await
            .0,
            StatusCode::OK
        );
        let (unowned, _, _) = fixture();
        state.runtime.sessions().save(&unowned).await.unwrap();
        assert_eq!(
            get(
                &state,
                "GET",
                &target(unowned.id, history),
                Some("alice-turn-result-token")
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
async fn turn_results_owners_remain_within_their_domain() {
    for history in [false, true] {
        let root =
            std::env::temp_dir().join(format!("apeireth-turn-domain-{}", uuid::Uuid::new_v4()));
        let registry = DomainRegistry::open(&root).unwrap();
        registry
            .create("alice", "Alice", DomainRole::Owner)
            .unwrap();
        registry.create("bob", "Bob", DomainRole::Owner).unwrap();
        let mut state = state().await;
        bind_users(&mut state, &[("alice", "owner"), ("bob", "owner")]);
        let alice = Arc::new(SessionManager::new(
            Arc::new(InMemorySessionStore::new()),
            system_clock(),
        ));
        let bob = Arc::new(SessionManager::new(
            Arc::new(InMemorySessionStore::new()),
            system_clock(),
        ));
        let (session, _, _) = fixture();
        bob.save(&session).await.unwrap();
        let mut managers = BTreeMap::new();
        managers.insert(DomainUserId::new("alice").unwrap(), alice);
        managers.insert(DomainUserId::new("bob").unwrap(), bob);
        managers.insert(
            DomainUserId::default_domain(),
            state.runtime.sessions_arc().clone(),
        );
        state.domains = Some(Arc::new(DomainSessionPlane::new(registry, managers)));
        assert_eq!(
            get(
                &state,
                "GET",
                &target(session.id, history),
                Some("bob-turn-result-token")
            )
            .await
            .0,
            StatusCode::OK
        );
        let cross = get(
            &state,
            "GET",
            &target(session.id, history),
            Some("alice-turn-result-token"),
        )
        .await;
        let missing = get(
            &state,
            "GET",
            &target(SessionId::new(), history),
            Some("alice-turn-result-token"),
        )
        .await;
        assert_eq!(cross.0, StatusCode::NOT_FOUND);
        assert_eq!(
            cross, missing,
            "owner does not gain cross-domain visibility"
        );
        drop(state);
        fs_err::remove_dir_all(root).unwrap();
    }
}
