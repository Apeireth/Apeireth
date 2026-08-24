//! Token budget (VCP §6.2.2 #15).

pub const MAX_INJECTION_CHARS: usize = 16_000;
pub const MIN_INJECTION_CHARS: usize = 0;
pub const DEFAULT_BRIEF_TOKEN_BUDGET: usize = 512;
pub const LIGHT_LIST_TOKEN_BUDGET: usize = 256;

/// Truncate a string to a maximum length.
pub fn truncate_to_max(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { s[..max].to_string() }
}

/// Whether the string exceeds the budget.
pub fn exceeds_budget(s: &str, max: usize) -> bool {
    s.len() > max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short() {
        assert_eq!(truncate_to_max("hi", 100), "hi");
    }

    #[test]
    fn truncate_long() {
        let s = "a".repeat(20);
        assert_eq!(truncate_to_max(&s, 5).len(), 5);
    }

    #[test]
    fn exceeds_works() {
        assert!(!exceeds_budget("hi", 5));
        assert!(exceeds_budget("hello world", 5));
    }

    #[test]
    fn constants() {
        assert_eq!(MAX_INJECTION_CHARS, 16_000);
        assert_eq!(DEFAULT_BRIEF_TOKEN_BUDGET, 512);
    }
}
