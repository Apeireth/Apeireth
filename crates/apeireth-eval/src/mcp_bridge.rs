//! R114: EvalScenario → MCP `tools` 桥接 (apeireth-eval 真接 apeireth-mcp)
//!
//! **目标**: apeireth-eval 的 SmokeReport / RealLlmSmokeReport / cross_model_benchmark
//! 报告作为 MCP tools 暴露, 任意 MCP client 都能 list/call 跑 eval.
//!
//! **Apeireth 真接 (本 module)**:
//! - `EvalToolServer` impl `apeireth_mcp::tools::ToolServer`
//!   - `list()` — 3 个 tools:
//!     - `eval_smoke` — 跑 1 个 smoke task (per smoke_task.rs, 0 LLM, stub F)
//!     - `eval_real_llm` — 跑 1 个 real LLM smoke (per real_llm_smoke.rs, APEIRETH_EVAL_LIVE=1 gate)
//!     - `eval_cross_model` — 跑 cross_model_benchmark on N model (per cross_model_benchmark.rs)
//!   - `call(name, args)` — 按 name dispatch, 返 JSON 报告 (SmokeReport / RealLlmSmokeReport / benchmark report)
//! - `EvalToolCallHandler` trait — 跟 R86 SkillCallHandler 一样, 让 caller 注入真 eval runner
//! - `EvalToolServer::new()` — 默认 0 真跑 (返 metadata, 标 simulated)
//! - `EvalToolServer::with_handler(handler)` — 委托模式 (handler 真接)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `smoke_task.rs` / `real_llm_smoke.rs` / `cross_model_benchmark.rs` (R32-3 / R32-3-1 / R32-3-2 LOCKED)
//! - 0 改 `apeireth_mcp::tools` 已有 Tool / ToolServer (R65 LOCKED)
//! - 0 引入 I/O / 网络 (handler 注入, 0 自创 I/O; 跑 LLM 必须 caller 显式 opt-in via APEIRETH_EVAL_LIVE)
//!
//! **借鉴锚 (S-12)**:
//! - VCP `vcptoolbox/modules/eval` (module 自带 eval scenario)
//! - OpenAI Evals §composite (eval pool 1:1 → MCP tool list)
//! - LangChain `EvaluatorCallbackHandler` (每个 tool 跑完自动 eval)

use apeireth_mcp::protocol::JsonRpcError;
use apeireth_mcp::tools::{Tool, ToolCallResult, ToolContent, ToolServer, TOOL_NOT_FOUND};
use serde_json::{json, Value};

// ============================================================
// EvalToolCallHandler trait
// ============================================================

/// **Eval tool call handler 抽象** — 消费者注入真 eval runner.
///
/// 不实现 trait 的消费者用 `EvalToolServer::new()` (纯 metadata 模式),
/// call 返 stub 报告, 标 is_error=false (不假装真跑).
pub trait EvalToolCallHandler: Send + Sync {
    /// 按 tool name + 实际 args 跑 eval, 返 MCP ToolCallResult
    fn call(&self, name: &str, arguments: &Value) -> Result<ToolCallResult, JsonRpcError>;
}

// ============================================================
// EvalToolServer
// ============================================================

/// **MCP ToolServer impl, 把 eval 报告 (3 类) 暴露为 MCP tools**
pub struct EvalToolServer {
    /// 是否启用 (disabled 模式下所有 call 都是 no-op)
    enabled: bool,
    handler: Option<Box<dyn EvalToolCallHandler>>,
}

impl std::fmt::Debug for EvalToolServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvalToolServer")
            .field("enabled", &self.enabled)
            .field("has_handler", &self.handler.is_some())
            .finish()
    }
}

impl Default for EvalToolServer {
    fn default() -> Self {
        Self::new()
    }
}

impl EvalToolServer {
    /// **metadata 模式** — 3 个 tool 列表可拿, call 返 stub 报告 (0 真跑)
    pub fn new() -> Self {
        Self {
            enabled: true,
            handler: None,
        }
    }

    /// **disabled 模式** — list 仍返 3 tool, call 返 Err
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            handler: None,
        }
    }

    /// **委托模式** — call 走 caller 提供的 handler (真跑)
    pub fn with_handler(handler: Box<dyn EvalToolCallHandler>) -> Self {
        Self {
            enabled: true,
            handler: Some(handler),
        }
    }

    /// 是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// **3 tool 名字** (per MCP tools/list item)
    pub const TOOL_SMOKE: &'static str = "eval_smoke";
    pub const TOOL_REAL_LLM: &'static str = "eval_real_llm";
    pub const TOOL_CROSS_MODEL: &'static str = "eval_cross_model";

    /// **构造 1 个 Tool** (per kind)
    fn make_tool(name: &str, description: &str, params_schema: Value) -> Tool {
        Tool::new(name)
            .with_description(description)
            .with_input_schema(params_schema)
    }
}

// ============================================================
// ToolServer trait impl
// ============================================================

impl ToolServer for EvalToolServer {
    fn list(&self) -> Vec<Tool> {
        if !self.enabled {
            return Vec::new();
        }
        vec![
            Self::make_tool(
                Self::TOOL_SMOKE,
                "Run a smoke task eval (0 LLM, stub F, 7 阶段 metric per smoke_task.rs)",
                json!({
                    "type": "object",
                    "properties": {
                        "workspace_root": {
                            "type": "string",
                            "description": "workspace root path (default: cwd)"
                        }
                    },
                    "required": []
                }),
            ),
            Self::make_tool(
                Self::TOOL_REAL_LLM,
                "Run real LLM smoke eval (1 round-trip MiniMax Anthropic API, APEIRETH_EVAL_LIVE=1 required)",
                json!({
                    "type": "object",
                    "properties": {
                        "model": {
                            "type": "string",
                            "description": "model id (default: MiniMax-M2.7-highspeed)"
                        },
                        "user_message": {
                            "type": "string",
                            "description": "user message to send (default: smoke prompt)"
                        }
                    },
                    "required": []
                }),
            ),
            Self::make_tool(
                Self::TOOL_CROSS_MODEL,
                "Run cross-model benchmark on N models (per cross_model_benchmark.rs, returns markdown report)",
                json!({
                    "type": "object",
                    "properties": {
                        "models": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "list of model ids (default: DEFAULT_MODELS + EXTENDED_MODELS)"
                        },
                        "prompt": {
                            "type": "string",
                            "description": "benchmark prompt (default: DEFAULT_BENCHMARK_PROMPT)"
                        },
                        "max_tokens": {
                            "type": "integer",
                            "description": "max tokens per response (default: 512)"
                        }
                    },
                    "required": []
                }),
            ),
        ]
    }

    fn call(&self, name: &str, arguments: &Value) -> Result<ToolCallResult, JsonRpcError> {
        if !self.enabled {
            return Err(JsonRpcError::new(
                TOOL_NOT_FOUND,
                format!("EvalToolServer disabled, cannot call `{}`", name),
            ));
        }

        // 委托 handler
        if let Some(handler) = &self.handler {
            return handler.call(name, arguments);
        }

        // metadata 模式: 返 stub 报告
        match name {
            Self::TOOL_SMOKE => {
                let text = format!(
                    "[metadata mode] eval_smoke (stub) — args={}\n\n\
                     真跑需 EvalToolServer::with_handler(handler), handler 实现 EvalToolCallHandler.\n\
                     7 阶段 metric: setup_ok / prompt_built / tool_loop_init / tool_call_dispatched\n\
                     / tool_result_digested / final_reply_correct / no_regression",
                    arguments
                );
                Ok(ToolCallResult::ok(vec![ToolContent::text_with_mime(
                    text,
                    "application/json",
                )]))
            }
            Self::TOOL_REAL_LLM => {
                let text = format!(
                    "[metadata mode] eval_real_llm (stub) — args={}\n\n\
                     真跑需 APEIRETH_EVAL_LIVE=1 env + handler 注入. 7 阶段:\n\
                     apikey_loaded / conventions_scanned / prompt_built / http_request_ok\n\
                     / response_shape_valid / content_non_empty / token_usage_recorded",
                    arguments
                );
                Ok(ToolCallResult::ok(vec![ToolContent::text_with_mime(
                    text,
                    "application/json",
                )]))
            }
            Self::TOOL_CROSS_MODEL => {
                let text = format!(
                    "[metadata mode] eval_cross_model (stub) — args={}\n\n\
                     真跑需 handler 注入. 返 Markdown 报告: per model latency / token / pass status.",
                    arguments
                );
                Ok(ToolCallResult::ok(vec![ToolContent::text_with_mime(
                    text,
                    "application/json",
                )]))
            }
            _ => Err(JsonRpcError::new(
                TOOL_NOT_FOUND,
                format!("eval tool `{}` not found (known: eval_smoke / eval_real_llm / eval_cross_model)", name),
            )),
        }
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoHandler;
    impl EvalToolCallHandler for EchoHandler {
        fn call(&self, name: &str, arguments: &Value) -> Result<ToolCallResult, JsonRpcError> {
            Ok(ToolCallResult::ok(vec![ToolContent::text_with_mime(
                format!("handler: name={}, args={}", name, arguments),
                "application/json",
            )]))
        }
    }

    struct AlwaysFailHandler;
    impl EvalToolCallHandler for AlwaysFailHandler {
        fn call(&self, _name: &str, _arguments: &Value) -> Result<ToolCallResult, JsonRpcError> {
            Err(JsonRpcError::new(-32099, "handler always fails (test)"))
        }
    }

    #[test]
    fn tool_server_default_enabled() {
        let s = EvalToolServer::default();
        assert!(s.is_enabled());
        assert_eq!(s.list().len(), 3);
    }

    #[test]
    fn tool_server_new_enabled() {
        let s = EvalToolServer::new();
        assert!(s.is_enabled());
    }

    #[test]
    fn tool_server_disabled_mode_list_empty() {
        let s = EvalToolServer::disabled();
        assert!(!s.is_enabled());
        assert_eq!(s.list().len(), 0);
    }

    #[test]
    fn list_three_tools() {
        let s = EvalToolServer::new();
        let tools = s.list();
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&EvalToolServer::TOOL_SMOKE));
        assert!(names.contains(&EvalToolServer::TOOL_REAL_LLM));
        assert!(names.contains(&EvalToolServer::TOOL_CROSS_MODEL));
    }

    #[test]
    fn list_tools_have_descriptions() {
        let s = EvalToolServer::new();
        for t in s.list() {
            assert!(t.description.is_some());
            assert!(t.input_schema.is_some());
        }
    }

    #[test]
    fn tool_constants() {
        assert_eq!(EvalToolServer::TOOL_SMOKE, "eval_smoke");
        assert_eq!(EvalToolServer::TOOL_REAL_LLM, "eval_real_llm");
        assert_eq!(EvalToolServer::TOOL_CROSS_MODEL, "eval_cross_model");
    }

    #[test]
    fn call_metadata_mode_smoke() {
        let s = EvalToolServer::new();
        let result = s.call(EvalToolServer::TOOL_SMOKE, &json!({})).unwrap();
        assert!(!result.is_error);
        match &result.content[0] {
            ToolContent::Text { text, .. } => {
                assert!(text.contains("metadata mode"));
                assert!(text.contains("eval_smoke"));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn call_metadata_mode_real_llm() {
        let s = EvalToolServer::new();
        let result = s
            .call(
                EvalToolServer::TOOL_REAL_LLM,
                &json!({"model": "MiniMax-M3"}),
            )
            .unwrap();
        assert!(!result.is_error);
        match &result.content[0] {
            ToolContent::Text { text, .. } => {
                assert!(text.contains("APEIRETH_EVAL_LIVE"));
                assert!(text.contains("MiniMax-M3"));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn call_metadata_mode_cross_model() {
        let s = EvalToolServer::new();
        let result = s
            .call(
                EvalToolServer::TOOL_CROSS_MODEL,
                &json!({"models": ["a", "b"]}),
            )
            .unwrap();
        assert!(!result.is_error);
    }

    #[test]
    fn call_unknown_tool_errors() {
        let s = EvalToolServer::new();
        let err = s.call("eval_unknown", &json!({})).unwrap_err();
        assert_eq!(err.code, TOOL_NOT_FOUND);
    }

    #[test]
    fn call_disabled_errors() {
        let s = EvalToolServer::disabled();
        let err = s.call(EvalToolServer::TOOL_SMOKE, &json!({})).unwrap_err();
        assert_eq!(err.code, TOOL_NOT_FOUND);
    }

    #[test]
    fn with_handler_delegates_call() {
        let s = EvalToolServer::with_handler(Box::new(EchoHandler));
        let result = s
            .call(EvalToolServer::TOOL_SMOKE, &json!({"workspace": "/tmp"}))
            .unwrap();
        assert!(!result.is_error);
        match &result.content[0] {
            ToolContent::Text { text, .. } => {
                assert!(text.contains("handler:"));
                assert!(text.contains("eval_smoke"));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn handler_can_fail() {
        let s = EvalToolServer::with_handler(Box::new(AlwaysFailHandler));
        let err = s.call(EvalToolServer::TOOL_SMOKE, &json!({})).unwrap_err();
        assert_eq!(err.code, -32099);
    }

    #[test]
    fn debug_impl_works() {
        let s = EvalToolServer::new();
        let str = format!("{:?}", s);
        assert!(str.contains("EvalToolServer"));
        assert!(str.contains("enabled"));
    }
}
