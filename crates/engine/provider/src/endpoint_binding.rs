//! **凭据 ↔ 端点绑定**（2026-11-01 安全审计 P1 的落点）。
//!
//! # 要解决的泄漏路径（实测形态）
//!
//! ```text
//! 凭据来源（三选一，独立配置）        端点来源（独立配置）
//!   env / keyring / 运行时热写   ←→     APEIRETH_*_URL
//! ```
//!
//! 只改**端点**那一份 ⇒ **旧凭据被发往新主机**（厂商 key 出境）。
//! 典型受害面：`canonical_anthropic.rs`（默认端点 `api.minimaxi.com`，
//! 凭据走 `x-api-key` 头）与 `canonical_openai_compatible.rs`（协议族，端点由部署方选）。
//!
//! # 判定口径
//!
//! 两级信任模型（因为**协议族** provider 的端点本来就由部署方选，没有唯一「默认端点」）：
//!
//! 允许集合 follows the credential revision:
//! * **audience** = 构造时确定的「这个凭据该去哪些端点」：
//!   厂商 provider（anthropic / minimax）= **厂商默认端点** ⇒ 换端点**默认拒绝**；
//!   协议族 provider（openai-compatible）= 空（端点本就由部署方选）⇒ 保持既有行为。
//! * **显式绑定** = 部署方声明（凭据库热写：`HotCredentialStore::set_with_endpoints`）
//!   ⇒ 它属于这版凭据并**替代** vendor audience，不能靠默认端点把显式绑定加宽。
//! * 两者皆空 ⇒ **不拦截**；否则端点不在集合内 ⇒ **拒绝**（fail-closed）。
//!
//! # 绑定从哪来
//!
//! 绑定与值由 [`CredentialResolver::resolve_snapshot`] 同笔提供：
//! * [`None`] = 该 resolver **不参与**绑定（既有实现者）⇒ 不做端点拦截；
//! * [`Some(list)`] = 该 resolver 参与绑定，`list` 是该凭据**全部**被授权的 origin。
//!
//! # 0 装边界（如实）
//!
//! * 对**协议族** provider，**不声明绑定时仍不拦截** —— 因为「部署方选的端点」
//!   没有可推导的默认值，硬判会把正常部署全部打死。
//!   ⇒ 想要拦截就**显式声明**绑定（凭据库热写）。
//! * 绑定是**部署方的显式声明**，不是密码学证明：它防的是「**只改端点**就泄漏」，
//!   不防「能同时改端点与绑定声明」的对手（那种对手本来就能改凭据）。

use apeireth_plugin::{CredentialResolver, ProviderError, Secret};

/// Resolve one coherent credential snapshot and authorize it for `base_url`.
///
/// An explicit binding is authoritative for that credential revision and
/// replaces the vendor fallback. Without a declaration, vendor providers use
/// their audience and protocol-family providers retain documented unmanaged
/// legacy behavior.
pub fn resolve_credential_for_endpoint(
    resolver: &dyn CredentialResolver,
    provider_id: &str,
    credential_name: &str,
    base_url: &str,
    audience: Option<&[String]>,
) -> Result<Secret, ProviderError> {
    let origin = endpoint_origin(base_url).ok_or_else(|| ProviderError::AuthFailed {
        provider: provider_id.to_string(),
        detail: "endpoint is not a parseable absolute URL; refusing to attach the credential"
            .to_string(),
    })?;
    let snapshot =
        resolver
            .resolve_snapshot(credential_name)
            .ok_or_else(|| ProviderError::AuthFailed {
                provider: provider_id.to_string(),
                detail: format!("missing API key for {credential_name}"),
            })?;

    let declared = match snapshot.bound_endpoints() {
        Some(endpoints) => Some(endpoints),
        None => audience,
    };
    let Some(declared) = declared else {
        return Ok(snapshot.into_secret());
    };
    let allowed: Vec<String> = declared
        .iter()
        .filter_map(|raw| endpoint_origin(raw))
        .collect();

    if allowed.iter().any(|candidate| candidate == &origin) {
        Ok(snapshot.into_secret())
    } else {
        Err(ProviderError::AuthFailed {
            provider: provider_id.to_string(),
            detail: format!(
                "credential {credential_name} is not bound to endpoint {origin}; allowed endpoints: {}. \
                 Refusing to send the credential to an unbound host. Declare the binding \
                 explicitly (credential-store level) to use a custom endpoint.",
                if allowed.is_empty() {
                    "<none declared>".to_string()
                } else {
                    allowed.join(", ")
                }
            ),
        })
    }
}

/// 把 URL 归一成 **origin**（scheme://host[:port]），用于端点比较。
///
/// * 端口：默认端口归一化掉；非默认显式端口保留。
/// * 大小写：scheme 与 host **小写化**（host 大小写不敏感）。
/// * 路径 / 查询 / 片段：**丢弃**（比较的是端点，不是资源）。
/// * 用户信息（`user:pass@`）：**拒绝**，避免验证目标与请求认证语义分叉。
/// * 无法解析 / 无 host ⇒ [`None`]（调用方据此 fail-closed）。
pub fn endpoint_origin(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let authority = trimmed.split_once("://")?.1.split(['/', '?', '#']).next()?;
    if authority.contains('@') {
        return None;
    }
    let parsed = reqwest::Url::parse(trimmed).ok()?;
    let scheme = parsed.scheme();
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return None;
    }
    let raw_host = parsed.host_str()?.to_ascii_lowercase();
    let host = if raw_host.starts_with('[') && raw_host.ends_with(']') {
        raw_host
    } else if raw_host.contains(':') {
        format!("[{raw_host}]")
    } else {
        raw_host
    };
    let port = parsed
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    Some(format!("{scheme}://{host}{port}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct BoundResolver(BTreeMap<String, Vec<String>>);

    impl CredentialResolver for BoundResolver {
        fn resolve(&self, _name: &str) -> Option<Secret> {
            Some(Secret::new("synthetic-secret"))
        }

        fn bound_endpoints(&self, name: &str) -> Option<Vec<String>> {
            self.0.get(name).cloned()
        }
    }

    fn vendor_audience() -> Vec<String> {
        vec!["https://api.minimaxi.com/anthropic".to_string()]
    }

    #[test]
    fn origin_normalizes_case_path_and_default_port() {
        assert_eq!(
            endpoint_origin("https://API.Example.com/v1/chat"),
            Some("https://api.example.com".to_string())
        );
        assert_eq!(
            endpoint_origin("https://api.example.com:443/v1"),
            Some("https://api.example.com".to_string())
        );
        assert_eq!(
            endpoint_origin("http://127.0.0.1:8080"),
            Some("http://127.0.0.1:8080".to_string())
        );
        assert_eq!(
            endpoint_origin("http://[::1]:8080/v1"),
            Some("http://[::1]:8080".to_string())
        );
    }

    #[test]
    fn origin_rejects_unparseable_urls() {
        assert_eq!(endpoint_origin(""), None);
        assert_eq!(endpoint_origin("   "), None);
        assert_eq!(endpoint_origin("api.example.com"), None);
        assert_eq!(endpoint_origin("https://"), None);
        assert_eq!(endpoint_origin("https://host:notaport/x"), None);
        assert_eq!(endpoint_origin("https://host:99999/x"), None);
        assert_eq!(endpoint_origin("https://[::1]garbage/x"), None);
        assert_eq!(endpoint_origin("https://user:pass@api.example.com/x"), None);
        assert_eq!(endpoint_origin("https://@api.example.com/x"), None);
        assert_eq!(endpoint_origin("ftp://api.example.com/key"), None);
    }

    #[test]
    fn vendor_default_endpoint_is_always_allowed() {
        let resolver = BoundResolver(BTreeMap::new());
        let resolved = resolve_credential_for_endpoint(
            &resolver,
            "anthropic",
            "provider.anthropic.api_key",
            "https://api.minimaxi.com/anthropic",
            Some(&vendor_audience()),
        )
        .expect("vendor default allowed");
        assert_eq!(resolved.expose(), "synthetic-secret");
    }

    #[test]
    fn vendor_provider_refuses_an_unbound_custom_endpoint() {
        let resolver = BoundResolver(BTreeMap::new());
        let err = resolve_credential_for_endpoint(
            &resolver,
            "anthropic",
            "provider.anthropic.api_key",
            "https://attacker.example/v1",
            Some(&vendor_audience()),
        )
        .expect_err("must refuse");
        let text = format!("{err:?}");
        assert!(
            text.contains("attacker.example"),
            "diagnostic names the host: {text}"
        );
        assert!(!text.contains("sk-"), "no credential material in the error");
    }

    #[test]
    fn explicitly_bound_custom_endpoint_is_allowed() {
        let mut map = BTreeMap::new();
        map.insert(
            "provider.anthropic.api_key".to_string(),
            vec!["https://gw.internal".to_string()],
        );
        let resolver = BoundResolver(map);
        let resolved = resolve_credential_for_endpoint(
            &resolver,
            "anthropic",
            "provider.anthropic.api_key",
            "https://gw.internal/anthropic",
            Some(&vendor_audience()),
        )
        .expect("bound endpoint allowed");
        assert_eq!(resolved.expose(), "synthetic-secret");
    }

    #[test]
    fn bound_list_does_not_cover_other_hosts() {
        let mut map = BTreeMap::new();
        map.insert(
            "provider.anthropic.api_key".to_string(),
            vec!["https://gw.internal".to_string()],
        );
        let resolver = BoundResolver(map);
        let err = resolve_credential_for_endpoint(
            &resolver,
            "anthropic",
            "provider.anthropic.api_key",
            "https://attacker.example",
            Some(&vendor_audience()),
        )
        .expect_err("must refuse");
        assert!(format!("{err:?}").contains("attacker.example"));
    }

    #[test]
    fn protocol_family_provider_is_not_intercepted_without_a_declaration() {
        let resolver = BoundResolver(BTreeMap::new());
        let resolved = resolve_credential_for_endpoint(
            &resolver,
            "openai-compatible",
            "provider.openai-compatible.api_key",
            "https://api.deepseek.com/v1",
            None,
        )
        .expect("not intercepted");
        assert_eq!(resolved.expose(), "synthetic-secret");
    }

    #[test]
    fn protocol_family_provider_is_enforced_once_declared() {
        let mut map = BTreeMap::new();
        map.insert(
            "provider.openai-compatible.api_key".to_string(),
            vec!["https://api.deepseek.com".to_string()],
        );
        let resolver = BoundResolver(map);
        assert!(resolve_credential_for_endpoint(
            &resolver,
            "openai-compatible",
            "provider.openai-compatible.api_key",
            "https://api.deepseek.com/v1",
            None,
        )
        .is_ok());
        assert!(resolve_credential_for_endpoint(
            &resolver,
            "openai-compatible",
            "provider.openai-compatible.api_key",
            "https://attacker.example/v1",
            None,
        )
        .is_err());
    }

    #[test]
    fn unparseable_endpoint_is_refused_even_without_an_audience() {
        let resolver = BoundResolver(BTreeMap::new());
        assert!(resolve_credential_for_endpoint(
            &resolver,
            "anthropic",
            "provider.anthropic.api_key",
            "not-a-url",
            None,
        )
        .is_err());
    }

    #[test]
    fn explicit_binding_replaces_instead_of_widens_vendor_audience() {
        let mut map = BTreeMap::new();
        map.insert(
            "provider.anthropic.api_key".to_string(),
            vec!["https://gw.internal".to_string()],
        );
        let resolver = BoundResolver(map);
        assert!(resolve_credential_for_endpoint(
            &resolver,
            "anthropic",
            "provider.anthropic.api_key",
            "https://api.minimaxi.com/anthropic",
            Some(&vendor_audience()),
        )
        .is_err());
    }
}
