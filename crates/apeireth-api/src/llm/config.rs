//! TOML 配置加载 (apeireth-api.toml)
//!
//! 示例:
//! ```toml
//! [providers.apeireth-api]
//! type = "apeireth-api"
//! base_url = "http://localhost:3000/v1"
//! api_key_env = "APEIRETH_API_KEY"
//! models = ["MiniMax-M3", "MiniMax-M3-thinking"]
//!
//! [providers.openai]
//! type = "openai-compatible"
//! base_url = "https://api.openai.com/v1"
//! api_key_env = "OPENAI_API_KEY"
//! models = ["gpt-4o", "gpt-4o-mini"]
//!
//! [router.fallback_order]
//! order = ["apeireth-api", "openai"]
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::llm::error::LlmError;
use crate::llm::providers::apeireth_api::{ApeirethApiConfig, ApeirethApiProvider};
use crate::llm::providers::openai_compat::{OpenAiCompatibleConfig, OpenAiCompatibleProvider};
use crate::llm::providers::scripted::ScriptedLlmProvider;
use crate::llm::router::MultiLlmRouter;
use crate::llm::semantic_router::{SemanticRouter, SemanticRouterConfig};
use crate::llm::traits::LlmProvider;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub router: RouterConfig,
    /// R30 U7: 语义路由 (按 user message 关键词相似度选 route)
    #[serde(default)]
    pub semantic_routes: Option<SemanticRouterConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: String,
    pub base_url: Option<String>,
    pub api_key_env: String,
    #[serde(default)]
    pub models: Vec<String>,
    /// scripted 类型专用: 关键词 → 响应映射
    #[serde(default)]
    pub scripts: HashMap<String, String>,
    #[serde(default)]
    pub default_response: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RouterConfig {
    #[serde(default)]
    pub fallback_order: Vec<String>,
}

impl LlmConfig {
    /// 从 TOML 文件加载
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// 从 TOML 字符串加载
    #[allow(dead_code)]
    pub fn from_str(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// 构建 router (从 config)
    pub fn build_router(&self) -> Result<MultiLlmRouter, LlmError> {
        let mut router = MultiLlmRouter::new();

        for (name, cfg) in &self.providers {
            let provider = self.build_provider(name, cfg)?;
            router = router.with_provider(provider);
        }

        if !self.router.fallback_order.is_empty() {
            router = router.with_fallback(self.router.fallback_order.clone());
        }

        Ok(router)
    }

    /// R30 U7: 构建 SemanticRouter (如果 config 配了 semantic_routes)
    ///
    /// 内部走 `build_router()` 构造 MultiLlmRouter, 再用 SemanticRouter 包装.
    /// 如果没配 semantic_routes: 返 None.
    pub fn build_semantic_router(&self) -> Result<Option<SemanticRouter>, LlmError> {
        let Some(cfg) = self.semantic_routes.clone() else {
            return Ok(None);
        };
        let router = self.build_router()?;
        Ok(Some(SemanticRouter::new(cfg, Arc::new(router))))
    }

    fn build_provider(
        &self,
        name: &str,
        cfg: &ProviderConfig,
    ) -> Result<Arc<dyn LlmProvider>, LlmError> {
        let api_key = std::env::var(&cfg.api_key_env).map_err(|_| {
            LlmError::Config(format!(
                "env var {} not set (provider {})",
                cfg.api_key_env, name
            ))
        })?;

        match cfg.provider_type.as_str() {
            "apeireth-api" => {
                let base_url = cfg.base_url.clone().ok_or_else(|| {
                    LlmError::Config(format!("apeireth-api provider {} missing base_url", name))
                })?;
                let config = ApeirethApiConfig::new(api_key, base_url, cfg.models.clone());
                Ok(Arc::new(ApeirethApiProvider::new(config)?))
            }
            "openai-compatible" => {
                let base_url = cfg.base_url.clone().ok_or_else(|| {
                    LlmError::Config(format!(
                        "openai-compatible provider {} missing base_url",
                        name
                    ))
                })?;
                let config =
                    OpenAiCompatibleConfig::new(name, base_url, api_key, cfg.models.clone());
                Ok(Arc::new(OpenAiCompatibleProvider::new(config)?))
            }
            "scripted" => {
                let mut provider = ScriptedLlmProvider::new(name);
                for (keyword, response) in &cfg.scripts {
                    provider = provider.with_script(
                        keyword.clone(),
                        super::providers::scripted::ScriptedResponse::new(response.clone()),
                    );
                }
                if let Some(default) = &cfg.default_response {
                    provider = provider.with_default(
                        super::providers::scripted::ScriptedResponse::new(default.clone()),
                    );
                }
                Ok(Arc::new(provider))
            }
            unknown => Err(LlmError::Config(format!(
                "unknown provider type '{}' (provider {})",
                unknown, name
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::llm::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
            [providers.apeireth-api]
            type = "apeireth-api"
            base_url = "http://localhost:3000/v1"
            api_key_env = "APEIRETH_API_KEY"
            models = ["MiniMax-M3"]

            [router]
            fallback_order = ["apeireth-api"]
        "#;
        let config = LlmConfig::from_str(toml).unwrap();
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.router.fallback_order, vec!["apeireth-api"]);
    }

    #[test]
    fn test_parse_multi_provider_config() {
        let toml = r#"
            [providers.apeireth-api]
            type = "apeireth-api"
            base_url = "http://localhost:3000/v1"
            api_key_env = "APEIRETH_API_KEY"
            models = ["MiniMax-M3"]

            [providers.openai]
            type = "openai-compatible"
            base_url = "https://api.openai.com/v1"
            api_key_env = "OPENAI_API_KEY"
            models = ["gpt-4o"]

            [providers.test]
            type = "scripted"
            api_key_env = "APEIRETH_LLM_NO_KEY"
            scripts = { "hello" = "hi back" }
            default_response = "default"

            [router]
            fallback_order = ["apeireth-api", "openai"]
        "#;
        let config = LlmConfig::from_str(toml).unwrap();
        assert_eq!(config.providers.len(), 3);
    }

    #[test]
    fn test_build_scripted_provider() {
        std::env::set_var("APEIRETH_LLM_NO_KEY", "placeholder");
        let toml = r#"
            [providers.test]
            type = "scripted"
            api_key_env = "APEIRETH_LLM_NO_KEY"
            scripts = { "hello" = "hi back" }
            default_response = "no match"
        "#;
        let config = LlmConfig::from_str(toml).unwrap();
        let router = config.build_router().unwrap();
        assert!(router.supports_model("anything"));
    }

    #[test]
    fn test_unknown_provider_type_errors() {
        std::env::set_var("APEIRETH_LLM_NO_KEY", "placeholder");
        let toml = r#"
            [providers.test]
            type = "unknown-future-provider"
            api_key_env = "APEIRETH_LLM_NO_KEY"
        "#;
        let config = LlmConfig::from_str(toml).unwrap();
        let result = config.build_router();
        assert!(matches!(result, Err(LlmError::Config(_))));
    }
}
