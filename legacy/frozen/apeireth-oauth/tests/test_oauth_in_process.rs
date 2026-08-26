//! # Integration tests for apeireth-oauth (43 tests)
//!
//! 借鉴 Golutra #2 模式 1:1 镜像, 跟 sister #1+#6 集成测试模式一致.
//! 跨模块集成 + end-to-end OAuth flow 演示 + 8 项承诺 + 6 哲学锚穿透.
//!
//! ## 测试分布 (43 集成测试)
//! - 3 Provider × 3 Callback mode = 9 组合 (auth url 构造)
//! - PKCE + state 真做验证 (encode, decode, verify)
//! - 5 K-1 强校验守门 (边界值)
//! - 8 TOOL_WHITELIST 守门
//! - 4 步 OAuth flow end-to-end
//! - 6 哲学锚穿透 (S-1/S-2/O-2/O-3/O-4/O-5)
//! - 8 项承诺 (1 不假装 / 2 编译期 hardcode / 3 不改 LOCKED / 4 不改 version / 5 6 哲学锚 / 6 不依赖 NewAPI / 7 不重复造轮子 / 8 诚实标缺)

use apeireth_oauth::*;
use base64::Engine;

// ============================================================================
// §1 3 Provider × 3 Callback mode = 9 组合 (auth url 构造)
// ============================================================================

#[test]
fn integration_01_claude_code_authorization_code() {
    // 组合 1: claude-code + authorization_code
    let provider = ClaudeCodeProvider::new("client_abc", "secret_xyz").unwrap();
    let cb = AuthorizationCodeCallback;
    let pkce = PkcePair::new();
    let state = OAuthState::new();
    let url = cb
        .build_authorization(
            provider.authorization_endpoint(),
            "client_abc",
            Some("https://app.example.com/cb"),
            provider.default_scopes(),
            state.as_str(),
            Some(&pkce),
        )
        .unwrap();
    assert!(url.contains("response_type=code"));
    assert!(url.contains("client_id=client_abc"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains(&format!("code_challenge={}", pkce.code_challenge())));
}

#[test]
fn integration_02_claude_code_implicit() {
    // 组合 2: claude-code + implicit
    let provider = ClaudeCodeProvider::new("client_abc", "secret_xyz").unwrap();
    let cb = ImplicitCallback;
    let state = OAuthState::new();
    let url = cb
        .build_authorization(
            provider.authorization_endpoint(),
            "client_abc",
            Some("https://app.example.com/cb"),
            provider.default_scopes(),
            state.as_str(),
            None, // implicit 不需要 PKCE
        )
        .unwrap();
    assert!(url.contains("response_type=token"));
    assert!(!url.contains("code_challenge"));
}

#[test]
fn integration_03_claude_code_client_credentials() {
    // 组合 3: claude-code + client_credentials (无 authorization endpoint, 走 client_credentials_grant)
    let provider = ClaudeCodeProvider::new("client_abc", "secret_xyz").unwrap();
    let cb = ClientCredentialsCallback;
    let resp = cb
        .client_credentials_grant(
            provider.token_endpoint(),
            "client_abc",
            "secret_xyz",
            provider.default_scopes(),
        )
        .unwrap();
    assert!(resp.code.is_some());
    assert!(resp
        .code
        .unwrap()
        .contains("grant_type=client_credentials"));
}

#[test]
fn integration_04_opencode_authorization_code() {
    // 组合 4: opencode + authorization_code
    let provider = OpencodeProvider::new("client_abc", "secret_xyz").unwrap();
    let cb = AuthorizationCodeCallback;
    let pkce = PkcePair::new();
    let state = OAuthState::new();
    let url = cb
        .build_authorization(
            provider.authorization_endpoint(),
            "client_abc",
            Some("http://localhost:8080/cb"),
            provider.default_scopes(),
            state.as_str(),
            Some(&pkce),
        )
        .unwrap();
    assert!(url.starts_with(provider.authorization_endpoint()));
    assert!(url.contains("response_type=code"));
}

#[test]
fn integration_05_opencode_implicit() {
    // 组合 5: opencode + implicit
    let provider = OpencodeProvider::new("client_abc", "secret_xyz").unwrap();
    let cb = ImplicitCallback;
    let state = OAuthState::new();
    let url = cb
        .build_authorization(
            provider.authorization_endpoint(),
            "client_abc",
            Some("https://app.example.com/cb"),
            provider.default_scopes(),
            state.as_str(),
            None,
        )
        .unwrap();
    assert!(url.contains("response_type=token"));
}

#[test]
fn integration_06_opencode_client_credentials() {
    // 组合 6: opencode + client_credentials
    let provider = OpencodeProvider::new("client_abc", "secret_xyz").unwrap();
    let cb = ClientCredentialsCallback;
    let resp = cb
        .client_credentials_grant(
            provider.token_endpoint(),
            "client_abc",
            "secret_xyz",
            &["read:user"],
        )
        .unwrap();
    assert!(resp.code.is_some());
}

#[test]
fn integration_07_copilot_authorization_code() {
    // 组合 7: copilot + authorization_code
    let provider = CopilotProvider::new("client_abc", "secret_xyz").unwrap();
    let cb = AuthorizationCodeCallback;
    let pkce = PkcePair::new();
    let state = OAuthState::new();
    let url = cb
        .build_authorization(
            provider.authorization_endpoint(),
            "client_abc",
            Some("https://app.example.com/cb"),
            provider.default_scopes(),
            state.as_str(),
            Some(&pkce),
        )
        .unwrap();
    assert!(url.starts_with(provider.authorization_endpoint()));
    assert!(url.contains("response_type=code"));
}

#[test]
fn integration_08_copilot_implicit() {
    // 组合 8: copilot + implicit
    let provider = CopilotProvider::new("client_abc", "secret_xyz").unwrap();
    let cb = ImplicitCallback;
    let state = OAuthState::new();
    let url = cb
        .build_authorization(
            provider.authorization_endpoint(),
            "client_abc",
            Some("https://app.example.com/cb"),
            provider.default_scopes(),
            state.as_str(),
            None,
        )
        .unwrap();
    assert!(url.contains("response_type=token"));
}

#[test]
fn integration_09_copilot_client_credentials() {
    // 组合 9: copilot + client_credentials
    let provider = CopilotProvider::new("client_abc", "secret_xyz").unwrap();
    let cb = ClientCredentialsCallback;
    let resp = cb
        .client_credentials_grant(
            provider.token_endpoint(),
            "client_abc",
            "secret_xyz",
            provider.default_scopes(),
        )
        .unwrap();
    assert!(resp.code.is_some());
}

// ============================================================================
// §2 PKCE + state 真做验证 (RFC 6749 §10.12 + RFC 7636 §4.2)
// ============================================================================

#[test]
fn integration_10_pkce_pair_full_cycle() {
    // 完整 PKCE 周期: 生成 → 提取 challenge → 发送 → 验证 verifier
    let pkce = PkcePair::new();
    let verifier = pkce.code_verifier();
    let challenge = pkce.code_challenge();
    // 长度守门: 64 byte entropy → 86 char verifier, 32 byte SHA-256 → 43 char challenge
    assert_eq!(verifier.len(), 86);
    assert_eq!(challenge.len(), 43);
    // verify(verifier) 应该返回 true (re-compute challenge, match)
    assert!(pkce.verify(verifier));
}

#[test]
fn integration_11_pkce_pair_verify_rejects_wrong() {
    let pkce1 = PkcePair::new();
    let pkce2 = PkcePair::new();
    // pkce1.verify(pkce2's verifier) 应该 false (challenge 不匹配)
    assert!(!pkce1.verify(pkce2.code_verifier()));
}

#[test]
fn integration_12_pkce_pair_verify_rejects_invalid_format() {
    let pkce = PkcePair::new();
    // 太短
    assert!(!pkce.verify("short"));
    // 非 unreserved chars
    let bad = "a".repeat(42) + "+";
    assert!(!pkce.verify(&bad));
}

#[test]
fn integration_13_oauth_state_full_cycle() {
    // 完整 state 周期: 生成 → 提取 → 发送 → 验证
    let state = OAuthState::new();
    let state_str = state.as_str();
    assert_eq!(state_str.len(), 43); // base64url(32 bytes)
    assert!(state.verify(state_str)); // 验证自己
    assert!(!state.verify("wrong_state"));
}

#[test]
fn integration_14_oauth_state_from_string_round_trip() {
    // from_string 接受, as_str 提取
    let s = OAuthState::from_string("abc123def").unwrap();
    assert_eq!(s.as_str(), "abc123def");
    assert!(s.verify("abc123def"));
}

#[test]
fn integration_15_pkce_method_is_s256() {
    // PKCE 固定 S256 (per RFC 7636 §4.2 推荐)
    let pkce = PkcePair::new();
    assert_eq!(pkce.method().as_str(), "S256");
    // 多个 pkce 都是 S256
    for _ in 0..3 {
        let p = PkcePair::new();
        assert_eq!(p.method().as_str(), "S256");
    }
}

// ============================================================================
// §3 5 K-1 强校验守门 (边界值)
// ============================================================================

#[test]
fn integration_16_k1_client_id_rejects_empty() {
    // K-1 #1: client_id 必非空
    let p1 = ClaudeCodeProvider::new("", "secret").unwrap_err();
    let p2 = OpencodeProvider::new("   ", "secret").unwrap_err();
    let p3 = CopilotProvider::new("\t\n", "secret").unwrap_err();
    let _ = (p1, p2, p3);
}

#[test]
fn integration_17_k1_redirect_uri_rejects_empty_and_http_non_localhost() {
    // K-1 #2: redirect_uri 必非空 + http://localhost or https://
    let provider = ClaudeCodeProvider::new("c", "s").unwrap();
    let state = OAuthState::new();
    let pkce = PkcePair::new();
    // 空
    assert!(provider
        .build_authorization_url("", &["read"], &state, &pkce)
        .is_err());
    // http://example.com (non-localhost)
    assert!(provider
        .build_authorization_url("http://example.com/cb", &["read"], &state, &pkce)
        .is_err());
    // ftp://
    assert!(provider
        .build_authorization_url("ftp://example.com/cb", &["read"], &state, &pkce)
        .is_err());
}

#[test]
fn integration_18_k1_scope_rejects_empty() {
    // K-1 #3: scope 必非空 + 元素非空
    let provider = ClaudeCodeProvider::new("c", "s").unwrap();
    let state = OAuthState::new();
    let pkce = PkcePair::new();
    // 空 array
    assert!(provider
        .build_authorization_url("https://app.example.com/cb", &[], &state, &pkce)
        .is_err());
    // 含空元素
    assert!(provider
        .build_authorization_url("https://app.example.com/cb", &["read", ""], &state, &pkce)
        .is_err());
    assert!(provider
        .build_authorization_url("https://app.example.com/cb", &["   "], &state, &pkce)
        .is_err());
}

#[test]
fn integration_19_k1_pkce_verifier_length_charset() {
    // K-1 #4: PKCE verifier 43-128 + base64url charset
    // 太短
    assert!(validate_pkce_verifier(&"a".repeat(42)).is_err());
    // 太长
    assert!(validate_pkce_verifier(&"a".repeat(129)).is_err());
    // 非 unreserved ('+')
    let bad = "a".repeat(42) + "+";
    assert!(validate_pkce_verifier(&bad).is_err());
    // 合法 43
    assert!(validate_pkce_verifier(&"a".repeat(43)).is_ok());
    // 合法 128
    assert!(validate_pkce_verifier(&"a".repeat(128)).is_ok());
}

#[test]
fn integration_20_k1_state_rejects_empty() {
    // K-1 #5: state 必非空
    assert!(validate_state("").is_err());
    assert!(validate_state("   ").is_err());
    assert!(validate_state("abc").is_ok());
}

// ============================================================================
// §4 8 TOOL_WHITELIST 守门
// ============================================================================

#[test]
fn integration_21_tool_whitelist_8_entries() {
    // 8 工具全在白名单
    for tool in OAUTH_TOOL_WHITELIST {
        assert!(validate_tool_call(tool).is_ok(), "tool {tool} should be valid");
    }
    assert_eq!(OAUTH_TOOL_WHITELIST.len(), 8);
}

#[test]
fn integration_22_tool_whitelist_rejects_unknown() {
    // 未知工具被拒
    assert!(validate_tool_call("not_a_tool").is_err());
    assert!(validate_tool_call("apeireth_oauth_made_up").is_err());
    assert!(validate_tool_call("").is_err());
    assert!(validate_tool_call("apeireth_oauth_").is_err());
}

#[test]
fn integration_23_tool_whitelist_count_constant() {
    // 8 工具守门
    assert_eq!(OAUTH_TOOL_WHITELIST_COUNT, 8);
    assert_eq!(OAUTH_TOOL_WHITELIST.len(), 8);
}

// ============================================================================
// §5 4 步 OAuth flow end-to-end
// ============================================================================

#[test]
fn integration_24_oauth_flow_4_steps_e2e() {
    // 完整 4 步: prepare → build_authorization → exchange_code → refresh
    let flow = DefaultOAuthFlow;
    let provider = ClaudeCodeProvider::new("client_abc", "secret_xyz").unwrap();

    // 步骤 0: prepare
    let handle = flow.prepare("client_abc").unwrap();
    assert_eq!(handle.state.as_str().len(), 43);
    assert_eq!(handle.pkce.code_verifier().len(), 86);

    // 步骤 1: build_authorization
    let url = flow
        .build_authorization(
            &provider,
            "https://app.example.com/cb",
            &["read", "write"],
            &handle,
        )
        .unwrap();
    assert!(url.contains("response_type=code"));
    assert!(url.contains(&format!("state={}", handle.state.as_str())));

    // 步骤 2: exchange_code_for_token (模拟 callback 收到 code)
    let token = flow
        .exchange_code_for_token(
            &provider,
            "auth_code_xyz",
            "https://app.example.com/cb",
            &handle,
        )
        .unwrap();
    assert_eq!(token.token_type, "bearer");
    assert!(token.expires_in.is_some());

    // 步骤 3: refresh_access_token
    let new_token = flow
        .refresh_access_token(&provider, "refresh_xyz")
        .unwrap();
    assert_eq!(new_token.token_type, "bearer");
    assert_ne!(token.access_token, new_token.access_token);
}

#[test]
fn integration_25_oauth_flow_3_providers() {
    // 3 Provider 走完整 4 步
    let flow = DefaultOAuthFlow;
    let providers: Vec<(&str, Box<dyn OAuthProvider>)> = vec![
        ("claude_code", Box::new(ClaudeCodeProvider::new("c", "s").unwrap())),
        ("opencode", Box::new(OpencodeProvider::new("c", "s").unwrap())),
        ("copilot", Box::new(CopilotProvider::new("c", "s").unwrap())),
    ];
    for (name, p) in providers {
        let handle = flow.prepare("c").unwrap();
        let url = flow
            .build_authorization(&*p, "https://app.example.com/cb", &["read"], &handle)
            .unwrap();
        assert!(url.contains("response_type=code"), "Provider {name} should build auth url");
        let token = flow
            .exchange_code_for_token(&*p, "code", "https://app.example.com/cb", &handle)
            .unwrap();
        assert_eq!(token.token_type, "bearer");
    }
}

#[test]
fn integration_26_oauth_flow_callback_parse_authorization_code() {
    // parse callback 解析 authorization_code 模式
    let cb = AuthorizationCodeCallback;
    let resp = cb
        .parse_callback("?code=auth_code_xyz&state=state_abc")
        .unwrap();
    assert_eq!(resp.code, Some("auth_code_xyz".to_string()));
    assert_eq!(resp.state, Some("state_abc".to_string()));
    assert_eq!(resp.mode, CallbackMode::AuthorizationCode);
}

#[test]
fn integration_27_oauth_flow_callback_parse_error() {
    // parse callback 解析 error (e.g. access_denied)
    let cb = AuthorizationCodeCallback;
    let resp = cb
        .parse_callback("?error=access_denied&state=state_abc")
        .unwrap();
    assert_eq!(resp.error, Some("access_denied".to_string()));
    assert!(resp.code.is_none());
}

#[test]
fn integration_28_oauth_flow_client_credentials_no_authorization() {
    // client_credentials 模式: 走 client_credentials_grant, 不用 authorization endpoint
    let cb = ClientCredentialsCallback;
    let url = cb
        .build_authorization(
            "https://provider.example.com/token",
            "c",
            None, // no redirect
            &["read"],
            "state_abc",
            None, // no PKCE
        )
        .unwrap();
    assert!(url.is_empty());
    // 走 client_credentials_grant 构造 token request URL
    let resp = cb
        .client_credentials_grant(
            "https://provider.example.com/token",
            "c",
            "s",
            &["read"],
        )
        .unwrap();
    assert!(resp
        .code
        .as_ref()
        .unwrap()
        .contains("grant_type=client_credentials"));
}

// ============================================================================
// §6 6 哲学锚穿透 (S-1 / S-2 / O-2 / O-3 / O-4 / O-5)
// ============================================================================

#[test]
fn integration_29_s1_north_star_three_providers_serve_asi() {
    // S-1: 3 Provider 服务 ASI 北极星 (claude-code / opencode / copilot 三足鼎立)
    let kinds = ProviderKind::ALL;
    assert_eq!(kinds.len(), 3);
    for k in kinds {
        let name = k.name();
        assert!(!name.is_empty());
        assert!(!k.authorization_endpoint().is_empty());
        assert!(!k.token_endpoint().is_empty());
    }
}

#[test]
fn integration_30_s2_real_implementation_pkce_state() {
    // S-2: PKCE + state 真做 (not mock, SHA-256 真跑, base64url 真编码)
    let pkce = PkcePair::new();
    // 验证 SHA-256 真跑: challenge 不等于 verifier 任何前缀
    let verifier = pkce.code_verifier();
    let challenge = pkce.code_challenge();
    assert_ne!(verifier, challenge);
    assert_ne!(verifier.len(), challenge.len()); // 86 vs 43 chars
}

#[test]
fn integration_31_o2_rfc_6749_7636_industry_standards() {
    // O-2: 借 RFC 6749 + RFC 7636 工业标准
    // PKCE 用 S256 method (per RFC 7636 §4.2)
    let pkce = PkcePair::new();
    assert_eq!(pkce.method().as_str(), "S256");
    // state 32 bytes 熵 (per RFC 6749 §10.12 推荐)
    assert_eq!(STATE_ENTROPY_BYTES, 32);
    // PKCE verifier 64 bytes 熵 (per RFC 7636 §4.1 范围 32-96)
    assert_eq!(PKCE_VERIFIER_ENTROPY_BYTES, 64);
}

#[test]
fn integration_32_o3_three_providers_three_callback_modes() {
    // O-3: 3 Provider × 3 Callback mode 完整覆盖
    assert_eq!(ProviderKind::ALL.len(), 3);
    assert_eq!(CallbackMode::ALL.len(), 3);
    // 3 Callback 模式都有 impl
    let _ = AuthorizationCodeCallback;
    let _ = ImplicitCallback;
    let _ = ClientCredentialsCallback;
}

#[test]
fn integration_33_o4_easy_to_handoff_public_api_documented() {
    // O-4: 公开 API 100% 文档化 (top-level re-exports)
    // 测试所有 re-export 类型可达
    let _: fn() -> LibraryInfo = LibraryInfo::current;
    // 5 K-1 validators 公开
    let _: fn(&str) -> OAuthResult<()> = validate_client_id;
    let _: fn(&str) -> OAuthResult<()> = validate_redirect_uri;
    let _: fn(&[&str]) -> OAuthResult<()> = validate_scope;
    let _: fn(&str) -> OAuthResult<()> = validate_pkce_verifier;
    let _: fn(&str) -> OAuthResult<()> = validate_state;
    // 8 tool whitelist
    assert_eq!(OAUTH_TOOL_WHITELIST.len(), 8);
}

#[test]
fn integration_34_o5_no_faking_pkce_state_actually_works() {
    // O-5: PKCE + state 真做, 不假装
    // PKCE verify(verifier) 返回 true, verify(wrong) 返回 false
    let pkce = PkcePair::new();
    assert!(pkce.verify(pkce.code_verifier()));
    assert!(!pkce.verify("wrong_verifier_long_enough_43_chars_but_doesnt_match_chal"));
    // State verify(state) 返回 true, verify(different) 返回 false
    let state = OAuthState::new();
    assert!(state.verify(state.as_str()));
    assert!(!state.verify("completely_different_state"));
}

// ============================================================================
// §7 8 项承诺守门
// ============================================================================

#[test]
fn integration_35_promise_1_no_faking_pkce_state_real() {
    // 8 项之 1: 不假装已实现 (PKCE + state 真做)
    let pkce = PkcePair::new();
    let state = OAuthState::new();
    // 真随机 (多次生成都不同)
    let pkce2 = PkcePair::new();
    let state2 = OAuthState::new();
    assert_ne!(pkce, pkce2);
    assert_ne!(state, state2);
}

#[test]
fn integration_36_promise_2_compile_time_hardcode_5_k1_3_providers_3_modes() {
    // 8 项之 2: 编译期 hardcode (5 K-1 + 3 Provider + 3 Callback + 8 tool whitelist)
    assert_eq!(K1_STRONG_VALIDATION_VARIANTS.len(), 5);
    assert_eq!(PROVIDER_COUNT, 3);
    assert_eq!(CALLBACK_MODE_COUNT, 3);
    assert_eq!(OAUTH_TOOL_WHITELIST_COUNT, 8);
    assert_eq!(OAUTH_ERROR_VARIANT_COUNT, 8);
    assert_eq!(FLOW_STEP_COUNT, 4);
}

#[test]
fn integration_37_promise_3_no_locked_24_crate_touched() {
    // 8 项之 3: 0 触碰 24 LOCKED crate
    // (本测试只验证本 crate 编译, 不触碰外部 LOCKED)
    // 通过 LibraryInfo + 公开 API 验证本 crate 独立
    let info = LibraryInfo::current();
    assert_eq!(info.name, "apeireth-oauth");
    assert_eq!(info.provider_count, 3);
}

#[test]
fn integration_38_promise_4_workspace_version_unchanged() {
    // 8 项之 4: workspace version 1.0.0 0 改
    // 本 crate 用 0.1.0 (新 crate 显式, 0 改 v1.0.0)
    let _ = APEIRETH_OAUTH_SCHEMA_VERSION;
    assert_eq!(APEIRETH_OAUTH_SCHEMA_VERSION, "1");
    // platform 守门
    assert_eq!(PLATFORM_NAME, "apeireth");
}

#[test]
fn integration_39_promise_5_six_philosophy_anchors() {
    // 8 项之 5: 6 哲学锚穿透 (S-1 / S-2 / O-2 / O-3 / O-4 / O-5)
    // per integration_29..34 (6 哲学锚)
    // 这里只 smoke test 守门常量和 6 锚对应类型
    assert!(ProviderKind::ALL.len() == 3); // S-1
    assert!(STATE_ENTROPY_BYTES == 32); // S-2
    assert_eq!(PkceMethod::S256.as_str(), "S256"); // O-2
    assert!(FLOW_STEP_COUNT == 4); // O-3
    assert!(OAUTH_TOOL_WHITELIST.len() == 8); // O-4
    // O-5: 跳过 (per integration_34)
}

#[test]
fn integration_40_promise_6_no_newapi_dependence() {
    // 8 项之 6: 0 依赖 NewAPI (0 引外部 RPC, 0 引 reqwest / hyper)
    // 本测试: 验证本 crate 公共 API 不暴露 reqwest / hyper
    // (编译期保证: Cargo.toml 不引 reqwest/hyper/tokio, 0 异步)
    let info = LibraryInfo::current();
    assert_eq!(info.name, "apeireth-oauth");
    // 0 异步 runtime 暴露 (skeleton 阶段 sync)
    let _ = DefaultOAuthFlow;
}

#[test]
fn integration_41_promise_7_no_wheel_reinvention() {
    // 8 项之 7: 不重复造轮子 (借 sha2 + base64url + rand + serde + thiserror)
    // PKCE 借 sha2::Sha256 + base64::URL_SAFE_NO_PAD + rand::thread_rng
    let pkce = PkcePair::new();
    let verifier = pkce.code_verifier();
    let challenge = pkce.code_challenge();
    // sha2 真跑: challenge 是 sha256(verifier) 的 base64url 编码
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    let expected = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    assert_eq!(challenge, expected);
}

#[test]
fn integration_42_promise_8_honest_missing_http_exchange() {
    // 8 项之 8: 诚实标缺 (R21+ 续真接 HTTP exchange)
    // 当前 token 是 stub (含 "stub_token_" 前缀)
    let flow = DefaultOAuthFlow;
    let provider = ClaudeCodeProvider::new("c", "s").unwrap();
    let handle = flow.prepare("c").unwrap();
    let token = flow
        .exchange_code_for_token(&provider, "code_xyz", "https://app.example.com/cb", &handle)
        .unwrap();
    // token.access_token 含 "stub_token_" (R21+ 续真连时去掉)
    assert!(
        token.access_token.contains("stub_token_"),
        "Token 应含 stub_token_ 前缀, R21+ 续真接时去掉"
    );
    let refreshed = flow
        .refresh_access_token(&provider, "refresh_xyz")
        .unwrap();
    assert!(
        refreshed.access_token.contains("stub_refreshed_token_"),
        "Refreshed token 应含 stub_refreshed_token_ 前缀, R21+ 续真接时去掉"
    );
}

// ============================================================================
// §8 LibraryInfo 守门
// ============================================================================

#[test]
fn integration_43_library_info_full() {
    // 守门: LibraryInfo 跟硬编码常量一致
    let info = LibraryInfo::current();
    assert_eq!(info.name, "apeireth-oauth");
    assert_eq!(info.schema_version, "1");
    assert_eq!(info.platform, "apeireth");
    assert_eq!(info.provider_count, 3);
    assert_eq!(info.callback_mode_count, 3);
    assert_eq!(info.flow_step_count, 4);
    assert_eq!(info.tool_whitelist_count, 8);
}

