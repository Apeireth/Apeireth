//! Runtime configuration hot reload (`/v1/admin/config`).
//!
//! The gateway keeps an effective runtime config in an [`RwLock`] on
//! [`GatewayState`]. `POST /v1/admin/config` patches it in place; the next
//! request reads the patched value without a process restart. Startup values
//! come from the existing environment (`APEIRETH_OPENAI_URL`, `OPENAI_API_KEY`,
//! `APEIRETH_MODEL`); an admin patch always wins over those startup values.

use std::sync::RwLock;

use apeireth_core::kernel::CapabilityId;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error_frame::{ErrorCode, ErrorFrame};
use crate::panels::GatewayState;

/// The provider family the gateway hot config manages by default.
pub const DEFAULT_PROVIDER: &str = "openai-compatible";
/// The safe default endpoint for the generic provider family.
pub const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
/// The environment variable read for the managed API key at startup.
pub const API_KEY_ENV: &str = "OPENAI_API_KEY";
/// The environment variable read for the managed base URL at startup.
pub const BASE_URL_ENV: &str = "APEIRETH_OPENAI_URL";
/// The environment variable read for the default model at startup.
pub const MODEL_ENV: &str = "APEIRETH_MODEL";

/// A bounded-context port that writes a credential to the process's credential
/// backend (production: the OS keyring via the CLI composition root). The
/// gateway cannot depend on the concrete keyring crate, so the write seam is
/// injected as a service.
pub trait CredentialWriter: Send + Sync {
    /// Write `value` under the backend credential name `name`.
    fn write(&self, name: &str, value: &str) -> Result<(), String>;
}

/// Capability flags carried by the hot config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitiesConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
}

/// The effective runtime config held by the gateway.
///
/// `api_key` lives in plaintext only inside this lock; the [`Debug`] impl and
/// every serialized projection redact it.
pub struct GatewayRuntimeConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub capabilities: CapabilitiesConfig,
    /// Whether `model` was set through `/v1/admin/config` (vs the startup env
    /// baseline). Only an admin-patched model is injected into requests; the
    /// env baseline is already the runtime's own default model.
    pub(crate) admin_model_set: bool,
}

impl GatewayRuntimeConfig {
    /// Initialize from the same environment the composition root reads.
    pub fn from_env() -> Self {
        Self {
            provider: DEFAULT_PROVIDER.to_string(),
            base_url: std::env::var(BASE_URL_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            api_key: std::env::var(API_KEY_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty()),
            model: std::env::var(MODEL_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty()),
            capabilities: CapabilitiesConfig {
                streaming: Some(true),
            },
            admin_model_set: false,
        }
    }

    /// A secret-safe projection for the admin GET handler.
    pub fn view(&self) -> ConfigView {
        ConfigView {
            provider: self.provider.clone(),
            base_url: self.base_url.clone(),
            api_key: self.api_key.as_deref().map(mask_api_key),
            model: self.model.clone(),
            capabilities: self.capabilities.clone(),
        }
    }
}

impl std::fmt::Debug for GatewayRuntimeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GatewayRuntimeConfig")
            .field("provider", &self.provider)
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("model", &self.model)
            .field("capabilities", &self.capabilities)
            .field("admin_model_set", &self.admin_model_set)
            .finish()
    }
}

/// The GET response: the current config with the API key masked.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigView {
    pub provider: String,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub capabilities: CapabilitiesConfig,
}

/// A partial `POST /v1/admin/config` body. Every field is optional; `None`
/// means "leave unchanged".
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConfigPatch {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub capabilities: Option<CapabilitiesConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigUpdateResponse {
    pub ok: bool,
    pub warnings: Vec<String>,
}

/// `GET /v1/admin/config`.
pub async fn admin_config_get(State(state): State<GatewayState>) -> Json<ConfigView> {
    let config = state
        .hot_config
        .read()
        .expect("gateway hot config lock poisoned");
    Json(config.view())
}

/// `POST /v1/admin/config`.
pub async fn admin_config_update(
    State(state): State<GatewayState>,
    Json(patch): Json<ConfigPatch>,
) -> Result<Json<ConfigUpdateResponse>, Response> {
    if let Some(provider) = patch.provider.as_deref() {
        if !valid_provider(provider) {
            return Err(ErrorFrame::response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                format!("provider 非法: {provider:?}"),
            ));
        }
    }
    if let Some(base_url) = patch.base_url.as_deref() {
        if !valid_base_url(base_url) {
            return Err(ErrorFrame::response(
                StatusCode::BAD_REQUEST,
                ErrorCode::InvalidRequest,
                format!("base_url 非法: {base_url:?}"),
            ));
        }
    }

    // Validation passed: mutate the hot config state, then push the fields that
    // can reach the running graph (base_url -> provider capability, api_key ->
    // keyring) outward. Anything that cannot take effect is reported honestly
    // in `warnings` rather than faked.
    let mut warnings = Vec::new();
    {
        let mut config = state
            .hot_config
            .write()
            .expect("gateway hot config lock poisoned");
        if let Some(provider) = patch.provider.clone() {
            config.provider = provider;
        }
        if let Some(base_url) = patch.base_url.clone() {
            config.base_url = base_url;
        }
        if let Some(api_key) = patch.api_key.clone() {
            config.api_key = Some(api_key);
        }
        if let Some(model) = patch.model.clone() {
            config.model = Some(model);
            config.admin_model_set = true;
        }
        if let Some(capabilities) = patch.capabilities.clone() {
            config.capabilities = capabilities;
        }
    }

    let provider = state
        .hot_config
        .read()
        .expect("gateway hot config lock poisoned")
        .provider
        .clone();

    if !provider_registered(&state, &provider) {
        warnings.push(format!(
            "provider {provider:?} 未在当前运行时注册; provider 选择仍由已注册 provider 决定"
        ));
    }

    if let Some(base_url) = patch.base_url.as_deref() {
        match provider_capability_id(&provider) {
            Ok(provider_id) => {
                if let Err(error) = state
                    .runtime
                    .providers()
                    .apply_hot_config(&provider_id, Some(base_url))
                {
                    warnings.push(format!("base_url 热更新未生效: {error}"));
                }
            }
            Err(error) => warnings.push(error),
        }
    }

    if patch.capabilities.is_some() {
        warnings.push(
            "capabilities 仅存 gateway 热配置状态并在 GET 回显; 无法对运行中的 provider 图热生效"
                .to_string(),
        );
    }

    if let Some(api_key) = patch.api_key.as_deref() {
        let name = credential_name(&provider);
        match &state.services.credentials {
            Some(writer) => {
                if let Err(error) = writer.write(&name, api_key) {
                    warnings.push(format!("api_key 未写入 keyring ({name}): {error}"));
                }
            }
            None => warnings.push(format!(
                "api_key 未写入 keyring ({name}): gateway 未挂凭据写入端口"
            )),
        }
    }

    Ok(Json(ConfigUpdateResponse { ok: true, warnings }))
}

fn provider_registered(state: &GatewayState, provider: &str) -> bool {
    state.runtime.providers().provider_ids().iter().any(|id| {
        id.as_str() == provider || id.as_str() == format!("provider.{provider}")
    })
}

fn provider_capability_id(provider: &str) -> Result<CapabilityId, String> {
    let raw = if provider.starts_with("provider.") {
        provider.to_string()
    } else {
        format!("provider.{provider}")
    };
    CapabilityId::new(raw).map_err(|error| format!("provider 名无法映射到 capability id: {error}"))
}

/// The backend credential name for a provider's API key, following the
/// production naming convention in `apeireth-provider::credentials`.
pub fn credential_name(provider: &str) -> String {
    if provider.starts_with("provider.") {
        format!("{provider}.api_key")
    } else {
        format!("provider.{provider}.api_key")
    }
}

pub(crate) fn valid_provider(provider: &str) -> bool {
    let provider = provider.trim();
    !provider.is_empty() && !provider.chars().any(char::is_whitespace)
}

pub(crate) fn valid_base_url(base_url: &str) -> bool {
    let base_url = base_url.trim();
    let Some(rest) = base_url
        .strip_prefix("https://")
        .or_else(|| base_url.strip_prefix("http://"))
    else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    !host.is_empty() && !host.chars().any(char::is_whitespace)
}

/// Mask an API key as `sk-****c4a` (first 3 + redacted middle + last 3).
pub fn mask_api_key(api_key: &str) -> String {
    if api_key.len() <= 8 {
        return "****".to_string();
    }
    format!("{}****{}", &api_key[..3], &api_key[api_key.len() - 3..])
}

/// A shared handle for the mutable hot config, kept in [`GatewayState`].
pub type HotConfigHandle = std::sync::Arc<RwLock<GatewayRuntimeConfig>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_long_and_short_keys() {
        assert_eq!(mask_api_key("sk-abcdef123c4a"), "sk-****c4a");
        assert_eq!(mask_api_key("short"), "****");
    }

    #[test]
    fn validates_provider_and_base_url() {
        assert!(valid_provider("openai-compatible"));
        assert!(!valid_provider(""));
        assert!(!valid_provider("bad provider"));

        assert!(valid_base_url("https://api.example.com/v1"));
        assert!(valid_base_url("http://localhost:8080"));
        assert!(!valid_base_url("ftp://example.com"));
        assert!(!valid_base_url("not-a-url"));
    }

    #[test]
    fn maps_provider_to_credential_and_capability_names() {
        assert_eq!(
            credential_name("openai-compatible"),
            "provider.openai-compatible.api_key"
        );
        assert_eq!(
            provider_capability_id("openai-compatible").unwrap().as_str(),
            "provider.openai-compatible"
        );
    }

    #[test]
    fn debug_never_prints_the_api_key() {
        let config = GatewayRuntimeConfig {
            provider: "openai-compatible".into(),
            base_url: "https://example.com/v1".into(),
            api_key: Some("sk-super-secret-value".into()),
            model: Some("deepseek-v4-flash".into()),
            capabilities: CapabilitiesConfig {
                streaming: Some(true),
            },
            admin_model_set: false,
        };
        let printed = format!("{config:?}");
        assert!(!printed.contains("sk-super-secret-value"), "{printed}");
        assert!(printed.contains("<redacted>"), "{printed}");
    }
}
