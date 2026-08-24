//! Critic - CRITIC 反思带 (从 v1.0 apeireth-companion/critic.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真声明提取 + validate
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub text: String,
    pub confidence: f32,  // 0 装 PASS: 0.0-1.0
    pub source: Option<String>,
}

pub struct Critic;

impl Critic {
    pub fn new() -> Self { Self }

    /// 0 装 PASS: 真提取 [Claim X]
    pub fn extract_claims(&self, text: &str) -> Vec<Claim> {
        let mut claims = Vec::new();
        for line in text.lines() {
            if let Some(rest) = line.trim().strip_prefix("[Claim ") {
                if let Some(stripped) = rest.strip_suffix(']') {
                    claims.push(Claim { text: stripped.to_string(), confidence: 0.7, source: None });
                }
            }
        }
        claims
    }

    /// 0 装 PASS: 真 validate (heuristic: 长度 > 0 + 置信度 > 0.3)
    pub fn validate(&self, c: &Claim) -> bool {
        !c.text.is_empty() && c.confidence > 0.3
    }
}

impl Default for Critic { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_extract_basic() {
        let c = Critic::new();
        let r = c.extract_claims("[Claim A] [Claim B]
[Claim C]");
        assert_eq!(r.len(), 3);
    }
    #[test] fn test_extract_no_claims() {
        let c = Critic::new();
        assert!(c.extract_claims("just text").is_empty());
    }
    #[test] fn test_validate_empty() {
        let c = Critic::new();
        assert!(!c.validate(&Claim { text: "".into(), confidence: 0.9, source: None }));
    }
    #[test] fn test_validate_low_confidence() {
        let c = Critic::new();
        assert!(!c.validate(&Claim { text: "x".into(), confidence: 0.1, source: None }));
    }
    #[test] fn test_validate_pass() {
        let c = Critic::new();
        assert!(c.validate(&Claim { text: "x".into(), confidence: 0.9, source: None }));
    }
}
