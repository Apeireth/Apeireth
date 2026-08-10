//! # apeireth-credentials demo
//!
//! 演示 5 Provider 切换 + 5 鉴权 + 4 轮换策略 + 5 Scope + 4 audit 事件.
//! R20 阶段 6 skeleton 阶段: 所有 Provider 操作返 `NotImplemented` (R21+ 真接).
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-credentials --example credentials_demo
//! ```

use apeireth_credentials::{
    audit::{AuditEvent, AuditEventKind, AuditLog},
    auth::AuthMethod,
    make_provider,
    provider::{AnthropicProvider, AzureProvider, GoogleProvider, LocalProvider, OpenAIProvider},
    rotation::{count_default, hybrid_default, time_default, RotationStrategy},
    scope::Scope,
    validate_tool_call, CredentialsConfig, CredentialsManager, K1_STRONG_VALIDATION_VARIANTS,
    AUDIT_EVENT_COUNT, AUTH_METHOD_COUNT, CREDENTIALS_SCHEMA_VERSION, PLATFORM_NAME,
    PROVIDER_COUNT, ROTATION_STRATEGY_COUNT, SCOPE_COUNT, STUB_MODE, TOOL_WHITELIST,
    TOOL_WHITELIST_COUNT,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== apeireth-credentials demo (R20 阶段 6 skeleton) ===");
    println!();

    // ---- §1 编译期 hardcode 常量 ----
    println!("[§1 编译期 hardcode 常量]");
    println!("  CREDENTIALS_SCHEMA_VERSION = {CREDENTIALS_SCHEMA_VERSION}");
    println!("  PLATFORM_NAME               = {PLATFORM_NAME}");
    println!("  STUB_MODE                   = {STUB_MODE}");
    println!("  PROVIDER_COUNT              = {PROVIDER_COUNT}");
    println!("  AUTH_METHOD_COUNT           = {AUTH_METHOD_COUNT}");
    println!("  SCOPE_COUNT                 = {SCOPE_COUNT}");
    println!("  ROTATION_STRATEGY_COUNT     = {ROTATION_STRATEGY_COUNT}");
    println!("  AUDIT_EVENT_COUNT           = {AUDIT_EVENT_COUNT}");
    println!(
        "  K-1 强校验 ({} 项): {}",
        K1_STRONG_VALIDATION_VARIANTS.len(),
        K1_STRONG_VALIDATION_VARIANTS.join(", ")
    );
    println!();

    // ---- §2 m3 防御: 8 工具白名单 ----
    println!("[§2 m3 防御: 8 工具白名单]");
    println!("  TOOL_WHITELIST_COUNT = {TOOL_WHITELIST_COUNT}");
    for (i, tool) in TOOL_WHITELIST.iter().enumerate() {
        println!("  [{:>2}] {}", i + 1, tool);
    }
    let valid = validate_tool_call("apeireth_credentials_get_token", &serde_json::json!({}));
    let invalid = validate_tool_call("apeireth_credentials_bogus", &serde_json::json!({}));
    println!("  白名单内工具: {valid:?}");
    println!("  非白名单工具: {invalid:?}");
    println!();

    // ---- §3 5 Provider 切换 ----
    println!("[§3 5 Provider 切换 + 5 鉴权 + 5 K-1 强校验]");
    let api_key = AuthMethod::ApiKey {
        api_key: "sk-test-demo".to_string(),
    };
    for (kind_name, kind) in [
        ("Anthropic", apeireth_credentials::provider::ProviderKind::Anthropic),
        ("OpenAI", apeireth_credentials::provider::ProviderKind::OpenAI),
        ("Google", apeireth_credentials::provider::ProviderKind::Google),
        ("Azure", apeireth_credentials::provider::ProviderKind::Azure),
        ("Local", apeireth_credentials::provider::ProviderKind::Local),
    ] {
        let p = make_provider(kind, api_key.clone()).expect("make provider");
        let _ = p; // 抑制 unused 警告
        println!("  {kind_name:<10} | auth={} | header={} | base={}", api_key.type_name(), kind.auth_header_name(), kind.api_base_url());
    }
    println!();

    // ---- §4 5 鉴权方式 K-1 强校验 ----
    println!("[§4 5 鉴权方式 K-1 强校验]");
    let empty_api_key = AuthMethod::ApiKey { api_key: String::new() };
    let err = empty_api_key.validate();
    println!("  EmptyApiKey 校验: {err:?}");

    let empty_oauth = AuthMethod::OAuth2 {
        client_id: String::new(),
        client_secret: None,
        refresh_token: None,
        access_token: None,
        token_url: "https://oauth.example.com/token".to_string(),
    };
    let err = empty_oauth.validate();
    println!("  EmptyOAuthClientId 校验: {err:?}");

    let empty_jwt = AuthMethod::Jwt {
        token: "eyJ".to_string(),
        audience: String::new(),
        issuer: None,
        expires_at_unix: None,
    };
    let err = empty_jwt.validate();
    println!("  EmptyJwtAudience 校验: {err:?}");

    let empty_iam = AuthMethod::Iam {
        role_arn: String::new(),
        session_name: None,
        region: None,
        access_key_id: "AKIA".to_string(),
        secret_access_key: "secret".to_string(),
    };
    let err = empty_iam.validate();
    println!("  EmptyIamRole 校验: {err:?}");

    let invalid_mtls = AuthMethod::Mtls {
        cert_path: std::path::PathBuf::from("/nonexistent/cert.pem"),
        key_path: std::path::PathBuf::from("/nonexistent/key.pem"),
        ca_path: None,
        server_name: "api.example.com".to_string(),
    };
    let err = invalid_mtls.validate();
    println!("  InvalidMtlsCertPath 校验: {err:?}");
    println!();

    // ---- §5 5 Scope 演示 ----
    println!("[§5 5 Scope 演示 (越权检查)]");
    for scope in Scope::all() {
        let required = Scope::Write;
        let can_do = Scope::can_perform(scope, required);
        println!("  {scope:<7} 试图 {required:<7}: {can_do:?}");
    }
    println!();

    // ---- §6 4 轮换策略演示 ----
    println!("[§6 4 轮换策略演示 (should_rotate)]");
    let now = chrono::Utc::now();
    let recent = now - chrono::Duration::days(15);
    for strategy in [
        RotationStrategy::Manual,
        time_default(),
        count_default(),
        hybrid_default(),
    ] {
        let desc = strategy.describe();
        let should = strategy.should_rotate(Some(recent), 500);
        println!("  {desc:<55} | should_rotate(15d, 500) = {should}");
    }
    println!();

    // ---- §7 CredentialsManager 5 Provider 集成 ----
    println!("[§7 CredentialsManager 5 Provider 集成 + audit 记录]");
    let mut mgr = CredentialsManager::with_options(Scope::Owner, time_default(), 100)
        .expect("with_options");
    mgr.add_provider(
        apeireth_credentials::provider::ProviderKind::Anthropic,
        Box::new(AnthropicProvider::new(api_key.clone()).expect("new")),
    )
    .expect("add");
    mgr.add_provider(
        apeireth_credentials::provider::ProviderKind::OpenAI,
        Box::new(OpenAIProvider::new(api_key.clone()).expect("new")),
    )
    .expect("add");
    mgr.add_provider(
        apeireth_credentials::provider::ProviderKind::Google,
        Box::new(GoogleProvider::new(api_key.clone()).expect("new")),
    )
    .expect("add");
    mgr.add_provider(
        apeireth_credentials::provider::ProviderKind::Azure,
        Box::new(AzureProvider::new(api_key.clone(), "myresource").expect("new")),
    )
    .expect("add");
    mgr.add_provider(
        apeireth_credentials::provider::ProviderKind::Local,
        Box::new(LocalProvider::new(api_key, "http://localhost:8080").expect("new")),
    )
    .expect("add");
    println!("  已注册 {} 个 Provider: {:?}", mgr.list_providers().len(), mgr.list_providers());
    let get_result = mgr.get_token(apeireth_credentials::provider::ProviderKind::Anthropic).await;
    println!("  Anthropic get_token: {get_result:?} (skeleton 返 NotImplemented)");
    let events = mgr.audit_log().list_by_kind(AuditEventKind::Get).await;
    println!("  audit 记录 Get 事件数: {}", events.len());
    println!();

    // ---- §8 AuditLog 4 事件 ----
    println!("[§8 AuditLog 4 事件演示 (P0 安全铁律守门)]");
    let log = AuditLog::new();
    log.record(AuditEvent::get(apeireth_credentials::provider::ProviderKind::Anthropic, "demo-user"))
        .await
        .expect("record get");
    log.record(AuditEvent::put(apeireth_credentials::provider::ProviderKind::OpenAI, "demo-user", "keyring"))
        .await
        .expect("record put");
    log.record(AuditEvent::rotate(
        apeireth_credentials::provider::ProviderKind::Google,
        "demo-service",
        hybrid_default(),
        Some(chrono::Utc::now() - chrono::Duration::days(1)),
        Some(chrono::Utc::now() + chrono::Duration::days(30)),
    ))
    .await
    .expect("record rotate");
    log.record(AuditEvent::revoke(
        apeireth_credentials::provider::ProviderKind::Azure,
        "demo-admin",
        "key rotation overdue",
    ))
    .await
    .expect("record revoke");
    println!("  Get    事件: {}", log.list_by_kind(AuditEventKind::Get).await.len());
    println!("  Put    事件: {}", log.list_by_kind(AuditEventKind::Put).await.len());
    println!("  Rotate 事件: {}", log.list_by_kind(AuditEventKind::Rotate).await.len());
    println!("  Revoke 事件: {}", log.list_by_kind(AuditEventKind::Revoke).await.len());
    // P0 安全铁律: 凭证值不能进 audit
    let bad = AuditEvent::new(
        AuditEventKind::Get,
        apeireth_credentials::provider::ProviderKind::Anthropic,
        "demo",
        "leaked token=sk-12345",
    );
    let err = log.record(bad).await;
    println!("  P0 安全铁律 (含 'token=') → {err:?}");
    println!();

    // ---- §9 CredentialsConfig 演示 ----
    println!("[§9 CredentialsConfig 演示 (serde 兼容)]");
    let cfg = CredentialsConfig::default_config();
    let json = serde_json::to_string(&cfg).expect("serialize");
    println!("  default config JSON: {json}");
    cfg.validate().expect("valid");
    println!("  validate: OK");
    println!();

    println!("=== demo 完成 (R20 阶段 6 skeleton) ===");
    println!("注: 所有 Provider 操作返 `NotImplemented`, R21+ 真接商业版 SDK 后返真值.");
}
