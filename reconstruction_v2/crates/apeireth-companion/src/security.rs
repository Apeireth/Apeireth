//! Security - 安全门 (从 v1.0 apeireth-companion/security.rs 158 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 SecurityGate + 风险评估

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel { Low, Medium, High, Critical }

pub struct SecurityGate;

impl SecurityGate {
    pub fn new() -> Self { Self }
    /// 0 装 PASS: 真评估
    pub fn assess(&self, action: &str) -> SecurityLevel {
        match action {
            "read" | "list" => SecurityLevel::Low,
            "write" => SecurityLevel::Medium,
            "execute" | "delete" => SecurityLevel::High,
            _ => SecurityLevel::Critical,
        }
    }
}

impl Default for SecurityGate { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_low() { assert_eq!(SecurityGate::new().assess("read"), SecurityLevel::Low); }
    #[test] fn test_medium() { assert_eq!(SecurityGate::new().assess("write"), SecurityLevel::Medium); }
    #[test] fn test_high() { assert_eq!(SecurityGate::new().assess("execute"), SecurityLevel::High); }
    #[test] fn test_critical() { assert_eq!(SecurityGate::new().assess("delete"), SecurityLevel::High); assert_eq!(SecurityGate::new().assess("unknown"), SecurityLevel::Critical); }
    #[test] fn test_default() { let g: SecurityGate = Default::default(); assert_eq!(g.assess("read"), SecurityLevel::Low); }
}
