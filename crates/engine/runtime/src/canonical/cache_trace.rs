//! Debug-only prefix-cache diagnostic brief for provider requests.
//!
//! Provider prefix caching reuses the longest byte-identical leading span of
//! consecutive requests. This module turns the two byte strings of two
//! consecutive provider requests into a one-line, **redacted** brief — how many
//! leading bytes stayed stable and where the two requests diverged — so an
//! engineer can watch the cacheable prefix grow over a session without ever
//! seeing prompt content.
//!
//! It is gated behind `APEIRETH_CACHE_TRACE=1` and writes to stderr (the
//! diagnostic log channel). It is never on in production and never records
//! message text, tool names, or model output: only byte counts and offsets.

/// Whether the prefix-cache diagnostic brief is switched on.
///
/// Reads `APEIRETH_CACHE_TRACE`. Set it to `1` (or `true` / `yes` / `on`) to
/// emit one brief per provider request. Absent or anything else keeps the hook
/// silent and costs nothing.
pub fn enabled() -> bool {
    enabled_from(std::env::var("APEIRETH_CACHE_TRACE").ok().as_deref())
}

/// Pure switch logic over a captured env value, so the gate is testable without
/// mutating process-global state.
fn enabled_from(value: Option<&str>) -> bool {
    matches!(
        value.map(str::trim),
        Some(v) if v == "1"
            || v.eq_ignore_ascii_case("true")
            || v.eq_ignore_ascii_case("yes")
            || v.eq_ignore_ascii_case("on")
    )
}

/// Length, in bytes, of the longest byte-identical leading span of `prev` and
/// `curr`: the prefix a provider cache could reuse between the two requests.
pub fn stable_prefix_bytes(prev: &[u8], curr: &[u8]) -> usize {
    prev.iter()
        .zip(curr.iter())
        .take_while(|(a, b)| a == b)
        .count()
}

/// A redacted one-line brief for one provider request.
///
/// Reports the stable prefix byte count and the changed segment as a byte
/// range. When there is no previous request the whole current request is the
/// (new) stable prefix and the changed span is empty. Content is never included.
pub fn report(prev: Option<&[u8]>, curr: &[u8]) -> String {
    match prev {
        Some(prev) => {
            let stable = stable_prefix_bytes(prev, curr);
            format!(
                "cache-trace: stable_prefix_bytes={stable} prev_bytes={} curr_bytes={} \
                 changed_segment={stable}..{} [redacted]",
                prev.len(),
                curr.len(),
                curr.len()
            )
        }
        None => format!(
            "cache-trace: stable_prefix_bytes={} prev_bytes=0 curr_bytes={} \
             changed_segment=<none> [redacted]",
            curr.len(),
            curr.len()
        ),
    }
}

/// Emit the brief for one request and remember it as the "previous" request for
/// the next comparison. Call only when [`enabled`]; `prev` is updated to `curr`
/// so consecutive calls compare consecutive provider requests.
pub fn observe(prev: &mut Option<Vec<u8>>, curr: &[u8]) {
    eprintln!("{}", report(prev.as_deref(), curr));
    *prev = Some(curr.to_vec());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⑥ The brief carries the stable-prefix segment (its byte count) and the
    /// changed span, so a reader can see how much of the request a prefix cache
    /// could reuse — and the brief never leaks content.
    #[test]
    fn report_carries_the_stable_prefix_segment() {
        let prev = b"STATIC-PREFIX|round-1-tail";
        let curr = b"STATIC-PREFIX|other-2-tail-longer";
        let out = report(Some(prev), curr);

        let stable = stable_prefix_bytes(prev, curr);
        assert_eq!(stable, "STATIC-PREFIX|".len(), "shared leading bytes");
        assert!(
            out.contains(&format!("stable_prefix_bytes={stable}")),
            "brief names the stable prefix byte count: {out}"
        );
        assert!(
            out.contains(&format!("changed_segment={stable}..{}", curr.len())),
            "brief names the changed segment range: {out}"
        );
        assert!(out.contains("[redacted]"), "brief is redacted: {out}");
        // Redaction: no request payload text appears in the brief.
        assert!(!out.contains("round-1"), "no content leaks: {out}");
        assert!(!out.contains("round-2"), "no content leaks: {out}");
    }

    /// A request with no predecessor treats its whole length as the new stable
    /// prefix and reports an empty changed span.
    #[test]
    fn first_request_reports_full_length_as_new_stable_prefix() {
        let out = report(None, b"anything at all");
        assert!(out.contains("stable_prefix_bytes=15"), "{out}");
        assert!(out.contains("changed_segment=<none>"), "{out}");
        assert!(out.contains("[redacted]"), "{out}");
    }

    /// The gate reads only the documented values.
    #[test]
    fn gate_accepts_documented_switch_values() {
        assert!(enabled_from(Some("1")));
        assert!(enabled_from(Some("true")));
        assert!(enabled_from(Some("YES")));
        assert!(enabled_from(Some(" on ")));
        assert!(!enabled_from(Some("0")));
        assert!(!enabled_from(Some("off")));
        assert!(!enabled_from(None));
    }

    /// `observe` advances the previous-request pointer so the next brief
    /// compares against this one.
    #[test]
    fn observe_advances_the_previous_request_pointer() {
        let mut prev: Option<Vec<u8>> = None;
        observe(&mut prev, b"first");
        assert_eq!(prev.as_deref(), Some(&b"first"[..]));
        observe(&mut prev, b"second");
        assert_eq!(prev.as_deref(), Some(&b"second"[..]));
    }
}
