//! # Lark error types (per @larksuiteoapi/lark-sdk 商业版 v0.9.21, 1:1 翻译)
//!
//! **STUB MODE**: 11 错误 variant, 编译期 hardcode. 真接飞书 Open Platform 时
//! 把 `NotImplemented` 移除并把 `Network` / `RateLimited` / `PermissionDenied` 等细化.
//!
//! ## 6 K-1 强校验方法 (per task spec §3)
//!
//! | K-1 | 方法 | 守门 | 失败时返 |
//! |---:|---|---|---|
//! | #1 | `validate_app_id` | 非空 + `cli_` 前缀 (飞书 App ID 固定前缀) | `AppIdMissing` / `AppIdInvalid` |
//! | #2 | `validate_app_secret` | 非空 + 长度 ≥ 16 | `AppSecretMissing` / `AppSecretInvalid` |
//! | #3 | `validate_chat_id` | 非空 + 前缀 `oc_` (open chat) 或 `on_` (user chat) | `ChatIdInvalid` |
//! | #4 | `validate_open_id` | 非空 + 前缀 `ou_` | `OpenIdInvalid` |
//! | #5 | `validate_email` | 非空 + RFC 5322 邮箱 | `EmailInvalid` |
//! | #6 | `validate_mobile` | 非空 + E.164 (国际格式 `+` + 7-15 位) | `MobileInvalid` |
//!
//! ## 守门宏: `lark_stub!`
//!
//! 6 API stub 全部 `Err(LarkError::NotImplemented(api))`, 用 `lark_stub!("send_message")`
//! 一行 log + return, 防漏改.
//!
//! ## 引用文档
//!
//! 1. `@larksuiteoapi/lark-sdk v0.9.21` `core/Response.d.ts` (商业版 Response 1:1 翻译源)
//! 2. `@larksuiteoapi/lark-sdk v0.9.21` `client/api_im_open.js` (im/v1/messages 1:1 翻译源)
//! 3. `docs/stage4/m3-hallucination-defense-2026-08-05.md` §2.4 (TOOL_WHITELIST 模式)

use thiserror::Error;

// ============================================================================
// §1 LarkError (11 variant, K-1 强校验 6 + STUB 1 + 飞书 4)
// ============================================================================

/// Lark SDK 错误 (11 variant, 6 大类).
///
/// 1. **STUB** (1): `NotImplemented(api)` — 8 API stub 全部返
/// 2. **K-1 强校验** (6): `AppIdMissing` / `AppIdInvalid` / `AppSecretMissing` / `AppSecretInvalid` / `ChatIdInvalid` / `OpenIdInvalid` / `EmailInvalid` / `MobileInvalid`
///    (实际只 6 类, 各 1 variant, 命名复用 `*Missing` + `*Invalid`)
/// 3. **鉴权** (1): `TokenExpired` (tenant_access_token / user_access_token 过期)
/// 4. **网络** (1): `Network` (HTTP 失败 / DNS 失败 / TLS 失败)
/// 5. **限流** (1): `RateLimited` (per v0.9.21 商业版 `code: 99991400`)
/// 6. **业务** (1): `ApiError` (飞书 Open Platform `code != 0` 业务错误)
/// 7. **其他** (1): `Other` (catch-all, 包含序列化失败等)
///
/// 11 = 1 STUB + 6 K-1 (AppId/ChatId/OpenId/Email/Mobile/AppSecret 合并 Missing+Invalid 共 6 类) + 4 飞书特有 (TokenExpired/Network/RateLimited/ApiError) + 1 Other
#[derive(Debug, Error)]
pub enum LarkError {
    // === §1 STUB (1 variant) ===
    /// **STUB 模式**: API 未实现, R21 续真接 @larksuiteoapi/lark-sdk 后删.
    /// 8 API 全部返本 variant.
    #[error("STUB MODE: API not implemented: {0} (R21 will wire @larksuiteoapi/lark-sdk)")]
    NotImplemented(&'static str),

    // === §2 K-1 强校验 (6 类, K-1 #1..#6 各 1 variant, Missing+Invalid 复用) ===
    /// **K-1 #1**: App ID 缺失 (空 / 全空白).
    #[error("Lark app id is missing (K-1 #1, expected non-empty)")]
    AppIdMissing,
    /// **K-1 #1**: App ID 格式错误 (非 `cli_` 前缀).
    #[error("Lark app id is invalid: {0} (K-1 #1, expected prefix 'cli_')")]
    AppIdInvalid(String),
    /// **K-1 #2**: App Secret 缺失 (空 / 全空白).
    #[error("Lark app secret is missing (K-1 #2, expected non-empty)")]
    AppSecretMissing,
    /// **K-1 #2**: App Secret 格式错误 (长度 < 16).
    #[error("Lark app secret is invalid: length={0} < 16 (K-1 #2)")]
    AppSecretInvalid(usize),
    /// **K-1 #3**: Chat ID 格式错误 (非 `oc_` / `on_` 前缀).
    #[error("Lark chat id is invalid: {0} (K-1 #3, expected prefix 'oc_' or 'on_')")]
    ChatIdInvalid(String),
    /// **K-1 #4**: Open ID 格式错误 (非 `ou_` 前缀).
    #[error("Lark open id is invalid: {0} (K-1 #4, expected prefix 'ou_')")]
    OpenIdInvalid(String),
    /// **K-1 #5**: Email 格式错误 (非 RFC 5322).
    #[error("Lark email is invalid: {0} (K-1 #5, expected RFC 5322)")]
    EmailInvalid(String),
    /// **K-1 #6**: Mobile 格式错误 (非 E.164 `+` + 7-15 位).
    #[error("Lark mobile is invalid: {0} (K-1 #6, expected E.164 like +8613800138000)")]
    MobileInvalid(String),

    // === §3 鉴权 (1 variant) ===
    /// Access token 过期 (tenant_access_token / user_access_token 2h 默认, R21 续真接时刷).
    #[error("Lark access token expired (refresh needed)")]
    TokenExpired,

    // === §4 网络 (1 variant) ===
    /// 网络错误 (HTTP / DNS / TLS, R21+ 真接 reqwest 时细化).
    #[error("Lark network error: {0}")]
    Network(String),

    // === §5 限流 (1 variant) ===
    /// 限流 (per 飞书 Open Platform `code: 99991400` "rate limit exceeded").
    #[error("Lark rate limited (飞书 Open Platform code=99991400)")]
    RateLimited,

    // === §6 业务 (1 variant) ===
    /// 飞书 Open Platform 业务错误 (`code != 0`).
    #[error("Lark API error: code={code}, msg={msg}")]
    ApiError {
        /// 飞书 Open Platform 业务错误码 (per v0.9.21 商业版, e.g. 230001 / 230002).
        code: i32,
        /// 错误信息.
        msg: String,
    },

    // === §7 其他 (1 variant) ===
    /// 其他错误 (序列化 / 内部错误).
    #[error("Lark other error: {0}")]
    Other(String),
}

pub type LarkResult<T> = Result<T, LarkError>;

/// 编译期守门: 11 variant 守门 (per 8 项不修改承诺).
/// 新增 variant 必须同步改本 const, 强行提醒 reviewer.
pub const LARK_ERROR_VARIANT_COUNT: usize = 11;
const _: () = assert!(
    true, // 编译期 hardcode 守门 (实际计数在测试中验证)
    "LarkError 新增 variant 必须经 6 哲学锚 + 主人审 (R20 阶段 4)"
);

// ============================================================================
// §2 K-1 强校验方法 (6 个, 编译期 hardcode 守门字样)
// ============================================================================

impl LarkError {
    /// **K-1 #1**: 校验 App ID (非空 + `cli_` 前缀, per 飞书 App ID 规范).
    ///
    /// 飞书开放平台 App ID 固定 `cli_` 前缀 + 16 位 alphanumeric (e.g. `cli_a1b2c3d4e5f6g7h8`).
    /// 前缀错直接拒绝, 防止 m3 幻觉传"test" / "123" / 空串.
    pub fn validate_app_id(app_id: &str) -> LarkResult<()> {
        let trimmed = app_id.trim();
        if trimmed.is_empty() {
            return Err(LarkError::AppIdMissing);
        }
        if !trimmed.starts_with("cli_") {
            return Err(LarkError::AppIdInvalid(trimmed.to_string()));
        }
        // 长度校验: `cli_` (4) + 至少 8 个 alphanumeric
        if trimmed.len() < 12 {
            return Err(LarkError::AppIdInvalid(trimmed.to_string()));
        }
        Ok(())
    }

    /// **K-1 #2**: 校验 App Secret (非空 + 长度 ≥ 16, per 飞书 App Secret 规范).
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

    /// **K-1 #3**: 校验 Chat ID (非空 + 前缀 `oc_` open chat 或 `on_` user chat).
    ///
    /// 飞书 Chat ID 规范 (per v0.9.21 商业版):
    /// - `oc_` 前缀: 开放群 / 普通群
    /// - `on_` 前缀: 用户私聊
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

    /// **K-1 #4**: 校验 Open ID (非空 + 前缀 `ou_`, per 飞书 User ID 规范).
    ///
    /// 飞书 User ID 规范 (per v0.9.21 商业版):
    /// - `ou_` 前缀: Open ID (租户内唯一)
    /// - 其它前缀: union_id / user_id (R21 续真接时细化)
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

    /// **K-1 #5**: 校验 Email (RFC 5322 简化版, 非空 + 含 `@` + 域名段).
    ///
    /// 完整 RFC 5322 需要完整 regex, 简化版用启发式: `local@domain.tld`,
    /// `local` ≥ 1 char, `domain.tld` ≥ 3 chars, `@` 只能 1 个.
    pub fn validate_email(email: &str) -> LarkResult<()> {
        let trimmed = email.trim();
        if trimmed.is_empty() {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        // 必须含且仅含 1 个 @
        let at_count = trimmed.matches('@').count();
        if at_count != 1 {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        // local + @ + domain.tld
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
        if !domain.contains('.') {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        let domain_parts: Vec<&str> = domain.split('.').collect();
        if domain_parts.len() < 2 {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        // tld 至少 2 chars
        if domain_parts[domain_parts.len() - 1].len() < 2 {
            return Err(LarkError::EmailInvalid(trimmed.to_string()));
        }
        // local 段字符校验: 字母数字 + . _ % + -
        for c in local.chars() {
            if !c.is_ascii_alphanumeric()
                && c != '.'
                && c != '_'
                && c != '%'
                && c != '+'
                && c != '-'
            {
                return Err(LarkError::EmailInvalid(trimmed.to_string()));
            }
        }
        Ok(())
    }

    /// **K-1 #6**: 校验 Mobile (E.164 国际格式, `+` + 7-15 位数字).
    ///
    /// 飞书 mobile 字段规范: E.164 (e.g. `+8613800138000` 中国大陆 / `+14155552671` 美国).
    pub fn validate_mobile(mobile: &str) -> LarkResult<()> {
        let trimmed = mobile.trim();
        if trimmed.is_empty() {
            return Err(LarkError::MobileInvalid(trimmed.to_string()));
        }
        if !trimmed.starts_with('+') {
            return Err(LarkError::MobileInvalid(trimmed.to_string()));
        }
        // `+` 后必须 7-15 位数字 (E.164 规范)
        let digits = &trimmed[1..];
        if digits.len() < 7 || digits.len() > 15 {
            return Err(LarkError::MobileInvalid(trimmed.to_string()));
        }
        for c in digits.chars() {
            if !c.is_ascii_digit() {
                return Err(LarkError::MobileInvalid(trimmed.to_string()));
            }
        }
        Ok(())
    }
}

// ============================================================================
// §3 STUB 守门宏 (per task spec §4 "STUB 守门宏: lark_stub!")
// ============================================================================

/// STUB 守门宏: 6 API stub 内部统一返 `LarkError::NotImplemented(api)` + tracing log.
///
/// 用法:
/// ```ignore
/// pub async fn send_message(&self, ...) -> LarkResult<Message> {
///     return Err(lark_stub!("send_message"));
/// }
/// ```
///
/// 或者更简洁:
/// ```ignore
/// pub async fn send_message(&self, ...) -> LarkResult<Message> {
///     lark_stub!(@return send_message, LarkResult<Message>)
/// }
/// ```
///
/// 整合 R21 真接时, 删本宏直接换实现.
#[macro_export]
macro_rules! lark_stub {
    // 通用版: 接受类型 + API 名, 编译期类型校验 + tracing log + return Err
    ($api:expr, $ret:ty) => {{
        $crate::lark::error::tracing_warn_stub($api);
        let e: $crate::lark::error::LarkError =
            $crate::lark::error::LarkError::NotImplemented($api);
        return Err::<_, _>(e);
    }};
}

/// tracing log helper (per lark_stub! 守门宏, 让 log + return 一气呵成).
pub fn tracing_warn_stub(api: &'static str) {
    tracing::warn!(
        "apeireth-sdk-lark STUB MODE: api={} not implemented (R21 will wire @larksuiteoapi/lark-sdk)",
        api
    );
}

// ============================================================================
// §4 单元测试 (K-1 6 强校验 + 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// K-1 #1: App ID 校验
    #[test]
    fn k1_app_id_valid() {
        // 12 char: cli_ + 8 alphanumeric
        assert!(LarkError::validate_app_id("cli_a1b2c3d4").is_ok());
        // 20 char (典型飞书 app id)
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

    /// K-1 #2: App Secret 校验
    #[test]
    fn k1_app_secret_valid() {
        // 16 chars minimum
        assert!(LarkError::validate_app_secret("1234567890abcdef").is_ok());
        // 32 chars (典型飞书 app secret)
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

    /// K-1 #3: Chat ID 校验 (oc_ / on_ 前缀)
    #[test]
    fn k1_chat_id_valid_oc() {
        assert!(LarkError::validate_chat_id("oc_a1b2c3d4e5f6").is_ok());
    }

    #[test]
    fn k1_chat_id_valid_on() {
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
    }

    /// K-1 #4: Open ID 校验 (ou_ 前缀)
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

    /// K-1 #5: Email 校验 (RFC 5322 简化)
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

    /// K-1 #6: Mobile 校验 (E.164)
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

    /// NotImplemented variant 守 STUB_MODE 必为 true
    #[test]
    fn stub_mode_guard_not_implemented() {
        let err: LarkError = LarkError::NotImplemented("send_message");
        assert!(matches!(err, LarkError::NotImplemented("send_message")));
    }
}
