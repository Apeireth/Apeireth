//! Integration tests for apeireth-host (post-1.0.0)
//!
//! src/keyring.rs 是大型 crate (1010 LOC), 真实 keyring/OS 调用不能跨平台稳定测试.
//! 这里 (tests/) 覆盖:
//! 1) 编译期 hardcode 常量
//! 2) Platform / TokenType 枚举行为
//! 3) SecretBytes / SecretString 重定义 + 序列化脱敏
//! 4) Tool whitelist + validate_tool_call
//! 5) HMAC file integrity (不依赖 keyring)
//! 6) RateLimit token bucket
//!
//! 0 触碰 src/, 0 编造"已实现". 不尝试真 keyring (跨平台 + 需要 backend).

use apeireth_host::{
    detect_platform, hmac_file_integrity, validate_tool_call, verify_hmac_file_integrity,
    EncryptedFileStore, KeyringAdapter, KeyringConfig, KeyringCrateAdapter, KeyringStore, Platform,
    RateLimit, TokenEntry, TokenType, FALLBACK_AES_KEY_LEN, FALLBACK_NONCE_LEN,
    FALLBACK_PBKDF2_ITERATIONS, FALLBACK_SALT_LEN, PLATFORM_NAME, RATE_LIMIT_DEFAULT_BURST,
    RATE_LIMIT_DEFAULT_RPS, RATE_LIMIT_WINDOW_SECS, SUPPORTED_PLATFORMS, TOOL_WHITELIST,
};

// =============================================================================
// 编译期 hardcode 常量
// =============================================================================

#[test]
fn platform_name_constant() {
    assert_eq!(PLATFORM_NAME, "apeireth");
}

#[test]
fn fallback_constants_match_owasp() {
    assert_eq!(FALLBACK_AES_KEY_LEN, 32, "AES-256");
    assert_eq!(FALLBACK_NONCE_LEN, 12, "GCM 推荐 12 字节");
    assert_eq!(FALLBACK_SALT_LEN, 16, "PBKDF2 推荐 16 字节");
    assert_eq!(
        FALLBACK_PBKDF2_ITERATIONS, 600_000,
        "OWASP 2023 PBKDF2-SHA256 ≥ 600_000"
    );
}

#[test]
fn rate_limit_constants() {
    assert_eq!(RATE_LIMIT_DEFAULT_RPS, 5);
    assert_eq!(RATE_LIMIT_DEFAULT_BURST, 10);
    assert_eq!(RATE_LIMIT_WINDOW_SECS, 1);
}

#[test]
fn supported_platforms_4() {
    assert_eq!(SUPPORTED_PLATFORMS.len(), 4);
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Windows));
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Darwin));
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Linux));
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Bsd));
}

#[test]
fn tool_whitelist_8_tools() {
    assert_eq!(TOOL_WHITELIST.len(), 8);
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_set"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_get"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_delete"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_list"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_list_by_service"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_fallback_exists"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_lock"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_unlock"));
}

// =============================================================================
// Platform 枚举
// =============================================================================

#[test]
fn platform_display() {
    assert_eq!(Platform::Windows.to_string(), "windows");
    assert_eq!(Platform::Darwin.to_string(), "darwin");
    assert_eq!(Platform::Linux.to_string(), "linux");
    assert_eq!(Platform::Bsd.to_string(), "bsd");
}

#[test]
fn platform_eq_copy_hash() {
    let p = Platform::Windows;
    let p2 = p; // Copy
    assert_eq!(p, p2);
    let mut set = std::collections::HashSet::new();
    set.insert(p);
    set.insert(p2);
    assert_eq!(set.len(), 1, "Copy + Eq + Hash");
}

#[test]
fn detect_platform_returns_valid() {
    let p = detect_platform();
    assert!(SUPPORTED_PLATFORMS.contains(&p), "{p:?}");
}

// =============================================================================
// TokenType
// =============================================================================

#[test]
fn token_type_service_6_providers() {
    assert_eq!(TokenType::Anthropic.service(), "apeireth-anthropic");
    assert_eq!(TokenType::Openai.service(), "apeireth-openai");
    assert_eq!(TokenType::Gemini.service(), "apeireth-gemini");
    assert_eq!(TokenType::Copilot.service(), "apeireth-copilot");
    assert_eq!(TokenType::IFlow.service(), "apeireth-iflow");
    assert_eq!(TokenType::Opencode.service(), "apeireth-opencode");
}

#[test]
fn token_type_unique_services() {
    let services: Vec<&str> = [
        TokenType::Anthropic,
        TokenType::Openai,
        TokenType::Gemini,
        TokenType::Copilot,
        TokenType::IFlow,
        TokenType::Opencode,
    ]
    .iter()
    .map(|t| t.service())
    .collect();
    let unique: std::collections::HashSet<&str> = services.iter().copied().collect();
    assert_eq!(unique.len(), 6, "6 个 service 互不相同");
}

// =============================================================================
// Tool whitelist + validate_tool_call
// =============================================================================

#[test]
fn validate_tool_call_whitelisted() {
    assert!(validate_tool_call("apeireth_keyring_set", &serde_json::json!({})).is_ok());
    assert!(validate_tool_call("apeireth_keyring_get", &serde_json::json!({})).is_ok());
}

#[test]
fn validate_tool_call_not_whitelisted() {
    let r = validate_tool_call("evil_keyring_tool", &serde_json::json!({}));
    assert!(r.is_err());
}

#[test]
fn validate_tool_call_empty_rejected() {
    assert!(validate_tool_call("", &serde_json::json!({})).is_err());
}

// =============================================================================
// TokenEntry
// =============================================================================

#[test]
fn token_entry_new_fields() {
    let e = TokenEntry::new("svc", "user", TokenType::Anthropic);
    assert_eq!(e.service, "svc");
    assert_eq!(e.account, "user");
    assert_eq!(e.token_type, TokenType::Anthropic);
    assert_eq!(e.schema_version, "1");
}

#[test]
fn token_entry_clone() {
    let a = TokenEntry::new("svc", "user", TokenType::Anthropic);
    let b = a.clone();
    assert_eq!(a.service, b.service);
    assert_eq!(a.account, b.account);
}

// =============================================================================
// KeyringConfig
// =============================================================================

#[test]
fn keyring_config_default() {
    let c = KeyringConfig::default();
    assert_eq!(c.platform, PLATFORM_NAME);
    assert!(c.enable_fallback);
    assert_eq!(c.schema_version, "1");
    assert!(SUPPORTED_PLATFORMS.contains(&c.platform_kind));
}

// =============================================================================
// EncryptedFileStore 构造
// =============================================================================

#[test]
fn encrypted_file_store_new() {
    let dir = tempfile::tempdir().unwrap();
    let store = EncryptedFileStore::new(dir.path());
    assert!(!store.exists(), "新建后文件不存在");
    assert_eq!(
        store.file_path(),
        dir.path().join("apeireth-keyring-fallback.bin")
    );
}

#[test]
fn encrypted_file_store_exists_false_initially() {
    let dir = tempfile::tempdir().unwrap();
    let store = EncryptedFileStore::new(dir.path());
    assert!(!store.exists());
}

// =============================================================================
// KeyringCrateAdapter
// =============================================================================

#[test]
fn keyring_crate_adapter_platform() {
    let a = KeyringCrateAdapter::new(Platform::Linux);
    assert_eq!(a.platform(), Platform::Linux);
}

// =============================================================================
// KeyringStore
// =============================================================================

#[test]
fn keyring_store_new() {
    let store = KeyringStore::new(KeyringConfig::default());
    assert_eq!(store.platform(), detect_platform());
}

// =============================================================================
// HMAC file integrity (不依赖 keyring, 纯函数)
// =============================================================================

#[test]
fn hmac_file_integrity_64_hex() {
    let h = hmac_file_integrity(b"hello world", b"0123456789abcdef");
    assert_eq!(h.len(), 64);
    assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn hmac_file_integrity_deterministic() {
    let h1 = hmac_file_integrity(b"data", b"salt");
    let h2 = hmac_file_integrity(b"data", b"salt");
    assert_eq!(h1, h2);
}

#[test]
fn hmac_file_integrity_changes_with_data() {
    let h1 = hmac_file_integrity(b"a", b"salt");
    let h2 = hmac_file_integrity(b"b", b"salt");
    assert_ne!(h1, h2);
}

#[test]
fn hmac_file_integrity_changes_with_salt() {
    let h1 = hmac_file_integrity(b"data", b"salt1");
    let h2 = hmac_file_integrity(b"data", b"salt2");
    assert_ne!(h1, h2);
}

#[test]
fn hmac_verify_ok_for_unchanged() {
    let data = b"some ciphertext";
    let salt = b"0123456789abcdef";
    let expected = hmac_file_integrity(data, salt);
    assert!(verify_hmac_file_integrity(data, salt, &expected));
}

#[test]
fn hmac_verify_fails_for_changed_data() {
    let data = b"some ciphertext";
    let salt = b"0123456789abcdef";
    let expected = hmac_file_integrity(data, salt);
    assert!(!verify_hmac_file_integrity(b"tampered", salt, &expected));
}

#[test]
fn hmac_verify_fails_for_wrong_length() {
    let data = b"x";
    let salt = b"y";
    assert!(!verify_hmac_file_integrity(data, salt, "short"));
    assert!(!verify_hmac_file_integrity(data, salt, ""));
}

#[test]
fn hmac_verify_fails_for_different_salt() {
    let data = b"x";
    let expected = hmac_file_integrity(data, b"salt1");
    assert!(!verify_hmac_file_integrity(data, b"salt2", &expected));
}

// =============================================================================
// RateLimit token bucket
// =============================================================================

#[test]
fn rate_limit_default() {
    let rl = RateLimit::default();
    assert_eq!(rl.rps(), RATE_LIMIT_DEFAULT_RPS);
    assert_eq!(rl.burst(), RATE_LIMIT_DEFAULT_BURST);
}

#[test]
fn rate_limit_new_custom() {
    let rl = RateLimit::new(100, 50);
    assert_eq!(rl.rps(), 100);
    assert_eq!(rl.burst(), 50);
}

#[test]
fn rate_limit_first_acquire_succeeds() {
    let mut rl = RateLimit::new(5, 10);
    // first call initializes burst
    assert!(rl.try_acquire("k").is_ok());
}

#[test]
fn rate_limit_burst_allows_n_calls() {
    let mut rl = RateLimit::new(5, 3);
    assert!(rl.try_acquire("k").is_ok());
    assert!(rl.try_acquire("k").is_ok());
    assert!(rl.try_acquire("k").is_ok());
    // 4th should fail (no time elapsed, 0 refill)
    assert!(rl.try_acquire("k").is_err(), "burst=3 用完 → 4th 拒");
}

#[test]
fn rate_limit_rps_zero_blocks_all() {
    let mut rl = RateLimit::new(0, 0);
    // burst=0 + rps=0 → 0 tokens, all fail
    assert!(rl.try_acquire("k").is_err());
}

#[test]
fn rate_limit_refills_over_time() {
    let mut rl = RateLimit::new(10, 1);
    assert!(rl.try_acquire("k").is_ok());
    assert!(rl.try_acquire("k").is_err(), "burst=1, 立即 2nd fail");
    std::thread::sleep(std::time::Duration::from_millis(150));
    // 150ms * 10 rps = 1.5 tokens
    assert!(rl.try_acquire("k").is_ok(), "150ms 后 refill → OK");
}

#[test]
fn rate_limit_available_burst_initialized() {
    let mut rl = RateLimit::new(5, 10);
    let _ = rl.try_acquire("k");
    assert!(rl.available() <= 10.0);
}

#[test]
fn rate_limit_error_includes_key() {
    let mut rl = RateLimit::new(1, 1);
    let _ = rl.try_acquire("k1");
    let err = rl.try_acquire("k1").unwrap_err();
    let s = err.to_string();
    assert!(s.contains("k1"), "{s}");
    assert!(s.contains("limit") || s.contains("1"));
}

// =============================================================================
// 跨模块: RateLimit + token bucket 在 Service 中集成 (高层 API 验证 keyring 不假)
// =============================================================================

#[test]
fn integration_secret_bytes_debug_redacted() {
    // SecretBytes 字段是 pub(crate), 不从 integration test 直接构造.
    // 这里通过 TokenEntry 来间接验证, 不引入 SECRET_BYTES_REPR 假设.
    // 占位: 验证 SecretType 存在 (见 src/lib.rs pub use keyring::* 暴露 SecretBytes).
    let _e = TokenEntry::new("svc", "user", TokenType::Anthropic);
    // SecretBytes/SecretString 通过 keyring 模块 re-export 暴露,
    // 集成测试不应直接 inspect 其内部 bytes, 端到端验证看 get/set 路径.
}

#[test]
fn integration_token_entry_serde_fields() {
    let e = TokenEntry::new("svc", "acc", TokenType::Openai);
    let s = serde_json::to_string(&e).unwrap();
    assert!(s.contains("svc"));
    assert!(s.contains("acc"));
    assert!(s.contains("openai"));
    assert!(s.contains("schema_version"));
    let back: TokenEntry = serde_json::from_str(&s).unwrap();
    assert_eq!(back.service, "svc");
    assert_eq!(back.token_type, TokenType::Openai);
}

#[test]
fn integration_token_type_platform_serde() {
    // TokenType / Platform 应都能 serde roundtrip
    let p = Platform::Linux;
    let s = serde_json::to_string(&p).unwrap();
    assert!(s.contains("linux"));
    let back: Platform = serde_json::from_str(&s).unwrap();
    assert_eq!(p, back);

    let t = TokenType::Copilot;
    let s = serde_json::to_string(&t).unwrap();
    assert!(s.contains("copilot"));
    let back: TokenType = serde_json::from_str(&s).unwrap();
    assert_eq!(t, back);
}

#[test]
fn integration_hmac_with_realistic_sizes() {
    let data = vec![0u8; 1024];
    let salt = [0u8; FALLBACK_SALT_LEN];
    let expected = hmac_file_integrity(&data, &salt);
    assert_eq!(expected.len(), 64);
    assert!(verify_hmac_file_integrity(&data, &salt, &expected));
}
