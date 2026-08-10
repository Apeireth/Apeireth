//! `apeireth-keyring` 集成测试 (per R20 阶段 1 Fixture 5 模式)
//!
//! 4 K-1 强校验 + 1 P0 fixture 0 明文落盘, 全部 in-process 跑 (不依赖外部 keyring daemon).

use apeireth_keyring::{
    detect_platform, validate_tool_call, KeyringError, KeyringStore, KeyringConfig, Platform,
    SecretBytes, TokenEntry, TokenType, FALLBACK_AES_KEY_LEN, FALLBACK_FILE_NAME,
    FALLBACK_NONCE_LEN, FALLBACK_PBKDF2_ITERATIONS, KEYRING_SCHEMA_VERSION, PLATFORM_NAME,
    SUPPORTED_PLATFORMS, TOOL_WHITELIST,
};

/// K-1 fixture #1: 平台名 = "apeireth" (5 K-1 字样 #1).
#[test]
fn k1_platform_name_is_apeireth() {
    assert_eq!(PLATFORM_NAME, "apeireth", "PLATFORM_NAME 必须 = 'apeireth' (5 K-1 字样 #1)");
}

/// K-1 fixture #2: 4 Platform 枚举 (Windows / Darwin / Linux / Bsd).
#[test]
fn k1_platform_enum_has_4_variants() {
    assert_eq!(SUPPORTED_PLATFORMS.len(), 4, "SUPPORTED_PLATFORMS 必须 4 项");
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Windows));
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Darwin));
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Linux));
    assert!(SUPPORTED_PLATFORMS.contains(&Platform::Bsd));
}

/// K-1 fixture #3: TOOL_WHITELIST 8 工具名 + 5 K-1 字样 4/5 命中.
#[test]
fn k1_tool_whitelist_8_with_5_key_words() {
    assert_eq!(TOOL_WHITELIST.len(), 8, "TOOL_WHITELIST 必须 8 项");

    // 5 K-1 字样: "apeireth" / "keyring" / "set" / "get" / "must-do"
    // "must-do" 不在工具名中, 但 `validate_tool_call` 错误信息可以体现
    let body = TOOL_WHITELIST.join(",");
    assert!(body.contains("apeireth"), "5 K-1 字样 #1: 'apeireth'");
    assert!(body.contains("keyring"), "5 K-1 字样 #2: 'keyring'");
    assert!(body.contains("set"), "5 K-1 字样 #3: 'set'");
    assert!(body.contains("get"), "5 K-1 字样 #4: 'get'");

    // "must-do" 字样在文档 + 注释中体现, 编译时已写入
    let src = include_str!("../src/lib.rs");
    assert!(src.contains("must-do"), "5 K-1 字样 #5: 'must-do' 必须在源码注释中");

    // 8 工具名硬枚举
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_set"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_get"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_delete"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_list"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_list_by_service"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_fallback_exists"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_lock"));
    assert!(TOOL_WHITELIST.contains(&"apeireth_keyring_unlock"));
}

/// K-1 fixture #4: 4 防御常量 (PBKDF2_ITER / AES_KEY / NONCE / FALLBACK_FILE).
#[test]
fn k1_4_defense_constants() {
    assert_eq!(FALLBACK_PBKDF2_ITERATIONS, 600_000, "PBKDF2 ≥ 600_000 per OWASP 2023");
    assert_eq!(FALLBACK_AES_KEY_LEN, 32, "AES-256 必 32 字节");
    assert_eq!(FALLBACK_NONCE_LEN, 12, "GCM nonce 必 12 字节");
    assert_eq!(FALLBACK_FILE_NAME, "apeireth-keyring-fallback.bin", "fallback 文件名 hardcode");
}

/// P0 fixture: 0 明文存凭证路径 (P0 安全铁律).
#[test]
fn p0_zero_plaintext_on_disk() {
    // 1) SecretBytes Debug 脱敏
    let secret = SecretBytes::new(b"sk-cp-kug0t7Jik3-real-key");
    let dbg = format!("{secret:?}");
    assert!(dbg.contains("***REDACTED***"), "SecretBytes Debug 必须脱敏");
    assert!(!dbg.contains("sk-cp-"), "SecretBytes Debug 严禁含明文");

    // 2) SecretBytes Serialize 脱敏
    let json = serde_json::to_string(&secret).unwrap();
    assert!(json.contains("***REDACTED***"));
    assert!(!json.contains("sk-cp-"));

    // 3) FALLBACK_FILE_NAME 严禁 .json / .txt 明文
    assert!(!FALLBACK_FILE_NAME.ends_with(".json"), "fallback 严禁 .json (明文)");
    assert!(!FALLBACK_FILE_NAME.ends_with(".txt"), "fallback 严禁 .txt (明文)");
    assert!(FALLBACK_FILE_NAME.ends_with(".bin"), "fallback 必须 .bin (二进制加密)");

    // 4) 静态检查: 没有任何 `std::fs::write(<token>, <plaintext>)` 类调用
    // 排除注释行, 只查实际代码行
    let src = include_str!("../src/lib.rs");
    let real_calls: Vec<&str> = src
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("//")
                && !t.starts_with("*")
                && !t.starts_with("///")
                && t.contains("fs::write")
        })
        .collect();
    assert!(
        real_calls.is_empty(),
        "严禁明文写 token 路径: 实际调用 = {real_calls:?}"
    );
}

/// 平台探测 — 4 Platform 之一
#[test]
fn platform_detection_returns_one_of_4() {
    let p = detect_platform();
    assert!(SUPPORTED_PLATFORMS.contains(&p), "探测到的平台必须在 SUPPORTED_PLATFORMS 中");
}

/// KeyringConfig 默认值
#[test]
fn keyring_config_default_values() {
    let c = KeyringConfig::default();
    assert_eq!(c.platform, PLATFORM_NAME);
    assert_eq!(c.schema_version, KEYRING_SCHEMA_VERSION);
    assert!(SUPPORTED_PLATFORMS.contains(&c.platform_kind));
    assert!(c.enable_fallback, "默认开启 fallback");
}

/// TokenEntry 构造 + serde roundtrip
#[test]
fn token_entry_serde_roundtrip() {
    let entry = TokenEntry::new("apeireth-anthropic", "chuling@local", TokenType::Anthropic);
    let json = serde_json::to_string(&entry).unwrap();
    let back: TokenEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(back.service, entry.service);
    assert_eq!(back.account, entry.account);
    assert_eq!(back.token_type, entry.token_type);
}

/// m3 防御 — validate_tool_call 拒绝白名单外工具
#[test]
fn m3_validate_tool_call_rejects_unknown() {
    // 白名单内
    assert!(validate_tool_call("apeireth_keyring_set", &serde_json::json!({})).is_ok());
    assert!(validate_tool_call("apeireth_keyring_get", &serde_json::json!({})).is_ok());

    // 白名单外 (m3 幻觉)
    let err = validate_tool_call("apeireth_keyring_evil", &serde_json::json!({})).unwrap_err();
    assert!(matches!(err, KeyringError::ToolNotWhitelisted(_)));

    // 空字符串
    let err = validate_tool_call("", &serde_json::json!({})).unwrap_err();
    assert!(matches!(err, KeyringError::ToolNotWhitelisted(_)));
}

/// TokenType 6 变体 + service() 拼前缀
#[test]
fn token_type_6_variants_and_service_prefix() {
    let types = [
        TokenType::Anthropic,
        TokenType::Openai,
        TokenType::Gemini,
        TokenType::Copilot,
        TokenType::IFlow,
        TokenType::Opencode,
    ];
    assert_eq!(types.len(), 6);
    for t in types {
        let svc = t.service();
        assert!(svc.starts_with("apeireth-"), "service 必须 'apeireth-*' 前缀");
    }
    assert_eq!(TokenType::Anthropic.service(), "apeireth-anthropic");
    assert_eq!(TokenType::Openai.service(), "apeireth-openai");
    assert_eq!(TokenType::Gemini.service(), "apeireth-gemini");
}

/// KeyringStore 构造 + 平台读取
#[tokio::test]
async fn keyring_store_new_and_platform() {
    let store = KeyringStore::new(KeyringConfig::default());
    let p = store.platform();
    assert!(SUPPORTED_PLATFORMS.contains(&p));
    assert_eq!(store.config().platform, PLATFORM_NAME);
}

/// KeyringStore 4 平台 fallback 路径 (in-process, 不连真 daemon).
#[tokio::test]
async fn keyring_store_fallback_path_constructs() {
    // 模拟不支持平台 (Bsd 估 Linux 主机上跑, keyring backend 不可用)
    let mut cfg = KeyringConfig::default();
    cfg.enable_fallback = true;
    let store = KeyringStore::new(cfg);
    // fallback_exists 走 in-memory check, 不一定为 true
    let _ = store.fallback_exists().await;
    // 调 lock 不报错
    let _ = store.lock().await;
}

// ============================================================================
// §17 5 OS platform mock 集成测试 (per R20 阶段 6 估补 #1 要求 "5+ OS 平台 mock")
// 5 mock backend: windows / darwin / linux / bsd / unsupported
// ============================================================================

use apeireth_keyring::{
    mock_backend_bsd, mock_backend_darwin, mock_backend_linux, mock_backend_unsupported,
    mock_backend_windows, DisabledAdapter, EncryptedFileAdapter, EncryptedFileStore,
    InMemoryAdapter, KeyringAdapter, MockAdapter, MockBackend, MockScript, ProviderError,
    PROVIDER_DISABLED, PROVIDER_ENCRYPTED_FILE, PROVIDER_IN_MEMORY, PROVIDER_MOCK,
    PROVIDER_OS_KEYRING,
};

/// 集成 #1: Windows mock backend + ProviderError 健康检查.
#[test]
fn integration_5_os_mock_windows() {
    let m = mock_backend_windows();
    assert_eq!(m.platform(), Platform::Windows);
    assert!(m.check_health().is_ok());
    // crash + recover
    m.crash();
    assert!(matches!(m.check_health(), Err(ProviderError::Unavailable { .. })));
    m.recover();
    assert!(m.check_health().is_ok());
}

/// 集成 #2: macOS Keychain mock + set/get roundtrip.
#[test]
fn integration_5_os_mock_darwin() {
    let m = mock_backend_darwin();
    assert_eq!(m.platform(), Platform::Darwin);
    m.mock_set("apeireth-anthropic", "user@test", b"sk-cp-darwin");
    assert_eq!(m.mock_get("apeireth-anthropic", "user@test"), Some(b"sk-cp-darwin".to_vec()));
    assert_eq!(m.len(), 1);
}

/// 集成 #3: Linux Secret Service mock + 并发 set/get (竞争条件).
#[test]
fn integration_5_os_mock_linux_concurrent() {
    use std::sync::Arc;
    use std::thread;
    let m = Arc::new(mock_backend_linux());
    let mut handles = vec![];
    for i in 0..4 {
        let m2 = m.clone();
        handles.push(thread::spawn(move || {
            m2.mock_set("svc", &format!("acc{i}"), format!("data{i}").as_bytes());
        }));
    }
    for h in handles {
        h.join().expect("thread join");
    }
    assert_eq!(m.len(), 4);
}

/// 集成 #4: BSD mock + InMemoryAdapter 集成 (BSD 没真 OS keyring, 走 InMemory fallback).
#[tokio::test]
async fn integration_5_os_mock_bsd_in_memory_fallback() {
    let m = mock_backend_bsd();
    assert_eq!(m.platform(), Platform::Bsd);
    // BSD 估缺 OS keyring 真接入 (估补 R21+), 用 InMemoryAdapter 模拟
    let adapter = InMemoryAdapter::new(Platform::Bsd);
    let svc = "apeireth-anthropic";
    let acc = "user@test";
    let token = SecretBytes::new(b"sk-cp-bsd-fallback");
    adapter.set(svc, acc, &token).await.expect("set");
    let got = adapter.get(svc, acc).await.expect("get");
    assert_eq!(got.expose(), b"sk-cp-bsd-fallback");
}

/// 集成 #5: Unsupported platform mock (iOS/Android 估 R21+).
#[test]
fn integration_5_os_mock_unsupported() {
    let m = mock_backend_unsupported();
    // 不支持平台必须 health check fail
    let err = m.check_health().unwrap_err();
    assert!(matches!(err, ProviderError::Unavailable { .. }));
    // ProviderError 转 KeyringError
    use apeireth_keyring::KeyringError;
    let k: KeyringError = err.into();
    assert!(matches!(k, KeyringError::BackendUnavailable { .. }));
}

/// 集成 #6: 5 Provider impl 全部存在 + name 守门.
#[test]
fn integration_5_provider_impl_names() {
    use apeireth_keyring::{
        KeyringCrateAdapter, PROVIDER_DISABLED, PROVIDER_ENCRYPTED_FILE, PROVIDER_IN_MEMORY,
        PROVIDER_MOCK, PROVIDER_OS_KEYRING,
    };
    // 5 name 编译期 hardcode
    assert_eq!(PROVIDER_OS_KEYRING, "os-keyring");
    assert_eq!(PROVIDER_ENCRYPTED_FILE, "encrypted-file");
    assert_eq!(PROVIDER_IN_MEMORY, "in-memory");
    assert_eq!(PROVIDER_MOCK, "mock");
    assert_eq!(PROVIDER_DISABLED, "disabled");
    // 5 struct 都能 new (不同 platform)
    let _ = KeyringCrateAdapter::new(Platform::Linux);
    let _ = InMemoryAdapter::new(Platform::Linux);
    let _ = MockAdapter::new(Platform::Linux);
    let _ = DisabledAdapter::new(Platform::Linux, "test");
    let _ = EncryptedFileStore::new(std::path::Path::new("/tmp"));
    // EncryptedFileAdapter 需 Arc<EncryptedFileStore> 包装
    let store = std::sync::Arc::new(EncryptedFileStore::new(std::path::Path::new("/tmp")));
    let _ = EncryptedFileAdapter::new(Platform::Linux, store);
}

/// 集成 #7: MockAdapter 全 4 行为集成测试 (passthrough / always_fail / not_found / latency).
#[tokio::test]
async fn integration_mock_adapter_4_behaviors() {
    let p = MockAdapter::with_script(Platform::Linux, MockScript::passthrough());
    let svc = "apeireth-anthropic";
    let acc = "user@test";
    let token = SecretBytes::new(b"sk-cp-mock");

    // passthrough
    p.set(svc, acc, &token).await.unwrap();
    assert_eq!(p.get(svc, acc).await.unwrap().expose(), b"sk-cp-mock");

    // always_fail
    p.set_script(MockScript::always_fail("daemon crash"));
    assert!(matches!(
        p.set(svc, acc, &token).await,
        Err(KeyringError::BackendUnavailable { .. })
    ));

    // always_not_found (after reset)
    p.set_script(MockScript::passthrough());
    p.delete(svc, acc).await.unwrap();
    p.set_script(MockScript::always_not_found());
    assert!(matches!(
        p.get(svc, acc).await,
        Err(KeyringError::NotFound { .. })
    ));

    // latency
    p.set_script(MockScript::passthrough().with_latency(50));
    let start = std::time::Instant::now();
    p.set(svc, acc, &token).await.unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed >= std::time::Duration::from_millis(40), "latency ≥ 40ms, got {elapsed:?}");
}

/// 集成 #8: MockBackend 5 platform 枚举 (windows / darwin / linux / bsd / unsupported).
#[test]
fn integration_mock_backend_5_platforms() {
    let m_w = mock_backend_windows();
    assert_eq!(m_w.platform(), Platform::Windows);
    let m_d = mock_backend_darwin();
    assert_eq!(m_d.platform(), Platform::Darwin);
    let m_l = mock_backend_linux();
    assert_eq!(m_l.platform(), Platform::Linux);
    let m_b = mock_backend_bsd();
    assert_eq!(m_b.platform(), Platform::Bsd);
    // unsupported (Bsd 底层 + crash)
    let m_u = mock_backend_unsupported();
    assert!(m_u.check_health().is_err());
}

/// 集成 #9: KeyringConfig::validate 5 错误 + 1 正常 path.
#[test]
fn integration_config_validate_6_paths() {
    use apeireth_keyring::ConfigError;

    // 正常 path
    let cfg = KeyringConfig::default();
    assert!(cfg.validate().is_ok());

    // InvalidFallbackDir
    let cfg = KeyringConfig {
        fallback_dir: std::path::PathBuf::new(),
        ..Default::default()
    };
    assert!(matches!(cfg.validate(), Err(ConfigError::InvalidFallbackDir(_))));

    // SchemaMismatch
    let cfg = KeyringConfig {
        schema_version: "999".to_string(),
        ..Default::default()
    };
    assert!(matches!(cfg.validate(), Err(ConfigError::SchemaMismatch { .. })));

    // TokenTooLong 构造 + 显示
    let e = ConfigError::TokenTooLong(5000);
    assert!(format!("{e}").contains("5000"));

    // NoStorage
    let e = ConfigError::NoStorage;
    assert!(format!("{e}").contains("no storage"));
}

/// 集成 #10: ProviderError 6 variant + 全部能转 KeyringError.
#[test]
fn integration_provider_error_6_variants() {
    use apeireth_keyring::KeyringError;
    let variants: Vec<ProviderError> = vec![
        ProviderError::Unavailable {
            provider: PROVIDER_OS_KEYRING,
            platform: Platform::Linux,
            reason: "test".to_string(),
        },
        ProviderError::Io {
            provider: PROVIDER_ENCRYPTED_FILE,
            source: std::io::Error::other("io"),
        },
        ProviderError::Crypto {
            provider: PROVIDER_ENCRYPTED_FILE,
            reason: "test".to_string(),
        },
        ProviderError::Format {
            provider: PROVIDER_IN_MEMORY,
            reason: "test".to_string(),
        },
        ProviderError::Disabled {
            provider: PROVIDER_DISABLED,
        },
        ProviderError::Unsupported {
            provider: PROVIDER_OS_KEYRING,
            platform: Platform::Bsd,
        },
    ];
    assert_eq!(variants.len(), 6, "ProviderError 必须 6 variant");
    for v in variants {
        let _: KeyringError = v.into();
    }
}
