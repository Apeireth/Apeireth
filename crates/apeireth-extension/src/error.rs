//! error — ExtensionError + Result
//!
//! 7 类错误:
//! - Manifest 解析/校验
//! - Plugin 重复注册 / 未找到
//! - 沙盒拒绝 (权限 / 输入大小)
//! - 执行失败
//! - 审计不通过
//! - 内部 / Other

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtensionError {
    /// extension.toml 解析失败 (TOML 语法)
    #[error("manifest parse error: {0}")]
    ManifestParse(String),

    /// manifest schema 校验失败 (必填字段缺失 / 类型错 / 范围越界)
    #[error("manifest schema error: {0}")]
    ManifestSchema(String),

    /// 重复注册 (name 已存在)
    #[error("plugin already registered: {0}")]
    AlreadyRegistered(String),

    /// 插件未找到
    #[error("plugin not found: {0}")]
    NotFound(String),

    /// 权限不足 (sandbox 拒绝)
    #[error("permission denied: plugin '{plugin}' needs '{required}', caller has '{caller}'")]
    PermissionDenied {
        /// 插件名
        plugin: String,
        /// 需要的权限
        required: String,
        /// 调用方持有的权限
        caller: String,
    },

    /// 输入大小超限
    #[error("input too large: {actual} > {max} bytes (plugin={plugin})")]
    InputTooLarge {
        /// 实际字节数
        actual: usize,
        /// 上限
        max: usize,
        /// 插件名
        plugin: String,
    },

    /// 审核不通过
    #[error("audit rejected: {0}")]
    AuditRejected(String),

    /// 插件执行失败
    #[error("execution failed: {0}")]
    Execution(String),

    /// 内部 / 其他
    #[error("extension: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ExtensionError>;
