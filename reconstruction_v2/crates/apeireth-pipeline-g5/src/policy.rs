//! Stage 2: Default policy.

pub const MAX_POLICY_ATTEMPTS: usize = 3;
pub const MAX_POLICY_PAYLOAD_SIZE: usize = 64 * 1024;
pub const POLICY_DENY_KINDS: &[&str] = &["forbidden", "blocked"];
pub const POLICY_REQUIRE_KIND: &str = "chat";

#[derive(Debug, Default, Clone)]
pub struct DefaultPolicy;

impl DefaultPolicy {
    pub fn new() -> Self { Self }
    pub fn allow(&self, kind: &str) -> bool {
        !POLICY_DENY_KINDS.contains(&kind)
    }
    pub fn enforce_size(&self, size: usize) -> bool {
        size <= MAX_POLICY_PAYLOAD_SIZE
    }
    pub fn required_kind() -> &'static str { POLICY_REQUIRE_KIND }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_normal() {
        let p = DefaultPolicy::new();
        assert!(p.allow("chat"));
        assert!(!p.allow("forbidden"));
    }

    #[test]
    fn enforce_size_works() {
        let p = DefaultPolicy::new();
        assert!(p.enforce_size(100));
        assert!(!p.enforce_size(MAX_POLICY_PAYLOAD_SIZE + 1));
    }

    #[test]
    fn constants() {
        assert_eq!(MAX_POLICY_ATTEMPTS, 3);
        assert_eq!(POLICY_REQUIRE_KIND, "chat");
        assert!(POLICY_DENY_KINDS.len() >= 2);
    }
}
