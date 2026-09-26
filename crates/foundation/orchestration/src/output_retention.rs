//! Output retention primitive: UTF-8-safe truncation with graded omission
//! wording.
//!
//! Two bare failure modes this module removes:
//! - **Cuts that split characters.** A byte budget applied naively can cut a
//!   multi-byte character (or an astral character carried as a surrogate pair
//!   on UTF-16 surfaces) in half and emit invalid text. Every cut here lands on
//!   a UTF-8 character boundary by construction.
//! - **Omission wording with invented precision.** A note that claims a count
//!   it did not measure is worse than no count. [`OmittedHow`] grades the
//!   wording three ways: say the exact number when one was measured, say
//!   plainly that content was omitted when one was not, and say nothing at all
//!   when nothing was omitted.
//!
//! Accounting rule (anti-fake-precision): the omitted amount is **measured
//! from what was actually retained** (`original bytes - retained bytes` after
//! the cut), never derived from the requested budget — rounding to a character
//! boundary keeps less than the budget, and the note must not overstate.
//!
//! Shared core, not a third copy: the boundary-safe cut helpers
//! ([`prefix_within_bytes`] / [`prefix_within_chars`] / [`suffix_within_chars`])
//! and the graded note builder ([`omission_note`]) are the single
//! implementation reused by [`crate::context_budget`]'s head/tail preview and
//! spill-backed truncation; their existing wording and behaviour stay intact.
//! Library primitive only: this module performs no I/O and owns no session.

/// The unit label used for byte-measured omission counts.
const BYTE_UNIT: &str = "字节";

/// How much content an omission removed — graded so the wording never invents
/// a count it does not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmittedHow {
    /// Nothing was omitted; the note must stay silent.
    None,
    /// Exactly `count` units were omitted. The count was measured from the
    /// content actually retained (bytes for [`retain_within_bytes`]), never
    /// derived from the requested budget.
    Exact { count: usize },
    /// The omitted amount is not knowable here (the source itself is already a
    /// fragment). Say so plainly instead of fabricating a number.
    Unknown,
}

/// Where the omitted content sat relative to what was kept, so the note reads
/// correctly for both retention shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OmittedWhere {
    /// The kept parts bracket the omission (head + tail preview).
    Middle,
    /// Everything after the kept prefix is gone (prefix truncation).
    Tail,
}

/// Graded omission note (anti-fake-precision wording).
///
/// - [`OmittedHow::None`] → empty string: never claim an omission that did not
///   happen.
/// - [`OmittedHow::Exact`] → the measured count plus its unit, placed to match
///   the retention shape (e.g. `…[中间 400 字符已省略]…`).
/// - [`OmittedHow::Unknown`] → `部分内容已省略`: honest about the loss without
///   a fake number.
pub fn omission_note(how: &OmittedHow, where_: OmittedWhere, unit: &str) -> String {
    match how {
        OmittedHow::None => String::new(),
        OmittedHow::Exact { count } => match where_ {
            OmittedWhere::Middle => format!("…[中间 {count} {unit}已省略]…"),
            OmittedWhere::Tail => format!("…[尾部 {count} {unit}已省略]…"),
        },
        OmittedHow::Unknown => "部分内容已省略".to_string(),
    }
}

/// The outcome of one retention cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedOutput {
    /// What survives the cut. Byte-for-byte the original when nothing was
    /// omitted.
    pub text: String,
    /// Graded omission accounting for the cut.
    pub omitted: OmittedHow,
}

impl RetainedOutput {
    /// Bytes actually retained — the accounting base of `omitted`.
    pub fn retained_bytes(&self) -> usize {
        self.text.len()
    }

    /// The graded note for this prefix cut (`""` when nothing was omitted).
    pub fn note(&self) -> String {
        omission_note(&self.omitted, OmittedWhere::Tail, BYTE_UNIT)
    }

    /// The retained text with its note appended as a separate line (only when
    /// something was omitted).
    pub fn text_with_note(&self) -> String {
        let note = self.note();
        if note.is_empty() {
            self.text.clone()
        } else {
            format!("{}\n{}", self.text, note)
        }
    }
}

/// Largest prefix of `content` that fits within `byte_budget` bytes, cut only
/// at a UTF-8 character boundary.
///
/// A budget that lands inside a multi-byte character retreats to the start of
/// that character, so a 2/3/4-byte character is never split — and a
/// supplementary-plane (4-byte, surrogate-pair elsewhere) character survives
/// whole or not at all.
pub fn prefix_within_bytes(content: &str, byte_budget: usize) -> &str {
    if byte_budget >= content.len() {
        return content;
    }
    let mut cut = byte_budget;
    while cut > 0 && !content.is_char_boundary(cut) {
        cut -= 1;
    }
    &content[..cut]
}

/// Largest prefix of `content` holding `char_budget` Unicode scalar values
/// (cut at a character boundary by construction).
pub fn prefix_within_chars(content: &str, char_budget: usize) -> &str {
    match content.char_indices().nth(char_budget) {
        Some((byte_index, _)) => &content[..byte_index],
        None => content,
    }
}

/// Last `char_budget` Unicode scalar values of `content` (cut at a character
/// boundary by construction).
pub fn suffix_within_chars(content: &str, char_budget: usize) -> &str {
    let total = content.chars().count();
    if char_budget >= total {
        return content;
    }
    match content.char_indices().nth(total - char_budget) {
        Some((byte_index, _)) => &content[byte_index..],
        None => "",
    }
}

/// UTF-8-safe truncation of a **complete** source to a byte budget.
///
/// Nothing is omitted below the budget (the original is returned unchanged).
/// On a cut the omitted count is measured from the bytes actually retained:
/// `content.len() - kept.len()`, which is larger than `content.len() -
/// byte_budget` whenever boundary rounding bit — the note reports what
/// happened, not what was asked for.
pub fn retain_within_bytes(content: &str, byte_budget: usize) -> RetainedOutput {
    let kept = prefix_within_bytes(content, byte_budget);
    if kept.len() == content.len() {
        return RetainedOutput {
            text: content.to_string(),
            omitted: OmittedHow::None,
        };
    }
    RetainedOutput {
        text: kept.to_string(),
        // Measured, not budget-derived: see the module accounting rule.
        omitted: OmittedHow::Exact {
            count: content.len() - kept.len(),
        },
    }
}

/// UTF-8-safe truncation of a source that is **already a fragment** (a slice
/// of something larger whose original size is unknown here).
///
/// The cut is as boundary-safe as [`retain_within_bytes`], but the omission is
/// graded [`OmittedHow::Unknown`]: the amount lost *here* is measurable, yet
/// the note must not imply that the retained text is everything that exists —
/// so it says `部分内容已省略` instead of a count.
pub fn retain_fragment_within_bytes(content: &str, byte_budget: usize) -> RetainedOutput {
    let kept = prefix_within_bytes(content, byte_budget);
    if kept.len() == content.len() {
        return RetainedOutput {
            text: content.to_string(),
            omitted: OmittedHow::None,
        };
    }
    RetainedOutput {
        text: kept.to_string(),
        omitted: OmittedHow::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every budget on multi-byte content yields a whole-character prefix:
    /// 2-byte, 3-byte and 4-byte (surrogate-pair elsewhere) characters are
    /// never split, and the result is always valid text.
    #[test]
    fn truncation_never_splits_multibyte_characters() {
        // 2-byte "é", 3-byte "語", 4-byte "🦀" mixed with ASCII.
        let content = "aé語🦀b";
        for budget in 0..=content.len() {
            let kept = prefix_within_bytes(content, budget);
            assert!(
                content.starts_with(kept),
                "budget {budget}: kept must be a prefix: {kept:?}"
            );
            assert!(
                kept.len() <= budget,
                "budget {budget}: kept {} bytes must fit",
                kept.len()
            );
            // Boundary retreat: the byte just past the kept prefix starts a new
            // character (or the content ends) — never mid-character.
            if kept.len() < content.len() {
                assert!(content.is_char_boundary(kept.len()));
            }
        }
        // Whole astral characters only: a budget of 3 bytes on "🦀x" keeps nothing
        // of the emoji rather than half of it.
        assert_eq!(prefix_within_bytes("🦀x", 3), "");
        assert_eq!(prefix_within_bytes("🦀x", 4), "🦀");
        // Budget inside the 3-byte character retreats to before it.
        assert_eq!(prefix_within_bytes("日本語", 5), "日");
        assert_eq!(prefix_within_bytes("日本語", 6), "日本");
    }

    /// The wording is graded three ways: silent when nothing was omitted, the
    /// exact measured count when one exists, and a plain admission without a
    /// number when one does not.
    #[test]
    fn omission_wording_is_graded_three_ways() {
        // None: no note at all, never a claim of omission.
        assert_eq!(
            omission_note(&OmittedHow::None, OmittedWhere::Tail, BYTE_UNIT),
            ""
        );

        // Exact: the measured number and its unit, placed per retention shape.
        let tail = omission_note(
            &OmittedHow::Exact { count: 24 },
            OmittedWhere::Tail,
            BYTE_UNIT,
        );
        assert!(tail.contains("24"), "精确数必须出现: {tail}");
        assert!(tail.contains(BYTE_UNIT), "单位必须出现: {tail}");
        assert!(tail.contains("已省略"), "省略必须声明: {tail}");

        let middle = omission_note(
            &OmittedHow::Exact { count: 400 },
            OmittedWhere::Middle,
            "字符",
        );
        assert_eq!(middle, "…[中间 400 字符已省略]…");

        // Unknown: honest admission, no invented number.
        let unknown = omission_note(&OmittedHow::Unknown, OmittedWhere::Tail, BYTE_UNIT);
        assert_eq!(unknown, "部分内容已省略");
        assert!(
            !unknown.chars().any(|c| c.is_ascii_digit()),
            "Unknown 不得携带任何数字"
        );
    }

    /// The omitted count is measured from the bytes actually retained, not
    /// derived from the requested budget — boundary rounding shows up as an
    /// honest difference between the two.
    #[test]
    fn omitted_count_is_measured_not_budget_derived() {
        // 3 characters, 9 bytes. Budget 8 lands mid-character: only 6 bytes
        // survive, so 3 bytes were omitted — not the budget-derived 1.
        let out = retain_within_bytes("日本語", 8);
        assert_eq!(out.text, "日本");
        assert_eq!(out.retained_bytes(), 6);
        assert_eq!(out.omitted, OmittedHow::Exact { count: 3 });
        assert_eq!(out.note(), "…[尾部 3 字节已省略]…");

        // The measured count always equals original - retained.
        let out = retain_within_bytes("🦀🦀🦀", 10);
        assert_eq!(out.text, "🦀🦀");
        assert_eq!(out.omitted, OmittedHow::Exact { count: 4 });

        // text_with_note carries the note on its own line, text untouched.
        let joined = out.text_with_note();
        let mut lines = joined.lines();
        assert_eq!(lines.next(), Some("🦀🦀"));
        assert_eq!(lines.next(), Some("…[尾部 4 字节已省略]…"));
        assert_eq!(lines.next(), None);
    }

    /// Empty, exact-fit and over-long boundaries all stay sane.
    #[test]
    fn empty_and_oversized_boundaries() {
        // Empty content: nothing to retain, nothing omitted.
        let out = retain_within_bytes("", 0);
        assert_eq!(out.text, "");
        assert_eq!(out.omitted, OmittedHow::None);
        assert_eq!(out.note(), "");
        assert_eq!(out.text_with_note(), "");

        let out = retain_within_bytes("", 100);
        assert_eq!(out.omitted, OmittedHow::None);

        // Exactly at the budget: zero change, no omission.
        let out = retain_within_bytes("hello", 5);
        assert_eq!(out.text, "hello");
        assert_eq!(out.omitted, OmittedHow::None);

        // Beyond the budget: zero change too.
        let out = retain_within_bytes("hello", 5_000);
        assert_eq!(out.text, "hello");
        assert_eq!(out.omitted, OmittedHow::None);

        // Zero budget on non-empty content: everything is omitted and the
        // count says so exactly.
        let out = retain_within_bytes("hello", 0);
        assert_eq!(out.text, "");
        assert_eq!(out.omitted, OmittedHow::Exact { count: 5 });

        // Oversized content stays whole-characters at the cut.
        let long = "語".repeat(10_000);
        let out = retain_within_bytes(&long, 7);
        assert_eq!(out.text, "語語", "预算 7 → 退回字符边界, 实留 6 字节");
        assert_eq!(out.omitted, OmittedHow::Exact { count: 29_994 });
    }

    /// A fragment source never invents a count: the cut is boundary-safe but
    /// the note admits only that *something* was omitted.
    #[test]
    fn fragment_source_never_invents_a_count() {
        let out = retain_fragment_within_bytes("日本語", 8);
        assert_eq!(out.text, "日本", "同样的 UTF-8 安全切点");
        assert_eq!(out.omitted, OmittedHow::Unknown);
        assert_eq!(out.note(), "部分内容已省略");

        // No cut: no omission claim either, fragment or not.
        let out = retain_fragment_within_bytes("日本語", 9);
        assert_eq!(out.omitted, OmittedHow::None);
        assert_eq!(out.note(), "");
    }

    /// Zero conflict with the existing truncation surfaces: their pinned
    /// wording and cut behaviour are reproduced exactly by the shared core.
    #[test]
    fn zero_conflict_with_existing_truncation() {
        // The head/tail middle marker keeps its exact historical wording.
        let marker = omission_note(
            &OmittedHow::Exact { count: 400 },
            OmittedWhere::Middle,
            "字符",
        );
        assert_eq!(marker, "…[中间 400 字符已省略]…");

        // The char-budget helpers agree with scalar-value counting everywhere.
        let content = "ab🦀cd🦀ef";
        for keep in 0..=content.chars().count() {
            let expected_head: String = content.chars().take(keep).collect();
            assert_eq!(
                prefix_within_chars(content, keep),
                expected_head,
                "keep={keep}"
            );
            let expected_tail: String = content
                .chars()
                .skip(content.chars().count() - keep)
                .collect();
            assert_eq!(
                suffix_within_chars(content, keep),
                expected_tail,
                "keep={keep}"
            );
        }

        // The byte-budget prefix matches a plain byte cut whenever that cut is
        // already on a boundary (ASCII and aligned multi-byte alike).
        assert_eq!(prefix_within_bytes("abcdefgh", 3), "abc");
        assert_eq!(prefix_within_bytes("日本語", 6), "日本");
    }
}
