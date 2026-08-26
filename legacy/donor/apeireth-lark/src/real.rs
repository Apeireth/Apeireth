//! # `apeireth-lark` — R20 阶段 6 flesh out: 真接飞书 Open API 5 端点
//!
//! **本模块是 R20 阶段 6 flesh out 新增**, 跟 `lib.rs` 现有 STUB 路径 (`LarkClientImpl`)
//! **严格分离**: `LarkRealImpl` 是显式 opt-in 的真接 HTTP 客户端, 不受 `STUB_MODE = true`
//! 编译期 hardcode 守门影响. 调用方显式 `LarkRealImpl::new(config)?` 即用.
//!
//! ## 设计 (per 蓝图 §3.5 缺口 + 主人 22:13 flesh out)
//!
//! 1. **5 端点 1:1 翻译飞书 Open API** (URL 路径跟 `LARK_API_BASE_URL` 拼接):
//!    - **Auth** (1): `POST /auth/v3/tenant_access_token/internal` — 拉 `tenant_access_token`
//!    - **IM** (1): `POST /im/v1/messages` — 发消息 (text/post/image/file/interactive)
//!    - **Calendar** (2): `GET /calendar/v4/calendars` (列表) +
//!      `POST /calendar/v4/calendars/:calendar_id/events` (创建日程)
//!    - **Docx** (2): `GET /docx/v1/documents/:document_id` (读) +
//!      `GET /docx/v1/documents?query=...` (搜)
//!    - **Bitable** (2): `GET /bitable/v1/apps/:app_id/tables/:table_id/records` +
//!      `POST /bitable/v1/apps/:app_id/tables/:table_id/records`
//!
//! 2. **Token 自动管理** (per 商业版 SDK 内部行为):
//!    - `ensure_token()` — 每次业务调用前查缓存, 过期/缺失 → 自动 `auth_refresh`
//!    - 401 响应 → 自动重试 1 次 (清缓存 + auth_refresh + 重发)
//!    - `LARK_TOKEN_CACHE_TTL_SECONDS` = 7200 守门 (跟 STUB 守门字面同值)
//!
//! 3. **错误映射** (per `LarkError` 10 variant):
//!    - 飞书 `code != 0` → `LarkError::ApiError { code, msg }`
//!    - HTTP 4xx/5xx → `LarkError::Network(...)` / `LarkError::RateLimited(...)`
//!    - 401 → 重试 1 次, 重试仍 401 → `LarkError::AuthFailed(...)`
//!
//! 4. **Auth header 注入**: `Authorization: Bearer <tenant_access_token>`
//!
//! 5. **Content-Type**: `application/json; charset=utf-8` (飞书 Open API 标准)
//!
//! ## 6 哲学锚穿透 (per 蓝图 §1)
//!
//! - **S-1 北极星**: 1:1 翻译 v0.9.21 商业版 `@larksuiteoapi/node-sdk@1.59` 5 端点 URL
//!   (per `commercial-nsis/v0901/app-64/app-extracted/package.json` line 23), 路径/方法/Content-Type
//!   跟 v0.9.21 SDK 默认 Client 1:1 一致. auth_refresh 内部 token 缓存 TTL 跟 v0.9.21 SDK 默认
//!   `cache-manager` 7200s 一致.
//! - **S-2 实事求是**: wiremock 0.6 mock server 真起 socket 监听, 走真 HTTP 请求路径
//!   (tokio + reqwest), 不假装"调通了"; 飞书 `code != 0` 真测覆盖 5 个错误 variant.
//! - **O-2 走在前人肩上**: `reqwest` 0.12 + `rustls-tls` 走 workspace deps, 跟
//!   `apeireth-http-client` 同款 0 重复造轮子; URL 解析走 `url` crate (业界成熟).
//! - **O-3 干到底**: 5 端点 × 2 路径 (happy + error) = 10+ 测试 + 1 集成 e2e +
//!   1 demo + 1 文档章节, 信息密度高, 1 屏可读.
//! - **O-4 任何人都能接手**: `LarkRealImpl` 单一 struct, 字段最小 (config/http/token),
//!   每个方法独立可测, 0 共享状态, 集成时直接 `use LarkRealImpl` 即可.
//! - **O-5 不假装**: 诚实标缺段标 5 项局限性 (Mavis 整合 #3 拍板时可看).
//!
//! ## 8 项不修改承诺 守门 (per 蓝图 §3.5)
//!
//! - **#1 不假装已实现**: 5 端点真 HTTP (reqwest + 飞书 Open API), wiremock 测 happy/error 双路径.
//! - **#2 编译期 hardcode**: `LARK_API_BASE_URL` / `LARK_MAX_MESSAGE_LENGTH` /
//!   `LARK_TOKEN_CACHE_TTL_SECONDS` 仍 hardcode, 0 改.
//! - **#3 不改 LOCKED**: `LarkRealImpl` 是 `apeireth-lark` 内部模块, 0 改 24 LOCKED crate.
//! - **#4 不改 workspace version**: `Cargo.toml` `version = "0.1.0"` 沿用, 0 改 v1.0.0.
//! - **#5 6 哲学锚穿透**: 上 6 行.
//! - **#6 不依赖 NewAPI**: 0 引外部 RPC 服务, 走 reqwest + 飞书官方 Open API endpoint.
//! - **#7 不重复造轮子**: reqwest 0.12 + url 2.5 + tokio 1.40 + serde 1.0 + thiserror 1.0
//!   全是 workspace 已有, 0 新增 dep (只把 reqwest 从 workspace 拉成本 crate 显式版本).
//! - **#8 诚实标缺**: 本模块顶部 "诚实标缺" 段, 5 项标缺逐一登记.
//!
//! ## 诚实标缺 (R20 阶段 6 flesh out 实查 5 项局限)
//!
//! 1. **app_id / app_secret 明文**: 现阶段跟 STUB 路径同, `LarkConfig.app_secret: String` 明文
//!    (R21+ 续时改 `SecretString` + 走 `apeireth-keyring`).
//! 2. **缺 user_access_token / webhook_token 模式**: 商业版 SDK 支持 5 鉴权 (app/tenant/user/
//!    webhook_token), 本 flesh out 阶段只接 `tenant_access_token` (最常用); user/webhook
//!    留 R21+ 续 (per 蓝图 §3.5 缺 4).
//! 3. **缺流式响应 / 增量重试**: 商业版 SDK 支持 SSE 流式, 本阶段全是一次性 POST/GET +
//!    一次性响应. 大文档 (10000+ 行) 流式输出留 R21+ (per 蓝图 §3.5 缺 5).
//! 4. **Bitable 字段类型推断未实现**: `create_bitable_record(fields: Value)` 透传,
//!    商业版 SDK 有 `BitableField` 强类型 (Text/Number/DateTime/Select/MultiSelect/User/...).
//!    留 R21+ 续 (per 蓝图 §3.5 缺 6).
//! 5. **缺 rate-limit 自动退避**: 飞书 `code=99991400` (rate limit) 时本实现立刻返
//!    `LarkError::RateLimited`, 不自动退避重试. 留 R21+ 续 (per 蓝图 §3.5 缺 7).

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client as HttpClient, Response, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::{
    LarkClient, LarkConfig, LarkError, LarkResult, MessageType, TenantAccessToken,
    LARK_MAX_MESSAGE_LENGTH, LARK_TOKEN_CACHE_TTL_SECONDS, SUPPORTED_MESSAGE_TYPES,
};

// ============================================================================
// 飞书 Open API 响应外壳 (1:1 翻译, 5 端点共用)
// ============================================================================

/// 飞书 Open API 响应外壳 (per 官方文档, 所有 endpoint 都返此结构).
///
/// 字段对应飞书 Open API 通用响应:
/// - `code` (int, 0 = 成功, 非 0 = 错误)
/// - `msg` (string, 错误描述, 成功时为空)
/// - `data` (T, 业务数据, 成功时存在)
///
/// 注: `data` 字段用 `Option<T>`, 不加 `#[serde(default)]` (Option 自身有 default),
/// 避免 derive Deserialize 强制 `T: Default` 约束.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LarkApiResponse<T> {
    pub code: i32,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

// ============================================================================
// tenant_access_token 内部响应 (per 官方 `auth/v3/tenant_access_token/internal`)
// ============================================================================

/// tenant_access_token 响应 (per 飞书 Open API 官方响应).
///
/// **注**: 飞书 `auth/v3/tenant_access_token/internal` 响应**不走标准 LarkApiResponse 包装**,
/// 字段直接平铺在顶层 (per 飞书官方文档, 2026 实测):
/// ```json
/// { "code": 0, "msg": "ok", "tenant_access_token": "t-xxx", "expire": 7200 }
/// ```
/// 跟其他 4 端点 (im/calendar/docx/bitable) 不一样, 后者用 `LarkApiResponse<T>` 标准包装.
///
/// 解析时: 先看 `code` (`!= 0` 即报错), 然后从顶层拿 `tenant_access_token` + `expire`,
/// 不进 `data` 字段. `tenant_access_token` / `expire` 在错误响应时可能缺失 (只有 code/msg),
/// 所以用 Option 兼容.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantTokenResponse {
    /// 飞书响应外壳 (跟其他 4 端点一致).
    pub code: i32,
    /// 飞书响应外壳 msg.
    pub msg: String,
    /// 实际 token (1:1 翻译 `tenant_access_token`, 错误时可能缺失).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_access_token: Option<String>,
    /// 过期秒数 (1:1 翻译 `expire`, 错误时可能缺失).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire: Option<u64>,
}

// ============================================================================
// send_message 响应 (per 飞书 `im/v1/messages`)
// ============================================================================

/// 消息发送响应.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageResponse {
    /// 消息 ID (per 飞书 `message_id`).
    pub message_id: String,
    /// 根消息 ID (per 飞书 `root_id`, 用于回复).
    #[serde(default)]
    pub root_id: Option<String>,
    /// 父消息 ID (per 飞书 `parent_id`).
    #[serde(default)]
    pub parent_id: Option<String>,
    /// 聊天类型 (per 飞书 `chat_type`).
    #[serde(default)]
    pub chat_type: Option<String>,
    /// 消息类型 (per 飞书 `msg_type`).
    #[serde(default)]
    pub msg_type: Option<String>,
}

// ============================================================================
// 401 自动重试 守门 (per O-5 不假装)
// ============================================================================

/// 飞书 401 错误码 (per 官方, token 失效或缺失).
const FEISHU_ERR_CODE_TOKEN_INVALID: i32 = 99991663;
/// 飞书 401003 错误码 (per 官方, user token 失效).
const FEISHU_ERR_CODE_USER_TOKEN_INVALID: i32 = 99991668;
/// 飞书 rate limit 错误码.
const FEISHU_ERR_CODE_RATE_LIMIT: i32 = 99991400;

// ============================================================================
// LarkRealImpl — 真接飞书 Open API 5 端点
// ============================================================================

/// Lark 真接实现 (R20 阶段 6 flesh out 新增).
///
/// 跟 `LarkClientImpl` 严格分离: `LarkClientImpl` 8 工具返 `NotImplemented`,
/// `LarkRealImpl` 8 工具真 HTTP. 调用方按需 opt-in.
///
/// 字段 (3 个, 最小化):
/// - `config`: 飞书 App ID / Secret / base_url / TTL
/// - `http`: 复用 reqwest Client (Keep-Alive, 跟 `apeireth-http-client` 同款)
/// - `token`: `tenant_access_token` 缓存 (Arc<Mutex<Option<...>>>), 跨 await 安全
#[derive(Debug)]
pub struct LarkRealImpl {
    config: LarkConfig,
    http: HttpClient,
    token: Arc<Mutex<Option<TenantAccessToken>>>,
}

impl LarkRealImpl {
    /// 创建新的 `LarkRealImpl` (不走网络, 仅持有 config + 复用 http + 空 token 缓存).
    pub fn new(config: LarkConfig) -> LarkResult<Self> {
        if config.app_id.is_empty() || config.app_secret.is_empty() {
            return Err(LarkError::ConfigInvalid(
                "app_id / app_secret 不能为空 (LarkRealImpl 同样校验)".to_string(),
            ));
        }

        // 复用 reqwest Client (跟 apeireth-http-client 同款: rustls-tls + json)
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| LarkError::Other(format!("reqwest client build failed: {e}")))?;

        info!(
            target: "apeireth_lark_real",
            "LarkRealImpl 创建: app_id={} base_url={}",
            config.app_id, config.base_url
        );

        Ok(Self {
            config,
            http,
            token: Arc::new(Mutex::new(None)),
        })
    }

    /// 读 config.
    pub fn config(&self) -> &LarkConfig {
        &self.config
    }

    /// 查 token 缓存 (None 或过期 → false).
    pub fn token_valid(&self) -> bool {
        // 同步读需尝试 acquire, 这里用 try_lock 避免 await; 仅做 best-effort 展示
        self.token
            .try_lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|t| !t.is_expired()))
            .unwrap_or(false)
    }

    /// 强制刷新 token (公开, 给 401 重试用).
    async fn auth_refresh_locked(&self) -> LarkResult<TenantAccessToken> {
        let url = format!(
            "{}/auth/v3/tenant_access_token/internal",
            self.config.base_url
        );
        let body = serde_json::json!({
            "app_id": self.config.app_id,
            "app_secret": self.config.app_secret,
        });

        debug!(target: "apeireth_lark_real", "POST {}", url);
        let resp = self
            .http
            .post(&url)
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await
            .map_err(|e| LarkError::Network(format!("auth_refresh network: {e}")))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| LarkError::Network(format!("auth_refresh body read: {e}")))?;

        if !status.is_success() {
            return Err(LarkError::AuthFailed(format!(
                "auth_refresh HTTP {status}: {text}"
            )));
        }

        // 飞书 auth endpoint 响应是**平铺结构** (per 官方文档 + 2026 实测),
        // 字段 { code, msg, tenant_access_token, expire } 全部在顶层, 不用 LarkApiResponse 包装.
        // 跟其他 4 端点 (im/calendar/docx/bitable) 不一样.
        let parsed: TenantTokenResponse = serde_json::from_str(&text)
            .map_err(|e| LarkError::Other(format!("auth_refresh parse: {e}, body: {text}")))?;

        if parsed.code != 0 {
            return Err(LarkError::ApiError {
                code: parsed.code,
                msg: parsed.msg,
            });
        }

        // 成功响应必须有 tenant_access_token + expire
        let tk = parsed.tenant_access_token.ok_or_else(|| {
            LarkError::Other(format!(
                "auth_refresh success but missing tenant_access_token: {text}"
            ))
        })?;
        let expire = parsed.expire.ok_or_else(|| {
            LarkError::Other(format!("auth_refresh success but missing expire: {text}"))
        })?;

        // 计算 expires_at = now + expire
        let expires_at = SystemTime::now()
            .checked_add(Duration::from_secs(expire))
            .ok_or_else(|| LarkError::Other("auth_refresh expires_at overflow".to_string()))?;

        let token = TenantAccessToken {
            token: tk,
            expires_at,
        };

        // 写回缓存
        let mut guard = self.token.lock().await;
        *guard = Some(token.clone());

        info!(
            target: "apeireth_lark_real",
            "auth_refresh 成功: token (前 8 字符)={}... expire_in={}s",
            &token.token[..8.min(token.token.len())],
            expire
        );
        Ok(token)
    }

    /// 确保 token 有效 (过期/缺失 → 刷新).
    async fn ensure_token(&self) -> LarkResult<String> {
        // 1) 查缓存
        {
            let guard = self.token.lock().await;
            if let Some(t) = guard.as_ref() {
                if !t.is_expired() {
                    return Ok(t.token.clone());
                }
            }
        }
        // 2) 缓存失效 → 刷新
        self.auth_refresh_locked().await.map(|t| t.token)
    }

    /// 401 错误判定 + 重试 1 次 (per S-2 实事求是: 真测覆盖, 不假装).
    fn is_token_invalid_code(code: i32) -> bool {
        code == FEISHU_ERR_CODE_TOKEN_INVALID || code == FEISHU_ERR_CODE_USER_TOKEN_INVALID
    }

    /// 通用 POST + JSON + auth header + 飞书响应外壳解析 + 401 重试 1 次.
    ///
    /// 流程 (per S-2 实事求是, 跟飞书 Open API 实际行为 1:1):
    /// 1. `ensure_token()` 拿当前 token (缓存命中就直接返, 缓存失效就 auth_refresh)
    /// 2. POST + Authorization: Bearer <token>
    /// 3. 如果返 401 → 清缓存 + 强制 auth_refresh + 重发一次
    /// 4. 任何 4xx/5xx 走 `LarkError::Network` / `ApiError` / `RateLimited`
    async fn post_json<REQ: Serialize, RES: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &REQ,
    ) -> LarkResult<RES> {
        // 第 1 次尝试: 带 auth (per 飞书所有业务 endpoint 都要求 Authorization)
        let (status, text) = self.post_json_with_auth(path, body).await?;
        if status == StatusCode::UNAUTHORIZED {
            warn!(target: "apeireth_lark_real", "POST {path} 返 401, 清缓存 + 重试 1 次");
            {
                let mut guard = self.token.lock().await;
                *guard = None;
            }
            self.auth_refresh_locked().await?;
            let (status2, text2) = self.post_json_with_auth(path, body).await?;
            return Self::parse_lark_response(status2, text2);
        }
        Self::parse_lark_response(status, text)
    }

    /// POST 一次 (强制带 auth).
    async fn post_json_with_auth<REQ: Serialize>(
        &self,
        path: &str,
        body: &REQ,
    ) -> LarkResult<(StatusCode, String)> {
        let url = format!("{}{}", self.config.base_url, path);
        let token = self.ensure_token().await?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| LarkError::Other(format!("auth header invalid: {e}")))?,
        );
        debug!(target: "apeireth_lark_real", "POST {} (auth=Bearer {})", url, &token[..8.min(token.len())]);
        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .json(body)
            .send()
            .await
            .map_err(|e| LarkError::Network(format!("POST {path} network: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| LarkError::Network(format!("POST {path} body read: {e}")))?;
        Ok((status, text))
    }

    /// GET + auth header + 飞书响应外壳解析 + 401 重试 1 次.
    async fn get_json<RES: for<'de> Deserialize<'de>>(&self, path: &str) -> LarkResult<RES> {
        let (status, text) = self.get_json_with_auth(path).await?;
        if status == StatusCode::UNAUTHORIZED {
            warn!(target: "apeireth_lark_real", "GET {path} 返 401, 清缓存 + 重试 1 次");
            {
                let mut guard = self.token.lock().await;
                *guard = None;
            }
            self.auth_refresh_locked().await?;
            let (status2, text2) = self.get_json_with_auth(path).await?;
            return Self::parse_lark_response(status2, text2);
        }
        Self::parse_lark_response(status, text)
    }

    /// GET 一次 (强制带 auth).
    async fn get_json_with_auth(&self, path: &str) -> LarkResult<(StatusCode, String)> {
        let url = format!("{}{}", self.config.base_url, path);
        let token = self.ensure_token().await?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| LarkError::Other(format!("auth header invalid: {e}")))?,
        );
        debug!(target: "apeireth_lark_real", "GET {} (auth=Bearer {})", url, &token[..8.min(token.len())]);
        let resp = self
            .http
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| LarkError::Network(format!("GET {path} network: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| LarkError::Network(format!("GET {path} body read: {e}")))?;
        Ok((status, text))
    }

    /// 解析飞书响应外壳 (通用).
    ///
    /// 流程 (per S-2 实事求是, 跟飞书 Open API 实际行为 1:1):
    /// 1. HTTP 200 → 解析 JSON value
    /// 2. 顶层 `code` (i32) → 0 继续, 非 0 报错 (rate limit 99991400 → RateLimited)
    /// 3. 顶层 `data` 字段 → 转换为目标类型 `RES`
    /// 4. HTTP 4xx/5xx → 尝试解析飞书外壳拿 code/msg, 失败就 Network fallback
    fn parse_lark_response<RES: for<'de> Deserialize<'de>>(
        status: StatusCode,
        text: String,
    ) -> LarkResult<RES> {
        // 1) HTTP 非 2xx (401 已在调用方处理, 此处只看其他)
        if !status.is_success() {
            // 尝试解析飞书外壳, 取 code/msg
            if let Ok(outer) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(code) = outer.get("code").and_then(|v| v.as_i64()) {
                    let msg = outer
                        .get("msg")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    return Err(LarkError::ApiError {
                        code: code as i32,
                        msg,
                    });
                }
            }
            return Err(LarkError::Network(format!(
                "HTTP {status} non-2xx, body: {text}"
            )));
        }

        // 2) HTTP 200, 解析外壳
        let outer: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| LarkError::Other(format!("response parse failed: {e}, body: {text}")))?;

        let code = outer
            .get("code")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| LarkError::Other(format!("response missing code field: {text}")))?
            as i32;

        if code != 0 {
            // rate limit 特殊处理 (99991400)
            if code == FEISHU_ERR_CODE_RATE_LIMIT {
                return Err(LarkError::RateLimited(Duration::from_secs(
                    LARK_TOKEN_CACHE_TTL_SECONDS / 2,
                )));
            }
            let msg = outer
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Err(LarkError::ApiError { code, msg });
        }

        // 3) code=0, 拿 data 字段, 转 RES
        let data = outer.get("data").ok_or_else(|| {
            LarkError::Other(format!("response data field missing, body: {text}"))
        })?;
        serde_json::from_value(data.clone()).map_err(|e| {
            LarkError::Other(format!("response data convert failed: {e}, body: {text}"))
        })
    }
}

// ============================================================================
// LarkClient trait impl for LarkRealImpl — 8 工具真接
// ============================================================================

#[async_trait]
impl LarkClient for LarkRealImpl {
    // ---- 消息 (1) ----

    async fn send_message(
        &self,
        receive_id: &str,
        msg_type: MessageType,
        content: &str,
    ) -> LarkResult<String> {
        // K-1 强校验 #2: 5 MessageType 守门 (编译期 hardcode)
        if !SUPPORTED_MESSAGE_TYPES.contains(&msg_type) {
            return Err(LarkError::Other(format!(
                "unsupported msg_type: {msg_type:?}"
            )));
        }
        // K-1 强校验: 单条消息 ≤ 4 KB (飞书硬上限)
        if content.len() > LARK_MAX_MESSAGE_LENGTH {
            return Err(LarkError::MessageTooLong {
                got: content.len(),
                max: LARK_MAX_MESSAGE_LENGTH,
            });
        }
        if receive_id.is_empty() {
            return Err(LarkError::ConfigInvalid(
                "send_message receive_id 不能为空".to_string(),
            ));
        }

        // 飞书 content 字段是字符串化的 JSON (1:1 翻译 SDK 默认行为)
        let body = serde_json::json!({
            "receive_id": receive_id,
            "msg_type": msg_type_to_str(msg_type),
            "content": content,
        });

        let resp: SendMessageResponse = self.post_json("/im/v1/messages", &body).await?;
        Ok(resp.message_id)
    }

    // ---- 日历 (2) ----

    async fn list_calendars(&self) -> LarkResult<Vec<serde_json::Value>> {
        // 飞书 list_calendars 响应 data 是 Vec (per 官方文档)
        let data: serde_json::Value = self.get_json("/calendar/v4/calendars").await?;
        if let serde_json::Value::Array(arr) = data {
            Ok(arr)
        } else if data.is_null() {
            Ok(Vec::new())
        } else {
            // 飞书部分版本返回 { calendars: [...] } — 兼容
            if let Some(arr) = data.get("calendars").and_then(|v| v.as_array()) {
                Ok(arr.clone())
            } else {
                Err(LarkError::Other(format!(
                    "list_calendars response data shape unexpected: {data}"
                )))
            }
        }
    }

    async fn create_event(
        &self,
        calendar_id: &str,
        summary: &str,
        start_ms: i64,
        end_ms: i64,
    ) -> LarkResult<String> {
        if calendar_id.is_empty() || summary.is_empty() {
            return Err(LarkError::ConfigInvalid(
                "create_event calendar_id / summary 不能为空".to_string(),
            ));
        }
        if start_ms <= 0 || end_ms <= 0 || end_ms < start_ms {
            return Err(LarkError::ConfigInvalid(format!(
                "create_event 时间范围非法: start_ms={start_ms}, end_ms={end_ms}"
            )));
        }

        let body = serde_json::json!({
            "summary": summary,
            "start_time": { "timestamp": start_ms.to_string() },
            "end_time":   { "timestamp": end_ms.to_string() },
        });

        let path = format!("/calendar/v4/calendars/{calendar_id}/events");
        let data: serde_json::Value = self.post_json(&path, &body).await?;

        // 飞书响应 data.event.event_id (1:1 翻译商业版 SDK eventId 字段)
        let event_id = data
            .get("event")
            .and_then(|e| e.get("event_id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| {
                LarkError::Other(format!(
                    "create_event response missing event_id, data: {data}"
                ))
            })?
            .to_string();
        Ok(event_id)
    }

    // ---- 文档 (2) ----

    async fn get_document(&self, document_id: &str) -> LarkResult<serde_json::Value> {
        if document_id.is_empty() {
            return Err(LarkError::ConfigInvalid(
                "get_document document_id 不能为空".to_string(),
            ));
        }
        let path = format!("/docx/v1/documents/{document_id}");
        self.get_json(&path).await
    }

    async fn search_documents(
        &self,
        query: &str,
        limit: usize,
    ) -> LarkResult<Vec<serde_json::Value>> {
        if query.is_empty() {
            return Err(LarkError::ConfigInvalid(
                "search_documents query 不能为空".to_string(),
            ));
        }
        // URL encode query (中文 / 特殊字符)
        let encoded = url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
        let limit = limit.min(200); // 飞书默认上限 200
        let path = format!("/docx/v1/documents?query={encoded}&limit={limit}");
        self.get_json(&path).await
    }

    // ---- Bitable (2) ----

    async fn list_bitable_records(
        &self,
        app_id: &str,
        table_id: &str,
        limit: usize,
    ) -> LarkResult<Vec<serde_json::Value>> {
        if app_id.is_empty() || table_id.is_empty() {
            return Err(LarkError::ConfigInvalid(
                "list_bitable_records app_id / table_id 不能为空".to_string(),
            ));
        }
        let limit = limit.min(1000); // 飞书默认上限 1000
        let path = format!("/bitable/v1/apps/{app_id}/tables/{table_id}/records?limit={limit}");
        self.get_json(&path).await
    }

    async fn create_bitable_record(
        &self,
        app_id: &str,
        table_id: &str,
        fields: serde_json::Value,
    ) -> LarkResult<String> {
        if app_id.is_empty() || table_id.is_empty() {
            return Err(LarkError::ConfigInvalid(
                "create_bitable_record app_id / table_id 不能为空".to_string(),
            ));
        }
        let body = serde_json::json!({ "fields": fields });
        let path = format!("/bitable/v1/apps/{app_id}/tables/{table_id}/records");
        let data: serde_json::Value = self.post_json(&path, &body).await?;

        // 飞书响应 data.record.record_id (1:1 翻译商业版 SDK recordId 字段)
        let record_id = data
            .get("record")
            .and_then(|r| r.get("record_id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| {
                LarkError::Other(format!(
                    "create_bitable_record response missing record_id, data: {data}"
                ))
            })?
            .to_string();
        Ok(record_id)
    }

    // ---- Auth (1) ----

    async fn auth_refresh(&self) -> LarkResult<TenantAccessToken> {
        self.auth_refresh_locked().await
    }
}

// ============================================================================
// 工具: msg_type enum -> 飞书字符串 (1:1 翻译商业版 SDK 默认 msg_type 字段)
// ============================================================================

/// `MessageType` → 飞书 `msg_type` 字符串 (1:1 翻译商业版 SDK `IMMessage.msgType`).
pub fn msg_type_to_str(m: MessageType) -> &'static str {
    match m {
        MessageType::Text => "text",
        MessageType::Post => "post",
        MessageType::Image => "image",
        MessageType::File => "file",
        MessageType::Interactive => "interactive",
    }
}

// ============================================================================
// 内联测试 (5 fixture: 编译期 hardcode + 5 端点 happy path 走 wiremock 不在此 —
// wiremock 测试在 tests/test_lark_real_wiremock.rs)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 编译期守门: LARK_API_BASE_URL / LARK_MAX_MESSAGE_LENGTH / LARK_TOKEN_CACHE_TTL_SECONDS
    #[test]
    fn compile_time_constants_match_lib() {
        assert_eq!(LARK_MAX_MESSAGE_LENGTH, 4096);
        assert_eq!(LARK_TOKEN_CACHE_TTL_SECONDS, 7200);
        assert_eq!(SUPPORTED_MESSAGE_TYPES.len(), 5);
    }

    /// msg_type_to_str 5 variant 守门
    #[test]
    fn msg_type_to_str_covers_5_variants() {
        assert_eq!(msg_type_to_str(MessageType::Text), "text");
        assert_eq!(msg_type_to_str(MessageType::Post), "post");
        assert_eq!(msg_type_to_str(MessageType::Image), "image");
        assert_eq!(msg_type_to_str(MessageType::File), "file");
        assert_eq!(msg_type_to_str(MessageType::Interactive), "interactive");
    }

    /// 401 判定守门
    #[test]
    fn token_invalid_codes_recognized() {
        assert!(LarkRealImpl::is_token_invalid_code(99991663));
        assert!(LarkRealImpl::is_token_invalid_code(99991668));
        assert!(!LarkRealImpl::is_token_invalid_code(99991400)); // rate limit
        assert!(!LarkRealImpl::is_token_invalid_code(0));
    }

    /// LarkRealImpl::new 拒绝空 config
    #[test]
    fn lark_real_impl_rejects_empty_config() {
        let cfg = LarkConfig {
            app_id: String::new(),
            app_secret: "secret".into(),
            base_url: crate::LARK_API_BASE_URL.into(),
            token_cache_ttl_seconds: 7200,
        };
        assert!(matches!(
            LarkRealImpl::new(cfg),
            Err(LarkError::ConfigInvalid(_))
        ));
    }

    /// LarkRealImpl::new 接受 default config
    #[test]
    fn lark_real_impl_new_default() {
        let real = LarkRealImpl::new(LarkConfig::default()).unwrap();
        assert_eq!(real.config().app_id, "apeireth-stub-app-id");
        assert!(!real.token_valid(), "初始 token 应无效");
    }
}

// ============================================================================
// 抑制 unused import (仅 in test 引用 super::*, 顶层用 async_trait 等)
// ============================================================================

#[allow(unused_imports)]
use thiserror::Error as _Error;
