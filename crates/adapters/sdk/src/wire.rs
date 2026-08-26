//! WireFormat: JSON envelope + 跨语言一致的字段命名.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::version::SdkVersion;

/// Wire kind (envelope 的 kind 字段).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireKind {
    /// LLM chat 调用
    Chat,
    /// 工具调用
    ToolCall,
    /// 记忆读取
    MemoryRead,
    /// 健康检查
    Health,
    /// 自由扩展 (kind 落在 noneof 时落这里)
    Other(String),
}

/// 顶层 envelope: 所有 wire-format 消息都套这层.
///
/// 字段约定 (跨语言一致):
/// - `v`     : SDK 版本 (semver 字符串)
/// - `kind`  : 消息类型 (snake_case)
/// - `id`    : request/response correlation id
/// - `body`  : 实际 payload (serde_json::Value, 由各 kind 自行解析)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    /// 协议版本
    pub v: String,
    /// 消息类型
    pub kind: WireKind,
    /// 相关 id
    pub id: String,
    /// payload
    pub body: Value,
}

impl Envelope {
    /// 构造一个 envelope (自动填默认 v).
    pub fn new(kind: WireKind, id: impl Into<String>, body: Value) -> Self {
        Self {
            v: crate::version::SDK_VERSION.as_str(),
            kind,
            id: id.into(),
            body,
        }
    }

    /// 构造带自定义 SDK 版本的 envelope.
    pub fn with_version(kind: WireKind, id: impl Into<String>, body: Value, v: SdkVersion) -> Self {
        Self {
            v: v.as_str(),
            kind,
            id: id.into(),
            body,
        }
    }

    /// 字符串化 (snake_case) — 给 HTTP / ws 发送用.
    pub fn encode(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 用 `current SDK` 默认版本反序列化.
    pub fn decode(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }

    /// 提取并校验版本: 解析 `v` 字段, 与 expected 比对.
    pub fn expected_version(&self) -> Option<SdkVersion> {
        SdkVersion::parse(&self.v)
    }
}
