//! Retained Host compatibility calls must obey the common dispatch boundary.
use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_plugin::{task_scope::ModuleTaskScope, FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use apeireth_runtime::canonical::dispatcher::{DispatchLimits, ToolDispatcher, ToolScope};
use async_trait::async_trait;
use serde_json::json;
use std::sync::{Arc, Mutex};

type Seen = (ToolCall, Option<FrozenInvocation>, Option<SessionId>);
struct Echo {
    id: CapabilityId,
    seen: Mutex<Vec<Seen>>,
}
#[async_trait]
impl ToolCapability for Echo {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("echo").with_parameters(json!({"type":"object","properties":{"value":{"type":"integer"}},"required":["value"],"additionalProperties":false}).as_object().unwrap().clone())
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invoke_in_session(call, None, None).await
    }
    async fn invoke_in_session(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
        session: Option<SessionId>,
    ) -> ToolResult {
        self.seen
            .lock()
            .unwrap()
            .push((call.clone(), frozen.cloned(), session));
        ToolResult::ok(&call.id, call.arguments.clone())
    }
}

#[tokio::test]
async fn compatibility_invocations_preserve_payload_and_enforce_schema_scope_and_shutdown() {
    let tool = Arc::new(Echo {
        id: CapabilityId::new("tool.echo").unwrap(),
        seen: Mutex::new(vec![]),
    });
    let dispatcher = Arc::new(ToolDispatcher::new(DispatchLimits::default()));
    let scope = ModuleTaskScope::new();
    let adapter = dispatcher.adapt(tool.clone(), Some(scope.clone()), false);
    let call = ToolCall {
        id: "host-call".into(),
        name: "echo".into(),
        arguments: json!({"value":7}),
    };
    let frozen = FrozenInvocation::new(json!({"bound":7}), json!({"display":"redacted"}));
    assert!(adapter.invoke(&call).await.is_ok());
    assert!(adapter.invoke_frozen(&call, Some(&frozen)).await.is_ok());
    let session = SessionId::new();
    assert!(adapter
        .invoke_in_session(&call, Some(&frozen), Some(session))
        .await
        .is_ok());
    let seen = tool.seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[2].0, call);
    assert_eq!(seen[2].1, Some(frozen));
    assert_eq!(seen[2].2, Some(session));

    let invalid = ToolCall {
        arguments: json!({"value":"wrong"}),
        ..call.clone()
    };
    assert!(adapter
        .invoke(&invalid)
        .await
        .render()
        .contains("invalid_arguments"));
    dispatcher.set_scope(session, ToolScope::Only(Default::default()));
    assert!(adapter
        .invoke_in_session(&call, None, Some(session))
        .await
        .render()
        .contains("scope_denied"));
    scope.close_and_wait().await;
    assert!(adapter.invoke(&call).await.render().contains("stopping"));
    assert_eq!(
        tool.seen.lock().unwrap().len(),
        3,
        "rejected calls must never reach the tool"
    );
}
