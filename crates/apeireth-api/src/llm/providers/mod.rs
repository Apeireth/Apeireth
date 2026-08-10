//! Provider 实现集合
//!
//! **R17 实装**:
//! - `apeireth_api::ApeirethApiProvider` — minimaxi 专有 LLM provider (OpenAI Chat Completion 协议)
//! - `anthropic_compat::AnthropicCompatibleProvider` — Anthropic Messages API 协议 provider (走 minimaxi `/anthropic` 端点)
//! - `openai_compat::OpenAiCompatibleProvider` — 通用 OpenAI 兼容 (其他平台, e.g. OpenAI / Ollama / Together / vLLM / 等)
//! - `scripted::ScriptedLlmProvider` — 测试用 mock (脚本化响应, 单元测试和离线测试用)
//!
//! **R17 主语**:
//! apeireth-api 直连 **Anthropic Messages API** + **OpenAI Chat Completion API** 双协议.
//! 任何实现这两套协议的 provider 都能接 (minimaxi / OpenAI / Anthropic / Ollama / Together / vLLM / LMStudio / 等).
//! 不依赖 NewAPI 进程 (R17 砍掉).
//!
//! **加新 provider**:
//! 1. 在本 mod.rs 加 `pub mod xxx;`
//! 2. 创建 `xxx.rs` 实现 `LlmProvider` trait
//! 3. 在 `config.rs` 加 type variant (如需 TOML 驱动配置)
//! 4. 零修改 router / trait / 其他 provider 代码

pub mod anthropic_compat;
pub mod apeireth_api;
pub mod openai_compat;
pub mod scripted;
