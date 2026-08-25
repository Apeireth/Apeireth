//! Mock Biometric Provider

use crate::ha::{BiometricProvider, BiometricResult};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoercionBehavior {
    Normal,
    Coerce,
    Fail,
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct MockBiometricBehavior {
    per_human: HashMap<String, CoercionBehavior>,
    pub default_behavior: CoercionBehavior,
    pub available: bool,
}

impl Default for MockBiometricBehavior {
    fn default() -> Self {
        Self { per_human: HashMap::new(), default_behavior: CoercionBehavior::Normal, available: true }
    }
}

impl MockBiometricBehavior {
    pub fn new() -> Self { Self::default() }
    pub fn with_default(mut self, behavior: CoercionBehavior) -> Self { { self.default_behavior = behavior; self } }
    pub fn with_human(mut self, human_id: impl Into<String>, behavior: CoercionBehavior) -> Self {
        self.per_human.insert(human_id.into(), behavior); self
    }
    pub fn with_available(mut self, available: bool) -> Self { { self.available = available; self } }
    pub fn behavior_for(&self, human_id: &str) -> CoercionBehavior {
        self.per_human.get(human_id).copied().unwrap_or(self.default_behavior)
    }
}

pub struct MockBiometric {
    behavior: Mutex<MockBiometricBehavior>,
    pub provider_name: String,
}

impl MockBiometric {
    pub fn new() -> Self {
        Self { behavior: Mutex::new(MockBiometricBehavior::default()), provider_name: "mock-biometric".to_string() }
    }
    pub fn with_behavior(behavior: MockBiometricBehavior) -> Self {
        Self { behavior: Mutex::new(behavior), provider_name: "mock-biometric".to_string() }
    }
    pub fn offline() -> Self {
        Self { behavior: Mutex::new(MockBiometricBehavior::new().with_available(false)), provider_name: "mock-biometric-offline".to_string() }
    }
    pub fn set_behavior(&self, human_id: &str, behavior: CoercionBehavior) {
        let mut b = self.behavior.lock().expect("biometric poisoned");
        b.per_human.insert(human_id.to_string(), behavior);
    }
    pub fn current_behavior(&self) -> MockBiometricBehavior {
        self.behavior.lock().expect("biometric poisoned").clone()
    }
    fn now_ms(&self) -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
    }
}

impl Default for MockBiometric { fn default() -> Self { Self::new() } }

impl BiometricProvider for MockBiometric {
    fn authenticate(&self, human_id: &str) -> BiometricResult {
        let (behavior, available, at_ms) = {
            let b = self.behavior.lock().expect("biometric poisoned");
            let at = self.now_ms();
            (b.behavior_for(human_id), b.available, at)
        };
        if !available {
            return BiometricResult::Unavailable { reason: "提供者整体不可用 (离线模式)".into() };
        }
        match behavior {
            CoercionBehavior::Normal => BiometricResult::Authenticated { confidence: 0.95, at_ms: at_ms },
            CoercionBehavior::Coerce => BiometricResult::CoercionDetected { stress_level: 0.88, at_ms: at_ms },
            CoercionBehavior::Fail => BiometricResult::Failed { reason: "模拟认证失败".into(), at_ms: at_ms },
            CoercionBehavior::Unavailable => BiometricResult::Unavailable { reason: "特定 human 模拟不可用".into() },
        }
    }
    fn is_available(&self) -> bool { self.behavior.lock().expect("biometric poisoned").available }
    fn provider_name(&self) -> &str { &self.provider_name }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn mock_default_normal() {
        let b = MockBiometric::new();
        match b.authenticate("alice") {
            BiometricResult::Authenticated { confidence, .. } => assert!(confidence > 0.9),
            _ => panic!("expected Authenticated"),
        }
    }
    #[test] fn mock_offline_returns_unavailable() {
        let b = MockBiometric::offline();
        assert!(!b.is_available());
        match b.authenticate("alice") {
            BiometricResult::Unavailable { .. } => {}
            _ => panic!("expected Unavailable"),
        }
    }
    #[test] fn mock_set_behavior_per_human() {
        let b = MockBiometric::new();
        b.set_behavior("alice", CoercionBehavior::Coerce);
        match b.authenticate("alice") {
            BiometricResult::CoercionDetected { stress_level, .. } => assert!(stress_level > 0.5),
            _ => panic!("expected CoercionDetected"),
        }
    }
    #[test] fn mock_fail_returns_failed() {
        let b = MockBiometric::new();
        b.set_behavior("bob", CoercionBehavior::Fail);
        match b.authenticate("bob") {
            BiometricResult::Failed { reason, .. } => assert!(reason.contains("模拟")),
            _ => panic!("expected Failed"),
        }
    }
    #[test] fn mock_per_human_unavailable() {
        let b = MockBiometric::new();
        b.set_behavior("carol", CoercionBehavior::Unavailable);
        match b.authenticate("carol") {
            BiometricResult::Unavailable { .. } => {}
            _ => panic!("expected Unavailable"),
        }
    }
    #[test] fn current_behavior_snapshot() {
        let b = MockBiometric::new();
        let snap = b.current_behavior();
        assert!(snap.available);
        assert_eq!(snap.default_behavior, CoercionBehavior::Normal);
    }
}
