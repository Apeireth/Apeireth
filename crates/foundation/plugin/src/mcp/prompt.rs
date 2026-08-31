//! MCP prompts/list + prompts/get models.
//!
//! Engine: `legacy/canonical/apeireth-mcp/src/prompts.rs`.
//!
//! Library primitives only. Rendering is injected via [`PromptServer`];
//! this crate does not own prompt templates or call an LLM.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::mcp::jsonrpc::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::mcp::schema::ContentBlock;

/// Prompt template not found.
pub const PROMPT_NOT_FOUND: i32 = -32020;
/// Missing / malformed arguments.
pub const PROMPT_INVALID_ARGS: i32 = -32021;
/// Render failed.
pub const PROMPT_RENDER_FAILED: i32 = -32022;

/// MCP Prompt template (spec §prompts/list item).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prompt {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

impl Prompt {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            arguments: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_arguments(mut self, args: Vec<PromptArgument>) -> Self {
        self.arguments = Some(args);
        self
    }
}

/// Prompt argument declaration (spec §prompts/list item.arguments[]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptArgument {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

impl PromptArgument {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            required: false,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Prompt message (spec §prompts/get messages[]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: PromptContent,
}

impl PromptMessage {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: PromptRole::User,
            content: ContentBlock::text(text),
        }
    }

    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self {
            role: PromptRole::Assistant,
            content: ContentBlock::text(text),
        }
    }
}

/// Role enum (spec: `"user"` | `"assistant"`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    User,
    Assistant,
}

/// Prompt content. Same wire shape as [`ContentBlock`] (`mimeType` camelCase).
pub type PromptContent = ContentBlock;

/// prompts/get result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetPromptResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

impl GetPromptResult {
    pub fn new(messages: Vec<PromptMessage>) -> Self {
        Self {
            description: None,
            messages,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Server-side prompt provider. Injected; this crate does no I/O.
pub trait PromptServer: Send + Sync {
    fn list(&self) -> Vec<Prompt>;
    fn get(&self, name: &str, arguments: &Value) -> Result<GetPromptResult, JsonRpcError>;
}

/// Handle `prompts/list`.
pub fn handle_prompts_list(req: &JsonRpcRequest, server: &dyn PromptServer) -> JsonRpcResponse {
    let prompts = server.list();
    JsonRpcResponse::ok(req.id.clone(), json!({ "prompts": prompts }))
}

/// Handle `prompts/get`. Params: `{name: string, arguments?: object}`.
pub fn handle_prompts_get(req: &JsonRpcRequest, server: &dyn PromptServer) -> JsonRpcResponse {
    let Some(params) = req.params.as_ref() else {
        return JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(PROMPT_INVALID_ARGS, "params missing"),
        );
    };
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::err(
                req.id.clone(),
                JsonRpcError::new(PROMPT_INVALID_ARGS, "params.name missing or not string"),
            );
        }
    };
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
    match server.get(&name, &arguments) {
        Ok(result) => JsonRpcResponse::ok(
            req.id.clone(),
            json!({
                "description": result.description,
                "messages": result.messages,
            }),
        ),
        Err(e) => JsonRpcResponse::err(req.id.clone(), e),
    }
}

/// Route `prompts/list` / `prompts/get`; unknown → −32601.
pub fn dispatch(req: &JsonRpcRequest, server: &dyn PromptServer) -> JsonRpcResponse {
    match req.method.as_str() {
        "prompts/list" => handle_prompts_list(req, server),
        "prompts/get" => handle_prompts_get(req, server),
        other => JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(
                JsonRpcError::CODE_METHOD_NOT_FOUND,
                format!("unknown prompts method: {other}"),
            ),
        ),
    }
}

/// In-memory prompt table. `get` interpolates `{name}` placeholders in a
/// stored user-text template; unknown names → [`PROMPT_NOT_FOUND`].
#[derive(Debug, Clone, Default)]
pub struct StaticPromptServer {
    prompts: Vec<(Prompt, String)>,
}

impl StaticPromptServer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a template. `template` is the user-message body; `{key}`
    /// is replaced with `arguments[key]` (stringified) at get-time.
    pub fn with_prompt(mut self, prompt: Prompt, template: impl Into<String>) -> Self {
        self.prompts.push((prompt, template.into()));
        self
    }
}

impl PromptServer for StaticPromptServer {
    fn list(&self) -> Vec<Prompt> {
        self.prompts.iter().map(|(p, _)| p.clone()).collect()
    }

    fn get(&self, name: &str, arguments: &Value) -> Result<GetPromptResult, JsonRpcError> {
        let Some((prompt, template)) = self.prompts.iter().find(|(p, _)| p.name == name) else {
            return Err(JsonRpcError::new(
                PROMPT_NOT_FOUND,
                format!("prompt `{name}` not found"),
            ));
        };
        if let Some(declared) = &prompt.arguments {
            for arg in declared {
                if arg.required && arguments.get(&arg.name).is_none() {
                    return Err(JsonRpcError::new(
                        PROMPT_INVALID_ARGS,
                        format!("missing required argument `{}`", arg.name),
                    ));
                }
            }
        }
        let mut rendered = template.clone();
        if let Some(obj) = arguments.as_object() {
            for (k, v) in obj {
                let needle = format!("{{{k}}}");
                let replacement = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                rendered = rendered.replace(&needle, &replacement);
            }
        }
        Ok(
            GetPromptResult::new(vec![PromptMessage::user_text(rendered)])
                .with_description(prompt.description.clone().unwrap_or_default()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::jsonrpc::Id;

    struct TestPromptServer;
    impl PromptServer for TestPromptServer {
        fn list(&self) -> Vec<Prompt> {
            vec![
                Prompt::new("summarize")
                    .with_description("Summarize a topic")
                    .with_arguments(vec![
                        PromptArgument::new("topic")
                            .required()
                            .with_description("Topic to summarize"),
                        PromptArgument::new("max_words").with_description("Max word count"),
                    ]),
                Prompt::new("greet").with_description("Say hello"),
            ]
        }
        fn get(&self, name: &str, arguments: &Value) -> Result<GetPromptResult, JsonRpcError> {
            match name {
                "summarize" => {
                    let topic = arguments
                        .get("topic")
                        .and_then(|v| v.as_str())
                        .unwrap_or("(no topic)");
                    let max = arguments
                        .get("max_words")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(100);
                    Ok(GetPromptResult::new(vec![
                        PromptMessage::user_text(format!(
                            "Please summarize `{topic}` in at most {max} words."
                        )),
                        PromptMessage::assistant_text("Understood. Here is the summary: ..."),
                    ])
                    .with_description(format!("Rendered summarize for `{topic}`")))
                }
                "greet" => Ok(GetPromptResult::new(vec![PromptMessage::assistant_text(
                    "Hello! How can I help?",
                )])),
                _ => Err(JsonRpcError::new(
                    PROMPT_NOT_FOUND,
                    format!("prompt `{name}` not found"),
                )),
            }
        }
    }

    #[test]
    fn prompt_new_and_with() {
        let p = Prompt::new("x")
            .with_description("d")
            .with_arguments(vec![PromptArgument::new("a").required()]);
        assert_eq!(p.name, "x");
        assert_eq!(p.description.as_deref(), Some("d"));
        let args = p.arguments.unwrap();
        assert_eq!(args.len(), 1);
        assert!(args[0].required);
    }

    #[test]
    fn prompt_role_serde_round_trip() {
        assert_eq!(
            serde_json::to_value(&PromptRole::User).unwrap(),
            json!("user")
        );
        assert_eq!(
            serde_json::to_value(&PromptRole::Assistant).unwrap(),
            json!("assistant")
        );
    }

    #[test]
    fn prompt_content_uses_mime_type_camel_case() {
        let m = PromptMessage {
            role: PromptRole::User,
            content: ContentBlock::text_with_mime("hi", "text/plain"),
        };
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["role"], "user");
        assert_eq!(v["content"]["type"], "text");
        assert_eq!(v["content"]["mimeType"], "text/plain");
    }

    #[test]
    fn prompt_server_list_and_get() {
        let s = TestPromptServer;
        assert_eq!(s.list().len(), 2);
        let result = s
            .get(
                "summarize",
                &json!({"topic": "Rust async", "max_words": 50}),
            )
            .unwrap();
        assert_eq!(result.messages.len(), 2);
        match &result.messages[0].content {
            ContentBlock::Text { text, .. } => {
                assert!(text.contains("Rust async"));
                assert!(text.contains("50"));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn prompt_server_get_unknown_errors() {
        let err = TestPromptServer.get("nope", &json!({})).unwrap_err();
        assert_eq!(err.code, PROMPT_NOT_FOUND);
    }

    #[test]
    fn handle_prompts_list_returns_json_rpc_ok() {
        let req = JsonRpcRequest::new("prompts/list", None, Id::Num(1));
        let resp = handle_prompts_list(&req, &TestPromptServer);
        assert!(resp.error.is_none());
        let result = resp.into_result().unwrap();
        let prompts = result.get("prompts").and_then(|v| v.as_array()).unwrap();
        assert_eq!(prompts.len(), 2);
    }

    #[test]
    fn handle_prompts_get_with_name_returns_messages() {
        let params = json!({"name": "greet"});
        let req = JsonRpcRequest::new("prompts/get", Some(params), Id::Num(2));
        let resp = handle_prompts_get(&req, &TestPromptServer);
        let result = resp.into_result().unwrap();
        let messages = result.get("messages").and_then(|v| v.as_array()).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "assistant");
    }

    #[test]
    fn handle_prompts_get_missing_name_errors() {
        let req = JsonRpcRequest::new("prompts/get", None, Id::Num(3));
        let resp = handle_prompts_get(&req, &TestPromptServer);
        assert_eq!(resp.error.unwrap().code, PROMPT_INVALID_ARGS);
    }

    #[test]
    fn handle_prompts_get_unknown_name_errors() {
        let params = json!({"name": "nope"});
        let req = JsonRpcRequest::new("prompts/get", Some(params), Id::Num(4));
        let resp = handle_prompts_get(&req, &TestPromptServer);
        assert_eq!(resp.error.unwrap().code, PROMPT_NOT_FOUND);
    }

    #[test]
    fn dispatch_known_and_unknown() {
        let s = TestPromptServer;
        let ok = dispatch(&JsonRpcRequest::new("prompts/list", None, Id::Num(5)), &s);
        assert!(ok.error.is_none());
        let bad = dispatch(&JsonRpcRequest::new("prompts/foo", None, Id::Num(6)), &s);
        assert_eq!(bad.error.unwrap().code, JsonRpcError::CODE_METHOD_NOT_FOUND);
    }

    #[test]
    fn static_prompt_server_interpolates_and_requires() {
        let s = StaticPromptServer::new().with_prompt(
            Prompt::new("greet")
                .with_description("hi")
                .with_arguments(vec![PromptArgument::new("who").required()]),
            "Hello {who}!",
        );
        assert_eq!(s.list().len(), 1);
        let missing = s.get("greet", &json!({})).unwrap_err();
        assert_eq!(missing.code, PROMPT_INVALID_ARGS);
        let ok = s.get("greet", &json!({"who": "Ada"})).unwrap();
        match &ok.messages[0].content {
            ContentBlock::Text { text, .. } => assert_eq!(text, "Hello Ada!"),
            _ => panic!("expected Text"),
        }
        assert_eq!(
            s.get("nope", &json!({})).unwrap_err().code,
            PROMPT_NOT_FOUND
        );
    }
}
