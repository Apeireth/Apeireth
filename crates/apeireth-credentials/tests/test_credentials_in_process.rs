//! # 凭证集成测试 (25+ 测试)
//!
//! 1:1 翻译 v0.9.21 商业版 `out/main` 集成测试, 验证 5 Provider + 5 鉴权 + 4 轮换 +
//! 5 Scope + 4 audit 事件 + CredentialsManager 的 in-process 行为.
//!
//! ## 测试组织 (per task spec §8)
//!
//! - §1 编译期常量守门 (5 测试)
//! - §2 5 Provider (5+1 测试)
//! - §3 5 鉴权方式 + K-1 强校验 (5+5 测试)
//! - §4 5 Scope (1 测试)
//! - §5 4 轮换策略 (4 测试)
//! - §6 4 audit 事件 (4 测试)
//! - §7 CredentialsManager 集成 (5 测试)
//! - §8 m3 防御 (1 测试)
//! - **总计: 30+ 测试**

use apeireth_credentials::{
    audit::{AuditEvent, AuditEventKind, AuditLog},
    auth::AuthMethod,
    error::{CredentialsError, ErrorCategory},
    provider::{
        AnthropicProvider, AzureProvider, GoogleProvider, LocalProvider, OpenAIProvider, Provider,
        ProviderKind, SecretString,
    },
    rotation::{count_default, hybrid_default, time_default, RotationStrategy},
    scope::Scope,
    token::{ProviderTokenManager, TokenManager},
    validate_tool_call, CredentialsConfig, CredentialsManager, K1_STRONG_VALIDATION_VARIANTS,
    AUDIT_EVENT_COUNT, AUTH_METHOD_COUNT, CREDENTIALS_SCHEMA_VERSION, PLATFORM_NAME,
    PROVIDER_COUNT, ROTATION_STRATEGY_COUNT, SCOPE_COUNT, STUB_MODE, TOOL_WHITELIST,
    TOOL_WHITELIST_COUNT,
};

// ============================================================================
// §1 编译期常量守门 (5 测试)
// ============================================================================

#[test]
fn test_constants_provider_count_5() {
    assert_eq!(PROVIDER_COUNT, 5);
    assert_eq!(ProviderKind::all().len(), 5);
    assert_eq!(ProviderKind::COUNT, 5);
}

#[test]
fn test_constants_auth_method_count_5() {
    assert_eq!(AUTH_METHOD_COUNT, 5);
    assert_eq!(AuthMethod::ALL_TYPE_NAMES.len(), 5);
    assert_eq!(AuthMethod::all_type_names().len(), 5);
}

#[test]
fn test_constants_scope_count_5() {
    assert_eq!(SCOPE_COUNT, 5);
    assert_eq!(apeireth_credentials::scope::ALL_SCOPES.len(), 5);
    assert_eq!(apeireth_credentials::scope::SCOPE_COUNT, 5);
}

#[test]
fn test_constants_rotation_count_4() {
    assert_eq!(ROTATION_STRATEGY_COUNT, 4);
    assert_eq!(RotationStrategy::ALL_NAMES.len(), 4);
}

#[test]
fn test_constants_audit_count_4() {
    assert_eq!(AUDIT_EVENT_COUNT, 4);
    assert_eq!(AuditEventKind::all().len(), 4);
    assert_eq!(AuditEventKind::ALL_NAMES.len(), 4);
}

#[test]
fn test_constants_platform_name() {
    assert_eq!(PLATFORM_NAME, "apeireth");
    assert_eq!(CREDENTIALS_SCHEMA_VERSION, "1");
    let _ = STUB_MODE;
}

// ============================================================================
// §2 5 Provider (6 测试)
// ============================================================================

#[test]
fn test_provider_5_enums() {
    let all = ProviderKind::all();
    assert_eq!(all.len(), 5);
    assert!(all.contains(&ProviderKind::Anthropic));
    assert!(all.contains(&ProviderKind::OpenAI));
    assert!(all.contains(&ProviderKind::Google));
    assert!(all.contains(&ProviderKind::Azure));
    assert!(all.contains(&ProviderKind::Local));
}

#[tokio::test]
async fn test_anthropic_provider_not_implemented() {
    let p = AnthropicProvider::new(fixture_api_key()).expect("new");
    assert!(matches!(p.get_token().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(p.refresh().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(p.revoke().await, Err(CredentialsError::NotImplemented(_))));
    assert_eq!(p.kind(), ProviderKind::Anthropic);
    assert!(!p.is_valid().await);
    assert!(p.expires_at().await.is_none());
}

#[tokio::test]
async fn test_openai_provider_not_implemented() {
    let p = OpenAIProvider::new(fixture_api_key()).expect("new");
    assert!(matches!(p.get_token().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(p.refresh().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(p.revoke().await, Err(CredentialsError::NotImplemented(_))));
    assert_eq!(p.kind(), ProviderKind::OpenAI);
}

#[tokio::test]
async fn test_google_provider_not_implemented() {
    let p = GoogleProvider::new(fixture_api_key()).expect("new");
    assert!(matches!(p.get_token().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(p.refresh().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(p.revoke().await, Err(CredentialsError::NotImplemented(_))));
    assert_eq!(p.kind(), ProviderKind::Google);
}

#[tokio::test]
async fn test_azure_provider_not_implemented() {
    let p = AzureProvider::new(fixture_api_key(), "myresource").expect("new");
    assert!(matches!(p.get_token().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(p.refresh().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(p.revoke().await, Err(CredentialsError::NotImplemented(_))));
    assert_eq!(p.kind(), ProviderKind::Azure);
}

#[tokio::test]
async fn test_local_provider_not_implemented() {
    let p = LocalProvider::new(fixture_api_key(), "http://localhost:8080").expect("new");
    assert!(matches!(p.get_token().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(p.refresh().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(p.revoke().await, Err(CredentialsError::NotImplemented(_))));
    assert_eq!(p.kind(), ProviderKind::Local);
}

// ============================================================================
// §3 5 鉴权 + K-1 强校验 (10 测试)
// ============================================================================

#[test]
fn test_auth_5_modes() {
    let names = AuthMethod::all_type_names();
    assert_eq!(names.len(), 5);
    assert!(names.contains(&"api_key"));
    assert!(names.contains(&"oauth2"));
    assert!(names.contains(&"jwt"));
    assert!(names.contains(&"iam"));
    assert!(names.contains(&"mtls"));
}

#[test]
fn test_k1_api_key_empty() {
    let m = AuthMethod::ApiKey { api_key: String::new() };
    let err = m.validate().unwrap_err();
    assert!(matches!(err, CredentialsError::EmptyApiKey));
    assert!(err.to_string().contains("K-1"));
}

#[test]
fn test_k1_oauth_client_id_empty() {
    let m = AuthMethod::OAuth2 {
        client_id: String::new(),
        client_secret: Some("secret".to_string()),
        refresh_token: None,
        access_token: None,
        token_url: "https://oauth.example.com/token".to_string(),
    };
    let err = m.validate().unwrap_err();
    assert!(matches!(err, CredentialsError::EmptyOAuthClientId));
}

#[test]
fn test_k1_jwt_audience_empty() {
    let m = AuthMethod::Jwt {
        token: "eyJ...".to_string(),
        audience: String::new(),
        issuer: Some("issuer".to_string()),
        expires_at_unix: Some(1735689600),
    };
    let err = m.validate().unwrap_err();
    assert!(matches!(err, CredentialsError::EmptyJwtAudience));
}

#[test]
fn test_k1_iam_role_empty() {
    let m = AuthMethod::Iam {
        role_arn: String::new(),
        session_name: Some("session".to_string()),
        region: Some("us-east-1".to_string()),
        access_key_id: "AKIA".to_string(),
        secret_access_key: "secret".to_string(),
    };
    let err = m.validate().unwrap_err();
    assert!(matches!(err, CredentialsError::EmptyIamRole));
}

#[test]
fn test_k1_mtls_cert_path_invalid() {
    let m = AuthMethod::Mtls {
        cert_path: std::path::PathBuf::from("/nonexistent/cert.pem"),
        key_path: std::path::PathBuf::from("/nonexistent/key.pem"),
        ca_path: None,
        server_name: "api.example.com".to_string(),
    };
    let err = m.validate().unwrap_err();
    assert!(matches!(err, CredentialsError::InvalidMtlsCertPath(_)));
    if let CredentialsError::InvalidMtlsCertPath(s) = err {
        assert!(s.contains("nonexistent"));
    }
}

#[test]
fn test_k1_5_variants_in_constant() {
    // K-1 强校验 5 变体必在 K1_STRONG_VALIDATION_VARIANTS
    assert_eq!(K1_STRONG_VALIDATION_VARIANTS.len(), 5);
    assert!(K1_STRONG_VALIDATION_VARIANTS.contains(&"EmptyApiKey"));
    assert!(K1_STRONG_VALIDATION_VARIANTS.contains(&"EmptyOAuthClientId"));
    assert!(K1_STRONG_VALIDATION_VARIANTS.contains(&"EmptyJwtAudience"));
    assert!(K1_STRONG_VALIDATION_VARIANTS.contains(&"EmptyIamRole"));
    assert!(K1_STRONG_VALIDATION_VARIANTS.contains(&"InvalidMtlsCertPath"));
}

#[test]
fn test_auth_oauth2_valid() {
    let m = AuthMethod::OAuth2 {
        client_id: "client".to_string(),
        client_secret: Some("secret".to_string()),
        refresh_token: Some("rt".to_string()),
        access_token: Some("at".to_string()),
        token_url: "https://oauth.example.com/token".to_string(),
    };
    m.validate().expect("valid oauth2");
    assert_eq!(m.type_name(), "oauth2");
}

#[test]
fn test_auth_jwt_valid() {
    let m = AuthMethod::Jwt {
        token: "eyJ...".to_string(),
        audience: "apeireth-api".to_string(),
        issuer: Some("issuer".to_string()),
        expires_at_unix: Some(1735689600),
    };
    m.validate().expect("valid jwt");
    assert_eq!(m.type_name(), "jwt");
}

#[test]
fn test_auth_iam_valid() {
    let m = AuthMethod::Iam {
        role_arn: "arn:aws:iam::123:role/R".to_string(),
        session_name: Some("s".to_string()),
        region: Some("us-east-1".to_string()),
        access_key_id: "AKIA".to_string(),
        secret_access_key: "secret".to_string(),
    };
    m.validate().expect("valid iam");
    assert_eq!(m.type_name(), "iam");
}

#[test]
fn test_auth_mtls_valid() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let cert_path = dir.path().join("cert.pem");
    let key_path = dir.path().join("key.pem");
    std::fs::write(&cert_path, b"cert").expect("write cert");
    std::fs::write(&key_path, b"key").expect("write key");
    let m = AuthMethod::Mtls {
        cert_path,
        key_path,
        ca_path: None,
        server_name: "api.example.com".to_string(),
    };
    m.validate().expect("valid mtls");
    assert_eq!(m.type_name(), "mtls");
}

// ============================================================================
// §4 5 Scope (1 测试)
// ============================================================================

#[test]
fn test_scope_5_levels() {
    let all = Scope::all();
    assert_eq!(all.len(), 5);
    assert_eq!(all[0], Scope::Read);
    assert_eq!(all[1], Scope::Write);
    assert_eq!(all[2], Scope::Admin);
    assert_eq!(all[3], Scope::Owner);
    assert_eq!(all[4], Scope::Root);
    // 顺序: Read < Write < Admin < Owner < Root
    assert!(all[0] < all[1]);
    assert!(all[1] < all[2]);
    assert!(all[2] < all[3]);
    assert!(all[3] < all[4]);
}

// ============================================================================
// §5 4 轮换策略 (4 测试)
// ============================================================================

#[test]
fn test_rotation_4_strategies() {
    let all = RotationStrategy::all_names();
    assert_eq!(all.len(), 4);
    assert_eq!(RotationStrategy::ALL_NAMES.len(), 4);
    assert!(all.contains(&"manual"));
    assert!(all.contains(&"time"));
    assert!(all.contains(&"count"));
    assert!(all.contains(&"hybrid"));
}

#[test]
fn test_rotation_time_30_days() {
    let s = time_default();
    let now = chrono::Utc::now();
    let recent = now - chrono::Duration::days(15);
    let old = now - chrono::Duration::days(31);
    assert!(!s.should_rotate(Some(recent), 0));
    assert!(s.should_rotate(Some(old), 0));
    assert!(s.should_rotate(None, 0));
    assert!(s.describe().contains("30 days"));
}

#[test]
fn test_rotation_count_1000_uses() {
    let s = count_default();
    let now = chrono::Utc::now();
    assert!(!s.should_rotate(Some(now), 0));
    assert!(!s.should_rotate(Some(now), 999));
    assert!(s.should_rotate(Some(now), 1000));
    assert!(s.should_rotate(Some(now), 5000));
    assert!(s.describe().contains("1000"));
}

#[test]
fn test_rotation_hybrid() {
    let s = hybrid_default();
    let now = chrono::Utc::now();
    let recent = now - chrono::Duration::days(15);
    let old = now - chrono::Duration::days(31);
    assert!(!s.should_rotate(Some(recent), 500));
    assert!(s.should_rotate(Some(old), 500));
    assert!(s.should_rotate(Some(recent), 1500));
    assert!(s.should_rotate(None, 0));
}

#[test]
fn test_rotation_manual_never() {
    let s = RotationStrategy::Manual;
    let now = chrono::Utc::now();
    assert!(!s.should_rotate(Some(now), 0));
    assert!(!s.should_rotate(Some(now), 1_000_000));
    assert!(!s.should_rotate(None, 0));
}

// ============================================================================
// §6 4 audit 事件 (4 测试)
// ============================================================================

#[tokio::test]
async fn test_audit_get_event() {
    let log = AuditLog::new();
    let ev = AuditEvent::get(ProviderKind::Anthropic, "user-001");
    log.record(ev).await.expect("record get");
    let events = log.list_by_kind(AuditEventKind::Get).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].provider, ProviderKind::Anthropic);
    assert_eq!(events[0].requester, "user-001");
}

#[tokio::test]
async fn test_audit_put_event() {
    let log = AuditLog::new();
    let ev = AuditEvent::put(ProviderKind::OpenAI, "user-002", "keyring");
    log.record(ev).await.expect("record put");
    let events = log.list_by_kind(AuditEventKind::Put).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].provider, ProviderKind::OpenAI);
    assert!(events[0].message.contains("keyring"));
}

#[tokio::test]
async fn test_audit_rotate_event() {
    let log = AuditLog::new();
    let old = chrono::Utc::now() - chrono::Duration::days(1);
    let new = chrono::Utc::now() + chrono::Duration::days(30);
    let ev = AuditEvent::rotate(
        ProviderKind::Google,
        "service-001",
        time_default(),
        Some(old),
        Some(new),
    );
    log.record(ev).await.expect("record rotate");
    let events = log.list_by_kind(AuditEventKind::Rotate).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].old_expires_at, Some(old));
    assert_eq!(events[0].new_expires_at, Some(new));
    assert!(events[0].rotation_strategy.is_some());
}

#[tokio::test]
async fn test_audit_revoke_event() {
    let log = AuditLog::new();
    let ev = AuditEvent::revoke(
        ProviderKind::Azure,
        "admin-001",
        "key compromise suspected",
    );
    log.record(ev).await.expect("record revoke");
    let events = log.list_by_kind(AuditEventKind::Revoke).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].revoke_reason.as_deref(), Some("key compromise suspected"));
}

// ============================================================================
// §7 CredentialsManager 集成 (5 测试)
// ============================================================================

#[tokio::test]
async fn test_manager_5_providers_add() {
    let mut mgr = CredentialsManager::new(Scope::Owner).expect("new");
    for kind in ProviderKind::all() {
        let p: Box<dyn Provider> = match kind {
            ProviderKind::Anthropic => Box::new(AnthropicProvider::new(fixture_api_key()).expect("new")),
            ProviderKind::OpenAI => Box::new(OpenAIProvider::new(fixture_api_key()).expect("new")),
            ProviderKind::Google => Box::new(GoogleProvider::new(fixture_api_key()).expect("new")),
            ProviderKind::Azure => Box::new(AzureProvider::new(fixture_api_key(), "r").expect("new")),
            ProviderKind::Local => Box::new(LocalProvider::new(fixture_api_key(), "http://localhost:8080").expect("new")),
        };
        mgr.add_provider(kind, p).expect("add");
    }
    assert_eq!(mgr.list_providers().len(), 5);
}

#[tokio::test]
async fn test_manager_get_token_audit_recorded() {
    let mut mgr = CredentialsManager::new(Scope::Read).expect("new");
    mgr.add_provider(
        ProviderKind::OpenAI,
        Box::new(OpenAIProvider::new(fixture_api_key()).expect("new")),
    )
    .expect("add");
    let _ = mgr.get_token(ProviderKind::OpenAI).await; // 返 NotImplemented
    let events = mgr.audit_log().list_by_kind(AuditEventKind::Get).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].provider, ProviderKind::OpenAI);
}

#[tokio::test]
async fn test_manager_revoke_insufficient_scope() {
    let mut mgr = CredentialsManager::new(Scope::Read).expect("new");
    mgr.add_provider(
        ProviderKind::Anthropic,
        Box::new(AnthropicProvider::new(fixture_api_key()).expect("new")),
    )
    .expect("add");
    // Read 不能 revoke (需要 Admin)
    let err = mgr.revoke_token(ProviderKind::Anthropic, "test").await.unwrap_err();
    assert!(matches!(err, CredentialsError::InsufficientScope { .. }));
}

#[tokio::test]
async fn test_manager_provider_not_found() {
    let mgr = CredentialsManager::new(Scope::Read).expect("new");
    let err = mgr.get_token(ProviderKind::Anthropic).await.unwrap_err();
    assert!(matches!(err, CredentialsError::ProviderNotFound(_)));
}

#[tokio::test]
async fn test_manager_scope_change_rotation() {
    let mut mgr = CredentialsManager::new(Scope::Read).expect("new");
    assert_eq!(mgr.scope(), Scope::Read);
    mgr.set_rotation_strategy(time_default());
    assert_eq!(mgr.rotation_strategy(), time_default());
    mgr.set_rotation_strategy(hybrid_default());
    assert_eq!(mgr.rotation_strategy(), hybrid_default());
    mgr.set_rotation_strategy(count_default());
    assert_eq!(mgr.rotation_strategy(), count_default());
}

#[tokio::test]
async fn test_manager_4_rotation_via_with_options() {
    for strategy in [
        RotationStrategy::Manual,
        time_default(),
        count_default(),
        hybrid_default(),
    ] {
        let mgr = CredentialsManager::with_options(Scope::Owner, strategy, 100).expect("with_options");
        assert_eq!(mgr.rotation_strategy(), strategy);
    }
}

#[tokio::test]
async fn test_manager_audit_p0_redaction_in_manager() {
    // P0: 不能记录含 "sk-" 的事件
    let log = AuditLog::new();
    let ev = AuditEvent::new(
        AuditEventKind::Get,
        ProviderKind::Anthropic,
        "u",
        "leaked token=sk-12345",
    );
    let err = log.record(ev).await.unwrap_err();
    assert!(matches!(err, CredentialsError::AuditLogFailed { .. }));
}

#[tokio::test]
async fn test_manager_credentials_config_default() {
    let cfg = CredentialsConfig::default_config();
    cfg.validate().expect("valid");
    assert_eq!(cfg.default_provider, ProviderKind::Anthropic);
    assert_eq!(cfg.default_scope, Scope::Read);
    assert_eq!(cfg.rotation_strategy, RotationStrategy::Manual);
}

// ============================================================================
// §8 m3 防御 + Token Manager (3 测试)
// ============================================================================

#[test]
fn test_m3_tool_whitelist() {
    // 8 工具白名单编译期守门
    assert_eq!(TOOL_WHITELIST.len(), 8);
    assert_eq!(TOOL_WHITELIST_COUNT, 8);
}

#[test]
fn test_m3_validate_tool_call() {
    let valid = validate_tool_call("apeireth_credentials_get_token", &serde_json::json!({}));
    assert!(valid.is_ok());
    let invalid = validate_tool_call("apeireth_credentials_bogus", &serde_json::json!({}));
    assert!(matches!(invalid, Err(CredentialsError::ToolNotWhitelisted(_))));
}

#[tokio::test]
async fn test_token_manager_5_methods_via_provider() {
    // 5 TokenManager 方法都通过 Provider 包装
    let p = AnthropicProvider::new(fixture_api_key()).expect("new");
    let tm = ProviderTokenManager::new(p);
    assert!(matches!(tm.get_token().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(tm.refresh().await, Err(CredentialsError::NotImplemented(_))));
    assert!(matches!(tm.revoke().await, Err(CredentialsError::NotImplemented(_))));
    assert!(!tm.is_valid().await);
    assert!(tm.expires_at().await.is_none());
}

// ============================================================================
// §9 错误类别 + 跨模块集成 (3 测试)
// ============================================================================

#[test]
fn test_error_categories_6() {
    // 6 错误类别都有覆盖
    let mut categories = std::collections::HashSet::new();
    categories.insert(CredentialsError::NotImplemented("x").category());
    categories.insert(CredentialsError::EmptyApiKey.category());
    categories.insert(CredentialsError::TokenExpired { expires_at: "x".to_string(), now: "x".to_string() }.category());
    categories.insert(CredentialsError::InsufficientScope { current: "x".to_string(), required: "x".to_string() }.category());
    categories.insert(CredentialsError::RotationFailed { strategy: "x".to_string(), reason: "x".to_string() }.category());
    categories.insert(CredentialsError::AuditLogFailed { event: "x".to_string(), reason: "x".to_string() }.category());
    assert_eq!(categories.len(), 6);
}

#[test]
fn test_error_codes_15_unique() {
    // 13 独立 code (K-1 5 变体共享 "config_empty", 跨 category 13 个不同 code)
    let codes = [
        CredentialsError::NotImplemented("x").code(),
        CredentialsError::Internal("x".to_string()).code(),
        CredentialsError::EmptyApiKey.code(),
        CredentialsError::InvalidMtlsCertPath("x".to_string()).code(),
        CredentialsError::TokenExpired { expires_at: "x".to_string(), now: "x".to_string() }.code(),
        CredentialsError::TokenRefreshFailed("x".to_string()).code(),
        CredentialsError::TokenRevokeFailed("x".to_string()).code(),
        CredentialsError::InsufficientScope { current: "x".to_string(), required: "x".to_string() }.code(),
        CredentialsError::UnknownScope("x".to_string()).code(),
        CredentialsError::RotationFailed { strategy: "x".to_string(), reason: "x".to_string() }.code(),
        CredentialsError::AuditLogFailed { event: "x".to_string(), reason: "x".to_string() }.code(),
        CredentialsError::ToolNotWhitelisted("x".to_string()).code(),
        CredentialsError::ProviderNotFound("x".to_string()).code(),
    ];
    let unique: std::collections::HashSet<_> = codes.iter().collect();
    assert_eq!(unique.len(), 13);
}

#[test]
fn test_secret_string_redacts_in_display() {
    let s = SecretString::new("sk-very-secret");
    let displayed = format!("{s}");
    assert!(!displayed.contains("sk-very-secret"));
    assert!(displayed.contains("redacted"));
}

// ============================================================================
// Helper functions
// ============================================================================

fn fixture_api_key() -> AuthMethod {
    AuthMethod::ApiKey {
        api_key: "sk-test-1234".to_string(),
    }
}

// 避免 unused import 警告
#[allow(dead_code)]
fn _unused() {
    let _ = ErrorCategory::General;
}
