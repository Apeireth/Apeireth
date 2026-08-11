//! `apeireth-provider` — **R35: 5 Provider client unified**
//!
//! **背景**: R20 阶段 4 留 5 个 0.1.0 skeleton crate (claude-code/codex/copilot/gemini-cli/opencode),
//! 5 个 crate 内容几乎一样 (8 工具 + 3-4 ModelKind + 8 TOOL_WHITELIST + 5 K-1 强校验), 重复维护.
//!
//! **R35 合并**: 1 个 `apeireth-provider` 主 crate, 5 module 各自包含 1 个 provider 实现.
//! 5 个老 crate 留作 shell (re-export 新 crate module), 0 回归.
//!
//! **R36 阶段 2**: 删 5 个老 crate, 改所有 import 到 apeireth-provider::{claude_code, codex, copilot, gemini_cli, opencode}.
//!
//! **5 module** (R20 阶段 4 owner 1 owner × 1 仓库 模式, 字段级 1:1 翻译 v0.9.21 上游 client):
//! - `claude_code` — @anthropic-ai/claude-agent-sdk 0.2.112 (8 工具 + 3 ModelKind)
//! - `codex` — @openai/codex 0.9.21 (8 工具 + 4 ModelKind + 3 SandboxType)
//! - `copilot` — @github/copilot-sdk 0.9.21 (8 工具 + 3 ModelKind + OAuth)
//! - `gemini_cli` — @google/gemini-cli 0.9.21 (8 工具 + 3 ModelKind + Embedding)
//! - `opencode` — @opencode-ai/opencode 0.9.21 (8 工具 + 3 ModelKind)

#![warn(missing_docs)]

// 5 module, 各自由 src/<name>.rs 实现 (R35 阶段 1: 只 5 Provider struct + 8 工具 + 3 ModelKind, R21+ 真接 SDK)
pub mod claude_code;
pub mod codex;
pub mod copilot;
pub mod gemini_cli;
pub mod opencode;
pub mod minimax;  // R128: 6th provider (MiniMax-M3 family)

/// R35: 5 provider name 1:1 对应, 启动时配置用
pub const ALL_PROVIDERS: [&str; 6] = [
    "claude-code",
    "codex",
    "copilot",
    "gemini-cli",
    "opencode",
    "minimax",
];

#[cfg(test)]
mod r35_provider_umbrella_tests {
    use super::*;

    #[test]
    fn r35_6_providers_all_present() {
        assert_eq!(ALL_PROVIDERS.len(), 6);
        // 5 module 都在
        let _claude = std::any::type_name::<claude_code::ClaudeCodeProvider>();
        let _codex = std::any::type_name::<codex::CodexProvider>();
        let _copilot = std::any::type_name::<copilot::CopilotProvider>();
        let _gemini = std::any::type_name::<gemini_cli::GeminiCliProvider>();
        let _opencode = std::any::type_name::<opencode::OpencodeProvider>();
        let _minimax = std::any::type_name::<minimax::MinimaxProvider>();
    }
}
