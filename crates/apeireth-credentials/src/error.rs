//! # Credentials Error 类型
//!
//! 凭证子系统的统一错误类型, 1:1 翻译 v0.9.21 商业版 `out/main` 错误码体系.
//! 所有 Provider / Token / Rotation / Scope / Audit 操作返 `CredentialsError` 或
//! `CredentialsResult<T>` (type alias for `Result<T, CredentialsError>`).
//!
//! ## 设计原则 (per S-2 实事求是 + O-5 不假装)
//!
//! 1. **错误码编译期 hardcode**: 10+ 变体, 不可运行时增删, 防止 m3 hallucination 注入伪错误
//! 2. **0 暴露内部凭证**: 错误消息不包含 API key / token / secret 等敏感字段
//! 3. **每个变体 1:1 翻译 v0.9.21 商业版**: 对齐 SDK 错误码, 方便 R21+ 真接时迁移
//! 4. **`NotImplemented` 守门**: 所有 stub 操作返此错误, 配 `warn!` 日志防止静默失败
//!
//! ## 错误分类 (10+ 变体)
//!
//! | 类别 | 变体 | 用途 |
//! |------|------|------|
//! | 通用 | `NotImplemented` | stub 操作未实现 (5 Provider / 4 轮换 / Token refresh) |
//! | 通用 | `Internal` | 内部错误 (panic / invariant violation) |
//! | 配置 | `EmptyApiKey` | K-1 强校验: API key 为空 |
//! | 配置 | `EmptyOAuthClientId` | K-1 强校验: OAuth client_id 为空 |
//! | 配置 | `EmptyJwtAudience` | K-1 强校验: JWT audience 为空 |
//! | 配置 | `EmptyIamRole` | K-1 强校验: IAM role_arn 为空 |
//! | 配置 | `InvalidMtlsCertPath` | K-1 强校验: mTLS 证书路径无效 |
//! | Token | `TokenExpired` | Token 过期 |
//! | Token | `TokenRefreshFailed` | Token 刷新失败 |
//! | Token | `TokenRevokeFailed` | Token 撤销失败 |
//! | Scope | `InsufficientScope` | 越权操作 (read 试图 write) |
//! | Scope | `UnknownScope` | 未知 scope 级别 |
//! | Rotation | `RotationFailed` | 轮换失败 |
//! | Audit | `AuditLogFailed` | 审计日志写入失败 |
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 错误码 1:1 翻译 v0.9.21 商业版, 0 业务重设计
//! - **S-2 实事求是**: 10+ 变体覆盖 K-1 强校验 5 项 + 操作失败 5 类, 0 过度设计
//! - **O-2 走在前人肩上**: 借鉴 RFC 6749 (OAuth 2.0) 错误码 (invalid_request / invalid_grant 等)
//! - **O-3 干到底**: 10+ 变体 + Display + Error trait 完整, 测试 25+ fixture 覆盖
//! - **O-4 任何人都能接手**: 跟 keyring / i18n / voice 错误码同模式 (thiserror 派生)
//! - **O-5 不假装**: `NotImplemented` 守门 + 0 暴露内部凭证, 防假装已对接

use std::fmt;

use thiserror::Error;

// ============================================================================
// §1 错误类型 (CredentialsError, 10+ 变体, 编译期 hardcode)
// ============================================================================

/// Credentials 子系统统一错误类型.
///
/// 1:1 翻译 v0.9.21 商业版 `out/main/chunks/credentials-*.js` 错误码.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CredentialsError {
    /// **stub 操作未实现**: 5 Provider / 4 轮换 / Token refresh 等 R20 阶段 6 skeleton
    /// 全部 stub, 返此错误 + `warn!` 日志. R21+ 真接时移除.
    ///
    /// 字段: `api_name` (例: `"anthropic_provider_get_token"`)
    #[error("[not_implemented] {0}: R20 阶段 6 skeleton, R21+ 真接 v0.9.21 商业版 credentials")]
    NotImplemented(&'static str),

    /// **内部错误**: panic / invariant violation / 编译期不可能路径.
    ///
    /// 字段: `reason` (例: `"invariant violated: STUB_MODE must be true"`)
    #[error("[internal] {0}")]
    Internal(String),

    /// **K-1 强校验 #1**: API key 为空.
    ///
    /// 触发: `ApiKey { api_key: "" }` 试图 get_token / refresh.
    /// 防御: P0 安全铁律 — 凭证绝不存明文, 空值校验必保留.
    #[error("[config_empty] api_key must not be empty (K-1 强校验)")]
    EmptyApiKey,

    /// **K-1 强校验 #2**: OAuth 2.0 client_id 为空.
    #[error("[config_empty] oauth client_id must not be empty (K-1 强校验)")]
    EmptyOAuthClientId,

    /// **K-1 强校验 #3**: JWT audience 为空.
    #[error("[config_empty] jwt audience must not be empty (K-1 强校验)")]
    EmptyJwtAudience,

    /// **K-1 强校验 #4**: IAM role_arn 为空.
    #[error("[config_empty] iam role_arn must not be empty (K-1 强校验)")]
    EmptyIamRole,

    /// **K-1 强校验 #5**: mTLS cert_path 路径无效 (不存在 / 非文件 / 权限不足).
    #[error("[config_invalid] mtls cert_path invalid or not found: {0}")]
    InvalidMtlsCertPath(String),

    /// **Token 过期**: 当前 token 超过 expires_at 时间.
    ///
    /// 字段: `expires_at` (ISO 8601 字符串), `now` (当前时间 ISO 8601).
    #[error("[token_expired] token expired at {expires_at} (now: {now})")]
    TokenExpired {
        /// ISO 8601 格式的过期时间.
        expires_at: String,
        /// ISO 8601 格式的当前时间.
        now: String,
    },

    /// **Token 刷新失败**: OAuth 2.0 refresh_token 换取新 access_token 失败.
    ///
    /// 字段: `reason` (例: `"invalid_grant"`, 不暴露具体 server 响应)
    #[error("[token_refresh_failed] {0}")]
    TokenRefreshFailed(String),

    /// **Token 撤销失败**: revoke API 调用失败 (OAuth 2.0 RFC 7009).
    #[error("[token_revoke_failed] {0}")]
    TokenRevokeFailed(String),

    /// **越权操作**: 当前 scope 级别不足 (例: read 试图 write).
    ///
    /// 字段: `current` (例: `Read`), `required` (例: `Write`)
    #[error("[insufficient_scope] current={current}, required={required}")]
    InsufficientScope {
        /// 当前 scope 级别字符串.
        current: String,
        /// 所需 scope 级别字符串.
        required: String,
    },

    /// **未知 scope 级别**: 5 Scope 枚举外的值.
    #[error("[unknown_scope] scope not in [Read, Write, Admin, Owner, Root]: {0}")]
    UnknownScope(String),

    /// **轮换失败**: time / count / hybrid 任一策略触发时轮换失败.
    ///
    /// 字段: `strategy` (例: `"Time(30 days)"`), `reason` (失败原因)
    #[error("[rotation_failed] strategy={strategy}, reason={reason}")]
    RotationFailed {
        /// 轮换策略描述.
        strategy: String,
        /// 失败原因 (不暴露内部凭证).
        reason: String,
    },

    /// **审计日志写入失败**: 4 事件 (get/put/rotate/revoke) 任一记录失败.
    #[error("[audit_log_failed] event={event}, reason={reason}")]
    AuditLogFailed {
        /// 事件名 (get/put/rotate/revoke).
        event: String,
        /// 失败原因.
        reason: String,
    },

    /// **m3 防御**: 工具调用不在白名单内 (防 m3 模型幻觉调用不存在的工具).
    #[error("[tool_not_whitelisted] tool not in TOOL_WHITELIST: {0}")]
    ToolNotWhitelisted(String),

    /// **Provider 未找到**: CredentialsManager.get_token 等操作时, Provider 未注册.
    #[error("[provider_not_found] provider not registered in manager: {0}")]
    ProviderNotFound(String),
}

/// Credentials 子系统统一 Result 类型别名.
pub type CredentialsResult<T> = Result<T, CredentialsError>;

// ============================================================================
// §2 Display 辅助 (用于日志和断言)
// ============================================================================

impl CredentialsError {
    /// 错误码 (短字符串, 用于日志 / 监控 metric label).
    ///
    /// 1:1 翻译 v0.9.21 商业版错误码:
    /// - `not_implemented` → R20 skeleton stub
    /// - `internal` → panic / invariant
    /// - `config_empty` / `config_invalid` → K-1 强校验
    /// - `token_expired` / `token_refresh_failed` / `token_revoke_failed` → Token 操作
    /// - `insufficient_scope` / `unknown_scope` → Scope 检查
    /// - `rotation_failed` → 轮换失败
    /// - `audit_log_failed` → 审计失败
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotImplemented(_) => "not_implemented",
            Self::Internal(_) => "internal",
            Self::EmptyApiKey
            | Self::EmptyOAuthClientId
            | Self::EmptyJwtAudience
            | Self::EmptyIamRole => "config_empty",
            Self::InvalidMtlsCertPath(_) => "config_invalid",
            Self::TokenExpired { .. } => "token_expired",
            Self::TokenRefreshFailed(_) => "token_refresh_failed",
            Self::TokenRevokeFailed(_) => "token_revoke_failed",
            Self::InsufficientScope { .. } => "insufficient_scope",
            Self::UnknownScope(_) => "unknown_scope",
            Self::RotationFailed { .. } => "rotation_failed",
            Self::AuditLogFailed { .. } => "audit_log_failed",
            Self::ToolNotWhitelisted(_) => "tool_not_whitelisted",
            Self::ProviderNotFound(_) => "provider_not_found",
        }
    }

    /// 错误类别 (用于分类 metric / log).
    #[must_use]
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::NotImplemented(_) | Self::Internal(_) => ErrorCategory::General,
            Self::EmptyApiKey
            | Self::EmptyOAuthClientId
            | Self::EmptyJwtAudience
            | Self::EmptyIamRole
            | Self::InvalidMtlsCertPath(_) => ErrorCategory::Config,
            Self::TokenExpired { .. }
            | Self::TokenRefreshFailed(_)
            | Self::TokenRevokeFailed(_) => ErrorCategory::Token,
            Self::InsufficientScope { .. } | Self::UnknownScope(_) => ErrorCategory::Scope,
            Self::RotationFailed { .. } => ErrorCategory::Rotation,
            Self::AuditLogFailed { .. } => ErrorCategory::Audit,
            Self::ToolNotWhitelisted(_) => ErrorCategory::General,
            Self::ProviderNotFound(_) => ErrorCategory::Config,
        }
    }
}

/// 错误类别 (5 类, 用于分类 metric / log).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// 通用错误 (NotImplemented / Internal).
    General,
    /// 配置错误 (K-1 强校验 5 项).
    Config,
    /// Token 操作错误 (expired / refresh / revoke).
    Token,
    /// Scope 错误 (insufficient / unknown).
    Scope,
    /// Rotation 错误.
    Rotation,
    /// Audit 日志错误.
    Audit,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::General => "general",
            Self::Config => "config",
            Self::Token => "token",
            Self::Scope => "scope",
            Self::Rotation => "rotation",
            Self::Audit => "audit",
        };
        f.write_str(s)
    }
}

// ============================================================================
// §3 编译期守门 (K-1 强校验 + 5 鉴权 + 4 轮换错误码对齐)
// ============================================================================

/// 编译期守门: K-1 强校验 5 项 (EmptyApiKey / EmptyOAuthClientId / EmptyJwtAudience / EmptyIamRole / InvalidMtlsCertPath).
pub const K1_STRONG_VALIDATION_COUNT: usize = 5;

/// 编译期守门: 5 鉴权方式 → 5 K-1 错误变体.
pub const K1_VARIANT_NAMES: &[&str] = &[
    "EmptyApiKey",
    "EmptyOAuthClientId",
    "EmptyJwtAudience",
    "EmptyIamRole",
    "InvalidMtlsCertPath",
];

const _: () = assert!(K1_VARIANT_NAMES.len() == K1_STRONG_VALIDATION_COUNT);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_unique() {
        // 验证 code() 不为空 + 监控 label 一致性 (同 category 共享 code, 跨 category 不同)
        let codes = [
            CredentialsError::NotImplemented("test").code(),
            CredentialsError::Internal("test".to_string()).code(),
            CredentialsError::EmptyApiKey.code(),
            CredentialsError::InvalidMtlsCertPath("test".to_string()).code(),
            CredentialsError::TokenExpired {
                expires_at: "x".to_string(),
                now: "x".to_string(),
            }
            .code(),
            CredentialsError::TokenRefreshFailed("x".to_string()).code(),
            CredentialsError::TokenRevokeFailed("x".to_string()).code(),
            CredentialsError::InsufficientScope {
                current: "x".to_string(),
                required: "x".to_string(),
            }
            .code(),
            CredentialsError::UnknownScope("x".to_string()).code(),
            CredentialsError::RotationFailed {
                strategy: "x".to_string(),
                reason: "x".to_string(),
            }
            .code(),
            CredentialsError::AuditLogFailed {
                event: "x".to_string(),
                reason: "x".to_string(),
            }
            .code(),
            CredentialsError::ToolNotWhitelisted("x".to_string()).code(),
            CredentialsError::ProviderNotFound("x".to_string()).code(),
        ];
        // 13 个独立 code (K-1 强校验 5 变体共享 "config_empty" code, 这是设计而非 bug)
        for code in &codes {
            assert!(!code.is_empty(), "code must not be empty");
        }
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), 13, "13 unique cross-category codes expected");
    }

    #[test]
    fn test_error_category_coverage() {
        // 验证 6 个 ErrorCategory 都至少有 1 个变体
        let mut categories = std::collections::HashSet::new();
        categories.insert(CredentialsError::NotImplemented("x").category());
        categories.insert(CredentialsError::EmptyApiKey.category());
        categories.insert(
            CredentialsError::TokenExpired {
                expires_at: "x".to_string(),
                now: "x".to_string(),
            }
            .category(),
        );
        categories.insert(
            CredentialsError::InsufficientScope {
                current: "x".to_string(),
                required: "x".to_string(),
            }
            .category(),
        );
        categories.insert(
            CredentialsError::RotationFailed {
                strategy: "x".to_string(),
                reason: "x".to_string(),
            }
            .category(),
        );
        categories.insert(
            CredentialsError::AuditLogFailed {
                event: "x".to_string(),
                reason: "x".to_string(),
            }
            .category(),
        );
        assert_eq!(categories.len(), 6, "all 6 categories must be covered");
    }

    #[test]
    fn test_not_implemented_carries_api_name() {
        let err = CredentialsError::NotImplemented("anthropic_get_token");
        assert!(err.to_string().contains("anthropic_get_token"));
        assert!(err.to_string().contains("R20 阶段 6"));
    }

    #[test]
    fn test_k1_variant_count() {
        // 5 K-1 强校验变体必须在 K1_VARIANT_NAMES 中
        assert_eq!(K1_VARIANT_NAMES.len(), K1_STRONG_VALIDATION_COUNT);
        assert!(K1_VARIANT_NAMES.contains(&"EmptyApiKey"));
        assert!(K1_VARIANT_NAMES.contains(&"EmptyOAuthClientId"));
        assert!(K1_VARIANT_NAMES.contains(&"EmptyJwtAudience"));
        assert!(K1_VARIANT_NAMES.contains(&"EmptyIamRole"));
        assert!(K1_VARIANT_NAMES.contains(&"InvalidMtlsCertPath"));
    }
}
