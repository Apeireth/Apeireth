//! Names for wire values, for error text and logs.
//!
//! Mesh payloads carry tool arguments, tool results, prompts and completions.
//! A `{:?}` of one in an error message would copy that content into logs and
//! into errors that travel back to a peer, so diagnostics name the *shape*
//! only.

use apeireth_mesh_protocol::{CapabilityCall, CapabilityOutcome, MeshPayload, PairingMessage};

pub(crate) fn payload_kind(payload: &MeshPayload) -> &'static str {
    match payload {
        MeshPayload::Hello(_) => "hello",
        MeshPayload::HelloAck(_) => "hello_ack",
        MeshPayload::CapabilityAdvert(_) => "capability_advert",
        MeshPayload::InvokeCapability(_) => "invoke_capability",
        MeshPayload::CapabilityResult(_) => "capability_result",
        MeshPayload::CapabilityError(_) => "capability_error",
        MeshPayload::Heartbeat(_) => "heartbeat",
        MeshPayload::Cancel(_) => "cancel",
        MeshPayload::Pair(_) => "pair",
        _ => "unknown",
    }
}

pub(crate) fn outcome_kind(outcome: &CapabilityOutcome) -> &'static str {
    match outcome {
        CapabilityOutcome::Tool(_) => "tool",
        CapabilityOutcome::Provider(_) => "provider",
        _ => "unknown",
    }
}

pub(crate) fn call_kind(call: &CapabilityCall) -> &'static str {
    match call {
        CapabilityCall::Tool(_) => "tool",
        CapabilityCall::Provider(_) => "provider",
        _ => "unknown",
    }
}

pub(crate) fn pairing_kind(message: &PairingMessage) -> &'static str {
    match message {
        PairingMessage::Hello(_) => "hello",
        PairingMessage::Challenge(_) => "challenge",
        PairingMessage::Reveal(_) => "reveal",
        PairingMessage::CodeReady(_) => "code_ready",
        PairingMessage::Confirm(_) => "confirm",
        PairingMessage::Reject(_) => "reject",
        PairingMessage::Complete(_) => "complete",
        _ => "unknown",
    }
}
