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

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_displays_correctly() {
        let e = ExtensionError::ManifestParse("bad".into());
        assert_eq!(e.to_string(), "manifest parse error: bad");
    }

    #[test]
    fn error_permission_denied_fields() {
        let e = ExtensionError::PermissionDenied {
            plugin: "p1".into(),
            required: "write".into(),
            caller: "read".into(),
        };
        let s = e.to_string();
        assert!(s.contains("p1"));
        assert!(s.contains("write"));
        assert!(s.contains("read"));
    }

    #[test]
    fn error_input_too_large_fields() {
        let e = ExtensionError::InputTooLarge {
            actual: 100,
            max: 50,
            plugin: "p".into(),
        };
        assert!(e.to_string().contains("100"));
        assert!(e.to_string().contains("50"));
    }

    #[test]
    fn error_audit_rejected_display() {
        let e = ExtensionError::AuditRejected("no perms".into());
        assert_eq!(e.to_string(), "audit rejected: no perms");
    }
}
