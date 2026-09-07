//! Fusion of deterministic guard evidence and optional model output.

use apeireth_governance::Decision;

use crate::classifier::{ClassifierEnforcementMode, RiskClass, RiskPrediction};
use crate::decision::{GuardDecision, GuardStage};
use crate::fast_guard::FastGuardResult;
use crate::features::AgentChainFeatureV1;
use crate::features_v2::AgentChainFeatureV2;

/// Final decision fusion. Deterministic denials always win. Model output is
/// applied according to the rollout mode: shadow never changes the decision,
/// advisory may escalate Allow to RequireApproval, and enforce may deny only
/// with supporting structural evidence.
pub struct DecisionFusion;

impl DecisionFusion {
    pub fn attach_prediction(base: &GuardDecision, prediction: &RiskPrediction) -> GuardDecision {
        let mut observed = base.clone();
        observed.classifier_prediction = Some(prediction.clone());
        observed
    }

    pub fn apply(
        mode: ClassifierEnforcementMode,
        base: &GuardDecision,
        fast: &FastGuardResult,
        prediction: &RiskPrediction,
        features: &AgentChainFeatureV2,
    ) -> GuardDecision {
        match mode {
            ClassifierEnforcementMode::Disabled | ClassifierEnforcementMode::Shadow => {
                Self::attach_prediction(base, prediction)
            }
            ClassifierEnforcementMode::Advisory => {
                let mut fused = Self::fuse_v2(base, fast, prediction, features);
                if matches!(fused.decision, Decision::Deny { .. })
                    && !matches!(base.decision, Decision::Deny { .. })
                    && !fast.immediate_deny
                {
                    fused.decision = Decision::require_approval(
                        "local classifier advises human confirmation; model cannot deny in advisory mode",
                    );
                    fused
                        .reasons
                        .push("advisory_mode_caps_model_at_approval".to_string());
                }
                fused
            }
            ClassifierEnforcementMode::Enforce => Self::fuse_v2(base, fast, prediction, features),
        }
    }

    pub fn fuse_v2(
        base: &GuardDecision,
        fast: &FastGuardResult,
        prediction: &RiskPrediction,
        features: &AgentChainFeatureV2,
    ) -> GuardDecision {
        let mut fused = Self::fuse(base, fast, prediction, &features.v1);
        if features.alignment_score >= 0.9
            && matches!(fused.decision, Decision::Allow)
            && prediction.available
        {
            fused.decision = Decision::deny("intent and action semantics are contradictory");
            fused.risk_score = fused.risk_score.max(features.alignment_score);
            fused
                .reasons
                .push("intent_alignment_high_risk_mismatch".to_string());
            fused.evidence.push(format!(
                "alignment_score={:.2} schema={} intent={:?}",
                features.alignment_score, features.schema_version, features.intent_class
            ));
            fused.stage = GuardStage::DecisionFusion;
        }
        fused
    }

    pub fn fuse(
        base: &GuardDecision,
        fast: &FastGuardResult,
        prediction: &RiskPrediction,
        features: &AgentChainFeatureV1,
    ) -> GuardDecision {
        let mut fused = base.clone();
        fused.classifier_prediction = Some(prediction.clone());
        if !prediction.available {
            return fused;
        }

        if matches!(base.decision, Decision::Deny { .. }) || fast.immediate_deny {
            return fused;
        }

        let structural = features.sensitive_to_external_flow
            || features.external_sink_count > 0
            || features.network_egress_count > 0
            || features.retry_after_denial
            || features.alternate_tool_after_denial
            || features.denied_count > 0;
        let model_supports_deny = matches!(prediction.class, RiskClass::Critical) && structural
            || (matches!(prediction.class, RiskClass::High) && structural);
        if model_supports_deny {
            fused.decision = Decision::deny("local classifier detected high-risk behavior flow");
            fused.risk_score = fused.risk_score.max(prediction.score);
            fused.reasons.push("local_classifier_high_risk".to_string());
            fused.evidence.push(format!(
                "classifier={} class={:?} score={:.2} confidence={:.2} kind={}",
                prediction.model_version,
                prediction.class,
                prediction.score,
                prediction.confidence,
                prediction.confidence_kind
            ));
            fused.stage = GuardStage::DecisionFusion;
            return fused;
        }

        let model_requires_approval = matches!(
            prediction.class,
            RiskClass::Critical | RiskClass::High | RiskClass::Medium
        ) && prediction.score >= 0.55;
        if model_requires_approval && matches!(base.decision, Decision::Allow) {
            fused.decision =
                Decision::require_approval("local classifier requires human confirmation");
            fused.risk_score = fused.risk_score.max(prediction.score);
            fused
                .reasons
                .push("local_classifier_requires_approval".to_string());
            fused.evidence.push(format!(
                "classifier={} class={:?} score={:.2} confidence={:.2} kind={}",
                prediction.model_version,
                prediction.class,
                prediction.score,
                prediction.confidence,
                prediction.confidence_kind
            ));
            fused.stage = GuardStage::DecisionFusion;
        }
        fused
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::GuardDecision;

    fn critical_prediction() -> RiskPrediction {
        RiskPrediction {
            class: RiskClass::Critical,
            score: 0.97,
            confidence: 0.97,
            confidence_kind: crate::classifier::MARGIN_CONFIDENCE_KIND.to_string(),
            model_version: "test-model".into(),
            available: true,
        }
    }

    #[test]
    fn unavailable_model_is_observable_but_cannot_change_deterministic_decision() {
        let base = GuardDecision::allow_fast();
        let fast = FastGuardResult::allow();
        let prediction = RiskPrediction::unavailable();
        let features = AgentChainFeatureV1::default();
        let fused = DecisionFusion::fuse(&base, &fast, &prediction, &features);
        assert_eq!(fused.decision, base.decision);
        assert_eq!(fused.classifier_prediction, Some(prediction));
        assert_eq!(fused.stage, GuardStage::FastGuard);
    }

    #[test]
    fn high_risk_model_requires_supporting_sensitive_flow_before_denial() {
        let base = GuardDecision::allow_fast();
        let fast = FastGuardResult::allow();
        let prediction = RiskPrediction {
            class: RiskClass::High,
            score: 0.91,
            confidence: 0.8,
            confidence_kind: crate::classifier::MARGIN_CONFIDENCE_KIND.to_string(),
            model_version: "test-model".into(),
            available: true,
        };
        let mut features = AgentChainFeatureV1::default();
        features.sensitive_to_external_flow = true;
        let fused = DecisionFusion::fuse(&base, &fast, &prediction, &features);
        assert!(matches!(fused.decision, Decision::Deny { .. }));
        assert_eq!(fused.stage, GuardStage::DecisionFusion);
        assert!(fused.to_json()["classifier_prediction"]["available"]
            .as_bool()
            .unwrap());
    }

    #[test]
    fn shadow_advisory_and_enforce_differ_for_the_same_critical_prediction() {
        let base = GuardDecision::allow_fast();
        let fast = FastGuardResult::allow();
        let prediction = critical_prediction();
        let mut features = AgentChainFeatureV2::default();
        features.v1.sensitive_to_external_flow = true;
        let shadow = DecisionFusion::apply(
            ClassifierEnforcementMode::Shadow,
            &base,
            &fast,
            &prediction,
            &features,
        );
        assert_eq!(shadow.decision, Decision::Allow);
        let advisory = DecisionFusion::apply(
            ClassifierEnforcementMode::Advisory,
            &base,
            &fast,
            &prediction,
            &features,
        );
        assert!(matches!(
            advisory.decision,
            Decision::RequireApproval { .. }
        ));
        let enforce = DecisionFusion::apply(
            ClassifierEnforcementMode::Enforce,
            &base,
            &fast,
            &prediction,
            &features,
        );
        assert!(matches!(enforce.decision, Decision::Deny { .. }));
    }
}
