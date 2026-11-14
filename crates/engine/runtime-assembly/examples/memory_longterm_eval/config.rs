//! Run configuration. The offline path is the default; a live path exists only
//! behind an explicit endpoint plus a credential environment variable, and it
//! is never exercised in this round (no credential is available).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub endpoint: Option<String>,
    pub model: String,
    pub key_env: Option<String>,
    pub temperature: f64,
    /// Whether the vendor's reasoning mode is switched explicitly.
    ///
    /// `None` ⇒ **the vendor switch is not sent at all** (the vendor default
    /// applies, and endpoints that do not know the field are left alone).
    /// `Some(true)` / `Some(false)` ⇒ send the vendor's thinking switch.
    ///
    /// ⭐ Why this is opt-in: the field is a **vendor-private** request knob
    /// (DeepSeek's `{"thinking": {"type": ...}}`). Sending it to an endpoint
    /// that does not know it can be refused by the vendor, so it is never sent
    /// unless the operator asked for it.
    pub thinking: Option<bool>,
    pub reasoning_effort: Option<String>,
    pub max_output_tokens: u32,
    pub max_request_bytes: usize,
    pub max_calls: u32,
    pub timeout_ms: u64,
    pub context_chars: usize,
    /// `scripted_ack` keeps setup turns on the deterministic acknowledgement
    /// path; `live` would route setup turns through the configured model.
    pub history_main: String,
    pub input_price_per_million: Option<f64>,
    pub output_price_per_million: Option<f64>,
    pub cache_hit_price_per_million: Option<f64>,
    pub currency: Option<String>,
    pub price_date: Option<String>,
    /// Where the price table came from, so that a cost figure can be checked
    /// against its source.
    pub price_source: Option<String>,
}

impl Config {
    /// Deterministic offline configuration: no endpoint, no credential, no
    /// model call. Every answer in this mode comes from the injected reader
    /// fixture.
    pub fn offline() -> Self {
        Self {
            endpoint: None,
            model: "memory-longterm-offline-contract".to_string(),
            key_env: None,
            temperature: 0.0,
            thinking: None,
            reasoning_effort: None,
            max_output_tokens: 1024,
            max_request_bytes: 65536,
            max_calls: 64,
            timeout_ms: 30_000,
            context_chars: 4000,
            history_main: "scripted_ack".to_string(),
            input_price_per_million: None,
            output_price_per_million: None,
            cache_hit_price_per_million: None,
            currency: None,
            price_date: None,
            price_source: None,
        }
    }

    /// The live configuration: an endpoint, a model id and the **name** of the
    /// environment variable that carries the credential.
    ///
    /// ⭐ The credential itself never enters this struct: only the name of the
    /// environment variable does, so the configuration can be written into an
    /// artifact without leaking anything.
    pub fn live(endpoint: &str, model: &str, key_env: &str) -> Self {
        Self {
            endpoint: Some(endpoint.to_string()),
            model: model.to_string(),
            key_env: Some(key_env.to_string()),
            temperature: 0.0,
            thinking: None,
            reasoning_effort: None,
            max_output_tokens: 1024,
            max_request_bytes: 65536,
            max_calls: 64,
            timeout_ms: 60_000,
            context_chars: 4000,
            history_main: "scripted_ack".to_string(),
            input_price_per_million: None,
            output_price_per_million: None,
            cache_hit_price_per_million: None,
            currency: None,
            price_date: None,
            price_source: None,
        }
    }

    pub fn is_live(&self) -> bool {
        self.endpoint.is_some()
    }

    /// Validate the configuration. In live mode the endpoint must be https
    /// (loopback http is allowed), must not carry credentials, a query or a
    /// fragment, and the credential must come from an environment variable
    /// name and never from a literal.
    pub fn validate(&self, live: bool) -> Result<(), String> {
        if !self.temperature.is_finite() || !(0.0..=2.0).contains(&self.temperature) {
            return Err(format!("temperature out of range: {}", self.temperature));
        }
        if self.max_output_tokens == 0 {
            return Err("max_output_tokens must be positive".into());
        }
        if self.max_request_bytes == 0 {
            return Err("max_request_bytes must be positive".into());
        }
        if self.max_calls == 0 {
            return Err("max_calls must be positive".into());
        }
        if self.timeout_ms == 0 {
            return Err("timeout_ms must be positive".into());
        }
        if !(400..=16_000).contains(&self.context_chars) {
            return Err(format!(
                "context_chars out of range: {}",
                self.context_chars
            ));
        }
        if !matches!(self.history_main.as_str(), "scripted_ack" | "live") {
            return Err(format!("unknown history_main: {}", self.history_main));
        }
        for price in [
            self.input_price_per_million,
            self.output_price_per_million,
            self.cache_hit_price_per_million,
        ]
        .into_iter()
        .flatten()
        {
            if !price.is_finite() || price < 0.0 {
                return Err(format!(
                    "price is not a finite non-negative number: {price}"
                ));
            }
        }
        if !live {
            return Ok(());
        }
        let endpoint = self
            .endpoint
            .as_ref()
            .ok_or_else(|| "live mode requires an endpoint".to_string())?;
        if endpoint.is_empty() {
            return Err("live mode requires a non-empty endpoint".into());
        }
        let is_loopback = endpoint.contains("://127.0.0.1") || endpoint.contains("://localhost");
        if !endpoint.starts_with("https://") && !is_loopback {
            return Err(format!("live endpoint must be https: {endpoint}"));
        }
        if endpoint.contains('@') || endpoint.contains('?') || endpoint.contains('#') {
            return Err("live endpoint must not carry credentials, query or fragment".into());
        }
        if self.model.trim().is_empty() {
            return Err("live mode requires a non-empty model id".into());
        }
        let key_env = self.key_env.as_ref().ok_or_else(|| {
            "live mode requires a credential environment variable name".to_string()
        })?;
        if key_env.is_empty()
            || !key_env
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(format!(
                "credential environment variable name is invalid: {key_env}"
            ));
        }
        Ok(())
    }
}

/// Filesystem layout of a run.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Paths {
    pub input: String,
    pub config: Option<String>,
    pub seal: Option<String>,
    pub freeze: Option<String>,
    pub build: Option<String>,
    pub output: String,
}
