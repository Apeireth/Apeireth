//! Fetch engine configuration.

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct FetchConfig {
    pub timeout_ms: u64,
    pub user_agent: String,
    pub max_retries: u32,
    pub cache_ttl_ms: u64,
    pub max_response_bytes: usize,
    pub follow_redirects: bool,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 15_000,
            user_agent: "Apeireth/1.2 (R149 tool-fetch)".into(),
            max_retries: 2,
            cache_ttl_ms: 60_000,
            max_response_bytes: 5 * 1024 * 1024,
            follow_redirects: true,
        }
    }
}

impl FetchConfig {
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_millis(self.cache_ttl_ms)
    }
}
