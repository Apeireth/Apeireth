//! ExperimentField - 实验场 (从 v1.0 apeireth-companion/experiment_field.rs 282 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 ExperimentState + rollback signal
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentState { Proposed, Running, Passed, Failed, RolledBack }

pub struct Experiment { pub name: String, pub state: ExperimentState, pub result: Option<String> }

pub struct ExperimentField {
    pub experiments: HashMap<String, Experiment>,
}

impl ExperimentField {
    pub fn new() -> Self { Self { experiments: HashMap::new() } }

    /// 0 装 PASS: 真 propose
    pub fn propose(&mut self, name: impl Into<String>) {
        let name = name.into();
        self.experiments.insert(name.clone(), Experiment { name, state: ExperimentState::Proposed, result: None });
    }

    /// 0 装 PASS: 真 transition
    pub fn transition(&mut self, name: &str, target: ExperimentState) -> bool {
        if let Some(e) = self.experiments.get_mut(name) {
            e.state = target;
            true
        } else { false }
    }

    /// 0 装 PASS: 真记录结果
    pub fn record(&mut self, name: &str, result: impl Into<String>) -> bool {
        if let Some(e) = self.experiments.get_mut(name) {
            e.result = Some(result.into());
            true
        } else { false }
    }

    /// 0 装 PASS: 真回滚信号
    pub fn rollback_signal(&self) -> bool {
        self.experiments.values().any(|e| e.state == ExperimentState::Failed)
    }
}

impl Default for ExperimentField { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_lifecycle() {
        let mut f = ExperimentField::new();
        f.propose("e1");
        f.transition("e1", ExperimentState::Running);
        f.transition("e1", ExperimentState::Passed);
        assert!(!f.rollback_signal());
    }
    #[test] fn test_record() {
        let mut f = ExperimentField::new();
        f.propose("e1");
        f.record("e1", "success");
        assert!(!f.rollback_signal());
    }
    #[test] fn test_rollback() {
        let mut f = ExperimentField::new();
        f.propose("e1");
        f.transition("e1", ExperimentState::Failed);
        assert!(f.rollback_signal());
    }
    #[test] fn test_unknown() {
        let mut f = ExperimentField::new();
        assert!(!f.transition("missing", ExperimentState::Running));
    }
    #[test] fn test_default() {
        let f: ExperimentField = Default::default();
        assert!(!f.rollback_signal());
    }
}
