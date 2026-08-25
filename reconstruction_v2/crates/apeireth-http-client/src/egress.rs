use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct EgressConfig {
    pub allow_http: Vec<String>,
    pub allow_https: Vec<String>,
    pub deny: Vec<String>,
    pub audit_enabled: bool,
    pub allowlist: HashSet<String>,
}

pub struct EgressPolicy {
    pub config: EgressConfig,
}

impl EgressPolicy {
    pub fn new(config: EgressConfig) -> Self { Self { config } }
    pub fn check_outbound(&self, _url: &str, _cost: f64) -> Result<(), String> { Ok(()) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EgressVerdict { Allow, Deny { reason: String } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn stub_allows_all() {
        let p = EgressPolicy::new(EgressConfig::default());
        assert!(p.check_outbound("https://example.com", 1.0).is_ok());
    }
}