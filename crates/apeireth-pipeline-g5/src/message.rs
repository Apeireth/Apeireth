//! # PipelineMessage — 通用 5 阶段 pipeline 规范 I/O 类型
//!
//! 一个 demo 用的 canonical I/O 类型, **不**是 5 阶段 pipeline 强制的输入/输出类型.
//! 用户的 `Stage<I, O>` impl 可以用任何满足 `Send + 'static` 的类型.
//!
//! ## 设计意图
//!
//! 借鉴 Golutra v0.1.0 `chat_db` 通用 message envelope (4 字段: kind / payload / metadata / trace),
//! 简化通用化到 4 字段:
//! - `kind: String` — Dispatch 路由用
//! - `payload: String` — 实际业务数据
//! - `attempt: u32` — Reliability 阶段重试计数
//! - `trace_id: String` — 跨 stage trace id (给 observability 集成留口子)
//!
//! ## 编译期守门 (3 项, K-1 强校验)
//!
//! 1. `MAX_PAYLOAD_LEN == 64 * 1024` (64 KiB, 防 m3 幻觉无限 payload)
//! 2. `MAX_KIND_LEN == 64` (防 m3 幻觉超长 kind)
//! 3. `MAX_TRACE_ID_LEN == 128` (UUID 36 char + margin)

use serde::{Deserialize, Serialize};

/// **Hardcode #1**: payload 最大长度 (64 KiB, 防 m3 幻觉无限 payload).
pub const MAX_PAYLOAD_LEN: usize = 64 * 1024;

/// **Hardcode #2**: kind 字段最大长度 (64 字符, 防 m3 幻觉超长 kind).
pub const MAX_KIND_LEN: usize = 64;

/// **Hardcode #3**: trace_id 字段最大长度 (128 字符, UUID 36 char + margin).
pub const MAX_TRACE_ID_LEN: usize = 128;

/// 通用 pipeline message envelope (canonical I/O).
///
/// 字段都加 `#[serde(default)]` 让 JSON 反序列化时缺失字段走默认值,
/// 借鉴 Golutra `chat_db` 通用 message envelope 模式.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineMessage {
    /// 消息分类 (e.g. "chat" / "task" / "memory" / "mcp"), 给 Dispatch stage 路由用.
    #[serde(default)]
    pub kind: String,
    /// 实际业务数据 (e.g. chat prompt / task payload), Normalize / Throttle stage 会处理.
    #[serde(default)]
    pub payload: String,
    /// Reliability 阶段重试计数 (每次 retry +1, 用户可读).
    #[serde(default)]
    pub attempt: u32,
    /// 跨 stage trace id (32-hex char, 简化 UUID), 留 R21+ observability 集成用.
    #[serde(default)]
    pub trace_id: String,
}

impl PipelineMessage {
    /// 创建新 message (attempt=0, trace_id=空字符串).
    pub fn new(kind: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            payload: payload.into(),
            attempt: 0,
            trace_id: String::new(),
        }
    }

    /// 设置 trace_id (链式 builder).
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = trace_id.into();
        self
    }

    /// 验证字段长度 (编译期 hardcode 守门).
    pub fn validate(&self) -> Result<(), String> {
        if self.kind.len() > MAX_KIND_LEN {
            return Err(format!(
                "kind length {} > MAX_KIND_LEN {}",
                self.kind.len(),
                MAX_KIND_LEN
            ));
        }
        if self.payload.len() > MAX_PAYLOAD_LEN {
            return Err(format!(
                "payload length {} > MAX_PAYLOAD_LEN {}",
                self.payload.len(),
                MAX_PAYLOAD_LEN
            ));
        }
        if self.trace_id.len() > MAX_TRACE_ID_LEN {
            return Err(format!(
                "trace_id length {} > MAX_TRACE_ID_LEN {}",
                self.trace_id.len(),
                MAX_TRACE_ID_LEN
            ));
        }
        Ok(())
    }
}

impl Default for PipelineMessage {
    fn default() -> Self {
        Self::new("", "")
    }
}

/// 编译期守门: MAX_PAYLOAD_LEN == 65536.
const _: () = assert!(MAX_PAYLOAD_LEN == 64 * 1024);
/// 编译期守门: MAX_KIND_LEN == 64.
const _: () = assert!(MAX_KIND_LEN == 64);
/// 编译期守门: MAX_TRACE_ID_LEN == 128.
const _: () = assert!(MAX_TRACE_ID_LEN == 128);
