//! Usage accounting: per-call token/cost/latency records and aggregate queries.
//!
//! Recovered from the legacy `apeireth-pipeline::provider_registry` `UsageRecord`
//! + `CostTracker` semantics (field-level translation of LiteLLM's public
//! `Usage` / `CostBreakdown` / `completion_cost` aggregate-query surface).
//! The legacy provider *registry* / *fallback chain* / *selection strategies*
//! stay with the canonical ProviderRouter (`crates/engine/provider`) — only the
//! accounting vocabulary lives here, as pure serializable types.
//!
//! Design:
//! - [`ModelPricing`] — per-1k-token USD rates + cost estimation (the legacy
//!   `ProviderSpec::estimate_cost` algorithm, decoupled from any registry).
//! - [`UsageRecord`] — one record per LLM call: timestamp, provider, model,
//!   tokens, cost, latency, success. The cost field is authored by the caller
//!   (usually via [`ModelPricing::estimate_cost`]); this crate never invents
//!   prices.
//! - [`CostTracker`] — append-only record store with aggregate queries
//!   (total/per-provider/per-model cost, success rate, latency stats, token
//!   totals). No persistence, no I/O: ownership of a tracker is up to the
//!   caller (a turn, a session, a scheduler). Nothing here wires itself into
//!   the runtime.
//!
//! This module is pure types + arithmetic: serde for the record contract,
//! no dependencies beyond the crate's existing ones.

use serde::{Deserialize, Serialize};

use crate::normalized::NormalizedResponse;

/// Per-1k-token pricing for one model family (USD).
///
/// Legacy source: `apeireth-pipeline::provider_registry::ProviderSpec`
/// `cost_per_1k_input_tokens` / `cost_per_1k_output_tokens` and its
/// `estimate_cost` formula: `(tokens / 1000) * rate`, summed over directions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModelPricing {
    /// USD cost per 1000 input (prompt) tokens.
    pub input_per_1k_usd: f64,
    /// USD cost per 1000 output (completion) tokens.
    pub output_per_1k_usd: f64,
}

impl ModelPricing {
    /// Create a pricing entry.
    pub const fn new(input_per_1k_usd: f64, output_per_1k_usd: f64) -> Self {
        Self {
            input_per_1k_usd,
            output_per_1k_usd,
        }
    }

    /// Estimate the USD cost of one call from token counts.
    ///
    /// Legacy 1:1: `(input / 1000) * input_rate + (output / 1000) * output_rate`.
    /// Rates of `0.0` (unknown pricing) legitimately produce `0.0` — callers
    /// must not fabricate prices they do not have.
    pub fn estimate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_per_1k_usd;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_per_1k_usd;
        input_cost + output_cost
    }
}

impl Default for ModelPricing {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

/// One LLM call's complete cost + performance record (8 fields).
///
/// Legacy 1:1: `apeireth-pipeline::provider_registry::UsageRecord`
/// (LiteLLM public `Usage` + `CostBreakdown` field translation). `success =
/// false` records typically carry `cost_usd = 0.0`; the flag exists for
/// success-rate aggregation, not for cost suppression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRecord {
    /// Call start timestamp (millis since `UNIX_EPOCH`).
    pub timestamp_ms: u64,
    /// Provider name (e.g. `"minimax"`, `"anthropic"`).
    pub provider: String,
    /// Model actually called (e.g. `"MiniMax-M3"`).
    pub model: String,
    /// Input (prompt) token count.
    pub input_tokens: u64,
    /// Output (completion) token count.
    pub output_tokens: u64,
    /// Actual cost in USD (computed by the caller, e.g. `ModelPricing`).
    pub cost_usd: f64,
    /// Call duration in milliseconds.
    pub latency_ms: u64,
    /// Whether the call succeeded.
    pub success: bool,
}

impl UsageRecord {
    /// Construct a record (8 fields).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        timestamp_ms: u64,
        provider: impl Into<String>,
        model: impl Into<String>,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        latency_ms: u64,
        success: bool,
    ) -> Self {
        Self {
            timestamp_ms,
            provider: provider.into(),
            model: model.into(),
            input_tokens,
            output_tokens,
            cost_usd,
            latency_ms,
            success,
        }
    }

    /// Build the token half of a record from a normalized response.
    ///
    /// Bridges the canonical [`NormalizedResponse`] `usage` into the
    /// accounting vocabulary. Cost is `0.0` — price it via [`ModelPricing`]
    /// if the caller knows the rates; this crate never invents prices.
    pub fn from_normalized(resp: &NormalizedResponse, latency_ms: u64, success: bool) -> Self {
        Self {
            timestamp_ms: now_ms(),
            provider: String::new(),
            model: resp.model.clone(),
            input_tokens: u64::from(resp.usage.prompt_tokens),
            output_tokens: u64::from(resp.usage.completion_tokens),
            cost_usd: 0.0,
            latency_ms,
            success,
        }
    }

    /// Total tokens (input + output).
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Append-only usage ledger with aggregate queries.
///
/// Legacy 1:1: `apeireth-pipeline::provider_registry::CostTracker` (LiteLLM
/// public `completion_cost` aggregate-query pattern). Pure in-memory; the
/// owner decides the lifetime (turn / session / scheduler) and any
/// persistence.
#[derive(Debug, Clone, Default)]
pub struct CostTracker {
    records: Vec<UsageRecord>,
}

impl CostTracker {
    /// Create an empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one record.
    pub fn record(&mut self, r: UsageRecord) {
        self.records.push(r);
    }

    /// Total record count.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Total cost across all records (USD).
    pub fn total_cost(&self) -> f64 {
        self.records.iter().map(|r| r.cost_usd).sum()
    }

    /// Cost aggregated by provider name (USD).
    pub fn cost_by_provider(&self, provider: &str) -> f64 {
        self.records
            .iter()
            .filter(|r| r.provider == provider)
            .map(|r| r.cost_usd)
            .sum()
    }

    /// Cost aggregated by model name (USD).
    pub fn cost_by_model(&self, model: &str) -> f64 {
        self.records
            .iter()
            .filter(|r| r.model == model)
            .map(|r| r.cost_usd)
            .sum()
    }

    /// Call count for one provider.
    pub fn calls_by_provider(&self, provider: &str) -> usize {
        self.records
            .iter()
            .filter(|r| r.provider == provider)
            .count()
    }

    /// Success rate in `[0.0, 1.0]`; `0.0` when empty (legacy semantics).
    pub fn success_rate(&self) -> f64 {
        if self.records.is_empty() {
            return 0.0;
        }
        let ok = self.records.iter().filter(|r| r.success).count();
        ok as f64 / self.records.len() as f64
    }

    /// Mean latency in milliseconds; `0.0` when empty.
    pub fn avg_latency_ms(&self) -> f64 {
        if self.records.is_empty() {
            return 0.0;
        }
        let sum: u64 = self.records.iter().map(|r| r.latency_ms).sum();
        sum as f64 / self.records.len() as f64
    }

    /// Median latency in milliseconds (lower median for even counts); `0` when empty.
    pub fn p50_latency_ms(&self) -> u64 {
        if self.records.is_empty() {
            return 0;
        }
        let mut lats: Vec<u64> = self.records.iter().map(|r| r.latency_ms).collect();
        lats.sort_unstable();
        lats[lats.len() / 2]
    }

    /// Total input tokens across records.
    pub fn total_input_tokens(&self) -> u64 {
        self.records.iter().map(|r| r.input_tokens).sum()
    }

    /// Total output tokens across records.
    pub fn total_output_tokens(&self) -> u64 {
        self.records.iter().map(|r| r.output_tokens).sum()
    }

    /// Distinct provider names in first-seen order.
    pub fn provider_names(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for r in &self.records {
            if seen.insert(r.provider.as_str()) {
                out.push(r.provider.as_str());
            }
        }
        out
    }

    /// Read-only view of all records.
    pub fn records(&self) -> &[UsageRecord] {
        &self.records
    }
}

/// Millis since `UNIX_EPOCH` (best effort; `0` on clock failure rather than panic).
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalized::NormalizedUsage;

    fn rec(
        provider: &str,
        model: &str,
        input: u64,
        output: u64,
        cost: f64,
        latency: u64,
    ) -> UsageRecord {
        UsageRecord::new(0, provider, model, input, output, cost, latency, true)
    }

    #[test]
    fn pricing_estimate_matches_legacy_formula() {
        // Legacy acceptance example: anthropic-like rates.
        // (1500/1000)*0.003 + (800/1000)*0.015 = 0.0045 + 0.012 = 0.0165
        let pricing = ModelPricing::new(0.003, 0.015);
        let cost = pricing.estimate_cost(1500, 800);
        assert!((cost - 0.0165).abs() < 1e-9, "cost = {cost}");
    }

    #[test]
    fn pricing_zero_rates_legitimately_zero() {
        let pricing = ModelPricing::default();
        assert_eq!(pricing.estimate_cost(10_000, 10_000), 0.0);
    }

    #[test]
    fn cost_tracker_aggregates_by_provider_and_model() {
        let mut tracker = CostTracker::new();
        tracker.record(rec("anthropic", "claude-3", 1500, 800, 0.0165, 200));
        tracker.record(rec("openai", "gpt-4o", 1000, 500, 0.010, 120));
        tracker.record(rec("anthropic", "claude-3", 500, 200, 0.0045, 90));

        assert_eq!(tracker.record_count(), 3);
        assert!((tracker.total_cost() - 0.031).abs() < 1e-9);
        assert!((tracker.cost_by_provider("anthropic") - 0.021).abs() < 1e-9);
        assert!((tracker.cost_by_provider("openai") - 0.010).abs() < 1e-9);
        assert_eq!(tracker.calls_by_provider("anthropic"), 2);
        assert!((tracker.cost_by_model("claude-3") - 0.021).abs() < 1e-9);
        assert_eq!(tracker.calls_by_provider("unknown"), 0);
    }

    #[test]
    fn cost_tracker_token_and_latency_stats() {
        let mut tracker = CostTracker::new();
        tracker.record(rec("p", "m", 100, 50, 0.0, 200));
        tracker.record(rec("p", "m", 10, 5, 0.0, 100));
        tracker.record(rec("q", "m", 1, 1, 0.0, 300));

        assert_eq!(tracker.total_input_tokens(), 111);
        assert_eq!(tracker.total_output_tokens(), 56);
        assert!((tracker.avg_latency_ms() - 200.0).abs() < 1e-9);
        // sorted [100, 200, 300] → median index 1 = 200
        assert_eq!(tracker.p50_latency_ms(), 200);
        assert_eq!(tracker.provider_names(), vec!["p", "q"]);
    }

    #[test]
    fn cost_tracker_empty_is_safe() {
        let tracker = CostTracker::new();
        assert_eq!(tracker.record_count(), 0);
        assert_eq!(tracker.total_cost(), 0.0);
        assert_eq!(tracker.success_rate(), 0.0);
        assert_eq!(tracker.avg_latency_ms(), 0.0);
        assert_eq!(tracker.p50_latency_ms(), 0);
        assert!(tracker.provider_names().is_empty());
    }

    #[test]
    fn success_rate_counts_only_successes() {
        let mut tracker = CostTracker::new();
        tracker.record(rec("p", "m", 1, 1, 0.0, 10));
        let mut failed = rec("p", "m", 1, 1, 0.0, 10);
        failed.success = false;
        tracker.record(failed);
        assert!((tracker.success_rate() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn usage_record_total_tokens_and_serde_round_trip() {
        let r = UsageRecord::new(
            1722931200000,
            "minimax",
            "MiniMax-M3",
            1500,
            800,
            0.0165,
            234,
            true,
        );
        assert_eq!(r.total_tokens(), 2300);
        let json = serde_json::to_string(&r).unwrap();
        let back: UsageRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
        assert!(json.contains("\"provider\":\"minimax\""));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn usage_record_from_normalized_uses_response_usage_without_fabricating_cost() {
        let resp = crate::normalized::NormalizedResponse {
            id: "resp_1".into(),
            model: "MiniMax-M3".into(),
            content: "hi".into(),
            finish_reason: Some(crate::normalized::NormalizedFinishReason::Stop),
            usage: NormalizedUsage::new(1500, 800),
            tool_calls: Vec::new(),
            raw_metadata: Default::default(),
        };
        let r = UsageRecord::from_normalized(&resp, 234, true);
        assert_eq!(r.input_tokens, 1500);
        assert_eq!(r.output_tokens, 800);
        assert_eq!(r.model, "MiniMax-M3");
        assert_eq!(r.cost_usd, 0.0);
        assert!(r.success);

        // Price it explicitly with known rates.
        let priced = ModelPricing::new(0.003, 0.015).estimate_cost(r.input_tokens, r.output_tokens);
        assert!((priced - 0.0165).abs() < 1e-9);
    }
}
