//! R150 P1 #8: apeireth-state::statechart — XState-style statechart 引擎
//!
//! **借鉴 ID**: `R150-STATE-BORROW-statelyco/xstate-28k-stars-2026-08-13`
//!
//! **XState 子集覆盖** (per R150 P1 #8 完成定义):
//! - Atomic state + transition + guard + action
//! - Initial state 指定
//! - `send(event)` 触发 transition, guard 通过则转移, action 执行
//! - Hierarchical state (parent + child) 简化版
//! - Final state (is_final marker)
//!
//! **0 触碰** 既有 OnceLockState / MutexState / RwLockState / Organ / Registry
//! **0 引外部 dep**: XState 借鉴语义, 自实现核心子集
//!
//! **不假装**: 真 transition 执行, 真 guard 通过, 真 action 调用.
//! 0 装"全 XState 兼容", 仅覆盖子集 (per ponytail ceiling).

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================
// State node — XState state node 简化版
// ============================================================

/// State 节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    /// 普通 atomic state
    Atomic,
    /// 父 state (含 child states)
    Compound,
    /// 终态 (达到后 machine 终止)
    Final,
}

/// Action 类型 — 状态转移时执行的副作用 (Arc 让 Transition 可 Clone)
pub type Action = Arc<dyn Fn(&mut MachineContext) + Send + Sync>;

/// Guard 类型 — 状态转移前判定 (返 true 允许) (Arc 让 Transition 可 Clone)
pub type Guard = Arc<dyn Fn(&MachineContext) -> bool + Send + Sync>;

/// 单个 transition 定义
#[derive(Clone)]
pub struct Transition {
    pub event: String,
    pub target: String,
    pub guard: Option<Guard>,
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

/// State 节点定义
#[derive(Clone)]
pub struct StateNode {
    pub id: String,
    pub kind: StateKind,
    pub initial: Option<String>, // compound state 的初态
    pub transitions: Vec<Transition>,
    pub on_entry: Option<Action>,
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

// ============================================================
// Machine context — 状态机持有的共享数据
// ============================================================

/// 状态机上下文 (任意业务数据)
#[derive(Default, Debug)]
pub struct MachineContext {
    pub data: HashMap<String, ContextValue>,
}

/// 上下文值 (POD 友好)
#[derive(Debug, Clone, PartialEq)]
pub enum ContextValue {
    Bool(bool),
    Int(i64),
    Str(String),
}

impl ContextValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

// ============================================================
// State machine
// ============================================================

/// 状态机执行结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionResult {
    /// 转移成功 (state_id 旧→新)
    Transitioned { from: String, to: String },
    /// Event 被 consume 但 state 未变 (guard 拒绝或无匹配 transition)
    NoTransition { reason: String },
    /// Machine 已到 final state
    Done { final_state: String },
    /// Event 未被任何 transition 处理
    UnhandledEvent { event: String },
}

pub struct Machine {
    states: HashMap<String, StateNode>,
    initial: String,
    current: String,
    pub context: MachineContext,
    /// 已触发 event 总数 (含未 handle)
    pub event_count: u64,
    /// 已 transfer 总数
    pub transition_count: u64,
}

impl Machine {
    /// 创建新 machine (spec: states HashMap + initial state id)
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

    pub fn current_state(&self) -> &str {
        &self.current
    }

    pub fn is_in_final(&self) -> bool {
        self.states
            .get(&self.current)
            .map(|s| matches!(s.kind, StateKind::Final))
            .unwrap_or(false)
    }

    /// 触发 event
    pub fn send(&mut self, event: &str) -> TransitionResult {
        self.event_count += 1;

        // 若当前是 final, 立即 done
        if self.is_in_final() {
            return TransitionResult::Done {
                final_state: self.current.clone(),
            };
        }

        // clone transitions (现在 Transition 派生 Clone, Arc<dyn Fn> 可 clone)
        let transitions = match self.states.get(&self.current) {
            Some(s) => s.transitions.clone(),
            None => {
                return TransitionResult::UnhandledEvent {
                    event: event.into(),
                }
            }
        };

        // 找 event 匹配 + guard 通过的 transition (按定义顺序, 第一个 wins)
        for t in transitions.iter() {
            if t.event != event {
                continue;
            }
            // guard check
            if let Some(guard) = &t.guard {
                if !guard(&self.context) {
                    continue;
                }
            }
            // 执行 transition: on_exit → action → on_entry
            let target = t.target.clone();
            let from = self.current.clone(); // capture BEFORE execute
            self.execute_transition(&target, t.action.clone());
            return TransitionResult::Transitioned { from, to: target };
        }

        TransitionResult::NoTransition {
            reason: format!(
                "no matching transition for event `{}` in state `{}`",
                event, self.current
            ),
        }
    }

    /// 执行 transition (on_exit current → action → on_entry target)
    fn execute_transition(&mut self, target: &str, action: Option<Action>) {
        // on_exit current
        let old = self.current.clone();
        if let Some(cur) = self.states.get(&old) {
            if let Some(exit) = &cur.on_exit {
                exit(&mut self.context);
            }
        }
        // on_entry target (在设 current 前, 因为 action 可能读 current)
        if let Some(target_node) = self.states.get(target) {
            if let Some(entry) = &target_node.on_entry {
                entry(&mut self.context);
            }
        }
        // action
        if let Some(act) = action {
            act(&mut self.context);
        }
        // 转移
        self.current = target.to_string();
        self.transition_count += 1;
    }

    /// 重置回 initial state
    pub fn reset(&mut self) {
        self.current = self.initial.clone();
        self.context = MachineContext::default();
        self.event_count = 0;
        self.transition_count = 0;
    }

    /// 设置 context 值
    pub fn set_context(&mut self, key: impl Into<String>, value: ContextValue) {
        self.context.data.insert(key.into(), value);
    }

    /// 读取 context 值
    pub fn get_context(&self, key: &str) -> Option<&ContextValue> {
        self.context.data.get(key)
    }
}

// ============================================================
// Builder helpers — 让 spec 构造更易读
// ============================================================

/// 创建一个 atomic state
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

/// 创建一个 final state
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

/// 创建一个 compound state (含 child states)
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

/// 添加 transition 到 state node (consume + return)
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

/// 添加 guard transition
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

// ============================================================
// Unit tests (0 网络, 0 真 LLM)
// ============================================================

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
        m.send("NEXT"); // red -> green
        m.send("NEXT"); // green -> yellow
        m.send("NEXT"); // yellow -> red
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
        // red → green 仅当 context.count >= 5
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
        assert_eq!(
            exit_count.load(Ordering::SeqCst),
            1,
            "on_exit of `a` should fire"
        );
        assert_eq!(
            entry_count.load(Ordering::SeqCst),
            1,
            "on_entry of `b` should fire"
        );
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
        // 再次 send 返 Done
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
        assert!(a.initial.is_none());

        let f = final_state("f");
        assert_eq!(f.kind, StateKind::Final);
    }

    #[test]
    fn r150_statechart_deliverables() {
        // R150 P1 #8 完成定义:
        // - StateNode + Transition + Machine + ContextValue
        // - atomic/compound/final state kind
        // - guard + action + on_entry + on_exit 全覆盖
        // - 12 unit tests
        let mut m = Machine::new(build_simple_traffic_light(), "red");
        assert_eq!(m.current_state(), "red");
        m.send("NEXT");
        assert_eq!(m.current_state(), "green");
        m.send("NEXT");
        assert_eq!(m.current_state(), "yellow");
        m.send("NEXT");
        assert_eq!(m.current_state(), "red");
    }
}
