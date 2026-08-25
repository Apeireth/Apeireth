//! Skill execution layer (5 phase state machines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPhase { Plan, Verify, Parallel, Review, Meta }

pub struct SkillExecutor {
    pub max_phases: usize,
}

impl SkillExecutor {
    pub fn new() -> Self { Self { max_phases: 5 } }
    pub fn execute<F: FnMut(ExecutionPhase) -> bool>(&self, mut f: F) -> usize {
        let phases = [
            ExecutionPhase::Plan, ExecutionPhase::Verify,
            ExecutionPhase::Parallel, ExecutionPhase::Review, ExecutionPhase::Meta,
        ];
        let mut count = 0;
        for p in phases.iter().take(self.max_phases) {
            if f(*p) { count += 1; }
        }
        count
    }
}

impl Default for SkillExecutor {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs_5_phases() {
        let e = SkillExecutor::new();
        let c = e.execute(|_| true);
        assert_eq!(c, 5);
    }
}
