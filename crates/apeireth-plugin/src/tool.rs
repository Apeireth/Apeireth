//! The tool capability: something a model can call.

use apeireth_core::kernel::CapabilityId;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A capability-frozen effective invocation.
///
/// The runtime treats both fields as opaque. A tool that supplies this value
/// owns the `payload` schema and must be able to execute it later via
/// [`ToolCapability::invoke_frozen`]. The `display` field is the safe,
/// human-facing representation shown in approval views; it must not expose
/// secret values that are required only for execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenInvocation {
    /// Versioned, tool-owned execution payload. The runtime never inspects it.
    pub payload: serde_json::Value,
    /// Redacted human-facing display payload. Approval views use this.
    pub display: serde_json::Value,
}

impl FrozenInvocation {
    /// Builds a frozen invocation from its execution and display payloads.
    pub fn new(payload: serde_json::Value, display: serde_json::Value) -> Self {
        Self { payload, display }
    }
}

/// A capability a model can invoke by name.
///
/// # Why `invoke` cannot fail
///
/// It returns [`ToolResult`], not `Result<ToolResult, _>`. A tool that fails has
/// not broken the turn — the model asked for something, the answer is "that did
/// not work, here is why", and the model gets to react. Modelling tool failure as
/// a `Result` error pushes every implementer toward propagating it, which aborts
/// the turn and denies the model the one piece of information it needs to recover.
///
/// Failures that genuinely are not the model's business — the plugin is not
/// active, the capability does not exist — are caught by the registry before
/// `invoke` is ever reached.
#[async_trait]
pub trait ToolCapability: Send + Sync {
    /// Stable identity, e.g. `tool.calculator`. Must match the manifest.
    fn id(&self) -> &CapabilityId;

    /// The declaration sent to the model: name, description, JSON-schema
    /// parameters.
    ///
    /// The `name` here is what the model emits in a [`ToolCall`], and it need not
    /// equal [`ToolCapability::id`]: ids are namespaced for the registry, while
    /// model-facing names are short because they cost tokens on every request.
    fn declaration(&self) -> NormalizedTool;

    /// Execute one call and describe what happened.
    ///
    /// Simple tools implement this method and leave [`ToolCapability::invoke_frozen`]
    /// at its default. The runtime uses this path for normal `Allow` dispatches.
    async fn invoke(&self, call: &ToolCall) -> ToolResult;

    /// Execute a previously frozen effective invocation.
    ///
    /// When `frozen` is `None`, the original [`ToolCall`] was the complete
    /// frozen operation and the default simply delegates to
    /// [`ToolCapability::invoke`]. Tools that returned a
    /// [`FrozenInvocation`] from [`ToolCapability::freeze_invocation`] must
    /// override this method, deserialize their own `frozen.payload`, and build
    /// the real execution from that payload alone — never from current ambient
    /// configuration.
    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        let _ = frozen;
        self.invoke(call).await
    }

    /// Optional canonical frozen-invocation payload for approval binding.
    ///
    /// The runtime stores whatever this returns inside the pending approval and
    /// includes it in the operation fingerprint. Tools whose security-relevant
    /// execution shape is not fully captured by [`ToolCall::arguments`] — for
    /// example a shell resolving cwd, shell executable, timeout, and
    /// environment — should override this method and return the exact effective
    /// invocation the human is approving.
    ///
    /// Returning `Ok(None)` means the original [`ToolCall`] is the complete
    /// frozen operation. Returning `Err(result)` means the operation cannot be
    /// prepared for approval: the runtime must surface the tool failure and
    /// must not create a pending approval for an invalid operation.
    fn freeze_invocation(&self, _call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::ToolParameters;

    struct Calculator {
        id: CapabilityId,
    }

    #[async_trait]
    impl ToolCapability for Calculator {
        fn id(&self) -> &CapabilityId {
            &self.id
        }

        fn declaration(&self) -> NormalizedTool {
            NormalizedTool {
                name: "calculator".into(),
                description: Some("Add two integers".into()),
                parameters: ToolParameters::new(),
                strict: false,
            }
        }

        async fn invoke(&self, call: &ToolCall) -> ToolResult {
            let (Some(a), Some(b)) = (
                call.arguments.get("a").and_then(serde_json::Value::as_i64),
                call.arguments.get("b").and_then(serde_json::Value::as_i64),
            ) else {
                return ToolResult::permanent_error(&call.id, "expected integer fields a and b");
            };
            ToolResult::ok(&call.id, serde_json::json!(a + b))
        }
    }

    fn calculator() -> Calculator {
        Calculator {
            id: CapabilityId::new("tool.calculator").unwrap(),
        }
    }

    #[tokio::test]
    async fn a_successful_call_returns_a_correlated_result() {
        let call = ToolCall {
            id: "call_1".into(),
            name: "calculator".into(),
            arguments: serde_json::json!({ "a": 1, "b": 1 }),
        };
        let result = calculator().invoke(&call).await;

        assert!(result.is_ok());
        assert_eq!(result.tool_call_id, "call_1");
        assert_eq!(result.render(), "2");
    }

    #[tokio::test]
    async fn a_bad_call_yields_a_result_rather_than_aborting_the_turn() {
        let call = ToolCall {
            id: "call_2".into(),
            name: "calculator".into(),
            arguments: serde_json::json!({ "a": "not a number" }),
        };
        let result = calculator().invoke(&call).await;

        assert!(!result.is_ok());
        assert_eq!(
            result.tool_call_id, "call_2",
            "even a failure stays correlated, so the model can answer the right call"
        );
        assert!(
            result.render().contains("integer fields"),
            "{}",
            result.render()
        );
    }

    #[test]
    fn the_model_facing_name_may_differ_from_the_registry_id() {
        let c = calculator();
        assert_eq!(c.id().as_str(), "tool.calculator");
        assert_eq!(c.declaration().name, "calculator");
    }
}
