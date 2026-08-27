//! Canonical Apeireth command-line entry points.
//!
//! The CLI is a thin adapter: it bootstraps one canonical runtime, constructs
//! a canonical turn request, and delegates execution to `Runtime::execute`.

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_runtime::canonical::{Runtime, TurnRequest, TurnResponse};

/// Build the one canonical runtime used by CLI chat and the HTTP gateway.
///
/// Provider implementations are injected as plugins. Credentials are resolved
/// at execution time, so neither the runtime nor a provider stores API keys.
pub async fn build_canonical_runtime_from_env() -> Result<Runtime, String> {
    use apeireth_provider::canonical_anthropic::AnthropicProviderPlugin;
    use apeireth_provider::canonical_minimax::MinimaxProviderPlugin;
    use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;
    use apeireth_provider::credentials::EnvCredentialResolver;

    let configured_model = std::env::var("APEIRETH_MODEL")
        .ok()
        .filter(|model| !model.trim().is_empty());

    let mut builder = Runtime::builder();
    let resolver: Arc<dyn apeireth_plugin::CredentialResolver> =
        Arc::new(EnvCredentialResolver::new());
    builder = builder.with_credentials(resolver);

    let workspace_root = std::env::current_dir()
        .map_err(|error| format!("canonical runtime bootstrap failed: current_dir: {error}"))?;
    builder = builder.with_plugin(Arc::new(apeireth_tools_canonical::BuiltinToolsPlugin::new(
        workspace_root,
    )));

    let first_default_model: Option<String>;
    let mut fallback_order: Vec<CapabilityId> = Vec::new();

    let minimax = MinimaxProviderPlugin::from_env()
        .map_err(|error| format!("minimax provider activation failed: {error}"))?;
    first_default_model = minimax.model_ids().first().cloned();
    fallback_order.push(CapabilityId::new("provider.minimax").unwrap());
    builder = builder.with_plugin(Arc::new(minimax));

    let anthropic = AnthropicProviderPlugin::from_env()
        .map_err(|error| format!("anthropic provider activation failed: {error}"))?;
    fallback_order.push(CapabilityId::new("provider.anthropic").unwrap());
    builder = builder.with_plugin(Arc::new(anthropic));

    if std::env::var("APEIRETH_OPENAI_MODELS")
        .ok()
        .as_ref()
        .is_some_and(|models| !models.trim().is_empty())
    {
        let openai = OpenAiCompatibleProviderPlugin::from_env()
            .map_err(|error| format!("openai-compatible provider activation failed: {error}"))?;
        fallback_order.push(CapabilityId::new("provider.openai-compatible").unwrap());
        builder = builder.with_plugin(Arc::new(openai));
    }

    builder = builder.with_fallback_order(fallback_order);
    if let Some(model) = configured_model.or(first_default_model) {
        builder = builder.with_default_model(model);
    }

    builder
        .build()
        .await
        .map_err(|error| format!("canonical runtime bootstrap failed: {error}"))
}

/// Execute one CLI turn directly through [`Runtime::execute`].
pub async fn execute_canonical_cli_turn(
    runtime: &Runtime,
    prompt: impl Into<String>,
    model: Option<String>,
    session: Option<SessionId>,
) -> Result<TurnResponse, String> {
    let mut request = TurnRequest::new(session.unwrap_or_else(SessionId::new), prompt);
    if let Some(model) = model {
        request = request.with_model(model);
    }
    runtime
        .execute(request)
        .await
        .map_err(|error| error.to_string())
}

/// Bootstrap and execute the canonical CLI chat path.
pub async fn dispatch_canonical_chat(
    prompt: impl Into<String>,
    model: Option<String>,
    session: Option<String>,
) -> Result<TurnResponse, String> {
    let session = session
        .map(|id| id.parse::<SessionId>().map_err(|error| error.to_string()))
        .transpose()?;
    let runtime = build_canonical_runtime_from_env().await?;
    execute_canonical_cli_turn(&runtime, prompt, model, session).await
}

/// Start the HTTP Gateway backed by one long-lived canonical runtime.
/// Blocks until the server exits.
pub async fn dispatch_gateway_serve(port: u16) -> Result<String, String> {
    let runtime = Arc::new(build_canonical_runtime_from_env().await?);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .map_err(|error| format!("bind 0.0.0.0:{port} failed: {error}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| format!("local_addr: {error}"))?;
    let url = format!("http://{local_addr}");

    eprintln!("canonical gateway started at {url}");
    apeireth_gateway::serve_canonical(listener, runtime)
        .await
        .map_err(|error| format!("gateway server failed: {error}"))?;

    Ok(format!("server stopped at {url}"))
}
