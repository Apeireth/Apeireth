//! `apeireth-vcp-bridge` - R141 VCP protocol bridge (4-protocol 5→1 merge per v2 plan §9.5).
//!
//! VCP had 4 separate protocol adapters (OpenAI / Anthropic / Gemini / Responses) and 5 plugins
//! (vcp-protocol-*). We merge into 1 cohesive bridge that:
//! 1. Detects incoming protocol from headers
//! 2. Routes to apeireth-api's unified handler
//! 3. Converts response back to original protocol
//! 4. Audit log (each conversion recorded)
//!
//! Honest scope (per O-5 不假装):
//! - Protocol detection based on URL path + header heuristics
//! - Conversion functions for 4 protocols (OpenAI Chat Completions / Anthropic Messages /
//!   OpenAI Responses / Gemini)
//! - Real HTTP routing delegates to `apeireth-api`
//! - Audit log is in-memory (not persisted)

#![warn(missing_docs)]

pub mod protocol;
pub mod detect;
pub mod convert;
pub mod audit;
pub mod bridge;
pub mod mcp;
pub mod vcp_compat;
pub mod enhanced;

pub use protocol::{VcpProtocol, ProtocolHints};
pub use detect::detect_protocol;
pub use convert::{convert_request, convert_response};
pub use audit::{AuditLog, AuditEntry};
pub use bridge::{VcpBridge, BridgeError};
pub use mcp::{VcpBridgeMcp, BridgeTool};
pub use vcp_compat::{VcpBridgeCommand, VcpBridgeRouter, VCP_BRIDGE_COMMAND_COUNT};
pub use enhanced::EnhancedVcpBridge;

/// R141 deliverables for vcp-bridge:
/// - 4 modules (protocol / detect / convert / audit) + bridge + mcp + vcp_compat + enhanced
/// - 5→1 merge per v2 plan §9.5
pub const R141_VCP_BRIDGE_DELIVERABLES: usize = 8;