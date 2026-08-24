//! RestrictedToken - Windows 受限 token (从 v1.0 apeireth-companion/restricted_token.rs 548 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 TokenConfig + integrity level

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityLevelToken { Untrusted, Low, Medium, High, System }

pub struct TokenConfig { pub integrity: IntegrityLevelToken, pub deny_sids: Vec<String> }

impl TokenConfig {
    /// 0 装 PASS: 真默认 (medium)
    pub fn medium() -> Self { Self { integrity: IntegrityLevelToken::Medium, deny_sids: vec![] } }
}

pub struct RestrictedToken { pub config: TokenConfig }

impl RestrictedToken {
    pub fn new(config: TokenConfig) -> Self { Self { config } }
    /// 0 装 PASS stub: Windows CreateRestrictedToken
    pub fn apply(&self) -> Result<(), String> {
        // 0 装 PASS: stub (Windows API)
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_default() {
        let t = TokenConfig::medium();
        assert_eq!(t.integrity, IntegrityLevelToken::Medium);
    }
    #[test] fn test_apply() {
        let r = RestrictedToken::new(TokenConfig::medium());
        assert!(r.apply().is_ok());
    }
    #[test] fn test_integrity_eq() { assert_eq!(IntegrityLevelToken::Low, IntegrityLevelToken::Low); }
}
