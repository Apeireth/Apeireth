//! Role divider (VCP roleDivider.js).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivisionDecision {
    Keep,
    Split,
    Merge,
}

#[derive(Debug, Default, Clone)]
pub struct RoleDivider {
    pub max_chars: usize,
}

impl RoleDivider {
    pub fn new(max_chars: usize) -> Self { Self { max_chars } }

    pub fn decide(&self, content: &str) -> DivisionDecision {
        if content.len() <= self.max_chars { DivisionDecision::Keep }
        else if content.len() <= self.max_chars * 2 { DivisionDecision::Split }
        else { DivisionDecision::Merge }
    }
}

/// Divide role content.
pub fn divide_role(content: &str, max_chars: usize) -> DivisionDecision {
    RoleDivider::new(max_chars).decide(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_short() {
        assert_eq!(divide_role("hi", 100), DivisionDecision::Keep);
    }

    #[test]
    fn split_medium() {
        let s = "x".repeat(150);
        assert_eq!(divide_role(&s, 100), DivisionDecision::Split);
    }

    #[test]
    fn merge_long() {
        let s = "x".repeat(300);
        assert_eq!(divide_role(&s, 100), DivisionDecision::Merge);
    }

    #[test]
    fn divider_default() {
        let d = RoleDivider::default();
        assert_eq!(d.max_chars, 0);
    }
}
