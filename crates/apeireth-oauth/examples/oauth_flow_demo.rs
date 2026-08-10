//! # OAuth Flow Demo
//!
//! 借鉴 Golutra #2 模式 1:1 镜像, 演示 3 Provider × 3 Callback mode + PKCE + state 真做.
//!
//! ## 8 段演示
//! 1. 3 Provider 注册 (claude-code / opencode / copilot)
//! 2. 3 Callback mode 准备 (AuthorizationCode / Implicit / ClientCredentials)
//! 3. PKCE pair 生成 (SHA-256 真跑 + base64url 真编码)
//! 4. State 生成 (32 字节熵 + base64url)
//! 5. 4 步 OAuth flow (prepare / build_authorization / exchange_code / refresh)
//! 6. 3 Provider × 3 Callback mode = 9 组合 authorization URL 构造
//! 7. Callback 解析 (authorization_code 模式 + error 模式)
//! 8. 8 TOOL_WHITELIST 守门

use apeireth_oauth::*;

fn main() {
    println!("=== apeireth-oauth demo: 借鉴 Golutra #2 模式 (3 Provider × 3 Callback mode) ===\n");

    // ---- Demo 1: 3 Provider 注册 ----
    println!("--- Demo 1: 3 Provider 注册 (claude-code / opencode / copilot) ---");
    let providers: Vec<(&str, Box<dyn OAuthProvider>)> = vec![
        ("claude_code", Box::new(ClaudeCodeProvider::new("client_abc", "secret_xyz").unwrap())),
        ("opencode", Box::new(OpencodeProvider::new("client_abc", "secret_xyz").unwrap())),
        ("copilot", Box::new(CopilotProvider::new("client_abc", "secret_xyz").unwrap())),
    ];
    for (name, p) in &providers {
        println!(
            "  [{}] auth={}, token={}, scopes={:?}, public_client={}",
            name,
            p.authorization_endpoint(),
            p.token_endpoint(),
            p.default_scopes(),
            p.kind().is_default_public_client(),
        );
    }

    // ---- Demo 2: 3 Callback mode 准备 ----
    println!("\n--- Demo 2: 3 Callback mode (authorization_code / implicit / client_credentials) ---");
    let auth_code = AuthorizationCodeCallback;
    let implicit = ImplicitCallback;
    let client_creds = ClientCredentialsCallback;
    println!(
        "  [{}] user_interaction={}, redirect_uri={}, pkce={}",
        auth_code.mode().as_str(),
        auth_code.mode().requires_user_interaction(),
        auth_code.mode().requires_redirect_uri(),
        auth_code.mode().requires_pkce(),
    );
    println!(
        "  [{}] user_interaction={}, redirect_uri={}, pkce={}",
        implicit.mode().as_str(),
        implicit.mode().requires_user_interaction(),
        implicit.mode().requires_redirect_uri(),
        implicit.mode().requires_pkce(),
    );
    println!(
        "  [{}] user_interaction={}, redirect_uri={}, pkce={}",
        client_creds.mode().as_str(),
        client_creds.mode().requires_user_interaction(),
        client_creds.mode().requires_redirect_uri(),
        client_creds.mode().requires_pkce(),
    );

    // ---- Demo 3: PKCE pair 生成 (SHA-256 真跑 + base64url) ----
    println!("\n--- Demo 3: PKCE pair 生成 (RFC 7636 §4.2, S256 method) ---");
    let pkce = PkcePair::new();
    println!("  code_verifier  ({} chars): {}...", pkce.code_verifier().len(), &pkce.code_verifier()[..20]);
    println!("  code_challenge ({} chars): {}...", pkce.code_challenge().len(), &pkce.code_challenge()[..20]);
    println!("  method: {}", pkce.method().as_str());
    println!("  verify(self) = {} (re-compute SHA-256 + base64url, 应该 true)", pkce.verify(pkce.code_verifier()));

    // ---- Demo 4: State 生成 (32 字节熵 + base64url) ----
    println!("\n--- Demo 4: State 生成 (RFC 6749 §10.12, 32 字节熵) ---");
    let state = OAuthState::new();
    println!("  state ({} chars): {}...", state.as_str().len(), &state.as_str()[..20]);
    println!("  verify(self) = {} (CSRF 防御, 应该 true)", state.verify(state.as_str()));

    // ---- Demo 5: 4 步 OAuth flow (claude-code + authorization_code) ----
    println!("\n--- Demo 5: 4 步 OAuth flow (claude-code + authorization_code) ---");
    let flow = DefaultOAuthFlow;
    let claude = ClaudeCodeProvider::new("client_abc", "secret_xyz").unwrap();

    // 步骤 0: prepare
    let handle = flow.prepare("client_abc").unwrap();
    println!("  Step 0 prepare: state={}..., pkce_verifier={}...", &handle.state.as_str()[..10], &handle.pkce.code_verifier()[..10]);

    // 步骤 1: build_authorization
    let auth_url = flow
        .build_authorization(&claude, "https://app.example.com/cb", &["read", "write"], &handle)
        .unwrap();
    println!("  Step 1 build_authorization: {}...", &auth_url[..80]);

    // 步骤 2: exchange_code_for_token (模拟 callback 收到 code)
    let token = flow
        .exchange_code_for_token(&claude, "auth_code_xyz_123", "https://app.example.com/cb", &handle)
        .unwrap();
    println!(
        "  Step 2 exchange_code: token_type={}, expires_in={:?}, access_token={}...",
        token.token_type,
        token.expires_in,
        &token.access_token[..30]
    );

    // 步骤 3: refresh_access_token
    let new_token = flow.refresh_access_token(&claude, "refresh_xyz_456").unwrap();
    println!(
        "  Step 3 refresh: token_type={}, access_token={}...",
        new_token.token_type,
        &new_token.access_token[..30]
    );

    // ---- Demo 6: 3 Provider × 3 Callback mode = 9 组合 ----
    println!("\n--- Demo 6: 3 Provider × 3 Callback mode = 9 组合 authorization URL ---");
    let callbacks: Vec<(&str, Box<dyn OAuthCallback>)> = vec![
        ("authorization_code", Box::new(AuthorizationCodeCallback)),
        ("implicit", Box::new(ImplicitCallback)),
        ("client_credentials", Box::new(ClientCredentialsCallback)),
    ];
    for (p_name, p) in &providers {
        for (c_name, cb) in &callbacks {
            let handle = flow.prepare("c").unwrap();
            // client_credentials 模式用 client_credentials_grant, 其他用 build_authorization
            let result = if cb.mode() == CallbackMode::ClientCredentials {
                cb.client_credentials_grant(
                    p.token_endpoint(),
                    "client_abc",
                    "secret_xyz",
                    p.default_scopes(),
                )
                .ok()
                .and_then(|r| r.code)
            } else {
                cb.build_authorization(
                    p.authorization_endpoint(),
                    "client_abc",
                    Some("https://app.example.com/cb"),
                    p.default_scopes(),
                    handle.state.as_str(),
                    if cb.mode() == CallbackMode::AuthorizationCode { Some(&handle.pkce) } else { None },
                )
                .ok()
            };
            match result {
                Some(url) => println!("  [{} + {}] {}...", p_name, c_name, &url[..80.min(url.len())]),
                None => println!("  [{} + {}] ERROR", p_name, c_name),
            }
        }
    }

    // ---- Demo 7: Callback 解析 ----
    println!("\n--- Demo 7: Callback 解析 (authorization_code 模式) ---");
    let cb = AuthorizationCodeCallback;
    let success_resp = cb
        .parse_callback("?code=auth_code_xyz&state=state_abc")
        .unwrap();
    println!(
        "  success: code={:?}, state={:?}, error={:?}",
        success_resp.code, success_resp.state, success_resp.error
    );
    let error_resp = cb
        .parse_callback("?error=access_denied&state=state_abc")
        .unwrap();
    println!(
        "  error: code={:?}, state={:?}, error={:?}",
        error_resp.code, error_resp.state, error_resp.error
    );

    // ---- Demo 8: 8 TOOL_WHITELIST 守门 ----
    println!("\n--- Demo 8: 8 TOOL_WHITELIST 守门 ---");
    for (i, tool) in OAUTH_TOOL_WHITELIST.iter().enumerate() {
        assert!(validate_tool_call(tool).is_ok());
        println!("  [{}/{}] {} OK", i + 1, OAUTH_TOOL_WHITELIST.len(), tool);
    }
    assert!(validate_tool_call("not_a_real_tool").is_err());
    println!("  [reject] not_a_real_tool → ERR (m3 防御)");

    // ---- LibraryInfo ----
    println!("\n--- LibraryInfo ---");
    let info = LibraryInfo::current();
    println!("  name={}, schema_version={}, platform={}", info.name, info.schema_version, info.platform);
    println!(
        "  provider_count={}, callback_mode_count={}, flow_step_count={}, tool_whitelist_count={}",
        info.provider_count, info.callback_mode_count, info.flow_step_count, info.tool_whitelist_count
    );

    println!("\n=== 8 段演示完成 (0 panic, 0 错误) ===");
}
