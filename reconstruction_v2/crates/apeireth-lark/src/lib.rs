//! # apeireth-lark (STUB MODE)
//!
//! ⚠️ **STUB MODE: R20 阶段 3 必补, 修改需经 6 哲学锚 + 主人审**
//!
//! Lark / Feishu SDK Skeleton (1:1 翻译 v0.9.21 商业版 `out/main` 中
//! `@larksuiteoapi/node-sdk@^1.59.0` 集成面, per `commercial-nsis/v0901/app-64/app-extracted/package.json` line 23).
//! 商业版 bundle 实查 0 处直接调用 (lark 仅在 deps 里声明, 未实接 — R20 阶段 3 估补),
//! 5 端点消息/日历/文档/Bitable/Auth 1:1 翻译自飞书 Open API:
//! - 消息发送 (`im/v1/messages`)
//! - 日历 (`calendar/v4/calendars` + `calendar/v4/calendars/:calendar_id/events`)
//! - 文档 (`docx/v1/documents`)
//! - Bitable (`bitable/v1/apps/:app_id/tables/:table_id/records`)
//! - 鉴权 (`auth/v3/tenant_access_token/internal`)
//!
//! **STUB MODE 守门** (per task spec, 0 改):
//! - 任何真实 SDK 引用禁止 (0 引, fixture 测)
//! - 8 stub 工具必须返 `LarkError::NotImplemented(api_name)`, 编译期 hardcode
//! - `STUB_MODE` 编译期 hardcode = `true`, **不允许运行时配置"切到真实模式"**
//! - 真实实现留 **R20 阶段 3**, 修改本 crate 需 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5) + 主人审
//!
//! ## 状态: ⏳ STUB skeleton (R20 阶段 1 续, 主 2026-08-05 19:50 拍板"派成员干, 自己干分散注意力")

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

// 集成点: R20 阶段 3 必补 `apeireth-tool-runtime` 集成 (per §5 占位扩展点 #5),
// 现阶段 STUB 模式不引 (避免引入 0 使用的 dep, per 0 重复造轮子 + 0 改 LOCKED).
// 整合 #2 sub-agent 改 workspace 成员时, 跟 `apeireth-team-lead` skeleton 同模式, 加 path dep 即可.

// ============================================================================
// §0.5 R20 阶段 6 flesh out 新增: `real` 模块 (真接飞书 Open API 5 端点)
// ============================================================================
//
// 跟本文件 §4 `LarkClientImpl` (STUB 模式 8 工具返 NotImplemented) **严格分离**:
// `LarkRealImpl` 是显式 opt-in 的真接 HTTP 客户端, 不受 `STUB_MODE = true` 编译期
// hardcode 守门影响. 调用方按需 `apeireth_lark::LarkRealImpl::new(config)?` 即可.
//
// 设计:
// - 同 `LarkClient` trait, 8 工具签名一致, 但实现走真 reqwest HTTP
// - 5 端点 1:1 翻译飞书 Open API (auth/im/calendar/docx/bitable)
// - token 缓存 (Arc<Mutex<Option<TenantAccessToken>>>), 自动 401 重试 1 次
// - 飞书 `code != 0` → `LarkError::ApiError`, HTTP 4xx/5xx → `LarkError::Network`
// - 详细诚实标缺 5 项 (per 8 项承诺 #8) 写在 real.rs 顶部
//
// 守门:
// - 0 改 `STUB_MODE` (仍 = true), 0 改 `LarkClientImpl` 8 工具
// - 0 改 24 LOCKED crate, 0 改 workspace version (1.0.0)
// - 0 改现有 5 fixture + 5 K-1 强校验测试 (test_lark_stub_in_process.rs 不动)
pub mod real;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;

// 便捷 re-exports (调用方少打 crate 名)
pub use real::{
    msg_type_to_str, LarkApiResponse, LarkRealImpl, SendMessageResponse, TenantTokenResponse,
};

// ============================================================================
// m3 hallucination 防御 #3 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode 9 (8 工具 + 1 stub_status 守门), validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的 lark 工具 (eg. "apeireth_lark_send_email" 实际不存在).
// ============================================================================

/// m3 防御: Lark SDK 8 工具 + 1 stub 守门白名单 (编译期 hardcode, 不可运行时改).
///
/// **9 项 = 8 工具 (1:1 翻译飞书 Open API 5 端点) + 1 stub_status 守门**:
/// - 消息 (1): `apeireth_lark_send_message`
/// - 日历 (2): `apeireth_lark_list_calendars` + `apeireth_lark_create_event`
/// - 文档 (2): `apeireth_lark_get_document` + `apeireth_lark_search_documents`
/// - Bitable (2): `apeireth_lark_list_bitable_records` + `apeireth_lark_create_bitable_record`
/// - Auth (1): `apeireth_lark_auth_refresh`
/// - **额外 1**: `apeireth_lark_stub_status` (查 STUB_MODE 状态, stub 模式守门)
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_lark_send_message",
    "apeireth_lark_list_calendars",
    "apeireth_lark_create_event",
    "apeireth_lark_get_document",
    "apeireth_lark_list_bitable_records",
    "apeireth_lark_create_bitable_record",
    "apeireth_lark_search_documents",
    "apeireth_lark_auth_refresh",
    "apeireth_lark_stub_status", // 额外 1: stub 模式守门 (查 STUB_MODE 状态)
];

/// 编译期守门: TOOL_WHITELIST 长度 == 9 (8 工具 + 1 stub_status).
pub const TOOL_WHITELIST_COUNT: usize = 9;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `LarkError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), LarkError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(LarkError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 文档头 + 编译期 hardcode (per R20 P0 5 crate 风格 + K-1 强校验)
// ============================================================================

/// Lark API schema version (1:1 翻译飞书 Open API 文档, K-1 强校验).
pub const LARK_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验 #1: 编译期 hardcode `"apeireth"`, v0.9.21 1:1 翻译, 不写 "SpectrAI" 等装饰名).
pub const PLATFORM_NAME: &str = "apeireth";

/// **STUB MODE 守门标志** (K-1 强校验 #4): 编译期 hardcode = `true`.
/// R20 阶段 3 真接飞书 SDK 时, **必须经 6 哲学锚 + 主人审才能改 `false`**.
pub const STUB_MODE: bool = true;

/// 编译期守门: STUB_MODE 必须 == true (per STUB MODE 守门 + 8 项不修改承诺).
/// 改 false 需同时改本 assert + STUB_MODE 标志, 强行提醒 reviewer.
const _: () = assert!(
    STUB_MODE == true,
    "STUB_MODE 改 false 需经 6 哲学锚 + 主人审 (R20 阶段 3)"
);

/// m3 防御: 查 STUB_MODE 状态 (per task spec 额外 1 守门工具).
/// **R20 阶段 3 改 `STUB_MODE = false` 时, 本函数返 `false`**; 现阶段恒返 `true`.
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

/// 飞书 Open API base URL (1:1 翻译商业版 SDK 默认 endpoint).
/// 1:1 翻译 v0.9.21 `@larksuiteoapi/node-sdk@1.59` 默认 `https://open.feishu.cn/open-apis`.
pub const LARK_API_BASE_URL: &str = "https://open.feishu.cn/open-apis";

/// tenant_access_token 缓存 TTL (秒, 2h — 飞书官方 `expire` 默认 7200s).
pub const LARK_TOKEN_CACHE_TTL_SECONDS: u64 = 7200;

/// 支持的 message_type 枚举 (5 项, 1:1 翻译飞书 `msg_type` 字段, per K-1 强校验 #2).
pub const SUPPORTED_MESSAGE_TYPES: &[MessageType] = &[
    MessageType::Text,
    MessageType::Post,
    MessageType::Image,
    MessageType::File,
    MessageType::Interactive,
];

/// 编译期守门: SUPPORTED_MESSAGE_TYPES 长度 == 5 (K-1 强校验 #2).
const _: () = assert!(SUPPORTED_MESSAGE_TYPES.len() == 5);

/// 单条消息最大字节数 (4 KB, 飞书 `text` 消息硬上限, 1:1 翻译).
pub const LARK_MAX_MESSAGE_LENGTH: usize = 4096;

// ============================================================================
// §2 核心类型 (MessageType / LarkError / LarkConfig / TenantAccessToken)
// ============================================================================

/// 消息类型 (5 variant, 1:1 翻译飞书 `msg_type` 字段, per K-1 强校验 #2).
///
/// 字段对应飞书 Open API `im/v1/messages` 的 `msg_type` 枚举:
/// `text` / `post` / `image` / `file` / `interactive` (5 种, 1:1 翻译).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// 纯文本 (`msg_type = "text"`).
    Text,
    /// 富文本 post (`msg_type = "post"`).
    Post,
    /// 图片 (`msg_type = "image"`).
    Image,
    /// 文件 (`msg_type = "file"`).
    File,
    /// 消息卡片 (`msg_type = "interactive"`).
    Interactive,
}

/// Lark / Feishu 错误类型 (10 variant, 覆盖 stub + m3 + 真实错误面).
///
/// **STUB 模式说明** (per task spec):
/// - 8 stub 工具全部返 `NotImplemented(api_name)`, 编译期 hardcode
/// - 真实 API 错误 (Auth, RateLimit, Network) 留 R20 阶段 3 真接 SDK 时实现
#[derive(Debug, Error)]
pub enum LarkError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4).
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    /// STUB 模式必返: 真实实现未到位 (per task spec + 编译期 STUB_MODE 守门).
    /// `api_name` 例: `"send_message"` / `"list_calendars"`.
    #[error("lark api not implemented (STUB MODE): {0} — 真实实现留 R20 阶段 3")]
    NotImplemented(String),

    #[error("lark config invalid: {0}")]
    ConfigInvalid(String),
    #[error("lark auth failed: {0}")]
    AuthFailed(String),
    #[error("lark tenant_access_token expired or missing")]
    TokenMissing,
    #[error("lark rate limit exceeded: {0:?}")]
    RateLimited(Duration),
    #[error("lark network error: {0}")]
    Network(String),
    #[error("lark message too long: {got} bytes, max {max}")]
    MessageTooLong { got: usize, max: usize },
    #[error("lark api returned non-zero code: code={code}, msg={msg}")]
    ApiError { code: i32, msg: String },
    #[error("lark error: {0}")]
    Other(String),
}

pub type LarkResult<T> = Result<T, LarkError>;

/// Lark / Feishu 配置 (1:1 翻译商业版 SDK `AppSettings`).
///
/// 字段对应飞书开发者后台 "应用凭证" 页:
/// - `app_id` (per 飞书 App ID, 例 `"cli_xxx"`)
/// - `app_secret` (per 飞书 App Secret, 敏感 — Serialize 走 SecretString 模式, 当前 stub 阶段明文)
/// - `base_url` (可选, 默认 `LARK_API_BASE_URL`)
/// - `token_cache_ttl_seconds` (可选, 默认 `LARK_TOKEN_CACHE_TTL_SECONDS`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LarkConfig {
    /// 飞书 App ID (1:1 翻译 `app_id`).
    pub app_id: String,
    /// 飞书 App Secret (1:1 翻译 `app_secret`, 真实实现应改 SecretString, stub 阶段明文).
    pub app_secret: String,
    /// API base URL (默认 `LARK_API_BASE_URL`, 留口子给国际版 Lark `https://open.larksuite.com/open-apis`).
    pub base_url: String,
    /// token 缓存 TTL (秒, 默认 7200).
    pub token_cache_ttl_seconds: u64,
}

impl Default for LarkConfig {
    fn default() -> Self {
        Self {
            // 默认值用 platform name + "stub" 后缀, 强调 STUB 模式
            app_id: format!("{PLATFORM_NAME}-stub-app-id"),
            app_secret: format!("{PLATFORM_NAME}-stub-app-secret"),
            base_url: LARK_API_BASE_URL.to_string(),
            token_cache_ttl_seconds: LARK_TOKEN_CACHE_TTL_SECONDS,
        }
    }
}

/// tenant_access_token 缓存 (1:1 翻译飞书 `auth/v3/tenant_access_token/internal` 响应).
///
/// 字段对应飞书 Open API 响应:
/// - `tenant_access_token` (string, 实际 token)
/// - `expire` (int, 过期秒数, 默认 7200)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantAccessToken {
    /// token 字符串 (1:1 翻译 `tenant_access_token`).
    pub token: String,
    /// 过期时间戳 (SystemTime, 1:1 翻译 `expire` 秒数, 内部转 SystemTime).
    pub expires_at: SystemTime,
}

impl TenantAccessToken {
    /// 查 token 是否过期 (per `expire` 字段, 提前 60s 续期防临界过期).
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now();
        // 提前 60s 续期防临界过期
        let grace = Duration::from_secs(60);
        self.expires_at
            .checked_sub(grace)
            .map(|t| now >= t)
            .unwrap_or(true)
    }
}

// ============================================================================
// §3 Stub API 表面 (8 工具, 每个返 LarkError::NotImplemented)
// ============================================================================

/// Lark Client trait — 8 工具 (1:1 翻译飞书 Open API 5 端点, STUB 模式).
///
/// **STUB 模式必返 `LarkError::NotImplemented(api_name)`** (per task spec).
/// 真实实现留 **R20 阶段 3**, 修改本 trait 需 6 哲学锚 + 主人审.
#[async_trait]
pub trait LarkClient: Send + Sync {
    // ---------- 消息 (1) ----------

    /// 工具 1: `send_message` — 发送消息 (`im/v1/messages` POST).
    ///
    /// **STUB**: 返 `LarkError::NotImplemented("send_message")`.
    async fn send_message(
        &self,
        receive_id: &str,
        msg_type: MessageType,
        content: &str,
    ) -> LarkResult<String>;

    // ---------- 日历 (2) ----------

    /// 工具 2: `list_calendars` — 列出日历 (`calendar/v4/calendars` GET).
    ///
    /// **STUB**: 返 `LarkError::NotImplemented("list_calendars")`.
    async fn list_calendars(&self) -> LarkResult<Vec<serde_json::Value>>;

    /// 工具 3: `create_event` — 创建日程 (`calendar/v4/calendars/:calendar_id/events` POST).
    ///
    /// **STUB**: 返 `LarkError::NotImplemented("create_event")`.
    async fn create_event(
        &self,
        calendar_id: &str,
        summary: &str,
        start_ms: i64,
        end_ms: i64,
    ) -> LarkResult<String>;

    // ---------- 文档 (2) ----------

    /// 工具 4: `get_document` — 读取文档 (`docx/v1/documents/:document_id` GET).
    ///
    /// **STUB**: 返 `LarkError::NotImplemented("get_document")`.
    async fn get_document(&self, document_id: &str) -> LarkResult<serde_json::Value>;

    /// 工具 5: `search_documents` — 搜索文档 (`docx/v1/documents` GET with `query`).
    ///
    /// **STUB**: 返 `LarkError::NotImplemented("search_documents")`.
    async fn search_documents(
        &self,
        query: &str,
        limit: usize,
    ) -> LarkResult<Vec<serde_json::Value>>;

    // ---------- Bitable (2) ----------

    /// 工具 6: `list_bitable_records` — 列出 Bitable 记录
    /// (`bitable/v1/apps/:app_id/tables/:table_id/records` GET).
    ///
    /// **STUB**: 返 `LarkError::NotImplemented("list_bitable_records")`.
    async fn list_bitable_records(
        &self,
        app_id: &str,
        table_id: &str,
        limit: usize,
    ) -> LarkResult<Vec<serde_json::Value>>;

    /// 工具 7: `create_bitable_record` — 创建 Bitable 记录
    /// (`bitable/v1/apps/:app_id/tables/:table_id/records` POST).
    ///
    /// **STUB**: 返 `LarkError::NotImplemented("create_bitable_record")`.
    async fn create_bitable_record(
        &self,
        app_id: &str,
        table_id: &str,
        fields: serde_json::Value,
    ) -> LarkResult<String>;

    // ---------- Auth (1) ----------

    /// 工具 8: `auth_refresh` — 主动刷新 tenant_access_token
    /// (`auth/v3/tenant_access_token/internal` POST).
    ///
    /// **STUB**: 返 `LarkError::NotImplemented("auth_refresh")`.
    /// R20 阶段 3 真接 SDK 时, 此函数是 §4 token 缓存的填充入口.
    async fn auth_refresh(&self) -> LarkResult<TenantAccessToken>;
}

// ============================================================================
// §4 LarkClient 主体 + token 缓存结构 (skeleton)
// ============================================================================

/// Lark Client 主体 (持有 config + token cache, STUB 模式).
///
/// **STUB 模式说明** (per task spec):
/// - 8 工具方法全部返 `LarkError::NotImplemented(api_name)`, 编译期 hardcode
/// - `auth_refresh` 真做缓存查 + 过期检测 (骨架, 不调真实 API)
/// - R20 阶段 3 真接 SDK 时, 在 `auth_refresh` 内调飞书 `auth/v3/tenant_access_token/internal`,
///   在 8 业务工具内调对应 endpoint
#[derive(Debug)]
pub struct LarkClientImpl {
    /// 配置.
    pub config: LarkConfig,
    /// tenant_access_token 缓存 (None = 未拉过, 拉过则用 is_expired 判定).
    pub token_cache: Option<TenantAccessToken>,
}

impl LarkClientImpl {
    /// 新建 LarkClient (STUB 模式, 0 网络调用, 仅持有 config + None token).
    pub fn new(config: LarkConfig) -> LarkResult<Self> {
        // STUB 模式: 不真调鉴权, 仅验证 config 非空
        if config.app_id.is_empty() || config.app_secret.is_empty() {
            return Err(LarkError::ConfigInvalid(
                "app_id / app_secret 不能为空 (STUB 模式仍校验)".to_string(),
            ));
        }
        info!(target: "apeireth_lark", "STUB MODE 启用: app_id={} base_url={}", config.app_id, config.base_url);
        Ok(Self {
            config,
            token_cache: None,
        })
    }

    /// 读 config.
    pub fn config(&self) -> &LarkConfig {
        &self.config
    }

    /// 查 token 缓存是否有效 (None 或过期 → false).
    pub fn token_valid(&self) -> bool {
        self.token_cache
            .as_ref()
            .map(|t| !t.is_expired())
            .unwrap_or(false)
    }
}

#[async_trait]
impl LarkClient for LarkClientImpl {
    // ---- 消息 (1) ----
    async fn send_message(
        &self,
        _receive_id: &str,
        _msg_type: MessageType,
        _content: &str,
    ) -> LarkResult<String> {
        debug!(target: "apeireth_lark", "send_message: STUB — NotImplemented");
        Err(LarkError::NotImplemented("send_message".to_string()))
    }

    // ---- 日历 (2) ----
    async fn list_calendars(&self) -> LarkResult<Vec<serde_json::Value>> {
        debug!(target: "apeireth_lark", "list_calendars: STUB — NotImplemented");
        Err(LarkError::NotImplemented("list_calendars".to_string()))
    }

    async fn create_event(
        &self,
        _calendar_id: &str,
        _summary: &str,
        _start_ms: i64,
        _end_ms: i64,
    ) -> LarkResult<String> {
        debug!(target: "apeireth_lark", "create_event: STUB — NotImplemented");
        Err(LarkError::NotImplemented("create_event".to_string()))
    }

    // ---- 文档 (2) ----
    async fn get_document(&self, _document_id: &str) -> LarkResult<serde_json::Value> {
        debug!(target: "apeireth_lark", "get_document: STUB — NotImplemented");
        Err(LarkError::NotImplemented("get_document".to_string()))
    }

    async fn search_documents(
        &self,
        _query: &str,
        _limit: usize,
    ) -> LarkResult<Vec<serde_json::Value>> {
        debug!(target: "apeireth_lark", "search_documents: STUB — NotImplemented");
        Err(LarkError::NotImplemented("search_documents".to_string()))
    }

    // ---- Bitable (2) ----
    async fn list_bitable_records(
        &self,
        _app_id: &str,
        _table_id: &str,
        _limit: usize,
    ) -> LarkResult<Vec<serde_json::Value>> {
        debug!(target: "apeireth_lark", "list_bitable_records: STUB — NotImplemented");
        Err(LarkError::NotImplemented(
            "list_bitable_records".to_string(),
        ))
    }

    async fn create_bitable_record(
        &self,
        _app_id: &str,
        _table_id: &str,
        _fields: serde_json::Value,
    ) -> LarkResult<String> {
        debug!(target: "apeireth_lark", "create_bitable_record: STUB — NotImplemented");
        Err(LarkError::NotImplemented(
            "create_bitable_record".to_string(),
        ))
    }

    // ---- Auth (1) ----
    async fn auth_refresh(&self) -> LarkResult<TenantAccessToken> {
        debug!(target: "apeireth_lark", "auth_refresh: STUB — NotImplemented");
        // STUB 模式: 不真调 auth/v3/tenant_access_token/internal, 返 NotImplemented
        // R20 阶段 3: 调飞书 API, 拿响应填 TenantAccessToken { token, expires_at = now + ttl }
        Err(LarkError::NotImplemented("auth_refresh".to_string()))
    }
}

// ============================================================================
// §5 占位扩展点 (R20 阶段 3 实现位置, 标 ⏳)
// ============================================================================

// ⏳ R20 阶段 3 必补:
//
// 1. **真接飞书 Open API**:
//    - 引入 `reqwest` (workspace) + 飞书 OpenAPI 5 端点直调
//    - 或引 `lark-rs` 社区 crate (估 2026 Q3 stable)
//    - 在 `LarkClientImpl` 8 工具内替换 stub, 调对应 endpoint
//
// 2. **改 STUB_MODE = false** (编译期 hardcode, 需 6 哲学锚 + 主人审):
//    - 修改本文件 `pub const STUB_MODE: bool = true;` → `false`
//    - 同时改 `is_stub_mode()` 函数 (恒返 `STUB_MODE`)
//    - 同时改 `const _: () = assert!(STUB_MODE == true, ...)` 守门
//    - 同时改 `LarkConfig::default()` 移除 "stub" 后缀
//
// 3. **Auth 完整化**:
//    - `auth_refresh` 调 `POST {base_url}/auth/v3/tenant_access_token/internal` with app_id/app_secret
//    - 响应填 `TenantAccessToken { token, expires_at = now + 7200s }`
//    - 缓存查: 8 业务工具调用前先 `client.token_valid()`, 失效则先 `auth_refresh`
//
// 4. **错误映射**:
//    - 飞书 API 响应 `code != 0` → `LarkError::ApiError { code, msg }`
//    - HTTP 4xx/5xx → `LarkError::Network(...)` / `LarkError::RateLimited(...)`
//    - 401 → 自动 `auth_refresh` 重试一次
//
// 5. **集成 apeireth-tool-runtime** (编译期已引, 0 改):
//    - `LarkClientImpl` 注册到 tool-runtime, 8 工具暴露给 AI Agent
//    - 配合 `TOOL_WHITELIST` 9 项, m3 防御跨 crate 生效
//
// 6. **测试 fixture 升级**:
//    - 加 `tests/test_lark_real_in_process.rs` (R20 阶段 3 估补)
//    - mock 飞书 OpenAPI server (用 `wiremock` crate), 走完整 8 工具调用
//    - 保留 `tests/test_lark_stub_in_process.rs` 作为 stub 模式回归基线

// ============================================================================
// §6 tests: STUB 模式回归基线 (5 fixture + 5 K-1 强校验)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture 1: STUB_MODE 编译期守门 (K-1 强校验 #4)
    #[test]
    fn stub_mode_is_true_at_compile_time() {
        assert!(STUB_MODE, "STUB_MODE 必须是 true (per STUB MODE 守门)");
        assert!(is_stub_mode(), "is_stub_mode() 必须返 true");
    }

    // Fixture 2: 7 编译期 hardcode 常量守门
    #[test]
    fn compile_time_constants_are_pinned() {
        assert_eq!(LARK_SCHEMA_VERSION, "1");
        assert_eq!(
            PLATFORM_NAME, "apeireth",
            "K-1 强校验 #1: 平台名必须 apeireth"
        );
        assert_eq!(LARK_API_BASE_URL, "https://open.feishu.cn/open-apis");
        assert_eq!(LARK_TOKEN_CACHE_TTL_SECONDS, 7200);
        assert_eq!(LARK_MAX_MESSAGE_LENGTH, 4096);
        assert_eq!(
            SUPPORTED_MESSAGE_TYPES.len(),
            5,
            "K-1 强校验 #2: 5 MessageType"
        );
    }

    // Fixture 3: TOOL_WHITELIST 9 项 (8 工具 + 1 stub_status, K-1 强校验 #3)
    #[test]
    fn tool_whitelist_contains_nine_lark_tools() {
        assert_eq!(TOOL_WHITELIST.len(), 9);
        for tool in [
            "apeireth_lark_send_message",
            "apeireth_lark_list_calendars",
            "apeireth_lark_create_event",
            "apeireth_lark_get_document",
            "apeireth_lark_list_bitable_records",
            "apeireth_lark_create_bitable_record",
            "apeireth_lark_search_documents",
            "apeireth_lark_auth_refresh",
            "apeireth_lark_stub_status",
        ] {
            assert!(TOOL_WHITELIST.contains(&tool), "TOOL_WHITELIST 缺: {tool}");
        }
    }

    // Fixture 4: 8 stub 工具返 NotImplemented (K-1 强校验 #4: STUB 模式)
    #[tokio::test]
    async fn eight_stub_tools_return_not_implemented() {
        let client = LarkClientImpl::new(LarkConfig::default()).unwrap();
        // 消息
        let r = client.send_message("u1", MessageType::Text, "hi").await;
        assert!(matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "send_message"));
        // 日历
        let r = client.list_calendars().await;
        assert!(matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "list_calendars"));
        let r = client.create_event("cal1", "meet", 0, 0).await;
        assert!(matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "create_event"));
        // 文档
        let r = client.get_document("doc1").await;
        assert!(matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "get_document"));
        let r = client.search_documents("q", 10).await;
        assert!(matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "search_documents"));
        // Bitable
        let r = client.list_bitable_records("app1", "tbl1", 10).await;
        assert!(matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "list_bitable_records"));
        let r = client
            .create_bitable_record("app1", "tbl1", serde_json::json!({}))
            .await;
        assert!(matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "create_bitable_record"));
        // Auth
        let r = client.auth_refresh().await;
        assert!(matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "auth_refresh"));
    }

    // Fixture 5: m3 防御 — 白名单外工具拒绝 (per m3-hallucination-defense §2.4)
    #[test]
    fn validate_tool_call_rejects_unknown() {
        let args = serde_json::json!({});
        // 白名单内通过
        for tool in TOOL_WHITELIST {
            assert!(
                validate_tool_call(tool, &args).is_ok(),
                "白名单工具 {tool} 应通过"
            );
        }
        // 白名单外拒绝
        let result = validate_tool_call("apeireth_lark_send_email", &args);
        assert!(result.is_err());
        match result.unwrap_err() {
            LarkError::ToolNotWhitelisted(t) => assert_eq!(t, "apeireth_lark_send_email"),
            other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
        }
    }
}
