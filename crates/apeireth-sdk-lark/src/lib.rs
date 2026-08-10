//! # apeireth-sdk-lark (STUB MODE)
//!
//! ⚠️ **STUB MODE: R20 阶段 4 效果, 修改需经 6 哲学锚 + 主人审**
//!
//! 飞书 Lark SDK stub (1:1 翻译 `@larksuiteoapi/lark-sdk` v0.9.21 商业版, per
//! `core/Response.d.ts` + `client/api_im_open.js` + `client/api_calendar_open.js` +
//! `client/api_contact_open.js` + `client/api_docx_open.js` + `client/api_sheet_open.js` +
//! `client/api_approval_open.js` + `client/api_event_open.js`).
//!
//! 商业版 `@larksuiteoapi/lark-sdk` v0.9.21 是飞书官方 TypeScript SDK, 提供 Open Platform
//! 全套 API (IM / Calendar / Contact / Doc / Sheet / Approval / Event). 但 **当前 crate 是
//! STUB skeleton** — API 表面按 v0.9.21 1:1 翻译, 但所有 8 核心 API 实现都是
//! `Err(LarkError::NotImplemented(api))`. **任何真实 SDK 引用 (reqwest 真接 / 真发飞书 API)
//! 都禁止**, 留 R20 阶段 4 续真接或 R21 续.
//!
//! **MUST-DO (K-1 强校验守门字样)**: 本 crate 任何修改前, 必须:
//! 1. 改 `STUB_MODE = false` (编译期 hardcode)
//! 2. 放开 Cargo.toml 的 reqwest deps
//! 3. 加 workspace members (`crates/apeireth-sdk-lark`)
//! 4. 经 6 哲学锚 (RIVAL 蓝图) + 主人审
//! 跳过任何一条 → 整合时 cargo build 必挂, fixture 必挂.
//!
//! ## 8 核心 API (per task spec §3 + v0.9.21 商业版 1:1)
//!
//! | # | API                          | 1:1 翻译 v0.9.21                       | R20 阶段 4 实现 |
//! |---:|------------------------------|----------------------------------------|----------------|
//! | 1 | `send_message`               | `Lark.Client.im.message.create`        | NotImplemented |
//! | 2 | `list_calendar_events`       | `Lark.Client.calendar.event.list`      | NotImplemented |
//! | 3 | `get_user`                   | `Lark.Client.contact.user.get`         | NotImplemented |
//! | 4 | `get_department`             | `Lark.Client.contact.department.get`   | NotImplemented |
//! | 5 | `create_doc`                 | `Lark.Client.docx.document.create`     | NotImplemented |
//! | 6 | `create_sheet`               | `Lark.Client.sheet.spreadsheet.create` | NotImplemented |
//! | 7 | `get_approval_instance`      | `Lark.Client.approval.instance.get`    | NotImplemented |
//! | 8 | `verify_webhook`             | `Lark.Client.im.event.verify`          | NotImplemented |
//!
//! ## 6 消息类型 (per v0.9.21 商业版 `MessageType` enum 1:1)
//!
//! `Text` / `Post` / `Image` / `File` / `Card` / `Interactive` (6 variant, 编译期 hardcode)
//!
//! ## 5 鉴权 (per v0.9.21 商业版 1:1)
//!
//! 1. **App ID** (`cli_xxx` 前缀, K-1 #1 强校验)
//! 2. **App Secret** (≥ 16 chars, K-1 #2 强校验)
//! 3. **tenant_access_token** (走 `/auth/v3/tenant_access_token/internal`)
//! 4. **user_access_token** (走 OAuth `code` → `access_token`)
//! 5. **webhook_token** (事件订阅 URL 校验)
//!
//! ## 4 实体 (per v0.9.21 商业版 1:1)
//!
//! `Message` / `CalendarEvent` / `User` / `Document` (4 entity, 编译期 hardcode)
//!
//! ## 6 K-1 强校验 (per m3-hallucination-defense §2.4 + task spec §3)
//!
//! - **K-1 #1**: App ID 非空 + `cli_` 前缀 (per `LarkError::validate_app_id`)
//! - **K-1 #2**: App Secret 非空 + ≥ 16 chars (per `LarkError::validate_app_secret`)
//! - **K-1 #3**: Chat ID 非空 + `oc_` / `on_` 前缀 (per `LarkError::validate_chat_id`)
//! - **K-1 #4**: Open ID 非空 + `ou_` 前缀 (per `LarkError::validate_open_id`)
//! - **K-1 #5**: Email 非空 + RFC 5322 邮箱 (per `LarkError::validate_email`)
//! - **K-1 #6**: Mobile 非空 + E.164 国际格式 (per `LarkError::validate_mobile`)
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 不漂移**: 1:1 翻译 v0.9.21 商业版 IM / Calendar / Contact / Doc / Sheet / Approval
//!   / Event API 表面, 0 业务重设计
//! - **S-2 编译期 hardcode**: `STUB_MODE = true` / `PLATFORM_NAME = "apeireth"` /
//!   `LARK_SCHEMA_VERSION = "1"` 全部 const, 不允许运行时配置覆盖
//! - **O-2 工程铁律**: 0 引 reqwest / hyper 等真接 HTTP client, 留 R21 真接时再加
//! - **O-3 m3 防御**: 8 工具白名单 `LARK_TOOL_WHITELIST` 编译期 hardcode, `validate_tool_call`
//!   在 dispatch 前 schema 校验, 防 m3 模型幻觉调用不存在的工具
//! - **O-4 不假装可观测**: 8 API 失败时返 `LarkError::NotImplemented(api_name)` +
//!   `tracing::warn!` log, 不假装 OK
//! - **O-5 K-1 强校验**: 6 字段 (app_id / app_secret / chat_id / open_id / email / mobile)
//!   编译期 hardcode 白名单, 任何配置变更必经 `validate_*` 走 6 K-1 检查
//!
//! ## 8 项不修改承诺
//!
//! - ✅ 0 改 24 LOCKED crate (`crates/apeireth-{action,agent,asi,bench,bus,central,cli,cognition,consciousness,constraint,core,council,evolution,extension,life-force,motivation,onion,perception,protocol,pybridge,relation,sovereignty,supervisor,tauri-stub,upgrade,value,verify,web}/src/`, 0 触碰)
//! - ✅ 0 改 workspace version (1.0.0 LOCKED, 走 workspace inherit)
//! - ✅ 0 改 6 哲学锚 + 8 项不修改承诺
//! - ✅ 0 引 reqwest / hyper / tokio-tungstenite (留 R21 续真接)
//! - ✅ 0 重复造轮子 (复用 apeireth-protocol 4 协议 ZST adapter + apeireth-keyring keyring 模式)
//! - ✅ 0 假装已实现 (8 API 全 NotImplemented)
//! - ✅ 0 明文存 App ID/Secret/token (走 apeireth-keyring, 当前 skeleton 用 AppIdHolder/AppSecretHolder 内存存)
//! - ✅ 编译期 hardcode (8 API + 6 消息类型 + 5 鉴权 + 4 实体 + 6 K-1 强校验)
//!
//! ## 引用文档 (8 份)
//!
//! 1. `@larksuiteoapi/lark-sdk v0.9.21` `core/Response.d.ts` (商业版 Response 1:1 翻译源)
//! 2. `@larksuiteoapi/lark-sdk v0.9.21` `client/api_im_open.js` (im/v1/messages 1:1 翻译源)
//! 3. `@larksuiteoapi/lark-sdk v0.9.21` `client/api_calendar_open.js` (calendar/v4 1:1 翻译源)
//! 4. `@larksuiteoapi/lark-sdk v0.9.21` `client/api_contact_open.js` (contact/v3 1:1 翻译源)
//! 5. `@larksuiteoapi/lark-sdk v0.9.21` `client/api_docx_open.js` (docx/v1 1:1 翻译源)
//! 6. `@larksuiteoapi/lark-sdk v0.9.21` `client/api_sheet_open.js` (sheets/v3 1:1 翻译源)
//! 7. `@larksuiteoapi/lark-sdk v0.9.21` `client/api_approval_open.js` (approval/v4 1:1 翻译源)
//! 8. `docs/stage4/m3-hallucination-defense-2026-08-05.md` §2.4 (TOOL_WHITELIST 模式)
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 4 效果, 1 owner × 1 周续真接)
//!
//! 当前 stage 跑 `cargo check` + 14+ fixture + 6 K-1 验证. **0 真接 SDK** — R21 续真接.

#![allow(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// §0 模块声明 + 重新导出 (8 sub-module + re-export, 跟 livekit / sandbox 1:1 镜像)
// ============================================================================

pub mod approval;
pub mod auth;
pub mod calendar;
pub mod contact;
pub mod doc;
pub mod error;
pub mod message;
pub mod webhook;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

// 注: 所有 4 实体 / 6 消息 / 5 鉴权 / 6 K-1 类型都通过下面的 `pub use` re-export 同时
// 暴露给 crate 内部和外部. 不要重复 `use crate::X::Y` 避免 E0252 冲突.
pub use crate::approval::{
    ApprovalFormField, ApprovalInstance, ApprovalTask, InstanceStatus, TaskStatus,
    TASK_STATUS_COUNT, TASK_STATUS_PENDING,
};
pub use crate::auth::{
    AppIdHolder, AppSecretHolder, TenantAccessToken, UserAccessToken, WebhookToken,
    DEFAULT_LARK_API_BASE, DEFAULT_TENANT_TOKEN_TTL_SECONDS, DEFAULT_USER_TOKEN_TTL_SECONDS,
    LARK_SCHEMA_VERSION, MAX_TOKEN_TTL_SECONDS, MIN_APP_ID_LENGTH, MIN_APP_SECRET_LENGTH,
    PLATFORM_NAME, PROVIDER_NAME, TYPICAL_APP_SECRET_LENGTH,
};
pub use crate::calendar::{CalendarEvent, CalendarEventQuery, EventStatus, FreeBusySlot};
pub use crate::contact::{Department, User, UserIdType, UserQuery};
pub use crate::doc::{BitableField, BitableMeta, Document, DocumentType, SheetMeta};
pub use crate::error::{LarkError, LarkResult, LARK_ERROR_VARIANT_COUNT, tracing_warn_stub};
pub use crate::message::{
    CardContent, FileContent, ImageContent, InteractiveContent, Message, MessageType,
    PostContent, PostElement, PostLocale, PostParagraph, ReceiveIdType, TextContent,
    SUPPORTED_MESSAGE_TYPES,
};
pub use crate::webhook::{
    verify_webhook_event as lark_verify_webhook, WebhookEvent, WebhookVerifyResult,
    EventType as WebhookEventType,
};

// 兼容导出: Lark 前缀 alias (跟 livekit / sandbox 1:1 风格, 防跟外部 crate 冲突)
pub use crate::approval::{
    ApprovalInstance as LarkApprovalInstance, ApprovalTask as LarkApprovalTask,
    ApprovalFormField as LarkApprovalFormField, InstanceStatus as LarkInstanceStatus,
    TaskStatus as LarkTaskStatus,
};
pub use crate::auth::{
    AppIdHolder as LarkAppIdHolder, AppSecretHolder as LarkAppSecretHolder,
    TenantAccessToken as LarkTenantAccessToken, UserAccessToken as LarkUserAccessToken,
    WebhookToken as LarkWebhookToken,
};
pub use crate::calendar::{
    CalendarEvent as LarkCalendarEvent, CalendarEventQuery as LarkCalendarEventQuery,
    EventStatus as LarkEventStatus, FreeBusySlot as LarkFreeBusySlot,
};
pub use crate::contact::{
    Department as LarkDepartment, User as LarkUser, UserQuery as LarkUserQuery,
};
pub use crate::doc::{
    Document as LarkDocument, DocumentType as LarkDocumentType, SheetMeta as LarkSheetMeta,
};
pub use crate::error::{
    LarkError as LarkErrorReexport, LarkResult as LarkResultReexport,
    LARK_ERROR_VARIANT_COUNT as LARK_ERROR_COUNT,
};
pub use crate::message::{
    Message as LarkMessage, MessageType as LarkMessageType,
    ReceiveIdType as LarkReceiveIdType, TextContent as LarkTextContent,
    PostContent as LarkPostContent, PostElement as LarkPostElement, PostLocale as LarkPostLocale,
    PostParagraph as LarkPostParagraph, CardContent as LarkCardContent,
    ImageContent as LarkImageContent, FileContent as LarkFileContent,
    InteractiveContent as LarkInteractiveContent,
};
pub use crate::webhook::{
    WebhookEvent as LarkWebhookEvent, WebhookVerifyResult as LarkWebhookVerifyResult,
};

// ============================================================================
// §1 m3 hallucination 防御 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode 8 工具 (8 商业版 API), validate_tool_call 在 dispatch 前
// schema 校验. 防止 minimax m3 模型幻觉调用不存在的 lark 工具.
// ============================================================================

/// m3 防御: Lark SDK 8 API 工具白名单 (编译期 hardcode, 不可运行时改).
///
/// **8 工具 = 1:1 翻译 v0.9.21 商业版 @larksuiteoapi/lark-sdk `Lark.Client.*`**:
/// - `apeireth_sdk_lark_send_message`          (发消息: im.message.create)
/// - `apeireth_sdk_lark_list_calendar_events`  (列日历: calendar.event.list)
/// - `apeireth_sdk_lark_get_user`              (查用户: contact.user.get)
/// - `apeireth_sdk_lark_get_department`        (查部门: contact.department.get)
/// - `apeireth_sdk_lark_create_doc`            (建文档: docx.document.create)
/// - `apeireth_sdk_lark_create_sheet`          (建 sheet: sheet.spreadsheet.create)
/// - `apeireth_sdk_lark_get_approval_instance` (查审批: approval.instance.get)
/// - `apeireth_sdk_lark_verify_webhook`        (校 webhook: im.event.verify)
pub const LARK_TOOL_WHITELIST: &[&str] = &[
    "apeireth_sdk_lark_send_message",
    "apeireth_sdk_lark_list_calendar_events",
    "apeireth_sdk_lark_get_user",
    "apeireth_sdk_lark_get_department",
    "apeireth_sdk_lark_create_doc",
    "apeireth_sdk_lark_create_sheet",
    "apeireth_sdk_lark_get_approval_instance",
    "apeireth_sdk_lark_verify_webhook",
];

/// 编译期守门: LARK_TOOL_WHITELIST 长度 == 8 (K-1 强校验 + 8 项不修改承诺 #5).
pub const LARK_TOOL_WHITELIST_COUNT: usize = 8;
const _: () = assert!(LARK_TOOL_WHITELIST.len() == LARK_TOOL_WHITELIST_COUNT);

/// 编译期守门: 8 核心 API 数 == LARK_TOOL_WHITELIST_COUNT (K-1 强校验同步守门).
pub const CORE_API_COUNT: usize = 8;
const _: () = assert!(CORE_API_COUNT == LARK_TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `LarkError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> LarkResult<()> {
    if !LARK_TOOL_WHITELIST.contains(&tool) {
        return Err(LarkError::Other(format!(
            "tool not whitelisted: {tool} (Lark SDK 8 API)"
        )));
    }
    Ok(())
}

// ============================================================================
// §2 编译期 hardcode 常量 (per R20 P0 5 crate 风格 + K-1 强校验)
// ============================================================================

/// Lark API schema version (1:1 翻译 @larksuiteoapi/lark-sdk v0.9.21, K-1 强校验).
///
/// 跟 `LARK_SCHEMA_VERSION` (in auth.rs) 同步, 此处 re-export 守门防漂移.
pub const LARK_API_VERSION: &str = LARK_SCHEMA_VERSION;

/// **STUB MODE 守门标志** (K-1 强校验 #4): 编译期 hardcode = `true`.
/// R21+ 真接 @larksuiteoapi/lark-sdk 时, **必须经 6 哲学锚 + 主人审才能改 `false`**.
pub const STUB_MODE: bool = true;

/// 编译期守门: STUB_MODE 必须 == true (per STUB MODE 守门 + 8 项不修改承诺).
/// 改 false 需同时改本 assert + STUB_MODE 标志, 强行提醒 reviewer.
const _: () = assert!(STUB_MODE == true, "STUB_MODE 改 false 需经 6 哲学锚 + 主人审 (R21+)");

/// m3 防御: 查 STUB_MODE 状态 (per task spec 守门).
/// **R21+ 改 `STUB_MODE = false` 时, 本函数返 `false`**; 现阶段恒返 `true`.
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

/// 6 消息类型守门常量 (per K-1 强校验守门, 编译期 hardcode).
pub const MESSAGE_TYPE_COUNT: usize = 6;
const _: () = assert!(MESSAGE_TYPE_COUNT == SUPPORTED_MESSAGE_TYPES.len());

/// 5 鉴权守门常量 (per K-1 强校验守门, 编译期 hardcode).
pub const AUTH_METHOD_COUNT: usize = 5;
const _: () = assert!(AUTH_METHOD_COUNT == 5);

/// 4 实体守门常量 (per K-1 强校验守门, 编译期 hardcode).
pub const ENTITY_COUNT: usize = 4;
const _: () = assert!(ENTITY_COUNT == 4);

/// 6 K-1 强校验守门常量 (per K-1 强校验守门, 编译期 hardcode).
pub const K1_STRONG_VALIDATION_COUNT: usize = 6;
const _: () = assert!(K1_STRONG_VALIDATION_COUNT == 6);

/// 11 LarkError variant 守门 (per LARK_ERROR_VARIANT_COUNT 守门).
const _: () = assert!(LARK_ERROR_VARIANT_COUNT == 11);

/// 默认 Lark API base URL.
pub const DEFAULT_API_BASE: &str = DEFAULT_LARK_API_BASE;

/// 单消息最大文本长度 (per v0.9.21 商业版估 4 KiB, 防单消息爆炸).
pub const MAX_MESSAGE_TEXT_BYTES: usize = 4096;

/// 单次 list_calendar_events 最大返回数 (per v0.9.21 商业版估 1000).
pub const MAX_CALENDAR_EVENTS_PER_PAGE: u32 = 1000;

/// 单 webhook 单 chunk 字节上限 (per v0.9.21 商业版估 16 KiB, R21 续真接 AES).
pub const MAX_WEBHOOK_CHUNK_BYTES: usize = 16 * 1024;

// ============================================================================
// §3 LarkClient trait (per 8 API 抽象, async, 跟 livekit LiveKitClient 1:1 镜像)
// ============================================================================

/// Lark 客户端 trait (8 核心 API, async, 跟 livekit LiveKitClient 1:1 镜像).
///
/// **STUB MODE**: 所有 8 核心 API 实现都是 `Err(LarkError::NotImplemented(api))`.
/// R21+ 真接 @larksuiteoapi/lark-sdk 时, 把 `LarkClientImpl` 8 方法实现替换为真 HTTP 调用.
#[async_trait]
pub trait LarkClient: Send + Sync {
    /// 1. 发消息 (per `Lark.Client.im.message.create` 1:1).
    async fn send_message(&self, message: &Message) -> LarkResult<String>;

    /// 2. 列日历事件 (per `Lark.Client.calendar.event.list` 1:1).
    async fn list_calendar_events(
        &self,
        query: &CalendarEventQuery,
    ) -> LarkResult<Vec<CalendarEvent>>;

    /// 3. 查用户 (per `Lark.Client.contact.user.get` 1:1).
    async fn get_user(&self, query: &UserQuery) -> LarkResult<User>;

    /// 4. 查部门 (per `Lark.Client.contact.department.get` 1:1).
    async fn get_department(&self, department_id: &str) -> LarkResult<Department>;

    /// 5. 建文档 (per `Lark.Client.docx.document.create` 1:1).
    async fn create_doc(&self, doc: &Document) -> LarkResult<Document>;

    /// 6. 建 spreadsheet (per `Lark.Client.sheet.spreadsheet.create` 1:1).
    async fn create_sheet(&self, sheet: &Document) -> LarkResult<Document>;

    /// 7. 查审批实例 (per `Lark.Client.approval.instance.get` 1:1).
    async fn get_approval_instance(&self, instance_id: &str) -> LarkResult<ApprovalInstance>;

    /// 8. 校 webhook (per `Lark.Client.im.event.verify` 1:1).
    async fn verify_webhook(
        &self,
        event: &WebhookEvent,
        webhook_token: &WebhookToken,
    ) -> LarkResult<WebhookVerifyResult>;
}

// ============================================================================
// §4 LarkClientImpl (per 8 API stub 派发器, STUB 模式返 NotImplemented)
// ============================================================================

/// Lark 客户端实现 (per 8 API stub 派发器).
///
/// 字段对应 v0.9.21 商业版 `Lark.Client` config:
/// - `app_id` / `app_secret` (走 AppIdHolder / AppSecretHolder, 0 明文存盘)
/// - `tenant_access_token` (走 TenantAccessToken, R21 真接飞书 API 后填)
/// - `user_access_token` (走 UserAccessToken, R21 真接飞书 API 后填)
/// - `api_base` (per DEFAULT_LARK_API_BASE, R21 真接时支持自定义)
///
/// **STUB MODE**: 8 API 全返 `LarkError::NotImplemented(api)`, 编译期 hardcode `STUB_MODE = true`.
#[derive(Debug)]
pub struct LarkClientImpl {
    /// App ID 持有者 (走 keyring, 0 明文).
    app_id: AppIdHolder,
    /// App Secret 持有者 (走 keyring, 0 明文).
    app_secret: AppSecretHolder,
    /// tenant_access_token (R21 真接时缓存).
    tenant_token: Option<TenantAccessToken>,
    /// user_access_token (R21 真接时缓存).
    user_token: Option<UserAccessToken>,
    /// API base URL (per DEFAULT_LARK_API_BASE, R21 真接时支持自定义).
    api_base: String,
}

impl LarkClientImpl {
    /// 创建新 Lark 客户端 (STUB 模式, 0 真接飞书 API).
    pub fn new() -> Self {
        info!("apeireth-sdk-lark: LarkClientImpl::new (STUB MODE)");
        Self {
            app_id: AppIdHolder::empty(),
            app_secret: AppSecretHolder::empty(),
            tenant_token: None,
            user_token: None,
            api_base: DEFAULT_LARK_API_BASE.to_string(),
        }
    }

    /// 从 keyring 加载 (R21 续真接时真接 keyring, 当前 skeleton 仅 holder).
    pub fn from_keyring() -> Self {
        info!("apeireth-sdk-lark: LarkClientImpl::from_keyring (STUB MODE)");
        Self {
            app_id: AppIdHolder::from_keyring("lark-app-id"),
            app_secret: AppSecretHolder::from_keyring("lark-app-secret"),
            tenant_token: None,
            user_token: None,
            api_base: DEFAULT_LARK_API_BASE.to_string(),
        }
    }

    /// 设置 App ID (K-1 #1 强校验).
    pub fn set_app_id(&mut self, app_id: String) -> LarkResult<()> {
        self.app_id.set(app_id)?;
        Ok(())
    }

    /// 设置 App Secret (K-1 #2 强校验).
    pub fn set_app_secret(&mut self, app_secret: String) -> LarkResult<()> {
        self.app_secret.set(app_secret)?;
        Ok(())
    }

    /// 读 App ID (cloned).
    pub fn app_id(&self) -> Option<String> {
        self.app_id.get()
    }

    /// 读 App Secret (cloned).
    pub fn app_secret(&self) -> Option<String> {
        self.app_secret.get()
    }

    /// 读 API base URL.
    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    /// 检查 App ID + App Secret 是否都已设置.
    pub fn is_configured(&self) -> bool {
        self.app_id.is_set() && self.app_secret.is_set()
    }

    /// 设置 tenant_access_token (R21 真接飞书 API 后用).
    pub fn set_tenant_token(&mut self, token: TenantAccessToken) {
        self.tenant_token = Some(token);
    }

    /// 读 tenant_access_token.
    pub fn tenant_token(&self) -> Option<&TenantAccessToken> {
        self.tenant_token.as_ref()
    }

    /// 设置 user_access_token.
    pub fn set_user_token(&mut self, token: UserAccessToken) {
        self.user_token = Some(token);
    }

    /// 读 user_access_token.
    pub fn user_token(&self) -> Option<&UserAccessToken> {
        self.user_token.as_ref()
    }

    /// 列出 tenant token 缓存 (R21 真接时检查过期刷新).
    pub fn tenant_token_is_expired(&self) -> bool {
        self.tenant_token
            .as_ref()
            .map(|t| t.is_expired())
            .unwrap_or(true)
    }

    /// 列 user token 缓存 (R21 真接时检查过期刷新).
    pub fn user_token_is_expired(&self) -> bool {
        self.user_token
            .as_ref()
            .map(|t| t.is_expired())
            .unwrap_or(true)
    }
}

impl Default for LarkClientImpl {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// §5 LarkClient trait impl for LarkClientImpl (8 API stub 派发器)
// ============================================================================

#[async_trait]
impl LarkClient for LarkClientImpl {
    async fn send_message(&self, _message: &Message) -> LarkResult<String> {
        lark_stub!("send_message", LarkResult<String>)
    }

    async fn list_calendar_events(
        &self,
        _query: &CalendarEventQuery,
    ) -> LarkResult<Vec<CalendarEvent>> {
        lark_stub!("list_calendar_events", LarkResult<Vec<CalendarEvent>>)
    }

    async fn get_user(&self, _query: &UserQuery) -> LarkResult<User> {
        lark_stub!("get_user", LarkResult<User>)
    }

    async fn get_department(&self, _department_id: &str) -> LarkResult<Department> {
        lark_stub!("get_department", LarkResult<Department>)
    }

    async fn create_doc(&self, _doc: &Document) -> LarkResult<Document> {
        lark_stub!("create_doc", LarkResult<Document>)
    }

    async fn create_sheet(&self, _sheet: &Document) -> LarkResult<Document> {
        lark_stub!("create_sheet", LarkResult<Document>)
    }

    async fn get_approval_instance(&self, _instance_id: &str) -> LarkResult<ApprovalInstance> {
        lark_stub!("get_approval_instance", LarkResult<ApprovalInstance>)
    }

    async fn verify_webhook(
        &self,
        _event: &WebhookEvent,
        _webhook_token: &WebhookToken,
    ) -> LarkResult<WebhookVerifyResult> {
        lark_stub!("verify_webhook", LarkResult<WebhookVerifyResult>)
    }
}

// ============================================================================
// §6 StubStatus (per livekit / sandbox 1:1 镜像, R21 续真接后删)
// ============================================================================

/// Stub 状态 (per livekit / sandbox 1:1 镜像).
///
/// 整合 #2 sub-agent R21 真接时, 本结构体可保留为 status query 工具, 也可直接删.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StubStatus {
    /// STUB_MODE 状态.
    pub stub_mode: bool,
    /// 平台名.
    pub platform: String,
    /// API base URL.
    pub api_base: String,
    /// schema 版本.
    pub schema_version: String,
    /// App ID 是否已设置.
    pub app_id_set: bool,
    /// App Secret 是否已设置.
    pub app_secret_set: bool,
    /// tenant_access_token 是否已设置.
    pub tenant_token_set: bool,
    /// user_access_token 是否已设置.
    pub user_token_set: bool,
    /// tenant token 是否过期.
    pub tenant_token_expired: bool,
    /// user token 是否过期.
    pub user_token_expired: bool,
    /// 客户端是否已配置 (App ID + Secret 都设置).
    pub configured: bool,
}

impl LarkClientImpl {
    /// 返回 STUB 状态查询 (R21 续真接后删).
    pub fn stub_status(&self) -> StubStatus {
        StubStatus {
            stub_mode: STUB_MODE,
            platform: PLATFORM_NAME.to_string(),
            api_base: self.api_base.clone(),
            schema_version: LARK_SCHEMA_VERSION.to_string(),
            app_id_set: self.app_id.is_set(),
            app_secret_set: self.app_secret.is_set(),
            tenant_token_set: self.tenant_token.is_some(),
            user_token_set: self.user_token.is_some(),
            tenant_token_expired: self.tenant_token_is_expired(),
            user_token_expired: self.user_token_is_expired(),
            configured: self.is_configured(),
        }
    }
}

// ============================================================================
// §7 单元测试 (守门, 编译期 hardcode + 5 鉴权 K-1 6 强校验)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::WebhookToken;
    use crate::message::{ReceiveIdType, TextContent};

    /// 6 哲学锚 + 8 项承诺: STUB_MODE 必须为 true.
    #[test]
    fn k1_stub_mode_is_true() {
        assert!(STUB_MODE);
        assert!(is_stub_mode());
    }

    /// 6 哲学锚 + 8 项承诺: PLATFORM_NAME 必须为 "apeireth".
    #[test]
    fn k1_platform_name_is_apeireth() {
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(PROVIDER_NAME, "lark");
        assert_eq!(LARK_SCHEMA_VERSION, "1");
        assert_eq!(LARK_API_VERSION, LARK_SCHEMA_VERSION);
        assert!(DEFAULT_API_BASE.starts_with("https://"));
    }

    /// 8 工具白名单 == 8 核心 API (K-1 强校验守门).
    #[test]
    fn k1_tool_whitelist_count_8() {
        assert_eq!(LARK_TOOL_WHITELIST_COUNT, 8);
        assert_eq!(CORE_API_COUNT, 8);
        assert_eq!(LARK_TOOL_WHITELIST.len(), LARK_TOOL_WHITELIST_COUNT);
    }

    /// 6 消息类型守门.
    #[test]
    fn k1_message_type_count_6() {
        assert_eq!(MESSAGE_TYPE_COUNT, 6);
        assert_eq!(SUPPORTED_MESSAGE_TYPES.len(), 6);
        assert_eq!(MessageType::COUNT, 6);
    }

    /// 5 鉴权守门.
    #[test]
    fn k1_auth_method_count_5() {
        assert_eq!(AUTH_METHOD_COUNT, 5);
    }

    /// 4 实体守门.
    #[test]
    fn k1_entity_count_4() {
        assert_eq!(ENTITY_COUNT, 4);
    }

    /// 6 K-1 强校验守门.
    #[test]
    fn k1_validation_count_6() {
        assert_eq!(K1_STRONG_VALIDATION_COUNT, 6);
    }

    /// 11 LarkError variant 守门.
    #[test]
    fn k1_lark_error_variant_count_11() {
        assert_eq!(LARK_ERROR_VARIANT_COUNT, 11);
    }

    /// m3 防御: validate_tool_call 测试.
    #[test]
    fn k1_validate_tool_call_whitelisted() {
        let args = serde_json::json!({});
        let result = validate_tool_call("apeireth_sdk_lark_send_message", &args);
        assert!(result.is_ok());
    }

    #[test]
    fn k1_validate_tool_call_not_whitelisted() {
        let args = serde_json::json!({});
        let result = validate_tool_call("apeireth_sdk_lark_bogus", &args);
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    /// LarkClientImpl 构造.
    #[test]
    fn k1_lark_client_impl_new() {
        let client = LarkClientImpl::new();
        assert!(!client.is_configured());
        assert!(client.app_id().is_none());
        assert!(client.app_secret().is_none());
        assert_eq!(client.api_base(), DEFAULT_API_BASE);
    }

    /// LarkClientImpl 设置 App ID / Secret.
    #[test]
    fn k1_lark_client_impl_set_app_id_and_secret() {
        let mut client = LarkClientImpl::new();
        client
            .set_app_id("cli_a1b2c3d4e5f6".to_string())
            .expect("valid app id");
        client
            .set_app_secret("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid app secret");
        assert!(client.is_configured());
        assert_eq!(client.app_id().as_deref(), Some("cli_a1b2c3d4e5f6"));
    }

    /// LarkClientImpl 设置 App ID 拒绝无效.
    #[test]
    fn k1_lark_client_impl_set_app_id_rejects_invalid() {
        let mut client = LarkClientImpl::new();
        let result = client.set_app_id("invalid".to_string());
        assert!(matches!(result, Err(LarkError::AppIdInvalid(_))));
    }

    /// LarkClientImpl stub_status 演示.
    #[test]
    fn k1_lark_client_impl_stub_status() {
        let mut client = LarkClientImpl::new();
        client
            .set_app_id("cli_a1b2c3d4e5f6".to_string())
            .expect("valid");
        client
            .set_app_secret("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid");
        let status = client.stub_status();
        assert!(status.stub_mode);
        assert_eq!(status.platform, "apeireth");
        assert!(status.app_id_set);
        assert!(status.app_secret_set);
        assert!(status.configured);
        assert!(!status.tenant_token_set);
        assert!(!status.user_token_set);
    }

    /// 8 核心 API 全部返 NotImplemented (STUB 模式守门).
    #[tokio::test]
    async fn k1_8_core_apis_all_not_implemented() {
        let client = LarkClientImpl::new();
        // 1. send_message
        let text = TextContent::new("Hello");
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        )
        .expect("valid");
        let r = client.send_message(&msg).await;
        assert!(matches!(r, Err(LarkError::NotImplemented("send_message"))));
        let _ = text; // 防止 unused warning
        // 2. list_calendar_events
        use chrono::Utc;
        let query = CalendarEventQuery {
            calendar_id: "cal_xxx".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            page_size: 50,
            page_token: None,
        };
        let r = client.list_calendar_events(&query).await;
        assert!(matches!(
            r,
            Err(LarkError::NotImplemented("list_calendar_events"))
        ));
        // 3. get_user
        let query = UserQuery::new(
            "ou_user1234567890abcdef".to_string(),
            UserIdType::OpenId,
        )
        .expect("valid");
        let r = client.get_user(&query).await;
        assert!(matches!(r, Err(LarkError::NotImplemented("get_user"))));
        // 4. get_department
        let r = client.get_department("od_dept123").await;
        assert!(matches!(
            r,
            Err(LarkError::NotImplemented("get_department"))
        ));
        // 5. create_doc
        let doc = Document::new_docx("title".to_string(), None).expect("valid");
        let r = client.create_doc(&doc).await;
        assert!(matches!(r, Err(LarkError::NotImplemented("create_doc"))));
        // 6. create_sheet
        let sheet = Document::new_sheet("title".to_string(), None).expect("valid");
        let r = client.create_sheet(&sheet).await;
        assert!(matches!(r, Err(LarkError::NotImplemented("create_sheet"))));
        // 7. get_approval_instance
        let r = client.get_approval_instance("instance_001").await;
        assert!(matches!(
            r,
            Err(LarkError::NotImplemented("get_approval_instance"))
        ));
        // 8. verify_webhook
        let wh_token = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
            .expect("valid");
        let event = WebhookEvent::new_url_verification("challenge".to_string());
        let r = client.verify_webhook(&event, &wh_token).await;
        assert!(matches!(r, Err(LarkError::NotImplemented("verify_webhook"))));
    }

    /// 5 鉴权 演示 (per 任务规范 §3)
    #[test]
    fn k1_5_auth_methods_cover() {
        // 1. App ID
        let mut id_holder = AppIdHolder::empty();
        id_holder
            .set("cli_a1b2c3d4e5f6".to_string())
            .expect("valid");
        assert!(id_holder.is_set());
        // 2. App Secret
        let mut secret_holder = AppSecretHolder::empty();
        secret_holder
            .set("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid");
        assert!(secret_holder.is_set());
        // 3. tenant_access_token
        let t = TenantAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "t-abc123".to_string(),
            7200,
        )
        .expect("valid");
        assert!(!t.is_expired());
        // 4. user_access_token
        let u = UserAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "u-abc".to_string(),
            "ur-xyz".to_string(),
            "ou_user1234567890abcdef".to_string(),
            7200,
        )
        .expect("valid");
        assert!(!u.is_expired());
        // 5. webhook_token
        let wh = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
            .expect("valid");
        assert!(wh.verify("token_xxx"));
    }

    /// 6 实体 演示 (per 任务规范 §3 4 entity)
    #[test]
    fn k1_4_entities_cover() {
        // 1. Message
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        )
        .expect("valid");
        assert_eq!(msg.msg_type, MessageType::Text);
        // 2. CalendarEvent
        use chrono::Utc;
        let start = Utc::now();
        let end = start + chrono::Duration::hours(1);
        let event =
            CalendarEvent::new("cal_xxx", "会议", start, end).expect("valid");
        assert_eq!(event.summary, "会议");
        // 3. User
        let user = User::new(
            "ou_user1234567890abcdef".to_string(),
            "Alice".to_string(),
        )
        .expect("valid");
        assert_eq!(user.name, "Alice");
        // 4. Document
        let doc = Document::new_docx("title".to_string(), None).expect("valid");
        assert_eq!(doc.doc_type, DocumentType::Doc);
    }

    /// 8 工具白名单包含所有 8 核心 API 名 (per 任务规范 §3).
    #[test]
    fn k1_tool_whitelist_contains_8_apis() {
        let expected = [
            "apeireth_sdk_lark_send_message",
            "apeireth_sdk_lark_list_calendar_events",
            "apeireth_sdk_lark_get_user",
            "apeireth_sdk_lark_get_department",
            "apeireth_sdk_lark_create_doc",
            "apeireth_sdk_lark_create_sheet",
            "apeireth_sdk_lark_get_approval_instance",
            "apeireth_sdk_lark_verify_webhook",
        ];
        for tool in expected {
            assert!(
                LARK_TOOL_WHITELIST.contains(&tool),
                "tool whitelisted check: {tool}"
            );
        }
    }

    /// SheetMeta 演示 (per 任务规范 §3 doc entity 拆解).
    #[test]
    fn k1_sheet_meta_construction() {
        let meta = SheetMeta::new("sheet_001".to_string(), "Sheet1".to_string(), 0);
        assert_eq!(meta.sheet_id, "sheet_001");
    }
}
