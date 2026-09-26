//! Token metering anchor: one deterministic fold behind every metering view.
//!
//! Three metering needs — how much context a transcript occupies
//! (上下文占用), how much a compression pass saved (压缩节省), and what the
//! telemetry counter should report (遥测计数) — must not each compute their own
//! token number. [`TokenMeter`] meters all three through the same
//! deterministic fold, so the three readings share one source and cannot
//! drift apart.
//!
//! The estimate stays `chars / 4` ([`fold_text`]). Its known bias is documented
//! rather than patched: scripts that pack roughly one token per character
//! (CJK-heavy text) are systematically **under-counted** by this fold. A
//! per-script correction factor would trade a known bias for invented
//! precision. When real provider usage exists alongside the estimate,
//! [`UsageAnchor`] decides — by a strict calibration rule — when the
//! measurement may stand in for the estimate, and when a full re-estimate is
//! required instead.
//!
//! The single fold also backs [`crate::context_fold::approx_tokens`], so the
//! frozen compaction surfaces count through this module without re-implementing
//! the formula.

/// Characters per estimated token in the deterministic fold (`chars / 4`).
pub const CHARS_PER_TOKEN: u64 = 4;

/// The one deterministic estimation fold: `chars / 4`.
///
/// Known bias (deliberately not "corrected"): CJK-heavy text is under-counted;
/// see the module docs. Every metering view goes through this function so the
/// bias is at least identical everywhere.
pub fn fold_chars(chars: u64) -> u64 {
    chars / CHARS_PER_TOKEN
}

/// [`fold_chars`] over a text's Unicode scalar values.
pub fn fold_text(text: &str) -> u64 {
    fold_chars(text.chars().count() as u64)
}

/// One metering reading. All three views are computed from the same fold over
/// the same passes, so they are consistent by construction:
/// `telemetry == context_occupancy + compression_saved`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MeterReading {
    /// Context occupancy (上下文占用): estimated tokens of retained content.
    pub context_occupancy_tokens: u64,
    /// Compression savings (压缩节省): estimated tokens of removed content.
    pub compression_saved_tokens: u64,
    /// Telemetry count (遥测计数): everything metered through the fold —
    /// occupancy plus savings, always adding up.
    pub telemetry_tokens: u64,
}

/// Unified metering skeleton: meter a retention pass (or a replayed transcript
/// segment) once, then read all three views from the same deterministic fold.
///
/// Builder-style: each `meter_*` call accumulates and returns `self`.
#[derive(Debug, Clone, Default)]
pub struct TokenMeter {
    occupancy_tokens: u64,
    saved_tokens: u64,
}

impl TokenMeter {
    /// An empty meter (nothing metered yet).
    pub fn new() -> Self {
        Self::default()
    }

    /// Meter one retention pass: `retained` stays in the context, `removed` was
    /// cut away. Both sides are counted by the one fold, and each text is
    /// folded as its own unit (the fold is per piece, floor included).
    #[must_use]
    pub fn meter_pass(mut self, retained: &str, removed: &str) -> Self {
        self.occupancy_tokens = self.occupancy_tokens.saturating_add(fold_text(retained));
        self.saved_tokens = self.saved_tokens.saturating_add(fold_text(removed));
        self
    }

    /// Meter a piece that is retained with nothing removed in this pass.
    #[must_use]
    pub fn meter_retained(self, retained: &str) -> Self {
        self.meter_pass(retained, "")
    }

    /// The three metering views for everything metered so far.
    pub fn reading(&self) -> MeterReading {
        MeterReading {
            context_occupancy_tokens: self.occupancy_tokens,
            compression_saved_tokens: self.saved_tokens,
            telemetry_tokens: self.occupancy_tokens.saturating_add(self.saved_tokens),
        }
    }
}

/// Identity of one metered payload: a deterministic fingerprint over the exact
/// content metered.
///
/// A measurement may stand in for an estimate only for the *same* payload —
/// two envelopes that differ by one byte are different payloads, and a
/// measurement of one never counts for the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeteredEnvelope {
    fingerprint: u64,
    bytes: u64,
}

impl MeteredEnvelope {
    /// Total bytes of the metered payload.
    pub fn len_bytes(&self) -> u64 {
        self.bytes
    }

    /// Whether the metered payload is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes == 0
    }
}

/// Fingerprint the exact texts a reading covers, in order. Parts are
/// length-delimited into the fold, so `["ab", "c"]` and `["a", "bc"]` are
/// different envelopes even though their concatenation matches.
pub fn envelope_of(parts: &[&str]) -> MeteredEnvelope {
    // 64-bit multiply-xor fold over the bytes, with a separator byte between
    // parts. Deterministic and dependency-free; identity checking only — not a
    // security primitive.
    let mut fingerprint: u64 = 0xcbf2_9ce4_8422_2325;
    let mut bytes = 0u64;
    for part in parts {
        for byte in part.as_bytes() {
            fingerprint ^= u64::from(*byte);
            fingerprint = fingerprint.wrapping_mul(0x0000_0100_0000_01b3);
        }
        fingerprint ^= 0xff;
        fingerprint = fingerprint.wrapping_mul(0x0000_0100_0000_01b3);
        bytes = bytes.saturating_add(part.len() as u64);
    }
    MeteredEnvelope { fingerprint, bytes }
}

/// Calibration verdict for one real provider measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Calibration {
    /// The measurement applies: same envelope, and the measured total at least
    /// covers the anchor. Reuse the measured total as the metered count.
    ReuseMeasured { measured_total: u64 },
    /// The measurement cannot stand in: either a different envelope, or a
    /// total below the anchor (which cannot cover what the anchor already
    /// accounts for). Re-estimate the payload in full — never splice a foreign
    /// or partial measurement into an existing count.
    Reestimate,
}

/// The best token count known for one exact payload: the deterministic fold
/// estimate until a real measurement passes calibration and replaces it.
///
/// Coexistence rule (anti-fake-precision, symmetric to the omission grading):
/// an estimate and a real measurement may both exist for the same turn. The
/// anchor keeps them from being averaged, spliced, or silently mixed — one of
/// them wins outright for one exact envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsageAnchor {
    envelope: MeteredEnvelope,
    anchor_tokens: u64,
}

impl UsageAnchor {
    /// Seed the anchor for `envelope` from the deterministic fold estimate.
    pub fn from_estimate(envelope: MeteredEnvelope, estimated_tokens: u64) -> Self {
        Self {
            envelope,
            anchor_tokens: estimated_tokens,
        }
    }

    /// The payload the anchor applies to.
    pub fn envelope(&self) -> MeteredEnvelope {
        self.envelope
    }

    /// The anchor token count (estimate until a measurement replaces it).
    pub fn tokens(&self) -> u64 {
        self.anchor_tokens
    }

    /// The calibration rule: adopt the measurement only when it is for the
    /// same envelope **and** its total at least covers the anchor; anything
    /// else triggers a full re-estimate.
    pub fn calibrate(&self, envelope: MeteredEnvelope, measured_total: u64) -> Calibration {
        if envelope == self.envelope && measured_total >= self.anchor_tokens {
            Calibration::ReuseMeasured { measured_total }
        } else {
            Calibration::Reestimate
        }
    }

    /// Full calibration step: the effective token count for the payload — the
    /// measured total when adopted, otherwise `reestimated_tokens` from a full
    /// re-run of the fold over the whole payload (the caller measures
    /// everything again; nothing is mixed in from the rejected measurement).
    pub fn resolve(
        &self,
        envelope: MeteredEnvelope,
        measured_total: u64,
        reestimated_tokens: u64,
    ) -> u64 {
        match self.calibrate(envelope, measured_total) {
            Calibration::ReuseMeasured { measured_total } => measured_total,
            Calibration::Reestimate => reestimated_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three metering views come from one fold over the same passes and
    /// always add up: occupancy + savings == telemetry.
    #[test]
    fn three_views_share_one_fold() {
        let reading = TokenMeter::new()
            .meter_pass("retained content", "removed content")
            .meter_pass("more retained", "")
            .meter_retained("tail piece")
            .reading();

        let expected_occupancy =
            fold_text("retained content") + fold_text("more retained") + fold_text("tail piece");
        let expected_saved = fold_text("removed content");
        assert_eq!(reading.context_occupancy_tokens, expected_occupancy);
        assert_eq!(reading.compression_saved_tokens, expected_saved);
        assert_eq!(
            reading.telemetry_tokens,
            reading.context_occupancy_tokens + reading.compression_saved_tokens,
            "三口径同源: telemetry 必为两口径之和"
        );

        // The same fold feeds every view: each side equals its own fold value,
        // not a separately-derived number.
        let empty = TokenMeter::new().reading();
        assert_eq!(
            empty,
            MeterReading {
                context_occupancy_tokens: 0,
                compression_saved_tokens: 0,
                telemetry_tokens: 0,
            }
        );
    }

    /// Anchor adoption: same envelope and a measured total that at least
    /// covers the anchor → the measurement is reused as-is.
    #[test]
    fn anchor_adopted_on_same_envelope_and_covering_total() {
        let envelope = envelope_of(&["prompt body", "tool declarations"]);
        let anchor = UsageAnchor::from_estimate(envelope, 120);

        // Exactly covering the anchor: adopted.
        assert_eq!(
            anchor.calibrate(envelope, 120),
            Calibration::ReuseMeasured {
                measured_total: 120
            }
        );
        // More than covering: adopted at the measured total (never clamped to
        // the anchor).
        assert_eq!(
            anchor.calibrate(envelope, 300),
            Calibration::ReuseMeasured {
                measured_total: 300
            }
        );
        assert_eq!(anchor.resolve(envelope, 300, 120), 300);
    }

    /// Anchor rejection: a different envelope, or a total below the anchor —
    /// either way the measurement must not stand in.
    #[test]
    fn anchor_rejected_on_foreign_envelope_or_under_covering_total() {
        let envelope = envelope_of(&["prompt body"]);
        let anchor = UsageAnchor::from_estimate(envelope, 120);

        let other = envelope_of(&["prompt body changed"]);
        assert_eq!(anchor.calibrate(other, 500), Calibration::Reestimate);
        assert_eq!(
            anchor.calibrate(envelope, 119),
            Calibration::Reestimate,
            "实测低于锚点 = 无法覆盖已记账量, 拒绝"
        );
    }

    /// A rejection triggers the full re-estimate path: `resolve` returns the
    /// caller's full re-measured total, never a splice of the rejected number.
    #[test]
    fn rejection_triggers_full_reestimate() {
        let envelope = envelope_of(&["segment a", "segment b"]);
        let anchor = UsageAnchor::from_estimate(envelope, 80);

        // Foreign measurement: full re-measured value wins.
        assert_eq!(anchor.resolve(envelope_of(&["other"]), 999, 64), 64);
        // Under-covering measurement: same.
        assert_eq!(anchor.resolve(envelope, 10, 64), 64);
        // Adoption path unchanged.
        assert_eq!(anchor.resolve(envelope, 90, 64), 90);

        // The anchor itself stays on its envelope and estimate until replaced.
        assert_eq!(anchor.envelope(), envelope);
        assert_eq!(anchor.tokens(), 80);
    }

    /// The fold stays exactly `chars / 4` at every boundary — including the
    /// documented CJK under-count, which is pinned rather than papered over.
    #[test]
    fn fold_stays_chars_over_four_at_boundaries() {
        assert_eq!(fold_text(""), 0);
        assert_eq!(fold_text("abc"), 0, "3 chars / 4 = 0");
        assert_eq!(fold_text("abcd"), 1, "4 chars / 4 = 1");
        assert_eq!(fold_text("abcde"), 1, "floor, not round");
        assert_eq!(fold_chars(0), 0);
        assert_eq!(fold_chars(7), 1);
        assert_eq!(fold_chars(8), 2);

        // Known bias pinned: 4 CJK characters are 1 token by this estimate
        // even though real tokenization counts far more. No fake correction.
        assert_eq!(fold_text("你好世界"), 1);
        assert_eq!(fold_text("你好"), 0);

        // One fold everywhere: the shared entry used by existing call sites
        // returns the same numbers (behaviour unchanged after rewiring).
        let mut samples: Vec<String> = vec![
            String::new(),
            "abcd".to_string(),
            "你好世界".to_string(),
            "aé語🦀b".to_string(),
        ];
        samples.push("x".repeat(1000));
        for sample in &samples {
            assert_eq!(
                crate::context_fold::approx_tokens(sample) as u64,
                fold_text(sample),
                "single fold behind every entry: {sample:?}"
            );
        }
    }

    /// Envelope identity is over the exact parts: different splits of the same
    /// bytes are different payloads, identical parts are the same payload.
    #[test]
    fn envelope_distinguishes_split_boundaries() {
        let a = envelope_of(&["ab", "c"]);
        let b = envelope_of(&["a", "bc"]);
        assert_ne!(a, b, "分段边界不同 = 不同 payload");

        let same = envelope_of(&["ab", "c"]);
        assert_eq!(a, same, "同样分段 = 同一 payload");

        let empty = envelope_of(&[]);
        assert!(empty.is_empty());
        assert_eq!(empty.len_bytes(), 0);
        assert_ne!(empty, envelope_of(&[""]), "空段也是内容差异");

        let bytes = envelope_of(&["héllo"]);
        assert_eq!(bytes.len_bytes(), 6, "字节长度按 UTF-8 计 (é 是 2 字节)");
    }
}
