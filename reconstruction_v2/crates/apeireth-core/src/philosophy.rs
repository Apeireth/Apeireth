use std::collections::HashMap;

/// The 13 foundational philosophy keys of Apeireth
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PhilosophyKey {

    K1_ApeironEmergence,
    K2_ZeroPretending,
    K3_TenantModel,
    K4_MechanismsOverPatches,
    K5_SovereignDignity,
    K6_EpistemicHonesty,
    K7_CausalTransparency,
    K8_AntiSycophancy,
    K9_MemoryIntegrity,
    K10_BoundaryContainment,
    K11_ReflectiveAudit,
    K12_TemporalContinuity,
    K13_GracefulDegradation,
}

pub const EIGHT_ANCHORS: [&str; 8] = [
    "A1: Authenticity (0 Pretend)",
    "A2: Tenant Sovereign Containment",
    "A3: Epistemic Uncertainty Awareness",
    "A4: Immutable Audit Traceability",
    "A5: Dynamic PAD Emotional Resonance",
    "A6: Causal Graph Interpretability",
    "A7: Multi-Process Sleep/Rhythm Balance",
    "A8: Default-Deny Physical Security",
];

pub const REFLECTION_WHITELIST: &[&str] = &[
    "self_diagnosis",
    "memory_consolidation",
    "dream_synthesis",
    "intent_calibration",
    "drift_correction",
];

pub const META_FORBIDDEN_PATTERNS: &[&str] = &[
    "pretend_to_feel",
    "bypass_gate",
    "suppress_audit",
    "fabricate_memory",
    "privilege_escalation",
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Verdict {
    pub key: PhilosophyKey,
    pub allowed: bool,
    pub rationale: String,
}

#[derive(Default)]
pub struct VerdictCache {
    cache: HashMap<(PhilosophyKey, String), Verdict>,
}

impl VerdictCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn insert(&mut self, action_name: &str, verdict: Verdict) {
        self.cache.insert((verdict.key, action_name.to_string()), verdict);
    }

    pub fn get(&self, key: &PhilosophyKey, action_name: &str) -> Option<&Verdict> {
        self.cache.get(&(*key, action_name.to_string()))
    }

    pub fn evaluate_action(&mut self, key: PhilosophyKey, action_name: &str) -> Verdict {
        if let Some(v) = self.get(&key, action_name) {
            return v.clone();
        }

        let is_forbidden = META_FORBIDDEN_PATTERNS.iter().any(|&pat| action_name.contains(pat));
        let verdict = if is_forbidden {
            Verdict {
                key,
                allowed: false,
                rationale: format!("Action '{}' violated forbidden pattern under {:?}", action_name, key),
            }
        } else {
            Verdict {
                key,
                allowed: true,
                rationale: format!("Action '{}' aligned with philosophy {:?}", action_name, key),
            }
        };

        self.insert(action_name, verdict.clone());
        verdict
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchors_and_keys() {
        assert_eq!(EIGHT_ANCHORS.len(), 8);
        assert_eq!(REFLECTION_WHITELIST.len(), 5);
        assert_eq!(META_FORBIDDEN_PATTERNS.len(), 5);
    }

    #[test]
    fn test_verdict_cache_and_evaluation() {
        let mut cache = VerdictCache::new();
        
        let v1 = cache.evaluate_action(PhilosophyKey::K2_ZeroPretending, "normal_reply");
        assert!(v1.allowed);

        let v2 = cache.evaluate_action(PhilosophyKey::K2_ZeroPretending, "pretend_to_feel_happy");
        assert!(!v2.allowed);

        // Cached retrieval
        let cached = cache.get(&PhilosophyKey::K2_ZeroPretending, "pretend_to_feel_happy").unwrap();
        assert_eq!(cached.allowed, v2.allowed);
    }

}

