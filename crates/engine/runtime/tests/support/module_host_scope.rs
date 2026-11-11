use super::*;
use apeireth_mesh::{
    RemoteCapabilityError, RemoteCapabilityPort, RemoteInvokeContext, RemoteProviderDescriptor,
    RemoteToolDescriptor, RemoteToolRoute,
};
use apeireth_protocol::canonical::{
    NormalizedRequest, NormalizedResponse, NormalizedTool, ToolCall, ToolParameters, ToolResult,
};
use std::collections::BTreeSet;

struct CatalogPort;

#[async_trait::async_trait]
impl RemoteCapabilityPort for CatalogPort {
    fn remote_tools(&self) -> Vec<RemoteToolDescriptor> {
        vec![RemoteToolDescriptor {
            node: apeireth_core::kernel::NodeId::new("peer").unwrap(),
            module: PluginId::new("module.peer").unwrap(),
            capability: CapabilityId::new("tool.peer.echo").unwrap(),
            declaration: NormalizedTool {
                name: "peer_echo".into(),
                description: None,
                parameters: ToolParameters::new(),
                strict: false,
            },
            available: true,
        }]
    }
    fn remote_providers(&self) -> Vec<RemoteProviderDescriptor> {
        vec![]
    }
    async fn invoke_tool(
        &self,
        _: RemoteToolRoute,
        _: ToolCall,
        _: RemoteInvokeContext,
    ) -> Result<ToolResult, RemoteCapabilityError> {
        panic!("catalog queries must never invoke a tool")
    }
    async fn invoke_provider(
        &self,
        _: RemoteToolRoute,
        _: NormalizedRequest,
        _: RemoteInvokeContext,
    ) -> Result<NormalizedResponse, RemoteCapabilityError> {
        panic!("catalog queries must never invoke a provider")
    }
}

#[tokio::test]
async fn remote_catalog_child_scope_cannot_widen_parent_scope() {
    use super::super::dispatcher::ToolScope;
    let runtime = Runtime::builder()
        .with_governance(Arc::new(apeireth_governance::AllowAll))
        .build()
        .await
        .unwrap();
    runtime.set_remote_port(Arc::new(CatalogPort));
    let parent = SessionId::new();
    let child = SessionId::new();
    let capability = CapabilityId::new("tool.peer.echo").unwrap();
    runtime.dispatcher.bind_parent(child, parent);
    runtime
        .dispatcher
        .set_scope(child, ToolScope::Only(BTreeSet::from([capability.clone()])));
    assert_eq!(runtime.tool_declarations_for_session(child).len(), 1);
    runtime
        .dispatcher
        .set_scope(parent, ToolScope::Only(BTreeSet::new()));
    assert!(!runtime.dispatcher.allows(child, &capability));
    assert!(runtime.tool_declarations_for_session(child).is_empty());

    // The model may guess an omitted name; the child's dispatch still
    // fails before the port (whose invocation methods panic above).
    use super::super::{
        dispatcher, execute::ToolDispatch, session::Session, trace::ExecutionTrace,
    };
    use apeireth_core::kernel::{RequestId, TraceId};
    let request = RequestId::new();
    let trace = TraceId::new();
    let call = ToolCall {
        id: "child_guess".into(),
        name: "peer_echo".into(),
        arguments: serde_json::json!({}),
    };
    let result = runtime
        .dispatch_one_tool(
            &mut ExecutionTrace::new(trace, child, request),
            &mut Session::new(child, runtime.clock().as_ref()),
            &child,
            request,
            trace,
            &call,
            1,
            None,
            false,
            None,
            false,
        )
        .await
        .unwrap();
    let ToolDispatch::Result(result) = result else {
        panic!("scope denial cannot mint an approval")
    };
    assert_eq!(result.tool_call_id, "child_guess");
    let apeireth_protocol::canonical::ToolOutcome::Error {
        message,
        retryable: false,
    } = result.outcome
    else {
        panic!("scope denial must be a permanent tool failure")
    };
    let error: dispatcher::DispatchFailure = serde_json::from_str(&message).unwrap();
    assert_eq!(error.code, dispatcher::DispatchErrorCode::ScopeDenied);
}
