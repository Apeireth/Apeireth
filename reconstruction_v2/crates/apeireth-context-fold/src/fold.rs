//! Fold strategies.

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldStrategy {
    /// Truncate to first N chars.
    Truncate(usize),
    /// Keep first N + last M chars.
    HeadTail { head: usize, tail: usize },
    /// Replace with placeholder (summary must be supplied externally).
    Summary,
    /// Replace with marker placeholder.
    MarkerReplace,
}

#[derive(Debug, Error)]
pub enum FoldError {
    #[error("invalid strategy parameters: {0}")]
    InvalidParameters(String),
    #[error("input empty")]
    EmptyInput,
}

#[derive(Debug, Clone)]
pub struct FoldResult {
    pub folded_text: String,
    pub original_len: usize,
    pub folded_len: usize,
    pub strategy: FoldStrategy,
}

pub fn fold(input: &str, strategy: FoldStrategy) -> Result<FoldResult, FoldError> {
    if input.is_empty() { return Err(FoldError::EmptyInput); }
    let original_len = input.len();
    let folded_text = match strategy {
        FoldStrategy::Truncate(n) => {
            if n == 0 { return Err(FoldError::InvalidParameters("truncate 0".into())); }
            input.chars().take(n).collect()
        }
        FoldStrategy::HeadTail { head, tail } => {
            let chars: Vec<char> = input.chars().collect();
            if head + tail >= chars.len() {
                return Err(FoldError::InvalidParameters("head+tail >= len".into()));
            }
            let mut out = String::new();
            out.extend(chars.iter().take(head));
            out.push_str("...");
            out.extend(chars.iter().skip(chars.len() - tail));
            out
        }
        FoldStrategy::Summary => "[summary]".to_string(),
        FoldStrategy::MarkerReplace => "[folded]".to_string(),
    };
    let folded_len = folded_text.len();
    Ok(FoldResult { folded_text, original_len, folded_len, strategy })
}

pub fn unfold(input: &str, original: &str, strategy: FoldStrategy) -> String {
    match strategy {
        FoldStrategy::MarkerReplace | FoldStrategy::Summary => original.to_string(),
        _ => input.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_works() {
        let r = fold("hello world", FoldStrategy::Truncate(5)).unwrap();
        assert_eq!(r.folded_text, "hello");
    }

    #[test]
    fn head_tail_works() {
        let r = fold("abcdefghij", FoldStrategy::HeadTail { head: 3, tail: 3 }).unwrap();
        assert_eq!(r.folded_text, "abc...hij");
    }

    #[test]
    fn summary_works() {
        let r = fold("hello world", FoldStrategy::Summary).unwrap();
        assert_eq!(r.folded_text, "[summary]");
    }

    #[test]
    fn empty_input_errs() {
        assert!(matches!(fold("", FoldStrategy::Truncate(5)), Err(FoldError::EmptyInput)));
    }

    #[test]
    fn unfold_returns_original_for_marker() {
        let r = unfold("[folded]", "hello world", FoldStrategy::MarkerReplace);
        assert_eq!(r, "hello world");
    }

    #[test]
    fn head_tail_invalid() {
        assert!(matches!(
            fold("abc", FoldStrategy::HeadTail { head: 2, tail: 2 }),
            Err(FoldError::InvalidParameters(_))
        ));
    }
}
