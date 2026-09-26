//! Repeat-call advisory — the pure advice channel for consecutive identical
//! tool calls, exposed at the assembly boundary.
//!
//! The detector, the chain-key identity, and the result-context composition are
//! one shared observation primitive in `apeireth_orchestration::repetition_advisory`;
//! this module surfaces that primitive next to the turn chain it decorates so a
//! runtime host can observe tool calls and attach reminders without touching the
//! governance path.
//!
//! Pure advice: the channel never blocks execution, never edits the tool result
//! value, and never enters a governance or audit decision. A reminder rides
//! along as clearly marked additional context after the untouched result
//! rendering.

pub use apeireth_orchestration::repetition_advisory::{
    append_result_context, canonical_arguments, chain_key, params_preview, AdvisoryLevel,
    RepetitionAdvisory, RepetitionDetector, RepetitionPolicy, RESULT_CONTEXT_MARKER,
};
