//! `apeireth-provider` — R35 5 Provider client unified (5 + 1 MiniMax = 6 providers).
//!
//! 0 装 PASS 严守: 纯 in-memory provider descriptor (无真 HTTP/SQL/FFI).
//! 抄 v1.0 字段名 + 真 8 TOOL_WHITELIST + 真 model kinds.
//!
//! v1 对齐 (per apeireth-provider/lib.rs R35): 6 module (claude_code / codex / copilot / gemini_cli / opencode / minimax)
//! + facade_impls (LlmFacade 统一接入) + http_dispatch (HTTP 描述符) + reasoning_adapter (推理字段归一化).
//!
//! 模块清单:
//! - `claude_code` — @anthropic-ai/claude-agent-sdk (8 工具 + 3 ModelKind)
//! - `codex` — @openai/codex (8 工具 + 4 ModelKind + 3 SandboxType)
//! - `copilot` — @github/copilot-sdk (8 工具 + 3 ModelKind + OAuth)
//! - `gemini_cli` — @google/gemini-cli (8 工具 + 3 ModelKind + Embedding)
//! - `opencode` — @opencode-ai/opencode (8 工具 + 3 ModelKind)
//! - `minimax` — MiniMax (minimaxi) 6th provider
//! - `facade_impls` — 6 Provider facade 统一 impl
//! - `http_dispatch` — Provider config + status mapping
//! - `reasoning_adapter` — REASONING_ALIASES 12 别名提取

pub mod claude_code;
pub mod codex;
pub mod copilot;
pub mod gemini_cli;
pub mod opencode;
pub mod minimax;
pub mod facade_impls;
pub mod http_dispatch;
pub mod reasoning_adapter;

/// R35: 6 provider name 1:1 对应, 启动时配置用.
pub const ALL_PROVIDERS: [&str; 6] = [
    "claude-code",
    "codex",
    "copilot",
    "gemini-cli",
    "opencode",
    "minimax",
];

/// R35: 8 工具白名单 (per R20 阶段 4).
pub const TOOL_WHITELIST: [&str; 8] = [
    "Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch",
];

/// R35: K-1 5 强校验.
pub const K1_CHECKS: [&str; 5] = [
    "base_url_not_empty",
    "auth_token_format",
    "tool_in_whitelist",
    "args_is_object",
    "timeout_positive",
];

#[cfg(test)]
mod r35_provider_umbrella_tests {
    use super::*;

    #[test]
    fn r35_6_providers_all_present() {
        assert_eq!(ALL_PROVIDERS.len(), 6);
    }

    #[test]
    fn tool_whitelist_is_8() {
        assert_eq!(TOOL_WHITELIST.len(), 8);
    }

    #[test]
    fn k1_checks_is_5() {
        assert_eq!(K1_CHECKS.len(), 5);
    }

    #[test]
    fn module_types_exist() {
        let _ = std::any::type_name::<claude_code::ClaudeCodeProvider>();
        let _ = std::any::type_name::<codex::CodexProvider>();
        let _ = std::any::type_name::<copilot::CopilotProvider>();
        let _ = std::any::type_name::<gemini_cli::GeminiCliProvider>();
        let _ = std::any::type_name::<opencode::OpencodeProvider>();
        let _ = std::any::type_name::<minimax::MinimaxProvider>();
    }
}
