//! The tool capability: something a model can call.

use apeireth_core::kernel::CapabilityId;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;

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
    async fn invoke(&self, call: &ToolCall) -> ToolResult;
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
