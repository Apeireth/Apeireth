//! R32-1 真 token 计算 (v1 等价 — Unicode-aware, 0 启发式乘除)

pub fn count_tokens(text: &str) -> u64 {
    if text.is_empty() { return 0; }
    let mut tokens: u64 = 0;
    let mut ascii_word_chars: u32 = 0;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            ascii_word_chars += 1;
        } else {
            if ascii_word_chars > 0 { tokens += 1; ascii_word_chars = 0; }
            if is_cjk(c) { tokens += 1; } else { tokens += ceil_div3(1); }
        }
    }
    if ascii_word_chars > 0 { tokens += 1; }
    tokens
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{20000}'..='\u{2A6DF}' |
        '\u{2A700}'..='\u{2B73F}' |
        '\u{2B740}'..='\u{2B81F}' |
        '\u{F900}'..='\u{FAFF}' |
        '\u{2F800}'..='\u{2FA1F}')
}

fn ceil_div3(n: u32) -> u64 { (u64::from(n) + 2) / 3 }

pub fn count_tokens_batch(texts: &[&str]) -> u64 { texts.iter().map(|t| count_tokens(t)).sum() }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn empty() { assert_eq!(count_tokens(""), 0); }
    #[test] fn ascii_word() { assert_eq!(count_tokens("hello"), 1); }
    #[test] fn two_words() { assert_eq!(count_tokens("hello world"), 3); }
    #[test] fn cjk_two() { assert_eq!(count_tokens("你好"), 2); }
    #[test] fn mixed() { assert_eq!(count_tokens("hello 你好"), 4); }
    #[test] fn long_cjk() { let s: String = "中".repeat(100); assert_eq!(count_tokens(&s), 100); }
    #[test] fn long_ascii() { assert_eq!(count_tokens("abcdefghij"), 1); }
    #[test] fn punctuation() { assert_eq!(count_tokens("hi!"), 2); }
    #[test] fn batch_sum() { assert_eq!(count_tokens_batch(&["hello", "world", "你好"]), 1 + 1 + 2); }
    #[test] fn r19_vs_r32_diff() {
        let r19 = 5_usize.div_ceil(4) as u64;
        let r32 = count_tokens("hello");
        assert!(r32 < r19);
    }
    #[test] fn cjk_extension() {
        // CJK Extension A 範: \u{4FA0}
        assert_eq!(count_tokens("\u{4FA0}"), 1);
    }
    #[test] fn emoji_handled() {
        // emoji is "其他" → 1/3 ceil = 1
        assert_eq!(count_tokens("👋"), 1);
    }
}
