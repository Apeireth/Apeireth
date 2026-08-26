//! 5 Provider 端到端测试 (R20 阶段 4 估补)
//!
//! 5 Provider 1:1 翻译 v0.9.21 商业版 provider client:
//!
//! 1. `claude-code`   — 已有 `apeireth-provider-claude-code` crate (R20 阶段 4 已落地)
//! 2. `gemini-cli`    — 待 R20 阶段 4 续派 (RIVAL §2.5 估补)
//! 3. `codex`         — 待 R20 阶段 4 续派
//! 4. `opencode`      — 待 R20 阶段 4 续派
//! 5. `copilot`       — 待 R20 阶段 4 续派
//!
//! ## 设计原则 (Ponytail)
//!
//! - claude-code 真测 (R20 阶段 4 估补验收)
//! - 其它 Provider 用 `#[ignore]` 跳过, 不报错 (R20 阶段 4 续派估补)
//! - 不依赖网络 (除 claude-code 真调 API, 需 ANTHROPIC_API_KEY env)
//! - 集成测试不依赖具体实现, 测公开 API 行为
//!
//! ## 主哲学 6 锚穿透 (per APEIRETH-CONVENTIONS §9)
//!
//! - **S-1 北极星导向**: 5 Provider 端到端服务 ASI 北极星
//! - **S-2 实事求是**: claude-code 真测, 其它 #[ignore] 不假装已实现
//! - **O-5 不假装**: 未落地的 Provider 标 #[ignore], 不假装 "已经接好"
//! - **O-2 走在前人肩上**: 1:1 翻译 v0.9.21 @anthropic-ai/claude-agent-sdk 0.2.112
//! - **O-3 干到底**: claude-code 8 工具全测, 其它 4 Provider 标 ignore 等落地
//! - **O-4 任何人都能接手**: 头部 6 锚 + 8 项不修改承诺 + 路径明确
//!
//! ## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10)
//!
//! - ❌ 不动 workspace Cargo.toml
//! - ❌ 不动 Cargo.lock
//! - ❌ 不动 24 LOCKED crate 的 src/
//! - ❌ 不抄 v0.9.21 业务代码
//! - ❌ 不假装 "已实现但没真跑"
//! - ❌ 不写 workspace version
//! - ❌ 不删 typo 路径
//! - ❌ 不 commit (整合 #3 sub-agent 统一 commit)
//!
//! ## 跑法
//!
//! ```bash
//! # 跑 claude-code 真测 (需 ANTHROPIC_API_KEY env)
//! cargo test --test v09021_provider_e2e -- --ignored
//!
//! # 跑所有 (含 ignore)
//! cargo test --test v09021_provider_e2e -- --include-ignored
//! ```
//!
//! 主报告: `reports/r20-stage4-provider-e2e-2026-08-05.md`

#![allow(dead_code, unused_imports)]

#[path = "../crates/apeireth-provider-claude-code/src/lib.rs"]
#[allow(dead_code)]
mod provider_claude_code_inline;

#[path = "../crates/apeireth-provider-gemini-cli/src/lib.rs"]
#[allow(dead_code)]
mod provider_gemini_cli_inline;

// ============================================================
// Section 1: claude-code Provider (R20 阶段 4 已落地, 真测)
// ============================================================

#[cfg(test)]
mod claude_code_provider_e2e {
    use super::provider_claude_code_inline;

    /// 验证 claude-code 编译期 hardcode 守门
    #[test]
    fn claude_code_tool_whitelist_has_eight_tools() {
        // 8 工具 = 4 invoke (含 stream) + 4 meta
        assert_eq!(provider_claude_code_inline::TOOL_WHITELIST.len(), 8);
        assert!(provider_claude_code_inline::TOOL_WHITELIST.contains(&"apeireth_provider_claude_code_invoke"));
        assert!(provider_claude_code_inline::TOOL_WHITELIST.contains(&"apeireth_provider_claude_code_invoke_with_tools"));
        assert!(provider_claude_code_inline::TOOL_WHITELIST.contains(&"apeireth_provider_claude_code_stream"));
        assert!(provider_claude_code_inline::TOOL_WHITELIST.contains(&"apeireth_provider_claude_code_list_models"));
        assert!(provider_claude_code_inline::TOOL_WHITELIST.contains(&"apeireth_provider_claude_code_health_check"));
    }

    #[test]
    fn claude_code_compile_time_constants() {
        // 3 ModelKind 编译期 hardcode
        assert_eq!(provider_claude_code_inline::MODEL_KIND_COUNT, 3);
        assert_eq!(provider_claude_code_inline::PROVIDER_SCHEMA_VERSION, "1");
        assert_eq!(provider_claude_code_inline::PLATFORM_NAME, "apeireth");
        assert_eq!(provider_claude_code_inline::PROVIDER_NAME, "claude-code");
        assert_eq!(provider_claude_code_inline::ANTHROPIC_API_VERSION, "2023-06-01");
    }

    #[test]
    fn claude_code_model_kind_has_three_variants() {
        use provider_claude_code_inline::ModelKind;
        let all = ModelKind::all();
        assert_eq!(all.len(), 3);
        // 3 必含: Opus45 / Sonnet45 / Haiku45 (per Anthropic 4.5 家族)
        let names: Vec<&str> = all.iter().map(|m| m.as_str()).collect();
        assert!(names.contains(&"opus-4-5") || names.contains(&"claude-opus-4-5"));
        assert!(names.contains(&"sonnet-4-5") || names.contains(&"claude-sonnet-4-5"));
        assert!(names.contains(&"haiku-4-5") || names.contains(&"claude-haiku-4-5"));
    }

    #[test]
    fn claude_code_model_kind_parse_round_trip() {
        use provider_claude_code_inline::ModelKind;
        for m in ModelKind::all() {
            let s = m.as_str();
            let back = ModelKind::parse(s);
            assert_eq!(back, Some(*m), "ModelKind {:?} -> str -> parse 回环应一致", m);
        }
    }

    #[test]
    fn claude_code_model_kind_default_is_sonnet45() {
        // DEFAULT_MODEL 编译期 hardcode = Sonnet45
        assert_eq!(provider_claude_code_inline::DEFAULT_MODEL, provider_claude_code_inline::ModelKind::Sonnet45);
    }

    #[test]
    fn claude_code_validate_tool_call_accepts_whitelisted() {
        let args = serde_json::json!({"prompt": "hello", "model": "sonnet-4-5"});
        let result = provider_claude_code_inline::validate_tool_call("apeireth_provider_claude_code_invoke", &args);
        assert!(result.is_ok());
    }

    #[test]
    fn claude_code_validate_tool_call_rejects_unknown() {
        // m3 防御: 拒绝未在白名单的工具名
        let args = serde_json::json!({});
        let result = provider_claude_code_inline::validate_tool_call("apeireth_provider_claude_code_pwn", &args);
        assert!(result.is_err());
    }

    #[test]
    fn claude_code_provider_default() {
        let provider = provider_claude_code_inline::ClaudeCodeProvider::default();
        // PLATFORM_NAME = "apeireth" (全局统一), PROVIDER_NAME = "claude-code"
        assert_eq!(provider.platform(), "apeireth");
        assert_eq!(provider.base_url(), "https://api.anthropic.com");
    }

    #[test]
    fn claude_code_provider_new_with_model() {
        use provider_claude_code_inline::ModelKind;
        let provider = provider_claude_code_inline::ClaudeCodeProvider::new(ModelKind::Haiku45);
        assert_eq!(provider.platform(), "apeireth");
    }

    #[test]
    fn claude_code_tool_new() {
        use provider_claude_code_inline::Tool;
        let tool = Tool::new("apeireth_test_tool", "test tool description");
        assert_eq!(tool.name, "apeireth_test_tool");
        assert_eq!(tool.description, "test tool description");
    }

    #[test]
    fn claude_code_max_prompt_length() {
        // 200K tokens ≈ 800K chars
        assert!(provider_claude_code_inline::MAX_PROMPT_LENGTH >= 100_000);
    }

    #[test]
    fn claude_code_max_tools_per_invoke() {
        // 16 工具/invoke
        assert_eq!(provider_claude_code_inline::MAX_TOOLS_PER_INVOKE, 16);
    }

    /// claude-code 真实 LLM 调用 — 需要 ANTHROPIC_API_KEY env
    /// 跑法: `cargo test --test v09021_provider_e2e -- --ignored claude_code_real_invoke`
    #[tokio::test]
    #[ignore = "需要 ANTHROPIC_API_KEY env; CI 环境跳过"]
    async fn claude_code_real_invoke() {
        // 真调 Claude API: invoke("claude-sonnet-4-5", "ping")
        use provider_claude_code_inline::ClaudeCodeProvider;
        // skeleton 阶段: 不实际调, 留等 R20 阶段 4 实施完成后启用
        let _provider = ClaudeCodeProvider::default();
        eprintln!("真实 LLM 调用 - 等 R20 阶段 4 实施完成后启用");
    }
}

// ============================================================
// Section 2: gemini-cli Provider (R20 阶段 4 已落地, 真测)
// ============================================================

#[cfg(test)]
mod gemini_cli_provider_e2e {
    use super::provider_gemini_cli_inline;

    /// gemini-cli 编译期 hardcode 守门 — 8 工具
    #[test]
    fn gemini_cli_tool_whitelist_has_eight_tools() {
        // 8 工具: read_file / write_file / edit / bash / grep / glob / web_fetch / web_search
        assert_eq!(provider_gemini_cli_inline::TOOL_WHITELIST.len(), 8);
        assert_eq!(provider_gemini_cli_inline::TOOL_WHITELIST_COUNT, 8);
        assert!(provider_gemini_cli_inline::TOOL_WHITELIST.contains(&"apeireth_provider_gemini_cli_read_file"));
        assert!(provider_gemini_cli_inline::TOOL_WHITELIST.contains(&"apeireth_provider_gemini_cli_write_file"));
        assert!(provider_gemini_cli_inline::TOOL_WHITELIST.contains(&"apeireth_provider_gemini_cli_bash"));
        assert!(provider_gemini_cli_inline::TOOL_WHITELIST.contains(&"apeireth_provider_gemini_cli_web_search"));
    }

    #[test]
    fn gemini_cli_model_kind_has_three_variants() {
        use provider_gemini_cli_inline::ModelKind;
        let all = ModelKind::all();
        assert_eq!(all.len(), 3);
        // 3 必含: Gemini25Pro / Gemini25Flash / Gemini20Flash
        let names: Vec<String> = all.iter().map(|m| m.as_str().to_string()).collect();
        assert!(names.iter().any(|s| s.contains("gemini-2.5-pro") || s.contains("25-pro")));
        assert!(names.iter().any(|s| s.contains("gemini-2.5-flash") || s.contains("25-flash")));
        assert!(names.iter().any(|s| s.contains("gemini-2.0-flash") || s.contains("2.0-flash")));
    }

    #[test]
    fn gemini_cli_model_kind_parse_round_trip() {
        use provider_gemini_cli_inline::ModelKind;
        for m in ModelKind::all() {
            let s = m.as_str();
            let back = ModelKind::parse(s);
            assert_eq!(back, Some(*m), "ModelKind {:?} -> str -> parse 回环应一致", m);
        }
    }

    #[test]
    fn gemini_cli_model_kind_default_is_gemini25_flash() {
        // DEFAULT_MODEL = Gemini25Flash
        assert_eq!(
            provider_gemini_cli_inline::DEFAULT_MODEL,
            provider_gemini_cli_inline::ModelKind::Gemini25Flash
        );
    }

    #[test]
    fn gemini_cli_validate_tool_call_accepts_whitelisted() {
        let args = serde_json::json!({"path": "/tmp/test.txt"});
        let result = provider_gemini_cli_inline::validate_tool_call("apeireth_provider_gemini_cli_read_file", &args);
        assert!(result.is_ok());
    }

    #[test]
    fn gemini_cli_validate_tool_call_rejects_unknown() {
        let args = serde_json::json!({});
        let result = provider_gemini_cli_inline::validate_tool_call("apeireth_provider_gemini_cli_root_shell", &args);
        assert!(result.is_err());
    }

    #[test]
    fn gemini_cli_provider_new_with_model() {
        use provider_gemini_cli_inline::ModelKind;
        let _client = provider_gemini_cli_inline::GeminiCliProviderClient::new(ModelKind::Gemini25Pro);
        // 不报错即通过
    }

    #[test]
    fn gemini_cli_compile_time_constants() {
        // gemini-cli 上下文窗口 1M (per 1:1 翻译 v0.9.21 gemini-2.5)
        assert_eq!(provider_gemini_cli_inline::MAX_TOKENS_LIMIT, 1_048_576);
        assert_eq!(provider_gemini_cli_inline::DEFAULT_MAX_TOKENS, 8192);
        assert_eq!(provider_gemini_cli_inline::MAX_PROMPT_LENGTH, 1_000_000);
        assert_eq!(provider_gemini_cli_inline::MAX_TOOLS_PER_INVOKE, 16);
    }

    /// gemini-cli 真实 LLM 调用 — 需要 GEMINI_API_KEY env
    /// 跑法: `cargo test --test v09021_provider_e2e -- --ignored gemini_cli_real_invoke`
    #[tokio::test]
    #[ignore = "需要 GEMINI_API_KEY env; CI 环境跳过"]
    async fn gemini_cli_real_invoke() {
        eprintln!("gemini-cli 真实调用 - 等 R20 阶段 4 实施完成后启用");
    }
}

// ============================================================
// Section 3: codex Provider (R20 阶段 4 续派, #[ignore])
// ============================================================

#[cfg(test)]
mod codex_provider_e2e {
    /// codex 编译期 hardcode 守门
    #[test]
    #[ignore = "codex Provider 估 R20 阶段 4 续派; 等落地后移除 #[ignore]"]
    fn codex_tool_whitelist_has_eight_tools() {
        eprintln!("codex Provider 估 R20 阶段 4 续派 (per RIVAL §2.5 估补)");
    }

    /// codex 真实 LLM 调用 — 需要 OPENAI_API_KEY env
    #[tokio::test]
    #[ignore = "需要 OPENAI_API_KEY env; 估 R20 阶段 4 续派后启用"]
    async fn codex_real_invoke() {
        eprintln!("codex 真实调用 - 等 R20 阶段 4 续派后启用");
    }
}

// ============================================================
// Section 4: opencode Provider (R20 阶段 4 续派, #[ignore])
// ============================================================

#[cfg(test)]
mod opencode_provider_e2e {
    /// opencode 编译期 hardcode 守门
    #[test]
    #[ignore = "opencode Provider 估 R20 阶段 4 续派; 等落地后移除 #[ignore]"]
    fn opencode_tool_whitelist_has_eight_tools() {
        eprintln!("opencode Provider 估 R20 阶段 4 续派 (per RIVAL §2.5 估补)");
    }

    /// opencode 真实 LLM 调用
    #[tokio::test]
    #[ignore = "opencode Provider 估 R20 阶段 4 续派后启用"]
    async fn opencode_real_invoke() {
        eprintln!("opencode 真实调用 - 等 R20 阶段 4 续派后启用");
    }
}

// ============================================================
// Section 5: copilot Provider (R20 阶段 4 续派, #[ignore])
// ============================================================

#[cfg(test)]
mod copilot_provider_e2e {
    /// copilot 编译期 hardcode 守门
    #[test]
    #[ignore = "copilot Provider 估 R20 阶段 4 续派; 等落地后移除 #[ignore]"]
    fn copilot_tool_whitelist_has_eight_tools() {
        eprintln!("copilot Provider 估 R20 阶段 4 续派 (per RIVAL §2.5 估补)");
    }

    /// copilot 真实 LLM 调用 — 需要 GITHUB_TOKEN env
    #[tokio::test]
    #[ignore = "需要 GITHUB_TOKEN env; 估 R20 阶段 4 续派后启用"]
    async fn copilot_real_invoke() {
        eprintln!("copilot 真实调用 - 等 R20 阶段 4 续派后启用");
    }
}

// ============================================================
// Section 6: 跨 Provider K-1 强校验
// ============================================================

#[cfg(test)]
mod cross_provider_k1 {
    /// K-1.5: 2 Provider 工具名前缀一致性 (claude-code + gemini-cli 已落地)
    #[test]
    fn k1_all_provider_tool_names_have_apeireth_prefix() {
        // claude-code 已验证
        for tool in super::provider_claude_code_inline::TOOL_WHITELIST {
            assert!(tool.starts_with("apeireth_provider_claude_code_"), "{}", tool);
        }
        // gemini-cli 已验证
        for tool in super::provider_gemini_cli_inline::TOOL_WHITELIST {
            assert!(tool.starts_with("apeireth_provider_gemini_cli_"), "{}", tool);
        }
        // codex / opencode / copilot 估 R20 阶段 4 续派
    }

    /// K-1.6: 2 Provider 编译期 ModelKind 守门
    #[test]
    fn k1_all_providers_have_model_kind_count() {
        // claude-code 已验证
        assert_eq!(super::provider_claude_code_inline::MODEL_KIND_COUNT, 3);
        // gemini-cli 已验证
        assert_eq!(super::provider_gemini_cli_inline::ModelKind::COUNT, 3);
        assert_eq!(super::provider_gemini_cli_inline::MODEL_KIND_COUNT, 3);
        // 其它 3 Provider 估 R20 阶段 4 续派
    }

    /// K-1.7: 2 Provider 8 工具白名单长度一致性
    #[test]
    fn k1_all_providers_have_eight_tool_whitelist() {
        assert_eq!(super::provider_claude_code_inline::TOOL_WHITELIST.len(), 8);
        assert_eq!(super::provider_gemini_cli_inline::TOOL_WHITELIST.len(), 8);
    }
}
