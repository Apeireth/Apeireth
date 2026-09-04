//! Closed-world memory injection (donor `apeireth-companion::memory_injection`).
//!
//! LLM retrieval easily fabricates "I remember we talked about…". The donor
//! template treats retrieved items as a **closed world of numbered evidence**:
//! numbered list + source truncation + an explicit anti-hallucination rule
//! forbidding claims outside the list.
//!
//! This module is a pure renderer. It does not own a store, does not call a
//! provider, and is not production-wired. Callers assemble the entry strings.
//!
//! Recovered from:
//! - `legacy/donor/apeireth-companion/src/memory_injection.rs`
//! - preference portrait rendering in `memory_extractor.rs::preference_injection`

/// Maximum visible characters per evidence line (donor truncation).
pub const EVIDENCE_MAX_CHARS: usize = 120;

/// Maximum preference portrait lines (donor `take(8)`).
pub const PREFERENCE_INJECTION_LIMIT: usize = 8;

/// Closed-world evidence block: numbered list + anti-hallucination rules.
///
/// Empty input yields an empty string (no injection).
pub fn build_memory_injection(entries: &[String]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut s = String::from("[记忆证据 — 你只知道以下条目, 不要声称记得列表之外的任何对话]\n");
    for (i, e) in entries.iter().enumerate() {
        s.push_str(&format!(
            "{}. {}\n",
            i + 1,
            e.chars().take(EVIDENCE_MAX_CHARS).collect::<String>()
        ));
    }
    s.push_str(
        "规则: 说话只能基于以上编号条目; 不确定就说「我猜」; \
         禁止说「我记得我们以前聊过」— 那是编造。",
    );
    s
}

/// Preference portrait injection: importance-sorted, truncated lines.
///
/// `entries` is `(importance 1..=10, content)`. Higher importance first;
/// ties keep input order. Empty input yields an empty string.
pub fn build_preference_injection(entries: &[(u8, String)]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut ranked: Vec<(usize, u8, &str)> = entries
        .iter()
        .enumerate()
        .map(|(i, (imp, content))| (i, *imp, content.as_str()))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut s = String::from("【主人偏好画像】(来自记忆提炼, 做审美/风格/交互类事情时优先沿用):\n");
    for (_, _, content) in ranked.iter().take(PREFERENCE_INJECTION_LIMIT) {
        s.push_str(&format!(
            "  • {}\n",
            content.chars().take(EVIDENCE_MAX_CHARS).collect::<String>()
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_entries_no_injection() {
        assert_eq!(build_memory_injection(&[]), "");
    }

    #[test]
    fn entries_numbered_with_closure_rules() {
        let s = build_memory_injection(&[
            "主人明天要交线代作业".to_string(),
            "主人换元法常忘换 dx".to_string(),
        ]);
        assert!(s.contains("[记忆证据"));
        assert!(s.contains("1. 主人明天要交线代作业"));
        assert!(s.contains("2. 主人换元法常忘换 dx"));
        assert!(
            s.contains("禁止说「我记得我们以前聊过」"),
            "anti-hallucination rule must exist: {s}"
        );
        assert!(s.contains("我猜"), "uncertainty cue must exist");
    }

    #[test]
    fn long_entries_truncated() {
        let long = "x".repeat(300);
        let s = build_memory_injection(&[long]);
        assert!(
            s.matches('x').count() <= EVIDENCE_MAX_CHARS,
            "entries truncate to {EVIDENCE_MAX_CHARS} chars: {}",
            s.matches('x').count()
        );
        assert!(
            s.contains("禁止说"),
            "anti-hallucination rule still present"
        );
    }

    #[test]
    fn preference_injection_empty() {
        assert!(build_preference_injection(&[]).is_empty());
    }

    #[test]
    fn preference_injection_sorts_by_importance_and_caps() {
        let mut entries = Vec::new();
        for i in 1..=10 {
            entries.push((i as u8, format!("pref-{i}")));
        }
        let s = build_preference_injection(&entries);
        assert!(s.contains("【主人偏好画像】"));
        assert!(s.contains("pref-10"));
        assert!(s.contains("pref-3"), "take(8) keeps importance 3..10");
        assert!(!s.contains("pref-2"), "ninth-lowest importance is dropped");
        let pos10 = s.find("pref-10").unwrap();
        let pos9 = s.find("pref-9").unwrap();
        assert!(pos10 < pos9, "higher importance renders first");
    }

    #[test]
    fn preference_injection_truncates_line() {
        let long = "y".repeat(300);
        let s = build_preference_injection(&[(8, long)]);
        assert!(s.matches('y').count() <= EVIDENCE_MAX_CHARS);
    }
}
