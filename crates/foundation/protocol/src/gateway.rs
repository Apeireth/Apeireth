//! `apeireth-protocol::gateway` — **R30 U10 跨协议网关 facade**
//!
//! **目标**: 把 apeireth 现有的 4 类协议统一进 1 个 `ProtocolGateway`:
//! - 4 LLM 协议 (OpenAI Chat / Responses, Anthropic Messages, Gemini)
//! - ACP (apeireth-acp) — Agent envelope
//! - MCP (apeireth-mcp) — 工具/资源 JSON-RPC 2.0
//! - OpenClaw Gateway — 本地 .openclaw/ 服务桥
//!
//! **设计**:
//! - `ProtocolKind` enum 标识 7 种协议
//! - `ProtocolGateway` 持 `HashMap<ProtocolKind, Arc<dyn ProtocolBridge>>`
//! - `dispatch(kind, request)` → bridge.handle(request) → Response
//! - 跨协议消息转换走 `bridges[kind].adapt_from_normalized(req)` 反向归一
//!
//! **Apeireth 扩展**:
//! - OpenClaw Gateway 是 1 个特殊本地桥 (e.g. 调本地 .openclaw 工作目录里的 service)
//! - 不强制全部 7 协议都注册 (懒加载)
//!
//! **降级**:
//! - 没注册的 kind: 返 `ProtocolError::UnknownKind` (不 panic, 不假装)
//! - bridge.handle 失败: 透传错误

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::ProtocolError;
use crate::normalized::NormalizedRequest;

/// R30 U10: 7 种协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolKind {
    /// OpenAI Chat Completions (/v1/chat/completions)
    OpenAiChat,
    /// OpenAI Responses API (/v1/responses)
    OpenAiResponses,
    /// Anthropic Messages (/v1/messages)
    AnthropicMessages,
    /// Google Gemini generateContent
    Gemini,
    /// ACP — Agent Communication Protocol (apeireth-acp)
    Acp,
    /// MCP — Model Context Protocol (apeireth-mcp)
    Mcp,
    /// OpenClaw Gateway — 本地 .openclaw 服务桥
    OpenClawGateway,
}

impl ProtocolKind {
    /// 全部 7 种
    pub const ALL: &'static [ProtocolKind] = &[
        Self::OpenAiChat,
        Self::OpenAiResponses,
        Self::AnthropicMessages,
        Self::Gemini,
        Self::Acp,
        Self::Mcp,
        Self::OpenClawGateway,
    ];

    /// 协议名 (调试/日志用)
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiChat => "openai-chat",
            Self::OpenAiResponses => "openai-responses",
            Self::AnthropicMessages => "anthropic-messages",
            Self::Gemini => "gemini",
            Self::Acp => "acp",
            Self::Mcp => "mcp",
            Self::OpenClawGateway => "openclaw-gateway",
        }
    }

    /// 协议名解析 (config 字符串解析用, 大小写不敏感)
    ///
    /// **接受**: `as_str()` 的全部返回 + 兼容大小写 (`OpenAI` / `OPENAI_RESPONSES` / `anthropic` / `gemini`)
    /// **拒绝**: 未知字符串返 `None`
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai-chat" | "openai_chat" | "openai" => Some(Self::OpenAiChat),
            "openai-responses" | "openai_responses" | "responses" => Some(Self::OpenAiResponses),
            "anthropic-messages" | "anthropic_messages" | "anthropic" => {
                Some(Self::AnthropicMessages)
            }
            "gemini" => Some(Self::Gemini),
            "acp" => Some(Self::Acp),
            "mcp" => Some(Self::Mcp),
            "openclaw-gateway" | "openclaw_gateway" | "openclaw" => Some(Self::OpenClawGateway),
            _ => None,
        }
    }
}

/// R30 U10: 跨协议 bridge trait
///
/// 每种协议实现这个 trait, 注册到 ProtocolGateway, 走 dispatch 路由.
/// 不强制同步 (用 async fn).
#[async_trait::async_trait]
pub trait ProtocolBridge: Send + Sync {
    /// bridge 名字 (调试用)
    fn name(&self) -> &str;

    /// 该 bridge 支持的协议类型 (1 个 bridge 可对应多个 kind)
    fn kinds(&self) -> Vec<ProtocolKind>;

    /// 处理归一化请求, 返归一化响应
    async fn handle(&self, req: NormalizedRequest) -> Result<NormalizedResponse, ProtocolError>;
}

/// R30 U10: 归一化响应 (跨协议统一格式, 简化版)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedResponse {
    /// 协议类型
    pub kind: ProtocolKind,
    /// 响应内容 (文本)
    pub content: String,
    /// 协议特定的元数据 (e.g. tool_calls, usage)
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl NormalizedResponse {
    pub fn new(kind: ProtocolKind, content: impl Into<String>) -> Self {
        Self {
            kind,
            content: content.into(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// R30 U10: 协议网关 facade
///
/// 持 `HashMap<ProtocolKind, Arc<dyn ProtocolBridge>>`, dispatch 走 map 查表.
pub struct ProtocolGateway {
    bridges: HashMap<ProtocolKind, Arc<dyn ProtocolBridge>>,
}

impl ProtocolGateway {
    pub fn new() -> Self {
        Self {
            bridges: HashMap::new(),
        }
    }

    /// 注册 1 个 bridge (覆盖该 bridge 支持的所有 kind)
    pub fn register(mut self, bridge: Arc<dyn ProtocolBridge>) -> Self {
        for kind in bridge.kinds() {
            self.bridges.insert(kind, bridge.clone());
        }
        self
    }

    /// 拿 1 个 kind 的 bridge
    pub fn get(&self, kind: ProtocolKind) -> Option<&Arc<dyn ProtocolBridge>> {
        self.bridges.get(&kind)
    }

    /// 列出已注册的 kind
    pub fn registered_kinds(&self) -> Vec<ProtocolKind> {
        let mut kinds: Vec<ProtocolKind> = self.bridges.keys().copied().collect();
        kinds.sort_by_key(|k| k.as_str());
        kinds
    }

    /// R30 U10: dispatch 归一请求到对应 bridge
    pub async fn dispatch(
        &self,
        kind: ProtocolKind,
        req: NormalizedRequest,
    ) -> Result<NormalizedResponse, ProtocolError> {
        let bridge = self.bridges.get(&kind).ok_or_else(|| {
            ProtocolError::Internal(format!("protocol not registered: {}", kind.as_str()))
        })?;
        bridge.handle(req).await
    }
}

impl Default for ProtocolGateway {
    fn default() -> Self {
        Self::new()
    }
}

/// R30 U10: OpenClaw Gateway 桥 (本地 .openclaw/ 服务)
///
/// **设计**: 简化 stub, 把请求当成本地命令跑 (e.g. 调本地 binary).
/// 实际生产可换成 HTTP client 调 OpenClaw daemon.
pub struct OpenClawGatewayBridge {
    /// 工作目录 (默认 ~/.openclaw/workspace/promethean)
    pub workspace: String,
}

impl OpenClawGatewayBridge {
    pub fn new(workspace: impl Into<String>) -> Self {
        Self {
            workspace: workspace.into(),
        }
    }
}

impl Default for OpenClawGatewayBridge {
    fn default() -> Self {
        // 默认 .openclaw 工作目录
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".into());
        Self::new(format!("{home}/.openclaw/workspace/promethean"))
    }
}

#[async_trait::async_trait]
impl ProtocolBridge for OpenClawGatewayBridge {
    fn name(&self) -> &str {
        "openclaw-gateway-bridge"
    }

    fn kinds(&self) -> Vec<ProtocolKind> {
        vec![ProtocolKind::OpenClawGateway]
    }

    async fn handle(&self, req: NormalizedRequest) -> Result<NormalizedResponse, ProtocolError> {
        // stub: 拼 echo 响应 (实际生产: 调本地 binary / HTTP / file IO)
        // 把请求的最后一条 user message + workspace 路径回显
        let last_user = req
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, crate::normalized::MessageRole::User))
            .map(|m| crate::normalized::ContentPart::join_text(&m.content))
            .unwrap_or_default();
        Ok(NormalizedResponse::new(
            ProtocolKind::OpenClawGateway,
            format!(
                "[openclaw-gateway stub] workspace={} last_user={}",
                self.workspace, last_user
            ),
        )
        .with_metadata(
            "workspace",
            serde_json::Value::String(self.workspace.clone()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalized::NormalizedMessage;

    fn dummy_req() -> NormalizedRequest {
        NormalizedRequest::new("test", vec![NormalizedMessage::user("hello openclaw")])
    }

    #[test]
    fn protocol_kind_as_str_all_seven() {
        assert_eq!(ProtocolKind::OpenAiChat.as_str(), "openai-chat");
        assert_eq!(ProtocolKind::OpenAiResponses.as_str(), "openai-responses");
        assert_eq!(
            ProtocolKind::AnthropicMessages.as_str(),
            "anthropic-messages"
        );
        assert_eq!(ProtocolKind::Gemini.as_str(), "gemini");
        assert_eq!(ProtocolKind::Acp.as_str(), "acp");
        assert_eq!(ProtocolKind::Mcp.as_str(), "mcp");
        assert_eq!(ProtocolKind::OpenClawGateway.as_str(), "openclaw-gateway");
        assert_eq!(ProtocolKind::ALL.len(), 7);
    }

    #[test]
    fn gateway_register_and_dispatch() {
        let gw = ProtocolGateway::new().register(Arc::new(OpenClawGatewayBridge::default()));
        let kinds = gw.registered_kinds();
        assert!(kinds.contains(&ProtocolKind::OpenClawGateway));
        assert!(gw.get(ProtocolKind::Acp).is_none(), "Acp 未注册");
    }

    #[tokio::test]
    async fn openclaw_bridge_echoes_workspace() {
        let bridge = OpenClawGatewayBridge::new("/tmp/workspace");
        let resp = bridge.handle(dummy_req()).await.expect("handle");
        assert_eq!(resp.kind, ProtocolKind::OpenClawGateway);
        assert!(resp.content.contains("/tmp/workspace"));
        assert!(resp.content.contains("hello openclaw"));
        assert_eq!(
            resp.metadata.get("workspace").and_then(|v| v.as_str()),
            Some("/tmp/workspace")
        );
    }

    #[tokio::test]
    async fn dispatch_unregistered_kind_errors() {
        let gw = ProtocolGateway::new();
        let r = gw.dispatch(ProtocolKind::Acp, dummy_req()).await;
        assert!(r.is_err(), "未注册 kind 应报错");
    }

    #[test]
    fn bridge_can_register_multiple_kinds() {
        struct MultiBridge;
        #[async_trait::async_trait]
        impl ProtocolBridge for MultiBridge {
            fn name(&self) -> &str {
                "multi"
            }
            fn kinds(&self) -> Vec<ProtocolKind> {
                vec![ProtocolKind::Acp, ProtocolKind::Mcp]
            }
            async fn handle(
                &self,
                _req: NormalizedRequest,
            ) -> Result<NormalizedResponse, ProtocolError> {
                Ok(NormalizedResponse::new(ProtocolKind::Acp, "ok"))
            }
        }
        let gw = ProtocolGateway::new().register(Arc::new(MultiBridge));
        assert!(gw.get(ProtocolKind::Acp).is_some());
        assert!(gw.get(ProtocolKind::Mcp).is_some());
    }
}
