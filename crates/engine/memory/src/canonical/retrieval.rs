//! Canonical memory retrieval semantics (M1B2).
//!
//! Retrieval is intentionally separated from persistence:
//!
//! - [`MemoryRepository`] persists values.
//! - this module interprets them.
//!
//! The donor formula is ACT-R-inspired activation plus an importance bonus.
//! It is implemented as a pure deterministic function: no database access, no
//! hidden wall clock, no global state.

use apeireth_core::kernel::Timestamp;

use super::domain::MemoryItem;
use super::error::MemoryError;
use super::repository::{MemoryFilter, MemoryRepository};

/// Default donor decay for ACT-R-style activation.
pub const DEFAULT_ACT_R_DECAY: f64 = 0.5;
/// Default donor beta for ACT-R-style activation.
pub const DEFAULT_ACT_R_BETA: f64 = 0.0;
/// Default donor importance weight in the retrieval score.
pub const DEFAULT_IMPORTANCE_WEIGHT: f64 = 2.0;

/// Calculates the donor ACT-R-inspired activation.
///
/// Formula (from `memory_v2.rs`):
///
/// ```text
/// sum = Σ (max(current_time - t_j, 1)).powf(-decay)
/// activation = if sum > 0 { sum.ln() + beta } else { beta }
/// ```
///
/// Times are compared in Unix **seconds**. Future access timestamps are
/// clipped to a one-second difference, exactly as in the donor
/// implementation. Empty access history returns `beta`.
///
/// # Errors
///
/// Returns [`MemoryError::InvalidData`] when `decay` is not finite or not
/// strictly positive, or when `beta` is not finite.
pub fn act_r_activation(
    access_times: &[Timestamp],
    as_of: Timestamp,
    decay: f64,
    beta: f64,
) -> Result<f64, MemoryError> {
    if !decay.is_finite() || decay <= 0.0 {
        return Err(MemoryError::InvalidData(format!(
            "ACT-R decay must be finite and strictly positive, got {decay}"
        )));
    }
    if !beta.is_finite() {
        return Err(MemoryError::InvalidData(format!(
            "ACT-R beta must be finite, got {beta}"
        )));
    }

    let current_sec = as_of.as_datetime().timestamp();
    let mut sum = 0.0;
    for access_time in access_times {
        let t_j = access_time.as_datetime().timestamp();
        let diff = (current_sec - t_j).max(1) as f64;
        sum += diff.powf(-decay);
    }

    if sum > 0.0 {
        Ok(sum.ln() + beta)
    } else {
        Ok(beta)
    }
}

/// A retrieval result: the item plus its transient retrieval score.
///
/// The score is deliberately not stored on [`MemoryItem`]; it belongs to the
/// retrieval layer, not to durable domain state.
#[derive(Debug, Clone)]
pub struct MemoryHit {
    pub item: MemoryItem,
    pub score: f64,
}

/// Explicit retrieval query options.
///
/// `as_of` is the only clock input and is required. Defaults preserve the
/// donor query semantics: decay `0.5`, beta `0.0`, importance weight `2.0`.
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalOptions {
    /// Temporal probe. Items are eligible iff they are effective at `as_of`
    /// (repository filtering).
    pub as_of: Timestamp,
    /// Include tombstoned items in the candidate set. Defaults to `false`.
    pub include_tombstones: bool,
    /// Optional maximum number of hits after ranking.
    pub limit: Option<usize>,
    /// Optional inclusive importance floor, applied before ranking.
    pub minimum_importance: Option<f64>,
    /// ACT-R decay parameter. Must be finite and strictly positive.
    pub decay: f64,
    /// ACT-R beta parameter. Must be finite.
    pub beta: f64,
    /// Weight multiplied with importance in the final score.
    pub importance_weight: f64,
}

impl RetrievalOptions {
    /// Creates donor-default retrieval options for `as_of`.
    pub fn new(as_of: Timestamp) -> Self {
        Self {
            as_of,
            include_tombstones: false,
            limit: None,
            minimum_importance: None,
            decay: DEFAULT_ACT_R_DECAY,
            beta: DEFAULT_ACT_R_BETA,
            importance_weight: DEFAULT_IMPORTANCE_WEIGHT,
        }
    }

    /// Sets whether tombstoned items are eligible.
    pub fn with_include_tombstones(mut self, include_tombstones: bool) -> Self {
        self.include_tombstones = include_tombstones;
        self
    }

    /// Caps the number of returned hits.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the inclusive importance floor.
    pub fn with_minimum_importance(mut self, minimum_importance: f64) -> Self {
        self.minimum_importance = Some(minimum_importance);
        self
    }

    /// Sets ACT-R parameters.
    pub fn with_act_r(mut self, decay: f64, beta: f64) -> Self {
        self.decay = decay;
        self.beta = beta;
        self
    }

    /// Sets the importance weight.
    pub fn with_importance_weight(mut self, importance_weight: f64) -> Self {
        self.importance_weight = importance_weight;
        self
    }

    fn validate(&self) -> Result<(), MemoryError> {
        if !self.decay.is_finite() || self.decay <= 0.0 {
            return Err(MemoryError::InvalidData(format!(
                "retrieval decay must be finite and strictly positive, got {}",
                self.decay
            )));
        }
        if !self.beta.is_finite() {
            return Err(MemoryError::InvalidData(format!(
                "retrieval beta must be finite, got {}",
                self.beta
            )));
        }
        if !self.importance_weight.is_finite() {
            return Err(MemoryError::InvalidData(format!(
                "retrieval importance_weight must be finite, got {}",
                self.importance_weight
            )));
        }
        if let Some(minimum_importance) = self.minimum_importance {
            if !minimum_importance.is_finite() {
                return Err(MemoryError::InvalidData(format!(
                    "retrieval minimum_importance must be finite, got {minimum_importance}"
                )));
            }
        }
        Ok(())
    }
}

/// Retrieves eligible memories from `repo` and ranks them deterministically.
///
/// The retrieval score is:
///
/// ```text
/// score = ACT-R activation(access_times, as_of, decay, beta)
///       + importance * importance_weight
/// ```
///
/// Ordering is `score` descending, then `created_at` ascending, then `id`
/// ascending. The tie-breakers are explicit and stable; no `HashMap`
/// iteration order is involved.
pub async fn retrieve(
    repo: &dyn MemoryRepository,
    options: &RetrievalOptions,
) -> Result<Vec<MemoryHit>, MemoryError> {
    options.validate()?;

    let filter =
        MemoryFilter::new(options.as_of).with_include_tombstones(options.include_tombstones);
    let mut items = repo.query(&filter).await?;

    if let Some(minimum_importance) = options.minimum_importance {
        items.retain(|item| item.importance >= minimum_importance);
    }

    let mut hits = Vec::with_capacity(items.len());
    for item in items {
        let activation = act_r_activation(
            &item.access_times,
            options.as_of,
            options.decay,
            options.beta,
        )?;
        let score = activation + item.importance * options.importance_weight;
        if !score.is_finite() {
            return Err(MemoryError::InvalidData(format!(
                "retrieval score became non-finite for memory {}",
                item.id
            )));
        }
        hits.push(MemoryHit { item, score });
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.item.created_at.cmp(&b.item.created_at))
            .then_with(|| a.item.id.cmp(&b.item.id))
    });

    if let Some(limit) = options.limit {
        hits.truncate(limit);
    }

    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(ms: i64) -> Timestamp {
        Timestamp::from_epoch_millis(ms).unwrap()
    }

    #[test]
    fn activation_empty_access_history_returns_beta() {
        let activation = act_r_activation(&[], ts(1_000_000), 0.5, 1.25).unwrap();
        assert!((activation - 1.25).abs() < 1e-12);
    }

    #[test]
    fn activation_matches_donor_formula_exact_values() {
        // access at t-100s with decay 0.5: sum = 100^-0.5 = 0.1
        // activation = ln(0.1) + 0 = -2.302585092994046
        let as_of = ts(200_000_000);
        let access = ts(as_of.epoch_millis() - 100_000);
        let activation = act_r_activation(&[access], as_of, 0.5, 0.0).unwrap();
        assert!((activation - (-2.302_585_092_994_046)).abs() < 1e-12);

        // Two accesses at t-100s and t-400s:
        // sum = 0.1 + 0.05 = 0.15
        // activation = ln(0.15) = -1.8971199848858813
        let access2 = ts(as_of.epoch_millis() - 400_000);
        let activation2 = act_r_activation(&[access, access2], as_of, 0.5, 0.0).unwrap();
        assert!((activation2 - (-1.897_119_984_885_881_3)).abs() < 1e-12);
    }

    #[test]
    fn activation_clips_future_timestamps_to_one_second() {
        // A future access would produce diff = -1; donor clamps to 1, so
        // sum = 1^-0.5 = 1 and activation = ln(1) + beta = beta.
        let as_of = ts(100_000_000);
        let future = ts(as_of.epoch_millis() + 100_000);
        let activation = act_r_activation(&[future], as_of, 0.5, 2.0).unwrap();
        assert!((activation - 2.0).abs() < 1e-12);
    }

    #[test]
    fn activation_rejects_invalid_numeric_parameters() {
        let as_of = ts(100_000_000);
        let access = ts(99_000_000);

        for decay in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                act_r_activation(&[access], as_of, decay, 0.0),
                Err(MemoryError::InvalidData(_))
            ));
        }

        assert!(matches!(
            act_r_activation(&[access], as_of, 0.5, f64::NAN),
            Err(MemoryError::InvalidData(_))
        ));
        assert!(matches!(
            act_r_activation(&[access], as_of, 0.5, f64::INFINITY),
            Err(MemoryError::InvalidData(_))
        ));
    }

    #[test]
    fn more_recent_access_ranks_higher_than_older_access() {
        let as_of = ts(200_000_000);
        let old_access = ts(as_of.epoch_millis() - 100_000);
        let recent_access = ts(as_of.epoch_millis() - 1_000);

        let old_activation = act_r_activation(&[old_access], as_of, 0.5, 0.0).unwrap();
        let recent_activation = act_r_activation(&[recent_access], as_of, 0.5, 0.0).unwrap();

        assert!(recent_activation > old_activation);
    }

    #[test]
    fn zero_access_items_are_ranked_by_importance_bonus() {
        let _as_of = ts(200_000_000);
        let zero_access = 0.0 + DEFAULT_ACT_R_BETA; // activation beta
        let importance_bonus_high = 0.9 * DEFAULT_IMPORTANCE_WEIGHT;
        let importance_bonus_low = 0.1 * DEFAULT_IMPORTANCE_WEIGHT;
        assert!(importance_bonus_high > importance_bonus_low);
        assert!(zero_access + importance_bonus_high > zero_access + importance_bonus_low);
    }
}
