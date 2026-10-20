//! # Sandbox error types — 闭合词表 (closed vocabulary)
//!
//! 沙箱编排客户端协议层的错误面。两个闭合词表:
//!
//! 1. [`SandboxErrorCode`] — 本层错误分类闭合词表 (17 类)。每个 [`SandboxError`]
//!    变体唯一归属一个 code; 新增分类必须同步扩词表并过评审, 词表不允许
//!    运行时动态扩张 (无 `String` 自由分类)。
//! 2. `WireErrorCode` (见 [`crate::sandbox::protocol`]) — 线上编排服务返回的
//!    错误码闭合词表。未知线上错误码一律归 [`SandboxErrorCode::Protocol`],
//!    不做字符串透传扩散。
//!
//! 错误消息本身允许携带诊断细节, 但**分类归属**永远落在闭合词表内。

use std::path::PathBuf;

use thiserror::Error;

use crate::sandbox::runtime::{IsolationLevel, RuntimeKind};

/// 本层错误分类闭合词表 (17 类, 编译期 hardcode)。
///
/// 稳定字符串 (snake_case) 供日志 / 界面 / 上层归类使用; 字符串是对外契约,
/// 改名即破坏契约。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SandboxErrorCode {
    /// 工具未在白名单内。
    ToolNotWhitelisted,
    /// 镜像名无效。
    InvalidImage,
    /// 命令无效。
    InvalidCommand,
    /// 配置无效 (字段越界 / 白名单外 / 相互矛盾)。
    InvalidConfig,
    /// 生命周期状态迁移非法。
    InvalidState,
    /// 目标沙箱不存在。
    NotFound,
    /// 配额超限 (并发数 / CPU / 内存记账)。
    QuotaExceeded,
    /// deadline 到期, 操作未在期限内交付。
    Timeout,
    /// 编排服务传输失败 (不可达 / 连接中断)。
    Transport,
    /// 协议违规 (编解码失败 / schema 版本不符 / 未知错误码)。
    Protocol,
    /// 底层运行时错误。
    Runtime,
    /// 隔离级别不兼容。
    Isolation,
    /// 资源超限。
    ResourceExhausted,
    /// 权限被拒 (编排服务侧策略)。
    PermissionDenied,
    /// 路径解析失败。
    InvalidPath,
    /// I/O 错误 (本地原子落盘等)。
    Io,
    /// 其他 (无法归入以上任何一类时的收口, 保持词表闭合)。
    Other,
}

impl SandboxErrorCode {
    /// 稳定 code 字符串 (对外契约)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ToolNotWhitelisted => "tool_not_whitelisted",
            Self::InvalidImage => "invalid_image",
            Self::InvalidCommand => "invalid_command",
            Self::InvalidConfig => "invalid_config",
            Self::InvalidState => "invalid_state",
            Self::NotFound => "not_found",
            Self::QuotaExceeded => "quota_exceeded",
            Self::Timeout => "timeout",
            Self::Transport => "transport",
            Self::Protocol => "protocol",
            Self::Runtime => "runtime",
            Self::Isolation => "isolation",
            Self::ResourceExhausted => "resource_exhausted",
            Self::PermissionDenied => "permission_denied",
            Self::InvalidPath => "invalid_path",
            Self::Io => "io",
            Self::Other => "other",
        }
    }

    /// 闭合词表全集 (长度 == [`SANDBOX_ERROR_CODE_COUNT`])。
    pub const ALL: &'static [SandboxErrorCode] = &[
        Self::ToolNotWhitelisted,
        Self::InvalidImage,
        Self::InvalidCommand,
        Self::InvalidConfig,
        Self::InvalidState,
        Self::NotFound,
        Self::QuotaExceeded,
        Self::Timeout,
        Self::Transport,
        Self::Protocol,
        Self::Runtime,
        Self::Isolation,
        Self::ResourceExhausted,
        Self::PermissionDenied,
        Self::InvalidPath,
        Self::Io,
        Self::Other,
    ];
}

impl std::fmt::Display for SandboxErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 编译期守门: 分类闭合词表长度 17。
pub const SANDBOX_ERROR_CODE_COUNT: usize = 17;
const _: () = assert!(SandboxErrorCode::ALL.len() == SANDBOX_ERROR_CODE_COUNT);

/// Sandbox SDK 错误 (17 variant, 每个 variant 归属 [`SandboxErrorCode`] 闭合词表)。
#[derive(Debug, Error)]
pub enum SandboxError {
    /// 工具未在白名单内。
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    /// 镜像名无效 (空 / 非法字符 / 白名单外 registry)。
    #[error("invalid image: {0}")]
    InvalidImage(String),
    /// 命令无效 (空 / 含注入字符)。
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    /// 配置无效 (字段越界 / 白名单外 / 相互矛盾)。
    #[error("invalid sandbox config: {0}")]
    InvalidConfig(String),

    /// 生命周期状态迁移非法 (见 `lifecycle` 模块迁移矩阵)。
    #[error("invalid sandbox state transition: {0}")]
    InvalidState(String),
    /// 目标沙箱不存在。
    #[error("sandbox not found: {sandbox_id}")]
    NotFound {
        /// 未命中的沙箱 ID。
        sandbox_id: String,
    },
    /// 配额超限 (并发数 / CPU / 内存记账)。
    #[error("quota exceeded: {0}")]
    QuotaExceeded(String),
    /// deadline 到期, 操作未在期限内交付。
    #[error("sandbox deadline expired: {0}")]
    Timeout(String),
    /// 编排服务传输失败 (不可达 / 连接中断)。
    #[error("orchestration transport failure: {0}")]
    Transport(String),
    /// 协议违规 (编解码失败 / schema 版本不符 / 未知错误码)。
    #[error("sandbox protocol violation: {0}")]
    Protocol(String),

    /// 底层运行时错误 (由编排服务上报)。
    #[error("sandbox runtime error: {runtime:?} - {message}")]
    Runtime {
        /// 报错的运行时。
        runtime: RuntimeKind,
        /// 诊断细节。
        message: String,
    },
    /// 隔离级别不兼容。
    #[error("isolation level {level:?} not compatible with {runtime:?}")]
    Isolation {
        /// 运行时。
        runtime: RuntimeKind,
        /// 隔离级别。
        level: IsolationLevel,
    },

    /// 资源超限 (CPU / 内存 / IO / 网络 / 临时目录)。
    #[error("resource exhausted: {0}")]
    ResourceExhausted(String),
    /// 权限被拒 (编排服务侧策略)。
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// 路径解析失败。
    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),
    /// I/O 错误 (本地原子落盘 / 状态文件)。
    #[error("sandbox I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// 其他错误 (闭合词表收口)。
    #[error("sandbox other error: {0}")]
    Other(String),
}

impl SandboxError {
    /// 归属分类 (闭合词表)。
    pub const fn code(&self) -> SandboxErrorCode {
        match self {
            Self::ToolNotWhitelisted(_) => SandboxErrorCode::ToolNotWhitelisted,
            Self::InvalidImage(_) => SandboxErrorCode::InvalidImage,
            Self::InvalidCommand(_) => SandboxErrorCode::InvalidCommand,
            Self::InvalidConfig(_) => SandboxErrorCode::InvalidConfig,
            Self::InvalidState(_) => SandboxErrorCode::InvalidState,
            Self::NotFound { .. } => SandboxErrorCode::NotFound,
            Self::QuotaExceeded(_) => SandboxErrorCode::QuotaExceeded,
            Self::Timeout(_) => SandboxErrorCode::Timeout,
            Self::Transport(_) => SandboxErrorCode::Transport,
            Self::Protocol(_) => SandboxErrorCode::Protocol,
            Self::Runtime { .. } => SandboxErrorCode::Runtime,
            Self::Isolation { .. } => SandboxErrorCode::Isolation,
            Self::ResourceExhausted(_) => SandboxErrorCode::ResourceExhausted,
            Self::PermissionDenied(_) => SandboxErrorCode::PermissionDenied,
            Self::InvalidPath(_) => SandboxErrorCode::InvalidPath,
            Self::Io(_) => SandboxErrorCode::Io,
            Self::Other(_) => SandboxErrorCode::Other,
        }
    }
}

/// deadline 超时错误 → 闭合词表 `timeout` 分类。
impl From<apeireth_core::deadline::TimeoutError> for SandboxError {
    fn from(err: apeireth_core::deadline::TimeoutError) -> Self {
        SandboxError::Timeout(err.to_string())
    }
}

/// serde 编解码错误 → 闭合词表 `protocol` 分类。
impl From<serde_json::Error> for SandboxError {
    fn from(err: serde_json::Error) -> Self {
        SandboxError::Protocol(err.to_string())
    }
}

pub type SandboxResult<T> = Result<T, SandboxError>;

/// 编译期守门: 17 variant 守门 (新增 variant 必须同步改本 const)。
pub const SANDBOX_ERROR_VARIANT_COUNT: usize = 17;

#[cfg(test)]
mod tests {
    use super::*;

    /// 错误分类闭合词表: 每个 variant 的 code 稳定且在词表内。
    #[test]
    fn sandbox_error_variants_map_into_closed_vocabulary() {
        let samples: Vec<(SandboxError, SandboxErrorCode)> = vec![
            (
                SandboxError::ToolNotWhitelisted("t".into()),
                SandboxErrorCode::ToolNotWhitelisted,
            ),
            (
                SandboxError::InvalidImage("i".into()),
                SandboxErrorCode::InvalidImage,
            ),
            (
                SandboxError::InvalidCommand("c".into()),
                SandboxErrorCode::InvalidCommand,
            ),
            (
                SandboxError::InvalidConfig("c".into()),
                SandboxErrorCode::InvalidConfig,
            ),
            (
                SandboxError::InvalidState("s".into()),
                SandboxErrorCode::InvalidState,
            ),
            (
                SandboxError::NotFound {
                    sandbox_id: "id".into(),
                },
                SandboxErrorCode::NotFound,
            ),
            (
                SandboxError::QuotaExceeded("q".into()),
                SandboxErrorCode::QuotaExceeded,
            ),
            (SandboxError::Timeout("t".into()), SandboxErrorCode::Timeout),
            (
                SandboxError::Transport("t".into()),
                SandboxErrorCode::Transport,
            ),
            (
                SandboxError::Protocol("p".into()),
                SandboxErrorCode::Protocol,
            ),
            (
                SandboxError::Runtime {
                    runtime: RuntimeKind::Docker,
                    message: "m".into(),
                },
                SandboxErrorCode::Runtime,
            ),
            (
                SandboxError::Isolation {
                    runtime: RuntimeKind::Docker,
                    level: IsolationLevel::Vm,
                },
                SandboxErrorCode::Isolation,
            ),
            (
                SandboxError::ResourceExhausted("r".into()),
                SandboxErrorCode::ResourceExhausted,
            ),
            (
                SandboxError::PermissionDenied("p".into()),
                SandboxErrorCode::PermissionDenied,
            ),
            (
                SandboxError::InvalidPath(PathBuf::from("/x")),
                SandboxErrorCode::InvalidPath,
            ),
            (
                SandboxError::Io(std::io::Error::other("io")),
                SandboxErrorCode::Io,
            ),
            (SandboxError::Other("o".into()), SandboxErrorCode::Other),
        ];
        assert_eq!(samples.len(), SANDBOX_ERROR_VARIANT_COUNT);
        assert_eq!(SANDBOX_ERROR_VARIANT_COUNT, SANDBOX_ERROR_CODE_COUNT);
        for (err, expected) in samples {
            assert_eq!(err.code(), expected, "misclassified: {err}");
            assert!(
                SandboxErrorCode::ALL.contains(&err.code()),
                "code must stay inside the closed vocabulary: {err}"
            );
        }
    }

    /// 词表字符串稳定 (对外契约) + 无重复。
    #[test]
    fn sandbox_error_code_strings_are_stable_and_unique() {
        assert_eq!(SandboxErrorCode::Timeout.as_str(), "timeout");
        assert_eq!(SandboxErrorCode::QuotaExceeded.as_str(), "quota_exceeded");
        assert_eq!(SandboxErrorCode::Protocol.as_str(), "protocol");
        assert_eq!(SandboxErrorCode::ALL.len(), SANDBOX_ERROR_CODE_COUNT);
        for (i, code) in SandboxErrorCode::ALL.iter().enumerate() {
            for other in &SandboxErrorCode::ALL[i + 1..] {
                assert_ne!(code.as_str(), other.as_str(), "duplicate code word");
            }
        }
    }

    /// deadline 超时与 serde 错误都收口进闭合词表。
    #[test]
    fn timeout_and_codec_errors_close_into_the_vocabulary() {
        let t: SandboxError = apeireth_core::deadline::TimeoutError::Zero.into();
        assert_eq!(t.code(), SandboxErrorCode::Timeout);
        let p: SandboxError = serde_json::from_str::<serde_json::Value>("{")
            .unwrap_err()
            .into();
        assert_eq!(p.code(), SandboxErrorCode::Protocol);
    }
}
