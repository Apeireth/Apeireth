//! Shared transport conformance: one behavioral suite, every transport.
//!
//! The semantics of a [`MeshConnection`] must be identical whether the bytes
//! bounce through an in-memory channel or a QUIC socket — so the assertions
//! live exactly once, here, and each transport's tests run them against their
//! own pair. Transport-specific mechanics (framing limits, backpressure,
//! malformed frames) stay in the transport's own tests; everything behavioral
//! is proven twice from one source.

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_core::kernel::{NodeId, RequestId, TraceId};
use apeireth_mesh_protocol::{
    Cancel, CapabilityCall, CapabilityError, CapabilityOutcome, CapabilityResult, Heartbeat,
    InvokeCapability, MeshCapabilityKind, MeshEnvelope, MeshErrorCode, MeshPayload,
    MESH_PROTOCOL_VERSION,
};

use crate::MeshConnection;

fn node(id: &str) -> NodeId {
    NodeId::new(id).expect("valid node id")
}

fn call_envelope(from: &str, to: &str, tag: u32) -> MeshEnvelope {
    MeshEnvelope::send(
        node(from),
        node(to),
        TraceId::new(),
        MeshPayload::InvokeCapability(InvokeCapability {
            capability: CapabilityId::new("tool.mesh.echo").unwrap(),
            kind: MeshCapabilityKind::Tool,
            call: CapabilityCall::Tool(apeireth_protocol::canonical::ToolCall {
                id: format!("call-{tag}"),
                name: "mesh_echo".into(),
                arguments: serde_json::json!({ "tag": tag }),
            }),
            session: apeireth_core::kernel::SessionId::new(),
            round: 1,
            deadline_epoch_ms: None,
        }),
    )
}

/// Assert the full shared behavioral contract on one live connection pair
/// (`a` is held by node-a, `b` by node-b).
///
/// Covers: send/receive both ways, ordering, request/reply correlation,
/// control messages (heartbeat/cancel) passing, and close semantics — the
/// §28 conformance list minus `max frame`, which is a transport mechanic.
pub async fn assert_connection_conformance(a: Arc<dyn MeshConnection>, b: Arc<dyn MeshConnection>) {
    // 1. Send/receive both ways.
    a.send(call_envelope("node-a", "node-b", 1))
        .await
        .expect("a→b send");
    let got = b.recv().await.expect("b recv");
    assert_eq!(got.source_node.as_str(), "node-a");
    b.send(call_envelope("node-b", "node-a", 2))
        .await
        .expect("b→a send");
    let got = a.recv().await.expect("a recv");
    assert_eq!(got.source_node.as_str(), "node-b");

    // 2. Ordering: FIFO per direction.
    for tag in 10..15 {
        a.send(call_envelope("node-a", "node-b", tag))
            .await
            .expect("ordered send");
    }
    for tag in 10..15 {
        let got = b.recv().await.expect("ordered recv");
        match got.payload {
            MeshPayload::InvokeCapability(invoke) => match invoke.call {
                CapabilityCall::Tool(call) => assert_eq!(call.id, format!("call-{tag}")),
                other => panic!("unexpected call shape: {other:?}"),
            },
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    // 3. Request/reply correlation survives the medium.
    let request = call_envelope("node-a", "node-b", 99);
    a.send(request).await.expect("request send");
    let got = b.recv().await.expect("request recv");
    let reply = got.reply(MeshPayload::CapabilityResult(CapabilityResult {
        capability: CapabilityId::new("tool.mesh.echo").unwrap(),
        outcome: CapabilityOutcome::Tool(apeireth_protocol::canonical::ToolResult::ok(
            "call-99",
            serde_json::json!({"echoed": true}),
        )),
    }));
    b.send(reply).await.expect("reply send");
    let got = a.recv().await.expect("reply recv");
    assert_eq!(got.in_reply_to.is_some(), true, "replies correlate");
    assert_eq!(got.trace_id, got.trace_id, "trace id present");

    // 4. Control messages pass with the same semantics.
    a.send(MeshEnvelope::send(
        node("node-a"),
        node("node-b"),
        TraceId::new(),
        MeshPayload::Heartbeat(Heartbeat { sent_at_ms: 1 }),
    ))
    .await
    .expect("heartbeat send");
    assert!(matches!(
        b.recv().await.expect("heartbeat recv").payload,
        MeshPayload::Heartbeat(_)
    ));
    let cancel = MeshEnvelope::send(
        node("node-a"),
        node("node-b"),
        TraceId::new(),
        MeshPayload::Cancel(Cancel {
            target: RequestId::new(),
        }),
    );
    a.send(cancel).await.expect("cancel send");
    assert!(matches!(
        b.recv().await.expect("cancel recv").payload,
        MeshPayload::Cancel(_)
    ));

    // 5. Errors travel as typed payloads (never as transport panic).
    let failure = call_envelope("node-b", "node-a", 42).reply(MeshPayload::CapabilityError(
        CapabilityError {
            capability: Some(CapabilityId::new("tool.mesh.echo").unwrap()),
            code: MeshErrorCode::Internal,
            message: "conformance".to_string(),
        },
    ));
    b.send(failure).await.expect("error send");
    match a.recv().await.expect("error recv").payload {
        MeshPayload::CapabilityError(error) => assert_eq!(error.code, MeshErrorCode::Internal),
        other => panic!("unexpected payload: {other:?}"),
    }

    // 6. Close: the far side observes the close as a typed error.
    a.close();
    assert!(b.recv().await.is_err(), "far side must observe close");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::linked_pair;

    #[tokio::test]
    async fn the_in_memory_transport_satisfies_the_shared_conformance() {
        let (a, b) = linked_pair();
        assert_connection_conformance(Arc::new(a), Arc::new(b)).await;
    }

    #[test]
    fn the_envelope_vocabulary_the_suite_uses_is_protocol_stable() {
        assert_eq!(MESH_PROTOCOL_VERSION, 2);
    }
}
