//! R22 ST-A3 — per-dimension 增强 (v1 等价, 8 项承诺 LOCKED)
use crate::measurement::{compute_dim, MeasurementError, MeasurementSample};

pub trait PerDimEnhance {
    fn name(&self) -> &'static str;
    fn enhance(&self, base_value: f64, sample: &MeasurementSample) -> Result<f64, MeasurementError>;
}

fn finite_base(name: &'static str, value: f64) -> Result<f64, MeasurementError> {
    if value.is_finite() { Ok(value.clamp(0.0, 1.0)) } else { Err(MeasurementError::NonFiniteValue(name.to_string())) }
}
fn quality(sample: &MeasurementSample, name: &str) -> f64 {
    sample.qualities.get(name).copied().unwrap_or(1.0).clamp(0.0, 1.0)
}

pub struct NoveltyScoreEnhance;
impl PerDimEnhance for NoveltyScoreEnhance {
    fn name(&self) -> &'static str { "novelty_score" }
    fn enhance(&self, base: f64, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
        let base = finite_base(self.name(), base)?;
        let penalty = sample.latencies_ms.get(self.name()).copied().unwrap_or(0.0).max(0.0).min(5_000.0) / 5_000.0 * 0.2;
        Ok((base - penalty).clamp(0.0, 1.0))
    }
}
pub struct ActionabilityScoreEnhance;
impl PerDimEnhance for ActionabilityScoreEnhance {
    fn name(&self) -> &'static str { "actionability_score" }
    fn enhance(&self, base: f64, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
        let base = finite_base(self.name(), base)?;
        Ok((base * quality(sample, self.name())).clamp(0.0, 1.0))
    }
}
pub struct ConfidenceScoreEnhance;
impl PerDimEnhance for ConfidenceScoreEnhance {
    fn name(&self) -> &'static str { "confidence_score" }
    fn enhance(&self, base: f64, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
        let base = finite_base(self.name(), base)?;
        let variance_penalty = 1.0 - (1.0 - quality(sample, self.name())) * 0.5;
        Ok((base * variance_penalty).clamp(0.0, 1.0))
    }
}
pub struct TemporalRelevanceEnhance;
impl PerDimEnhance for TemporalRelevanceEnhance {
    fn name(&self) -> &'static str { "temporal_relevance" }
    fn enhance(&self, base: f64, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
        let base = finite_base(self.name(), base)?;
        let recency = sample.latencies_ms.get("temporal_relevance_recency").copied().unwrap_or(0.0).clamp(0.0, 1.0);
        Ok((base + (1.0 - base) * recency * 0.2).clamp(0.0, 1.0))
    }
}
pub struct CoreValuesConsistencyEnhance;
impl PerDimEnhance for CoreValuesConsistencyEnhance {
    fn name(&self) -> &'static str { "core_values_consistency" }
    fn enhance(&self, base: f64, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
        let base = finite_base(self.name(), base)?;
        let guard = sample.philosophy_gate_trials.get("v1_pass_rate")
            .map(|(passed, total)| if *total == 0 { 0.0 } else { f64::from(*passed) / f64::from(*total) })
            .unwrap_or(1.0);
        Ok((base * guard.clamp(0.0, 1.0)).clamp(0.0, 1.0))
    }
}
pub struct VoiceConsistencyEnhance;
impl PerDimEnhance for VoiceConsistencyEnhance {
    fn name(&self) -> &'static str { "voice_consistency" }
    fn enhance(&self, base: f64, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
        let base = finite_base(self.name(), base)?;
        let identity = compute_dim("identity_persistence", sample)?;
        Ok((base * identity).clamp(0.0, 1.0))
    }
}

pub fn enhance_measurement(name: &str, base_value: f64, sample: &MeasurementSample) -> Result<f64, MeasurementError> {
    let enhancer: Option<Box<dyn PerDimEnhance>> = match name {
        "novelty_score" => Some(Box::new(NoveltyScoreEnhance)),
        "actionability_score" => Some(Box::new(ActionabilityScoreEnhance)),
        "confidence_score" => Some(Box::new(ConfidenceScoreEnhance)),
        "temporal_relevance" => Some(Box::new(TemporalRelevanceEnhance)),
        "core_values_consistency" => Some(Box::new(CoreValuesConsistencyEnhance)),
        "voice_consistency" => Some(Box::new(VoiceConsistencyEnhance)),
        _ => None,
    };
    match enhancer {
        Some(s) => s.enhance(base_value, sample),
        None => finite_base("unknown", base_value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> MeasurementSample {
        let mut s = MeasurementSample::default();
        for name in ["novelty_score","actionability_score","confidence_score","temporal_relevance","core_values_consistency","voice_consistency","identity_persistence"] {
            s.successes.insert(name.into(), 8);
            s.attempts.insert(name.into(), 10);
            s.qualities.insert(name.into(), 0.8);
        }
        s
    }
    #[test] fn m7_penalty_bounded() {
        let mut s = sample(); s.latencies_ms.insert("novelty_score".into(), 5_000.0);
        assert_eq!(NoveltyScoreEnhance.enhance(1.0, &s).unwrap(), 0.8);
    }
    #[test] fn m8_quality() { assert_eq!(ActionabilityScoreEnhance.enhance(1.0, &sample()).unwrap(), 0.8); }
    #[test] fn m9_variance_penalty() { assert!(ConfidenceScoreEnhance.enhance(1.0, &sample()).unwrap() < 1.0); }
    #[test] fn m10_recency_boost() {
        let mut s = sample(); s.latencies_ms.insert("temporal_relevance_recency".into(), 1.0);
        assert!(TemporalRelevanceEnhance.enhance(0.5, &s).unwrap() > 0.5);
    }
    #[test] fn m11_guard() {
        let mut s = sample(); s.philosophy_gate_trials.insert("v1_pass_rate".into(), (5, 10));
        assert_eq!(CoreValuesConsistencyEnhance.enhance(1.0, &s).unwrap(), 0.5);
    }
    #[test] fn m12_identity() { assert!(VoiceConsistencyEnhance.enhance(1.0, &sample()).unwrap() < 1.0); }
    #[test] fn dispatch_all_six() {
        let s = sample();
        for n in ["novelty_score","actionability_score","confidence_score","temporal_relevance","core_values_consistency","voice_consistency"] {
            assert!(enhance_measurement(n, 0.8, &s).is_ok());
        }
    }
    #[test] fn fallback_other() { assert_eq!(enhance_measurement("thread_continuity", 0.7, &sample()).unwrap(), 0.7); }
    #[test] fn rejects_nan() { assert!(enhance_measurement("novelty_score", f64::NAN, &sample()).is_err()); }
}
