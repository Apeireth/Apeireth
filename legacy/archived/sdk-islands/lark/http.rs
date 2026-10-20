//! # lark HTTP 传输层 (认证头 / 重试 / 限流退避 / Deadline 超时 / 分页)
//!
//! 本模块是 `lark` 全部协议面共用的传输基座:
//!
//! 1. **认证头**: `Authorization: Bearer <tenant_access_token>`。token 缺失 /
//!    临期自动补 (内存缓存 → 磁盘缓存 [`TokenCache`] → 平台颁发端点);
//!    API 返回认证失败时**失效缓存并重新颁发一次**再重试 (闭合一次, 不无限循环)。
//! 2. **重试 + 限流退避**: [`RetryPolicy`] 闭合策略 (指数退避 + 尊重服务端
//!    `Retry-After`, 统一钳制到 `max_backoff`); 只对分类为
//!    `ErrorClass::Retryable` 的错误重试, 永久 / 认证失败不盲重试。
//! 3. **超时**: 整次调用用 [`apeireth_core::deadline::Deadline`] 限定总预算
//!    (含所有重试); 单次尝试预算 = `min(attempt_timeout, deadline 剩余)`。
//! 4. **响应体契约** (wire contract, 严格形状 + 未知字段容错):
//!    - 业务响应信封: `{"code": <i32 必填>, "msg": <string 可选>, "data": <object 可选>}`
//!    - `code != 0` → [`LarkError::from_platform_code`] 闭合映射
//!      (限流码 → `RateLimited`, 认证码 → `TokenExpired`, 其它 → `ApiError`)
//!    - 信封内 `data` 的具体形状由各协议面的 wire DTO 定义:
//!      **必填字段缺失/类型错 = 永久错误**; **未知字段一律忽略** (向前兼容)。
//! 5. **分页**: [`Page<T>`] = `{"items": [...], "has_more": bool, "page_token": "..."}`
//!    列表端点统一用它翻页。
//! 6. **脱敏日志**: 只记录 method / path / attempt / status / 业务码, 0 记录
//!    请求体 / 响应体 / token / App Secret。
//!
//! token 颁发端点 (无认证头): `POST /auth/v3/tenant_access_token/internal`,
//! body `{"app_id": ..., "app_secret": ...}`,
//! 响应 `{"code": 0, "msg": "ok", "tenant_access_token": "t-...", "expire": 7200}`。

use std::path::PathBuf;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::lark::auth::{TenantAccessToken, TokenCache, DEFAULT_LARK_API_BASE};
use crate::lark::error::{ErrorClass, LarkError, LarkResult};

// ============================================================================
// §1 常量
// ============================================================================

/// 认证头名 (HTTP/1.1 标准)。
pub const AUTH_HEADER_NAME: &str = "Authorization";

/// 认证方案 (Bearer)。
pub const BEARER_SCHEME: &str = "Bearer";

/// tenant token 颁发端点 (相对 `api_base`)。
pub const TENANT_TOKEN_PATH: &str = "/auth/v3/tenant_access_token/internal";

// ============================================================================
// §2 RetryPolicy (重试 + 限流退避, 闭合策略)
// ============================================================================

/// 重试 + 退避策略 (闭合: 只对 `ErrorClass::Retryable` 生效)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    /// 单次调用最大尝试次数 (≥ 1; 1 = 不重试)。
    pub max_attempts: u32,
    /// 首次退避时长。
    pub initial_backoff: Duration,
    /// 退避/Retry-After 统一上限。
    pub max_backoff: Duration,
    /// 退避倍率 (每次 ×multiplier)。
    pub backoff_multiplier: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(2),
            backoff_multiplier: 2,
        }
    }
}

impl RetryPolicy {
    /// 不重试策略 (测试 / 显式单发)。
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            ..Self::default()
        }
    }

    /// 第 `attempt` 次尝试失败后的等待时长 (attempt 从 1 起)。
    ///
    /// - 服务端给了 `retry_after` → **尊重服务端** (钳到 `max_backoff`);
    /// - 否则指数退避 `initial_backoff * multiplier^(attempt-1)` (钳到 `max_backoff`)。
    pub fn backoff_delay(&self, attempt: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(ra) = retry_after {
            return ra.min(self.max_backoff);
        }
        let mut delay = self.initial_backoff;
        let steps = attempt.saturating_sub(1).min(16);
        for _ in 0..steps {
            delay = delay.saturating_mul(self.backoff_multiplier.max(1));
            if delay >= self.max_backoff {
                return self.max_backoff;
            }
        }
        delay.min(self.max_backoff)
    }
}

// ============================================================================
// §3 TransportConfig / ClientCredentials
// ============================================================================

/// 传输配置。
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// 平台开放 API base URL (https 强制, mock 场景可用 http 环回地址)。
    pub api_base: String,
    /// 单次尝试超时。
    pub attempt_timeout: Duration,
    /// 整次调用总预算 (含所有重试, 走 Deadline 计时)。
    pub call_deadline: Duration,
    /// 重试 + 退避策略。
    pub retry: RetryPolicy,
    /// tenant token 持久化缓存路径 (None = 不落盘)。
    pub token_cache_path: Option<PathBuf>,
    /// User-Agent。
    pub user_agent: String,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            api_base: DEFAULT_LARK_API_BASE.to_string(),
            attempt_timeout: Duration::from_secs(10),
            call_deadline: Duration::from_secs(30),
            retry: RetryPolicy::default(),
            token_cache_path: None,
            user_agent: format!("apeireth-sdk-lark/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

impl TransportConfig {
    /// 校验配置 (base URL 必须是 http(s) 且非空)。
    pub fn validate(&self) -> LarkResult<()> {
        let base = self.api_base.trim();
        if base.is_empty() {
            return Err(LarkError::Other("api_base is empty".to_string()));
        }
        if !(base.starts_with("https://") || base.starts_with("http://")) {
            return Err(LarkError::Other(format!(
                "api_base must be http(s): {}",
                redacted_url_scheme(base)
            )));
        }
        if self.retry.max_attempts == 0 {
            return Err(LarkError::Other(
                "retry.max_attempts must be >= 1".to_string(),
            ));
        }
        if self.attempt_timeout.is_zero() || self.call_deadline.is_zero() {
            return Err(LarkError::Other("timeouts must be > 0".to_string()));
        }
        Ok(())
    }
}

/// URL 校验失败时的脱敏预览 (只留 scheme 之前的可见部分, 防带出查询串)。
fn redacted_url_scheme(url: &str) -> String {
    let scheme_end = url.find("://").map(|i| i + 3).unwrap_or(0);
    format!(
        "<non-http(s) url, {} bytes>",
        url.len().saturating_sub(scheme_end)
    )
}

/// 应用凭证 (App ID + App Secret)。
///
/// **Debug 手写脱敏**: App Secret 不得进日志。
#[derive(Clone)]
pub struct ClientCredentials {
    /// App ID。
    pub app_id: String,
    /// App Secret (秘密)。
    pub app_secret: String,
}

impl std::fmt::Debug for ClientCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientCredentials")
            .field("app_id", &self.app_id)
            .field("app_secret", &"[redacted]")
            .finish()
    }
}

// ============================================================================
// §4 wire 基元 (严格形状 + 未知字段容错)
// ============================================================================

/// 业务响应信封: `{"code": <i32 必填>, "msg": <string 可选>, "data": <可选>}`。
///
/// 严格: `code` 缺失或非整数 = 永久错误;
/// 容错: 未知字段忽略 (不使用 deny_unknown_fields)。
#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
pub struct ApiEnvelope<T> {
    /// 平台业务码 (0 = 成功)。
    pub code: i32,
    /// 平台消息。
    #[serde(default)]
    pub msg: String,
    /// 负载。
    #[serde(default)]
    pub data: Option<T>,
}

impl<T> ApiEnvelope<T> {
    /// 信封 → 结果: `code != 0` 走闭合映射; `code == 0` 但缺 `data` → 永久错误。
    pub fn into_payload(self, surface: &'static str) -> LarkResult<T> {
        if self.code != 0 {
            return Err(LarkError::from_platform_code(self.code, &self.msg));
        }
        self.data.ok_or_else(|| {
            LarkError::Other(format!(
                "malformed response: missing data payload ({surface})"
            ))
        })
    }
}

/// 列表端点统一分页负载: `{"items": [...], "has_more": bool, "page_token": "..."}`。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
pub struct Page<T> {
    /// 当前页条目。
    #[serde(default)]
    pub items: Vec<T>,
    /// 是否还有下一页。
    #[serde(default)]
    pub has_more: bool,
    /// 下一页 token。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
}

/// 从 JSON 值严格解析信封 (未知字段容错, 缺 `code` / 类型错 = 永久错误)。
pub fn parse_envelope<T: DeserializeOwned>(
    value: &serde_json::Value,
    surface: &'static str,
) -> LarkResult<ApiEnvelope<T>> {
    serde_json::from_value(value.clone()).map_err(|e| {
        LarkError::Other(format!(
            "malformed response envelope ({surface}): {}",
            bounded(&e.to_string())
        ))
    })
}

/// 错误预览截断 (UTF-8 边界安全; 防畸形响应把错误串撑爆; 不含任何秘密面)。
fn bounded(s: &str) -> String {
    const MAX: usize = 200;
    if s.chars().count() <= MAX {
        return s.to_string();
    }
    let preview: String = s.chars().take(MAX).collect();
    format!("{}…(+{} bytes)", preview, s.len() - preview.len())
}

// ============================================================================
// §5 ApiRequest
// ============================================================================

/// HTTP 方法 (闭合: 协议面只用 GET / POST)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    /// GET。
    Get,
    /// POST。
    Post,
}

/// 认证模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// 携带 `Authorization: Bearer <tenant_access_token>` (自动补/刷 token)。
    Bearer,
    /// 不带认证头 (token 颁发端点自身)。
    None,
}

/// 单个 API 请求的描述 (纯数据; 拼 URL / 发送在传输层)。
#[derive(Debug, Clone)]
pub struct ApiRequest {
    /// 方法。
    pub method: HttpMethod,
    /// 路径 (以 `/` 开头, 拼到 `api_base` 之后)。
    pub path: String,
    /// 查询参数 (有序, URL 编码由 url crate 负责)。
    pub query: Vec<(String, String)>,
    /// JSON 请求体。
    pub body: Option<serde_json::Value>,
    /// 认证模式。
    pub auth: AuthMode,
}

impl ApiRequest {
    /// GET 请求。
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: HttpMethod::Get,
            path: path.into(),
            query: Vec::new(),
            body: None,
            auth: AuthMode::Bearer,
        }
    }

    /// POST 请求 (JSON 体)。
    pub fn post(path: impl Into<String>, body: serde_json::Value) -> Self {
        Self {
            method: HttpMethod::Post,
            path: path.into(),
            query: Vec::new(),
            body: Some(body),
            auth: AuthMode::Bearer,
        }
    }

    /// 追加查询参数。
    pub fn with_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    /// 覆盖认证模式。
    pub fn with_auth(mut self, auth: AuthMode) -> Self {
        self.auth = auth;
        self
    }

    /// 拼完整 URL (查询串走标准 URL 编码)。
    pub fn build_url(&self, api_base: &str) -> LarkResult<String> {
        let base = api_base.trim_end_matches('/');
        let joined = format!("{base}{}", self.path);
        let mut url = url::Url::parse(&joined)
            .map_err(|e| LarkError::Other(format!("invalid request url: {e}")))?;
        {
            let mut pairs = url.query_pairs_mut();
            for (k, v) in &self.query {
                pairs.append_pair(k, v);
            }
        }
        Ok(url.to_string())
    }
}

/// 路径段百分号编码 (unreserved 之外全部编码; 供 email 等含保留字符的路径参数用)。
pub fn encode_path_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char);
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

// ============================================================================
// §6 LarkTransport (重试循环 + 认证管理)
// ============================================================================

/// lark HTTP 传输层 (线程安全; token 槽内部互斥)。
pub struct LarkTransport {
    http: reqwest::Client,
    config: TransportConfig,
    tenant_token: std::sync::Mutex<Option<TenantAccessToken>>,
}

impl std::fmt::Debug for LarkTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LarkTransport")
            .field("config", &self.config)
            .field(
                "tenant_token",
                &self.tenant_token.lock().ok().map(|g| g.is_some()),
            )
            .finish()
    }
}

impl LarkTransport {
    /// 创建传输层 (校验配置 + 构造 reqwest 客户端)。
    pub fn new(config: TransportConfig) -> LarkResult<Self> {
        config.validate()?;
        let http = reqwest::Client::builder()
            .user_agent(config.user_agent.clone())
            .build()
            .map_err(|e| LarkError::Network(format!("http client build failed: {e}")))?;
        Ok(Self {
            http,
            config,
            tenant_token: std::sync::Mutex::new(None),
        })
    }

    /// 传输配置。
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }

    /// 当前内存 token 槽内容 (cloned)。
    pub fn cached_tenant_token(&self) -> Option<TenantAccessToken> {
        self.tenant_token
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// 手动注入 tenant token (跳过颁发端点)。
    pub fn set_tenant_token(&self, token: TenantAccessToken) {
        if let Ok(mut guard) = self.tenant_token.lock() {
            *guard = Some(token);
        }
    }

    /// 失效 token (内存 + 磁盘缓存)。
    pub fn invalidate_tenant_token(&self) {
        if let Ok(mut guard) = self.tenant_token.lock() {
            *guard = None;
        }
        if let Some(path) = &self.config.token_cache_path {
            if let Err(e) = TokenCache::new(path).clear() {
                tracing::warn!(
                    target: "apeireth_sdk_lark",
                    "token cache clear failed (non-fatal): {}", e
                );
            }
        }
    }

    /// 执行 API 请求并解析信封负载 (认证头 / 重试 / 退避 / Deadline 全套)。
    pub async fn execute<T: DeserializeOwned>(
        &self,
        creds: &ClientCredentials,
        request: &ApiRequest,
        surface: &'static str,
    ) -> LarkResult<T> {
        let value = self.send_with_retry(creds, request).await?;
        let envelope = parse_envelope::<T>(&value, surface)?;
        envelope.into_payload(surface)
    }

    /// 执行列表请求并解析分页负载。
    pub async fn execute_page<T: DeserializeOwned>(
        &self,
        creds: &ClientCredentials,
        request: &ApiRequest,
        surface: &'static str,
    ) -> LarkResult<Page<T>> {
        self.execute::<Page<T>>(creds, request, surface).await
    }

    /// 确保可用 tenant token (内存 → 磁盘缓存 → 平台颁发)。
    pub async fn ensure_tenant_token(
        &self,
        creds: &ClientCredentials,
    ) -> LarkResult<TenantAccessToken> {
        // 1. 内存槽
        if let Some(token) = self.cached_tenant_token() {
            if !token.is_expired_with_skew() {
                return Ok(token);
            }
        }
        // 2. 磁盘缓存 (storage_atomic 持久化 token 缓存)
        if let Some(path) = &self.config.token_cache_path {
            match TokenCache::new(path).load(&creds.app_id) {
                Ok(Some(token)) if !token.is_expired_with_skew() => {
                    self.set_tenant_token(token.clone());
                    return Ok(token);
                }
                Ok(_) => {}
                Err(e) => {
                    // 缓存读失败不阻断: 退化为直接颁发
                    tracing::warn!(
                        target: "apeireth_sdk_lark",
                        "token cache load failed (non-fatal): {}", e
                    );
                }
            }
        }
        // 3. 平台颁发
        let body = serde_json::json!({
            "app_id": creds.app_id,
            "app_secret": creds.app_secret,
        });
        let request = ApiRequest::post(TENANT_TOKEN_PATH, body).with_auth(AuthMode::None);
        // 递归边界: send_with_retry ↔ attempt_once ↔ ensure_tenant_token 的类型环
        // 在此装箱断开 (本方法是唯一递归调用点)。
        let value = Box::pin(self.send_with_retry(creds, &request)).await?;
        let token = parse_tenant_token_response(&value, &creds.app_id)?;
        self.set_tenant_token(token.clone());
        if let Some(path) = &self.config.token_cache_path {
            if let Err(e) = TokenCache::new(path).store(&token) {
                // 落盘失败不影响本次调用 (内存槽已就位)
                tracing::warn!(
                    target: "apeireth_sdk_lark",
                    "token cache store failed (non-fatal): {}", e
                );
            }
        }
        Ok(token)
    }

    // ------------------------------------------------------------------
    // 内部: 重试循环
    // ------------------------------------------------------------------

    /// 带重试的发送: 返回平台 JSON (信封解开留给上层)。
    ///
    /// 循环规则 (闭合):
    /// - `AuthFailed` 且是带认证的请求: 失效 token → 重新颁发 → 再试 (整次调用仅一次);
    /// - `Retryable` 且未到 `max_attempts` 且 Deadline 未尽: 等退避后重试;
    /// - 其它: 直接返回错误 (永久 / 认证失败不再盲重试)。
    async fn send_with_retry(
        &self,
        creds: &ClientCredentials,
        request: &ApiRequest,
    ) -> LarkResult<serde_json::Value> {
        let (deadline, _notice) =
            apeireth_core::deadline::Deadline::after(self.config.call_deadline)
                .map_err(|e| LarkError::Other(format!("deadline arm failed: {e:?}")))?;
        let mut attempt: u32 = 0;
        let mut auth_refreshed = false;
        loop {
            attempt += 1;
            let result = self.attempt_once(creds, request, &deadline).await;
            match result {
                Ok(value) => return Ok(value),
                Err(err)
                    if err.is_auth_failure()
                        && request.auth == AuthMode::Bearer
                        && !auth_refreshed =>
                {
                    // 认证失败: 闭合刷新一次 (内存 + 磁盘缓存失效 → 重新颁发)
                    tracing::warn!(
                        target: "apeireth_sdk_lark",
                        "auth failure on {} {}, refreshing tenant token once",
                        method_str(request.method),
                        request.path
                    );
                    auth_refreshed = true;
                    self.invalidate_tenant_token();
                    self.ensure_tenant_token(creds).await?;
                    continue;
                }
                Err(err)
                    if err.is_retryable()
                        && attempt < self.config.retry.max_attempts
                        && !deadline.is_expired()
                        && deadline.remaining() > Duration::ZERO =>
                {
                    let retry_after = match &err {
                        LarkError::RateLimited {
                            retry_after_secs: 0,
                        } => None,
                        LarkError::RateLimited { retry_after_secs } => {
                            Some(Duration::from_secs(*retry_after_secs))
                        }
                        _ => None,
                    };
                    let delay = self.config.retry.backoff_delay(attempt, retry_after);
                    tracing::warn!(
                        target: "apeireth_sdk_lark",
                        "retryable failure on {} {} (class={}, attempt={}/{}), backoff {:?}",
                        method_str(request.method),
                        request.path,
                        err.class().as_str(),
                        attempt,
                        self.config.retry.max_attempts,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// 单次尝试: 附认证头 → 发送 → 状态/业务码映射。
    async fn attempt_once(
        &self,
        creds: &ClientCredentials,
        request: &ApiRequest,
        deadline: &apeireth_core::deadline::Deadline,
    ) -> LarkResult<serde_json::Value> {
        let url = request.build_url(&self.config.api_base)?;
        let mut builder = match request.method {
            HttpMethod::Get => self.http.get(&url),
            HttpMethod::Post => self.http.post(&url),
        };
        if request.auth == AuthMode::Bearer {
            let token = self.ensure_tenant_token(creds).await?;
            builder = builder.header(AUTH_HEADER_NAME, format!("{BEARER_SCHEME} {}", token.token));
        }
        if let Some(body) = &request.body {
            builder = builder.json(body);
        }

        let budget = self
            .config
            .attempt_timeout
            .min(deadline.remaining())
            .max(Duration::from_millis(1));
        let response = match tokio::time::timeout(budget, builder.send()).await {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => return Err(LarkError::Network(format!("request failed: {e}"))),
            Err(_) => {
                return Err(LarkError::Network(format!(
                    "attempt timed out after {budget:?}"
                )))
            }
        };

        let status = response.status();
        let retry_after_secs = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());
        // 只留状态/业务码进日志 (脱敏: 0 体 0 token)
        tracing::debug!(
            target: "apeireth_sdk_lark",
            "http {} {} -> {}",
            method_str(request.method),
            request.path,
            status.as_u16()
        );

        if status.as_u16() == 429 {
            return Err(LarkError::RateLimited {
                retry_after_secs: retry_after_secs.unwrap_or(0),
            });
        }
        if matches!(status.as_u16(), 401 | 403) {
            return Err(LarkError::TokenExpired);
        }
        if matches!(status.as_u16(), 408 | 500..=599) {
            return Err(LarkError::Network(format!(
                "server error: http {}",
                status.as_u16()
            )));
        }
        if !status.is_success() {
            return Err(LarkError::Other(format!(
                "http status {} on {} {}",
                status.as_u16(),
                method_str(request.method),
                request.path
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| LarkError::Network(format!("response read failed: {e}")))?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            LarkError::Other(format!(
                "malformed response json: {}",
                bounded(&e.to_string())
            ))
        })?;
        // 平台业务码 != 0 → 闭合映射 (限流码可重试 / 认证码 AuthFailed / 其它永久)
        if let Some(code) = value.get("code").and_then(|c| c.as_i64()) {
            let code = i32::try_from(code).unwrap_or(i32::MAX);
            if code != 0 {
                let msg = value
                    .get("msg")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                return Err(LarkError::from_platform_code(code, &msg));
            }
        }
        Ok(value)
    }
}

fn method_str(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
    }
}

/// token 颁发响应 (flat 形状: 信封字段 + token 字段同层)。
///
/// 严格: 成功时 `tenant_access_token` 必须存在且非空;
/// 容错: 未知字段忽略。
#[derive(Debug, Clone, Deserialize)]
pub struct TenantTokenResponse {
    /// 平台业务码。
    pub code: i32,
    /// 平台消息。
    #[serde(default)]
    pub msg: String,
    /// 颁发的 token。
    #[serde(default)]
    pub tenant_access_token: Option<String>,
    /// 相对 TTL (秒)。
    #[serde(default)]
    pub expire: Option<u64>,
}

/// 解析 token 颁发响应 (含业务码闭合映射 + 字段校验)。
pub fn parse_tenant_token_response(
    value: &serde_json::Value,
    app_id: &str,
) -> LarkResult<TenantAccessToken> {
    let parsed: TenantTokenResponse = serde_json::from_value(value.clone()).map_err(|e| {
        LarkError::Other(format!(
            "malformed token response: {}",
            bounded(&e.to_string())
        ))
    })?;
    if parsed.code != 0 {
        return Err(LarkError::from_platform_code(parsed.code, &parsed.msg));
    }
    let token = parsed.tenant_token_or_err()?;
    TenantAccessToken::from_issue_response(app_id.to_string(), token, parsed.expire.unwrap_or(0))
}

impl TenantTokenResponse {
    fn tenant_token_or_err(&self) -> LarkResult<String> {
        match &self.tenant_access_token {
            Some(t) if !t.is_empty() => Ok(t.clone()),
            _ => Err(LarkError::Other(
                "malformed token response: missing tenant_access_token".to_string(),
            )),
        }
    }
}

// ============================================================================
// §7 单元测试 (纯逻辑: 退避 / 信封 / 分页 / token 响应)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 重试退避 ----

    #[test]
    fn backoff_is_exponential_and_capped() {
        let policy = RetryPolicy {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_millis(350),
            backoff_multiplier: 2,
        };
        assert_eq!(policy.backoff_delay(1, None), Duration::from_millis(100));
        assert_eq!(policy.backoff_delay(2, None), Duration::from_millis(200));
        assert_eq!(
            policy.backoff_delay(3, None),
            Duration::from_millis(350),
            "钳到 max_backoff"
        );
        assert_eq!(policy.backoff_delay(9, None), Duration::from_millis(350));
    }

    #[test]
    fn backoff_honors_retry_after_and_clamps() {
        let policy = RetryPolicy::default();
        assert_eq!(
            policy.backoff_delay(1, Some(Duration::from_secs(0))),
            Duration::from_secs(0),
            "Retry-After: 0 应原样尊重"
        );
        assert_eq!(
            policy.backoff_delay(1, Some(Duration::from_secs(1))),
            Duration::from_secs(1)
        );
        assert_eq!(
            policy.backoff_delay(1, Some(Duration::from_secs(999))),
            policy.max_backoff,
            "Retry-After 钳到 max_backoff"
        );
    }

    #[test]
    fn retry_policy_no_retry_has_single_attempt() {
        assert_eq!(RetryPolicy::no_retry().max_attempts, 1);
    }

    // ---- 信封严格解析 + 未知字段容错 ----

    #[derive(Debug, Clone, Deserialize)]
    struct DemoData {
        #[allow(dead_code)]
        name: String,
    }

    #[test]
    fn envelope_tolerates_unknown_fields() {
        let value = serde_json::json!({
            "code": 0,
            "msg": "ok",
            "data": {"name": "x", "future_field": 42},
            "extra_top_level": true
        });
        let env = parse_envelope::<DemoData>(&value, "demo").expect("parse");
        let payload = env.into_payload("demo").expect("payload");
        assert_eq!(payload.name, "x");
    }

    #[test]
    fn envelope_requires_code() {
        let value = serde_json::json!({"msg": "ok", "data": {"name": "x"}});
        let result = parse_envelope::<DemoData>(&value, "demo");
        assert!(matches!(result, Err(LarkError::Other(_))), "缺 code 必须拒");
    }

    #[test]
    fn envelope_maps_nonzero_codes_by_closed_table() {
        // 限流码 → RateLimited (可重试)
        let value = serde_json::json!({"code": 99991400, "msg": "rate limit"});
        let env = parse_envelope::<DemoData>(&value, "demo").expect("parse");
        match env.into_payload("demo") {
            Err(e) => assert_eq!(e.class(), ErrorClass::Retryable),
            Ok(_) => panic!("nonzero code must fail"),
        }
        // 认证码 → AuthFailed
        let value = serde_json::json!({"code": 99991663, "msg": "invalid token"});
        let env = parse_envelope::<DemoData>(&value, "demo").expect("parse");
        match env.into_payload("demo") {
            Err(e) => assert_eq!(e.class(), ErrorClass::AuthFailed),
            Ok(_) => panic!("nonzero code must fail"),
        }
        // 其它业务码 → 永久
        let value = serde_json::json!({"code": 230001, "msg": "bad"});
        let env = parse_envelope::<DemoData>(&value, "demo").expect("parse");
        match env.into_payload("demo") {
            Err(e) => assert_eq!(e.class(), ErrorClass::Permanent),
            Ok(_) => panic!("nonzero code must fail"),
        }
    }

    #[test]
    fn envelope_success_without_data_is_permanent_error() {
        let value = serde_json::json!({"code": 0, "msg": "ok"});
        let env = parse_envelope::<DemoData>(&value, "demo").expect("parse");
        assert!(matches!(env.into_payload("demo"), Err(LarkError::Other(_))));
    }

    // ---- 分页基元 ----

    #[test]
    fn page_parses_with_defaults_and_unknown_fields() {
        let value = serde_json::json!({
            "items": [{"name": "a"}, {"name": "b"}],
            "has_more": true,
            "page_token": "p2",
            "unknown": 1
        });
        let page: Page<DemoData> = serde_json::from_value(value).expect("parse");
        assert_eq!(page.items.len(), 2);
        assert!(page.has_more);
        assert_eq!(page.page_token.as_deref(), Some("p2"));

        // 缺字段 → 全默认 (容错)
        let page: Page<DemoData> = serde_json::from_value(serde_json::json!({})).expect("parse");
        assert!(page.items.is_empty());
        assert!(!page.has_more);
        assert!(page.page_token.is_none());
    }

    // ---- token 颁发响应 ----

    #[test]
    fn tenant_token_response_parses_success() {
        let value = serde_json::json!({
            "code": 0,
            "msg": "ok",
            "tenant_access_token": "t-abc123",
            "expire": 7200,
            "unknown": "ignored"
        });
        let token = parse_tenant_token_response(&value, "cli_a1b2c3d4e5f6").expect("token");
        assert_eq!(token.token, "t-abc123");
    }

    #[test]
    fn tenant_token_response_rejects_missing_token_and_nonzero_code() {
        let value = serde_json::json!({"code": 0, "msg": "ok"});
        assert!(matches!(
            parse_tenant_token_response(&value, "cli_a1b2c3d4e5f6"),
            Err(LarkError::Other(_))
        ));
        let value = serde_json::json!({"code": 99991663, "msg": "bad secret"});
        match parse_tenant_token_response(&value, "cli_a1b2c3d4e5f6") {
            Err(e) => assert_eq!(e.class(), ErrorClass::AuthFailed),
            Ok(_) => panic!("nonzero code must fail"),
        }
    }

    // ---- URL 拼接 (查询参数编码) ----

    #[test]
    fn request_url_joins_base_and_percent_encodes_query() {
        let req = ApiRequest::get("/calendar/v4/calendars/cal_1/events")
            .with_query("start_time", "2026-08-05T10:00:00Z");
        let url = req
            .build_url("https://api.example.test/open-apis")
            .expect("url");
        assert!(url
            .starts_with("https://api.example.test/open-apis/calendar/v4/calendars/cal_1/events?"));
        assert!(
            url.contains("start_time=2026-08-05T10%3A00%3A00Z"),
            "RFC3339 必须被编码: {url}"
        );
    }

    #[test]
    fn transport_config_rejects_non_http_base_and_zero_attempts() {
        let mut config = TransportConfig {
            api_base: "ftp://x".to_string(),
            ..TransportConfig::default()
        };
        assert!(config.validate().is_err());
        config.api_base = "https://ok.example.test".to_string();
        assert!(config.validate().is_ok());
        config.retry.max_attempts = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn path_segment_encoding_handles_reserved_chars() {
        assert_eq!(encode_path_segment("cal_abc-123"), "cal_abc-123");
        assert_eq!(
            encode_path_segment("user@example.com"),
            "user%40example.com"
        );
        assert_eq!(encode_path_segment("a b/c"), "a%20b%2Fc");
    }
}
