//! Integration tests for apeireth-provider (post-1.0.0)
//!
//! src/ 各 mod 已覆盖 (R35 umbrella + per-module). 这里 (tests/) 加 6-provider 统一测试.
//! 0 触碰 src/, 0 编造"已实现".

use apeireth_provider::claude_code::{ClaudeCodeProvider, K1_CHECKS, TOOL_WHITELIST as CC_WL};
use apeireth_provider::codex::{CodexProvider, SANDBOX_TYPES};
use apeireth_provider::copilot::CopilotProvider;
use apeireth_provider::gemini_cli::GeminiCliProvider;
use apeireth_provider::minimax::{MinimaxProvider, MINIMAX_BASE_URL, MINIMAX_PROTOCOLS};
use apeireth_provider::opencode::OpencodeProvider;

// =============================================================================
// ClaudeCodeProvider
// =============================================================================

#[test]
fn claude_code_new() {
    let p = ClaudeCodeProvider::new();
    assert_eq!(p.name, "claude-code");
    assert_eq!(p.tools.len(), 8);
    assert_eq!(p.model_kinds.len(), 3);
}

#[test]
fn claude_code_default() {
    let p = ClaudeCodeProvider::default();
    assert_eq!(p.name, "claude-code");
}

#[test]
fn claude_code_tool_whitelist_8() {
    assert_eq!(CC_WL.len(), 8);
    for t in &[
        "Read",
        "Write",
        "Edit",
        "Bash",
        "Glob",
        "Grep",
        "WebSearch",
        "WebFetch",
    ] {
        assert!(CC_WL.contains(t), "白名单应含 {t}");
    }
}

#[test]
fn claude_code_k1_checks_5() {
    assert_eq!(K1_CHECKS.len(), 5);
    for c in &[
        "base_url_not_empty",
        "auth_token_format",
        "tool_in_whitelist",
        "args_is_object",
        "timeout_positive",
    ] {
        assert!(K1_CHECKS.contains(c), "K-1 应含 {c}");
    }
}

// =============================================================================
// CodexProvider
// =============================================================================

#[test]
fn codex_new() {
    let p = CodexProvider::new();
    assert_eq!(p.name, "codex");
    assert_eq!(p.tools.len(), 8);
    assert_eq!(p.model_kinds.len(), 4, "Codex 4 vs Claude 3 ModelKind");
}

#[test]
fn codex_default() {
    let p = CodexProvider::default();
    assert_eq!(p.name, "codex");
}

#[test]
fn codex_sandbox_types_3() {
    assert_eq!(SANDBOX_TYPES.len(), 3, "3 种 SandboxType");
    assert!(SANDBOX_TYPES.contains(&"workspace-write"));
    assert!(SANDBOX_TYPES.contains(&"read-only"));
    assert!(SANDBOX_TYPES.contains(&"danger-full-access"));
}

// =============================================================================
// CopilotProvider
// =============================================================================

#[test]
fn copilot_new() {
    let p = CopilotProvider::new();
    assert_eq!(p.name, "copilot");
    assert_eq!(p.tools.len(), 8);
    assert_eq!(p.model_kinds.len(), 3);
    assert!(p.oauth_required, "Copilot 强制 OAuth");
}

#[test]
fn copilot_default() {
    let p = CopilotProvider::default();
    assert_eq!(p.name, "copilot");
}

// =============================================================================
// GeminiCliProvider
// =============================================================================

#[test]
fn gemini_cli_new() {
    let p = GeminiCliProvider::new();
    assert_eq!(p.name, "gemini-cli");
    assert_eq!(p.tools.len(), 8);
    assert_eq!(p.model_kinds.len(), 3);
    assert_eq!(p.embedding_dim, 768);
}

#[test]
fn gemini_cli_default() {
    let p = GeminiCliProvider::default();
    assert_eq!(p.name, "gemini-cli");
}

// =============================================================================
// OpencodeProvider
// =============================================================================

#[test]
fn opencode_new() {
    let p = OpencodeProvider::new();
    assert_eq!(p.name, "opencode");
    assert_eq!(p.tools.len(), 8);
    assert_eq!(p.model_kinds.len(), 3);
}

#[test]
fn opencode_default() {
    let p = OpencodeProvider::default();
    assert_eq!(p.name, "opencode");
}

// =============================================================================
// MinimaxProvider (R128 第 6 个 provider)
// =============================================================================

#[test]
fn minimax_new() {
    let p = MinimaxProvider::new();
    assert_eq!(p.name, "minimax");
    assert_eq!(p.tools.len(), 8);
    assert!(
        p.model_kinds.len() >= 7,
        "minimax 至少 7 model kinds (R82 LIVE 7/7)"
    );
}

#[test]
fn minimax_default() {
    let p = MinimaxProvider::default();
    assert_eq!(p.name, "minimax");
}

#[test]
fn minimax_protocols_4() {
    assert_eq!(MINIMAX_PROTOCOLS.len(), 4, "4 协议");
    assert!(MINIMAX_PROTOCOLS.contains(&"anthropic"));
    assert!(MINIMAX_PROTOCOLS.contains(&"openai_chat"));
    assert!(MINIMAX_PROTOCOLS.contains(&"openai_responses"));
    assert!(MINIMAX_PROTOCOLS.contains(&"gemini"));
}

#[test]
fn minimax_base_url() {
    assert!(MINIMAX_BASE_URL.starts_with("https://"));
    assert!(
        MINIMAX_BASE_URL.contains("minimax"),
        "base_url 应含 minimax: {MINIMAX_BASE_URL}"
    );
}

// =============================================================================
// 跨 provider 一致性
// =============================================================================

#[test]
fn all_6_providers_have_8_tools() {
    let p_claude = ClaudeCodeProvider::new();
    let p_codex = CodexProvider::new();
    let p_copilot = CopilotProvider::new();
    let p_gemini = GeminiCliProvider::new();
    let p_opencode = OpencodeProvider::new();
    let p_minimax = MinimaxProvider::new();
    assert_eq!(p_claude.tools.len(), 8, "claude-code 8 工具");
    assert_eq!(p_codex.tools.len(), 8, "codex 8 工具");
    assert_eq!(p_copilot.tools.len(), 8, "copilot 8 工具");
    assert_eq!(p_gemini.tools.len(), 8, "gemini-cli 8 工具");
    assert_eq!(p_opencode.tools.len(), 8, "opencode 8 工具");
    assert_eq!(p_minimax.tools.len(), 8, "minimax 8 工具");
}

#[test]
fn all_6_providers_have_unique_names() {
    let names = [
        ClaudeCodeProvider::new().name,
        CodexProvider::new().name,
        CopilotProvider::new().name,
        GeminiCliProvider::new().name,
        OpencodeProvider::new().name,
        MinimaxProvider::new().name,
    ];
    let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
    assert_eq!(unique.len(), 6, "6 个 provider 名互不相同");
}

#[test]
fn all_6_providers_have_kebab_case_names() {
    let names = [
        ClaudeCodeProvider::new().name,
        CodexProvider::new().name,
        CopilotProvider::new().name,
        GeminiCliProvider::new().name,
        OpencodeProvider::new().name,
        MinimaxProvider::new().name,
    ];
    for n in names {
        assert!(
            !n.contains('_') && !n.contains(' '),
            "provider 名应用 - 分隔: {n}"
        );
        assert!(
            n.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "应小写 + 可含 -: {n}"
        );
    }
}

#[test]
fn all_providers_in_all_providers_const() {
    use apeireth_provider::ALL_PROVIDERS;
    let names = [
        ClaudeCodeProvider::new().name,
        CodexProvider::new().name,
        CopilotProvider::new().name,
        GeminiCliProvider::new().name,
        OpencodeProvider::new().name,
        MinimaxProvider::new().name,
    ];
    for n in names {
        assert!(ALL_PROVIDERS.contains(&n), "ALL_PROVIDERS 应含 {n}");
    }
    assert_eq!(ALL_PROVIDERS.len(), 6);
}
