//! One turn's input and caller-scoped tool policy.
//!
//! Keeping the request contract out of the execution driver keeps the
//! per-file architecture ratchet intact while the turn switch stays a plain
//! value the driver can pattern-match on.

use apeireth_core::kernel::{RequestId, SessionId};
use apeireth_governance::TurnSecurityContext;

/// Whether this turn may use tools at all.
///
/// A caller-scoped turn switch, not a session or task mode: a no-tools turn
/// advertises no declarations, refuses provider tool calls before dispatch,
/// and skips module/hook side effects for that turn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TurnToolPolicy {
    /// Normal turns: tools follow the session scope and task mode.
    #[default]
    Advertise,
    /// Candidate/read-only turns: tools are neither advertised nor executed.
    Disabled,
}

/// One turn's input.
#[derive(Debug, Clone)]
pub struct TurnRequest {
    /// The conversation to continue. A session that does not exist is created.
    pub session: SessionId,
    /// What the user said.
    pub input: String,
    /// Model to use, or the runtime's default when absent.
    pub model: Option<String>,
    /// System instruction, applied only when the transcript is empty.
    ///
    /// Applied once rather than per turn so that a resumed session does not
    /// accumulate duplicate system messages, which quietly change behaviour and
    /// cost tokens on every subsequent request.
    pub system: Option<String>,
    /// Immutable intent/authorization context created before provider use.
    pub security_context: Option<TurnSecurityContext>,
    /// Caller-minted turn identity; the runtime mints one when absent.
    ///
    /// Supplying it up front lets a streaming caller address the turn (stop)
    /// from the first frame onward.
    pub request: Option<RequestId>,
    /// Per-turn tool policy; normal turns advertise tools.
    pub tool_policy: TurnToolPolicy,
}

impl TurnRequest {
    /// A turn against `session` saying `input`.
    pub fn new(session: SessionId, input: impl Into<String>) -> Self {
        Self {
            session,
            input: input.into(),
            model: None,
            system: None,
            security_context: None,
            request: None,
            tool_policy: TurnToolPolicy::Advertise,
        }
    }

    /// Use a specific model for this turn.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Seed a new session with a system instruction.
    #[must_use]
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    #[must_use]
    pub fn with_security_context(mut self, context: TurnSecurityContext) -> Self {
        self.security_context = Some(context);
        self
    }

    /// Run the turn under a caller-minted request id.
    #[must_use]
    pub fn with_request(mut self, request: RequestId) -> Self {
        self.request = Some(request);
        self
    }

    /// Run the turn without advertising or executing tools.
    #[must_use]
    pub fn without_tools(mut self) -> Self {
        self.tool_policy = TurnToolPolicy::Disabled;
        self
    }

    /// Whether tools are disabled for this turn.
    pub fn tools_disabled(&self) -> bool {
        self.tool_policy == TurnToolPolicy::Disabled
    }
}
