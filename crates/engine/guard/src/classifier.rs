//! Optional local classifier contract for behavior-chain risk.
//!
//! The runtime remains deterministic when no model is installed.  A model is
//! an input to the final fusion step, never a replacement for the Fast Guard
//! or Chain Guard evidence.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::features::AgentChainFeatureV1;
use crate::features_v2::AgentChainFeatureV2;

/// Coarse risk class emitted by a local classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassifierEnforcementMode {
    Shadow,
    Advisory,
    Enforce,
}

impl ClassifierEnforcementMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shadow => "shadow",
            Self::Advisory => "advisory",
            Self::Enforce => "enforce",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "shadow" => Some(Self::Shadow),
            "advisory" => Some(Self::Advisory),
            "enforce" => Some(Self::Enforce),
            _ => None,
        }
    }
}

/// Model output with explicit availability and provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskPrediction {
    pub class: RiskClass,
    pub score: f64,
    pub confidence: f64,
    pub model_version: String,
    pub available: bool,
}

impl RiskPrediction {
    pub fn unavailable() -> Self {
        Self {
            class: RiskClass::Unavailable,
            score: 0.0,
            confidence: 0.0,
            model_version: "none".to_string(),
            available: false,
        }
    }

    pub fn clamped(mut self) -> Self {
        self.score = self.score.clamp(0.0, 1.0);
        self.confidence = self.confidence.clamp(0.0, 1.0);
        self
    }
}

/// Synchronous, allocation-light classifier boundary for the hot path.
pub trait ChainRiskClassifier: Send + Sync {
    fn classify(&self, features: &AgentChainFeatureV1) -> RiskPrediction;

    fn classify_v2(&self, features: &AgentChainFeatureV2) -> RiskPrediction {
        self.classify(&features.v1)
    }

    fn available(&self) -> bool {
        true
    }

    fn model_version(&self) -> Option<String> {
        None
    }

    fn enforcement_mode(&self) -> ClassifierEnforcementMode {
        ClassifierEnforcementMode::Enforce
    }

    fn model_reason(&self) -> Option<String> {
        None
    }
}

/// Default classifier used when no model artifact is configured.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoClassifier;

impl ChainRiskClassifier for NoClassifier {
    fn classify(&self, _features: &AgentChainFeatureV1) -> RiskPrediction {
        RiskPrediction::unavailable()
    }

    fn available(&self) -> bool {
        false
    }

    fn enforcement_mode(&self) -> ClassifierEnforcementMode {
        ClassifierEnforcementMode::Shadow
    }

    fn model_reason(&self) -> Option<String> {
        Some("no local model artifact configured".to_string())
    }
}

/// Small deterministic classifier useful for local integration tests and
/// shadow-mode calibration.  It is deliberately not presented as a trained
/// model.
#[derive(Debug, Clone)]
pub struct ThresholdClassifier {
    pub model_version: String,
}

impl Default for ThresholdClassifier {
    fn default() -> Self {
        Self {
            model_version: "threshold-shadow-v1".to_string(),
        }
    }
}

impl ChainRiskClassifier for ThresholdClassifier {
    fn classify(&self, features: &AgentChainFeatureV1) -> RiskPrediction {
        let score = [
            (features.sensitive_to_external_flow, 0.95),
            (features.retry_after_denial, 0.88),
            (features.alternate_tool_after_denial, 0.82),
            (features.destructive_chain_score(), 0.80),
            (features.credential_access_count > 0, 0.75),
            (features.external_effect_count > 0, 0.55),
        ]
        .into_iter()
        .filter_map(|(matched, value)| matched.then_some(value))
        .max_by(f64::total_cmp)
        .unwrap_or(0.05);
        let class = if score >= 0.90 {
            RiskClass::Critical
        } else if score >= 0.75 {
            RiskClass::High
        } else if score >= 0.40 {
            RiskClass::Medium
        } else {
            RiskClass::Low
        };
        RiskPrediction {
            class,
            score,
            confidence: 0.55,
            model_version: self.model_version.clone(),
            available: true,
        }
    }

    fn classify_v2(&self, features: &AgentChainFeatureV2) -> RiskPrediction {
        let mut prediction = self.classify(&features.v1);
        let joint_score = [
            (features.alignment_score >= 0.9, 0.96),
            (features.credential_to_external, 0.98),
            (features.unrequested_network_egress, 0.9),
            (features.unrequested_publish, 0.78),
            (features.scope_expansion_count > 0, 0.72),
        ]
        .into_iter()
        .filter_map(|(matched, value)| matched.then_some(value))
        .max_by(f64::total_cmp)
        .unwrap_or(prediction.score);
        prediction.score = prediction.score.max(joint_score);
        prediction.class = if prediction.score >= 0.9 {
            RiskClass::Critical
        } else if prediction.score >= 0.75 {
            RiskClass::High
        } else if prediction.score >= 0.4 {
            RiskClass::Medium
        } else {
            RiskClass::Low
        };
        prediction
    }

    fn model_version(&self) -> Option<String> {
        Some(self.model_version.clone())
    }

    fn enforcement_mode(&self) -> ClassifierEnforcementMode {
        ClassifierEnforcementMode::Shadow
    }

    fn model_reason(&self) -> Option<String> {
        Some("deterministic heuristic; not trained ML".to_string())
    }
}

/// Portable local linear model artifact used by the shadow-training pipeline.
/// It intentionally has no runtime dependency on Python, ONNX, or a remote
/// service. The artifact is rejected when its feature schema does not match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointModelArtifact {
    pub schema_version: String,
    pub model_id: String,
    pub model_version: String,
    pub feature_names: Vec<String>,
    pub weights: Vec<f64>,
    pub bias: f64,
    pub critical_threshold: f64,
    pub high_threshold: f64,
    pub medium_threshold: f64,
    #[serde(default)]
    pub mode: Option<ClassifierEnforcementMode>,
}

/// Runtime inference for a trained/synthetic joint intent-behavior artifact.
#[derive(Debug, Clone)]
pub struct JointRiskClassifier {
    artifact: JointModelArtifact,
    mode: ClassifierEnforcementMode,
}

impl JointRiskClassifier {
    pub fn from_json_str(serialized: &str) -> Result<Self, String> {
        let artifact: JointModelArtifact = serde_json::from_str(serialized)
            .map_err(|_| "invalid local model artifact".to_string())?;
        if artifact.schema_version != crate::features_v2::AGENT_CHAIN_FEATURE_V2
            || artifact.feature_names.len() != artifact.weights.len()
            || artifact.feature_names.is_empty()
            || !artifact
                .weights
                .iter()
                .chain(std::iter::once(&artifact.bias))
                .all(|value| value.is_finite())
        {
            return Err("local model schema or numeric validation failed".to_string());
        }
        let mode = artifact.mode.unwrap_or(ClassifierEnforcementMode::Shadow);
        Ok(Self { artifact, mode })
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let serialized =
            std::fs::read_to_string(path).map_err(|_| "local model unavailable".to_string())?;
        Self::from_json_str(&serialized)
    }

    #[must_use]
    pub fn with_mode(mut self, mode: ClassifierEnforcementMode) -> Self {
        self.mode = mode;
        self
    }

    fn feature_value(name: &str, features: &AgentChainFeatureV2) -> f64 {
        let flag = |value: bool| f64::from(u8::from(value));
        match name {
            "alignment_score" => features.alignment_score,
            "credential_to_external" => flag(features.credential_to_external),
            "unrequested_network_egress" => flag(features.unrequested_network_egress),
            "unrequested_credential_access" => flag(features.unrequested_credential_access),
            "unrequested_shell_execution" => flag(features.unrequested_shell_execution),
            "unrequested_delete" => flag(features.unrequested_delete),
            "unrequested_publish" => flag(features.unrequested_publish),
            "sensitive_to_external_flow" => flag(features.v1.sensitive_to_external_flow),
            "retry_after_denial" => flag(features.v1.retry_after_denial),
            "alternate_tool_after_denial" => flag(features.v1.alternate_tool_after_denial),
            "denied_count" => f64::from(features.v1.denied_count),
            "external_effect_count" => f64::from(features.v1.external_effect_count),
            "scope_expansion_count" => f64::from(features.scope_expansion_count),
            "cross_turn_denied_action_count" => f64::from(features.cross_turn.denied_action_count),
            "cross_turn_credential_probe_count" => {
                f64::from(features.cross_turn.credential_probe_count)
            }
            "failed_action_ratio" => features.failed_action_ratio,
            _ => 0.0,
        }
    }
}

impl ChainRiskClassifier for JointRiskClassifier {
    fn classify(&self, features: &AgentChainFeatureV1) -> RiskPrediction {
        let v2 = AgentChainFeatureV2 {
            v1: features.clone(),
            schema_version: crate::features_v2::AGENT_CHAIN_FEATURE_V2.to_string(),
            ..AgentChainFeatureV2::default()
        };
        self.classify_v2(&v2)
    }

    fn classify_v2(&self, features: &AgentChainFeatureV2) -> RiskPrediction {
        let logit = self.artifact.bias
            + self
                .artifact
                .feature_names
                .iter()
                .zip(&self.artifact.weights)
                .map(|(name, weight)| weight * Self::feature_value(name, features))
                .sum::<f64>();
        let score = 1.0 / (1.0 + (-logit.clamp(-30.0, 30.0)).exp());
        let class = if score >= self.artifact.critical_threshold {
            RiskClass::Critical
        } else if score >= self.artifact.high_threshold {
            RiskClass::High
        } else if score >= self.artifact.medium_threshold {
            RiskClass::Medium
        } else {
            RiskClass::Low
        };
        RiskPrediction {
            class,
            score,
            confidence: (0.5 + (score - 0.5).abs()).clamp(0.0, 1.0),
            model_version: self.artifact.model_version.clone(),
            available: true,
        }
    }

    fn model_version(&self) -> Option<String> {
        Some(self.artifact.model_version.clone())
    }

    fn enforcement_mode(&self) -> ClassifierEnforcementMode {
        self.mode
    }

    fn model_reason(&self) -> Option<String> {
        Some(format!(
            "local linear artifact {}; mode={}",
            self.artifact.model_id,
            self.mode.as_str()
        ))
    }
}

impl AgentChainFeatureV1 {
    fn destructive_chain_score(&self) -> bool {
        self.delete_count > 0 && self.network_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact_json() -> String {
        serde_json::json!({
            "schema_version": "AgentChainFeatureV2",
            "model_id": "guard-joint-shadow-v0",
            "model_version": "guard-joint-shadow-v0.1",
            "feature_names": ["alignment_score", "credential_to_external"],
            "weights": [10.0, 4.0],
            "bias": -5.0,
            "critical_threshold": 0.9,
            "high_threshold": 0.7,
            "medium_threshold": 0.4,
            "mode": "shadow"
        })
        .to_string()
    }

    #[test]
    fn joint_artifact_validates_schema_and_scores_v2_features() {
        let classifier = JointRiskClassifier::from_json_str(&artifact_json()).unwrap();
        let mut features = AgentChainFeatureV2::default();
        let low = classifier.classify_v2(&features);
        assert!(matches!(low.class, RiskClass::Low));
        features.alignment_score = 0.95;
        let high = classifier.classify_v2(&features);
        assert!(matches!(high.class, RiskClass::Critical));
        assert_eq!(
            classifier.enforcement_mode(),
            ClassifierEnforcementMode::Shadow
        );
    }

    #[test]
    fn joint_artifact_rejects_wrong_feature_schema() {
        let invalid = artifact_json().replace("AgentChainFeatureV2", "AgentChainFeatureV1");
        assert!(JointRiskClassifier::from_json_str(&invalid).is_err());
    }

    #[test]
    fn generated_shadow_artifact_is_loadable() {
        let serialized = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../artifacts/guard-joint-shadow-v0.json"
        ));
        let classifier = JointRiskClassifier::from_json_str(serialized).unwrap();
        assert_eq!(
            classifier.model_version().as_deref(),
            Some("guard-joint-shadow-v0.1")
        );
        assert_eq!(
            classifier.enforcement_mode(),
            ClassifierEnforcementMode::Shadow
        );
    }
}
