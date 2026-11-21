//! Scoped readback of the last single provider exchange.

use apeireth_runtime::canonical::session::ProviderRequestContext;
use axum::extract::OriginalUri;

use super::*;

pub(super) const ENDPOINT: &str = "/v1/sessions/:session_id/provider-context";

#[derive(Serialize)]
pub(super) struct ContextReply {
    schema: u8,
    session: SessionId,
    provider_request: Option<ProviderRequestContext>,
}

pub(super) async fn read(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    user: Option<Extension<UserContext>>,
    Path(session): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<ContextReply>, Response> {
    let session = authorized_session_id(&state, user.as_ref().map(|user| &user.0), &session)?;
    if uri.query().is_some() {
        return Err(failure(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            "provider context read does not take a query",
        ));
    }
    let record = load_scoped_session(
        &state,
        domain.as_ref().map(|domain| &domain.0),
        session,
        ENDPOINT,
    )
    .await?;
    Ok(Json(ContextReply {
        schema: 1,
        session,
        provider_request: record.last_provider_request().cloned(),
    }))
}
