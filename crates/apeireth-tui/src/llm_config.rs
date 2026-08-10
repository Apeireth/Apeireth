//! R26-3 LLM 直连: TUI 不再走 apeireth-api server, 直接 reqwest POST 到用户大模型
//!
//! **配置存储**: `~/.config/apeireth/llm.json` (跟 settings.json 同目录, 不同文件)
//! - 真实用户角度: 不需要启动 server, TUI 启动前读 llm.json, 缺则进 onboarding wizard
//! - developer 角度: env `APEIRETH_API_URL` / `APEIRETH_API_KEY` / `APEIRETH_API_MODEL` 仍兜底 (向后兼容)
//!
//! **schema** (llm.json):
//! ```json
//! {
//!   "provider": "openai",     // "openai" | "anthropic" | "deepseek" | "ollama" | "custom"
//!   "base_url": "https://api.openai.com/v1",
//!   "api_key": "sk-...",
//!   "model": "gpt-4o"
//! }
//! ```
//!
//! **8 项承诺 (R23)**: workspace.version / 24 LOCKED / R11 LOCKED / 顶层 3 规范 0 触
//! - 不动 apeireth-api (它是完整 LLM provider 库, 我们这里只走 HTTP 瘦客户端, 不复用它的 provider 类型)
//! - 不动 persistence Settings JSON schema (5 字段 + settings.json 路径不变, 单独 llm.json 是新文件)

use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 同 persistence.rs: 跨平台 config 目录
/// - Windows: `%APPDATA%peireth`
/// - Unix: `${XDG_CONFIG_HOME:-~/.config}/apeireth`
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return Some(PathBuf::from(appdata).join(DIR_NAME));
            }
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return Some(PathBuf::from(xdg).join(DIR_NAME));
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            if !home.is_empty() {
                return Some(PathBuf::from(home).join(".config").join(DIR_NAME));
            }
        }
    }
    None
}

pub fn llm_config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join(LLM_FILE_NAME))
}

const DIR_NAME: &str = "apeireth";
const LLM_FILE_NAME: &str = "llm.json";

/// Provider 标识. 5 个预设 + custom (用户填 URL).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// OpenAI 主站 (gpt-4o 等)
    Openai,
    /// Anthropic Claude (api.anthropic.com, Messages API)
    Anthropic,
    /// DeepSeek (api.deepseek.com, OpenAI 兼容)
    Deepseek,
    /// Ollama 本地 (localhost:11434/v1)
    Ollama,
    /// 任意 OpenAI 兼容端点 (代理 / LMStudio / vLLM ...)
    Custom,
}

impl Provider {
    /// 预设底中 / base_url + 默认 model
    pub fn preset(&self) -> (&'static str, &'static str) {
        match self {
            Self::Openai => ("https://api.openai.com/v1", "gpt-4o-mini"),
            Self::Anthropic => ("https://api.anthropic.com", "claude-3-5-sonnet-latest"),
            Self::Deepseek => ("https://api.deepseek.com/v1", "deepseek-chat"),
            Self::Ollama => ("http://localhost:11434/v1", "llama3"),
            Self::Custom => ("", ""),
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Self::Openai => "OpenAI",
            Self::Anthropic => "Anthropic",
            Self::Deepseek => "DeepSeek",
            Self::Ollama => "Ollama",
            Self::Custom => "自定义",
        }
    }
    /// R26-3 fix: 不同 provider 的 endpoint path 取决于 base_url 是否含 /v1 前缀
    /// - OpenAI / DeepSeek / Ollama / Custom: base_url 已含 /v1 (OpenAI 兼容约定), 拼 /chat/completions
    /// - Anthropic: base_url 不含 /v1, 拼 /v1/messages (Messages API)
    ///
    /// **R26-3 404 根因**: 此前硬编码 `/v1/chat/completions`, 当 user base_url 是
    /// `https://api.minimax.chat/v1` 时, 最终 URL = `https://api.minimax.chat/v1/v1/chat/completions`
    /// → 404. MiniMax 主人 (主人反馈) 这次发现的就是这个 bug.
    pub fn endpoint_path(&self) -> &'static str {
        match self {
            Self::Openai | Self::Deepseek | Self::Ollama | Self::Custom => "/chat/completions",
            Self::Anthropic => "/v1/messages",
        }
    }
}

/// R26-3 LLM 直连配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmConfig {
    pub provider: Provider,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    /// R26-3-fixes: 最大输出 token (默认 8192, Settings 可配 1-32768)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_max_tokens() -> u32 {
    8192
}

impl LlmConfig {
    pub fn new(provider: Provider, base_url: &str, api_key: &str, model: &str) -> Self {
        Self::with_max_tokens(provider, base_url, api_key, model, default_max_tokens())
    }

    pub fn with_max_tokens(provider: Provider, base_url: &str, api_key: &str, model: &str, max_tokens: u32) -> Self {
        Self {
            provider,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            max_tokens,
        }
    }
    /// R27 C 方案: api_key 可空 (本地 server 模式, server 自己配 APEIRETH_API_KEY).
    /// base_url + model 是必填的; api_key 为空 → http_llm 不发 Authorization header.
    pub fn is_valid(&self) -> bool {
        !self.base_url.is_empty() && !self.model.is_empty()
    }

    /// R26-3-fixes: 读 max_tokens (env override > config > default 8192)
    pub fn resolved_max_tokens(&self) -> u32 {
        if let Ok(s) = std::env::var("APEIRETH_MAX_TOKENS") {
            if let Ok(v) = s.parse::<u32>() {
                return v;
            }
        }
        if self.max_tokens == 0 {
            return default_max_tokens();
        }
        self.max_tokens
    }
}

/// 从 llm.json 读 (不包括 env 兑底, 不包括到 default). 返 None 说明需进 onboarding.
/// 优先级:
///   1) llm.json 存在 + 解析成功 + 4 字段都非空
///   2) 价业仅: env APEIRETH_API_URL + APEIRETH_API_KEY + APEIRETH_API_MODEL 三者都设了
pub fn load() -> Option<LlmConfig> {
    if let Some(p) = llm_config_path() {
        if let Ok(s) = fs::read_to_string(&p) {
            if let Ok(c) = serde_json::from_str::<LlmConfig>(&s) {
                if c.is_valid() {
                    return Some(c);
                }
            }
        }
    }
    // developer fallback: env 3 个都设了 → 跳过 onboarding (developer 习惯 env)
    if let (Ok(url), Ok(key), Ok(model)) = (
        std::env::var("APEIRETH_API_URL"),
        std::env::var("APEIRETH_API_KEY"),
        std::env::var("APEIRETH_API_MODEL"),
    ) {
        if !url.is_empty() && !key.is_empty() && !model.is_empty() {
            let max_tokens = std::env::var("APEIRETH_MAX_TOKENS")
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or_else(default_max_tokens);
            return Some(LlmConfig::with_max_tokens(
                Provider::Custom,
                &url,
                &key,
                &model,
                max_tokens,
            ));
        }
    }
    None
}

/// 写 llm.json (保证父目录). 错误不 panic, 返 Err 给 caller.
pub fn save(c: &LlmConfig) -> io::Result<()> {
    let dir = config_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "config dir unresolved")
    })?;
    fs::create_dir_all(&dir)?;
    let path = dir.join(LLM_FILE_NAME);
    let s = serde_json::to_string_pretty(c)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, s)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_preset_openai() {
        let (url, model) = Provider::Openai.preset();
        assert!(url.starts_with("https://api.openai.com"));
        assert!(!model.is_empty());
    }
    #[test]
    fn new_trims_trailing_slash() {
        let c = LlmConfig::new(Provider::Openai, "https://api.openai.com/v1/", "sk", "gpt-4o");
        assert_eq!(c.base_url, "https://api.openai.com/v1");
    }
    #[test]
    fn is_valid_requires_base_url_and_model() {
        // R27 C: api_key 可空 (server 模式不需 client key)
        assert!(LlmConfig::new(Provider::Openai, "u", "k", "m").is_valid());
        assert!(LlmConfig::new(Provider::Openai, "u", "", "m").is_valid()); // R27 server mode
        assert!(!LlmConfig::new(Provider::Openai, "", "k", "m").is_valid());
        assert!(!LlmConfig::new(Provider::Openai, "u", "k", "").is_valid());
    }
    #[test]
    fn config_dir_present() {
        // 在所有平台上都应该能获得 config 目录 (Windows APPDATA 或 Linux XDG/HOME)
        assert!(config_dir().is_some());
    }

    #[test]
    fn default_max_tokens_is_8192() {
        assert_eq!(default_max_tokens(), 8192);
    }

    #[test]
    fn new_with_default_max_tokens() {
        let c = LlmConfig::new(Provider::Openai, "u", "k", "m");
        assert_eq!(c.max_tokens, 8192);
    }

    #[test]
    fn new_with_explicit_max_tokens() {
        let c = LlmConfig::with_max_tokens(Provider::Openai, "u", "k", "m", 4096);
        assert_eq!(c.max_tokens, 4096);
    }

    #[test]
    fn resolved_max_tokens_returns_config_value() {
        let c = LlmConfig::with_max_tokens(Provider::Openai, "u", "k", "m", 16384);
        assert_eq!(c.resolved_max_tokens(), 16384);
    }

    #[test]
    fn resolved_max_tokens_falls_back_to_default_when_zero() {
        let mut c = LlmConfig::with_max_tokens(Provider::Openai, "u", "k", "m", 0);
        c.max_tokens = 0;
        assert_eq!(c.resolved_max_tokens(), 8192);
    }
}
