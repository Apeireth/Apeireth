//! 跨语言统一错误码 (snake_case ↔ camelCase 双向).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 跨语言错误码 (snake_case 字符串).
///
/// 编号空间: 1xxx 通用, 2xxx 协议, 3xxx 资源, 4xxx 权限, 5xxx 内部.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdkErrorCode {
    /// 通用未分类
    Unknown,
    /// 协议 envelope 字段缺失
    InvalidEnvelope,
    /// SDK 版本不兼容 (major 不同)
    VersionIncompatible,
    /// 没找到资源 (id / 句柄)
    NotFound,
    /// 权限被拒
    PermissionDenied,
    /// 工具调用未授权
    ToolNotApproved,
    /// 内部错误 (panic / 不可恢复)
    Internal,
    /// 自由扩展
    Other(String),
}

impl SdkErrorCode {
    /// 数字 code (给日志 / 监控统一)
    pub fn numeric_code(&self) -> i32 {
        match self {
            Self::Unknown => 1000,
            Self::InvalidEnvelope => 2001,
            Self::VersionIncompatible => 2002,
            Self::NotFound => 3001,
            Self::PermissionDenied => 4001,
            Self::ToolNotApproved => 4002,
            Self::Internal => 5001,
            Self::Other(_) => 5999,
        }
    }

    /// snake_case 字符串 (默认 serde 行为, 但显式给出便于直接调用).
    pub fn snake_name(&self) -> String {
        // 用 serde_json::to_value + back 出来重新拿到 snake_case 字符串
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    }

    /// camelCase 字符串 (给 JS / Kotlin / Swift 调用方).
    pub fn camel_name(&self) -> String {
        let snake = self.snake_name();
        let mut out = String::with_capacity(snake.len());
        let mut upper = false;
        for c in snake.chars() {
            if c == '_' {
                upper = true;
            } else if upper {
                out.extend(c.to_uppercase());
                upper = false;
            } else {
                out.push(c);
            }
        }
        out
    }
}

/// SDK 顶层错误.
#[derive(Debug, Error)]
pub enum SdkError {
    /// 编码 / 解码错误
    #[error("wire error: {0}")]
    Wire(#[from] serde_json::Error),
    /// 业务错误
    #[error("sdk error: {code:?} — {message}")]
    Business {
        /// 错误码
        code: SdkErrorCode,
        /// 人类可读 message
        message: String,
    },
}

impl SdkError {
    /// 构造业务错误
    pub fn business(code: SdkErrorCode, message: impl Into<String>) -> Self {
        Self::Business {
            code,
            message: message.into(),
        }
    }
}
