//! The mesh wire envelope and its message set.
//!
//! One envelope for everything, so correlation, tracing, and versioning are
//! solved once:
//!
//! ```text
//! MeshEnvelope {
//!     protocol_version,          // fail closed on mismatch, never downgrade
//!     message_id,                // RequestId: this message's identity (Cancel handle)
//!     in_reply_to,               // correlates a response to its request
//!     trace_id,                  // TraceId survives the network hop unchanged
//!     source_node / target_node, // NodeId; post-handshake this must match the peer
//!     payload,                   // Hello | HelloAck | CapabilityAdvert |
//!                                // InvokeCapability | CapabilityResult |
//!                                // CapabilityError | Heartbeat | Cancel
//! }
//! ```
//!
//! The invocation payloads wrap the *canonical* [`ToolCall`] / [`ToolResult`]
//! as-is. There is deliberately no `MeshToolRequest` / `MeshToolResult`: a
//! remote tool call must be indistinguishable in shape from a local one, so the
//! network cannot grow a second tool contract alongside the real one.

use apeireth_core::kernel::{CapabilityId, RequestId, SessionId, TraceId};
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use serde::{Deserialize, Serialize};

use crate::identity::{CapabilityAdvertEntry, MeshCapabilityKind, ModuleAdvert, NodeDescriptor};

/// The mesh wire protocol version this build speaks.
///
/// Handshake requires an exact match: a version mismatch must fail closed
/// rather than silently downgrade to whatever the peer happens to speak.
pub const MESH_PROTOCOL_VERSION: u32 = 1;

/// Failure decoding, encoding, or validating a wire message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MeshProtocolError {
    /// The envelope could not be encoded.
    #[error("mesh envelope could not be encoded: {0}")]
    Encode(String),
    /// The envelope could not be decoded.
    #[error("mesh envelope could not be decoded: {0}")]
    Decode(String),
    /// The peer speaks a different wire protocol version. Fail closed.
    #[error("mesh protocol version mismatch: local {local}, remote {remote}")]
    ProtocolVersionMismatch {
        /// This node's version.
        local: u32,
        /// The peer's version.
        remote: u32,
    },
    /// The peer sent an empty runtime version; compatibility cannot be judged.
    #[error("peer {node} sent an empty runtime version")]
    MissingRuntimeVersion {
        /// The offending node.
        node: String,
    },
    /// A node tried to connect to itself.
    #[error("mesh handshake refused: node {0} must not connect to itself")]
    SelfConnect(String),
    /// The peer's identity is not the one the caller expected.
    #[error("mesh handshake identity mismatch: expected {expected}, got {actual}")]
    UnexpectedPeer {
        /// The identity the caller intended to reach.
        expected: String,
        /// The identity that answered.
        actual: String,
    },
    /// An advertised capability's kind disagrees with its id prefix.
    #[error("advertised capability {capability} is not well-formed: {reason}")]
    MalformedAdvert {
        /// The offending capability.
        capability: String,
        /// What is wrong.
        reason: String,
    },
}

/// A payload that requests remote execution of one capability call.
///
/// Phase 0 carries tool calls only; the enum is non-exhaustive so provider
/// invocations can join the same generic RPC later without a second envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "call", rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapabilityCall {
    /// One canonical tool call, byte-identical in shape to a local one.
    Tool(ToolCall),
}

/// The payload that answers a [`CapabilityCall`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "result", rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapabilityOutcome {
    /// One canonical tool result. Tool failures travel here too — a failed
    /// call is a result the caller can react to, not a mesh-level error.
    Tool(ToolResult),
}

/// Why a remote invocation could not produce a capability outcome.
///
/// Distinct from a [`CapabilityOutcome::Tool`] failure: these mean the
/// invocation never legitimately completed (refused, unknown, gone, cancelled),
/// and they map onto the caller's error surface rather than onto the model's
/// transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MeshErrorCode {
    /// The peer speaks a different wire protocol version.
    ProtocolVersionMismatch,
    /// The requesting node is not trusted to invoke capabilities here.
    UntrustedPeer,
    /// No such capability is indexed on the executing node.
    UnknownCapability,
    /// The capability exists but its module is not dispatchable right now.
    NotAvailable,
    /// Canonical governance refused the invocation. It did not execute.
    Denied,
    /// Governance requires a human decision; remote calls fail closed instead.
    ApprovalRequired,
    /// The invocation was cancelled before or during execution.
    Cancelled,
    /// The caller's deadline elapsed before execution completed.
    DeadlineExceeded,
    /// The connection failed or closed mid-invocation.
    Transport,
    /// The execution side failed in a way not described above.
    Internal,
    /// The requested invocation shape is not supported by this protocol build.
    Unsupported,
}

/// Handshake message: who I am and what I speak.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    /// The initiating node's descriptor.
    pub node: NodeDescriptor,
}

/// Handshake answer: who I am, and whether I accept.
///
/// A rejection is explicit (`accepted: false` plus a reason) and the connection
/// then closes; there is no downgrade path where the pair silently agrees on a
/// lesser protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloAck {
    /// The accepting node's descriptor.
    pub node: NodeDescriptor,
    /// Whether the connection is accepted.
    pub accepted: bool,
    /// Why not, when refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A node's current capability surface, as a read-only projection.
///
/// Full-snapshot semantics: whatever is absent from the latest advert is gone
/// from the peer's projection. No credentials, no internal module state, no
/// memory contents, no raw private metadata — identity and availability only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityAdvert {
    /// Every module the advertising node currently exposes to the mesh.
    pub modules: Vec<ModuleAdvert>,
}

impl CapabilityAdvert {
    /// Every advertised capability entry, flattened.
    pub fn entries(
        &self,
    ) -> impl Iterator<Item = (&apeireth_core::kernel::PluginId, &CapabilityAdvertEntry)> {
        self.modules.iter().flat_map(|module| {
            module
                .capabilities
                .iter()
                .map(move |entry| (&module.module, entry))
        })
    }

    /// Whether every module in the advert is well-formed.
    pub fn is_well_formed(&self) -> bool {
        self.modules
            .iter()
            .all(ModuleAdvert::entries_are_well_formed)
    }
}

/// A request to execute one capability call on the target node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvokeCapability {
    /// Capability identity, preserved end-to-end (`tool.mesh.echo` stays
    /// `tool.mesh.echo`).
    pub capability: CapabilityId,
    /// Capability category; must agree with the id's prefix.
    pub kind: MeshCapabilityKind,
    /// The canonical call to execute.
    pub call: CapabilityCall,
    /// The caller's session, so executing-side governance can correlate.
    pub session: SessionId,
    /// The caller's agent-loop round, for round-bounding policies.
    #[serde(default)]
    pub round: u32,
    /// Caller deadline, in epoch milliseconds. Propagated, never silently
    /// dropped; the execution side fails closed once it has elapsed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_epoch_ms: Option<u64>,
}

/// The successful outcome of a remote invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityResult {
    /// Capability identity, echoing the request.
    pub capability: CapabilityId,
    /// The canonical outcome produced by the executing node's module.
    pub outcome: CapabilityOutcome,
}

/// A failed remote invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityError {
    /// Capability identity, when the failure is tied to one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<CapabilityId>,
    /// What kind of failure.
    pub code: MeshErrorCode,
    /// Legible detail for the caller.
    pub message: String,
}

/// Liveness signal on an idle connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Sender's clock at send time, in epoch milliseconds.
    pub sent_at_ms: u64,
}

/// Best-effort cancellation of an in-flight request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cancel {
    /// The `message_id` of the [`MeshPayload::InvokeCapability`] to stop.
    pub target: RequestId,
}

/// The message set carried inside a [`MeshEnvelope`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "body", rename_all = "snake_case")]
#[non_exhaustive]
pub enum MeshPayload {
    /// Connection initiation.
    Hello(Hello),
    /// Connection answer.
    HelloAck(HelloAck),
    /// Capability projection update (full snapshot).
    CapabilityAdvert(CapabilityAdvert),
    /// Remote capability invocation.
    InvokeCapability(InvokeCapability),
    /// Successful invocation outcome.
    CapabilityResult(CapabilityResult),
    /// Failed invocation.
    CapabilityError(CapabilityError),
    /// Liveness signal.
    Heartbeat(Heartbeat),
    /// Cancellation of an in-flight invocation.
    Cancel(Cancel),
}

/// The one wire envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshEnvelope {
    /// Wire protocol version of the sender.
    pub protocol_version: u32,
    /// Identity of this message; also the cancellation handle for requests.
    pub message_id: RequestId,
    /// The request this message answers, when it is a response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<RequestId>,
    /// The causally-related span this hop belongs to. Never re-minted mid-span.
    pub trace_id: TraceId,
    /// Sending node. Post-handshake this must equal the connection's peer.
    pub source_node: apeireth_core::kernel::NodeId,
    /// Receiving node.
    pub target_node: apeireth_core::kernel::NodeId,
    /// What is being said.
    pub payload: MeshPayload,
}

impl MeshEnvelope {
    /// A fresh outbound message under an explicit trace.
    pub fn send(
        source: apeireth_core::kernel::NodeId,
        target: apeireth_core::kernel::NodeId,
        trace_id: TraceId,
        payload: MeshPayload,
    ) -> Self {
        Self {
            protocol_version: MESH_PROTOCOL_VERSION,
            message_id: RequestId::new(),
            in_reply_to: None,
            trace_id,
            source_node: source,
            target_node: target,
            payload,
        }
    }

    /// A reply to this message: same trace, reversed direction, correlated id.
    pub fn reply(&self, payload: MeshPayload) -> Self {
        Self {
            protocol_version: MESH_PROTOCOL_VERSION,
            message_id: RequestId::new(),
            in_reply_to: Some(self.message_id),
            trace_id: self.trace_id,
            source_node: self.target_node.clone(),
            target_node: self.source_node.clone(),
            payload,
        }
    }
}

/// Encode an envelope to its wire text.
pub fn encode_envelope(envelope: &MeshEnvelope) -> Result<String, MeshProtocolError> {
    serde_json::to_string(envelope).map_err(|e| MeshProtocolError::Encode(e.to_string()))
}

/// Decode an envelope from its wire text.
pub fn decode_envelope(text: &str) -> Result<MeshEnvelope, MeshProtocolError> {
    serde_json::from_str(text).map_err(|e| MeshProtocolError::Decode(e.to_string()))
}

/// Validate an inbound handshake initiator against this node.
///
/// Fail closed on every mismatch: protocol version (exact — no silent
/// downgrade), missing runtime version, and self-connection.
pub fn verify_hello(local: &NodeDescriptor, hello: &Hello) -> Result<(), MeshProtocolError> {
    let peer = &hello.node;
    if peer.protocol_version != local.protocol_version {
        return Err(MeshProtocolError::ProtocolVersionMismatch {
            local: local.protocol_version,
            remote: peer.protocol_version,
        });
    }
    if peer.runtime_version.trim().is_empty() {
        return Err(MeshProtocolError::MissingRuntimeVersion {
            node: peer.id.to_string(),
        });
    }
    if peer.id == local.id {
        return Err(MeshProtocolError::SelfConnect(peer.id.to_string()));
    }
    Ok(())
}

/// Validate an inbound handshake answer against this node and intent.
///
/// In addition to [`verify_hello`]'s checks, the answering node must be exactly
/// the one the caller set out to reach.
pub fn verify_hello_ack(
    local: &NodeDescriptor,
    ack: &HelloAck,
    expected: &apeireth_core::kernel::NodeId,
) -> Result<(), MeshProtocolError> {
    verify_hello(
        local,
        &Hello {
            node: ack.node.clone(),
        },
    )?;
    if ack.node.id != *expected {
        return Err(MeshProtocolError::UnexpectedPeer {
            expected: expected.to_string(),
            actual: ack.node.id.to_string(),
        });
    }
    if !ack.accepted {
        return Err(MeshProtocolError::UnexpectedPeer {
            expected: "accepted".to_string(),
            actual: ack.reason.clone().unwrap_or_else(|| "refused".to_string()),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::LocatedModuleId;
    use apeireth_core::kernel::{NodeId, PluginId};

    fn node(id: &str) -> NodeId {
        NodeId::new(id).unwrap()
    }

    fn descriptor(id: &str) -> NodeDescriptor {
        NodeDescriptor::new(node(id), id, MESH_PROTOCOL_VERSION, "2.0.0-rc.1")
    }

    fn invoke_payload() -> MeshPayload {
        MeshPayload::InvokeCapability(InvokeCapability {
            capability: CapabilityId::new("tool.mesh.echo").unwrap(),
            kind: MeshCapabilityKind::Tool,
            call: CapabilityCall::Tool(ToolCall {
                id: "call_1".into(),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({ "text": "hi" }),
            }),
            session: SessionId::new(),
            round: 1,
            deadline_epoch_ms: Some(1_700_000_000_000),
        })
    }

    #[test]
    fn an_envelope_round_trips_through_the_wire_text() {
        let env = MeshEnvelope::send(
            node("node-a"),
            node("node-b"),
            TraceId::new(),
            invoke_payload(),
        );
        let text = encode_envelope(&env).unwrap();
        let back = decode_envelope(&text).unwrap();
        assert_eq!(env, back);
        assert_eq!(back.protocol_version, MESH_PROTOCOL_VERSION);
    }

    #[test]
    fn a_reply_keeps_the_trace_and_correlates_to_its_request() {
        let request = MeshEnvelope::send(
            node("node-a"),
            node("node-b"),
            TraceId::new(),
            invoke_payload(),
        );
        let reply = request.reply(MeshPayload::CapabilityResult(CapabilityResult {
            capability: CapabilityId::new("tool.mesh.echo").unwrap(),
            outcome: CapabilityOutcome::Tool(ToolResult::ok("call_1", serde_json::json!("hi"))),
        }));

        assert_eq!(reply.trace_id, request.trace_id, "trace survives the hop");
        assert_eq!(reply.in_reply_to, Some(request.message_id));
        assert_eq!(reply.source_node, request.target_node);
        assert_eq!(reply.target_node, request.source_node);
    }

    #[test]
    fn canonical_tool_payloads_ride_the_wire_without_a_second_tool_contract() {
        // `CapabilityCall::Tool` carries the *canonical* ToolCall — the wire
        // must not grow a parallel MeshToolRequest shape.
        let text = encode_envelope(&MeshEnvelope::send(
            node("node-a"),
            node("node-b"),
            TraceId::new(),
            invoke_payload(),
        ))
        .unwrap();
        assert!(text.contains("\"tool_call_id\"") || text.contains("call_1"));
        let decoded = decode_envelope(&text).unwrap();
        match decoded.payload {
            MeshPayload::InvokeCapability(inv) => {
                // Exhaustive today (Tool is the only call shape in Phase 0);
                // adding a variant later fails here first, which is the
                // reminder to give it the same canonical-payload treatment.
                let CapabilityCall::Tool(call) = inv.call;
                assert_eq!(call.name, "mesh_echo");
                assert_eq!(call.arguments["text"], "hi");
            }
            other => panic!("expected invoke, got {other:?}"),
        }
    }

    #[test]
    fn a_version_mismatch_fails_closed_without_downgrading() {
        let local = descriptor("node-a");
        let mut remote = descriptor("node-b");
        remote.protocol_version = MESH_PROTOCOL_VERSION + 7;

        let err = verify_hello(&local, &Hello { node: remote }).unwrap_err();
        assert!(matches!(
            err,
            MeshProtocolError::ProtocolVersionMismatch {
                local: 1,
                remote: 8
            }
        ));
    }

    #[test]
    fn handshake_rejects_self_connect_and_identity_confusion() {
        let local = descriptor("node-a");

        // Self-connect: refused.
        let err = verify_hello(
            &local,
            &Hello {
                node: local.clone(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, MeshProtocolError::SelfConnect(_)));

        // Identity confusion: the answer must come from the node we intended.
        let ack = HelloAck {
            node: descriptor("node-c"),
            accepted: true,
            reason: None,
        };
        let err = verify_hello_ack(&local, &ack, &node("node-b")).unwrap_err();
        assert!(matches!(err, MeshProtocolError::UnexpectedPeer { .. }));
    }

    #[test]
    fn handshake_requires_a_runtime_version_to_judge_compatibility() {
        let local = descriptor("node-a");
        let mut remote = descriptor("node-b");
        remote.runtime_version = "  ".into();
        let err = verify_hello(&local, &Hello { node: remote }).unwrap_err();
        assert!(matches!(
            err,
            MeshProtocolError::MissingRuntimeVersion { .. }
        ));
    }

    #[test]
    fn a_rejection_is_explicit_rather_than_a_silent_downgrade() {
        let local = descriptor("node-a");
        let ack = HelloAck {
            node: descriptor("node-b"),
            accepted: false,
            reason: Some("not in known peers".into()),
        };
        let err = verify_hello_ack(&local, &ack, &node("node-b")).unwrap_err();
        assert!(matches!(err, MeshProtocolError::UnexpectedPeer { .. }));
    }

    #[test]
    fn an_advert_carries_identity_and_availability_and_never_implementations() {
        let advert = CapabilityAdvert {
            modules: vec![ModuleAdvert {
                module: PluginId::new("builtin.mesh-test").unwrap(),
                capabilities: vec![CapabilityAdvertEntry::tool(
                    CapabilityId::new("tool.mesh.echo").unwrap(),
                    "echo",
                    true,
                    apeireth_protocol::canonical::NormalizedTool {
                        name: "mesh_echo".into(),
                        description: Some("echo".into()),
                        parameters: apeireth_protocol::canonical::ToolParameters::new(),
                        strict: false,
                    },
                )],
            }],
        };
        assert!(advert.is_well_formed());
        let owner = PluginId::new("builtin.mesh-test").unwrap();
        assert_eq!(
            LocatedModuleId::new(node("node-b"), owner.clone()),
            LocatedModuleId::new(node("node-b"), owner)
        );
        let text = serde_json::to_string(&advert).unwrap();
        // Structural guard: an advert must not smuggle implementation objects.
        for forbidden in [
            "Arc",
            "dyn Plugin",
            "dyn ToolCapability",
            "credential",
            "secret",
        ] {
            assert!(!text.contains(forbidden), "advert leaked {forbidden:?}");
        }
    }

    #[test]
    fn cancel_targets_a_specific_request() {
        let target = RequestId::new();
        let env = MeshEnvelope::send(
            node("node-a"),
            node("node-b"),
            TraceId::new(),
            MeshPayload::Cancel(Cancel { target }),
        );
        match decode_envelope(&encode_envelope(&env).unwrap())
            .unwrap()
            .payload
        {
            MeshPayload::Cancel(cancel) => assert_eq!(cancel.target, target),
            other => panic!("expected cancel, got {other:?}"),
        }
    }
}
