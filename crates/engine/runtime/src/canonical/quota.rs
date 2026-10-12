//! Session-scoped cognitive quota: the production execution surface over the
//! foundation quota ledger ([`CognitiveQuota`]).
//!
//! # What is metered
//!
//! Two dimensions are metered this batch:
//!
//! * **tokens** — the sum of per-round usage metering (`usage.total_tokens` of
//!   every provider response), accumulated on the session's ledger;
//! * **tool steps** — one step per dispatched tool call.
//!
//! The other two dimensions of the four-dimension quota design are honestly
//! *not* wired: **cost** has no unit-price table (only the type + trait stub
//! below — installing a table is what would enable it) and **depth** has no
//! call-tree tracking. Nothing pretends otherwise.
//!
//! # Opt-in limits
//!
//! Quotas are opt-in: [`SessionQuotaLimits`] carries `Option` ceilings and
//! `None` means *no limit* (the ledger then receives an unreachable ceiling).
//! The metering itself always runs, so the self-report's "consumed" column is
//! real even when nothing is capped.
//!
//! # Exhausted behavior
//!
//! When a dimension reaches its ceiling (`consumed >= limit`, the same reading
//! as [`CognitiveQuota::is_exhausted`]) the wired behavior is the circuit-break
//! stop ([`QuotaExhaustedAction::CircuitBreak`]): the turn closes at the next
//! checkpoint with a budget-exhausted error frame carrying the dimension name,
//! the limit, and the consumed count. The ask-for-more-budget stop is an
//! interface stub, explicitly not wired (see [`QuotaExhaustedAction`]).

use std::fmt;

use apeireth_orchestration::CognitiveQuota;
use serde::{Deserialize, Serialize};

/// Which quota dimension a metering or exhaustion fact belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuotaDimension {
    /// Usage-metered tokens accumulated across provider responses.
    Tokens,
    /// Dispatched tool calls.
    ToolSteps,
}

impl QuotaDimension {
    /// The stable dimension name carried by error frames and self-reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tokens => "tokens",
            Self::ToolSteps => "tool_steps",
        }
    }
}

impl fmt::Display for QuotaDimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Largest session token quota a configuration value is clamped to.
///
/// Usage metering arrives in u32 columns (`NormalizedUsage`), so a per-session
/// ceiling above the u32 range can never bind; values above the ceiling clamp
/// to it rather than being invented.
pub const MAX_QUOTA_TOKENS: u64 = u32::MAX as u64;

/// Largest session tool-step quota a configuration value is clamped to.
/// Same rationale as [`MAX_QUOTA_TOKENS`]: past this a step ceiling is not a
/// budget any more, and the metering column could not express it either.
pub const MAX_QUOTA_TOOL_STEPS: u64 = u32::MAX as u64;

/// Resolve the session token quota from a raw configuration value.
///
/// Quota knobs are opt-in, so the fallback is *no limit*: an absent, empty,
/// zero, negative, or unparseable value maps to `None` (= unlimited). A
/// positive integer passes through, clamped into `1..=MAX_QUOTA_TOKENS`.
pub fn parse_quota_max_tokens(raw: Option<&str>) -> Option<u64> {
    parse_quota_ceiling(raw, MAX_QUOTA_TOKENS)
}

/// Resolve the session tool-step quota from a raw configuration value.
///
/// Same contract as [`parse_quota_max_tokens`]: positive integers pass
/// through (clamped to [`MAX_QUOTA_TOOL_STEPS`]); anything else is `None` =
/// unlimited.
pub fn parse_quota_max_tool_steps(raw: Option<&str>) -> Option<u64> {
    parse_quota_ceiling(raw, MAX_QUOTA_TOOL_STEPS)
}

fn parse_quota_ceiling(raw: Option<&str>, ceiling: u64) -> Option<u64> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<i64>().ok())
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .map(|value| value.min(ceiling))
}

/// The session quota ceilings in force. `None` = that dimension is unlimited.
///
/// This is the configuration shape (`RuntimeConfig::quota`); the accounting
/// itself lives on the session's [`CognitiveQuota`] ledger so consumption
/// survives restarts with the session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionQuotaLimits {
    /// Session-wide token ceiling. `None` = no limit.
    pub max_tokens: Option<u64>,
    /// Session-wide dispatched-tool-call ceiling. `None` = no limit.
    pub max_tool_steps: Option<u64>,
}

impl SessionQuotaLimits {
    /// A quota with both dimensions unlimited (the default: opt-in).
    pub const fn unlimited() -> Self {
        Self {
            max_tokens: None,
            max_tool_steps: None,
        }
    }

    /// Bind the live ceilings into a session ledger.
    ///
    /// An unlimited dimension receives the ledger's unreachable ceiling
    /// ([`CognitiveQuota::unlimited`]), so `CognitiveQuota::is_exhausted`
    /// agrees with [`SessionQuotaLimits::check`] by construction. The cost
    /// dimension is not metered this batch (no unit-price table), so its
    /// ceiling stays unreachable too.
    pub fn bind_into(&self, meter: &mut CognitiveQuota) {
        meter.max_tokens = self
            .max_tokens
            .map(|value| value.min(MAX_QUOTA_TOKENS) as usize)
            .unwrap_or(usize::MAX);
        meter.max_tool_steps = self
            .max_tool_steps
            .map(|value| value.min(MAX_QUOTA_TOOL_STEPS) as usize)
            .unwrap_or(usize::MAX);
        meter.max_cost_micros = u64::MAX;
    }

    /// Bind the live ceilings, then report the exhausted dimension.
    ///
    /// `consumed >= limit` is exhausted — the same reading as
    /// [`CognitiveQuota::is_exhausted`]. Tokens are checked first, then tool
    /// steps, so a frame always names the same dimension for the same ledger.
    pub fn check(&self, meter: &mut CognitiveQuota) -> Option<QuotaExceeded> {
        self.bind_into(meter);
        if meter.consumed_tokens >= meter.max_tokens {
            return Some(QuotaExceeded {
                dimension: QuotaDimension::Tokens,
                limit: meter.max_tokens as u64,
                consumed: meter.consumed_tokens as u64,
            });
        }
        if meter.consumed_tool_steps >= meter.max_tool_steps {
            return Some(QuotaExceeded {
                dimension: QuotaDimension::ToolSteps,
                limit: meter.max_tool_steps as u64,
                consumed: meter.consumed_tool_steps as u64,
            });
        }
        None
    }
}

/// The exhausted-dimension fact a budget-exhausted frame reports: the
/// dimension name, the limit in force, and the consumed count (real numbers,
/// never placeholders).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotaExceeded {
    /// Which dimension ran out.
    pub dimension: QuotaDimension,
    /// The ceiling that bound (the effective limit).
    pub limit: u64,
    /// What had been consumed when it bound.
    pub consumed: u64,
}

/// What happens when a session quota dimension is exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaExhaustedAction {
    /// 熔断档 — the wired behavior: the turn aborts at the next checkpoint
    /// with a budget-exhausted error frame (dimension / limit / consumed).
    CircuitBreak,
    /// 询问档 — interface stub, **explicitly not wired**: the intended
    /// semantics is "pause the turn and ask a human for a budget extension,
    /// then continue". No production path selects this variant and no code
    /// executes it; it exists so the second stop has a named home when the
    /// human-in-the-loop extension lands. Reporting says so too
    /// ([`QuotaExhaustedAction::is_wired`]).
    AskForBudgetExtension,
}

impl QuotaExhaustedAction {
    /// Whether this stop has a production implementation. Only the circuit
    /// break is wired; the ask stop is an interface stub.
    pub fn is_wired(self) -> bool {
        matches!(self, Self::CircuitBreak)
    }
}

/// One model's unit price: micro-units of cost per 1 000 tokens.
///
/// Interface stub, **explicitly not wired**: this batch has no unit-price
/// table, so the cost dimension of the quota is honestly un-metered (the
/// self-report says so). Installing a [`CostRateTable`] implementation and
/// recording `usage × price` at the metering points is what would enable it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostRateMicros {
    /// Cost of 1 000 input tokens, in micro-units.
    pub input_micros_per_1k_tokens: u64,
    /// Cost of 1 000 output tokens, in micro-units.
    pub output_micros_per_1k_tokens: u64,
}

/// The unit-price table lookup: model id to unit price.
///
/// Interface stub, **explicitly not wired** (see [`CostRateMicros`]). The
/// production metering points accept no price source yet; wiring one is the
/// documented path to enabling the cost dimension.
pub trait CostRateTable: Send + Sync {
    /// The unit price for `model`, or `None` when the table has no row (the
    /// cost dimension then cannot be metered for that model).
    fn rate_for(&self, model: &str) -> Option<CostRateMicros>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_parsing_passes_positive_integers_through_for_both_dimensions() {
        assert_eq!(parse_quota_max_tokens(Some("5000")), Some(5000));
        assert_eq!(parse_quota_max_tokens(Some("  42  ")), Some(42));
        assert_eq!(parse_quota_max_tokens(Some("1")), Some(1));
        assert_eq!(parse_quota_max_tool_steps(Some("250")), Some(250));
        assert_eq!(parse_quota_max_tool_steps(Some("  7  ")), Some(7));
        assert_eq!(parse_quota_max_tool_steps(Some("1")), Some(1));
    }

    #[test]
    fn quota_parsing_falls_back_to_unlimited_for_absent_zero_and_invalid_values() {
        // Opt-in semantics: the default is explicitly "no limit" (`None`).
        for raw in [
            None,
            Some(""),
            Some("   "),
            Some("0"),
            Some("-3"),
            Some("abc"),
            Some("2.5"),
            Some("99999999999999999999999"),
        ] {
            assert_eq!(
                parse_quota_max_tokens(raw),
                None,
                "token quota must read {raw:?} as unlimited"
            );
            assert_eq!(
                parse_quota_max_tool_steps(raw),
                None,
                "tool-step quota must read {raw:?} as unlimited"
            );
        }
    }

    #[test]
    fn quota_parsing_clamps_oversized_values_to_the_metering_ceiling() {
        assert_eq!(
            parse_quota_max_tokens(Some("99999999999999")),
            Some(MAX_QUOTA_TOKENS),
            "above the ceiling the value clamps, it is not invented"
        );
        assert_eq!(
            parse_quota_max_tool_steps(Some("99999999999999")),
            Some(MAX_QUOTA_TOOL_STEPS)
        );
    }

    #[test]
    fn unlimited_defaults_never_bind_a_ledger() {
        let limits = SessionQuotaLimits::default();
        assert_eq!(limits, SessionQuotaLimits::unlimited());
        assert_eq!(limits.max_tokens, None);
        assert_eq!(limits.max_tool_steps, None);

        let mut meter = CognitiveQuota::default();
        meter.consume_tokens(1_000_000);
        meter.consume_tool_step();
        assert_eq!(
            limits.check(&mut meter),
            None,
            "no ceiling is configured: nothing ever trips"
        );
        assert_eq!(meter.consumed_tokens, 1_000_000, "metering still runs");
    }

    #[test]
    fn bound_ledger_accounting_and_limits_agree_with_the_foundation_predicate() {
        let limits = SessionQuotaLimits {
            max_tokens: Some(100),
            max_tool_steps: Some(2),
        };
        let mut meter = CognitiveQuota::default();
        meter.consume_tokens(60);
        meter.consume_tool_step();
        assert_eq!(limits.check(&mut meter), None, "within budget");
        assert!(!meter.is_exhausted(), "口径唯一: same reading");

        meter.consume_tokens(40);
        assert_eq!(
            limits.check(&mut meter),
            Some(QuotaExceeded {
                dimension: QuotaDimension::Tokens,
                limit: 100,
                consumed: 100,
            })
        );
        assert!(meter.is_exhausted(), "口径唯一: same reading");

        let mut steps = CognitiveQuota::default();
        steps.consume_tool_step();
        steps.consume_tool_step();
        let exceeded = limits.check(&mut steps).expect("tool steps exhausted");
        assert_eq!(exceeded.dimension, QuotaDimension::ToolSteps);
        assert_eq!(exceeded.limit, 2);
        assert_eq!(exceeded.consumed, 2);
    }

    #[test]
    fn the_dimension_names_stay_stable_for_frames_and_reports() {
        assert_eq!(QuotaDimension::Tokens.as_str(), "tokens");
        assert_eq!(QuotaDimension::ToolSteps.as_str(), "tool_steps");
        assert_eq!(QuotaDimension::Tokens.to_string(), "tokens");
    }

    #[test]
    fn the_ask_stop_is_an_explicitly_unwired_interface_stub() {
        // 熔断档是唯一接线档; 询问档留接口桩, 显式未接线。
        assert!(QuotaExhaustedAction::CircuitBreak.is_wired());
        assert!(!QuotaExhaustedAction::AskForBudgetExtension.is_wired());
    }

    #[test]
    fn the_cost_dimension_is_an_explicitly_unwired_price_table_skeleton() {
        // 单价表骨架: type + trait 占位, 接单价表即启用。
        struct EmptyTable;
        impl CostRateTable for EmptyTable {
            fn rate_for(&self, _model: &str) -> Option<CostRateMicros> {
                None
            }
        }
        struct OneRowTable(CostRateMicros);
        impl CostRateTable for OneRowTable {
            fn rate_for(&self, model: &str) -> Option<CostRateMicros> {
                (model == "model-a").then_some(self.0)
            }
        }

        let rate = CostRateMicros {
            input_micros_per_1k_tokens: 2_000,
            output_micros_per_1k_tokens: 8_000,
        };
        let table: Box<dyn CostRateTable> = Box::new(OneRowTable(rate));
        assert_eq!(table.rate_for("model-a"), Some(rate));
        assert_eq!(table.rate_for("model-b"), None, "no row, no metering");
        assert_eq!(EmptyTable.rate_for("model-a"), None);

        // 未接线是显式事实: 账本的成本维上限恒不可达 (不计量)。
        let mut meter = CognitiveQuota::default();
        SessionQuotaLimits::default().bind_into(&mut meter);
        assert_eq!(meter.max_cost_micros, u64::MAX);
    }
}
