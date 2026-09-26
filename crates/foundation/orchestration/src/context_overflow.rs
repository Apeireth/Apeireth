//! Trigger math for context overflow, and the pure decision rules for
//! budget-shrink recovery.
//!
//! Two questions, both answered by pure functions over pure parameters (no
//! I/O, no state):
//!
//! - **When** to compact or truncate: [`trigger_threshold_tokens`] derives the
//!   estimated-token threshold from the model's context window, the overhead
//!   already committed (system block, tool declarations, protocol framing),
//!   and a fixed reserve; [`exceeds_trigger`] is the boundary comparator.
//! - **How much tail survives verbatim**: [`retain_tail_tokens`] keeps a
//!   configurable share of the post-overhead window (default
//!   [`DEFAULT_RETAIN_TAIL_RATIO`]).
//!
//! Plus the decision rules a budget-shrink retry obeys after the provider
//! reports the context window as exceeded: [`shrink_budget`] (monotonic budget
//! descent) and [`retry_makes_progress`] (a retry must reassemble differently,
//! or the original error stands). The retry loop itself lives in the runtime.

/// Fixed token reserve kept free for the reply scaffold and protocol framing.
///
/// Derivation: the trigger must leave room for the assistant's reply on top of
/// everything the request already commits — a request that fills the window to
/// the brim leaves the provider nothing to answer with. The reserve is a
/// constant rather than a ratio so the trigger stays stable and testable at
/// boundary values; 512 tokens is the smallest reserve that still fits a
/// multi-paragraph answer with framing at the conservative `chars / 4` token
/// estimate this codebase uses.
pub const RESERVE_TOKENS: u64 = 512;

/// Default share of the post-overhead window kept verbatim at the tail when a
/// long context is compacted. Configurable per call via
/// [`retain_tail_tokens`]; the value leaves the larger share to the head
/// (where instructions and identity live) while keeping a meaningful tail for
/// recency.
pub const DEFAULT_RETAIN_TAIL_RATIO: f64 = 0.16;

/// How many budget-shrink retries one provider request may make after a
/// context-window-exceeded report. Two retries bound the recovery at three
/// provider calls: enough to step the budget down meaningfully, small enough
/// that a wrong classification cannot amplify into a retry storm.
pub const MAX_OVERFLOW_RETRIES: u32 = 2;

/// Budget-shrink factor numerator (with [`BUDGET_SHRINK_DENOMINATOR`]):
/// `next budget = current * 7 / 10`.
pub const BUDGET_SHRINK_NUMERATOR: u64 = 7;

/// Budget-shrink factor denominator (with [`BUDGET_SHRINK_NUMERATOR`]).
pub const BUDGET_SHRINK_DENOMINATOR: u64 = 10;

/// Characters per estimated token — the `chars / 4` convention shared with
/// `context_fold::approx_tokens`.
const CHARS_PER_TOKEN: u64 = 4;

/// `trigger = floor(min(window * 0.8, window - overhead - reserve))`, in tokens.
///
/// Both bounds matter: `0.8 * window` keeps the request clear of the window
/// edge whatever the overhead is, while `window - overhead - reserve` accounts
/// for what the request must already carry (and the answer still needs). The
/// smaller bound wins, and the result floors at 0 when the overhead alone
/// leaves no headroom — never negative, never larger than the scaled window.
pub fn trigger_threshold_tokens(
    window_tokens: u64,
    overhead_tokens: u64,
    reserve_tokens: u64,
) -> u64 {
    // floor(0.8 * window) without floating point or overflow.
    let scaled_window = window_tokens / 5 * 4 + (window_tokens % 5) * 4 / 5;
    let headroom = window_tokens
        .saturating_sub(overhead_tokens)
        .saturating_sub(reserve_tokens);
    scaled_window.min(headroom)
}

/// [`trigger_threshold_tokens`] over char counts, converted to tokens with the
/// `chars / 4` estimate (window and overhead arrive as char counts in this
/// codebase's budget knobs).
pub fn trigger_threshold_chars(window_chars: u64, overhead_chars: u64, reserve_tokens: u64) -> u64 {
    trigger_threshold_tokens(
        window_chars / CHARS_PER_TOKEN,
        overhead_chars / CHARS_PER_TOKEN,
        reserve_tokens,
    )
}

/// Whether `estimated_tokens` has passed the trigger — strictly above it.
///
/// Exactly at the threshold nothing triggers yet: the trigger is the point to
/// act *before* the window fills, not a cliff at its edge.
pub fn exceeds_trigger(
    estimated_tokens: u64,
    window_tokens: u64,
    overhead_tokens: u64,
    reserve_tokens: u64,
) -> bool {
    estimated_tokens > trigger_threshold_tokens(window_tokens, overhead_tokens, reserve_tokens)
}

/// `retain_tail = floor(ratio * (window - overhead))`, in tokens.
///
/// `ratio` is clamped to `0.0..=1.0`, so the tail kept verbatim never exceeds
/// the post-overhead window. The default share is
/// [`DEFAULT_RETAIN_TAIL_RATIO`]; callers pass another ratio to tune it.
pub fn retain_tail_tokens(window_tokens: u64, overhead_tokens: u64, ratio: f64) -> u64 {
    let headroom = window_tokens.saturating_sub(overhead_tokens);
    let share = ratio.clamp(0.0, 1.0);
    (headroom as f64 * share).floor() as u64
}

/// The next injected-context budget after a window-exceeded report
/// (`current * 7 / 10`), or `None` when the budget cannot shrink any further.
///
/// Monotonic by construction: whenever a budget is returned it is strictly
/// below the current one, so repeated shrinking always terminates.
pub fn shrink_budget(current_budget_chars: u64) -> Option<u64> {
    let next =
        current_budget_chars.saturating_mul(BUDGET_SHRINK_NUMERATOR) / BUDGET_SHRINK_DENOMINATOR;
    (next < current_budget_chars).then_some(next)
}

/// Progress guard for a budget-shrink retry: take the retry only when the
/// budget strictly decreased **and** the reassembled context differs from the
/// previous assembly. Anything else means the retry would resend the request
/// that just failed, so the original error stands — recovery never retries
/// without progress, and never retries without end (the attempt cap lives with
/// [`MAX_OVERFLOW_RETRIES`]).
pub fn retry_makes_progress(
    current_budget_chars: u64,
    next_budget_chars: Option<u64>,
    assembly_changed: bool,
) -> bool {
    matches!(next_budget_chars, Some(next) if next < current_budget_chars) && assembly_changed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Small window: the reserve dominates and the headroom bound wins.
    #[test]
    fn trigger_small_window_is_bounded_by_headroom() {
        let trigger = trigger_threshold_tokens(1_000, 100, RESERVE_TOKENS);
        assert_eq!(trigger, 388, "min(800, 1000-100-512) = 388");
    }

    /// Large overhead: the trigger collapses to the remaining headroom (and to
    /// 0 when the overhead alone exceeds the window).
    #[test]
    fn trigger_large_overhead_collapses_to_headroom() {
        let trigger = trigger_threshold_tokens(100_000, 95_000, RESERVE_TOKENS);
        assert_eq!(trigger, 4_488, "min(80_000, 100_000-95_000-512)");
        assert_eq!(trigger_threshold_tokens(1_000, 2_000, 0), 0);
        assert_eq!(trigger_threshold_tokens(600, 0, 600), 0, "reserve 吃光窗口");
    }

    /// Typical window: the scaled term wins, and the char facade agrees via the
    /// `chars / 4` conversion.
    #[test]
    fn trigger_typical_window_is_the_scaled_term() {
        let trigger = trigger_threshold_tokens(128_000, 8_000, RESERVE_TOKENS);
        assert_eq!(trigger, 102_400, "min(0.8*128_000, 128_000-8_000-512)");
        assert_eq!(
            trigger_threshold_chars(128_000 * 4, 8_000 * 4, RESERVE_TOKENS),
            102_400
        );
    }

    /// Exactly at the threshold nothing triggers; one token over does.
    #[test]
    fn trigger_boundary_is_exclusive() {
        let (window, overhead) = (10_000u64, 1_000u64);
        let trigger = trigger_threshold_tokens(window, overhead, RESERVE_TOKENS);
        assert!(!exceeds_trigger(trigger, window, overhead, RESERVE_TOKENS));
        assert!(exceeds_trigger(
            trigger + 1,
            window,
            overhead,
            RESERVE_TOKENS
        ));
    }

    /// Retained tail is the configured share of the post-overhead window and
    /// never exceeds it.
    #[test]
    fn retain_tail_is_the_configured_share_within_headroom() {
        let (window, overhead) = (10_000u64, 2_500u64);
        assert_eq!(
            retain_tail_tokens(window, overhead, DEFAULT_RETAIN_TAIL_RATIO),
            1_200,
            "0.16 * (10_000 - 2_500)"
        );
        assert_eq!(retain_tail_tokens(window, overhead, 1.0), 7_500);
        assert_eq!(
            retain_tail_tokens(window, overhead, 2.0),
            7_500,
            "比例被夹到 1"
        );
        assert_eq!(retain_tail_tokens(window, overhead, 0.0), 0);
        assert_eq!(retain_tail_tokens(100, 200, 0.5), 0, "overhead 超窗时为 0");
    }

    /// Budget descent is strictly decreasing and always terminates.
    #[test]
    fn shrink_budget_descends_strictly_until_it_stops() {
        let mut budget = 24_000u64;
        let mut steps = 0;
        while let Some(next) = shrink_budget(budget) {
            assert!(next < budget, "预算必须单调下降: {budget} -> {next}");
            budget = next;
            steps += 1;
            assert!(steps < 64, "收缩必须有限步终止");
        }
        assert_eq!(shrink_budget(24_000), Some(16_800), "24000 * 0.7");
        assert_eq!(shrink_budget(1), Some(0));
        assert_eq!(shrink_budget(0), None);
    }

    /// The progress guard requires both a smaller budget and a different
    /// assembly; a no-progress retry is intercepted.
    #[test]
    fn retry_requires_budget_descent_and_new_assembly() {
        assert!(retry_makes_progress(1_000, Some(700), true));
        assert!(
            !retry_makes_progress(1_000, Some(700), false),
            "组装结果不变 = 无进展, 必须拦截"
        );
        assert!(!retry_makes_progress(1_000, Some(1_000), true), "预算不降");
        assert!(!retry_makes_progress(1_000, None, true), "预算无法再降");
    }
}
