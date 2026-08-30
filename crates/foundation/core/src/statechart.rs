//! XState-subset statechart recovered from `legacy/donor/apeireth-state`.
//!
//! Covered: atomic / compound / final nodes, event transitions, guards,
//! actions, on_entry / on_exit. Hierarchical child execution is **not**
//! implemented — `compound` records an `initial` child id only.

use std::collections::HashMap;
use std::sync::Arc;

/// State node kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    /// Leaf state.
    Atomic,
    /// Parent with an `initial` child id (not auto-entered).
    Compound,
    /// Terminal state: further `send` returns [`TransitionResult::Done`].
    Final,
}

/// Side effect run on transition / entry / exit.
pub type Action = Arc<dyn Fn(&mut MachineContext) + Send + Sync>;
/// Predicate that must pass for a transition to fire.
pub type Guard = Arc<dyn Fn(&MachineContext) -> bool + Send + Sync>;

/// One event → target edge.
#[derive(Clone)]
pub struct Transition {
    /// Event name.
    pub event: String,
    /// Target state id.
    pub target: String,
    /// Optional guard.
    pub guard: Option<Guard>,
    /// Optional action (runs after on_exit / on_entry).
    pub action: Option<Action>,
}

impl std::fmt::Debug for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transition")
            .field("event", &self.event)
            .field("target", &self.target)
            .field("has_guard", &self.guard.is_some())
            .field("has_action", &self.action.is_some())
            .finish()
    }
}

/// One node in the chart.
#[derive(Clone)]
pub struct StateNode {
    /// Node id.
    pub id: String,
    /// Kind.
    pub kind: StateKind,
    /// Compound initial child.
    pub initial: Option<String>,
    /// Outgoing transitions (first matching event+guard wins).
    pub transitions: Vec<Transition>,
    /// Run when entering.
    pub on_entry: Option<Action>,
    /// Run when leaving.
    pub on_exit: Option<Action>,
}

impl std::fmt::Debug for StateNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateNode")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("initial", &self.initial)
            .field("transitions_count", &self.transitions.len())
            .field("has_on_entry", &self.on_entry.is_some())
            .field("has_on_exit", &self.on_exit.is_some())
            .finish()
    }
}

/// Shared machine data.
#[derive(Default, Debug)]
pub struct MachineContext {
    /// POD bag.
    pub data: HashMap<String, ContextValue>,
}

/// Context value.
#[derive(Debug, Clone, PartialEq)]
pub enum ContextValue {
    /// Bool.
    Bool(bool),
    /// Signed int.
    Int(i64),
    /// String.
    Str(String),
}

impl ContextValue {
    /// Bool accessor.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Int accessor.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// String accessor.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Outcome of [`Machine::send`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionResult {
    /// State changed.
    Transitioned {
        /// Previous id.
        from: String,
        /// New id.
        to: String,
    },
    /// Event matched nothing, or every guard failed.
    NoTransition {
        /// Human-readable reason.
        reason: String,
    },
    /// Machine is already in a final state.
    Done {
        /// Final id.
        final_state: String,
    },
}

/// Runnable statechart.
pub struct Machine {
    states: HashMap<String, StateNode>,
    initial: String,
    current: String,
    /// Shared context.
    pub context: MachineContext,
    /// Events received, including unhandled.
    pub event_count: u64,
    /// Successful transfers.
    pub transition_count: u64,
}

impl Machine {
    /// `states` keyed by node id; `initial` must exist (not checked).
    pub fn new(states: HashMap<String, StateNode>, initial: impl Into<String>) -> Self {
        let initial = initial.into();
        Self {
            states,
            initial: initial.clone(),
            current: initial,
            context: MachineContext::default(),
            event_count: 0,
            transition_count: 0,
        }
    }

    /// Current state id.
    pub fn current_state(&self) -> &str {
        &self.current
    }

    /// Whether the current node is [`StateKind::Final`].
    pub fn is_in_final(&self) -> bool {
        self.states
            .get(&self.current)
            .map(|s| matches!(s.kind, StateKind::Final))
            .unwrap_or(false)
    }

    /// Dispatch an event.
    pub fn send(&mut self, event: &str) -> TransitionResult {
        self.event_count += 1;
        if self.is_in_final() {
            return TransitionResult::Done {
                final_state: self.current.clone(),
            };
        }
        let transitions = match self.states.get(&self.current) {
            Some(s) => s.transitions.clone(),
            None => {
                return TransitionResult::NoTransition {
                    reason: format!("unknown state `{}`", self.current),
                };
            }
        };
        for t in transitions.iter() {
            if t.event != event {
                continue;
            }
            if let Some(guard) = &t.guard {
                if !guard(&self.context) {
                    continue;
                }
            }
            let target = t.target.clone();
            let from = self.current.clone();
            self.execute_transition(&target, t.action.clone());
            return TransitionResult::Transitioned { from, to: target };
        }
        TransitionResult::NoTransition {
            reason: format!(
                "no matching transition for event `{event}` in state `{}`",
                self.current
            ),
        }
    }

    fn execute_transition(&mut self, target: &str, action: Option<Action>) {
        let old = self.current.clone();
        if let Some(cur) = self.states.get(&old) {
            if let Some(exit) = &cur.on_exit {
                exit(&mut self.context);
            }
        }
        if let Some(target_node) = self.states.get(target) {
            if let Some(entry) = &target_node.on_entry {
                entry(&mut self.context);
            }
        }
        if let Some(act) = action {
            act(&mut self.context);
        }
        self.current = target.to_string();
        self.transition_count += 1;
    }

    /// Return to the initial state and clear context / counters.
    pub fn reset(&mut self) {
        self.current = self.initial.clone();
        self.context = MachineContext::default();
        self.event_count = 0;
        self.transition_count = 0;
    }

    /// Write a context value.
    pub fn set_context(&mut self, key: impl Into<String>, value: ContextValue) {
        self.context.data.insert(key.into(), value);
    }

    /// Read a context value.
    pub fn get_context(&self, key: &str) -> Option<&ContextValue> {
        self.context.data.get(key)
    }
}

/// Atomic helper.
pub fn atomic_state(id: impl Into<String>) -> StateNode {
    StateNode {
        id: id.into(),
        kind: StateKind::Atomic,
        initial: None,
        transitions: Vec::new(),
        on_entry: None,
        on_exit: None,
    }
}

/// Final helper.
pub fn final_state(id: impl Into<String>) -> StateNode {
    StateNode {
        id: id.into(),
        kind: StateKind::Final,
        initial: None,
        transitions: Vec::new(),
        on_entry: None,
        on_exit: None,
    }
}

/// Compound helper.
pub fn compound_state(id: impl Into<String>, initial: impl Into<String>) -> StateNode {
    StateNode {
        id: id.into(),
        kind: StateKind::Compound,
        initial: Some(initial.into()),
        transitions: Vec::new(),
        on_entry: None,
        on_exit: None,
    }
}

/// Unguarded transition helper.
pub fn with_transition(
    mut state: StateNode,
    event: impl Into<String>,
    target: impl Into<String>,
) -> StateNode {
    state.transitions.push(Transition {
        event: event.into(),
        target: target.into(),
        guard: None,
        action: None,
    });
    state
}

/// Guarded transition helper.
pub fn with_guarded_transition(
    mut state: StateNode,
    event: impl Into<String>,
    target: impl Into<String>,
    guard: Guard,
) -> StateNode {
    state.transitions.push(Transition {
        event: event.into(),
        target: target.into(),
        guard: Some(guard),
        action: None,
    });
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn build_simple_traffic_light() -> HashMap<String, StateNode> {
        let mut states = HashMap::new();
        states.insert(
            "red".into(),
            with_transition(atomic_state("red"), "NEXT", "green"),
        );
        states.insert(
            "green".into(),
            with_transition(atomic_state("green"), "NEXT", "yellow"),
        );
        states.insert(
            "yellow".into(),
            with_transition(atomic_state("yellow"), "NEXT", "red"),
        );
        states
    }

    #[test]
    fn machine_initial_state() {
        let m = Machine::new(build_simple_traffic_light(), "red");
        assert_eq!(m.current_state(), "red");
        assert!(!m.is_in_final());
    }

    #[test]
    fn machine_sends_event_transitions() {
        let mut m = Machine::new(build_simple_traffic_light(), "red");
        let r = m.send("NEXT");
        assert_eq!(
            r,
            TransitionResult::Transitioned {
                from: "red".into(),
                to: "green".into()
            }
        );
        assert_eq!(m.current_state(), "green");
        assert_eq!(m.transition_count, 1);
    }

    #[test]
    fn machine_cycles_through_states() {
        let mut m = Machine::new(build_simple_traffic_light(), "red");
        m.send("NEXT");
        m.send("NEXT");
        m.send("NEXT");
        assert_eq!(m.current_state(), "red");
        assert_eq!(m.transition_count, 3);
        assert_eq!(m.event_count, 3);
    }

    #[test]
    fn machine_unhandled_event() {
        let mut m = Machine::new(build_simple_traffic_light(), "red");
        let r = m.send("UNKNOWN");
        assert!(matches!(r, TransitionResult::NoTransition { .. }));
        assert_eq!(m.current_state(), "red");
        assert_eq!(m.event_count, 1);
        assert_eq!(m.transition_count, 0);
    }

    #[test]
    fn machine_guard_rejects_transition() {
        let mut states = HashMap::new();
        let guard: Guard =
            Arc::new(|ctx| ctx.data.get("count").and_then(|v| v.as_int()).unwrap_or(0) >= 5);
        states.insert(
            "red".into(),
            with_guarded_transition(atomic_state("red"), "NEXT", "green", guard),
        );
        states.insert("green".into(), atomic_state("green"));
        let mut m = Machine::new(states, "red");
        m.set_context("count", ContextValue::Int(3));
        let r = m.send("NEXT");
        assert!(matches!(r, TransitionResult::NoTransition { .. }));
        assert_eq!(m.current_state(), "red");
        m.set_context("count", ContextValue::Int(5));
        let r = m.send("NEXT");
        assert!(matches!(r, TransitionResult::Transitioned { .. }));
        assert_eq!(m.current_state(), "green");
    }

    #[test]
    fn machine_action_invoked_on_transition() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let action: Action = Arc::new(move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });
        let mut states = HashMap::new();
        states.insert(
            "a".into(),
            StateNode {
                id: "a".into(),
                kind: StateKind::Atomic,
                initial: None,
                transitions: vec![Transition {
                    event: "GO".into(),
                    target: "b".into(),
                    guard: None,
                    action: Some(action),
                }],
                on_entry: None,
                on_exit: None,
            },
        );
        states.insert("b".into(), atomic_state("b"));
        let mut m = Machine::new(states, "a");
        m.send("GO");
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn machine_on_entry_on_exit_invoked() {
        let entry_count = Arc::new(AtomicU32::new(0));
        let exit_count = Arc::new(AtomicU32::new(0));
        let ec = entry_count.clone();
        let xc = exit_count.clone();
        let on_entry: Action = Arc::new(move |_| {
            ec.fetch_add(1, Ordering::SeqCst);
        });
        let on_exit: Action = Arc::new(move |_| {
            xc.fetch_add(1, Ordering::SeqCst);
        });
        let mut states = HashMap::new();
        let mut a = atomic_state("a");
        a.on_exit = Some(on_exit);
        let mut b = atomic_state("b");
        b.on_entry = Some(on_entry);
        states.insert("a".into(), with_transition(a, "GO", "b"));
        states.insert("b".into(), b);
        let mut m = Machine::new(states, "a");
        m.send("GO");
        assert_eq!(exit_count.load(Ordering::SeqCst), 1);
        assert_eq!(entry_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn machine_final_state_terminates() {
        let mut states = HashMap::new();
        states.insert(
            "a".into(),
            with_transition(atomic_state("a"), "FINISH", "end"),
        );
        states.insert("end".into(), final_state("end"));
        let mut m = Machine::new(states, "a");
        m.send("FINISH");
        assert!(m.is_in_final());
        let r = m.send("ANY");
        assert!(matches!(r, TransitionResult::Done { .. }));
    }

    #[test]
    fn machine_compound_state_has_initial() {
        let s = compound_state("parent", "child1");
        assert_eq!(s.kind, StateKind::Compound);
        assert_eq!(s.initial, Some("child1".into()));
    }

    #[test]
    fn machine_reset_clears_state_and_context() {
        let mut m = Machine::new(build_simple_traffic_light(), "red");
        m.set_context("count", ContextValue::Int(42));
        m.send("NEXT");
        assert_eq!(m.current_state(), "green");
        m.reset();
        assert_eq!(m.current_state(), "red");
        assert!(m.get_context("count").is_none());
        assert_eq!(m.transition_count, 0);
    }

    #[test]
    fn context_value_introspection() {
        assert_eq!(ContextValue::Bool(true).as_bool(), Some(true));
        assert_eq!(ContextValue::Bool(true).as_int(), None);
        assert_eq!(ContextValue::Int(42).as_int(), Some(42));
        assert_eq!(ContextValue::Int(42).as_str(), None);
        assert_eq!(ContextValue::Str("hi".into()).as_str(), Some("hi"));
    }

    #[test]
    fn atomic_and_final_state_helpers() {
        let a = atomic_state("a");
        assert_eq!(a.id, "a");
        assert_eq!(a.kind, StateKind::Atomic);
        let f = final_state("f");
        assert_eq!(f.kind, StateKind::Final);
    }
}
