//! ProtocolAdapter trait — 4 协议共同接口
//!
//! **设计目标**: 每个 adapter 实现两个方法
//! - `adapt_request`: NormalizedRequest → 协议特定 JSON
//! - `adapt_response`: 协议特定 JSON → NormalizedResponse
//!
//! **借鉴 VCP 真代码**:
//! - 归一化边界: `routes/protocolBridge.js:91-118` `extractProtectedTools`
//!   (Gemini functionDeclarations + legacy functions 只前向传递, 不进 messages)
//! - 工具归一化: `routes/protocolBridge.js:63-89` `toOpenAiChatTool` 3 步判定
//!   (OpenAI function 包装 / Anthropic 裸名 / 参数回退链)

use crate::error::ProtocolError;
use crate::normalized::{NormalizedRequest, NormalizedResponse};
use serde_json::Value;

/// 协议 adapter 共同 trait。
///
/// **设计原则** (借鉴 VCP 真代码的归一化哲学):
/// - **零耦合** — adapter 之间不互相依赖, 只通过 NormalizedRequest / Response
/// - **透明** — `adapt_request` 输出可直接 POST 给协议端点
/// - **健壮** — `adapt_response` 容忍字段缺失, 用 ProtocolError 表达
pub trait ProtocolAdapter: Send + Sync {
    /// Adapter 名 (用于 router dispatch)
    fn name(&self) -> &'static str;

    /// 协议端点路径 (POST)
    fn endpoint_path(&self) -> &'static str;

    /// 归一化请求 → 协议特定 JSON
    fn adapt_request(&self, req: &NormalizedRequest) -> Result<Value, ProtocolError>;

    /// 协议特定 JSON → 归一化响应
    fn adapt_response(&self, raw: &Value) -> Result<NormalizedResponse, ProtocolError>;
}
