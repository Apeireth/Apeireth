//! # apeireth-sdk-lark — 组织协作平台客户端子模块 (真实现实现)
//!
//! 本模块是组织协作集成的完整客户端实现: 协议逻辑 (认证头 / 分页 / 重试 /
//! 限流退避) + 严格请求/响应类型 (serde 严格形状 + 未知字段容错) + 错误分类
//! (可重试 / 永久 / 认证失败, 闭合词表) + 字段校验 (6 K-1 强校验)。
//! 外部服务在测试中一律以本地 mock HTTP 边界替代, 0 真实网络。
//!
//! ## 8 核心 API (全部真实现, 0 占位桩)
//!
//! | # | API | 协议端点 | 实现 |
//! |---:|---|---|---|
//! | 1 | [`LarkClient::send_message`] | `POST /im/v1/messages` | [`message`] 面 |
//! | 2 | [`LarkClient::list_calendar_events`] | `GET /calendar/v4/calendars/{id}/events` | [`calendar`] 面 (分页合并) |
//! | 3 | [`LarkClient::get_user`] | `GET /contact/v3/users/{id}` | [`contact`] 面 |
//! | 4 | [`LarkClient::get_department`] | `GET /contact/v3/departments/{id}` | [`contact`] 面 |
//! | 5 | [`LarkClient::create_doc`] | `POST /docx/v1/documents` | [`doc`] 面 |
//! | 6 | [`LarkClient::create_sheet`] | `POST /sheets/v3/spreadsheets` | [`doc`] 面 |
//! | 7 | [`LarkClient::get_approval_instance`] | `GET /approval/v4/instances/{id}` | [`approval`] 面 |
//! | 8 | [`LarkClient::verify_webhook`] | 本地校验 + AES 解密 | [`webhook`] 面 |
//!
//! ## 分层
//!
//! - [`http`] — 传输基座: `Authorization: Bearer <tenant_access_token>` 认证头、
//!   重试 + 限流退避 ([`RetryPolicy`])、整次调用 [`apeireth_core::deadline::Deadline`]
//!   超时预算、token 自动补/刷 (内存 → [`TokenCache`] 持久化缓存 → 平台颁发)、
//!   统一响应信封解析、[`Page`] 分页基元、脱敏日志。
//! - [`auth`] — 凭证持有 + token 生命周期 + `storage_atomic` 持久化 token 缓存。
//! - [`error`] — 事实错误 ([`LarkError`]) + 闭合分类 ([`ErrorClass`]) + 6 K-1 字段强校验。
//! - [`message`] / [`calendar`] / [`contact`] / [`doc`] / [`approval`] / [`webhook`]
//!   — 各协议面的 wire 契约 (请求构造 + 响应映射 + 字段校验)。
//!
//! ## 显式不支持 (非占位桩, 逐处声明)
//!
//! - `credential_store` —— 系统凭据库 (操作系统凭据设施) 的接入依赖部署环境,
//!   由 [`AppIdHolder::from_credential_store`] 等入口显式返回
//!   `Err(LarkError::Unsupported("credential_store"))`; 部署方在外围注入凭证后
//!   调用 `set_app_id` / `set_app_secret`。
//! - `webhook_event_type` —— 入站回调信封形状不在覆盖范围内时显式
//!   `Err(LarkError::Unsupported("webhook_event_type"))`, 0 静默放行。
//!
//! ## 6 K-1 字段强校验
//!
//! | K-1 | 字段 | 守门 | 失败变体 |
//! |---:|---|---|---|
//! | #1 | app_id | 非空 + `cli_` 前缀 + 主体字母数字 | `AppIdMissing` / `AppIdInvalid` |
//! | #2 | app_secret | 非空 + ≥ 16 字符 | `AppSecretMissing` / `AppSecretInvalid` |
//! | #3 | chat_id | 非空 + `oc_` / `on_` 前缀 | `ChatIdInvalid` |
//! | #4 | open_id | 非空 + `ou_` 前缀 | `OpenIdInvalid` |
//! | #5 | email | RFC 5322 简化语法 | `EmailInvalid` |
//! | #6 | mobile | E.164 (`+` + 7-15 位) | `MobileInvalid` |
//!
//! ## 安全口径
//!
//! - App ID / App Secret / 3 类 token 0 明文进日志 (Debug 全脱敏 `[redacted]`);
//! - token 落盘只经 [`TokenCache`] (storage_atomic 原子写 + 文件锁 + 0600);
//! - webhook 共享秘密 0 进错误串 (恒定时间比较 + 错误只报事实类别)。

#![allow(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// §0 模块声明 + 重新导出
// ============================================================================

pub mod approval;
pub mod auth;
pub mod calendar;
pub mod contact;
pub mod doc;
pub mod error;
pub mod http;
pub mod message;
pub mod webhook;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use crate::lark::approval::{
    ApprovalFormField, ApprovalInstance, ApprovalTask, InstanceStatus, TaskStatus,
    TASK_STATUS_COUNT, TASK_STATUS_PENDING,
};
pub use crate::lark::auth::{
    AppIdHolder, AppSecretHolder, TenantAccessToken, TokenCache, UserAccessToken, WebhookToken,
    DEFAULT_LARK_API_BASE, DEFAULT_TENANT_TOKEN_TTL_SECONDS, DEFAULT_USER_TOKEN_TTL_SECONDS,
    LARK_SCHEMA_VERSION, MAX_TOKEN_TTL_SECONDS, MIN_APP_ID_LENGTH, MIN_APP_SECRET_LENGTH,
    PLATFORM_NAME, PROVIDER_NAME, TOKEN_REFRESH_SKEW_SECS, TYPICAL_APP_SECRET_LENGTH,
};
pub use crate::lark::calendar::{CalendarEvent, CalendarEventQuery, EventStatus, FreeBusySlot};
pub use crate::lark::contact::{Department, User, UserIdType, UserQuery};
pub use crate::lark::doc::{BitableField, BitableMeta, Document, DocumentType, SheetMeta};
pub use crate::lark::error::{
    ErrorClass, LarkError, LarkResult, LARK_ERROR_VARIANT_COUNT, PLATFORM_AUTH_FAILURE_CODES,
    PLATFORM_CODE_RATE_LIMITED,
};
pub use crate::lark::http::{
    ApiEnvelope, ApiRequest, AuthMode, ClientCredentials, HttpMethod, LarkTransport, Page,
    RetryPolicy, TransportConfig,
};
pub use crate::lark::message::{
    CardContent, FileContent, ImageContent, InteractiveContent, Message, MessageType, PostContent,
    PostElement, PostLocale, PostParagraph, ReceiveIdType, TextContent, SUPPORTED_MESSAGE_TYPES,
};
pub use crate::lark::webhook::{
    decrypt_event_payload, encrypt_event_payload, verify_webhook_event as lark_verify_webhook,
    verify_webhook_event_at, EventType as WebhookEventType, WebhookEvent, WebhookVerifyResult,
    WEBHOOK_TIMESTAMP_SKEW_SECS,
};

// 兼容导出: Lark 前缀 alias (跟其它子模块风格一致, 防跟外部命名冲突)
pub use crate::lark::approval::{
    ApprovalFormField as LarkApprovalFormField, ApprovalInstance as LarkApprovalInstance,
    ApprovalTask as LarkApprovalTask, InstanceStatus as LarkInstanceStatus,
    TaskStatus as LarkTaskStatus,
};
pub use crate::lark::auth::{
    AppIdHolder as LarkAppIdHolder, AppSecretHolder as LarkAppSecretHolder,
    TenantAccessToken as LarkTenantAccessToken, UserAccessToken as LarkUserAccessToken,
    WebhookToken as LarkWebhookToken,
};
pub use crate::lark::calendar::{
    CalendarEvent as LarkCalendarEvent, CalendarEventQuery as LarkCalendarEventQuery,
    EventStatus as LarkEventStatus, FreeBusySlot as LarkFreeBusySlot,
};
pub use crate::lark::contact::{
    Department as LarkDepartment, User as LarkUser, UserQuery as LarkUserQuery,
};
pub use crate::lark::doc::{
    Document as LarkDocument, DocumentType as LarkDocumentType, SheetMeta as LarkSheetMeta,
};
pub use crate::lark::error::{
    LarkError as LarkErrorReexport, LarkResult as LarkResultReexport,
    LARK_ERROR_VARIANT_COUNT as LARK_ERROR_COUNT,
};
pub use crate::lark::message::{
    CardContent as LarkCardContent, FileContent as LarkFileContent,
    ImageContent as LarkImageContent, InteractiveContent as LarkInteractiveContent,
    Message as LarkMessage, MessageType as LarkMessageType, PostContent as LarkPostContent,
    PostElement as LarkPostElement, PostLocale as LarkPostLocale,
    PostParagraph as LarkPostParagraph, ReceiveIdType as LarkReceiveIdType,
    TextContent as LarkTextContent,
};
pub use crate::lark::webhook::{
    WebhookEvent as LarkWebhookEvent, WebhookVerifyResult as LarkWebhookVerifyResult,
};

// ============================================================================
// §1 工具白名单 (调用面防御: 8 个协议面入口, 编译期 hardcode)
// ============================================================================

/// 8 个协议面入口的工具白名单 (编译期 hardcode)。
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

/// 白名单长度守门 (8)。
pub const LARK_TOOL_WHITELIST_COUNT: usize = 8;
const _: () = assert!(LARK_TOOL_WHITELIST.len() == LARK_TOOL_WHITELIST_COUNT);

/// 8 核心 API 数守门。
pub const CORE_API_COUNT: usize = 8;
const _: () = assert!(CORE_API_COUNT == LARK_TOOL_WHITELIST_COUNT);

/// 工具调用白名单校验 (不在白名单内 = 拒绝)。
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> LarkResult<()> {
    if !LARK_TOOL_WHITELIST.contains(&tool) {
        return Err(LarkError::Other(format!(
            "tool not whitelisted: {tool} (lark 8 API)"
        )));
    }
    Ok(())
}

// ============================================================================
// §2 编译期常量
// ============================================================================

/// 协议 schema 版本 (跟 [`LARK_SCHEMA_VERSION`] 同步锚点)。
pub const LARK_API_VERSION: &str = LARK_SCHEMA_VERSION;

/// 占位桩守门标志: 真现实现后恒为 `false`。
///
/// 保留常量是为了让调用方有一处稳定的「该子模块是否还在桩模式」查询点;
/// 恒 `false` = 8 API 全部走真实协议逻辑。
pub const STUB_MODE: bool = false;

/// 编译期守门: 真现实现下 STUB_MODE 必须为 false。
const _: () = assert!(
    STUB_MODE == false,
    "lark 子模块已是真现实现, STUB_MODE 必须为 false"
);

/// 查询桩模式状态 (恒 `false`)。
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

/// 6 消息类型守门常量。
pub const MESSAGE_TYPE_COUNT: usize = 6;
const _: () = assert!(MESSAGE_TYPE_COUNT == SUPPORTED_MESSAGE_TYPES.len());

/// 5 鉴权要素守门常量 (App ID / App Secret / tenant token / user token / webhook token)。
pub const AUTH_METHOD_COUNT: usize = 5;
const _: () = assert!(AUTH_METHOD_COUNT == 5);

/// 4 实体守门常量 (Message / CalendarEvent / User / Document)。
pub const ENTITY_COUNT: usize = 4;
const _: () = assert!(ENTITY_COUNT == 4);

/// 6 K-1 强校验守门常量。
pub const K1_STRONG_VALIDATION_COUNT: usize = 6;
const _: () = assert!(K1_STRONG_VALIDATION_COUNT == 6);

/// LarkError variant 守门 (14)。
const _: () = assert!(LARK_ERROR_VARIANT_COUNT == 14);

/// 默认平台开放 API base URL (中性占位, 部署时覆盖)。
pub const DEFAULT_API_BASE: &str = DEFAULT_LARK_API_BASE;

/// 单消息最大文本字节数 (防单消息爆炸)。
pub const MAX_MESSAGE_TEXT_BYTES: usize = 4096;

/// 单次 list_calendar_events 单页最大返回数。
pub const MAX_CALENDAR_EVENTS_PER_PAGE: u32 = 1000;

/// 单 webhook 回调体字节上限 (防超大回调体)。
pub const MAX_WEBHOOK_CHUNK_BYTES: usize = 16 * 1024;

// ============================================================================
// §3 LarkClient trait (8 核心 API)
// ============================================================================

/// lark 客户端 trait (8 核心 API, async)。
///
/// 全部方法走真实协议逻辑 (见 [`LarkClientImpl`] 实现): 字段校验 →
/// 认证/重试/退避的 HTTP 调用 (或本地校验) → 严格响应映射。
#[async_trait]
pub trait LarkClient: Send + Sync {
    /// 1. 发消息 (`POST /im/v1/messages`), 返回颁发的 message_id。
    async fn send_message(&self, message: &Message) -> LarkResult<String>;

    /// 2. 列日历事件 (`GET /calendar/v4/calendars/{id}/events`, 自动翻页合并)。
    async fn list_calendar_events(
        &self,
        query: &CalendarEventQuery,
    ) -> LarkResult<Vec<CalendarEvent>>;

    /// 3. 查用户 (`GET /contact/v3/users/{id}`)。
    async fn get_user(&self, query: &UserQuery) -> LarkResult<User>;

    /// 4. 查部门 (`GET /contact/v3/departments/{id}`)。
    async fn get_department(&self, department_id: &str) -> LarkResult<Department>;

    /// 5. 建 docx 文档 (`POST /docx/v1/documents`)。
    async fn create_doc(&self, doc: &Document) -> LarkResult<Document>;

    /// 6. 建 spreadsheet (`POST /sheets/v3/spreadsheets`)。
    async fn create_sheet(&self, sheet: &Document) -> LarkResult<Document>;

    /// 7. 查审批实例 (`GET /approval/v4/instances/{id}`)。
    async fn get_approval_instance(&self, instance_id: &str) -> LarkResult<ApprovalInstance>;

    /// 8. 校验 webhook (本地: token 恒定时间比较 + 加密解密 + 重放窗口)。
    async fn verify_webhook(
        &self,
        event: &WebhookEvent,
        webhook_token: &WebhookToken,
    ) -> LarkResult<WebhookVerifyResult>;
}

// ============================================================================
// §4 LarkClientImpl (真实现派发器)
// ============================================================================

/// lark 客户端实现。
///
/// - `app_id` / `app_secret`: 走 [`AppIdHolder`] / [`AppSecretHolder`] (内存持有,
///   Debug 脱敏, 0 明文落盘);
/// - `tenant_token`: 走 [`LarkTransport`] 的 token 槽 + [`TokenCache`] 持久化缓存;
/// - `user_token`: 调用方注入的用户级令牌槽 (生命周期管理);
/// - `transport`: 认证头 / 重试 / 限流退避 / Deadline 超时 / 分页基元。
#[derive(Debug)]
pub struct LarkClientImpl {
    /// App ID 持有者。
    app_id: AppIdHolder,
    /// App Secret 持有者。
    app_secret: AppSecretHolder,
    /// user_access_token 槽。
    user_token: std::sync::Mutex<Option<UserAccessToken>>,
    /// HTTP 传输层 (含 tenant token 槽与持久化缓存)。
    transport: LarkTransport,
}

impl LarkClientImpl {
    /// 创建客户端 (默认传输配置; base URL 为中性占位, 部署时用
    /// [`Self::with_config`] 覆盖为组织协作平台实际开放端点)。
    pub fn new() -> Self {
        let transport = LarkTransport::new(TransportConfig::default())
            .expect("default transport config is valid");
        Self {
            app_id: AppIdHolder::empty(),
            app_secret: AppSecretHolder::empty(),
            user_token: std::sync::Mutex::new(None),
            transport,
        }
    }

    /// 用自定义传输配置创建 (base URL / 超时 / 重试 / token 缓存路径)。
    pub fn with_config(config: TransportConfig) -> LarkResult<Self> {
        let transport = LarkTransport::new(config)?;
        Ok(Self {
            app_id: AppIdHolder::empty(),
            app_secret: AppSecretHolder::empty(),
            user_token: std::sync::Mutex::new(None),
            transport,
        })
    }

    /// 从部署方系统凭据库加载凭证。
    ///
    /// 【显式不支持】`Err(LarkError::Unsupported("credential_store"))` ——
    /// 系统凭据库接入依赖部署环境, 客户端库不自带 (理由与注入方式见
    /// [`AppIdHolder::from_credential_store`])。
    pub fn try_from_credential_store() -> LarkResult<Self> {
        Err(LarkError::Unsupported("credential_store"))
    }

    /// 设置 App ID (K-1 #1 强校验)。
    pub fn set_app_id(&mut self, app_id: String) -> LarkResult<()> {
        self.app_id.set(app_id)
    }

    /// 设置 App Secret (K-1 #2 强校验)。
    pub fn set_app_secret(&mut self, app_secret: String) -> LarkResult<()> {
        self.app_secret.set(app_secret)
    }

    /// 读 App ID (cloned)。
    pub fn app_id(&self) -> Option<String> {
        self.app_id.get()
    }

    /// 读 App Secret (cloned; 调用方负责不落日志)。
    pub fn app_secret(&self) -> Option<String> {
        self.app_secret.get()
    }

    /// API base URL。
    pub fn api_base(&self) -> &str {
        &self.transport.config().api_base
    }

    /// 传输配置。
    pub fn transport_config(&self) -> &TransportConfig {
        self.transport.config()
    }

    /// App ID + App Secret 是否都已设置。
    pub fn is_configured(&self) -> bool {
        self.app_id.is_set() && self.app_secret.is_set()
    }

    /// 取调用凭证 (未配置 = 对应 K-1 缺失错误)。
    fn credentials(&self) -> LarkResult<ClientCredentials> {
        let app_id = self.app_id.get().ok_or(LarkError::AppIdMissing)?;
        let app_secret = self.app_secret.get().ok_or(LarkError::AppSecretMissing)?;
        Ok(ClientCredentials { app_id, app_secret })
    }

    /// 设置 tenant_access_token (手动注入, 跳过颁发端点)。
    pub fn set_tenant_token(&self, token: TenantAccessToken) {
        self.transport.set_tenant_token(token);
    }

    /// 读 tenant_access_token (cloned)。
    pub fn tenant_token(&self) -> Option<TenantAccessToken> {
        self.transport.cached_tenant_token()
    }

    /// 设置 user_access_token。
    pub fn set_user_token(&self, token: UserAccessToken) {
        if let Ok(mut guard) = self.user_token.lock() {
            *guard = Some(token);
        }
    }

    /// 读 user_access_token (cloned)。
    pub fn user_token(&self) -> Option<UserAccessToken> {
        self.user_token.lock().ok().and_then(|g| g.clone())
    }

    /// tenant token 是否已过期 (无缓存视为过期)。
    pub fn tenant_token_is_expired(&self) -> bool {
        self.transport
            .cached_tenant_token()
            .map(|t| t.is_expired())
            .unwrap_or(true)
    }

    /// user token 是否已过期 (无缓存视为过期)。
    pub fn user_token_is_expired(&self) -> bool {
        self.user_token
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|t| t.is_expired()))
            .unwrap_or(true)
    }

    /// 客户端状态快照 (桩模式恒 false + 凭证/令牌就位情况)。
    pub fn status(&self) -> ClientStatus {
        ClientStatus {
            stub_mode: STUB_MODE,
            platform: PLATFORM_NAME.to_string(),
            api_base: self.api_base().to_string(),
            schema_version: LARK_SCHEMA_VERSION.to_string(),
            app_id_set: self.app_id.is_set(),
            app_secret_set: self.app_secret.is_set(),
            tenant_token_set: self.tenant_token().is_some(),
            user_token_set: self.user_token().is_some(),
            tenant_token_expired: self.tenant_token_is_expired(),
            user_token_expired: self.user_token_is_expired(),
            configured: self.is_configured(),
        }
    }
}

impl Default for LarkClientImpl {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// §5 LarkClient trait impl (8 API 真实现)
// ============================================================================

#[async_trait]
impl LarkClient for LarkClientImpl {
    async fn send_message(&self, message: &Message) -> LarkResult<String> {
        let request = message::build_send_request(message)?;
        let creds = self.credentials()?;
        let data: message::SendMessageData = self
            .transport
            .execute(&creds, &request, "im.message.create")
            .await?;
        message::message_id_of(&data)
    }

    async fn list_calendar_events(
        &self,
        query: &CalendarEventQuery,
    ) -> LarkResult<Vec<CalendarEvent>> {
        query.validate()?;
        let creds = self.credentials()?;
        let mut items: Vec<CalendarEvent> = Vec::new();
        let mut page_token: Option<String> = query.page_token.clone();
        for _ in 0..calendar::MAX_EVENT_PAGES {
            let request = calendar::build_list_request(query, page_token.as_deref())?;
            let page: Page<calendar::CalendarEventWire> = self
                .transport
                .execute_page(&creds, &request, "calendar.event.list")
                .await?;
            for wire in &page.items {
                items.push(calendar::map_event(&query.calendar_id, wire)?);
            }
            if !page.has_more {
                return Ok(items);
            }
            match page.page_token {
                Some(next) if !next.is_empty() && Some(&next) != page_token.as_ref() => {
                    page_token = Some(next);
                }
                Some(_) => {
                    return Err(LarkError::Other(
                        "pagination loop detected: page_token did not advance".to_string(),
                    ));
                }
                None => {
                    return Err(LarkError::Other(
                        "pagination state inconsistent: has_more without page_token".to_string(),
                    ));
                }
            }
        }
        Err(LarkError::Other(format!(
            "pagination exceeded {} pages",
            calendar::MAX_EVENT_PAGES
        )))
    }

    async fn get_user(&self, query: &UserQuery) -> LarkResult<User> {
        let request = contact::build_user_request(query);
        let creds = self.credentials()?;
        let data: contact::UserData = self
            .transport
            .execute(&creds, &request, "contact.user.get")
            .await?;
        contact::map_user(&data.user)
    }

    async fn get_department(&self, department_id: &str) -> LarkResult<Department> {
        let request = contact::build_department_request(department_id)?;
        let creds = self.credentials()?;
        let data: contact::DepartmentData = self
            .transport
            .execute(&creds, &request, "contact.department.get")
            .await?;
        contact::map_department(&data.department)
    }

    async fn create_doc(&self, doc: &Document) -> LarkResult<Document> {
        let request = doc::build_create_doc_request(doc)?;
        let creds = self.credentials()?;
        let data: doc::CreateDocData = self
            .transport
            .execute(&creds, &request, "docx.document.create")
            .await?;
        doc::map_created_doc(doc, &data.document)
    }

    async fn create_sheet(&self, sheet: &Document) -> LarkResult<Document> {
        let request = doc::build_create_sheet_request(sheet)?;
        let creds = self.credentials()?;
        let data: doc::CreateSheetData = self
            .transport
            .execute(&creds, &request, "sheets.spreadsheet.create")
            .await?;
        doc::map_created_sheet(sheet, &data.spreadsheet)
    }

    async fn get_approval_instance(&self, instance_id: &str) -> LarkResult<ApprovalInstance> {
        let request = approval::build_instance_request(instance_id)?;
        let creds = self.credentials()?;
        let data: approval::ApprovalInstanceData = self
            .transport
            .execute(&creds, &request, "approval.instance.get")
            .await?;
        approval::map_instance(&data.instance)
    }

    async fn verify_webhook(
        &self,
        event: &WebhookEvent,
        webhook_token: &WebhookToken,
    ) -> LarkResult<WebhookVerifyResult> {
        webhook::verify_webhook_event(event, webhook_token)
    }
}

// ============================================================================
// §6 ClientStatus (状态快照)
// ============================================================================

/// 客户端状态快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientStatus {
    /// 桩模式 (真现实现恒 false)。
    pub stub_mode: bool,
    /// 平台名。
    pub platform: String,
    /// API base URL。
    pub api_base: String,
    /// schema 版本。
    pub schema_version: String,
    /// App ID 是否已设置。
    pub app_id_set: bool,
    /// App Secret 是否已设置。
    pub app_secret_set: bool,
    /// tenant_access_token 是否已设置。
    pub tenant_token_set: bool,
    /// user_access_token 是否已设置。
    pub user_token_set: bool,
    /// tenant token 是否过期。
    pub tenant_token_expired: bool,
    /// user token 是否过期。
    pub user_token_expired: bool,
    /// 客户端是否已配置 (App ID + Secret 都设置)。
    pub configured: bool,
}

// ============================================================================
// §7 单元测试 (常量守门 + 真派发行为: 未配置凭证的确定性错误面)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lark::message::{ReceiveIdType, TextContent};

    #[test]
    fn k1_stub_mode_is_false_after_real_implementation() {
        assert!(!STUB_MODE);
        assert!(!is_stub_mode());
    }

    #[test]
    fn k1_platform_constants() {
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(PROVIDER_NAME, "lark");
        assert_eq!(LARK_SCHEMA_VERSION, "1");
        assert_eq!(LARK_API_VERSION, LARK_SCHEMA_VERSION);
        assert!(DEFAULT_API_BASE.starts_with("https://"));
    }

    #[test]
    fn k1_tool_whitelist_count_8() {
        assert_eq!(LARK_TOOL_WHITELIST_COUNT, 8);
        assert_eq!(CORE_API_COUNT, 8);
        assert_eq!(LARK_TOOL_WHITELIST.len(), LARK_TOOL_WHITELIST_COUNT);
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
                "whitelist must contain {tool}"
            );
        }
    }

    #[test]
    fn k1_validate_tool_call_whitelisted() {
        let args = serde_json::json!({});
        assert!(validate_tool_call("apeireth_sdk_lark_send_message", &args).is_ok());
        assert!(matches!(
            validate_tool_call("apeireth_sdk_lark_bogus", &args),
            Err(LarkError::Other(_))
        ));
    }

    #[test]
    fn k1_count_constants() {
        assert_eq!(MESSAGE_TYPE_COUNT, 6);
        assert_eq!(SUPPORTED_MESSAGE_TYPES.len(), 6);
        assert_eq!(MessageType::COUNT, 6);
        assert_eq!(AUTH_METHOD_COUNT, 5);
        assert_eq!(ENTITY_COUNT, 4);
        assert_eq!(K1_STRONG_VALIDATION_COUNT, 6);
        assert_eq!(LARK_ERROR_VARIANT_COUNT, 14);
    }

    #[test]
    fn client_construction_and_credentials_gate() {
        let client = LarkClientImpl::new();
        assert!(!client.is_configured());
        assert!(client.app_id().is_none());
        assert!(client.app_secret().is_none());
        assert_eq!(client.api_base(), DEFAULT_API_BASE);
        // 未配置凭证 → 确定性 K-1 缺失错误 (不打网络)
        assert!(matches!(client.credentials(), Err(LarkError::AppIdMissing)));

        let mut client = LarkClientImpl::new();
        client
            .set_app_id("cli_a1b2c3d4e5f6".to_string())
            .expect("valid app id");
        assert!(matches!(
            client.credentials(),
            Err(LarkError::AppSecretMissing)
        ));
        client
            .set_app_secret("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid app secret");
        assert!(client.is_configured());
        assert!(client.credentials().is_ok());
    }

    #[test]
    fn client_rejects_invalid_app_id_and_secret() {
        let mut client = LarkClientImpl::new();
        assert!(matches!(
            client.set_app_id("invalid".to_string()),
            Err(LarkError::AppIdInvalid(_))
        ));
        assert!(matches!(
            client.set_app_secret("short".to_string()),
            Err(LarkError::AppSecretInvalid(_))
        ));
    }

    #[test]
    fn credential_store_load_is_explicitly_unsupported() {
        match LarkClientImpl::try_from_credential_store() {
            Err(LarkError::Unsupported("credential_store")) => {}
            other => panic!("expected Unsupported(credential_store), got {other:?}"),
        }
    }

    #[test]
    fn client_status_snapshot() {
        let mut client = LarkClientImpl::new();
        client
            .set_app_id("cli_a1b2c3d4e5f6".to_string())
            .expect("valid");
        client
            .set_app_secret("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid");
        let status = client.status();
        assert!(!status.stub_mode);
        assert_eq!(status.platform, "apeireth");
        assert!(status.app_id_set);
        assert!(status.app_secret_set);
        assert!(status.configured);
        assert!(!status.tenant_token_set);
        assert!(!status.user_token_set);
        assert!(status.tenant_token_expired);
        assert!(status.user_token_expired);
    }

    /// 8 API 真派发: 请求字段校验/凭证校验在任何网络调用前完成 (确定性错误面)。
    #[tokio::test]
    async fn k1_8_core_apis_dispatch_without_network() {
        let client = LarkClientImpl::new();

        // 1. send_message: 合法消息但未配置凭证 → AppIdMissing (不打网络)
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        )
        .expect("valid");
        assert!(matches!(
            client.send_message(&msg).await,
            Err(LarkError::AppIdMissing)
        ));

        // 2. list_calendar_events
        let start = chrono::Utc::now();
        let query = CalendarEventQuery {
            calendar_id: "cal_xxx".to_string(),
            start_time: start,
            end_time: start + chrono::Duration::hours(1),
            page_size: 50,
            page_token: None,
        };
        assert!(matches!(
            client.list_calendar_events(&query).await,
            Err(LarkError::AppIdMissing)
        ));

        // 3. get_user
        let query = UserQuery::new("ou_user1234567890abcdef".to_string(), UserIdType::OpenId)
            .expect("valid");
        assert!(matches!(
            client.get_user(&query).await,
            Err(LarkError::AppIdMissing)
        ));

        // 4. get_department
        assert!(matches!(
            client.get_department("od_dept123").await,
            Err(LarkError::AppIdMissing)
        ));

        // 5. create_doc
        let doc = Document::new_docx("title".to_string(), None).expect("valid");
        assert!(matches!(
            client.create_doc(&doc).await,
            Err(LarkError::AppIdMissing)
        ));

        // 6. create_sheet
        let sheet = Document::new_sheet("title".to_string(), None).expect("valid");
        assert!(matches!(
            client.create_sheet(&sheet).await,
            Err(LarkError::AppIdMissing)
        ));

        // 7. get_approval_instance
        assert!(matches!(
            client.get_approval_instance("instance_001").await,
            Err(LarkError::AppIdMissing)
        ));

        // 8. verify_webhook (本地校验, 不需凭证): challenge 回显成功
        let wh_token = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
            .expect("valid");
        let event = WebhookEvent::from_raw_json(
            r#"{"type":"url_verification","challenge":"challenge-1","token":"token_xxx"}"#,
        )
        .expect("parse");
        match client.verify_webhook(&event, &wh_token).await {
            Ok(WebhookVerifyResult::Challenge(c)) => assert_eq!(c, "challenge-1"),
            other => panic!("expected Challenge, got {other:?}"),
        }

        // 字段校验错误先于凭证校验: 非法 chat_id 的消息直接被 K-1 拒
        let bad = Message::text(
            "invalid".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        );
        assert!(matches!(bad, Err(LarkError::ChatIdInvalid(_))));
    }

    /// 5 鉴权要素覆盖 (App ID / App Secret / tenant token / user token / webhook token)。
    #[test]
    fn k1_5_auth_elements_cover() {
        let mut id_holder = AppIdHolder::empty();
        id_holder
            .set("cli_a1b2c3d4e5f6".to_string())
            .expect("valid");
        assert!(id_holder.is_set());
        let mut secret_holder = AppSecretHolder::empty();
        secret_holder
            .set("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid");
        assert!(secret_holder.is_set());
        let t =
            TenantAccessToken::new("cli_a1b2c3d4e5f6".to_string(), "t-abc123".to_string(), 7200)
                .expect("valid");
        assert!(!t.is_expired());
        let u = UserAccessToken::new(
            "cli_a1b2c3d4e5f6".to_string(),
            "u-abc".to_string(),
            "ur-xyz".to_string(),
            "ou_user1234567890abcdef".to_string(),
            7200,
        )
        .expect("valid");
        assert!(!u.is_expired());
        let wh = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
            .expect("valid");
        assert!(wh.verify("token_xxx"));
    }

    /// 4 实体覆盖 (Message / CalendarEvent / User / Document)。
    #[test]
    fn k1_4_entities_cover() {
        let msg = Message::text(
            "oc_a1b2c3d4e5f6".to_string(),
            ReceiveIdType::ChatId,
            "Hello".to_string(),
        )
        .expect("valid");
        assert_eq!(msg.msg_type, MessageType::Text);
        let _text = TextContent::new("Hello");

        let start = chrono::Utc::now();
        let event =
            CalendarEvent::new("cal_xxx", "会议", start, start + chrono::Duration::hours(1))
                .expect("valid");
        assert_eq!(event.summary, "会议");

        let user =
            User::new("ou_user1234567890abcdef".to_string(), "Alice".to_string()).expect("valid");
        assert_eq!(user.name, "Alice");

        let doc = Document::new_docx("title".to_string(), None).expect("valid");
        assert_eq!(doc.doc_type, DocumentType::Doc);
    }
}
