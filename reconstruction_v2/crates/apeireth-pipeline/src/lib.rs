//! `apeireth-pipeline` — v2 5-step chat pipeline (token budget → placeholder → force-translate → protocol → http).
//!
//! v1 API surface preserved: `Pipeline`, `PipelineConfig`, `BORROWED_LEGACY_COUNT`,
//! `PIPELINE_STEP_COUNT`, `LEGACY_RETRY_SUPPRESSION_MS`, `LEGACY_MAX_INJECTION_CHARS`,
//! `token_budget::{truncate_to_max, exceeds_budget, MAX_INJECTION_CHARS, MIN_INJECTION_CHARS, DEFAULT_BRIEF_TOKEN_BUDGET, LIGHT_LIST_TOKEN_BUDGET}`,
//! `placeholder::{resolve_placeholders, PlaceholderContext, MAX_RECURSION_DEPTH, PLACEHOLDER_REGEX_STR}`,
//! `force_translate::{force_translate_if_needed, needs_force_translate, ForceTranslateConfig, ForceTranslateStats}`,
//! `retry_suppression::{RetrySuppression, DEFAULT_SUPPRESSION_WINDOW_MS}`,
//! `streaming::{stream_to_sender, StreamChunk}`,
//! `tool_loop::{run_tool_loop, should_continue, ToolLoopMessage, ToolLoopState, LlmStepResult, DEFAULT_MAX_TOOL_TURNS}`,
//! `provider_registry::{ProviderRegistry, ProviderSpec, ProviderCapability, SelectionStrategy, CostTracker, FallbackChain, UsageRecord, RegistryError, FallbackError}`,
//! `tiktoken_counter::{count_tokens, TiktokenCounter}`,
//! `model_router::{ModelRouter, RouteDecision, ModelRoute}`,
//! `role_divider::{divide_role, RoleDivider, DivisionDecision}`.

#![deny(unsafe_code)]

pub mod force_translate;
pub mod model_router;
pub mod placeholder;
pub mod provider_registry;
pub mod retry_suppression;
pub mod role_divider;
pub mod streaming;
pub mod tiktoken_counter;
pub mod token_budget;
pub mod tool_loop;

pub use force_translate::{
    force_translate_if_needed, is_text_only_model_by_tag, messages_contain_base64_media,
    needs_force_translate, ForceTranslateConfig, ForceTranslateStats,
};
pub use placeholder::{
    resolve_placeholders, PlaceholderContext, MAX_RECURSION_DEPTH, PLACEHOLDER_REGEX_STR,
};
pub use provider_registry::{
    CostTracker, FallbackChain, FallbackError, ProviderCapability, ProviderRegistry, ProviderSpec,
    RegistryError, SelectionStrategy, UsageRecord, ALL_PROVIDER_CAPABILITIES,
    ALL_SELECTION_STRATEGIES,
};
pub use retry_suppression::{RetrySuppression, DEFAULT_SUPPRESSION_WINDOW_MS};
pub use role_divider::{divide_role, DivisionDecision, RoleDivider};
pub use streaming::{stream_to_sender, StreamChunk};
pub use tiktoken_counter::{count_tokens, TiktokenCounter};
pub use token_budget::{
    exceeds_budget, truncate_to_max, DEFAULT_BRIEF_TOKEN_BUDGET, LIGHT_LIST_TOKEN_BUDGET,
    MAX_INJECTION_CHARS, MIN_INJECTION_CHARS,
};
pub use tool_loop::{
    run_tool_loop, should_continue, LlmStepResult, ToolLoopMessage, ToolLoopState,
    DEFAULT_MAX_TOOL_TURNS,
};

/// VCP §6.2.2 borrowed legacy count (#15/#17/#19/#20).
pub const BORROWED_LEGACY_COUNT: usize = 4;

/// Pipeline 5 steps.
pub const PIPELINE_STEP_COUNT: usize = 5;

/// VCP retry suppression 15000ms.
pub const LEGACY_RETRY_SUPPRESSION_MS: u64 = 15_000;

/// VCP max injection chars 16000.
pub const LEGACY_MAX_INJECTION_CHARS: usize = 16_000;

/// Pipeline configuration.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub base_url: String,
    pub auth_token: Option<String>,
    pub force_translate: ForceTranslateConfig,
    pub max_injection_chars: usize,
    pub placeholder_context: PlaceholderContext,
    pub suppression: RetrySuppression,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.minimaxi.com".to_string(),
            auth_token: None,
            force_translate: ForceTranslateConfig::chat_default(),
            max_injection_chars: LEGACY_MAX_INJECTION_CHARS,
            placeholder_context: PlaceholderContext::new(),
            suppression: RetrySuppression::with_chat_default(),
        }
    }
}

/// Pipeline result.
pub type PipelineResult<T> = Result<T, PipelineError>;

/// Pipeline errors.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("placeholder resolution failed: {0}")]
    Placeholder(String),
    #[error("token budget exceeded: {0} chars")]
    TokenBudgetExceeded(usize),
    #[error("force translate failed: {0}")]
    ForceTranslate(String),
    #[error("retry suppressed for {0}ms")]
    RetrySuppressed(u64),
    #[error("http error: {0}")]
    Http(String),
}

/// Pipeline — 5-step orchestration over an HTTP transport.
///
/// This v2 implementation mirrors v1's structure but uses a generic
/// transport callback instead of binding to a specific http-client crate.
pub struct Pipeline {
    pub config: PipelineConfig,
    /// Transport callback: (url, body) -> response body bytes.
    pub transport: std::sync::Arc<dyn Fn(String, Vec<u8>) -> Result<Vec<u8>, PipelineError> + Send + Sync>,
}

impl Pipeline {
    pub fn new(config: PipelineConfig, transport: std::sync::Arc<dyn Fn(String, Vec<u8>) -> Result<Vec<u8>, PipelineError> + Send + Sync>) -> Self {
        Self { config, transport }
    }

    pub fn with_chat_defaults() -> Self {
        Self::new(PipelineConfig::default(), std::sync::Arc::new(|_, _| Ok(vec![])))
    }

    pub fn config(&self) -> &PipelineConfig { &self.config }

    /// Execute the 5-step pipeline.
    pub async fn execute(&self, input: &str) -> PipelineResult<Vec<u8>> {
        // Step 1: placeholder resolution
        let mut ctx = self.config.placeholder_context.clone();
        let resolved = resolve_placeholders(input, &mut ctx)?;

        // Step 2: token budget (check then truncate)
        if exceeds_budget(&resolved, self.config.max_injection_chars) {
            return Err(PipelineError::TokenBudgetExceeded(self.config.max_injection_chars));
        }
        let truncated = if resolved.len() > self.config.max_injection_chars {
            truncate_to_max(&resolved, self.config.max_injection_chars)
        } else {
            resolved
        };

        // Step 3: force translate
        let translated = force_translate_if_needed(&truncated, &self.config.force_translate);

        // Step 4: protocol normalization (simulated: just bytes)
        let body = translated.into_bytes();

        // Step 5: http
        if !self.config.suppression.allow() {
            return Err(PipelineError::RetrySuppressed(DEFAULT_SUPPRESSION_WINDOW_MS));
        }
        let url = format!("{}{}", self.config.base_url, "/v1/chat/completions");
        (self.transport)(url, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_legacy() {
        assert_eq!(BORROWED_LEGACY_COUNT, 4);
        assert_eq!(PIPELINE_STEP_COUNT, 5);
        assert_eq!(LEGACY_RETRY_SUPPRESSION_MS, 15_000);
        assert_eq!(LEGACY_MAX_INJECTION_CHARS, 16_000);
    }

    #[tokio::test]
    async fn pipeline_default_runs() {
        let p = Pipeline::with_chat_defaults();
        let result = p.execute("hello {{user}}").await.unwrap();
        assert!(result.is_empty()); // transport returns empty bytes
    }

    #[tokio::test]
    async fn pipeline_token_budget_exceeded() {
        let mut cfg = PipelineConfig::default();
        cfg.max_injection_chars = 5;
        let p = Pipeline::new(cfg, std::sync::Arc::new(|_, _| Ok(vec![])));
        let r = p.execute("hello world this is long").await;
        assert!(matches!(r, Err(PipelineError::TokenBudgetExceeded(_))));
    }
}
