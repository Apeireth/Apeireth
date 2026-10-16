//! The post-execute correction channel: accept, replace, or block.
//!
//! After execution, the result passes through a waterfall of correctors. Each
//! corrector sees the **current** result and answers with exactly one of
//! [`PostVerdict`]:
//!
//! * `accept` — keep the result as it is and let the next corrector look;
//! * `replace` — swap in a corrected result; the original is archived in the
//!   stage record before anything else happens to it;
//! * `block` — judge the call a failure; the original is archived the same way.
//!
//! The first decisive verdict (`replace` / `block`) wins and later correctors
//! are not consulted. The channel is a pipeline for future correctors to plug
//! into; with none installed every result is accepted unchanged.

use std::sync::Arc;

use apeireth_protocol::canonical::{ToolCall, ToolResult};

use super::ExecutionStage;

/// What a corrector decides for one executed result.
#[derive(Debug, Clone, PartialEq)]
pub enum PostVerdict {
    /// Keep the executed result unchanged.
    Accept,
    /// Replace the result with a corrected one; the original is archived.
    Replace {
        /// The corrected result to emit instead.
        corrected: ToolResult,
        /// Why the correction was made; archived beside the original.
        note: String,
    },
    /// Judge the call a failure; the original is archived.
    Block {
        /// Why the call is judged a failure.
        reason: String,
    },
}

impl PostVerdict {
    /// Whether this verdict leaves the result untouched.
    pub const fn is_accept(&self) -> bool {
        matches!(self, Self::Accept)
    }

    /// Stable label for records and traces.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Replace { .. } => "replace",
            Self::Block { .. } => "block",
        }
    }
}

/// The facts a corrector may judge.
#[derive(Debug, Clone, Copy)]
pub struct PostExecuteRequest<'a> {
    /// The call that was executed.
    pub call: &'a ToolCall,
    /// The result as it stands after execution (and any earlier correction).
    pub result: &'a ToolResult,
}

impl<'a> PostExecuteRequest<'a> {
    /// A request over one call and its current result.
    pub const fn new(call: &'a ToolCall, result: &'a ToolResult) -> Self {
        Self { call, result }
    }
}

/// One corrector in the post-execute channel.
pub trait PostExecuteHook: Send + Sync {
    /// Stable name of the corrector, used in verdicts and records.
    fn name(&self) -> &str;

    /// Judge one executed result.
    fn post_verdict(&self, request: &PostExecuteRequest<'_>) -> PostVerdict;
}

/// The outcome of walking the channel.
#[derive(Debug, Clone, PartialEq)]
pub struct PostDecision {
    /// The verdict that decided.
    pub verdict: PostVerdict,
    /// The corrector that decided, absent when every corrector accepted.
    pub decided_by: Option<String>,
    /// Corrector names actually consulted, in consultation order.
    pub consulted: Vec<String>,
}

impl PostDecision {
    /// A one-line record of the decision.
    pub fn summary(&self) -> String {
        match &self.decided_by {
            Some(hook) => format!("{} by {}", self.verdict.label(), hook),
            None => self.verdict.label().to_string(),
        }
    }
}

/// The post-execute channel: correctors in order, first decisive verdict wins.
#[derive(Default)]
pub struct PostExecuteWaterfall {
    hooks: Vec<Arc<dyn PostExecuteHook>>,
}

impl PostExecuteWaterfall {
    /// An empty channel: every result is accepted unchanged.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one corrector. Consultation order equals insertion order.
    #[must_use]
    pub fn with(mut self, hook: Arc<dyn PostExecuteHook>) -> Self {
        self.hooks.push(hook);
        self
    }

    /// Number of correctors.
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    /// Whether no corrector is listening.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// Corrector names in consultation order.
    pub fn names(&self) -> Vec<&str> {
        self.hooks.iter().map(|hook| hook.name()).collect()
    }

    /// Walk the channel in order and stop at the first decisive verdict.
    pub fn evaluate(&self, request: &PostExecuteRequest<'_>) -> PostDecision {
        let mut consulted = Vec::new();
        for hook in &self.hooks {
            consulted.push(hook.name().to_string());
            let verdict = hook.post_verdict(request);
            if !verdict.is_accept() {
                return PostDecision {
                    verdict,
                    decided_by: Some(hook.name().to_string()),
                    consulted,
                };
            }
        }
        PostDecision {
            verdict: PostVerdict::Accept,
            decided_by: None,
            consulted,
        }
    }
}

/// One archived result that a later stage superseded.
///
/// Nothing is ever thrown away silently: a replaced or blocked result is kept
/// here with the stage, the actor, and the stated reason.
#[derive(Debug, Clone, PartialEq)]
pub struct SupersededResult {
    /// The stage that superseded the result.
    pub stage: ExecutionStage,
    /// The hook or contract that superseded it.
    pub hook: String,
    /// The result exactly as it was before supersession.
    pub original: ToolResult,
    /// Why it was superseded.
    pub note: String,
}
