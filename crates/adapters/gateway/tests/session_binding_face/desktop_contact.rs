//! HTTP and durable-state coverage for desktop-owned contact sessions.

use super::*;
use apeireth_gateway::{
    ContactArchiveCommand, ContactCapabilitiesDto, ContactCreateCommand, ContactDto,
    ContactMemoryScopeDto, ContactPanel, ContactPanelError, ContactStatusDto, ContactUpdateCommand,
    ContactWorkspaceBindingDto, GatewayTokenGate,
};
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use axum::{http::HeaderValue, Router};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

const TOKEN: &str = "desktop-contact-test-token";
const CONTEXT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PERSONA_A: &str = "Identify yourself as the contact-session test persona.";

struct FixedContactPanel(Vec<ContactDto>);

#[async_trait]
impl ContactPanel for FixedContactPanel {
    async fn list_contacts(&self, _: bool) -> Result<Vec<ContactDto>, String> {
        Ok(self.0.clone())
    }

    async fn get_contact(&self, contact_id: &str) -> Result<ContactDto, ContactPanelError> {
        self.0
            .iter()
            .find(|row| row.contact_id == contact_id)
            .cloned()
            .ok_or_else(|| ContactPanelError::NotFound {
                contact_id: contact_id.into(),
            })
    }

    async fn create_contact(
        &self,
        _: ContactCreateCommand,
    ) -> Result<ContactDto, ContactPanelError> {
        panic!("opening a session must not create a contact")
    }

    async fn update_contact(
        &self,
        _: ContactUpdateCommand,
    ) -> Result<ContactDto, ContactPanelError> {
        panic!("opening a session must not update a contact")
    }

    async fn archive_contact(
        &self,
        _: ContactArchiveCommand,
    ) -> Result<ContactDto, ContactPanelError> {
        panic!("opening a session must not archive a contact")
    }
}

fn contact_row(contact: &Contact, workspace: &Workspace) -> ContactDto {
    ContactDto {
        contact_id: contact.contact_id().to_string(),
        agent_id: contact.agent_id().to_string(),
        display_name: contact.display_name().into(),
        subtitle: contact.subtitle().map(str::to_owned),
        avatar_ref: contact.avatar_ref().map(str::to_owned),
        status: ContactStatusDto::Ready,
        persona_revision: contact.persona_revision(),
        memory_scope: ContactMemoryScopeDto::Session,
        workspace_binding: Some(ContactWorkspaceBindingDto {
            workspace_id: workspace.workspace_id.to_string(),
            revision: workspace.revision,
        }),
        capabilities: ContactCapabilitiesDto {
            read: true,
            write: false,
            approval_required: true,
        },
        persona: contact.persona().map(|text| text.as_str().to_owned()),
        revision: contact.revision(),
        updated_at_ms: NOW_MS,
    }
}

async fn desktop() -> (Harness, Router, Contact, Contact, Workspace) {
    let harness = harness().await;
    let workspace = create_workspace(&harness.workspaces, "contact-fixture");
    let a = create_contact(&harness.contacts, "Contact A", Some(PERSONA_A));
    let b = create_contact(
        &harness.contacts,
        "Contact B",
        Some("Second contact persona."),
    );
    let services = GatewayServices {
        contacts: Some(Arc::new(FixedContactPanel(vec![
            contact_row(&a, &workspace),
            contact_row(&b, &workspace),
        ]))),
        ..GatewayServices::default()
    };
    let mut state = build_gateway_state_with_services(harness.runtime.clone(), services);
    state.token_gate = GatewayTokenGate::Enforced(TOKEN.into());
    (harness, canonical_router_with_state(state), a, b, workspace)
}

async fn send(router: &Router, mut request: Request<Body>) -> (StatusCode, String) {
    request
        .headers_mut()
        .insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    status_and_body(
        router
            .clone()
            .oneshot(request)
            .await
            .expect("HTTP response"),
    )
    .await
}

fn open(
    session: SessionId,
    contact: &Contact,
    incarnation: &str,
    require_existing: bool,
) -> Request<Body> {
    post(
        &format!("/v1/sessions/{session}/contact-session"),
        json!({
            "contact_id":contact.contact_id().to_string(), "context_key":CONTEXT,
            "incarnation":incarnation, "require_existing": require_existing,
        }),
    )
}

fn chat(session: SessionId, contact: &Contact, input: &str) -> Request<Body> {
    post(
        "/v1/chat",
        json!({"session":session.to_string(), "expected_contact_id": contact.contact_id().to_string(), "input":input}),
    )
}

async fn saved(harness: &Harness, session: SessionId) -> Session {
    harness
        .store
        .load(&session)
        .await
        .expect("durable read")
        .expect("saved session")
}

fn session_json(session: &Session) -> Value {
    serde_json::to_value(session).expect("session serialization")
}

#[tokio::test]
async fn desktop_contact_open_persists_once_and_rechecks_without_rebinding() {
    let (harness, router, a, b, workspace) = desktop().await;
    let session = session_id(9201);
    let (status, body) = send(&router, open(session, &a, "first-host", false)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap(),
        json!({
            "schema":1, "session_id":session.to_string(), "contact_id":a.contact_id().to_string(),
            "context_key":CONTEXT, "incarnation":"first-host"
        })
    );
    let initial = saved(&harness, session).await;
    assert!(initial.messages.is_empty());
    let binding = initial
        .agent_binding()
        .expect("binding persisted before the first message");
    assert_eq!(binding.contact_id(), a.contact_id());
    assert_eq!(binding.agent_id(), a.agent_id());
    assert_eq!(binding.persona_revision(), a.persona_revision());
    assert_eq!(binding.workspace_id(), workspace.workspace_id);
    assert_eq!(binding.workspace_revision(), workspace.revision);
    assert_eq!(binding.context_key(), Some(CONTEXT));
    assert_eq!(binding.incarnation(), Some("first-host"));

    let (status, body) = send(&router, open(session, &a, "second-host", true)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["incarnation"],
        "second-host"
    );
    assert_eq!(
        session_json(&saved(&harness, session).await),
        session_json(&initial)
    );

    let (status, body) = send(&router, open(session, &b, "second-host", false)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(error_code(&body), "session_binding_conflict");
    assert_eq!(
        session_json(&saved(&harness, session).await),
        session_json(&initial)
    );
    assert_eq!(harness.provider.call_count(), 0);
}

#[tokio::test]
async fn desktop_contact_recheck_requires_an_existing_bound_session() {
    let (harness, router, a, _, _) = desktop().await;
    let session = session_id(9202);
    let (status, body) = send(&router, open(session, &a, "new-host", true)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(error_code(&body), "session_binding_conflict");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["error"]["details"]["reason"],
        "contact_session_missing"
    );
    assert!(harness.store.load(&session).await.unwrap().is_none());

    let existing = Session::new(session, harness.clock.as_ref());
    harness
        .store
        .save(&existing)
        .await
        .expect("save an unrelated unbound session");
    let (status, body) = send(&router, open(session, &a, "new-host", false)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(error_code(&body), "session_binding_conflict");
    assert_eq!(
        session_json(&saved(&harness, session).await),
        session_json(&existing)
    );
}

#[tokio::test]
async fn desktop_native_chat_checks_contact_before_writing_and_runs_first_text_turn_without_tools()
{
    exercise_contact_chat(false).await;
}

#[tokio::test]
async fn desktop_streamed_chat_keeps_contact_persona_tool_refusal_and_operation_scope() {
    exercise_contact_chat(true).await;
}

async fn send_chat(
    router: &Router,
    mut request: Request<Body>,
    streaming: bool,
) -> (StatusCode, String) {
    if !streaming {
        return send(router, request).await;
    }
    request.headers_mut().insert(
        "accept",
        HeaderValue::from_static("application/x-apeireth-turn+ndjson"),
    );
    let (status, body) = send(router, request).await;
    assert_eq!(status, StatusCode::OK);
    let terminal: Value = serde_json::from_str(body.lines().last().unwrap()).unwrap();
    assert_eq!(terminal["kind"], "final");
    let canonical_status =
        StatusCode::from_u16(terminal["status"].as_u64().unwrap() as u16).unwrap();
    (canonical_status, terminal["body"].to_string())
}

async fn exercise_contact_chat(streaming: bool) {
    let (harness, router, a, b, _) = desktop().await;
    let session = session_id(if streaming { 9303 } else { 9203 });
    let (status, body) = send(&router, open(session, &a, "first-host", false)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = saved(&harness, session).await;

    let (status, body) = send_chat(
        &router,
        chat(session, &b, "must not reach provider"),
        streaming,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(error_code(&body), "agent_binding_rejected");
    assert_eq!(harness.provider.call_count(), 0);
    assert_eq!(
        session_json(&saved(&harness, session).await),
        session_json(&before)
    );

    // The common fixture asks to call the calculator on its first provider
    // call even if no tools were advertised. The runtime must refuse it.
    let (status, body) = send_chat(
        &router,
        chat(session, &a, "test the tool refusal"),
        streaming,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(error_code(&body), "invalid_request");
    assert_eq!(harness.provider.call_count(), 1);
    assert_eq!(harness.tool_calls.load(Ordering::SeqCst), 0);

    let empty = session_id(if streaming { 9304 } else { 9204 });
    let (status, body) = send(&router, open(empty, &a, "first-host", false)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(saved(&harness, empty).await.messages.is_empty());
    let receipt = |contact| {
        let mut request = chat(empty, contact, "first text turn");
        request.headers_mut().insert(
            "x-apeireth-operation",
            HeaderValue::from_static("desktop-contact-1"),
        );
        request
    };
    let (status, body) = send_chat(&router, receipt(&a), streaming).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let outcome: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(outcome["session"], empty.to_string());
    assert_eq!(outcome["text"], common::FINAL_TEXT);
    let completed = saved(&harness, empty).await;
    assert_eq!(
        completed.agent_binding().unwrap().contact_id(),
        a.contact_id()
    );
    let system_texts: Vec<_> = completed
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::System)
        .map(|message| ContentPart::join_text(&message.content))
        .collect();
    assert!(
        system_texts.iter().any(|system| system.contains(PERSONA_A)),
        "{system_texts:?}"
    );

    let (status, body) = send_chat(&router, receipt(&a), streaming).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let replay: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(replay["request"], outcome["request"]);

    let (status, body) = send_chat(&router, receipt(&b), streaming).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(error_code(&body), "operation_conflict");
    assert_eq!(
        session_json(&saved(&harness, empty).await),
        session_json(&completed)
    );
    assert_eq!(harness.provider.call_count(), 2);
    assert_eq!(harness.tool_calls.load(Ordering::SeqCst), 0);
}
