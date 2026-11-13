//! The live-model path.
//!
//! ⭐ The live path is **opt-in**: it only exists behind an explicit
//! `--live` switch plus an endpoint and a model id. The credential itself
//! never enters this module, never enters the configuration struct and never
//! enters an artifact: the repository's own provider plugin resolves it per
//! turn from an environment variable, and only the **name** of that variable
//! is recorded.
//!
//! The provider seam is the repository's own: the OpenAI-compatible plugin
//! (`apeireth_provider::canonical_openai_compatible`) plus the environment
//! credential resolver (`apeireth_provider::credentials::EnvCredentialResolver`).
//! No HTTP client is written here.

use std::sync::Arc;

use apeireth_core::kernel::PluginId;
use apeireth_plugin::{CapabilityKind, Plugin, PluginManifest, ProviderCapability};
use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;
use apeireth_provider::credentials::EnvCredentialResolver;

use super::artifacts::Result;
use super::config::Config;

/// The plugin id this evaluation declares for the live path.
const LIVE_PLUGIN_ID: &str = "plugin.memory-longterm.live-model";

/// The environment variable that carries **extra vendor-private request
/// fields** (a JSON object), for endpoints whose request shape has knobs the
/// canonical protocol does not model.
///
/// ⭐ It is a request *shape* knob, not a credential: it must never carry a
/// secret, and only the **key names** of this object reach the artifacts.
const VENDOR_EXTRA_BODY_ENV: &str = "APEIRETH_OPENAI_EXTRA_BODY";

/// The live provider of one run.
///
/// The capability is borrowed from the repository's provider plugin; this
/// struct only adds the manifest this evaluation owns and the non-secret
/// facts (model id, endpoint) that the artifacts must record.
pub struct LiveProvider {
    capability: Arc<dyn ProviderCapability>,
    manifest: PluginManifest,
    model: String,
    endpoint: String,
    /// The **key names** of the vendor-private request fields in play.
    vendor_fields: Vec<String>,
}

impl LiveProvider {
    /// Build the live provider from a validated live configuration.
    ///
    /// ⭐ The credential is **not** read here: the resolver reads the
    /// environment at call time, so the factory holds no secret at all.
    pub fn build(config: &Config) -> Result<Self> {
        let endpoint = config
            .endpoint
            .clone()
            .ok_or("live mode requires an endpoint")?;
        let model = config.model.clone();
        if model.trim().is_empty() {
            return Err("live mode requires a non-empty model id".into());
        }
        // The same redirect policy the repository's own factory uses: a chat
        // completions endpoint never redirects, and a redirect could carry the
        // credential to another host.
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let plugin = OpenAiCompatibleProviderPlugin::new(
            endpoint.clone(),
            vec![model.clone()],
            http,
            config.timeout_ms,
        )?;
        // The vendor-private request fields (e.g. the vendor's reasoning-mode
        // switch) travel through the provider's own opt-in passthrough. They
        // are part of the **request shape**, not of the credential: the
        // credential still resolves per turn from the environment.
        let vendor_body = vendor_body(config)?;
        let vendor_fields = vendor_body
            .as_object()
            .map(|fields| fields.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        plugin.set_extra_body(vendor_body);
        // The resolver slot of the provider plugin is private; attaching the
        // repository's environment resolver is the supported way to make the
        // per-turn credential resolution work outside a full runtime boot.
        plugin.attach_resolver_for_test(Arc::new(EnvCredentialResolver::new()));
        let capability = plugin
            .providers()
            .into_iter()
            .next()
            .ok_or("the provider plugin declared no capability")?;
        // The runtime routes by the canonical (folded) model id, not by the
        // spelling the operator typed.
        let model = plugin.model_ids().into_iter().next().unwrap_or(model);
        let manifest = PluginManifest::new(
            PluginId::new(LIVE_PLUGIN_ID).map_err(|error| error.to_string())?,
            "0.1.0",
            "live model path of the long-range memory evaluation",
        )
        .declare_capability(
            capability.id().clone(),
            CapabilityKind::Provider,
            "answers the evaluation questions through the configured endpoint",
        )
        .map_err(|error| error.to_string())?;
        Ok(Self {
            capability,
            manifest,
            model,
            endpoint,
            vendor_fields,
        })
    }

    /// The capability the runtime dispatches to.
    pub fn capability(&self) -> Arc<dyn ProviderCapability> {
        Arc::clone(&self.capability)
    }

    /// The manifest this evaluation declares for the live path.
    pub fn manifest(&self) -> PluginManifest {
        self.manifest.clone()
    }

    /// The canonical model id the run is bound to.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The non-secret endpoint of the run.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// The **key names** of the vendor-private request fields in play.
    ///
    /// ⭐ Only names, never values: the values are request-shape knobs chosen
    /// by the operator, and this run records enough to reproduce the request
    /// shape without copying anything the operator typed into an artifact.
    pub fn vendor_fields(&self) -> &[String] {
        &self.vendor_fields
    }
}

/// Build the vendor-private request fields of a live run.
///
/// Three sources, in this order (later wins):
/// 1. the configured reasoning-mode switch (`config.thinking`), when the
///    operator set it;
/// 2. the configured reasoning effort (`config.reasoning_effort`);
/// 3. the raw JSON object in [`VENDOR_EXTRA_BODY_ENV`].
///
/// ⭐ Reserved protocol keys are refused **here**, before the first request:
/// the provider would refuse them too, but refusing up front turns "the
/// vendor answered 400" into "the configuration is wrong".
fn vendor_body(config: &Config) -> Result<serde_json::Value> {
    let extra = std::env::var(VENDOR_EXTRA_BODY_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());
    vendor_body_from(config, extra.as_deref())
}

/// Build the vendor-private request fields from the configuration and an
/// optional raw extra-body object.
///
/// ⭐ Split from [`vendor_body`] so the rule can be tested without touching
/// the process environment (the environment is global state; tests must not
/// race on it).
pub(super) fn vendor_body_from(config: &Config, extra: Option<&str>) -> Result<serde_json::Value> {
    let mut fields = serde_json::Map::new();
    if let Some(thinking) = config.thinking {
        let mode = if thinking { "enabled" } else { "disabled" };
        fields.insert("thinking".to_string(), serde_json::json!({ "type": mode }));
    }
    if let Some(effort) = config.reasoning_effort.as_ref() {
        fields.insert("reasoning_effort".to_string(), serde_json::json!(effort));
    }
    if let Some(raw) = extra.map(str::trim).filter(|value| !value.is_empty()) {
        let parsed: serde_json::Value = serde_json::from_str(raw)
            .map_err(|error| format!("{VENDOR_EXTRA_BODY_ENV} 不是合法 JSON：{error}"))?;
        let Some(extra_fields) = parsed.as_object() else {
            return Err(format!("{VENDOR_EXTRA_BODY_ENV} 必须是 JSON 对象").into());
        };
        for (key, value) in extra_fields {
            fields.insert(key.clone(), value.clone());
        }
    }
    for key in fields.keys() {
        if apeireth_provider::openai_chat::RESERVED_EXTRA_BODY_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "{VENDOR_EXTRA_BODY_ENV} 不得覆盖协议保留键 '{key}'（保留键：{:?}）",
                apeireth_provider::openai_chat::RESERVED_EXTRA_BODY_KEYS
            )
            .into());
        }
    }
    Ok(serde_json::Value::Object(fields))
}
