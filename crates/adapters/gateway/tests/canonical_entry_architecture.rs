//! Architecture freeze guard for the canonical gateway entry.
//!
//! The semantic proof is `canonical_entry_e2e.rs`: an Axum request travels
//! through the real router and lands in `Runtime::execute`. This guard keeps the
//! adapter source itself from drifting back into orchestration.

const CANONICAL_ENTRY: &str = include_str!("../src/canonical_entry.rs");
const CHAT_REQUEST: &str = include_str!("../src/canonical_entry/chat_request.rs");
const NATIVE_HTTP: &str = include_str!("../src/canonical_entry/native_http.rs");
const NATIVE_STREAM: &str = include_str!("../src/canonical_entry/native_stream.rs");
const SERIALIZED_TURN: &str = include_str!("../src/canonical_entry/serialized_turn.rs");

#[test]
fn canonical_gateway_entry_delegates_semantics_to_runtime_execute() {
    assert!(
        CANONICAL_ENTRY.contains(".route(\"/v1/chat\", native_http::route())")
            && NATIVE_HTTP.contains("post(native_chat)")
            && NATIVE_HTTP.contains("execute_chat_attributed_operation("),
        "the native route must delegate to the attributed canonical adapter"
    );
    assert!(
        SERIALIZED_TURN.contains("run_turn_serialized(runtime, turn,")
            && SERIALIZED_TURN.contains("runtime.execute_outcome(turn)")
            && SERIALIZED_TURN.contains("turn.with_request(request).without_stream_replay()")
            && SERIALIZED_TURN.contains("runtime.execute_outcome_streaming(turn, sink)"),
        "the canonical adapter must enter Runtime::execute_outcome under session serialization"
    );
    assert!(
        CANONICAL_ENTRY.contains("pub async fn execute_chat"),
        "the transport-neutral gateway entry adapter must exist"
    );
    assert!(
        CANONICAL_ENTRY.contains(".pending_approvals("),
        "approval inbox must be a Runtime read projection, not a second store"
    );
    assert!(
        CANONICAL_ENTRY.contains("\"/v1/approvals\""),
        "the minimum pending-approval read surface must exist"
    );
}

#[test]
fn canonical_gateway_entry_does_not_reach_around_the_runtime() {
    for forbidden in [
        "ProviderCapability",
        "ToolCapability",
        "ToolRegistry",
        "SessionManager",
        "PluginManager",
        "ProviderRouter",
        "ToolResult",
        "ProcessExecutor",
        "ProcessRequest",
        "ProcessLimits",
        "ToolExecutor",
        "/v1/apeireth/grant",
        "master_token",
        "/v1/panel/",
    ] {
        for (name, source) in [
            ("canonical_entry", CANONICAL_ENTRY),
            ("chat_request", CHAT_REQUEST),
            ("native_http", NATIVE_HTTP),
            ("native_stream", NATIVE_STREAM),
            ("serialized_turn", SERIALIZED_TURN),
        ] {
            assert!(
                !source.contains(forbidden),
                "{name} must stay a transport adapter and not import {forbidden:?}"
            );
        }
    }
}
