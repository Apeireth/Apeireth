//! `tool.http_write` —— **带鉴权的受控 HTTP 写面**。
//!
//! # 定位
//!
//! 一条**产品面**：把「**写方法 + 鉴权头 + 受控出网闸**」合成一个模型可调的
//! 工具。动作词表**闭合**：`post` / `put` / `patch` / `delete` 四种，表外动作
//! **拒绝即帧**。
//!
//! # 硬约束（仓内既有架构不变量）
//!
//! `crates/capabilities/tools/tests/architecture_invariants.rs` 钉死「**不得有
//! 第二个 HTTP 客户端**」⇒ ⇒ 本面**不自带任何 HTTP 客户端**：请求一律经
//! [`ControlledEgress`] 的写方法（[`ControlledEgress::write`]）发出 ⇒
//! **白名单 / 私网判定 / DNS 钉扎 / 超时 / 响应体上限全部复用同一套**，
//! 不重写第二份。
//!
//! ⇒ 本面**不做 DNS 解析**、**不判定私网**、**不自己拼 URL**、**不重定向**。
//!
//! # 鉴权口径（凭据链）
//!
//! * 凭据**只从既有凭据链取**（[`CredentialResolver`]，运行时注入；环境变量 /
//!   钥匙串 / 运行时热写都走这一个接口）。
//! * **只认显式配置的凭据引用**：模型给的 `credential_ref` 必须命中配置里的
//!   绑定表，否则**拒绝即帧**；模型**不能**自带凭据明文，也**不能**自带
//!   请求头（请求头只来自固定配置 + 鉴权绑定）。
//! * 凭据**明文永不冻结、永不回显、不进日志**：冻结载荷里只有**凭据逻辑名**
//!   （名字不是秘密，值才是），值在**执行时刻**现取现用。
//! * 绑定里写明的主机与请求主机必须**完全一致** ⇒ 换主机 = 换凭据 = 拒绝。
//! * 远端把凭据**回显**进响应体时 ⇒ 按**值精确遮蔽** + 形状层扫描 ⇒ 明文
//!   不进模型上下文，且**命中可观测**。
//!
//! # 边界
//!
//! * 请求体字节上限（[`ControlledEgress::max_request_bytes`]）。
//! * 响应体字节上限（[`ControlledEgress::max_response_bytes`]）+ 结果面再收一道。
//! * 超时上限（[`ControlledEgress::timeout`]）。
//! * **重定向不跟随**（写请求带凭据 ⇒ 3xx 拒绝即帧）。
//! * 方法白名单（[`EgressMethod`] 闭合词表）。
//! * 目的地只认**显式域名白名单**（`EgressPolicy::ExplicitAllowList`）——
//!   写面**不得**用「任意公网」或「不设限」策略构造。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::untrusted_mark::UntrustedContentWrapper;
use apeireth_plugin::{CredentialResolver, FrozenInvocation, Secret, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::egress::{
    ControlledEgress, EgressAllowList, EgressError, EgressHttpResponse, EgressMethod, EgressPolicy,
};
use crate::guardrail::{attach_credential_tripwire, scan_result_face, LeakedCredentialKind};

/// Maximum accepted URL length in bytes.
pub const HTTP_WRITE_URL_MAX_BYTES: usize = 8 * 1024;
/// Maximum accepted request body length in **characters** (schema level).
///
/// ⚠️ 这是**模型面**的粗筛；真正的字节上限是
/// [`ControlledEgress::max_request_bytes`]（执行前再判一次）。
pub const HTTP_WRITE_BODY_MAX_CHARS: usize = 256 * 1024;
/// Maximum accepted `content_type` length in bytes.
pub const HTTP_WRITE_CONTENT_TYPE_MAX_BYTES: usize = 128;
/// Maximum accepted `credential_ref` length in bytes.
pub const HTTP_WRITE_CREDENTIAL_REF_MAX_BYTES: usize = 128;
/// Maximum accepted credential header **name** length in bytes.
pub const HTTP_WRITE_HEADER_NAME_MAX_BYTES: usize = 128;

/// 默认给模型看的响应体上限（**结果面**再收一道，比传输层上限更紧）。
pub const DEFAULT_HTTP_WRITE_RESULT_BYTES: usize = 64 * 1024;

/// 凭据回显时的遮蔽标记（**按值精确遮蔽**）。
pub const CREDENTIAL_ECHO_MASK: &str = "[REDACTED_CREDENTIAL_ECHO]";

const HTTP_WRITE_FROZEN_VERSION: u32 = 1;
const HTTP_WRITE_TOOL_NAME: &str = "http_write";
const DEFAULT_CONTENT_TYPE: &str = "application/json";

/// 凭据绑定方式（**闭合词表**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthScheme {
    /// `Authorization: Bearer <值>`
    Bearer,
    /// `<头名>: <值>`（原样放入一个固定头，例如 API key 头）。
    Raw,
}

/// 一条**显式配置**的凭据绑定。
///
/// # 为什么必须显式配置
///
/// 模型可以自由书写 `credential_ref` ⇒ 若「引用 → 凭据」的映射由模型决定,
/// 就等于让模型在**任何**主机上取**任何**凭据 ⇒ ⇒ 映射只存在于**配置里**,
/// 且每条绑定写明**这个凭据只许发给哪个主机**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialBinding {
    credential_name: String,
    host: String,
    header: String,
    scheme: AuthScheme,
}

impl CredentialBinding {
    /// `Authorization: Bearer` 绑定。
    pub fn bearer(credential_name: impl Into<String>, host: impl Into<String>) -> Self {
        let host = host.into();
        Self {
            credential_name: credential_name.into(),
            host: host.trim().to_ascii_lowercase(),
            header: "authorization".to_string(),
            scheme: AuthScheme::Bearer,
        }
    }

    /// 自定义头绑定（头名必须合法：非空、无 CR/LF、无冒号、无空白）。
    pub fn raw(
        credential_name: impl Into<String>,
        host: impl Into<String>,
        header: impl Into<String>,
    ) -> Self {
        let host = host.into();
        let header = header.into();
        Self {
            credential_name: credential_name.into(),
            host: host.trim().to_ascii_lowercase(),
            header: header.trim().to_ascii_lowercase(),
            scheme: AuthScheme::Raw,
        }
    }

    /// 凭据的**逻辑名**（不是秘密；值才是）。
    pub fn credential_name(&self) -> &str {
        &self.credential_name
    }

    /// 这条绑定允许发往的**唯一**主机。
    pub fn host(&self) -> &str {
        &self.host
    }

    /// 鉴权头名。
    pub fn header(&self) -> &str {
        &self.header
    }

    /// 鉴权方式。
    pub fn scheme(&self) -> AuthScheme {
        self.scheme
    }
}

/// 写面配置错误。
#[derive(Debug, thiserror::Error)]
pub enum HttpWriteConfigError {
    #[error(
        "the http write face requires an explicit domain allow list; \
         the egress policy must be EgressPolicy::ExplicitAllowList"
    )]
    PolicyNotAllowListed,
    #[error("credential binding header name is invalid: {0}")]
    InvalidHeaderName(String),
    #[error("credential binding host must not be empty")]
    EmptyHost,
    #[error("credential binding name must not be empty")]
    EmptyCredentialName,
}

/// `tool.http_write` 的可信配置。
///
/// # 三样东西
///
/// * **出网通道**：一条 [`ControlledEgress`]，其策略**必须**是
///   `EgressPolicy::ExplicitAllowList`（写面不得放到「任意公网」或「不设限」）。
/// * **凭据链**：一个 [`CredentialResolver`]（运行时注入；本面**不持**凭据明文）。
/// * **凭据绑定表**：显式配置的「凭据逻辑名 → 主机 + 头 + 方式」。
#[derive(Clone)]
pub struct HttpWriteConfig {
    egress: Arc<ControlledEgress>,
    credentials: Arc<dyn CredentialResolver>,
    bindings: BTreeMap<String, CredentialBinding>,
    max_result_bytes: usize,
}

impl std::fmt::Debug for HttpWriteConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // ⚠️ 凭据链**不进 Debug**（解析器实现可能把明文带进自己的 Debug）。
        f.debug_struct("HttpWriteConfig")
            .field("egress", &self.egress)
            .field("credentials", &"<redacted>")
            .field("bindings", &self.bindings.keys().collect::<Vec<_>>())
            .field("max_result_bytes", &self.max_result_bytes)
            .finish()
    }
}

impl HttpWriteConfig {
    /// 用一条**已配置好**的受控通道与一条凭据链构造。
    ///
    /// 拒绝非白名单策略：写面**必须**有显式域名白名单。
    pub fn new(
        egress: Arc<ControlledEgress>,
        credentials: Arc<dyn CredentialResolver>,
    ) -> Result<Self, HttpWriteConfigError> {
        if !matches!(egress.policy(), EgressPolicy::ExplicitAllowList(_)) {
            return Err(HttpWriteConfigError::PolicyNotAllowListed);
        }
        Ok(Self {
            egress,
            credentials,
            bindings: BTreeMap::new(),
            max_result_bytes: DEFAULT_HTTP_WRITE_RESULT_BYTES,
        })
    }

    /// 用一份**域名白名单**构造（默认超时 / 字节上限）。
    pub fn allow_list(
        list: EgressAllowList,
        credentials: Arc<dyn CredentialResolver>,
    ) -> Result<Self, HttpWriteConfigError> {
        let egress = Arc::new(ControlledEgress::new(EgressPolicy::ExplicitAllowList(list)));
        Self::new(egress, credentials)
    }

    /// 登记一条凭据绑定（同名覆盖 ⇒ 以最后一次为准）。
    pub fn with_binding(
        mut self,
        binding: CredentialBinding,
    ) -> Result<Self, HttpWriteConfigError> {
        if binding.credential_name.trim().is_empty() {
            return Err(HttpWriteConfigError::EmptyCredentialName);
        }
        if binding.host.trim().is_empty() {
            return Err(HttpWriteConfigError::EmptyHost);
        }
        if binding.scheme == AuthScheme::Raw && !is_valid_header_name(&binding.header) {
            return Err(HttpWriteConfigError::InvalidHeaderName(binding.header));
        }
        self.bindings
            .insert(binding.credential_name.clone(), binding);
        Ok(self)
    }

    /// 设置**结果面**的响应体上限（比传输层上限更紧的那一道）。
    #[must_use]
    pub fn with_max_result_bytes(mut self, max_result_bytes: usize) -> Self {
        self.max_result_bytes = max_result_bytes;
        self
    }

    /// 受控出网通道。
    pub fn egress(&self) -> &Arc<ControlledEgress> {
        &self.egress
    }

    /// 凭据链。
    pub fn credentials(&self) -> &Arc<dyn CredentialResolver> {
        &self.credentials
    }

    /// 结果面响应体上限。
    pub fn max_result_bytes(&self) -> usize {
        self.max_result_bytes
    }

    /// 按逻辑名取凭据绑定。
    pub fn credential_binding(&self, credential_name: &str) -> Option<&CredentialBinding> {
        self.bindings.get(credential_name)
    }
}

/// 头名合法性（与传输层的 CRLF 校验同族，再加「不能带冒号 / 空白」）。
fn is_valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= HTTP_WRITE_HEADER_NAME_MAX_BYTES
        && name
            .chars()
            .all(|c| c.is_ascii_graphic() && c != ':' && c != ' ' && c != '\t')
}

/// 工具调用参数（未知字段拒绝）。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HttpWriteParams {
    method: String,
    url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    content_type: Option<String>,
    #[serde(default)]
    credential_ref: Option<String>,
}

/// 冻结的凭据绑定快照（**不含明文**）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FrozenCredentialBinding {
    credential_name: String,
    host: String,
    header: String,
    scheme: AuthScheme,
}

/// 审批时冻结的**确切**执行载荷。
///
/// ⚠️ 凭据**值不在这里** ⇒ 冻结载荷可以被安全地落进审批记录与操作指纹。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HttpWriteFrozenInvocation {
    version: u32,
    method: EgressMethod,
    url: String,
    body: Option<String>,
    content_type: String,
    credential_ref: Option<String>,
    credential_binding: Option<FrozenCredentialBinding>,
    timeout_ms: u64,
    max_request_bytes: usize,
    max_response_bytes: usize,
    egress_policy: EgressPolicy,
}

/// `tool.http_write` —— 带鉴权的受控 HTTP 写面。
pub struct HttpWriteTool {
    id: CapabilityId,
    config: HttpWriteConfig,
}

impl std::fmt::Debug for HttpWriteTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // ⚠️ 同 [`HttpWriteConfig`]：凭据链不进 Debug。
        f.debug_struct("HttpWriteTool")
            .field("id", &self.id)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl HttpWriteTool {
    pub fn new(config: HttpWriteConfig) -> Self {
        Self {
            id: CapabilityId::new("tool.http_write").expect("tool.http_write is a valid id"),
            config,
        }
    }

    pub fn config(&self) -> &HttpWriteConfig {
        &self.config
    }

    fn declaration_parameters() -> ToolParameters {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "method": {
                    "type": "string",
                    "enum": ["post", "put", "patch", "delete"],
                    "description": "HTTP write method. The vocabulary is closed: post, put, patch, delete."
                },
                "url": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": HTTP_WRITE_URL_MAX_BYTES,
                    "description": "HTTP(S) URL to write to. It must be on the configured domain allow list; URL userinfo is rejected."
                },
                "body": {
                    "type": "string",
                    "maxLength": HTTP_WRITE_BODY_MAX_CHARS,
                    "description": "Optional request body (text)."
                },
                "content_type": {
                    "type": "string",
                    "maxLength": HTTP_WRITE_CONTENT_TYPE_MAX_BYTES,
                    "description": "Optional request content type; defaults to application/json."
                },
                "credential_ref": {
                    "type": "string",
                    "maxLength": HTTP_WRITE_CREDENTIAL_REF_MAX_BYTES,
                    "description": "Optional name of an explicitly configured credential binding. Unknown names are refused."
                }
            },
            "required": ["method", "url"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(parameters.as_object().cloned().unwrap_or_default());
        params
    }

    fn parse_params(&self, call: &ToolCall) -> Result<HttpWriteParams, ToolResult> {
        let object = call.arguments.as_object().ok_or_else(|| {
            ToolResult::permanent_error(&call.id, "http write arguments must be a JSON object")
                .with_name(HTTP_WRITE_TOOL_NAME)
        })?;

        for key in object.keys() {
            if !matches!(
                key.as_str(),
                "method" | "url" | "body" | "content_type" | "credential_ref"
            ) {
                return Err(ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "unknown http write parameter {key:?}; only \"method\", \"url\", \
                         \"body\", \"content_type\" and \"credential_ref\" are supported"
                    ),
                )
                .with_name(HTTP_WRITE_TOOL_NAME));
            }
        }

        serde_json::from_value(call.arguments.clone()).map_err(|e| {
            ToolResult::permanent_error(&call.id, format!("invalid http write parameters: {e}"))
                .with_name(HTTP_WRITE_TOOL_NAME)
        })
    }

    /// 归一化 URL（**与受控通道同一套口径**：只认 http/https、拒 userinfo、必须有主机）。
    fn normalize_url(&self, call: &ToolCall, raw: &str) -> Result<Url, ToolResult> {
        if raw.is_empty() {
            return Err(
                ToolResult::permanent_error(&call.id, "http write url must not be empty")
                    .with_name(HTTP_WRITE_TOOL_NAME),
            );
        }
        if raw.len() > HTTP_WRITE_URL_MAX_BYTES {
            return Err(ToolResult::permanent_error(
                &call.id,
                format!(
                    "http write url is {} bytes; the maximum is {HTTP_WRITE_URL_MAX_BYTES} bytes",
                    raw.len()
                ),
            )
            .with_name(HTTP_WRITE_TOOL_NAME));
        }

        let mut url = Url::parse(raw).map_err(|e| {
            ToolResult::permanent_error(&call.id, format!("invalid http write url: {e}"))
                .with_name(HTTP_WRITE_TOOL_NAME)
        })?;

        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported http write url scheme {:?}; only http and https are supported",
                    url.scheme()
                ),
            )
            .with_name(HTTP_WRITE_TOOL_NAME));
        }
        if !url.username().is_empty() || url.password().is_some() {
            // ⚠️ userinfo 是最常见的「白名单绕过」形态：
            //    `http://allowed.example@evil.example/` 的**真实主机**是后者。
            return Err(ToolResult::permanent_error(
                &call.id,
                "http write url userinfo is not supported and must not be sent",
            )
            .with_name(HTTP_WRITE_TOOL_NAME));
        }
        if url.host_str().is_none() {
            return Err(ToolResult::permanent_error(
                &call.id,
                "http write url must include a host",
            )
            .with_name(HTTP_WRITE_TOOL_NAME));
        }

        // 片段永远不会被发出去 ⇒ 冻结前先剥掉（冻结的必须是**真正会发出去**的请求）。
        url.set_fragment(None);
        Ok(url)
    }

    fn build_frozen(&self, call: &ToolCall) -> Result<HttpWriteFrozenInvocation, ToolResult> {
        let params = self.parse_params(call)?;

        // ① 动作词表**闭合**: 表外动作拒绝即帧。
        let method = EgressMethod::parse(&params.method).ok_or_else(|| {
            ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported http write method {:?}; the vocabulary is closed: post, put, patch, delete",
                    params.method
                ),
            )
            .with_name(HTTP_WRITE_TOOL_NAME)
        })?;

        let url = self.normalize_url(call, &params.url)?;

        // ② 请求体**有界**（字符粗筛 + 字节上限在执行前再判一次）。
        if let Some(body) = &params.body {
            if body.chars().count() > HTTP_WRITE_BODY_MAX_CHARS {
                return Err(ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "http write body is {} characters; the maximum is {HTTP_WRITE_BODY_MAX_CHARS}",
                        body.chars().count()
                    ),
                )
                .with_name(HTTP_WRITE_TOOL_NAME));
            }
        }

        // ③ 内容类型有界且不得含 CR/LF。
        let content_type = params
            .content_type
            .unwrap_or_else(|| DEFAULT_CONTENT_TYPE.to_string());
        if content_type.is_empty() || content_type.len() > HTTP_WRITE_CONTENT_TYPE_MAX_BYTES {
            return Err(ToolResult::permanent_error(
                &call.id,
                format!(
                    "http write content_type must be 1..{HTTP_WRITE_CONTENT_TYPE_MAX_BYTES} bytes"
                ),
            )
            .with_name(HTTP_WRITE_TOOL_NAME));
        }
        if content_type.contains('\r') || content_type.contains('\n') {
            return Err(ToolResult::permanent_error(
                &call.id,
                "http write content_type must not contain CR or LF",
            )
            .with_name(HTTP_WRITE_TOOL_NAME));
        }

        // ④ 凭据引用**只认显式配置的绑定**（未配置 ⇒ 拒绝即帧，连审批都不建）。
        let credential_ref = params
            .credential_ref
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty());
        let credential_binding = match &credential_ref {
            Some(name) => {
                if name.len() > HTTP_WRITE_CREDENTIAL_REF_MAX_BYTES {
                    return Err(ToolResult::permanent_error(
                        &call.id,
                        format!(
                            "http write credential_ref is {} bytes; the maximum is {HTTP_WRITE_CREDENTIAL_REF_MAX_BYTES}",
                            name.len()
                        ),
                    )
                    .with_name(HTTP_WRITE_TOOL_NAME));
                }
                let binding = self.config.credential_binding(name).ok_or_else(|| {
                    ToolResult::permanent_error(
                        &call.id,
                        format!(
                            "credential reference {name:?} is not an explicitly configured binding; \
                             refusing to resolve an unconfigured credential"
                        ),
                    )
                    .with_name(HTTP_WRITE_TOOL_NAME)
                })?;
                Some(FrozenCredentialBinding {
                    credential_name: binding.credential_name.clone(),
                    host: binding.host.clone(),
                    header: binding.header.clone(),
                    scheme: binding.scheme,
                })
            }
            None => None,
        };

        let egress = self.config.egress();
        Ok(HttpWriteFrozenInvocation {
            version: HTTP_WRITE_FROZEN_VERSION,
            method,
            url: url.to_string(),
            body: params.body,
            content_type,
            credential_ref,
            credential_binding,
            timeout_ms: egress.timeout().as_millis() as u64,
            max_request_bytes: egress.max_request_bytes(),
            max_response_bytes: egress.max_response_bytes(),
            egress_policy: egress.policy().clone(),
        })
    }

    /// 审批视图（**给人看的**）：含凭据**逻辑名**（不是秘密），不含任何明文。
    fn display_invocation(frozen: &HttpWriteFrozenInvocation) -> serde_json::Value {
        serde_json::json!({
            "version": frozen.version,
            "method": frozen.method.as_str(),
            "url": frozen.url,
            "content_type": frozen.content_type,
            "body_bytes": frozen.body.as_ref().map(|b| b.len()).unwrap_or(0),
            "credential_ref": frozen.credential_ref,
            "credential_host": frozen
                .credential_binding
                .as_ref()
                .map(|binding| binding.host.clone()),
            "egress_policy": format!("{:?}", frozen.egress_policy),
            "timeout_ms": frozen.timeout_ms,
            "max_request_bytes": frozen.max_request_bytes,
            "max_response_bytes": frozen.max_response_bytes,
        })
    }

    async fn execute_frozen(
        &self,
        call: &ToolCall,
        frozen: &HttpWriteFrozenInvocation,
    ) -> ToolResult {
        if frozen.version != HTTP_WRITE_FROZEN_VERSION {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported frozen http write invocation version {} (expected {})",
                    frozen.version, HTTP_WRITE_FROZEN_VERSION
                ),
            )
            .with_name(HTTP_WRITE_TOOL_NAME);
        }

        // 从冻结的**声明式**策略与上限重建通道（DNS 刻意**不冻结**: 连接时刻
        // 仍由受控通道解析并逐个复验）。
        let egress = ControlledEgress::new(frozen.egress_policy.clone())
            .with_timeout(Duration::from_millis(frozen.timeout_ms))
            .with_max_response_bytes(frozen.max_response_bytes)
            .with_max_request_bytes(frozen.max_request_bytes);

        self.execute_with_egress(call, &egress, frozen).await
    }

    async fn execute_with_egress(
        &self,
        call: &ToolCall,
        egress: &ControlledEgress,
        frozen: &HttpWriteFrozenInvocation,
    ) -> ToolResult {
        // ① 请求体字节上限（**发出之前**）。
        let body_bytes = frozen.body.as_deref().unwrap_or("").as_bytes();
        if body_bytes.len() > frozen.max_request_bytes {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "http write body is {} bytes; the maximum is {} bytes",
                    body_bytes.len(),
                    frozen.max_request_bytes
                ),
            )
            .with_name(HTTP_WRITE_TOOL_NAME);
        }

        // ② 鉴权：凭据**现取现用**（冻结载荷里没有值）。
        //
        // ⚠️ 凭据值**只在本函数内存活**（不进任何结果面、不进任何日志）。
        let mut resolved_secret: Option<Secret> = None;
        if let Some(binding) = &frozen.credential_binding {
            // 审批过的绑定必须与**当前**配置一致（批准的操作 == 执行的操作）。
            let Some(current) = self.config.credential_binding(&binding.credential_name) else {
                return ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "credential binding {:?} is no longer configured; refusing to send the write request",
                        binding.credential_name
                    ),
                )
                .with_name(HTTP_WRITE_TOOL_NAME);
            };
            if current.host != binding.host
                || current.header != binding.header
                || current.scheme != binding.scheme
            {
                return ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "credential binding {:?} changed after approval; refusing to send the write request",
                        binding.credential_name
                    ),
                )
                .with_name(HTTP_WRITE_TOOL_NAME);
            }

            // 绑定的主机必须与请求主机**完全一致**（换主机 = 换凭据 = 拒绝）。
            let Some(request_host) = Url::parse(&frozen.url)
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
            else {
                return ToolResult::permanent_error(
                    &call.id,
                    "http write url has no host; refusing to attach a credential",
                )
                .with_name(HTTP_WRITE_TOOL_NAME);
            };
            if request_host != binding.host {
                return ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "credential binding {:?} is bound to host {:?}, but the request targets {:?}; \
                         refusing to send the credential to an unbound host",
                        binding.credential_name, binding.host, request_host
                    ),
                )
                .with_name(HTTP_WRITE_TOOL_NAME);
            }

            // 端点绑定（若凭据链参与绑定 ⇒ 必须命中）。
            if let Err(detail) =
                self.ensure_credential_bound_to_endpoint(&binding.credential_name, &frozen.url)
            {
                return ToolResult::permanent_error(&call.id, detail)
                    .with_name(HTTP_WRITE_TOOL_NAME);
            }

            let Some(secret) = self.config.credentials().resolve(&binding.credential_name) else {
                return ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "credential {:?} is not available in the credential chain; \
                         refusing to send an unauthenticated write request",
                        binding.credential_name
                    ),
                )
                .with_name(HTTP_WRITE_TOOL_NAME);
            };
            if secret.expose().is_empty() {
                return ToolResult::permanent_error(
                    &call.id,
                    format!(
                        "credential {:?} resolved to an empty value; refusing to send it",
                        binding.credential_name
                    ),
                )
                .with_name(HTTP_WRITE_TOOL_NAME);
            }
            resolved_secret = Some(secret);
        }
        let secret_text: Option<&str> = resolved_secret.as_ref().map(|secret| secret.expose());

        // ③ 请求头：只来自**固定配置** + 鉴权绑定（模型不能自带头）。
        let mut auth_name: Option<String> = None;
        let mut auth_value: Option<String> = None;
        if let (Some(binding), Some(secret)) = (&frozen.credential_binding, secret_text) {
            match binding.scheme {
                AuthScheme::Bearer => {
                    auth_name = Some("authorization".to_string());
                    auth_value = Some(format!("Bearer {secret}"));
                }
                AuthScheme::Raw => {
                    auth_name = Some(binding.header.clone());
                    auth_value = Some(secret.to_string());
                }
            }
        }

        let mut headers: Vec<(&str, &str)> = Vec::with_capacity(3);
        headers.push(("accept", "*/*"));
        headers.push(("content-type", frozen.content_type.as_str()));
        if let (Some(name), Some(value)) = (&auth_name, &auth_value) {
            headers.push((name.as_str(), value.as_str()));
        }

        // ④ 发出去（**唯一**的出网通道）。
        let response = match egress
            .write(frozen.method, &frozen.url, &headers, body_bytes)
            .await
        {
            Ok(response) => response,
            Err(error) => return map_egress_error(&call.id, error),
        };

        // ⑤ 结果面：远端内容一律按**不可信**处理，且**凭据明文不得回显**。
        self.build_result(call, frozen, response, secret_text)
    }

    /// 端点绑定检查（凭据链**参与**绑定时必须命中）。
    ///
    /// 口径与仓内既有语义一致：`bound_endpoints` 返回 `None` = 该解析器
    /// **不参与**绑定（不拦截）；返回 `Some(list)` = 参与 ⇒ 目标 origin 必须在
    /// 表内（表内即**全部**被授权的端点）。
    fn ensure_credential_bound_to_endpoint(
        &self,
        credential_name: &str,
        url: &str,
    ) -> Result<(), String> {
        let Some(list) = self.config.credentials().bound_endpoints(credential_name) else {
            return Ok(());
        };
        let origin = endpoint_origin(url).ok_or_else(|| {
            format!(
                "target url of credential {credential_name:?} has no comparable origin; \
                 refusing to send the credential"
            )
        })?;
        let hit = list
            .iter()
            .filter_map(|raw| endpoint_origin(raw))
            .any(|candidate| candidate == origin);
        if hit {
            Ok(())
        } else {
            Err(format!(
                "credential {credential_name:?} is not bound to endpoint {origin}; \
                 refusing to send the credential to an unbound host"
            ))
        }
    }

    fn build_result(
        &self,
        call: &ToolCall,
        frozen: &HttpWriteFrozenInvocation,
        response: EgressHttpResponse,
        secret_text: Option<&str>,
    ) -> ToolResult {
        let total_bytes = response.body.len();
        let limit = self.config.max_result_bytes().min(total_bytes);
        let truncated = total_bytes > self.config.max_result_bytes();
        let slice = &response.body[..limit];

        let mut leaked_kinds: Vec<LeakedCredentialKind> = Vec::new();
        let mut credential_echo_blocked = false;

        // ① **按值精确遮蔽**：远端把凭据回显进正文时，明文不进模型上下文。
        let mut text: Option<String> = match std::str::from_utf8(slice) {
            Ok(text) => Some(text.to_string()),
            Err(_) => None,
        };
        if let (Some(secret), Some(body)) = (secret_text, text.as_mut()) {
            if !secret.is_empty() && body.contains(secret) {
                credential_echo_blocked = true;
                *body = body.replace(secret, CREDENTIAL_ECHO_MASK);
                leaked_kinds.push(LeakedCredentialKind::GenericBearer);
            }
        }

        // ② URL 也要过形状层（查询串里可能带凭据形状）。
        let url_scan = scan_result_face(&response.final_url);
        leaked_kinds.extend(url_scan.leaked_kinds.iter().copied());
        let url_text = url_scan.text;
        let envelope = envelope_source(&url_text);

        let mut value = serde_json::json!({
            "method": frozen.method.as_str(),
            "url": url_text,
            "status": response.status,
            "content_type": response.content_type,
            "bytes": total_bytes,
            "truncated": truncated,
            "credential_attached": frozen.credential_ref.is_some(),
            "credential_echo_blocked": credential_echo_blocked,
        });

        match text {
            Some(body) => {
                // ③ 形状层扫描（**先扫正文, 再包裹** —— 包裹会改写正文形态）。
                let body_scan = scan_result_face(&body);
                leaked_kinds.extend(body_scan.leaked_kinds.iter().copied());
                // ④ 远端内容一律按「不可信」包裹。
                value["body"] = serde_json::Value::String(UntrustedContentWrapper::wrap(
                    &envelope,
                    &body_scan.text,
                ));
            }
            None => {
                // 二进制正文**不回显**（只报长度与标记）。
                value["body"] = serde_json::Value::String("<binary response body omitted>".into());
                value["body_binary"] = serde_json::Value::Bool(true);
            }
        }

        // ⑤ 命中可观测（零命中 ⇒ 不挂侧车 ⇒ 干净答案的形状与字节都不变）。
        attach_credential_tripwire(&mut value, &leaked_kinds);

        ToolResult::ok(&call.id, value).with_name(HTTP_WRITE_TOOL_NAME)
    }
}

/// 信封的来源标签（引号会被换成单引号 ⇒ 封套边界不被伪造）。
fn envelope_source(url: &str) -> String {
    url.replace('"', "'")
}

/// 把 URL 归一成 **origin**（scheme://host[:port]），用于端点比较。
///
/// * 端口取**已知默认端口**（`https` ⇒ 443）⇒ 两边走**同一套**归一。
/// * 路径 / 查询 / 片段：**丢弃**（比较的是端点，不是资源）。
/// * 无法解析 / 无 host ⇒ `None`（调用方据此 fail-closed）。
fn endpoint_origin(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let url = Url::parse(trimmed).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let port = url.port_or_known_default()?;
    Some(format!(
        "{}://{host}:{port}",
        url.scheme().to_ascii_lowercase()
    ))
}

fn map_egress_error(call_id: &str, error: EgressError) -> ToolResult {
    let retryable = matches!(
        &error,
        EgressError::Timeout
            | EgressError::ConnectionFailed(_)
            | EgressError::ResolutionFailed { .. }
    );
    if retryable {
        ToolResult::retryable_error(call_id, format!("http write failed: {error}"))
            .with_name(HTTP_WRITE_TOOL_NAME)
    } else {
        ToolResult::permanent_error(call_id, format!("http write failed: {error}"))
            .with_name(HTTP_WRITE_TOOL_NAME)
    }
}

#[async_trait]
impl ToolCapability for HttpWriteTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new(HTTP_WRITE_TOOL_NAME)
            .with_description(
                "Sends one bounded, authenticated HTTP write request (post / put / patch / delete) \
                 through Apeireth's policy-enforced controlled egress transport. Only explicitly \
                 configured domains and explicitly configured credential bindings are reachable; \
                 request and response bodies are bounded; redirects are not followed.",
            )
            .with_parameters(Self::declaration_parameters())
    }

    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        let frozen = self.build_frozen(call)?;
        let payload = serde_json::to_value(&frozen).map_err(|e| {
            ToolResult::permanent_error(
                &call.id,
                format!("failed to serialize frozen http write invocation: {e}"),
            )
            .with_name(HTTP_WRITE_TOOL_NAME)
        })?;
        let display = Self::display_invocation(&frozen);
        Ok(Some(FrozenInvocation::new(payload, display)))
    }

    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        let Some(frozen) = frozen else {
            return self.invoke(call).await;
        };

        let write_frozen: HttpWriteFrozenInvocation =
            match serde_json::from_value(frozen.payload.clone()) {
                Ok(write_frozen) => write_frozen,
                Err(e) => {
                    return ToolResult::permanent_error(
                        &call.id,
                        format!("frozen http write invocation is invalid: {e}"),
                    )
                    .with_name(HTTP_WRITE_TOOL_NAME)
                }
            };

        self.execute_frozen(call, &write_frozen).await
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let frozen = match self.build_frozen(call) {
            Ok(frozen) => frozen,
            Err(result) => return result,
        };
        self.execute_frozen(call, &frozen).await
    }
}
