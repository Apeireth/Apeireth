//! MCP prompts/list + prompts/get models.
//!
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
        // 模板插值：**单遍**（值**不**再被当作模板解释）。
        //
        // 契约（`with_prompt` 文档）：「`{key}` is replaced with `arguments[key]`
        // (stringified) at get-time」—— 「值被**代入**」的字面读法就是**单遍**。
        // **旧实现是逐键 `replace` 迭代** ⇒ 被替换进去的**值会被后续键再次当模板解释**：
        // 模板 `{a}` + 参数 `{"a":"{b}","b":"SECRET"}` ⇒ 渲染出 `"SECRET"`（而非 `"{b}"`），
        // 且**是否展开取决于参数名的字典序** ⇒ 行为任意、未文档化。
        // ⇒ 收窄成**单遍**：一次遍历模板，遇 `{key}` 就查表代入，**代入的值不被再解释**。
        let mut rendered = String::with_capacity(template.len());
        if let Some(obj) = arguments.as_object() {
            let mut rest = template.as_str();
            while let Some(open) = rest.find('{') {
                rendered.push_str(&rest[..open]);
                let after = &rest[open..];
                let closed = after.find('}').filter(|close| {
                    // 占位符名必须是**完整**的 `{key}`（key 不含 `{`），
                    // 否则原样保留（与旧 `replace` 的「找不到就不动」同语义）。
                    !after[1..*close].contains('{')
                });
                match closed {
                    Some(close) => {
                        let key = &after[1..close];
                        match obj.get(key) {
                            Some(value) => {
                                let replacement = match value {
                                    Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                };
                                rendered.push_str(&replacement);
                            }
                            // 未知占位符 ⇒ **原样保留整个 `{key}`**（`{key}` 不是参数表里的键）。
                            None => {
                                rendered.push_str(&after[..close + 1]);
                            }
                        }
                        rest = &after[close + 1..];
                    }
                    None => {
                        rendered.push('{');
                        rest = after;
                    }
                }
            }
            rendered.push_str(rest);
        } else {
            rendered = template.clone();
        }
        Ok(
            GetPromptResult::new(vec![PromptMessage::user_text(rendered)])
                .with_description(prompt.description.clone().unwrap_or_default()),
        )
    }
}

#[cfg(test)]

mod tests {

    /// 取渲染出的文本（`PromptMessage.content` 是 `ContentBlock` 枚举）。
    fn rendered_text(result: &GetPromptResult) -> String {
        match &result.messages[0].content {
            ContentBlock::Text { text, .. } => text.clone(),
            other => panic!("提示词消息应是文本块: {other:?}"),
        }
    }

    /// 模板插值是**单遍**的：代入的**值不再被当作模板解释**。
    ///
    /// 契约（`with_prompt` 文档）：「`{key}` is replaced with `arguments[key]`
    /// (stringified) at get-time」—— 值被**代入**，不是被**再求值**。
    /// 旧实现逐键 `replace` 迭代 ⇒ 值会被后续键二次解释
    /// （模板 `{a}` + 参数 `{"a":"{b}","b":"SECRET"}` ⇒ 渲染出 `"SECRET"`），
    /// 且**是否展开取决于参数名字典序** ⇒ 行为任意、未文档化。
    #[test]
    fn prompt_template_substitution_is_single_pass() {
        let server = StaticPromptServer::new().with_prompt(
            Prompt {
                name: "demo".to_string(),
                description: None,
                arguments: None,
            },
            "值={a}",
        );
        // ① 值里的 `{b}` **不得**被二次解释。
        let out = server
            .get("demo", &serde_json::json!({"a": "{b}", "b": "SECRET"}))
            .expect("提示词可渲染");
        let text = rendered_text(&out);
        assert_eq!(
            text, "值={b}",
            "代入的值不得被再当模板解释（否则是模板逃逸）: {text}"
        );
        // ② 参数名顺序**不得**影响结果（字典序无关）。
        let out2 = server
            .get("demo", &serde_json::json!({"b": "SECRET", "a": "{b}"}))
            .expect("提示词可渲染");
        assert_eq!(
            rendered_text(&out2),
            "值={b}",
            "渲染结果不得依赖参数名的字典序"
        );
        // ③ 未知占位符原样保留（与旧 `replace`「找不到就不动」同语义）。
        let out3 = server
            .get("demo", &serde_json::json!({"z": "Z"}))
            .expect("提示词可渲染");
        assert_eq!(rendered_text(&out3), "值={a}", "未知占位符必须原样保留");
        // ④ 正常代入仍然逐字生效。
        let out4 = server
            .get("demo", &serde_json::json!({"a": "OK"}))
            .expect("提示词可渲染");
        assert_eq!(rendered_text(&out4), "值=OK");
    }
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
