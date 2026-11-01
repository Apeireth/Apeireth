//! Production credential resolution for canonical providers.
//!
//! A canonical [`ProviderCapability`] never
//! stores a secret as a plain `String` field and never reads a fixed path. It
//! asks the runtime's [`CredentialResolver`]
//! for a secret by *logical name* (e.g. `provider.minimax.api_key`) and the
//! resolver decides where that secret physically lives.
//!
//! [`EnvCredentialResolver`] is the production resolver: logical names map to
//! environment variables, and the resolved value is returned as a redacted
//! [`Secret`]. It is the honest upgrade over the legacy
//! `api_key: String` field that `ApeirethApiConfig` carried: no secret sits on a
//! long-lived struct, no `Debug` print can leak it, and a missing key resolves to
//! `None` so the provider can fail explicitly rather than silently fall back.
//!
//! # Precedence and defaults
//!
//! Logical name → environment variable is configuration, not a secret, so a
//! default mapping is acceptable (§19/§42). The default maps the pilot provider's
//! key to `APEIRETH_API_KEY`; `with_mapping` lets a second provider claim a
//! distinct variable without changing the contract or the resolver type. The
//! resolver itself holds **no** secret material — only name→variable strings.
//!
//! # What this is not
//!
//! This resolver reads environment variables. OS keyring, encrypted-file, and
//! KMS backends already exist in `apeireth-credentials` (`CredentialsStore`,
//! `KeyringSelector`); wiring those behind [`CredentialResolver`] is a tracked
//! P1 follow-up rather than duplicated here. The contract a provider sees is
//! identical either way.

use std::collections::BTreeMap;

use apeireth_plugin::{CredentialResolver, Secret};

/// The logical credential name for the minimax provider's API key.
///
/// Stable semantic identity (§15): the provider asks for this name; the resolver
/// maps it to whatever physical source serves this deployment.
pub const MINIMAX_API_KEY: &str = "provider.minimax.api_key";

/// The environment variable the default mapping reads for the minimax key.
pub const MINIMAX_API_KEY_ENV: &str = "APEIRETH_API_KEY";

/// The logical credential name for the anthropic provider's API key.
pub const ANTHROPIC_API_KEY: &str = "provider.anthropic.api_key";

/// The environment variable the default mapping reads for the anthropic key.
///
/// Follows the repository's existing convention (`APEIRETH_ANTHROPIC_KEY`, set
/// by the legacy `AnthropicCompatibleConfig::from_env`) rather than the
/// upstream `ANTHROPIC_API_KEY`, so existing user configuration keeps working.
pub const ANTHROPIC_API_KEY_ENV: &str = "APEIRETH_ANTHROPIC_KEY";

/// The logical credential name for the generic OpenAI-compatible provider's
/// API key. The provider identity is `provider.openai-compatible` (a protocol
/// family, not a vendor) — the key is named for that identity, not for "openai"
/// the vendor (§8/§9).
pub const OPENAI_COMPATIBLE_API_KEY: &str = "provider.openai-compatible.api_key";

/// The environment variable the default mapping reads for the
/// OpenAI-compatible key. Follows the repository's documented config
/// convention (`OPENAI_API_KEY`), so existing user configuration keeps working.
pub const OPENAI_COMPATIBLE_API_KEY_ENV: &str = "OPENAI_API_KEY";

/// A production [`CredentialResolver`] backed by environment variables.
///
/// Holds only a name→variable map (configuration, not secret material). Each
/// [`EnvCredentialResolver::resolve`] call reads the mapped environment variable
/// at call time and returns it wrapped in a redacted [`Secret`]; an unset or
/// empty variable resolves to `None`.
#[derive(Debug, Clone, Default)]
pub struct EnvCredentialResolver {
    mappings: BTreeMap<String, String>,
    /// **端点绑定声明**：逻辑名 → 该凭据被授权发送到的端点列表。
    ///
    /// 空表 ⇒ 该 resolver **不参与**绑定（[`CredentialResolver::bound_endpoints`]
    /// 返回 `None`）⇒ 调用方按「未声明绑定」处理。
    bindings: BTreeMap<String, Vec<String>>,
}

impl EnvCredentialResolver {
    /// A resolver with the default semantic-name → env-var mappings.
    ///
    /// Maps [`MINIMAX_API_KEY`] → [`MINIMAX_API_KEY_ENV`],
    /// [`ANTHROPIC_API_KEY`] → [`ANTHROPIC_API_KEY_ENV`], and
    /// [`OPENAI_COMPATIBLE_API_KEY`] → [`OPENAI_COMPATIBLE_API_KEY_ENV`].
    /// Adding a further canonical provider means adding its default mapping
    /// here, not teaching the provider a new resolver type. Unknown semantic
    /// ids resolve to `None` (§20): there is no catch-all
    /// `provider.<anything>.api_key` mapping.
    pub fn new() -> Self {
        let mut mappings = BTreeMap::new();
        mappings.insert(MINIMAX_API_KEY.to_string(), MINIMAX_API_KEY_ENV.to_string());
        mappings.insert(
            ANTHROPIC_API_KEY.to_string(),
            ANTHROPIC_API_KEY_ENV.to_string(),
        );
        mappings.insert(
            OPENAI_COMPATIBLE_API_KEY.to_string(),
            OPENAI_COMPATIBLE_API_KEY_ENV.to_string(),
        );
        let mut resolver = Self {
            mappings,
            bindings: BTreeMap::new(),
        };
        // 生产路径**自动**读端点绑定声明（`APEIRETH_CREDENTIAL_ENDPOINT_BINDINGS`）
        // ⇒ 部署方声明了才生效，不声明就没有（协议族 provider 不被误伤）。
        resolver = resolver.with_endpoint_bindings_from_env();
        resolver
    }

    /// Override or add a semantic-name → env-var mapping.
    ///
    /// `with_mapping("provider.openai.api_key", "OPENAI_API_KEY")` lets a second
    /// provider claim its own variable without disturbing the pilot's mapping.
    #[must_use]
    pub fn with_mapping(mut self, semantic: impl Into<String>, env_var: impl Into<String>) -> Self {
        self.mappings.insert(semantic.into(), env_var.into());
        self
    }

    /// The environment variable this resolver would read for `semantic`, if any.
    ///
    /// Configuration introspection only; never returns a secret.
    pub fn env_var_for(&self, semantic: &str) -> Option<&str> {
        self.mappings.get(semantic).map(String::as_str)
    }

    /// **声明**该凭据被授权发送到的端点（origin 形态）。
    ///
    /// 声明了 ⇒ 调用方（provider 侧 `endpoint_binding`）**强制**校验：
    /// 未在表内的端点 ⇒ **拒绝**发凭据。
    /// 没声明 ⇒ 该 resolver 不参与绑定（保持既有行为）。
    ///
    /// ⚠️ 这是**部署方的显式声明**，不是密码学证明：它防的是「**只改端点**就泄漏」，
    /// 不防「能同时改端点与声明」的对手（那种对手本来就能改凭据）。
    #[must_use]
    pub fn with_endpoint_binding(
        mut self,
        semantic: impl Into<String>,
        origin: impl Into<String>,
    ) -> Self {
        let semantic = semantic.into();
        self.bindings
            .entry(semantic)
            .or_default()
            .push(origin.into());
        self
    }

    /// 从环境变量 `APEIRETH_CREDENTIAL_ENDPOINT_BINDINGS` 读取绑定声明。
    ///
    /// 格式：**`凭据名@origin`**，多条用 **`;`** 分隔。
    /// 例：`provider.anthropic.api_key@https://gw.internal;provider.openai-compatible.api_key@https://api.deepseek.com`
    ///
    /// 格式非法的条目**被跳过**（不生效 ⇒ 该端点不会被放行，fail-closed 方向）。
    pub fn with_endpoint_bindings_from_env(mut self) -> Self {
        let raw = std::env::var(ENDPOINT_BINDINGS_ENV).unwrap_or_default();
        for (name, origin) in crate::endpoint_binding::parse_endpoint_bindings(&raw) {
            self = self.with_endpoint_binding(name, origin);
        }
        self
    }

    /// 该凭据已声明的端点（未声明 ⇒ `None`，即「不参与绑定」）。
    pub fn declared_endpoints(&self, semantic: &str) -> Option<&[String]> {
        self.bindings.get(semantic).map(Vec::as_slice)
    }
}

/// 环境变量名：端点绑定声明（`凭据名@origin`，多条 `;` 分隔）。
pub const ENDPOINT_BINDINGS_ENV: &str = "APEIRETH_CREDENTIAL_ENDPOINT_BINDINGS";

impl CredentialResolver for EnvCredentialResolver {
    fn resolve(&self, name: &str) -> Option<Secret> {
        let env_var = self.mappings.get(name)?;
        // Read at call time — the environment a process boots in is the source,
        // never a stored field. An empty value is treated as absent so a default
        // of "" cannot masquerade as a configured key.
        let value = std::env::var(env_var).ok().filter(|v| !v.is_empty())?;
        Some(Secret::new(value))
    }

    /// 已声明绑定 ⇒ `Some(表)`；未声明 ⇒ `None`（**不参与**绑定）。
    ///
    /// ⚠️ `None` 与 `Some(vec![])` 含义不同（见 trait 文档）：
    /// 这里只有**显式声明过**才返回 `Some` ⇒ 未声明的凭据保持既有行为
    /// （协议族 provider 的自定义端点仍可用，厂商 provider 仍由默认端点兜底）。
    fn bound_endpoints(&self, name: &str) -> Option<Vec<String>> {
        self.bindings.get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// `std::env` is process-global and lib tests run in parallel, so every test
    /// that mutates `APEIRETH_API_KEY` (or any shared variable) must take this
    /// lock first. Holding it for the whole test serializes the env-touching
    /// suite without burdening the production type with an injection seam.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Read and restore an env var so concurrent tests do not bleed state.
    struct EnvGuard {
        key: String,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &str, value: Option<&str>) -> Self {
            let prev = std::env::var(key).ok();
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
            Self {
                key: key.to_string(),
                prev,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(&self.key, v),
                None => std::env::remove_var(&self.key),
            }
        }
    }

    #[test]
    fn resolves_the_default_minimax_mapping_from_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(MINIMAX_API_KEY_ENV, Some("sk-from-env-123"));
        let resolver = EnvCredentialResolver::new();
        let secret = resolver.resolve(MINIMAX_API_KEY).expect("mapped + set");
        assert_eq!(secret.expose(), "sk-from-env-123");
    }

    #[test]
    fn an_unset_variable_resolves_to_none() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(MINIMAX_API_KEY_ENV, None);
        let resolver = EnvCredentialResolver::new();
        assert!(resolver.resolve(MINIMAX_API_KEY).is_none());
    }

    #[test]
    fn an_empty_variable_is_treated_as_absent() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(MINIMAX_API_KEY_ENV, Some(""));
        let resolver = EnvCredentialResolver::new();
        assert!(
            resolver.resolve(MINIMAX_API_KEY).is_none(),
            "an empty env value must not masquerade as a configured key"
        );
    }

    #[test]
    fn an_unmapped_name_resolves_to_none() {
        let _lock = ENV_LOCK.lock().unwrap();
        let resolver = EnvCredentialResolver::new();
        assert!(resolver.resolve("provider.unknown.api_key").is_none());
    }

    #[test]
    fn with_mapping_routes_a_second_provider_to_its_own_variable() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g_openai = EnvGuard::set("OPENAI_API_KEY", Some("sk-openai"));
        let _g_minimax = EnvGuard::set(MINIMAX_API_KEY_ENV, Some("sk-minimax"));
        let resolver =
            EnvCredentialResolver::new().with_mapping("provider.openai.api_key", "OPENAI_API_KEY");

        assert_eq!(
            resolver
                .resolve("provider.openai.api_key")
                .map(|s| s.expose().to_string()),
            Some("sk-openai".to_string())
        );
        assert_eq!(
            resolver
                .resolve(MINIMAX_API_KEY)
                .map(|s| s.expose().to_string()),
            Some("sk-minimax".to_string())
        );
        // Introspection returns configuration, not secrets.
        assert_eq!(
            resolver.env_var_for("provider.openai.api_key"),
            Some("OPENAI_API_KEY")
        );
    }

    #[test]
    fn the_resolver_does_not_carry_or_print_secrets() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(MINIMAX_API_KEY_ENV, Some("sk-super-secret-value"));
        let resolver = EnvCredentialResolver::new();
        // The resolver struct itself holds only name→variable strings.
        let printed = format!("{resolver:?}");
        assert!(
            !printed.contains("sk-super-secret-value"),
            "the resolver must not store the secret: {printed}"
        );
        // The resolved Secret is redacted.
        let secret = resolver.resolve(MINIMAX_API_KEY).expect("set");
        assert_eq!(format!("{secret:?}"), "Secret(<redacted>)");
        assert!(!format!("{secret}").contains("sk-super-secret-value"));
    }

    #[test]
    fn default_resolver_maps_each_known_provider_key() {
        let resolver = EnvCredentialResolver::new();
        assert_eq!(
            resolver.env_var_for(MINIMAX_API_KEY),
            Some(MINIMAX_API_KEY_ENV)
        );
        assert_eq!(
            resolver.env_var_for(ANTHROPIC_API_KEY),
            Some(ANTHROPIC_API_KEY_ENV)
        );
        assert_eq!(
            resolver.env_var_for(OPENAI_COMPATIBLE_API_KEY),
            Some(OPENAI_COMPATIBLE_API_KEY_ENV)
        );
        // Unknown semantic ids have no mapping — no catch-all (§20).
        assert!(resolver.env_var_for("provider.openai.api_key").is_none());
        assert!(resolver
            .env_var_for("provider.<anything>.api_key")
            .is_none());
    }

    #[test]
    fn resolves_the_anthropic_mapping_from_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(ANTHROPIC_API_KEY_ENV, Some("sk-ant-env-456"));
        let resolver = EnvCredentialResolver::new();
        let secret = resolver.resolve(ANTHROPIC_API_KEY).expect("mapped + set");
        assert_eq!(secret.expose(), "sk-ant-env-456");
    }

    #[test]
    fn resolves_the_openai_compatible_mapping_from_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(OPENAI_COMPATIBLE_API_KEY_ENV, Some("sk-openai-env-789"));
        let resolver = EnvCredentialResolver::new();
        let secret = resolver
            .resolve(OPENAI_COMPATIBLE_API_KEY)
            .expect("mapped + set");
        assert_eq!(secret.expose(), "sk-openai-env-789");
    }
}
