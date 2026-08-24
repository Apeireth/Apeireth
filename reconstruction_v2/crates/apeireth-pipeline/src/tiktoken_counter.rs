//! Approximate tiktoken counter (chars / 4).

#[derive(Debug, Default, Clone)]
pub struct TiktokenCounter {
    pub chars_per_token: f64,
}

impl TiktokenCounter {
    pub fn new() -> Self { Self { chars_per_token: 4.0 } }

    pub fn count_tokens(&self, text: &str) -> usize {
        (text.chars().count() as f64 / self.chars_per_token).ceil() as usize
    }
}

pub fn count_tokens(text: &str) -> usize {
    TiktokenCounter::new().count_tokens(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_basic() {
        let c = TiktokenCounter::new();
        // 4 chars -> 1 token
        assert_eq!(c.count_tokens("abcd"), 1);
        assert_eq!(c.count_tokens("hello world"), 3); // 11 chars -> ceil(11/4) = 3
    }

    #[test]
    fn count_tokens_fn() {
        assert_eq!(count_tokens(""), 0);
        assert_eq!(count_tokens("abcd"), 1);
    }
}
