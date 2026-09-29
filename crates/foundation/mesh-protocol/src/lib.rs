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

pub use envelope::{
    decode_envelope, encode_envelope, verify_hello, verify_hello_ack, Cancel, CapabilityAdvert,
    CapabilityCall, CapabilityError, CapabilityOutcome, CapabilityResult, Heartbeat, Hello,
    HelloAck, InvokeCapability, MeshEnvelope, MeshErrorCode, MeshPayload, MeshProtocolError,
    MESH_PROTOCOL_VERSION,
};
pub use identity::{
    CapabilityAdvertEntry, LocatedModuleId, MeshCapabilityKind, ModuleAdvert, ModuleLocation,
    NodeDescriptor,
};

pub use apeireth_core::kernel::NodeId;
