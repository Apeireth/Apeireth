//! Canonical HTTP gateway adapter.
//!
//! The gateway decodes transport requests, invokes the canonical runtime, and
//! encodes responses. It does not own provider routing, tool dispatch,
//! sessions, governance, or a second orchestration engine.

#![deny(unsafe_code)]

/// Native and OpenAI-compatible HTTP chat entry points.
pub mod canonical_entry;

pub use canonical_entry::{
    canonical_router, execute_chat, resolve_approval, serve_canonical, CanonicalApprovalRequest,
    CanonicalChatOutcome, CanonicalChatRequest, CanonicalChatResponse, CanonicalEntryError,
    CanonicalPendingApproval,
};
