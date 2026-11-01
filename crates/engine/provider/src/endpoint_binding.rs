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
//! 允许集合 = **audience ∪ 显式绑定**：
//! * **audience** = 构造时确定的「这个凭据该去哪些端点」：
//!   厂商 provider（anthropic / minimax）= **厂商默认端点** ⇒ 换端点**默认拒绝**；
//!   协议族 provider（openai-compatible）= 空（端点本就由部署方选）⇒ 保持既有行为。
//! * **显式绑定** = 部署方声明（凭据库热写：`HotCredentialStore::set_with_endpoints`）
//!   ⇒ **加宽** audience。
//! * 两者皆空 ⇒ **不拦截**；否则端点不在集合内 ⇒ **拒绝**（fail-closed）。
//!
//! # 绑定从哪来
//!
//! 绑定由 [`CredentialResolver::bound_endpoints`] 提供（见 `apeireth_plugin::credentials`）：
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

use apeireth_plugin::CredentialResolver;
use apeireth_plugin::ProviderError;

/// 端点绑定检查的结果（供日志/错误信息使用；**不含任何凭据内容**）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointBindingVerdict {
    /// 端点落在该凭据的 **audience**（构造时确定的「这个凭据该去哪儿」）内 ⇒ 放行。
    InAudience,
    /// 端点被**显式绑定**给该凭据 ⇒ 放行。
    ExplicitlyBound,
    /// 既没有 audience、resolver 也不参与绑定 ⇒ **未做端点拦截**。
    Unmanaged,
}

/// 判定「这个凭据**可以**被发往这个端点吗」。
///
/// # 口径（并集规则）
///
/// * `audience` = 构造时确定的「这个凭据**该去**哪些端点」（可空）。
///   · **厂商 provider**（anthropic / minimax）⇒ audience = **厂商默认端点**
///     ⇒ 换到别的端点**默认拒绝**（fail-closed）—— 这是安全审计 P1 的核心。
///   · **协议族 provider**（openai-compatible）⇒ audience = 空（端点本就由部署方选）
///     ⇒ 保持既有行为，不误伤自建端点。
/// * `resolver.bound_endpoints(cred)` = 部署方**显式声明**的绑定
///   （凭据库热写）⇒ **加宽** audience。
///
/// 最终允许集合 = `audience ∪ 显式绑定`：
/// * 两者皆空 ⇒ **不拦截**（`Unmanaged`，保持既有行为）。
/// * 否则端点必须在集合内 ⇒ 否则**拒绝**（fail-closed）。
///
/// # 参数
///
/// * `resolver` —— 取凭据的 resolver（**同一个** resolver ⇒ 绑定声明与凭据同源）。
/// * `credential_name` —— 逻辑凭据名（如 `provider.anthropic.api_key`）。
/// * `base_url` —— 本次请求要用的**基址**（端点）。
/// * `audience` —— 构造时确定的允许端点集（可空）。
///
/// # 失败时
///
/// 返回 [`ProviderError::AuthFailed`]，**错误信息里只有端点与凭据名，没有凭据内容**
/// （凭据名不是秘密；值才是）。
pub fn ensure_credential_bound_to_endpoint(
    resolver: &dyn CredentialResolver,
    credential_name: &str,
    base_url: &str,
    audience: Option<&[String]>,
) -> Result<EndpointBindingVerdict, ProviderError> {
    let origin = endpoint_origin(base_url).ok_or_else(|| ProviderError::AuthFailed {
        provider: credential_name.to_string(),
        detail: "endpoint is not a parseable absolute URL; refusing to attach the credential"
            .to_string(),
    })?;

    let mut allowed: Vec<String> = Vec::new();
    let mut managed = false;
    if let Some(audience) = audience {
        managed = true;
        allowed.extend(audience.iter().cloned());
    }
    if let Some(list) = resolver.bound_endpoints(credential_name) {
        managed = true;
        allowed.extend(list);
    }

    if !managed {
        return Ok(EndpointBindingVerdict::Unmanaged);
    }

    let hit = allowed
        .iter()
        .filter_map(|raw| endpoint_origin(raw))
        .any(|candidate| candidate == origin);
    if hit {
        Ok(EndpointBindingVerdict::InAudience)
    } else {
        Err(ProviderError::AuthFailed {
            provider: credential_name.to_string(),
            detail: format!(
                "credential is not bound to endpoint {origin}; allowed endpoints: {}. \
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
/// * 端口：显式端口**保留**（`:8443` 与默认端口**不是**同一个 origin）。
/// * 大小写：scheme 与 host **小写化**（host 大小写不敏感）。
/// * 路径 / 查询 / 片段：**丢弃**（比较的是端点，不是资源）。
/// * 用户信息（`user:pass@`）：**丢弃**（不参与 origin 比较，也避免把凭据带进比较串）。
/// * 无法解析 / 无 host ⇒ [`None`]（调用方据此 fail-closed）。
pub fn endpoint_origin(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (scheme, rest) = trimmed.split_once("://")?;
    let scheme = scheme.trim().to_ascii_lowercase();
    if scheme.is_empty()
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    {
        return None;
    }
    // 去掉 userinfo
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default();
    if authority.is_empty() {
        return None;
    }
    // IPv6 字面量：`[::1]:8443`
    let (host, port) = if let Some(stripped) = authority.strip_prefix('[') {
        let (host_part, tail) = stripped.split_once(']')?;
        let port = tail.strip_prefix(':').map(str::to_string);
        (host_part.to_string(), port)
    } else {
        match authority.split_once(':') {
            Some((host, port)) => (host.to_string(), Some(port.to_string())),
            None => (authority.to_string(), None),
        }
    };
    if host.is_empty() {
        return None;
    }
    let host = host.to_ascii_lowercase();
    let port = match port {
        Some(p) if p.is_empty() => return None,
        Some(p) => {
            if !p.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            format!(":{p}")
        }
        None => String::new(),
    };
    Some(format!("{scheme}://{host}{port}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_plugin::Secret;
    use std::collections::BTreeMap;

    struct BoundResolver(BTreeMap<String, Vec<String>>);

    impl CredentialResolver for BoundResolver {
        fn resolve(&self, _name: &str) -> Option<Secret> {
            None
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
            Some("https://api.example.com:443".to_string())
        );
        assert_eq!(
            endpoint_origin("http://127.0.0.1:8080"),
            Some("http://127.0.0.1:8080".to_string())
        );
        assert_eq!(
            endpoint_origin("https://user:pass@api.example.com/x"),
            Some("https://api.example.com".to_string())
        );
    }

    #[test]
    fn origin_rejects_unparseable_urls() {
        assert_eq!(endpoint_origin(""), None);
        assert_eq!(endpoint_origin("   "), None);
        assert_eq!(endpoint_origin("api.example.com"), None);
        assert_eq!(endpoint_origin("https://"), None);
        assert_eq!(endpoint_origin("https://host:notaport/x"), None);
    }

    #[test]
    fn vendor_default_endpoint_is_always_allowed() {
        let resolver = BoundResolver(BTreeMap::new());
        let verdict = ensure_credential_bound_to_endpoint(
            &resolver,
            "provider.anthropic.api_key",
            "https://api.minimaxi.com/anthropic",
            Some(&vendor_audience()),
        )
        .expect("vendor default allowed");
        assert_eq!(verdict, EndpointBindingVerdict::InAudience);
    }

    #[test]
    fn vendor_provider_refuses_an_unbound_custom_endpoint() {
        let resolver = BoundResolver(BTreeMap::new());
        let err = ensure_credential_bound_to_endpoint(
            &resolver,
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
        let verdict = ensure_credential_bound_to_endpoint(
            &resolver,
            "provider.anthropic.api_key",
            "https://gw.internal/anthropic",
            Some(&vendor_audience()),
        )
        .expect("bound endpoint allowed");
        assert_eq!(verdict, EndpointBindingVerdict::InAudience);
    }

    #[test]
    fn bound_list_does_not_cover_other_hosts() {
        let mut map = BTreeMap::new();
        map.insert(
            "provider.anthropic.api_key".to_string(),
            vec!["https://gw.internal".to_string()],
        );
        let resolver = BoundResolver(map);
        let err = ensure_credential_bound_to_endpoint(
            &resolver,
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
        let verdict = ensure_credential_bound_to_endpoint(
            &resolver,
            "provider.openai-compatible.api_key",
            "https://api.deepseek.com/v1",
            None,
        )
        .expect("not intercepted");
        assert_eq!(verdict, EndpointBindingVerdict::Unmanaged);
    }

    #[test]
    fn protocol_family_provider_is_enforced_once_declared() {
        let mut map = BTreeMap::new();
        map.insert(
            "provider.openai-compatible.api_key".to_string(),
            vec!["https://api.deepseek.com".to_string()],
        );
        let resolver = BoundResolver(map);
        assert!(ensure_credential_bound_to_endpoint(
            &resolver,
            "provider.openai-compatible.api_key",
            "https://api.deepseek.com/v1",
            None,
        )
        .is_ok());
        assert!(ensure_credential_bound_to_endpoint(
            &resolver,
            "provider.openai-compatible.api_key",
            "https://attacker.example/v1",
            None,
        )
        .is_err());
    }

    #[test]
    fn unparseable_endpoint_is_refused_even_without_an_audience() {
        let resolver = BoundResolver(BTreeMap::new());
        assert!(ensure_credential_bound_to_endpoint(
            &resolver,
            "provider.anthropic.api_key",
            "not-a-url",
            None,
        )
        .is_err());
    }
}
