//! Reflection module - post-decision review that produces a reflection report.

use apeireth_asi::{AsiV05Scores, V1136Submeasures};
use apeireth_core::PhilosophyVerdict;

use crate::{CognitiveInput, CognitiveOutput};

/// Reflection verdict.
#[derive(Debug, Clone)]
pub enum ReflectionVerdict {
    /// Cycle stable, no anomaly.
    Stable,
    /// Anomaly detected, further review needed.
    Anomaly(String),
}

/// Reflection report - cycle metadata + scores + reflection verdict.
#[derive(Debug, Clone)]
pub struct ReflectionReport {
    /// Reflection ID.
    pub reflection_id: String,
    /// Context tag of associated input.
    pub context_tag: String,
    /// ASI V0.5 average score.
    pub v05_avg: f64,
    /// ASI V1136 average score (across 7 sub-measures).
    pub v1136_avg: f64,
    /// Verdict reject count.
    pub block_count: usize,
    /// Reflection verdict.
    pub verdict: ReflectionVerdict,
}

/// Reflect on cycle results, producing a report.
pub fn reflect(
    input: &CognitiveInput,
    v05: &AsiV05Scores,
    v1136: &V1136Submeasures,
    verdicts: &[PhilosophyVerdict],
    output: &CognitiveOutput,
) -> ReflectionReport {
    let v05_avg =
        (v05.continuity + v05.salience + v05.identity + v05.philosophy_guard + v05.transferability)
            / 5.0;
    let v1136_avg = {
        let sum_c: f64 = v1136.continuity_5.iter().sum();
        let sum_t: f64 = v1136.transferability_2.iter().sum();
        (sum_c + sum_t) / 7.0
    };
    let block_count = verdicts
        .iter()
        .filter(|v| matches!(v, PhilosophyVerdict::Block(_)))
        .count();

    let verdict = if block_count > 0 {
        ReflectionVerdict::Anomaly(format!(
            "{} of {} verdicts blocked the cycle",
            block_count,
            verdicts.len()
        ))
    } else if v05_avg < 0.3 {
        ReflectionVerdict::Anomaly(format!("low v05 avg {:.3}", v05_avg))
    } else {
        ReflectionVerdict::Stable
    };

    let _ = output; // currently unused, reserved for extension

    ReflectionReport {
        reflection_id: format!("refl-{}", input.input_id),
        context_tag: input.context_tag.clone(),
        v05_avg,
        v1136_avg,
        block_count,
        verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::ActionTarget;
    use uuid::Uuid;

    fn sample_input() -> CognitiveInput {
        let target = ActionTarget::NormalAction("noop".to_string());
        CognitiveInput::new(vec![target], "reflection_test")
    }

    #[test]
    fn reflect_returns_stable_for_normal_action() {
        let input = sample_input();
        let v05 = AsiV05Scores {
            continuity: 0.8,
            salience: 0.7,
            identity: 0.9,
            philosophy_guard: 0.95,
            transferability: 0.85,
        };
        let v1136 = V1136Submeasures::default();
        let verdicts = vec![PhilosophyVerdict::Allow];
        let output = CognitiveOutput::Decision("ok".to_string());

        let report = reflect(&input, &v05, &v1136, &verdicts, &output);
        assert_eq!(report.block_count, 0);
        assert!(matches!(report.verdict, ReflectionVerdict::Stable));
    }

    #[test]
    fn reflect_returns_anomaly_when_block_present() {
        let input = sample_input();
        let v05 = AsiV05Scores::default();
        let v1136 = V1136Submeasures::default();
        let verdicts = vec![PhilosophyVerdict::Block(
            apeireth_core::PhilosophyKey::NotClone,
        )];
        let output = CognitiveOutput::Reject(apeireth_core::PhilosophyKey::NotClone);

        let report = reflect(&input, &v05, &v1136, &verdicts, &output);
        assert_eq!(report.block_count, 1);
        assert!(matches!(report.verdict, ReflectionVerdict::Anomaly(_)));
    }

    #[test]
    fn reflection_report_has_unique_id_per_input() {
        let input = sample_input();
        let v05 = AsiV05Scores::default();
        let v1136 = V1136Submeasures::default();
        let verdicts = vec![PhilosophyVerdict::Allow];
        let output = CognitiveOutput::Decision("ok".to_string());

        let report = reflect(&input, &v05, &v1136, &verdicts, &output);
        // reflection_id format: "refl-<uuid>"
        assert!(report.reflection_id.starts_with("refl-"));
        let uuid_str = report.reflection_id.trim_start_matches("refl-");
        assert!(Uuid::parse_str(uuid_str).is_ok());
    }

    #[test]
    fn reflect_returns_anomaly_when_v05_low() {
        let input = sample_input();
        let v05 = AsiV05Scores {
            continuity: 0.1,
            salience: 0.1,
            identity: 0.1,
            philosophy_guard: 0.1,
            transferability: 0.1,
        };
        let v1136 = V1136Submeasures::default();
        let verdicts = vec![PhilosophyVerdict::Allow];
        let output = CognitiveOutput::Decision("ok".to_string());

        let report = reflect(&input, &v05, &v1136, &verdicts, &output);
        assert_eq!(report.block_count, 0);
        assert!(matches!(report.verdict, ReflectionVerdict::Anomaly(_)));
    }

    #[test]
    fn reflection_report_context_tag_propagates() {
        let target = ActionTarget::NormalAction("x".into());
        let input = CognitiveInput::new(vec![target], "my_tag");
        let v05 = AsiV05Scores::default();
        let v1136 = V1136Submeasures::default();
        let verdicts = vec![PhilosophyVerdict::Allow];
        let output = CognitiveOutput::Decision("ok".to_string());

        let report = reflect(&input, &v05, &v1136, &verdicts, &output);
        assert_eq!(report.context_tag, "my_tag");
    }
}
