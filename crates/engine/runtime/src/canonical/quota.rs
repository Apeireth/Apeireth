//! Session-scoped cognitive quota: the production execution surface over the
//! foundation quota ledger ([`CognitiveQuota`]).
//!
//! # What is metered
//!
//! Three dimensions are metered:
//!
//! * **tokens** — the sum of per-round usage metering (`usage.total_tokens` of
//!   every provider response), accumulated on the session's ledger;
//! * **tool steps** — one step per dispatched tool call;
//! * **cost** — every provider round's usage converted to micro-units at the
//!   round model's unit price (the frozen [`CostRateMicros::cost_micros`]
//!   arithmetic, looked up through a [`CostRateTable`]) and accumulated on the
//!   ledger's cost column. A round whose model carries no unit price converts
//!   to `None`: it is **not** counted (nothing is ever invented) and lands in
//!   the ledger's `unpriced_rounds` counter instead, so a non-zero count marks
//!   the cost total as "部分计量" (partial coverage, honestly annotated in the
//!   self-report).
//!
//! The fourth dimension of the four-dimension quota design is honestly *not*
//! wired: **depth** has no call-tree tracking. Nothing pretends otherwise.
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

use apeireth_core::model_profile::ModelProfileTable;
use apeireth_orchestration::CognitiveQuota;
use serde::{Deserialize, Serialize};

/// Which quota dimension a metering or exhaustion fact belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuotaDimension {
    /// Usage-metered tokens accumulated across provider responses.
    Tokens,
    /// Dispatched tool calls.
    ToolSteps,
    /// Cost converted from usage at the round model's unit price. Rounds
    /// without a unit price are not converted (never invented) and mark the
    /// total as partial.
    CostMicros,
}

impl QuotaDimension {
    /// The stable dimension name carried by error frames and self-reports.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tokens => "tokens",
            Self::ToolSteps => "tool_steps",
            Self::CostMicros => "cost_micros",
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

/// Largest session cost quota a configuration value is clamped to.
///
/// 家族钳制锚点与 [`MAX_QUOTA_TOKENS`] / [`MAX_QUOTA_TOOL_STEPS`] 同口径
/// (u32::MAX 微单位): 单价以微单位计, 这个量级的单会话花费上限已经不是
/// 一份真实预算而是数值噪音; 越界值钳到锚点, 不发明更高的"预算"。
pub const MAX_QUOTA_COST_MICROS: u64 = u32::MAX as u64;

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

/// Resolve the session cost quota (micro-units) from a raw configuration
/// value.
///
/// Same contract as [`parse_quota_max_tokens`]: positive integers pass
/// through (clamped to [`MAX_QUOTA_COST_MICROS`]); anything else is `None` =
/// unlimited.
pub fn parse_quota_max_cost_micros(raw: Option<&str>) -> Option<u64> {
    parse_quota_ceiling(raw, MAX_QUOTA_COST_MICROS)
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
    /// Session-wide converted-cost ceiling, in micro-units. `None` = no limit.
    pub max_cost_micros: Option<u64>,
}

impl SessionQuotaLimits {
    /// A quota with all dimensions unlimited (the default: opt-in).
    pub const fn unlimited() -> Self {
        Self {
            max_tokens: None,
            max_tool_steps: None,
            max_cost_micros: None,
        }
    }

    /// Bind the live ceilings into a session ledger.
    ///
    /// An unlimited dimension receives the ledger's unreachable ceiling
    /// ([`CognitiveQuota::unlimited`]), so `CognitiveQuota::is_exhausted`
    /// agrees with [`SessionQuotaLimits::check`] by construction. The cost
    /// ceiling binds the metered cost column: it can only ever bind on the
    /// *priced* part of the spend, because unpriced rounds are never converted
    /// into the cost column (never invented).
    pub fn bind_into(&self, meter: &mut CognitiveQuota) {
        meter.max_tokens = self
            .max_tokens
            .map(|value| value.min(MAX_QUOTA_TOKENS) as usize)
            .unwrap_or(usize::MAX);
        meter.max_tool_steps = self
            .max_tool_steps
            .map(|value| value.min(MAX_QUOTA_TOOL_STEPS) as usize)
            .unwrap_or(usize::MAX);
        meter.max_cost_micros = self
            .max_cost_micros
            .map(|value| value.min(MAX_QUOTA_COST_MICROS))
            .unwrap_or(u64::MAX);
    }

    /// Bind the live ceilings, then report the exhausted dimension.
    ///
    /// `consumed >= limit` is exhausted — the same reading as
    /// [`CognitiveQuota::is_exhausted`]. Tokens are checked first, then tool
    /// steps, then cost, so a frame always names the same dimension for the
    /// same ledger.
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
        if meter.consumed_cost_micros >= meter.max_cost_micros {
            return Some(QuotaExceeded {
                dimension: QuotaDimension::CostMicros,
                limit: meter.max_cost_micros,
                consumed: meter.consumed_cost_micros,
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
/// This is the frozen conversion the round metering point calls (see
/// [`CostRateMicros::cost_micros`]): `usage × unit price` per provider round,
/// accumulated on the session ledger's cost column. A model without a rate is
/// never converted — never invented — and its rounds are counted as unpriced
/// instead (see [`CognitiveQuota::unpriced_rounds`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostRateMicros {
    /// Cost of 1 000 input tokens, in micro-units.
    pub input_micros_per_1k_tokens: u64,
    /// Cost of 1 000 output tokens, in micro-units.
    pub output_micros_per_1k_tokens: u64,
}

impl CostRateMicros {
    /// The cost, in micro-units, of metering `input_tokens` + `output_tokens`
    /// at this rate.
    ///
    /// Rounding is per-side ceil over 1 000 tokens (保守口径: 永不低估花费),
    /// saturating at `u64::MAX`. This is the frozen arithmetic the cost
    /// metering point calls (A-P2 起回合计量处按此换算记账).
    pub fn cost_micros(&self, input_tokens: u64, output_tokens: u64) -> u64 {
        ceil_micros(input_tokens, self.input_micros_per_1k_tokens)
            .saturating_add(ceil_micros(output_tokens, self.output_micros_per_1k_tokens))
    }
}

/// `tokens × micros_per_1k / 1000`, rounded up (never undercounts spend).
fn ceil_micros(tokens: u64, micros_per_1k: u64) -> u64 {
    let product = u128::from(tokens) * u128::from(micros_per_1k);
    let rounded = (product + 999) / 1000;
    u64::try_from(rounded).unwrap_or(u64::MAX)
}

/// The unit-price table lookup: model id to unit price.
///
/// The install point for the cost dimension (see [`CostRateMicros`]): the
/// runtime's round metering point takes one of these and converts every priced
/// round onto the session ledger. `None` = that model has no complete price
/// row, so its rounds are counted unpriced (never invented). The model-profile
/// table is the production source of these rows (see the [`CostRateTable`]
/// impl for [`ModelProfileTable`]).
pub trait CostRateTable: Send + Sync + fmt::Debug {
    /// The unit price for `model`, or `None` when the table has no row (the
    /// cost dimension then cannot be metered for that model).
    fn rate_for(&self, model: &str) -> Option<CostRateMicros>;
}

/// 模型画像表 = 单价表来源 (H-P1 接头): 有价的画像行给出单价,
/// 无价/未知的行回 `None` —— 绝不编价。
///
/// A row yields a rate only when **both** the input and the output unit price
/// are provided: a half-priced row cannot express a complete rate, and the
/// missing half is never invented. `cache_read_price_micros` stays profile
/// data (the [`CostRateMicros`] shape carries no cache column).
impl CostRateTable for ModelProfileTable {
    fn rate_for(&self, model: &str) -> Option<CostRateMicros> {
        let profile = self.get(model)?;
        match (profile.input_price_micros, profile.output_price_micros) {
            (Some(input), Some(output)) => Some(CostRateMicros {
                input_micros_per_1k_tokens: input,
                output_micros_per_1k_tokens: output,
            }),
            _ => None,
        }
    }
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
    fn cost_quota_parsing_covers_the_four_family_states() {
        // ① 正整数直通 (含空白剥除)。
        assert_eq!(parse_quota_max_cost_micros(Some("5000")), Some(5_000));
        assert_eq!(parse_quota_max_cost_micros(Some("  42  ")), Some(42));
        assert_eq!(parse_quota_max_cost_micros(Some("1")), Some(1));
        // ② 越界钳制到家族锚点 (钳制, 不发明更高预算)。
        assert_eq!(
            parse_quota_max_cost_micros(Some("99999999999999")),
            Some(MAX_QUOTA_COST_MICROS),
            "above the ceiling the value clamps, it is not invented"
        );
        // ③ 0 / 负数 / 非法 = None (0 非法, 不是"零预算")。
        for raw in ["0", "-3", "abc", "2.5", "99999999999999999999999"] {
            assert_eq!(
                parse_quota_max_cost_micros(Some(raw)),
                None,
                "cost quota must read {raw:?} as unlimited (0/illegal)"
            );
        }
        // ④ 未设 / 空 = 不设限 (opt-in 默认)。
        assert_eq!(parse_quota_max_cost_micros(None), None);
        assert_eq!(parse_quota_max_cost_micros(Some("")), None);
        assert_eq!(parse_quota_max_cost_micros(Some("   ")), None);
    }

    #[test]
    fn unlimited_defaults_never_bind_a_ledger() {
        let limits = SessionQuotaLimits::default();
        assert_eq!(limits, SessionQuotaLimits::unlimited());
        assert_eq!(limits.max_tokens, None);
        assert_eq!(limits.max_tool_steps, None);
        assert_eq!(limits.max_cost_micros, None);

        let mut meter = CognitiveQuota::default();
        meter.consume_tokens(1_000_000);
        meter.consume_tool_step();
        meter.consume_cost_micros(u64::MAX / 2);
        meter.record_unpriced_round();
        assert_eq!(
            limits.check(&mut meter),
            None,
            "no ceiling is configured: nothing ever trips"
        );
        assert_eq!(meter.consumed_tokens, 1_000_000, "metering still runs");
        assert_eq!(
            meter.consumed_cost_micros,
            u64::MAX / 2,
            "花费维照计量, 只是不设限"
        );
    }

    #[test]
    fn bound_ledger_accounting_and_limits_agree_with_the_foundation_predicate() {
        let limits = SessionQuotaLimits {
            max_tokens: Some(100),
            max_tool_steps: Some(2),
            max_cost_micros: None,
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
        assert_eq!(QuotaDimension::CostMicros.as_str(), "cost_micros");
        assert_eq!(QuotaDimension::Tokens.to_string(), "tokens");
    }

    #[test]
    fn the_ask_stop_is_an_explicitly_unwired_interface_stub() {
        // 熔断档是唯一接线档; 询问档留接口桩, 显式未接线。
        assert!(QuotaExhaustedAction::CircuitBreak.is_wired());
        assert!(!QuotaExhaustedAction::AskForBudgetExtension.is_wired());
    }

    #[test]
    fn the_cost_rate_table_seam_yields_prices_only_for_priced_rows() {
        // 单价表安装点合同: trait + type 是花费维的价格来源缝 (H-P1 起画像表
        // 来源已实现, A-P2 起计量点装表记账); 无价/缺半价回 None —— 绝不编价。
        #[derive(Debug)]
        struct EmptyTable;
        impl CostRateTable for EmptyTable {
            fn rate_for(&self, _model: &str) -> Option<CostRateMicros> {
                None
            }
        }
        #[derive(Debug)]
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

        // A-P2 起成本维上限随配置真绑定 (不再恒不可达)。
        let mut meter = CognitiveQuota::default();
        SessionQuotaLimits {
            max_tokens: None,
            max_tool_steps: None,
            max_cost_micros: Some(5_000),
        }
        .bind_into(&mut meter);
        assert_eq!(meter.max_cost_micros, 5_000);
    }

    #[test]
    fn cost_rates_come_from_the_profile_table() {
        // 画像表 = 单价表来源: 有价算成本, 无价 None (缺一半也不编另一半)。
        let table = ModelProfileTable::from_json_value(serde_json::json!([
            {"model_id": "priced", "input_price_micros": 2_000, "output_price_micros": 8_000},
            {"model_id": "half-priced", "input_price_micros": 2_000},
            {"model_id": "unpriced"}
        ]))
        .expect("profile table");
        let rate = table.rate_for("priced").expect("有价行必须给出单价");
        assert_eq!(
            rate,
            CostRateMicros {
                input_micros_per_1k_tokens: 2_000,
                output_micros_per_1k_tokens: 8_000,
            }
        );
        assert_eq!(
            rate.cost_micros(1_500, 500),
            7_000,
            "1.5k 输入 × 2 + 0.5k 输出 × 8"
        );
        assert_eq!(rate.cost_micros(1, 0), 2, "取整: 每侧向上取整, 永不低估");
        assert_eq!(rate.cost_micros(0, 0), 0);

        assert_eq!(table.rate_for("half-priced"), None, "缺输出价 = 不编价");
        assert_eq!(table.rate_for("unpriced"), None, "无价行 = None");
        assert_eq!(table.rate_for("unknown-model"), None, "未知模型无价");
    }

    #[test]
    fn a_priced_round_lands_in_the_cost_column_and_the_ceiling_binds_on_real_numbers() {
        // 花费维接线 (A-P2) 演进自「显式未接线」钉死 (演进理由: 计量点已装表
        // 记账): 有价轮累计真数字, 上限按 `consumed >= limit` 出超限事实,
        // 与 tokens/steps 完全同语义 (口径唯一)。
        let rate = CostRateMicros {
            input_micros_per_1k_tokens: 2_000,
            output_micros_per_1k_tokens: 8_000,
        };
        let limits = SessionQuotaLimits {
            max_tokens: None,
            max_tool_steps: None,
            max_cost_micros: Some(7_000),
        };
        let mut meter = CognitiveQuota::default();
        // 有价轮换算 (每侧向上取整): 1.5k 输入 × 2 + 0.5k 输出 × 8 = 7000。
        meter.consume_cost_micros(rate.cost_micros(1_500, 500));
        assert_eq!(meter.consumed_cost_micros, 7_000);
        let exceeded = limits.check(&mut meter).expect("花费维到顶即出超限事实");
        assert_eq!(
            exceeded,
            QuotaExceeded {
                dimension: QuotaDimension::CostMicros,
                limit: 7_000,
                consumed: 7_000,
            }
        );
        assert!(meter.is_exhausted(), "口径唯一: same reading");

        // 预算内不绑: 同一轮口径下的下半段。
        let mut within = CognitiveQuota::default();
        within.consume_cost_micros(rate.cost_micros(1_500, 499));
        assert_eq!(within.consumed_cost_micros, 6_992);
        assert_eq!(limits.check(&mut within), None);
        assert!(!within.is_exhausted(), "口径唯一: same reading");
    }

    #[test]
    fn unpriced_rounds_never_enter_the_cost_column_and_mark_the_total_partial() {
        // 绝不编价: 无价轮 cost=None 不计入成本列, 只计轮数 (部分计量标注的
        // 数据来源); 也不会伪造超限事实。
        let limits = SessionQuotaLimits {
            max_tokens: None,
            max_tool_steps: None,
            max_cost_micros: Some(1),
        };
        let mut meter = CognitiveQuota::default();
        meter.record_unpriced_round();
        meter.record_unpriced_round();
        meter.consume_cost_micros(2);
        meter.record_unpriced_round();
        assert_eq!(meter.consumed_cost_micros, 2, "无价轮不计入 (绝不编价)");
        assert_eq!(meter.unpriced_rounds, 3, "无价轮数如实计数 (部分计量)");
        let exceeded = limits.check(&mut meter).expect("有价花费绑上限");
        assert_eq!(exceeded.dimension, QuotaDimension::CostMicros);
        assert_eq!(exceeded.consumed, 2, "已耗只有有价部分的真数字");

        // 只有无价轮的会话: 0 花费, 不伪造超限。
        let mut unpriced_only = CognitiveQuota::default();
        unpriced_only.record_unpriced_round();
        assert_eq!(limits.check(&mut unpriced_only), None);
    }

    #[test]
    fn the_three_metered_dimensions_accumulate_and_bind_independently() {
        // 互不干扰: 各维只累计自己的列, 超限事实只出自己的维度名。
        let mut meter = CognitiveQuota::default();
        meter.consume_tokens(10);
        meter.consume_cost_micros(7_000);
        meter.consume_tool_step();
        meter.consume_cost_micros(2);
        meter.record_unpriced_round();
        assert_eq!(meter.consumed_tokens, 10);
        assert_eq!(meter.consumed_tool_steps, 1);
        assert_eq!(meter.consumed_cost_micros, 7_002);
        assert_eq!(meter.unpriced_rounds, 1);

        let cost_only = SessionQuotaLimits {
            max_tokens: None,
            max_tool_steps: None,
            max_cost_micros: Some(1),
        };
        assert_eq!(
            cost_only.check(&mut meter).map(|fact| fact.dimension),
            Some(QuotaDimension::CostMicros),
            "只有花费维绑"
        );

        let mut tokens_only = CognitiveQuota::default();
        tokens_only.consume_tokens(5);
        tokens_only.consume_cost_micros(7_002);
        let token_limits = SessionQuotaLimits {
            max_tokens: Some(5),
            max_tool_steps: None,
            max_cost_micros: None,
        };
        assert_eq!(
            token_limits
                .check(&mut tokens_only)
                .map(|fact| fact.dimension),
            Some(QuotaDimension::Tokens),
            "花费累计不牵连 token 维"
        );
    }
}
