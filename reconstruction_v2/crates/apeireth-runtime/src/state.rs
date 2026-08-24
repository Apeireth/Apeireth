//! State - 状态机框架 (从 v1.0 apeireth-state 4K LOC 收敛)
//!
//! 0 装 PASS: 简化版 state machine (transition table + guard), 完整 v1.0 era 不做 (Harel statecharts).

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: StateId,
    pub to: StateId,
    pub event: EventId,
}

#[derive(Debug, Clone, Default)]
pub struct StateMachine {
    pub current: StateId,
    pub transitions: HashMap<(StateId, EventId), StateId>,
    pub history: Vec<StateId>,
}

impl StateMachine {
    pub fn new(initial: StateId) -> Self {
        Self { current: initial.clone(), transitions: HashMap::new(), history: vec![initial] }
    }

    pub fn add_transition(&mut self, t: Transition) {
        self.transitions.insert((t.from, t.event), t.to);
    }

    /// 0 装 PASS: 真实状态转移 (无 transition 时返 None, 不假装)
    pub fn fire(&mut self, event: &EventId) -> Option<&StateId> {
        if let Some(next) = self.transitions.get(&(self.current.clone(), event.clone())) {
            self.current = next.clone();
            self.history.push(next.clone());
            Some(&self.current)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_state_machine_fsm() {
        let mut sm = StateMachine::new(StateId("Idle".into()));
        sm.add_transition(Transition { from: StateId("Idle".into()), to: StateId("Running".into()), event: EventId("start".into()) });
        sm.add_transition(Transition { from: StateId("Running".into()), to: StateId("Done".into()), event: EventId("stop".into()) });
        assert_eq!(sm.current.0, "Idle");
        assert_eq!(sm.fire(&EventId("start".into())).unwrap().0, "Running");
        assert_eq!(sm.fire(&EventId("stop".into())).unwrap().0, "Done");
    }
    #[test] fn test_invalid_event() {
        let mut sm = StateMachine::new(StateId("Idle".into()));
        assert_eq!(sm.fire(&EventId("invalid".into())), None);
    }
}
