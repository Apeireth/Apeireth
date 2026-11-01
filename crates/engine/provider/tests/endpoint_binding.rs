//! **端点绑定**的端到端回归（2026-11-01 安全审计 P1）。
//!
//! 要证明的**事实**（不是设计意图）：
//! 1. 厂商 provider（anthropic）的凭据**只**能发往**厂商默认端点**；
//! 2. 把端点改成别的主机 ⇒ **凭据根本取不到**（连请求都没发出去）；
//! 3. 显式**声明绑定**后 ⇒ 自定义端点放行；
//! 4. 协议族 provider（openai-compatible）**未声明绑定时不拦截**（不误伤自建端点），
//!    **声明后强制**。

use std::sync::Arc;

use apeireth_plugin::{CredentialResolver, ProviderCapability, Secret, StaticCredentials};
use apeireth_protocol::canonical::{NormalizedMessage, NormalizedRequest};
use apeireth_provider::canonical_anthropic::{AnthropicProviderPlugin, DEFAULT_BASE_URL};
use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;
use apeireth_provider::credentials::EnvCredentialResolver;
use apeireth_provider::credentials::{ANTHROPIC_API_KEY, OPENAI_COMPATIBLE_API_KEY};

const FAKE_KEY: &str = "sk-ant-fake-test-key-67890";
const MODEL: &str = "MiniMax-M3";

// ============================================================
// 夹具
// ============================================================

/// 一个**声明了绑定**的 resolver（模拟部署方显式声明「这个凭据可以去哪儿」）。
struct BoundResolver {
    inner: StaticCredentials,
    bound: Vec<String>,
}

impl BoundResolver {
    fn new(bound: Vec<String>) -> Self {
        Self {
            inner: StaticCredentials::new().with(ANTHROPIC_API_KEY, FAKE_KEY),
            bound,
        }
    }
}

impl CredentialResolver for BoundResolver {
    fn resolve(&self, name: &str) -> Option<Secret> {
        self.inner.resolve(name)
    }
    fn bound_endpoints(&self, _name: &str) -> Option<Vec<String>> {
        Some(self.bound.clone())
    }
}

fn anthropic_plugin(
    base_url: &str,
    resolver: Arc<dyn CredentialResolver>,
) -> Arc<AnthropicProviderPlugin> {
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        AnthropicProviderPlugin::new(base_url, vec![MODEL.to_string()], http, 500).unwrap(),
    );
    plugin.attach_resolver_for_test(resolver);
    plugin
}

fn openai_plugin(
    base_url: &str,
    resolver: Arc<dyn CredentialResolver>,
) -> Arc<OpenAiCompatibleProviderPlugin> {
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        OpenAiCompatibleProviderPlugin::new(base_url, vec![MODEL.to_string()], http, 500).unwrap(),
    );
    plugin.attach_resolver_for_test(resolver);
    plugin
}

fn request() -> NormalizedRequest {
    NormalizedRequest::new(
        MODEL,
        vec![
            NormalizedMessage::system("be brief"),
            NormalizedMessage::user("say hello"),
        ],
    )
}

// ============================================================
// 测试
// ============================================================

/// 厂商 provider：端点换成别的主机 ⇒ **凭据取不到**（fail-closed）。
#[tokio::test]
async fn anthropic_refuses_to_send_the_credential_to_an_unbound_host() {
    let plugin = anthropic_plugin(
        "https://attacker.example/anthropic",
        Arc::new(StaticCredentials::new().with(ANTHROPIC_API_KEY, FAKE_KEY)),
    );
    // 模拟生产：`from_env()` 会把 audience 设成**厂商默认端点**。
    plugin
        .provider_for_test()
        .set_credential_audience(vec![DEFAULT_BASE_URL.to_string()]);

    let err = plugin
        .provider_for_test()
        .complete(&request())
        .await
        .expect_err("must refuse an unbound host");
    let text = format!("{err:?}");
    assert!(
        text.contains("attacker.example"),
        "error must name the host: {text}"
    );
    assert!(
        !text.contains(FAKE_KEY),
        "error must not contain the credential: {text}"
    );
}

/// 厂商 provider：audience 内的端点（厂商默认）⇒ 绑定**不拦**。
#[tokio::test]
async fn anthropic_allows_the_vendor_default_endpoint() {
    let plugin = anthropic_plugin(
        DEFAULT_BASE_URL,
        Arc::new(StaticCredentials::new().with(ANTHROPIC_API_KEY, FAKE_KEY)),
    );
    plugin
        .provider_for_test()
        .set_credential_audience(vec![DEFAULT_BASE_URL.to_string()]);

    // 断言点：**不是**绑定错误（绑定错误里会出现 "not bound to endpoint"）。
    if let Err(err) = plugin.provider_for_test().complete(&request()).await {
        let text = format!("{err:?}");
        assert!(
            !text.contains("not bound to endpoint"),
            "binding must not block the vendor endpoint: {text}"
        );
    }
}

/// 厂商 provider：**显式声明绑定**后 ⇒ 自定义端点放行。
#[tokio::test]
async fn anthropic_allows_a_custom_endpoint_once_the_binding_is_declared() {
    let plugin = anthropic_plugin(
        "https://gw.internal/anthropic",
        Arc::new(BoundResolver::new(vec!["https://gw.internal".to_string()])),
    );
    plugin
        .provider_for_test()
        .set_credential_audience(vec![DEFAULT_BASE_URL.to_string()]);

    if let Err(err) = plugin.provider_for_test().complete(&request()).await {
        let text = format!("{err:?}");
        assert!(
            !text.contains("not bound to endpoint"),
            "a declared binding must allow the host: {text}"
        );
    }
}

/// 协议族 provider：**未声明绑定时不拦截**（不误伤部署方自选端点）。
#[tokio::test]
async fn protocol_family_provider_is_not_intercepted_without_a_declaration() {
    let plugin = openai_plugin(
        "https://api.deepseek.com/v1",
        Arc::new(StaticCredentials::new().with(OPENAI_COMPATIBLE_API_KEY, FAKE_KEY)),
    );
    if let Err(err) = plugin.provider_for_test().complete(&request()).await {
        let text = format!("{err:?}");
        assert!(
            !text.contains("not bound to endpoint"),
            "protocol family must not be intercepted by default: {text}"
        );
    }
}

/// 协议族 provider：**声明绑定后强制** ⇒ 未绑定的端点被拒。
#[tokio::test]
async fn protocol_family_provider_is_enforced_once_a_binding_is_declared() {
    let plugin = openai_plugin(
        "https://attacker.example/v1",
        Arc::new(BoundResolver::new(vec![
            "https://api.deepseek.com".to_string()
        ])),
    );
    let err = plugin
        .provider_for_test()
        .complete(&request())
        .await
        .expect_err("must refuse an unbound host");
    let text = format!("{err:?}");
    assert!(
        text.contains("attacker.example"),
        "error must name the host: {text}"
    );
}

/// 绑定声明的解析口径：没声明 ⇒ 不参与绑定（`None`）。
#[test]
fn binding_declarations_are_absent_unless_declared() {
    let resolver = EnvCredentialResolver::new().with_endpoint_bindings_from_env();
    assert!(resolver.declared_endpoints(ANTHROPIC_API_KEY).is_none());
}
