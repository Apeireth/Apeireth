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
use axum::http::{StatusCode, Uri};
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

    /// Write `value` **and** declare the endpoints this credential is allowed
    /// to reach (2026-11-01 安全审计 P1：凭据 ↔ 端点绑定).
    ///
    /// # 为什么默认实现是 **Err**（而不是静默忽略）
    ///
    /// 「静默丢弃绑定」会让部署方以为绑定生效了（实际没生效 ⇒ 保护是假的）。
    /// 不支持绑定的写入端必须在写入前拒绝。支持绑定的写入端必须成对发布值和绑定。
    /// 发布后若耐久性尚未确认，仍须返回 `Err`，保留已发布记录以便核对。
    fn write_with_endpoints(
        &self,
        _name: &str,
        _value: &str,
        _endpoints: &[String],
    ) -> Result<(), String> {
        Err("该凭据写入端不支持凭据与端点的成对保存；未写入任何状态".to_string())
    }

    /// 该凭据**已声明**的端点（`None` = **未声明** ⇒ 不参与绑定闸；空 `Some` = 声明了但为空）。
    ///
    /// # 为什么需要它（2026-11-04 · 端点绑定闸的连带残留）
    /// 绑定闸（`apeireth_provider::endpoint_binding`）在凭据**未声明绑定**时
    /// **不拦**（2026-11-01 P1 的有意权衡：否则正常的自建端点会被全部打死）。
    /// ⇒ ⇒ 于是「**只改端点、不带凭据**」时，若这把凭据**从未声明绑定**，
    /// 它会**静默跟随**到新端点 ⇒ ⇒ 真实 API key 可能被发往非预期主机。
    ///
    /// ⇒ ⇒ 本方法让网关能**读出**绑定状态 ⇒ ⇒ 跨 origin 热更时只有已显式
    /// 绑定的目标才能沿用现有凭据；未知或不匹配一律在变更前拒绝。
    ///
    /// ⚠️ 默认实现返回 `None`（「该写入端不可知」）⇒ ⇒ 与同 trait 既有
    /// 「默认实现 = 如实报失败」风格一致 ⇒ ⇒ **不假装**绑定存在。
    fn bound_endpoints(&self, _name: &str) -> Option<Vec<String>> {
        None
    }
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
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Json(config.view())
}

/// `POST /v1/admin/config`.
pub async fn admin_config_update(
    State(state): State<GatewayState>,
    Json(patch): Json<ConfigPatch>,
) -> Result<Json<ConfigUpdateResponse>, Response> {
    if state.services.model_connection.is_some() {
        return Err(ErrorFrame::response(
            StatusCode::CONFLICT,
            ErrorCode::InvalidRequest,
            "受管桌面请使用持久模型连接入口；当前运行时不接受热替换",
        ));
    }
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

    // Keep one write guard for the whole synchronous transaction. Axum can
    // execute two no-await handlers on different runtime workers, so dropping
    // a read snapshot here would let both requests validate against the same
    // old endpoint and then interleave their credential/config writes.
    let mut config = state
        .hot_config
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let current_provider = config.provider.clone();
    let current_base_url = config.base_url.clone();
    let provider = patch
        .provider
        .as_deref()
        .unwrap_or(&current_provider)
        .to_string();

    // Reusing a credential at another origin is never an implicit partial
    // update. A caller may still use any valid custom endpoint by either
    // declaring that origin in the credential store beforehand or supplying
    // the credential in the same request.
    if let Some(target) = patch.base_url.as_deref() {
        let current_origin = endpoint_origin(&current_base_url);
        let target_origin = endpoint_origin(target).expect("validated base_url has an origin");
        let provider_changed =
            normalize_provider(&provider) != normalize_provider(&current_provider);
        if (provider_changed || current_origin.as_deref() != Some(target_origin.as_str()))
            && patch.api_key.is_none()
        {
            let name = credential_name(&provider);
            let explicitly_bound = state
                .services
                .credentials
                .as_ref()
                .and_then(|writer| writer.bound_endpoints(&name))
                .is_some_and(|endpoints| {
                    endpoints.iter().any(|endpoint| {
                        endpoint_origin(endpoint).as_deref() == Some(target_origin.as_str())
                    })
                });
            if !explicitly_bound {
                return Err(ErrorFrame::response(
                    StatusCode::CONFLICT,
                    ErrorCode::InvalidRequest,
                    "更换模型端点来源时必须同笔提供 api_key，或先为现有凭据显式声明目标端点；配置保持不变",
                ));
            }
        }
    }

    // Publish the live configuration only after the credential writer confirms
    // the bound record. On error the durable record may need reconciliation.
    let compound_credential_written = match (patch.base_url.as_deref(), patch.api_key.as_deref()) {
        (Some(endpoint), Some(api_key)) => {
            let Some(writer) = &state.services.credentials else {
                return Err(ErrorFrame::response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    ErrorCode::Internal,
                    "当前凭据后端不支持凭据与端点的成对保存；配置保持不变",
                ));
            };
            let name = credential_name(&provider);
            if writer
                .write_with_endpoints(&name, api_key, &[endpoint.trim().to_string()])
                .is_err()
            {
                return Err(ErrorFrame::response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    ErrorCode::Internal,
                    "凭据与端点的持久保存未确认，当前运行配置保持不变。请核对后再重试",
                ));
            }
            true
        }
        _ => false,
    };

    // Validation passed: mutate the hot config state, then push the fields that
    // can reach the running graph (base_url -> provider capability, api_key ->
    // keyring) outward. Anything that cannot take effect is reported honestly
    // in `warnings` rather than faked.
    let mut warnings = Vec::new();
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

    if let Some(api_key) = patch
        .api_key
        .as_deref()
        .filter(|_| !compound_credential_written)
    {
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
        };
    }

    drop(config);
    Ok(Json(ConfigUpdateResponse { ok: true, warnings }))
}

fn provider_registered(state: &GatewayState, provider: &str) -> bool {
    let normalized = normalize_provider(provider);
    state
        .runtime
        .providers()
        .provider_ids()
        .iter()
        .any(|id| id.as_str() == normalized || id.as_str() == format!("provider.{normalized}"))
}

fn provider_capability_id(provider: &str) -> Result<CapabilityId, String> {
    let normalized = normalize_provider(provider);
    let raw = if normalized.starts_with("provider.") {
        normalized
    } else {
        format!("provider.{normalized}")
    };
    CapabilityId::new(raw).map_err(|error| format!("provider 名无法映射到 capability id: {error}"))
}

/// 归一 provider 家族名到注册时的 canonical 拼写。
///
/// 已知别名: admin/UI/钥匙串侧的 `openai` 指向注册的 `openai-compatible`
/// provider 家族 (capability id `provider.openai-compatible`)。不归一的话,
/// `{"provider":"openai","api_key":...}` 会把 key 写到
/// `provider.openai.api_key` 这个无人读取的逻辑名下, 而真正服务请求的
/// capability 按 `provider.openai-compatible.api_key` 解析 —— 热更永远打不中。
pub fn normalize_provider(provider: &str) -> String {
    let raw = provider.trim();
    let (prefix, family) = match raw.strip_prefix("provider.") {
        Some(rest) => ("provider.", rest),
        None => ("", raw),
    };
    let family = match family {
        "openai" => "openai-compatible",
        other => other,
    };
    format!("{prefix}{family}")
}

/// The backend credential name for a provider's API key, following the
/// production naming convention in `apeireth-provider::credentials`.
pub fn credential_name(provider: &str) -> String {
    let normalized = normalize_provider(provider);
    if normalized.starts_with("provider.") {
        format!("{normalized}.api_key")
    } else {
        format!("provider.{normalized}.api_key")
    }
}

pub(crate) fn valid_provider(provider: &str) -> bool {
    let provider = provider.trim();
    !provider.is_empty() && !provider.chars().any(char::is_whitespace)
}

/// M17 (2026-09-24 审计): https 强制, `http://` 仅放行环回主机
/// (本地 Ollama/vLLM 开发场景)。远端明文 HTTP 会把 Bearer API key 裸奔。
pub(crate) fn valid_base_url(base_url: &str) -> bool {
    let Some(uri) = parse_endpoint(base_url) else {
        return false;
    };
    uri.scheme == "https"
        || uri.scheme == "http" && matches!(uri.host.as_str(), "localhost" | "127.0.0.1" | "::1")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Endpoint {
    scheme: String,
    host: String,
    port: Option<u16>,
}

impl Endpoint {
    fn origin(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        match self.port {
            Some(port) => format!("{}://{host}:{port}", self.scheme),
            None => format!("{}://{host}", self.scheme),
        }
    }
}

fn parse_endpoint(base_url: &str) -> Option<Endpoint> {
    let raw = base_url.trim();
    if raw != base_url
        || raw.is_empty()
        || raw.contains(['?', '#'])
        || raw.chars().any(char::is_whitespace)
    {
        return None;
    }
    let uri = raw.parse::<Uri>().ok()?;
    let scheme = uri.scheme_str()?.to_ascii_lowercase();
    if !matches!(scheme.as_str(), "http" | "https") {
        return None;
    }
    let authority = uri.authority()?;
    if authority.as_str().contains('@') {
        return None;
    }
    // `http::Uri` accepts an authority such as `host:99999`, but exposes no
    // parsed port for it. Reqwest rejects that URL. Refuse any syntactically
    // explicit port that did not parse as `u16`, so validation and the actual
    // outbound client cannot disagree about the destination.
    let authority_text = authority.as_str();
    let has_explicit_port = if authority_text.starts_with('[') {
        let end = authority_text.find(']')?;
        match &authority_text[end + 1..] {
            "" => false,
            tail if tail.starts_with(':') => true,
            _ => return None,
        }
    } else {
        authority_text.contains(':')
    };
    let port = authority.port_u16();
    if has_explicit_port && port.is_none() {
        return None;
    }
    let port = match (scheme.as_str(), port) {
        ("http", Some(80)) | ("https", Some(443)) => None,
        (_, port) => port,
    };
    let host = authority
        .host()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    Some(Endpoint { scheme, host, port })
}

fn endpoint_origin(base_url: &str) -> Option<String> {
    parse_endpoint(base_url).map(|endpoint| endpoint.origin())
}

/// Mask an API key as `sk-****c4a` (first 3 + redacted middle + last 3).
/// M17 (2026-09-24 审计): 按 char 边界切片 —— 旧版字节切片在非 ASCII key
/// (如多字节 UTF-8) 上会 panic (`byte index is not a char boundary`)。
pub fn mask_api_key(api_key: &str) -> String {
    let chars: Vec<char> = api_key.chars().collect();
    if chars.len() <= 8 {
        return "****".to_string();
    }
    let prefix: String = chars[..3].iter().collect();
    let suffix: String = chars[chars.len() - 3..].iter().collect();
    format!("{prefix}****{suffix}")
}

/// A shared handle for the mutable hot config, kept in [`GatewayState`].
pub type HotConfigHandle = std::sync::Arc<RwLock<GatewayRuntimeConfig>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct WriteOnly(Mutex<Vec<(String, String)>>);

    impl CredentialWriter for WriteOnly {
        fn write(&self, name: &str, value: &str) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .push((name.to_string(), value.to_string()));
            Ok(())
        }
    }

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
        assert!(valid_base_url("http://[::1]:8080/v1"));
        assert!(!valid_base_url("ftp://example.com"));
        assert!(!valid_base_url("not-a-url"));
        assert!(!valid_base_url("https://user:pass@example.com/v1"));
        assert!(!valid_base_url("https://example.com/v1?key=value"));
        assert!(!valid_base_url("http://example.com/v1"));
        assert!(!valid_base_url("https://example.com:99999/v1"));
        assert!(!valid_base_url("https://example.com:notaport/v1"));
        assert!(!valid_base_url("https://[::1]garbage/v1"));
        assert!(!valid_base_url(" https://example.com/v1"));
        assert!(!valid_base_url("https://example.com/v1 "));
    }

    #[test]
    fn endpoint_origin_normalizes_case_and_path_without_merging_ports() {
        assert_eq!(
            endpoint_origin("https://API.Example.com/v1"),
            Some("https://api.example.com".to_string())
        );
        assert_eq!(
            endpoint_origin("https://api.example.com:443/other"),
            Some("https://api.example.com".to_string())
        );
        assert_eq!(
            endpoint_origin("https://api.example.com:8443/other"),
            Some("https://api.example.com:8443".to_string())
        );
        assert_eq!(
            endpoint_origin("http://[::1]:8080/v1"),
            Some("http://[::1]:8080".to_string())
        );

        for endpoint in [
            "https://API.Example.com/v1",
            "https://api.example.com:443/other",
            "https://api.example.com:8443/other",
            "http://127.0.0.1:80/v1",
            "http://[::1]:8080/v1",
        ] {
            assert_eq!(
                endpoint_origin(endpoint),
                apeireth_provider::endpoint_binding::endpoint_origin(endpoint),
                "admin and provider binding parsers must agree for {endpoint}"
            );
        }
    }

    #[test]
    fn unsupported_compound_write_is_side_effect_free() {
        let writer = WriteOnly::default();
        let result = writer.write_with_endpoints(
            "provider.synthetic.api_key",
            "synthetic-value",
            &["https://endpoint.example/v1".to_string()],
        );
        assert!(result.is_err());
        assert!(writer.0.lock().unwrap().is_empty());
    }

    #[test]
    fn maps_provider_to_credential_and_capability_names() {
        assert_eq!(
            credential_name("openai-compatible"),
            "provider.openai-compatible.api_key"
        );
        assert_eq!(
            provider_capability_id("openai-compatible")
                .unwrap()
                .as_str(),
            "provider.openai-compatible"
        );
    }

    #[test]
    fn openai_alias_normalizes_to_the_registered_family() {
        // admin/UI 侧 "openai" 拼写必须落到注册的 openai-compatible 家族,
        // 否则 api_key 写到无人读取的逻辑名下, 热更打不中服务请求的 capability。
        assert_eq!(normalize_provider("openai"), "openai-compatible");
        assert_eq!(
            normalize_provider("provider.openai"),
            "provider.openai-compatible"
        );
        assert_eq!(normalize_provider("openai-compatible"), "openai-compatible");
        assert_eq!(normalize_provider("minimax"), "minimax");
        assert_eq!(
            credential_name("openai"),
            "provider.openai-compatible.api_key"
        );
        assert_eq!(
            credential_name("provider.openai"),
            "provider.openai-compatible.api_key"
        );
        assert_eq!(
            provider_capability_id("openai").unwrap().as_str(),
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
