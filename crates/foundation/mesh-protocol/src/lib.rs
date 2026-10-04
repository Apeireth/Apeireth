//! The Apeireth Mesh wire contract.
//!
//! # The one idea
//!
//! **Remote execution is an extension of the Module/Capability system, not a
//! second runtime architecture.** This crate is only the vocabulary two
//! runtimes exchange so that one can invoke a capability owned by the other:
//! node identity, one envelope, one small message set. Modules still own
//! implementations at their origin node; capabilities still describe what can
//! be done; the network only changes *where execution happens*.
//!
//! # Layering
//!
//! Depends on `apeireth-core` (ids) and `apeireth-protocol` (canonical
//! `ToolCall` / `ToolResult`, which ride the wire unmodified). Must not
//! depend on `apeireth-plugin`, the runtime, tokio, or any transport: this is
//! pure data, and sockets live strictly above it in `apeireth-mesh`.
//!
//! # What is deliberately absent
//!
//! Capability implementations, manifests-as-truth, credentials, memory sync,
//! session handoff, scheduler messages, discovery gossip. An
//! [`CapabilityAdvert`] is a read-only projection of an origin node's canonical
//! indexed capability view — never a second ownership registry.

#![deny(unsafe_code)]

pub mod envelope;
pub mod identity;
pub mod observe;
pub mod pairing;
pub mod rendezvous;

pub use envelope::{
    decode_envelope, encode_envelope, verify_hello, verify_hello_ack, Cancel, CapabilityAdvert,
    CapabilityCall, CapabilityError, CapabilityOutcome, CapabilityResult, Heartbeat, Hello,
    HelloAck, InvokeCapability, MeshEnvelope, MeshErrorCode, MeshPayload, MeshProtocolError,
    MESH_PROTOCOL_VERSION,
};
pub use identity::{
    CapabilityAdvertEntry, KeyFingerprint, LocatedPluginId, MeshCapabilityKind, NodeDescriptor,
    PluginAdvert, PluginLocation, WireModelDescriptor,
};
pub use observe::{
    decode_probe, decode_reply, encode_probe, encode_reply, OBSERVE_DATAGRAM_BYTES, OBSERVE_MAGIC,
    OBSERVE_NONCE_BYTES, OBSERVE_VERSION,
};
pub use pairing::{
    PairChallenge, PairCodeReady, PairComplete, PairConfirm, PairHello, PairReject, PairReveal,
    PairingMessage, PAIRING_COMMIT_DOMAIN, PAIRING_PROTOCOL_VERSION, PAIRING_SAS_DOMAIN,
};
pub use rendezvous::{
    canonicalize_candidates, decode_frame, encode_control, encode_relay, EndpointCandidate,
    EndpointCandidateKind, Frame, PresenceRecord, RelayCircuitId, RelayControl, RelayFrame,
    RendezvousError, RendezvousRequest, RendezvousResponse, ED25519_PUBLIC_KEY_BYTES,
    FRAME_TAG_CONTROL, FRAME_TAG_RELAY, MAX_FRAME_BODY_BYTES, PRESENCE_DEFAULT_TTL_MS,
    PRESENCE_DOMAIN, PRESENCE_MAX_CANDIDATES, PRESENCE_MAX_TTL_MS, PRESENCE_MIN_TTL_MS,
    PRESENCE_NONCE_BYTES, RELAY_CIRCUIT_ID_BYTES, RENDEZVOUS_PROTOCOL_VERSION,
};

pub use apeireth_core::kernel::NodeId;
