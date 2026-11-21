//! The latest measured provider invocation for one session.

use apeireth_core::kernel::{CapabilityId, ModelId, RequestId, TraceId};
use apeireth_mesh::RemoteProviderDescriptor;
use apeireth_protocol::canonical::NormalizedResponse;
use serde::{Deserialize, Serialize};

use super::error::RuntimeResult;
use super::provider::RoutedCompletion;
use super::runtime::Runtime;
use super::session::Session;

/// Context facts for a single provider request, correlated to its owning turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRequestContext {
    /// The canonical turn request containing this provider invocation.
    pub request: RequestId,
    /// The turn execution trace.
    pub trace_id: TraceId,
    /// The provider that served this invocation.
    pub provider: CapabilityId,
    /// The exact registered model routed to that provider.
    pub model: ModelId,
    /// Provider-reported context input for this invocation, when known.
    pub input_tokens: Option<u32>,
    /// Explicit window advertised by the provider for the same model.
    pub context_window: Option<u32>,
}

impl ProviderRequestContext {
    pub(super) fn observed(
        routed: &RoutedCompletion,
        request: RequestId,
        trace_id: TraceId,
    ) -> Option<Self> {
        let model = routed.request_model.as_ref()?;
        Some(Self {
            request,
            trace_id,
            provider: routed.served_by.clone(),
            model: model.id.clone(),
            input_tokens: routed.response.provider_context_input(),
            context_window: model.context_window,
        })
    }
}

impl Runtime {
    pub(super) async fn clear_provider_request(&self, session: &mut Session) -> RuntimeResult<()> {
        if session.last_provider_request.take().is_some() {
            self.sessions().save(session).await?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct RoutedRequestModel {
    pub(super) id: ModelId,
    pub(super) context_window: Option<u32>,
}

/// Exact canonical ids and declared aliases can confirm the same model.
fn model_matches(model: &ModelId, alias: Option<&str>, actual: &str) -> bool {
    model.as_str().eq_ignore_ascii_case(actual) || alias == Some(actual)
}

pub(super) fn remote_completion(
    descriptor: RemoteProviderDescriptor,
    response: NormalizedResponse,
    requested: &str,
) -> RoutedCompletion {
    let request_model = routed_model(
        descriptor
            .models
            .iter()
            .filter(|model| model.provider == descriptor.capability)
            .map(|model| {
                (
                    &model.id,
                    model.display_name.as_deref(),
                    model.context_window,
                )
            }),
        requested,
        &response.model,
    );
    RoutedCompletion {
        served_by: descriptor.capability,
        response,
        request_model,
        failed_attempts: Vec::new(),
    }
}

pub(super) fn routed_model<'a>(
    models: impl Iterator<Item = (&'a ModelId, Option<&'a str>, Option<u32>)>,
    requested: &str,
    responded: &str,
) -> Option<RoutedRequestModel> {
    let mut matching = models.filter(|(id, alias, _)| model_matches(id, *alias, requested));
    let (id, wire_alias, window) = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let context_window = window.filter(|window| {
        *window > 0 && (id.as_str() == responded || wire_alias == Some(responded))
    });
    Some(RoutedRequestModel {
        id: id.clone(),
        context_window,
    })
}
