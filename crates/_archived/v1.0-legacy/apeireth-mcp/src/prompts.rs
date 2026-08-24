//! R84: MCP `prompts` protocol (MCP spec §prompts/list + §prompts/get)
//!
//! **MCP 协议 (per modelcontextprotocol/specification 2025-03-26)**:
//! - `prompts/list` — 客户端列 server 端 prompt 模板 (name + description + arguments)
//! - `prompts/get` — 客户端按 name + 实际 args 渲染 prompt, server 返 messages[] (对话历史)
//!
//! **Apeireth 真接 (本 module)**:
//! - `Prompt` struct (name + description + arguments[]) — MCP §prompts/list item 1:1
//! - `PromptArgument` (name + description + required) — 1:1
//! - `PromptMessage` (role: user/assistant + content: text/image) — 1:1
//! - `PromptContent` enum (Text/Image/Resource) — 复用 tool content 风格
//! - `PromptServer` trait (`list()` + `get(name, args)`) — server 端抽象
//! - `handle_prompts_list(req, server)` / `handle_prompts_get(req, server)` — JSON-RPC 帮手
//! - `dispatch_prompts(req, server)` — 顶层 dispatcher (跟 resources::dispatch 镜像)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `tools.rs` / `resources.rs` / `protocol.rs` / `subscriptions.rs` (LOCKED)
//! - 0 引入 I/O / 网络 (server 注入, 0 真接)
//! - 0 业务耦合 (任意 server impl 都能挂, SkillPromptServer / CouncilPromptServer 等)
//!
//! **借鉴锚 (S-3)**:
//! - MCP spec 2025-03-26 §prompts (fields 1:1)
//! - LangChain `PromptTemplate.from_template` (arguments → message 渲染)
//! - AutoGen `ConversableAgent.system_message` (role: system/user/assistant)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::protocol::{Id, JsonRpcError, JsonRpcRequest, JsonRpcResponse, JSON_RPC_VERSION};

// ============================================================
// 错误码
// ============================================================

/// MCP 错误码 (per MCP spec, server-define 范围 -32000 ~ -32099)
pub const PROMPT_NOT_FOUND: i32 = -32020;
pub const PROMPT_INVALID_ARGS: i32 = -32021;
pub const PROMPT_RENDER_FAILED: i32 = -32022;

// ============================================================
// 基础类型
// ============================================================

/// **MCP Prompt 模板** (per spec §prompts/list item)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prompt {
    /// 模板名 (e.g. "summarize-conversation")
    pub name: String,
    /// 人类可读描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 模板参数列表 (声明式, get 时实际传值)
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

/// **Prompt 参数声明** (per spec §prompts/list item.arguments[])
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptArgument {
    /// 参数名 (e.g. "topic")
    pub name: String,
    /// 人类可读描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 是否必填
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

// ============================================================
// 消息类型 (per spec §prompts/get result.messages[])
// ============================================================

/// **Prompt 消息** (per spec §prompts/get messages[])
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptMessage {
    /// 角色 (per MCP spec: "user" | "assistant")
    pub role: PromptRole,
    /// 消息内容
    pub content: PromptContent,
}

impl PromptMessage {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: PromptRole::User,
            content: PromptContent::Text {
                text: text.into(),
                mime_type: None,
            },
        }
    }
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self {
            role: PromptRole::Assistant,
            content: PromptContent::Text {
                text: text.into(),
                mime_type: None,
            },
        }
    }
}

/// **角色枚举** (per MCP spec §prompts)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    User,
    Assistant,
}

/// **Prompt 消息内容** (per spec §prompts content 1:1, mirror tools::ToolContent)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptContent {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    Image {
        data: String,
        mime_type: String,
    },
    Resource {
        uri: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
}

impl PromptContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            mime_type: None,
        }
    }
}

/// **Prompt get 结果** (per spec §prompts/get result)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GetPromptResult {
    /// 模板描述 (optional, e.g. 渲染时的注解)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 渲染后的消息列表
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

// ============================================================
// Server trait
// ============================================================

/// **MCP PromptServer trait** (server 端抽象, 跟 ResourceServer / ToolServer 对偶)
pub trait PromptServer: Send + Sync {
    /// 列所有 prompt 模板
    fn list(&self) -> Vec<Prompt>;
    /// 按 name + 实际 args 渲染, 返 messages 列表
    fn get(&self, name: &str, arguments: &Value) -> Result<GetPromptResult, JsonRpcError>;
}

// ============================================================
// Handlers
// ============================================================

/// **处理 `prompts/list` 请求** → JSON-RPC 响应
pub fn handle_prompts_list(req: &JsonRpcRequest, server: &dyn PromptServer) -> JsonRpcResponse {
    let prompts = server.list();
    JsonRpcResponse::ok(req.id.clone(), json!({ "prompts": prompts }))
}

/// **处理 `prompts/get` 请求** → JSON-RPC 响应
///
/// params 必填: `{name: string, arguments?: object}`
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

/// **顶层 dispatcher** (跟 resources::dispatch 镜像)
pub fn dispatch(req: &JsonRpcRequest, server: &dyn PromptServer) -> JsonRpcResponse {
    match req.method.as_str() {
        "prompts/list" => handle_prompts_list(req, server),
        "prompts/get" => handle_prompts_get(req, server),
        other => JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(
                JsonRpcError::CODE_METHOD_NOT_FOUND,
                format!("unknown prompts method: {}", other),
            ),
        ),
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// **测试用 prompt server**: 注册 2 个模板 (1 个带 args + 1 个无)
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
                            "Please summarize `{}` in at most {} words.",
                            topic, max
                        )),
                        PromptMessage::assistant_text(
                            "Understood. Here is the summary: ...".to_string(),
                        ),
                    ])
                    .with_description(format!("Rendered summarize for `{}`", topic)))
                }
                "greet" => Ok(GetPromptResult::new(vec![PromptMessage::assistant_text(
                    "Hello! How can I help?".to_string(),
                )])),
                _ => Err(JsonRpcError::new(
                    PROMPT_NOT_FOUND,
                    format!("prompt `{}` not found", name),
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
    fn prompt_argument_required_and_description() {
        let a = PromptArgument::new("topic")
            .required()
            .with_description("topic desc");
        assert_eq!(a.name, "topic");
        assert!(a.required);
        assert_eq!(a.description.as_deref(), Some("topic desc"));
    }

    #[test]
    fn prompt_message_user_text_constructor() {
        let m = PromptMessage::user_text("hi");
        assert_eq!(m.role, PromptRole::User);
        match m.content {
            PromptContent::Text { text, .. } => assert_eq!(text, "hi"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn prompt_message_assistant_text_constructor() {
        let m = PromptMessage::assistant_text("hello");
        assert_eq!(m.role, PromptRole::Assistant);
        match m.content {
            PromptContent::Text { text, .. } => assert_eq!(text, "hello"),
            _ => panic!("expected Text"),
        }
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
    fn prompt_server_list_returns_two() {
        let s = TestPromptServer;
        let list = s.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "summarize");
        assert_eq!(list[1].name, "greet");
    }

    #[test]
    fn prompt_server_list_includes_arguments() {
        let s = TestPromptServer;
        let list = s.list();
        let args = list[0].arguments.as_ref().unwrap();
        assert_eq!(args.len(), 2);
        assert!(args[0].required);
        assert!(!args[1].required);
    }

    #[test]
    fn prompt_server_get_summarize_with_args() {
        let s = TestPromptServer;
        let result = s
            .get(
                "summarize",
                &json!({"topic": "Rust async", "max_words": 50}),
            )
            .unwrap();
        assert_eq!(result.messages.len(), 2);
        match &result.messages[0].content {
            PromptContent::Text { text, .. } => {
                assert!(text.contains("Rust async"));
                assert!(text.contains("50"));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn prompt_server_get_greet_works() {
        let s = TestPromptServer;
        let result = s.get("greet", &json!({})).unwrap();
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].role, PromptRole::Assistant);
    }

    #[test]
    fn prompt_server_get_unknown_errors() {
        let s = TestPromptServer;
        let err = s.get("nope", &json!({})).unwrap_err();
        assert_eq!(err.code, PROMPT_NOT_FOUND);
    }

    #[test]
    fn handle_prompts_list_returns_json_rpc_ok() {
        let req = JsonRpcRequest::new("prompts/list", None, Id::Num(1));
        let s = TestPromptServer;
        let resp = handle_prompts_list(&req, &s);
        assert!(resp.error.is_none());
        let result = resp.into_result().unwrap();
        let prompts = result.get("prompts").and_then(|v| v.as_array()).unwrap();
        assert_eq!(prompts.len(), 2);
    }

    #[test]
    fn handle_prompts_get_with_name_returns_messages() {
        let params = json!({"name": "greet"});
        let req = JsonRpcRequest::new("prompts/get", Some(params), Id::Num(2));
        let s = TestPromptServer;
        let resp = handle_prompts_get(&req, &s);
        let result = resp.into_result().unwrap();
        let messages = result.get("messages").and_then(|v| v.as_array()).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "assistant");
    }

    #[test]
    fn handle_prompts_get_missing_name_errors() {
        let req = JsonRpcRequest::new("prompts/get", None, Id::Num(3));
        let s = TestPromptServer;
        let resp = handle_prompts_get(&req, &s);
        let err = resp.error.unwrap();
        assert_eq!(err.code, PROMPT_INVALID_ARGS);
    }

    #[test]
    fn handle_prompts_get_unknown_name_errors() {
        let params = json!({"name": "nope"});
        let req = JsonRpcRequest::new("prompts/get", Some(params), Id::Num(4));
        let s = TestPromptServer;
        let resp = handle_prompts_get(&req, &s);
        let err = resp.error.unwrap();
        assert_eq!(err.code, PROMPT_NOT_FOUND);
    }

    #[test]
    fn dispatch_known_method_routes() {
        let req = JsonRpcRequest::new("prompts/list", None, Id::Num(5));
        let s = TestPromptServer;
        let resp = dispatch(&req, &s);
        assert!(resp.error.is_none());
    }

    #[test]
    fn dispatch_unknown_method_errors() {
        let req = JsonRpcRequest::new("prompts/foo", None, Id::Num(6));
        let s = TestPromptServer;
        let resp = dispatch(&req, &s);
        let err = resp.error.unwrap();
        assert_eq!(err.code, JsonRpcError::CODE_METHOD_NOT_FOUND);
    }

    #[test]
    fn prompt_serde_round_trip() {
        let p = Prompt::new("x")
            .with_description("d")
            .with_arguments(vec![PromptArgument::new("a").required()]);
        let s = serde_json::to_string(&p).unwrap();
        let back: Prompt = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }
}
