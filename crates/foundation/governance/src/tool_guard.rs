//! The monotonic tool guard interface: a guard may refuse, never allow.
//!
//! # Monotonicity is the safety property
//!
//! A guard answers exactly one question: *do you refuse this call?* The answer
//! is [`Option<String>`] — a refusal reason, or "this guard has no objection".
//! The vocabulary has **no allow verdict**: no guard, and no later listener,
//! can express "this call is allowed". A refusal therefore can never be turned
//! back into an allow by whatever runs after it, and the listening order is
//! irreversible: a refusal is final the moment it appears, and every later
//! guard can only add further refusals.
//!
//! # Ownership
//!
//! This module is an interface plus a deny-only sequence container. It does not
//! execute tools, does not own the approval lifecycle, and never returns the
//! runtime's action verdicts. The tool execution chain consults it as one of
//! its stages; producing the reason strings is the guards' own business.

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use serde_json::Value;

/// The immutable facts one guard may judge before a tool executes.
#[derive(Debug, Clone, Copy)]
pub struct ToolGuardRequest<'a> {
    /// Stable capability identity of the tool about to run.
    pub capability: &'a CapabilityId,
    /// Model-facing name of the call being executed.
    pub tool_name: &'a str,
    /// The arguments produced for this call.
    pub arguments: &'a Value,
}

impl<'a> ToolGuardRequest<'a> {
    /// A request over the given capability identity, tool name, and arguments.
    pub const fn new(
        capability: &'a CapabilityId,
        tool_name: &'a str,
        arguments: &'a Value,
    ) -> Self {
        Self {
            capability,
            tool_name,
            arguments,
        }
    }
}

/// A monotonic guard: it may only refuse, never allow.
pub trait ToolGuard: Send + Sync {
    /// Stable name of the guard, used in refusals and audit records.
    fn name(&self) -> &str;

    /// Return `Some(reason)` to refuse the call.
    ///
    /// `None` means "this guard has no objection". It is never an allow
    /// verdict: the interface has no way to express one, so nothing downstream
    /// can mistake a passing guard for permission.
    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String>;
}

/// One refusal: which guard refused, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardRefusal {
    /// Name of the refusing guard.
    pub guard: String,
    /// The refusal reason exactly as the guard stated it.
    pub reason: String,
}

impl GuardRefusal {
    /// A refusal attributed to `guard` with `reason`.
    pub fn new(guard: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            guard: guard.into(),
            reason: reason.into(),
        }
    }
}

/// An ordered, deny-only guard sequence.
///
/// Guards are listened to in the order they were added; that order is fixed for
/// the life of the set and is reported by [`MonotonicGuards::names`]. Every
/// guard is consulted on every call so no objection is silently dropped, but
/// the reported refusal is always the **first** one — a later guard can never
/// revoke it, because nothing here accepts or returns an allow.
#[derive(Default)]
pub struct MonotonicGuards {
    guards: Vec<Arc<dyn ToolGuard>>,
}

impl MonotonicGuards {
    /// An empty set: nothing refuses, nothing allows.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one guard. Listening order equals insertion order.
    #[must_use]
    pub fn with(mut self, guard: Arc<dyn ToolGuard>) -> Self {
        self.guards.push(guard);
        self
    }

    /// Append every guard of `other`, after the guards already listening.
    #[must_use]
    pub fn merge(mut self, other: Self) -> Self {
        self.guards.extend(other.guards);
        self
    }

    /// Number of guards listening.
    pub fn len(&self) -> usize {
        self.guards.len()
    }

    /// Whether no guard is listening.
    pub fn is_empty(&self) -> bool {
        self.guards.is_empty()
    }

    /// Guard names in listening order.
    pub fn names(&self) -> Vec<&str> {
        self.guards.iter().map(|guard| guard.name()).collect()
    }

    /// Listen to every guard in order and return **all** refusals, in
    /// listening order.
    ///
    /// Refusals only accumulate: consulting a later guard cannot remove an
    /// earlier one.
    pub fn refusals(&self, request: &ToolGuardRequest<'_>) -> Vec<GuardRefusal> {
        self.guards
            .iter()
            .filter_map(|guard| {
                guard
                    .deny(request)
                    .map(|reason| GuardRefusal::new(guard.name(), reason))
            })
            .collect()
    }

    /// Listen to every guard in order and report the first refusal.
    ///
    /// Later guards are still consulted (their objections land in
    /// [`MonotonicGuards::refusals`]), but the first refusal is the verdict and
    /// nothing that runs afterwards can flip it back to allowed.
    pub fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<GuardRefusal> {
        self.refusals(request).into_iter().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FixedGuard {
        name: &'static str,
        refusal: Option<&'static str>,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl FixedGuard {
        fn new(
            name: &'static str,
            refusal: Option<&'static str>,
            log: &Arc<Mutex<Vec<&'static str>>>,
        ) -> Arc<Self> {
            Arc::new(Self {
                name,
                refusal,
                log: Arc::clone(log),
            })
        }
    }

    impl ToolGuard for FixedGuard {
        fn name(&self) -> &str {
            self.name
        }

        fn deny(&self, _request: &ToolGuardRequest<'_>) -> Option<String> {
            self.log.lock().unwrap().push(self.name);
            self.refusal.map(str::to_string)
        }
    }

    fn request<'a>(capability: &'a CapabilityId, arguments: &'a Value) -> ToolGuardRequest<'a> {
        ToolGuardRequest::new(capability, "tool.demo", arguments)
    }

    #[test]
    fn a_refusal_is_never_flipped_by_a_later_guard() {
        let capability = CapabilityId::new("tool.demo").unwrap();
        let arguments = Value::Null;
        let log = Arc::new(Mutex::new(Vec::new()));

        let refusing_first = MonotonicGuards::new()
            .with(FixedGuard::new("refusing", Some("no"), &log))
            .with(FixedGuard::new("silent", None, &log));
        assert_eq!(
            refusing_first.deny(&request(&capability, &arguments)),
            Some(GuardRefusal::new("refusing", "no")),
            "a later silent guard must not flip an earlier refusal"
        );

        let refusing_last = MonotonicGuards::new()
            .with(FixedGuard::new("silent", None, &log))
            .with(FixedGuard::new("refusing", Some("no"), &log));
        assert_eq!(
            refusing_last.deny(&request(&capability, &arguments)),
            Some(GuardRefusal::new("refusing", "no")),
            "the listening order is irreversible; a refusal is a refusal wherever it sits"
        );
    }

    #[test]
    fn guards_are_listened_in_their_fixed_order() {
        let capability = CapabilityId::new("tool.demo").unwrap();
        let arguments = Value::Null;
        let log = Arc::new(Mutex::new(Vec::new()));
        let guards = MonotonicGuards::new()
            .with(FixedGuard::new("first", None, &log))
            .with(FixedGuard::new("second", Some("stop"), &log))
            .with(FixedGuard::new("third", Some("also stop"), &log));

        assert_eq!(guards.names(), ["first", "second", "third"]);
        let refusals = guards.refusals(&request(&capability, &arguments));
        assert_eq!(
            refusals,
            vec![
                GuardRefusal::new("second", "stop"),
                GuardRefusal::new("third", "also stop")
            ],
            "refusals accumulate in listening order; the first one is the verdict"
        );
        assert_eq!(
            guards.deny(&request(&capability, &arguments)),
            refusals.first().cloned()
        );
        assert_eq!(
            *log.lock().unwrap(),
            ["first", "second", "third", "first", "second", "third"],
            "every guard listens on every call, in the same fixed order"
        );
    }

    #[test]
    fn no_guard_objection_is_not_an_allow() {
        let capability = CapabilityId::new("tool.demo").unwrap();
        let arguments = Value::Null;
        let log = Arc::new(Mutex::new(Vec::new()));
        let empty = MonotonicGuards::new();
        assert!(empty.is_empty());
        assert_eq!(empty.deny(&request(&capability, &arguments)), None);

        let silent = MonotonicGuards::new().with(FixedGuard::new("silent", None, &log));
        assert_eq!(silent.deny(&request(&capability, &arguments)), None);
        assert_eq!(silent.refusals(&request(&capability, &arguments)), []);
    }
}
