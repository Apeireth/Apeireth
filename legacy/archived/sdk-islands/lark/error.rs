//! # lark 错误类型 (闭合分类词表 + K-1 强校验)
//!
//! 本模块定义 `lark` 子模块的完整错误面:
//!
//! 1. **错误分类闭合词表** [`ErrorClass`] (3 类, 不可扩):
//!    `Retryable` (可重试) / `Permanent` (永久) / `AuthFailed` (认证失败)。
//!    任何错误都必须落到且只落到一类, 分类由 [`LarkError::class`] 单点给出。
//! 2. **错误事实** [`LarkError`] (14 variant): 保留错误的具体事实
//!    (字段校验失败 / 传输失败 / 限流 / 业务码 / 显式不支持), 便于调用方精确处理。
//! 3. **6 K-1 字段强校验** (per 任务规范 §3):
//!    `validate_app_id` / `validate_app_secret` / `validate_chat_id` /
//!    `validate_open_id` / `validate_email` / `validate_mobile`。
//! 4. **平台业务码 → 分类** 闭合映射表 ([`LarkError::from_platform_code`]),
//!    限流码 / token 失效码为编译期常量, 未知业务码一律归 `ApiError` → `Permanent`。
//!
//! ## 分类规则 (闭合词表)
//!
//! | 事实 | 分类 |
//! |---|---|
//! | `Network` / `RateLimited` | `Retryable` |
//! | `TokenExpired` (含平台 token 失效/过期业务码) | `AuthFailed` |
//! | `ApiError` (业务码命中认证闭合表) | `AuthFailed` |
//! | `ApiError` (其它业务码) | `Permanent` |
//! | 6 K-1 字段校验失败 | `Permanent` |
//! | `Unsupported` (协议面需外部配置 / 客户端不覆盖) | `Permanent` |
//! | `Other` (序列化 / 协议体畸形等) | `Permanent` |
//!
//! ## 安全 (脱敏)
//!
//! 所有错误消息 **不得** 含 App Secret / token / encrypt key 明文;
//! webhook 校验类错误只报事实类别, 不回显任何共享秘密。

use thiserror::Error;

// ============================================================================
// §1 ErrorClass — 错误分类闭合词表 (3 类)
// ============================================================================

/// 错误分类闭合词表 (3 类, 编译期 hardcode, 不可扩).
///
/// 调用方的重试策略只允许依赖本词表, 不允许解析错误字符串。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorClass {
    /// **可重试**: 传输抖动 / 超时 / 限流。退避后允许重试。
    Retryable,
    /// **永久**: 字段校验失败 / 请求畸形 / 业务规则拒绝 / 显式不支持。重试无意义。
    Permanent,
    /// **认证失败**: token 失效 / 过期 / 无效。需重新取 token 或修正凭证后才能成功。
    AuthFailed,
}

impl ErrorClass {
    /// 闭合词表大小 (3)。
    pub const COUNT: usize = 3;

    /// 闭合词表字符串值 (`"retryable"` / `"permanent"` / `"auth_failed"`)。
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorClass::Retryable => "retryable",
            ErrorClass::Permanent => "permanent",
            ErrorClass::AuthFailed => "auth_failed",
        }
    }

    /// 闭合词表全集 (供遍历/守门测试)。
    pub const ALL: [ErrorClass; ErrorClass::COUNT] = [
        ErrorClass::Retryable,
        ErrorClass::Permanent,
        ErrorClass::AuthFailed,
    ];
}

impl std::fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 平台业务码闭合表 (限流 / 认证失败)
// ============================================================================

/// 平台限流业务码 (命中即 [`ErrorClass::Retryable`], 退避后可重试)。
pub const PLATFORM_CODE_RATE_LIMITED: i32 = 99991400;

/// 平台 access token 无效业务码 (命中即 [`ErrorClass::AuthFailed`])。
pub const PLATFORM_CODE_TOKEN_INVALID: i32 = 99991663;

/// 平台 access token 过期业务码 (命中即 [`ErrorClass::AuthFailed`])。
pub const PLATFORM_CODE_TOKEN_EXPIRED: i32 = 99991668;

/// 认证失败业务码闭合表。
pub const PLATFORM_AUTH_FAILURE_CODES: &[i32] =
    &[PLATFORM_CODE_TOKEN_INVALID, PLATFORM_CODE_TOKEN_EXPIRED];

// ============================================================================
// §3 LarkError (14 variant, 事实保留 + 闭合分类)
// ============================================================================

/// lark 客户端错误 (14 variant).
///
/// 变体只承载**事实**; 重试决策请用 [`LarkError::class`] 的闭合词表。
#[derive(Debug, Error)]
pub enum LarkError {
    // === §1 显式不支持 (1 variant) ===
    /// **显式不支持**: 该协议面需要外部配置 (如部署方的系统凭据库),
    /// 或入站数据形状不在本客户端覆盖范围内。**不是**占位桩 —— 调用点
    /// 明确返回本变体并逐处附带原因标识, 调用方可据此降级或报配置缺失。
    #[error("unsupported lark protocol surface: {0} (requires external configuration or out of client coverage)")]
    Unsupported(&'static str),

    // === §2 K-1 强校验 (8 variant, 6 类, Missing+Invalid 分列) ===
    /// **K-1 #1**: App ID 缺失 (空 / 全空白)。
    #[error("lark app id is missing (K-1 #1, expected non-empty)")]
    AppIdMissing,
    /// **K-1 #1**: App ID 格式错误 (非 `cli_` 前缀 / 长度不足)。
    #[error("lark app id is invalid: {0} (K-1 #1, expected prefix 'cli_' + >=8 alphanumerics)")]
    AppIdInvalid(String),
    /// **K-1 #2**: App Secret 缺失 (空 / 全空白)。
    #[error("lark app secret is missing (K-1 #2, expected non-empty)")]
    AppSecretMissing,
    /// **K-1 #2**: App Secret 格式错误 (长度 < 16)。
    #[error("lark app secret is invalid: length={0} < 16 (K-1 #2)")]
    AppSecretInvalid(usize),
    /// **K-1 #3**: Chat ID 格式错误 (非 `oc_` / `on_` 前缀)。
    #[error("lark chat id is invalid: {0} (K-1 #3, expected prefix 'oc_' or 'on_')")]
    ChatIdInvalid(String),
    /// **K-1 #4**: Open ID 格式错误 (非 `ou_` 前缀)。
    #[error("lark open id is invalid: {0} (K-1 #4, expected prefix 'ou_')")]
    OpenIdInvalid(String),
    /// **K-1 #5**: Email 格式错误 (非 RFC 5322 简化语法)。
    #[error("lark email is invalid: {0} (K-1 #5, expected RFC 5322 syntax)")]
    EmailInvalid(String),
    /// **K-1 #6**: Mobile 格式错误 (非 E.164 `+` + 7-15 位数字)。
    #[error("lark mobile is invalid: {0} (K-1 #6, expected E.164 like +8613800138000)")]
    MobileInvalid(String),

    // === §3 鉴权 (1 variant) ===
    /// access token 失效 / 过期 (tenant / user token)。分类 `AuthFailed`。
    #[error("lark access token expired or invalid (re-acquire token required)")]
    TokenExpired,

    // === §4 传输 (1 variant) ===
    /// 传输错误 (连接 / DNS / TLS / 超时)。分类 `Retryable`。
    #[error("lark network error: {0}")]
    Network(String),

    // === §5 限流 (1 variant) ===
    /// 限流 (HTTP 429 或平台限流业务码)。`retry_after_secs == 0` 表示服务端
    /// 未给出 Retry-After, 调用方按自身退避策略处理。分类 `Retryable`。
    #[error("lark rate limited (retry_after_secs={retry_after_secs}, 0 = not provided)")]
    RateLimited {
        /// 服务端建议的重试等待秒数 (0 = 未提供)。
        retry_after_secs: u64,
    },

    // === §6 业务 (1 variant) ===
    /// 平台业务错误 (`code != 0`)。业务码命中认证闭合表时会先被
    /// [`LarkError::from_platform_code`] 转成 [`LarkError::TokenExpired`],
    /// 因此本变体默认分类 `Permanent`, 保留原始 `code` / `msg` 供排查。
    #[error("lark api error: code={code}, msg={msg}")]
    ApiError {
        /// 平台业务错误码。
        code: i32,
        /// 平台错误信息 (不含共享秘密)。
        msg: String,
    },

    // === §7 其他 (1 variant) ===
    /// 其他错误 (响应体畸形 / 序列化失败 / 内部错误)。分类 `Permanent`。
    #[error("lark other error: {0}")]
    Other(String),
}

/// lark 结果别名。
pub type LarkResult<T> = Result<T, LarkError>;

/// 编译期守门: `LarkError` variant 数 (14)。新增 variant 必须同步改本常量。
pub const LARK_ERROR_VARIANT_COUNT: usize = 14;

impl LarkError {
    /// 错误分类 (闭合词表, 单点)。
    pub fn class(&self) -> ErrorClass {
        match self {
            LarkError::Network(_) | LarkError::RateLimited { .. } => ErrorClass::Retryable,
            LarkError::TokenExpired => ErrorClass::AuthFailed,
            LarkError::ApiError { code, .. } => {
                if PLATFORM_AUTH_FAILURE_CODES.contains(code) {
                    ErrorClass::AuthFailed
                } else if *code == PLATFORM_CODE_RATE_LIMITED {
                    ErrorClass::Retryable
                } else {
                    ErrorClass::Permanent
                }
            }
            LarkError::Unsupported(_)
            | LarkError::AppIdMissing
            | LarkError::AppIdInvalid(_)
            | LarkError::AppSecretMissing
            | LarkError::AppSecretInvalid(_)
            | LarkError::ChatIdInvalid(_)
            | LarkError::OpenIdInvalid(_)
            | LarkError::EmailInvalid(_)
            | LarkError::MobileInvalid(_)
            | LarkError::Other(_) => ErrorClass::Permanent,
        }
    }

    /// 是否可重试 (分类 == `Retryable`)。
    pub fn is_retryable(&self) -> bool {
        self.class() == ErrorClass::Retryable
    }

    /// 是否认证失败 (分类 == `AuthFailed`)。
    pub fn is_auth_failure(&self) -> bool {
        self.class() == ErrorClass::AuthFailed
    }

    /// 平台业务码 → 错误事实 (闭合映射):
    /// - 限流码 → [`LarkError::RateLimited`] (`retry_after_secs = 0`, 服务端未给时)
    /// - 认证失败码 → [`LarkError::TokenExpired`]
    /// - 其它 → [`LarkError::ApiError`] 保留原始 `code` / `msg`
    pub fn from_platform_code(code: i32, msg: &str) -> LarkError {
        if code == PLATFORM_CODE_RATE_LIMITED {
            LarkError::RateLimited {
                retry_after_secs: parse_retry_after_from_msg(msg).unwrap_or(0),
            }
        } else if PLATFORM_AUTH_FAILURE_CODES.contains(&code) {
            LarkError::TokenExpired
        } else {
            LarkError::ApiError {
                code,
                msg: msg.to_string(),
            }
        }
    }
}

/// 从平台限流错误消息中解析建议等待秒数 (消息形如 `"... retry after 30 s"` /
/// `"retry_after=30"` 时取整数; 解析不到返回 `None`, 调用方走自身策略默认值)。
fn parse_retry_after_from_msg(msg: &str) -> Option<u64> {
    let bytes = msg.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx].is_ascii_digit() {
            let start = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_digit() {
                idx += 1;
            }
            let value: u64 = msg[start..idx].parse().ok()?;
            // 上限 1 天, 防畸形消息里的巨大数字把调用方挂死
            return Some(value.min(86_400));
        }
        idx += 1;
    }
    None
}

// ============================================================================
// §4 K-1 强校验方法 (6 个)
// ============================================================================

impl LarkError {
    /// **K-1 #1**: 校验 App ID (非空 + `cli_` 前缀 + 至少 8 位字母数字)。
    pub fn validate_app_id(app_id: &str) -> LarkResult<()> {
        let trimmed = app_id.trim();
        if trimmed.is_empty() {
            return Err(LarkError::AppIdMissing);
        }
        if !trimmed.starts_with("cli_") {
            return Err(LarkError::AppIdInvalid(trimmed.to_string()));
        }
        // 长度校验: `cli_` (4) + 至少 8 个 alphanumeric
        if trimmed.len() < 12 || !trimmed[4..].chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(LarkError::AppIdInvalid(trimmed.to_string()));
        }
        Ok(())
    }

    /// **K-1 #2**: 校验 App Secret (非空 + 长度 ≥ 16)。
    pub fn validate_app_secret(app_secret: &str) -> LarkResult<()> {
        let trimmed = app_secret.trim();
        if trimmed.is_empty() {
            return Err(LarkError::AppSecretMissing);
        }
        if trimmed.len() < 16 {
            return Err(LarkError::AppSecretInvalid(trimmed.len()));
        }
        Ok(())
    }

    /// **K-1 #3**: 校验 Chat ID (非空 + `oc_` 开放群 / `on_` 用户会话前缀)。
    pub fn validate_chat_id(chat_id: &str) -> LarkResult<()> {
        let trimmed = chat_id.trim();
        if trimmed.is_empty() {
            return Err(LarkError::ChatIdInvalid(trimmed.to_string()));
        }
        if !trimmed.starts_with("oc_") && !trimmed.starts_with("on_") {
            return Err(LarkError::ChatIdInvalid(trimmed.to_string()));
        }
        Ok(())
    }

    /// **K-1 #4**: 校验 Open ID (非空 + `ou_` 前缀)。
    pub fn validate_open_id(open_id: &str) -> LarkResult<()> {
        let trimmed = open_id.trim();
        if trimmed.is_empty() {
            return Err(LarkError::OpenIdInvalid(trimmed.to_string()));
        }
        if !trimmed.starts_with("ou_") {
            return Err(LarkError::OpenIdInvalid(trimmed.to_string()));
        }
        Ok(())
    }

    /// **K-1 #5**: 校验 Email (RFC 5322 简化语法: `local@domain.tld`)。
    ///
    /// 完整 RFC 5322 需要完整语法器, 此处用保守启发式:
    /// `local` 1..=64 字符 (字母数字 + `. _ % + -`), 恰好 1 个 `@`,
    /// `domain` 至少 2 段且末段 ≥ 2 字符。
    pub fn validate_email(email: &str) -> LarkResult<()> {
        let trimmed = email.trim();
        if trimmed.is_empty() {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        // 必须含且仅含 1 个 @
        if trimmed.matches('@').count() != 1 {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        let parts: Vec<&str> = trimmed.split('@').collect();
        if parts.len() != 2 {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        let local = parts[0];
        let domain = parts[1];
        if local.is_empty() || local.len() > 64 {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        // domain 必须含 `.`, 且 `.tld` ≥ 2 chars
        let domain_parts: Vec<&str> = domain.split('.').collect();
        if domain_parts.len() < 2 {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        if domain_parts[domain_parts.len() - 1].len() < 2 {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        // local 段字符校验: 字母数字 + . _ % + -
        for c in local.chars() {
            if !c.is_ascii_alphanumeric() && !matches!(c, '.' | '_' | '%' | '+' | '-') {
                return Err(LarkError::EmailInvalid(trimmed.to_string()));
            }
        }
        Ok(())
    }

    /// **K-1 #6**: 校验 Mobile (E.164: `+` + 7-15 位数字)。
    pub fn validate_mobile(mobile: &str) -> LarkResult<()> {
        let trimmed = mobile.trim();
        if trimmed.is_empty() {
            return Err(LarkError::MobileInvalid(trimmed.to_string()));
        }
        if !trimmed.starts_with('+') {
            return Err(LarkError::MobileInvalid(trimmed.to_string()));
        }
        // `+` 后必须 7-15 位数字 (E.164)
        let digits = &trimmed[1..];
        if digits.len() < 7 || digits.len() > 15 {
            return Err(LarkError::MobileInvalid(trimmed.to_string()));
        }
        if !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(LarkError::MobileInvalid(trimmed.to_string()));
        }
        Ok(())
    }
}

// ============================================================================
// §5 单元测试 (K-1 6 强校验 + 闭合分类词表)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- K-1 #1 App ID ----

    #[test]
    fn k1_app_id_valid() {
        assert!(LarkError::validate_app_id("cli_a1b2c3d4").is_ok());
        assert!(LarkError::validate_app_id("cli_a1b2c3d4e5f6g7h8").is_ok());
    }

    #[test]
    fn k1_app_id_missing() {
        assert!(matches!(
            LarkError::validate_app_id(""),
            Err(LarkError::AppIdMissing)
        ));
        assert!(matches!(
            LarkError::validate_app_id("   "),
            Err(LarkError::AppIdMissing)
        ));
    }

    #[test]
    fn k1_app_id_invalid_prefix() {
        assert!(matches!(
            LarkError::validate_app_id("app_a1b2c3d4"),
            Err(LarkError::AppIdInvalid(_))
        ));
        assert!(matches!(
            LarkError::validate_app_id("test"),
            Err(LarkError::AppIdInvalid(_))
        ));
    }

    #[test]
    fn k1_app_id_invalid_charset() {
        // 前缀对但主体含非法字符 / 长度不足
        assert!(matches!(
            LarkError::validate_app_id("cli_abcd!efgh"),
            Err(LarkError::AppIdInvalid(_))
        ));
        assert!(matches!(
            LarkError::validate_app_id("cli_abc"),
            Err(LarkError::AppIdInvalid(_))
        ));
    }

    // ---- K-1 #2 App Secret ----

    #[test]
    fn k1_app_secret_valid() {
        assert!(LarkError::validate_app_secret("1234567890abcdef").is_ok());
        assert!(LarkError::validate_app_secret("abcdef1234567890abcdef1234567890").is_ok());
    }

    #[test]
    fn k1_app_secret_missing() {
        assert!(matches!(
            LarkError::validate_app_secret(""),
            Err(LarkError::AppSecretMissing)
        ));
    }

    #[test]
    fn k1_app_secret_too_short() {
        assert!(matches!(
            LarkError::validate_app_secret("short"),
            Err(LarkError::AppSecretInvalid(5))
        ));
    }

    // ---- K-1 #3 Chat ID ----

    #[test]
    fn k1_chat_id_valid() {
        assert!(LarkError::validate_chat_id("oc_a1b2c3d4e5f6").is_ok());
        assert!(LarkError::validate_chat_id("on_a1b2c3d4e5f6").is_ok());
    }

    #[test]
    fn k1_chat_id_invalid_prefix() {
        assert!(matches!(
            LarkError::validate_chat_id("oc123"),
            Err(LarkError::ChatIdInvalid(_))
        ));
        assert!(matches!(
            LarkError::validate_chat_id("xx_a1b2c3d4"),
            Err(LarkError::ChatIdInvalid(_))
        ));
        assert!(matches!(
            LarkError::validate_chat_id(""),
            Err(LarkError::ChatIdInvalid(_))
        ));
    }

    // ---- K-1 #4 Open ID ----

    #[test]
    fn k1_open_id_valid() {
        assert!(LarkError::validate_open_id("ou_a1b2c3d4e5f6g7h8").is_ok());
    }

    #[test]
    fn k1_open_id_invalid() {
        assert!(matches!(
            LarkError::validate_open_id(""),
            Err(LarkError::OpenIdInvalid(_))
        ));
        assert!(matches!(
            LarkError::validate_open_id("cli_a1b2c3d4"),
            Err(LarkError::OpenIdInvalid(_))
        ));
    }

    // ---- K-1 #5 Email ----

    #[test]
    fn k1_email_valid() {
        assert!(LarkError::validate_email("user@example.com").is_ok());
        assert!(LarkError::validate_email("a.b+c@sub.example.co").is_ok());
    }

    #[test]
    fn k1_email_invalid() {
        assert!(matches!(
            LarkError::validate_email(""),
            Err(LarkError::EmailInvalid(_))
        ));
        assert!(matches!(
            LarkError::validate_email("not-an-email"),
            Err(LarkError::EmailInvalid(_))
        ));
        assert!(matches!(
            LarkError::validate_email("missing@domain"),
            Err(LarkError::EmailInvalid(_))
        ));
        assert!(matches!(
            LarkError::validate_email("@example.com"),
            Err(LarkError::EmailInvalid(_))
        ));
    }

    // ---- K-1 #6 Mobile ----

    #[test]
    fn k1_mobile_valid() {
        assert!(LarkError::validate_mobile("+8613800138000").is_ok());
        assert!(LarkError::validate_mobile("+14155552671").is_ok());
    }

    #[test]
    fn k1_mobile_invalid() {
        assert!(matches!(
            LarkError::validate_mobile(""),
            Err(LarkError::MobileInvalid(_))
        ));
        assert!(matches!(
            LarkError::validate_mobile("13800138000"),
            Err(LarkError::MobileInvalid(_))
        )); // 缺 +
        assert!(matches!(
            LarkError::validate_mobile("+12345"),
            Err(LarkError::MobileInvalid(_))
        )); // < 7 位
    }

    // ---- 闭合分类词表 ----

    #[test]
    fn error_class_closed_vocabulary() {
        assert_eq!(ErrorClass::COUNT, 3);
        assert_eq!(ErrorClass::ALL.len(), 3);
        assert_eq!(ErrorClass::Retryable.as_str(), "retryable");
        assert_eq!(ErrorClass::Permanent.as_str(), "permanent");
        assert_eq!(ErrorClass::AuthFailed.as_str(), "auth_failed");
    }

    #[test]
    fn error_class_retryable() {
        assert_eq!(
            LarkError::Network("boom".into()).class(),
            ErrorClass::Retryable
        );
        assert_eq!(
            LarkError::RateLimited {
                retry_after_secs: 5
            }
            .class(),
            ErrorClass::Retryable
        );
        assert!(LarkError::Network("boom".into()).is_retryable());
    }

    #[test]
    fn error_class_auth_failed() {
        assert_eq!(LarkError::TokenExpired.class(), ErrorClass::AuthFailed);
        assert!(LarkError::TokenExpired.is_auth_failure());
        let api = LarkError::ApiError {
            code: PLATFORM_CODE_TOKEN_INVALID,
            msg: "token invalid".into(),
        };
        assert_eq!(api.class(), ErrorClass::AuthFailed);
    }

    #[test]
    fn error_class_permanent() {
        assert_eq!(LarkError::Other("x".into()).class(), ErrorClass::Permanent);
        assert_eq!(
            LarkError::Unsupported("credential_store").class(),
            ErrorClass::Permanent
        );
        assert_eq!(LarkError::AppIdMissing.class(), ErrorClass::Permanent);
        let api = LarkError::ApiError {
            code: 230001,
            msg: "bad request".into(),
        };
        assert_eq!(api.class(), ErrorClass::Permanent);
    }

    #[test]
    fn platform_code_mapping_closed_table() {
        // 限流码 → RateLimited
        assert!(matches!(
            LarkError::from_platform_code(PLATFORM_CODE_RATE_LIMITED, "rate limit"),
            LarkError::RateLimited { .. }
        ));
        // 认证码 → TokenExpired
        assert!(matches!(
            LarkError::from_platform_code(PLATFORM_CODE_TOKEN_INVALID, "invalid"),
            LarkError::TokenExpired
        ));
        assert!(matches!(
            LarkError::from_platform_code(PLATFORM_CODE_TOKEN_EXPIRED, "expired"),
            LarkError::TokenExpired
        ));
        // 其它 → ApiError 保留 code/msg
        match LarkError::from_platform_code(230002, "not found") {
            LarkError::ApiError { code, msg } => {
                assert_eq!(code, 230002);
                assert_eq!(msg, "not found");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[test]
    fn retry_after_parse_from_msg_is_bounded() {
        assert_eq!(parse_retry_after_from_msg("retry after 30 s"), Some(30));
        assert_eq!(parse_retry_after_from_msg("no digits here"), None);
        assert_eq!(
            parse_retry_after_from_msg("retry after 999999999 s"),
            Some(86_400),
            "畸形巨大数字必须被上限钳制"
        );
    }

    #[test]
    fn unsupported_variant_replaces_stub_surface() {
        // 显式不支持是永久错误, 且带稳定原因标识 (供调用方降级/报配置缺失)
        let err = LarkError::Unsupported("credential_store");
        assert_eq!(err.class(), ErrorClass::Permanent);
        assert!(format!("{err}").contains("credential_store"));
    }
}
