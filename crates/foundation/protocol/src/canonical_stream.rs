//! C1 规范流帧 schema (后端合同 C 批 · C1 段)。
//!
//! 目标形态 (设计稿 §C + 裁决 15/16/17):
//! - 两族帧: [`C1_DELTA_EVENT`] (`apeireth.c1.delta.v1`, 运行中增量) 与
//!   [`C1_FINAL_EVENT`] (`apeireth.c1.final.v1`, 终帧);
//! - 帧身份 = (session_id, request_id) + 可选 trace_id; 帧序号 `sequence` 由运行时
//!   权威缓冲分配 (从 1 单调递增, 见 `apeireth-runtime` 的 `canonical::canonical_stream`);
//! - 终帧 `state` ∈ {completed, approval, failed, unknown}; `resync_required` 是
//!   **显式事实** (客户端落后于环形窗口时的事实帧), 不是错误码、不是异常。
//!
//! 不变量 (设计稿 §C):
//! - 回执是唯一完成事实 ⇒ 终帧是完成事实的唯一载体;
//! - `[DONE]` 只表示通道关闭 ⇒ 本模块**不引入** `[DONE]` 语义, 也不产出该字面量。
//!
//! 不做什么 (诚实边界):
//! - 不生成 `incarnation` / `context_key` 等宿主侧标签, 只透传 `trace_id`;
//! - 不自造 `operation` 字段 (设计稿标为「回显位」, C1 段不引入);
//! - 不做 `/bridge/*` 宿主映射 (由前端负责)。

use serde::{Deserialize, Serialize};

/// 运行中增量帧的事件名 (wire `event` 字段 / SSE `event:` 行)。
pub const C1_DELTA_EVENT: &str = "apeireth.c1.delta.v1";

/// 终帧的事件名 (wire `event` 字段 / SSE `event:` 行)。
pub const C1_FINAL_EVENT: &str = "apeireth.c1.final.v1";

/// 流状态词表 (serde `snake_case`)。
///
/// - `running`: 运行中 (只出现在增量帧);
/// - `completed`: 正常完成;
/// - `approval`: 停在审批 (终帧携带待批清单);
/// - `failed`: 失败 (终帧携带 `error`);
/// - `unknown`: 状态不可知 (含 `resync_required` 事实帧)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum C1StreamState {
    Running,
    Completed,
    Approval,
    Failed,
    Unknown,
}

impl C1StreamState {
    /// 该状态是否意味着「这条流已经到终态」。
    ///
    /// 终态帧只能出现一次 (终帧只允许发布一次) ⇒ 这是判定「流已结束」的唯一口径。
    #[must_use]
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Running)
    }

    /// 该状态的 wire 字面量 (与 serde 输出一致)。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Approval => "approval",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

/// 一次请求的用量回执 (终帧 `usage` 位)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct C1Usage {
    /// 输入 (prompt) token 数。
    pub prompt_tokens: u64,
    /// 输出 (completion) token 数。
    pub completion_tokens: u64,
    /// 总 token 数 (输入 + 输出)。
    pub total_tokens: u64,
}

impl C1Usage {
    /// 由输入/输出 token 数构造 (总量按相加给出, 不接受外部自报总量)。
    #[must_use]
    pub fn new(prompt_tokens: u64, completion_tokens: u64) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

/// 一条待批审批 (终帧 `approvals` 位)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct C1Approval {
    /// 审批 id (与运行时审批面同一 id)。
    pub approval_id: String,
    /// 动作绑定摘要 (审批时冻结; 缺失即空, 不给假摘要)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_binding_digest: Option<String>,
    /// 过期时刻 (epoch 毫秒; 缺失即不限期语义)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<i64>,
}

impl C1Approval {
    /// 构造一条待批审批 (摘要/过期时刻缺省即空)。
    pub fn new(approval_id: impl Into<String>) -> Self {
        Self {
            approval_id: approval_id.into(),
            action_binding_digest: None,
            expires_at_ms: None,
        }
    }
}

/// 运行中增量帧 (`apeireth.c1.delta.v1`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct C1DeltaFrame {
    /// 恒为 `apeireth.c1.delta.v1`。
    pub event: String,
    /// 会话 id。
    pub session_id: String,
    /// 请求 id (与 session_id 一起构成帧身份)。
    pub request_id: String,
    /// 追踪 id (只透传, 后端不生成)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// 帧序号 (运行时权威缓冲分配, 从 1 单调递增)。
    pub sequence: u64,
    /// 增量文本。
    pub delta: String,
    /// 运行状态 (增量帧恒为 `running`)。
    pub state: C1StreamState,
}

impl C1DeltaFrame {
    /// 构造一帧增量 (事件名与状态由本类型固定, 序号由调用方给定)。
    pub fn new(
        session_id: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: Option<&str>,
        sequence: u64,
        delta: impl Into<String>,
    ) -> Self {
        Self {
            event: C1_DELTA_EVENT.to_string(),
            session_id: session_id.into(),
            request_id: request_id.into(),
            trace_id: trace_id.map(str::to_string),
            sequence,
            delta: delta.into(),
            state: C1StreamState::Running,
        }
    }
}

/// 终帧 (`apeireth.c1.final.v1`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct C1FinalFrame {
    /// 恒为 `apeireth.c1.final.v1`。
    pub event: String,
    /// 会话 id。
    pub session_id: String,
    /// 请求 id (与 session_id 一起构成帧身份)。
    pub request_id: String,
    /// 追踪 id (只透传, 后端不生成)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// 帧序号 (运行时权威缓冲分配, 从 1 单调递增)。
    pub sequence: u64,
    /// 终态 (completed / approval / failed / unknown)。
    pub state: C1StreamState,
    /// 完整文本 (有则给出; 无则缺省)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// 用量回执 (有则给出)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<C1Usage>,
    /// 待批清单 (终态为 `approval` 时给出)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approvals: Option<Vec<C1Approval>>,
    /// 失败原因 (终态为 `failed` 时给出)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 落后事实位: 客户端请求的序号已早于环形窗口 ⇒ 该事实帧显式告知
    /// 「需要重同步」, **不是错误** (裁决 17)。
    #[serde(default)]
    pub resync_required: bool,
}

impl C1FinalFrame {
    /// 构造一帧终帧 (事件名由本类型固定; 序号由运行时权威缓冲分配后回填)。
    pub fn new(
        session_id: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: Option<&str>,
        state: C1StreamState,
    ) -> Self {
        Self {
            event: C1_FINAL_EVENT.to_string(),
            session_id: session_id.into(),
            request_id: request_id.into(),
            trace_id: trace_id.map(str::to_string),
            sequence: 0,
            state,
            text: None,
            usage: None,
            approvals: None,
            error: None,
            resync_required: false,
        }
    }
}

/// C1 帧族 (运行中增量 | 终帧) 的运行时容器。
///
/// **wire 透明**: 本枚举只做进程内分类, 序列化时**原样透传**内层帧
/// (内层帧自带 `event` 字段) ⇒ SSE `data:` 行就是帧本身, 不带额外包装层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum C1Frame {
    Delta(C1DeltaFrame),
    Final(C1FinalFrame),
}

impl C1Frame {
    /// 帧的 `event` 名 (增量 / 终帧二选一)。
    #[must_use]
    pub fn event_name(&self) -> &str {
        match self {
            Self::Delta(frame) => &frame.event,
            Self::Final(frame) => &frame.event,
        }
    }

    /// 帧序号 (权威缓冲分配)。
    #[must_use]
    pub fn sequence(&self) -> u64 {
        match self {
            Self::Delta(frame) => frame.sequence,
            Self::Final(frame) => frame.sequence,
        }
    }

    /// 是否是终帧 (终帧只允许出现一次)。
    #[must_use]
    pub fn is_final(&self) -> bool {
        matches!(self, Self::Final(_))
    }

    /// 会话 id。
    #[must_use]
    pub fn session_id(&self) -> &str {
        match self {
            Self::Delta(frame) => &frame.session_id,
            Self::Final(frame) => &frame.session_id,
        }
    }

    /// 请求 id。
    #[must_use]
    pub fn request_id(&self) -> &str {
        match self {
            Self::Delta(frame) => &frame.request_id,
            Self::Final(frame) => &frame.request_id,
        }
    }
}

impl Serialize for C1Frame {
    /// 透传内层帧的 wire 形态 (不额外加 `kind` 之类的包装字段)。
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Delta(frame) => frame.serialize(serializer),
            Self::Final(frame) => frame.serialize(serializer),
        }
    }
}
