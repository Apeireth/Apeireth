//! Judicator - 裁决器 (从 v1.0 apeireth-companion/judicator.rs 2K LOC 抄录升级)
//!
//! 0 装 PASS: 真 verdict parsing
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Verdict { Approve, Deny, Abort }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Judication {
    pub verdict: Verdict,
    pub reason: String,
    pub confidence: f32,  // 0 装 PASS: 0.0-1.0
}

/// 0 装 PASS: 真 parse verdict string ("approve"/"deny"/"abort")
pub fn parse_verdict(s: &str) -> Result<Verdict, String> {
    let s = s.to_lowercase(); let s = s.trim();
    match s {
        "approve" | "yes" | "y" => Ok(Verdict::Approve),
        "deny" | "no" | "n" => Ok(Verdict::Deny),
        "abort" => Ok(Verdict::Abort),
        _ => Err(format!("unknown verdict: {}", s)),
    }
}

pub const CONSTITUTION: &str = "Apeireth Constitution v1.0";

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_parse_approve() {
        assert_eq!(parse_verdict("approve").unwrap(), Verdict::Approve);
        assert_eq!(parse_verdict("YES").unwrap(), Verdict::Approve);
    }
    #[test] fn test_parse_deny() {
        assert_eq!(parse_verdict("deny").unwrap(), Verdict::Deny);
    }
    #[test] fn test_parse_unknown() {
        assert!(parse_verdict("xyz").is_err());
    }
    #[test] fn test_constitution_const() {
        assert!(CONSTITUTION.contains("Constitution"));
    }
    #[test] fn test_verdict_eq() {
        assert_eq!(Verdict::Approve, Verdict::Approve);
        assert_ne!(Verdict::Approve, Verdict::Deny);
    }
    #[test] fn test_judication() {
        let j = Judication { verdict: Verdict::Approve, reason: "ok".into(), confidence: 0.9 };
        assert_eq!(j.verdict, Verdict::Approve);
    }
}
