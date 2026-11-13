use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::io::EvalResult;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub provider: String,
    pub endpoint: String,
    pub model: String,
    pub key_env: String,
    pub temperature: f32,
    pub max_output_tokens: u32,
    pub max_request_bytes: usize,
    pub max_calls: usize,
    pub repetitions: usize,
    pub timeout_ms: u64,
    pub output: PathBuf,
    pub cases: Vec<String>,
    pub thinking: String,
    pub reasoning_effort: Option<String>,
}

impl Config {
    pub fn offline(output: PathBuf) -> Self {
        Self {
            provider: "offline-contract".into(),
            endpoint: String::new(),
            model: "memory-offline-contract".into(),
            key_env: String::new(),
            temperature: 0.0,
            max_output_tokens: 1024,
            max_request_bytes: 24576,
            max_calls: 128,
            repetitions: 1,
            timeout_ms: 60_000,
            output,
            cases: vec![],
            thinking: "disabled".into(),
            reasoning_effort: Some("none".into()),
        }
    }

    pub fn parse(args: &[String], live: bool) -> EvalResult<Self> {
        let mut config = Self::offline(PathBuf::new());
        let mut explicit = std::collections::BTreeSet::new();
        let mut items = args.iter();
        while let Some(flag) = items.next() {
            let value = items
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            if !explicit.insert(flag.as_str()) {
                return Err(format!("duplicate option {flag}").into());
            }
            match flag.as_str() {
                "--provider" => config.provider = value.clone(),
                "--endpoint" => config.endpoint = value.clone(),
                "--model" => config.model = value.clone(),
                "--key-env" => config.key_env = value.clone(),
                "--temperature" => config.temperature = value.parse()?,
                "--max-output-tokens" => config.max_output_tokens = value.parse()?,
                "--max-request-bytes" => config.max_request_bytes = value.parse()?,
                "--max-calls" => config.max_calls = value.parse()?,
                "--repetitions" => config.repetitions = value.parse()?,
                "--timeout-ms" => config.timeout_ms = value.parse()?,
                "--output" => config.output = value.into(),
                "--cases" => {
                    config.cases = if value == "all" {
                        vec![]
                    } else {
                        value.split(',').map(|s| s.trim().to_owned()).collect()
                    }
                }
                "--thinking" => config.thinking = value.clone(),
                "--reasoning-effort" => {
                    config.reasoning_effort = (value != "omit").then(|| value.clone())
                }
                _ => return Err(format!("unknown option {flag}").into()),
            }
        }
        if live {
            for required in [
                "--provider",
                "--endpoint",
                "--model",
                "--key-env",
                "--temperature",
                "--max-output-tokens",
                "--max-calls",
                "--repetitions",
                "--output",
                "--thinking",
            ] {
                if !explicit.contains(required) {
                    return Err(format!("run-live requires explicit {required}").into());
                }
            }
            if config.provider != "compatible" {
                return Err("this entry currently connects only provider=compatible; no implicit adapter substitution".into());
            }
            if config.model.trim().is_empty() || config.key_env.trim().is_empty() {
                return Err("model and key-env must be nonempty".into());
            }
            if !config
                .key_env
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err("key-env must name one environment variable".into());
            }
        }
        if !config.endpoint.is_empty() {
            let url = reqwest::Url::parse(&config.endpoint).map_err(|_| "invalid endpoint URL")?;
            let loopback = matches!(
                url.host_str(),
                Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
            );
            if (url.scheme() != "https" && !(url.scheme() == "http" && loopback))
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err("endpoint must be HTTPS (or loopback HTTP), without credentials, query or fragment".into());
            }
        }
        if !config.temperature.is_finite()
            || !(0.0..=2.0).contains(&config.temperature)
            || config.max_output_tokens == 0
            || config.max_request_bytes == 0
            || config.max_calls == 0
            || config.repetitions == 0
            || config.repetitions > 100
            || config.timeout_ms == 0
        {
            return Err("invalid finite sampling/call/request/repetition/timeout budget".into());
        }
        if config
            .cases
            .iter()
            .any(|id| !(1..=12).any(|n| *id == format!("e{n:02}")))
        {
            return Err("cases must name frozen IDs e01 through e12".into());
        }
        if (live || !args.is_empty())
            && config.output.as_os_str().is_empty()
            && explicit.contains("--output")
        {
            return Err("output must be a new nonempty directory path".into());
        }
        super::wire::build_body(
            &apeireth_protocol::canonical::NormalizedRequest::new(&config.model, vec![]),
            &config.model,
            "provider.eval.preflight",
            &config.wire_options(),
        )?;
        Ok(config)
    }

    pub fn selected(&self, id: &str) -> bool {
        self.cases.is_empty() || self.cases.iter().any(|selected| selected == id)
    }
    pub fn wire_options(&self) -> super::wire::WireOptions {
        super::wire::WireOptions {
            thinking: self.thinking.clone(),
            reasoning_effort: self.reasoning_effort.clone(),
        }
    }
}
