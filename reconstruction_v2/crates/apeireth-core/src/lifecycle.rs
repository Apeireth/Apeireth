/// The 9 discrete lifecycle phases of the Apeireth cognitive loop
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LifecyclePhase {

    P1_BootAndIntegrityCheck,
    P2_PerceptionAndObservation,
    P3_MemoryRetrievalAndACTR,
    P4_GovernanceAndGateEvaluation,
    P5_WorldModelAndMctsSimulation,
    P6_PromptAssemblyAndProtocolDispatch,
    P7_StreamingExecutionAndCoTDecompose,
    P8_BrierCalibrationAndMemoryConsolidation,
    P9_SleepAndDreamSynthesis,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum LifecycleError {
    #[error("Invalid phase transition from {0:?} to {1:?}")]
    InvalidTransition(LifecyclePhase, LifecyclePhase),
    #[error("Lifecycle execution aborted: {0}")]
    Aborted(String),
}

pub struct LifecycleStateMachine {
    current_phase: LifecyclePhase,
    history: Vec<LifecyclePhase>,
}

impl Default for LifecycleStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleStateMachine {
    pub fn new() -> Self {
        Self {
            current_phase: LifecyclePhase::P1_BootAndIntegrityCheck,
            history: vec![LifecyclePhase::P1_BootAndIntegrityCheck],
        }
    }

    pub fn current_phase(&self) -> LifecyclePhase {
        self.current_phase
    }

    pub fn transition_to(&mut self, next: LifecyclePhase) -> Result<(), LifecycleError> {
        let is_valid = match (self.current_phase, next) {
            (LifecyclePhase::P1_BootAndIntegrityCheck, LifecyclePhase::P2_PerceptionAndObservation) => true,
            (LifecyclePhase::P2_PerceptionAndObservation, LifecyclePhase::P3_MemoryRetrievalAndACTR) => true,
            (LifecyclePhase::P3_MemoryRetrievalAndACTR, LifecyclePhase::P4_GovernanceAndGateEvaluation) => true,
            (LifecyclePhase::P4_GovernanceAndGateEvaluation, LifecyclePhase::P5_WorldModelAndMctsSimulation) => true,
            (LifecyclePhase::P5_WorldModelAndMctsSimulation, LifecyclePhase::P6_PromptAssemblyAndProtocolDispatch) => true,
            (LifecyclePhase::P6_PromptAssemblyAndProtocolDispatch, LifecyclePhase::P7_StreamingExecutionAndCoTDecompose) => true,
            (LifecyclePhase::P7_StreamingExecutionAndCoTDecompose, LifecyclePhase::P8_BrierCalibrationAndMemoryConsolidation) => true,
            (LifecyclePhase::P8_BrierCalibrationAndMemoryConsolidation, LifecyclePhase::P9_SleepAndDreamSynthesis) => true,
            (LifecyclePhase::P9_SleepAndDreamSynthesis, LifecyclePhase::P2_PerceptionAndObservation) => true,
            // Direct cycle shortcut
            (LifecyclePhase::P8_BrierCalibrationAndMemoryConsolidation, LifecyclePhase::P2_PerceptionAndObservation) => true,
            _ => false,
        };

        if is_valid {
            self.current_phase = next;
            self.history.push(next);
            Ok(())
        } else {
            Err(LifecycleError::InvalidTransition(self.current_phase, next))
        }
    }

    pub fn history(&self) -> &[LifecyclePhase] {
        &self.history
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_lifecycle_cycle() {
        let mut sm = LifecycleStateMachine::new();
        assert_eq!(sm.current_phase(), LifecyclePhase::P1_BootAndIntegrityCheck);

        assert!(sm.transition_to(LifecyclePhase::P2_PerceptionAndObservation).is_ok());
        assert!(sm.transition_to(LifecyclePhase::P3_MemoryRetrievalAndACTR).is_ok());
        assert!(sm.transition_to(LifecyclePhase::P4_GovernanceAndGateEvaluation).is_ok());
        assert!(sm.transition_to(LifecyclePhase::P5_WorldModelAndMctsSimulation).is_ok());
        assert!(sm.transition_to(LifecyclePhase::P6_PromptAssemblyAndProtocolDispatch).is_ok());
        assert!(sm.transition_to(LifecyclePhase::P7_StreamingExecutionAndCoTDecompose).is_ok());
        assert!(sm.transition_to(LifecyclePhase::P8_BrierCalibrationAndMemoryConsolidation).is_ok());
        assert!(sm.transition_to(LifecyclePhase::P9_SleepAndDreamSynthesis).is_ok());
        assert!(sm.transition_to(LifecyclePhase::P2_PerceptionAndObservation).is_ok());
    }

    #[test]
    fn test_invalid_lifecycle_transition() {
        let mut sm = LifecycleStateMachine::new();
        // Illegal jump from P1 directly to P7
        let err = sm.transition_to(LifecyclePhase::P7_StreamingExecutionAndCoTDecompose);
        assert!(err.is_err());
    }
}

