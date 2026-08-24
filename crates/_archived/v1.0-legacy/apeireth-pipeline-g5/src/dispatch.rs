//! # Dispatch Stage — 5 阶段 pipeline 第 0 阶段 (路由 / 分发)
//!
//! 借鉴 Golutra v0.1.0 `chat_db/pipeline/dispatch.rs` 思想 (per
//! `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P2):
//! - Golutra dispatch: 根据 message kind / channel 路由到不同 handler
//! - 本 crate 通用化: 根据 message 字段路由, 拒绝空 kind
//!
//! ## 1 例子: Dispatch 阶段给 `apeireth-task` 用 (per 任务 spec)
//!
//! 用户给 `apeireth-task` 写自定义 Dispatch stage (示意, **不**引 `apeireth-task` crate dep):
//!
//! ```ignore
//! // 伪代码 — 阶段 6 skeleton 故意不引 workspace dep, 留 R21 续真接
//! use apeireth_task::{TaskKind, TASK_PRIORITY_COUNT};
//! use apeireth_pipeline_g5::{Stage, StageKind, PipelineError, PipelineMessage};
//!
//! pub struct TaskDispatch {
//!     allowed_kinds: Vec<TaskKind>,
//! }
//!
//! impl Stage<PipelineMessage, PipelineMessage> for TaskDispatch {
//!     fn kind(&self) -> StageKind { StageKind::Dispatch }
//!     fn name(&self) -> &str { "task-dispatch" }
//!
//!     fn process(&self, msg: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
//!         // 1. 把 `msg.kind` parse 成 TaskKind (e.g. "critical" → TaskKind::Critical)
//!         let task_kind: TaskKind = msg.kind.parse()
//!             .map_err(|e: ParseError| PipelineError::Stage {
//!                 kind: StageKind::Dispatch,
//!                 source: Box::new(e),
//!             })?;
//!
//!         // 2. 检查是否在白名单
//!         if !self.allowed_kinds.contains(&task_kind) {
//!             return Err(PipelineError::Stage {
//!                 kind: StageKind::Dispatch,
//!                 source: format!("task kind {:?} not in whitelist", task_kind).into(),
//!             });
//!         }
//!
//!         // 3. 路由: 把 kind 写到 msg.payload 前缀, 给后续 stage 用
//!         Ok(PipelineMessage {
//!             kind: msg.kind,
//!             payload: format!("[{}] {}", task_kind, msg.payload),
//!             ..msg
//!         })
//!     }
//! }
//! ```
//!
//! ## 编译期守门 (3 项, K-1 强校验)
//!
//! 1. `DefaultDispatch::KINDS` 编译期数组 (e.g. `["chat", "task", "memory", "mcp"]`)
//! 2. `KINDS.len() <= 16` (上限 16 种 kind, 防 m3 幻觉无限扩展)
//! 3. `kind()` 永远返回 `StageKind::Dispatch`

use std::fmt;

use crate::error::PipelineError;
use crate::message::PipelineMessage;
use crate::stage::{Stage, StageKind};

/// **Hardcode #1**: Dispatch 默认支持的 kind (4 类, 编译期数组).
///
/// 借鉴 Golutra `chat_db` 4 种 message channel (chat / task / memory / mcp),
/// Apeireth 通用化对齐 4 类, 加 1 个 "unknown" 兜底 = 5 类.
pub const DISPATCH_DEFAULT_KINDS: &[&str] = &["chat", "task", "memory", "mcp", "unknown"];

/// **Hardcode #2**: Dispatch kind 数量上限 (16 种, 防 m3 幻觉无限扩展).
pub const DISPATCH_MAX_KINDS: usize = 16;

/// **Hardcode #3**: 空 kind 拒绝消息 (编译期守门).
pub const DISPATCH_EMPTY_KIND_REJECT: bool = true;

/// Default Dispatch stage (kind 路由 + 空 kind 拒绝).
///
/// 行为:
/// - 接收 `PipelineMessage`, 检查 `kind` 非空
/// - 检查 `kind` 在 `DISPATCH_DEFAULT_KINDS` 白名单 (默认开启; 关闭用 `with_whitelist_disabled`)
/// - 给 `payload` 加 `[kind] ` 前缀 (给下游 stage 路由信息)
#[derive(Debug, Clone)]
pub struct DefaultDispatch {
    /// 是否启用白名单检查 (默认 true, 关闭 = 接受任意 kind).
    whitelist_enabled: bool,
}

impl DefaultDispatch {
    /// 创建默认 Dispatch stage (白名单开启).
    pub fn new() -> Self {
        Self {
            whitelist_enabled: true,
        }
    }

    /// 关闭白名单检查 (接受任意 kind, 留给用户自定义路由逻辑).
    pub fn with_whitelist_disabled(mut self) -> Self {
        self.whitelist_enabled = false;
        self
    }

    /// 编译期守门: `DISPATCH_DEFAULT_KINDS.len() <= DISPATCH_MAX_KINDS` (Hardcode #2).
    pub const fn validate_kinds_count() -> bool {
        DISPATCH_DEFAULT_KINDS.len() <= DISPATCH_MAX_KINDS
    }
}

impl Default for DefaultDispatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage<PipelineMessage, PipelineMessage> for DefaultDispatch {
    fn kind(&self) -> StageKind {
        StageKind::Dispatch
    }

    fn name(&self) -> &str {
        "default-dispatch"
    }

    fn process(&self, input: PipelineMessage) -> Result<PipelineMessage, PipelineError> {
        // 守门 1: kind 非空 (Hardcode #3)
        if DISPATCH_EMPTY_KIND_REJECT && input.kind.trim().is_empty() {
            return Err(PipelineError::Stage {
                kind: StageKind::Dispatch,
                source: Box::new(DispatchError::EmptyKind),
            });
        }

        // 守门 2: kind 在白名单
        if self.whitelist_enabled && !DISPATCH_DEFAULT_KINDS.contains(&input.kind.as_str()) {
            return Err(PipelineError::Stage {
                kind: StageKind::Dispatch,
                source: Box::new(DispatchError::KindNotInWhitelist {
                    kind: input.kind.clone(),
                    allowed: DISPATCH_DEFAULT_KINDS
                        .iter()
                        .map(|s| (*s).to_owned())
                        .collect(),
                }),
            });
        }

        // 路由: payload 加 `[kind] ` 前缀 (给下游 stage 路由信息)
        // (先 clone kind, 避免借用检查 double-move)
        let routed_kind = input.kind.clone();
        Ok(PipelineMessage {
            kind: routed_kind.clone(),
            payload: format!("[{}] {}", routed_kind, input.payload),
            attempt: input.attempt,
            trace_id: input.trace_id,
        })
    }
}

/// Dispatch 阶段内部错误 (Box<dyn Error> 透传).
#[derive(Debug)]
pub enum DispatchError {
    /// kind 字段为空.
    EmptyKind,
    /// kind 不在白名单.
    KindNotInWhitelist {
        /// 收到的 kind.
        kind: String,
        /// 允许的 kind 列表.
        allowed: Vec<String>,
    },
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DispatchError::EmptyKind => f.write_str("kind field is empty"),
            DispatchError::KindNotInWhitelist { kind, allowed } => {
                write!(f, "kind '{}' not in whitelist {:?}", kind, allowed)
            }
        }
    }
}

impl std::error::Error for DispatchError {}

/// 编译期守门: `DISPATCH_DEFAULT_KINDS.len() <= DISPATCH_MAX_KINDS`.
const _: () = assert!(DefaultDispatch::validate_kinds_count());
/// 编译期守门: `DISPATCH_DEFAULT_KINDS.len() == 5`.
const _: () = assert!(DISPATCH_DEFAULT_KINDS.len() == 5);
