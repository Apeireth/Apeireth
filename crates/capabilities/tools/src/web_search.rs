//! W1 · C-H4 互联网检索面（能力 id `tool.web_search`，模型面工具名 `web_search`）。
//!
//! # 这个面是什么
//!
//! 对**运维显式配置的检索端点**发起一次查询，把端点返回的检索结果**有界收敛**
//! 后交给模型：每条结果带**来源标注**（URL + 标题），**正文一律进不可信封套**。
//!
//! # 与既有面的边界（不许混）
//!
//! - `tool.search`（[`crate::search`]）= **本地**字面搜索：扫工作区文件名与
//!   文本行，**不出网**。本面 = **互联网**检索：走受控出网打**检索端点**。
//!   两者能力 id 不同、语义不同（本地命中 vs 外部检索结果）。
//! - `tool.fetch`（[`crate::fetch`]）= 打**单个已知 URL** 取原文。本面 = 打
//!   **检索端点**取**多条带来源的结果**。
//!
//! # 端点：只认显式配置（无内置端点 · 无内置账号密钥）
//!
//! **没有任何内置默认端点**，也**没有任何内置账号密钥**。端点由运维经配置面给
//! （env / 生产装配）；**没配 ⇒ 每次调用拒绝即帧**（不静默返回空结果，也不
//! 退化成"随便找个地方搜"）。
//!
//! v1 只发**固定头**（`accept` + 可选的运维固定 `user-agent`），**不注入任何
//! 凭据**（凭据注入面留待后续批次，见文末「未做」）。
//!
//! # 出网：受控出网唯一通道
//!
//! 本文件**不含任何 HTTP 客户端** ⇒ 查询走
//! [`ControlledEgress::get_with_headers`]（与 `tool.fetch` 同一通道）⇒
//! 域名白名单 / SSRF 判定 / DNS 固定 / 重定向逐跳重校验 / 响应体上限 / 超时
//! **全部**由传输层管，本面不重复实现任何一项。
//!
//! # 端点返回的规范线格式（canonical wire contract）
//!
//! ```json
//! { "results": [ { "url": "https://…", "title": "…", "snippet": "…" } ] }
//! ```
//!
//! - `results`：**必需**，数组；缺失或不是数组 ⇒ 整单拒绝（不是"空结果"）。
//! - `url`：**必需**，绝对 `http(s)` URL ⇒ **来源标注**。
//! - `title`：**必需**，非空文本 ⇒ **来源标注**。
//! - `snippet`：可选文本（缺省为空串）；给了但不是字符串 ⇒ 该条按畸形丢弃。
//!
//! # 结果处理：双上限 + 来源标注 + 不可信封套
//!
//! - **条数上限**：端点返回条数 > 本次生效上限 ⇒ **整单拒绝**（宁拒不截）。
//! - **字节上限**：单条超单条字节上限 ⇒ **该条丢弃**（并如实计数）；计入总量
//!   超总量上限 ⇒ **整单拒绝**（宁拒不截）。
//! - **来源标注**：每条必须有 `url` + `title`；**缺任一 ⇒ 该条丢弃**
//!   （**不补一个假来源**，也不把无来源片段塞进模型上下文）。
//! - **不可信封套**：标题与正文都是第三方内容 ⇒ 进模型上下文前**一律**经
//!   [`UntrustedEnvelope`]（固定警告头 + 显式边界 + 边界伪造转义 + 每源预算 +
//!   分级省略）。信封的来源标签就是该条的 `url`。
//!
//! # 口径（0 装诚实）
//!
//! - 本文件 = **实现态（IMPLEMENTED）**；**未接线**：生产装配注册、治理审批档
//!   （R2）、模型面工具登记、env 开关 —— 接线归生产装配线（本文件的写域之外）。
//! - **未做**：凭据注入面（v1 只发固定头）；端点分页；端点侧排序/去重
//!   （沿用端点返回顺序）；结果缓存。
//! - **未实测**：真实互联网端点（仓内无默认端点 ⇒ 无从测；本面只用本地假端点
//!   与纯函数注入测）。

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_orchestration::untrusted_envelope::{
    EnvelopeBudget, EnvelopeCompleteness, UntrustedEnvelope,
};
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;

/// 不可信封套的固定标记（再导出，供集成测试断言信封形态）。
pub use apeireth_orchestration::untrusted_envelope::{
    UNTRUSTED_REFERENCE_BEGIN_TOKEN as ENVELOPE_BEGIN_TOKEN,
    UNTRUSTED_REFERENCE_END_MARKER as ENVELOPE_END_MARKER,
    UNTRUSTED_REFERENCE_WARNING as ENVELOPE_WARNING,
};

use crate::egress::{ControlledEgress, EgressError, EgressPolicy};
use crate::guardrail::{attach_credential_tripwire, scan_result_face, LeakedCredentialKind};

/// 稳定能力标识（注册表 / 治理授权 / 审批档共用）。
pub const WEB_SEARCH_CAPABILITY_ID: &str = "tool.web_search";
/// 模型面工具名（模型在 `ToolCall.name` 里用的名字）。
pub const WEB_SEARCH_TOOL_NAME: &str = "web_search";
/// 冻结调用载荷版本。
pub const WEB_SEARCH_FROZEN_VERSION: u32 = 1;

/// 默认结果条数上限。
pub const DEFAULT_WEB_SEARCH_MAX_RESULTS: usize = 10;
/// 结果条数上限的**硬顶**（配置面也不得越过；模型更只能收窄）。
pub const WEB_SEARCH_MAX_RESULTS_CEILING: usize = 50;
/// 默认单条结果字节上限（`url` + `title` + `snippet` 三者之和）。
pub const DEFAULT_WEB_SEARCH_MAX_ITEM_BYTES: usize = 8 * 1024;
/// 默认结果总字节上限。
pub const DEFAULT_WEB_SEARCH_MAX_TOTAL_BYTES: usize = 64 * 1024;
/// 查询串字节上限。
pub const WEB_SEARCH_QUERY_MAX_BYTES: usize = 512;
/// 端点 URL 字节上限。
pub const WEB_SEARCH_ENDPOINT_MAX_BYTES: usize = 2 * 1024;
/// 默认查询参数名（**不是**默认端点：端点永远没有默认值）。
pub const DEFAULT_WEB_SEARCH_QUERY_PARAM: &str = "q";

/// `tool.web_search` 的可信配置面。
///
/// 端点**只能**由运维显式给出（`None` = 未配置 ⇒ 每次调用拒绝即帧）。
/// 本类型**不含任何默认端点**，也**不含任何内置凭据**。
#[derive(Debug, Clone)]
pub struct WebSearchConfig {
    /// 显式配置的检索端点（`None` = 未配置）。
    endpoint: Option<String>,
    /// 受控出网传输（唯一出网通道）。
    egress: Arc<ControlledEgress>,
    /// 查询参数名（端点把查询放在哪个参数里）。
    query_param: String,
    /// 结果条数上限（配置面；模型只能收窄）。
    max_results: usize,
    /// 单条结果字节上限。
    max_item_bytes: usize,
    /// 结果总字节上限。
    max_total_bytes: usize,
    /// 运维固定 `user-agent`（可选；模型不可改）。
    user_agent: Option<String>,
}

impl WebSearchConfig {
    /// 用**显式给定**的端点构造配置。`endpoint = None` ⇒ 未配置（调用即拒）。
    pub fn new(endpoint: Option<String>, egress: Arc<ControlledEgress>) -> Self {
        Self {
            endpoint,
            egress,
            query_param: DEFAULT_WEB_SEARCH_QUERY_PARAM.to_string(),
            max_results: DEFAULT_WEB_SEARCH_MAX_RESULTS,
            max_item_bytes: DEFAULT_WEB_SEARCH_MAX_ITEM_BYTES,
            max_total_bytes: DEFAULT_WEB_SEARCH_MAX_TOTAL_BYTES,
            user_agent: None,
        }
    }

    /// **未配置**配置面：没有端点 + 全禁出网 ⇒ 任何调用都拒绝即帧。
    pub fn disabled() -> Self {
        Self::new(None, Arc::new(ControlledEgress::new(EgressPolicy::DenyAll)))
    }

    /// 换端点（运维配置面）。
    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// 换查询参数名。
    #[must_use]
    pub fn with_query_param(mut self, query_param: impl Into<String>) -> Self {
        self.query_param = query_param.into();
        self
    }

    /// 换结果条数上限（**收敛**到硬顶；模型侧再收窄）。
    #[must_use]
    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = max_results.min(WEB_SEARCH_MAX_RESULTS_CEILING);
        self
    }

    /// 换单条结果字节上限。
    #[must_use]
    pub fn with_max_item_bytes(mut self, max_item_bytes: usize) -> Self {
        self.max_item_bytes = max_item_bytes;
        self
    }

    /// 换结果总字节上限。
    #[must_use]
    pub fn with_max_total_bytes(mut self, max_total_bytes: usize) -> Self {
        self.max_total_bytes = max_total_bytes;
        self
    }

    /// 设运维固定 `user-agent`（模型不可覆盖）。
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// 已配置的端点（`None` = 未配置）。
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    /// 受控出网传输。
    pub fn egress(&self) -> &Arc<ControlledEgress> {
        &self.egress
    }

    /// 结果条数上限（配置面口径）。
    pub fn max_results(&self) -> usize {
        self.max_results
    }

    /// 单条结果字节上限。
    pub fn max_item_bytes(&self) -> usize {
        self.max_item_bytes
    }

    /// 结果总字节上限。
    pub fn max_total_bytes(&self) -> usize {
        self.max_total_bytes
    }

    /// 固定 `user-agent`（若有）。
    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }
}

/// 工具调用参数（未知字段拒绝）。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WebSearchParams {
    query: String,
    max_results: Option<usize>,
}

/// 冻结调用载荷：版本化的**实际执行形状**。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WebSearchFrozenInvocation {
    version: u32,
    endpoint: String,
    query: String,
    query_param: String,
    max_results: usize,
    max_item_bytes: usize,
    max_total_bytes: usize,
    egress_policy: EgressPolicy,
    timeout_ms: u64,
    max_response_bytes: usize,
    max_redirects: usize,
    user_agent: Option<String>,
}

/// 一条**已通过来源校验**的候选结果。
#[derive(Debug, Clone)]
struct CandidateItem {
    url: String,
    title: String,
    snippet: String,
    bytes: usize,
}

/// 丢弃原因（计数如实回给调用方，不静默丢）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DropReason {
    /// 缺 `url` 或缺 `title` ⇒ **来源缺失**。
    MissingSource,
    /// `url` 不是合法的绝对 `http(s)` URL ⇒ **伪来源**。
    InvalidSource,
    /// 条目不是对象，或 `snippet` 类型不对。
    Malformed,
    /// 单条超字节上限。
    Oversize,
}

/// 被丢弃条目的分因计数。
#[derive(Debug, Default, Clone, Copy)]
struct DroppedCounts {
    missing_source: usize,
    invalid_source: usize,
    malformed: usize,
    oversize: usize,
}

impl DroppedCounts {
    fn bump(&mut self, reason: DropReason) {
        match reason {
            DropReason::MissingSource => self.missing_source += 1,
            DropReason::InvalidSource => self.invalid_source += 1,
            DropReason::Malformed => self.malformed += 1,
            DropReason::Oversize => self.oversize += 1,
        }
    }

    fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "missing_source": self.missing_source,
            "invalid_source": self.invalid_source,
            "malformed": self.malformed,
            "oversize": self.oversize,
        })
    }
}

/// W1 C-H4 互联网检索工具面。
pub struct WebSearchTool {
    id: CapabilityId,
    config: WebSearchConfig,
}

impl WebSearchTool {
    /// 构造检索工具面（端点由 [`WebSearchConfig`] 给）。
    pub fn new(config: WebSearchConfig) -> Self {
        Self {
            id: CapabilityId::new(WEB_SEARCH_CAPABILITY_ID).expect("capability id is valid"),
            config,
        }
    }

    /// 配置面。
    pub fn config(&self) -> &WebSearchConfig {
        &self.config
    }

    fn declaration_parameters() -> ToolParameters {
        let parameters = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": WEB_SEARCH_QUERY_MAX_BYTES,
                    "description": "Natural-language query text sent to the configured retrieval endpoint."
                },
                "max_results": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional result-count cap. It can only narrow the configured cap, never widen it."
                }
            },
            "required": ["query"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(parameters.as_object().cloned().unwrap_or_default());
        params
    }

    fn parse_params(&self, call: &ToolCall) -> Result<WebSearchParams, ToolResult> {
        let object = call.arguments.as_object().ok_or_else(|| {
            ToolResult::permanent_error(&call.id, "web search arguments must be a JSON object")
                .with_name(WEB_SEARCH_TOOL_NAME)
        })?;

        for key in object.keys() {
            if key != "query" && key != "max_results" {
                return Err(ToolResult::permanent_error(
                    &call.id,
                    format!("unknown web search parameter {key:?}; only \"query\" and \"max_results\" are supported"),
                )
                .with_name(WEB_SEARCH_TOOL_NAME));
            }
        }

        serde_json::from_value(call.arguments.clone()).map_err(|e| {
            ToolResult::permanent_error(&call.id, format!("invalid web search parameters: {e}"))
                .with_name(WEB_SEARCH_TOOL_NAME)
        })
    }

    /// 校验并冻结端点（**只认显式配置的端点**；没有默认值可退）。
    fn resolve_endpoint(&self, call: &ToolCall) -> Result<String, ToolResult> {
        let raw = self.config.endpoint.as_ref().ok_or_else(|| {
            ToolResult::permanent_error(
                &call.id,
                "web search endpoint is not configured; refusing to query any default endpoint",
            )
            .with_name(WEB_SEARCH_TOOL_NAME)
        })?;

        if raw.len() > WEB_SEARCH_ENDPOINT_MAX_BYTES {
            return Err(ToolResult::permanent_error(
                &call.id,
                format!(
                    "web search endpoint is {} bytes; the maximum is {WEB_SEARCH_ENDPOINT_MAX_BYTES} bytes",
                    raw.len()
                ),
            )
            .with_name(WEB_SEARCH_TOOL_NAME));
        }

        let url = Url::parse(raw).map_err(|e| {
            ToolResult::permanent_error(
                &call.id,
                format!("web search endpoint is not a valid URL: {e}"),
            )
            .with_name(WEB_SEARCH_TOOL_NAME)
        })?;

        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported web search endpoint scheme {:?}; only http and https are supported",
                    url.scheme()
                ),
            )
            .with_name(WEB_SEARCH_TOOL_NAME));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(ToolResult::permanent_error(
                &call.id,
                "web search endpoint must not carry userinfo",
            )
            .with_name(WEB_SEARCH_TOOL_NAME));
        }
        if url.host_str().is_none() {
            return Err(ToolResult::permanent_error(
                &call.id,
                "web search endpoint must include a host",
            )
            .with_name(WEB_SEARCH_TOOL_NAME));
        }

        let mut url = url;
        url.set_fragment(None);
        Ok(url.to_string())
    }

    fn build_frozen(&self, call: &ToolCall) -> Result<WebSearchFrozenInvocation, ToolResult> {
        let params = self.parse_params(call)?;

        // 入参校验先做（模型自己能改的错，先报给模型）；配置面缺失后报。
        let query = params.query.trim().to_string();
        if query.is_empty() {
            return Err(ToolResult::permanent_error(
                &call.id,
                "web search query must not be empty",
            )
            .with_name(WEB_SEARCH_TOOL_NAME));
        }
        if query.len() > WEB_SEARCH_QUERY_MAX_BYTES {
            return Err(ToolResult::permanent_error(
                &call.id,
                format!(
                    "web search query is {} bytes; the maximum is {WEB_SEARCH_QUERY_MAX_BYTES} bytes",
                    query.len()
                ),
            )
            .with_name(WEB_SEARCH_TOOL_NAME));
        }

        let endpoint = self.resolve_endpoint(call)?;

        // 模型只能**收窄**条数上限，不能放宽（放宽 = 越权扩面）。
        let requested = params.max_results.unwrap_or(self.config.max_results);
        if requested == 0 {
            return Err(ToolResult::permanent_error(
                &call.id,
                "web search max_results must be at least 1",
            )
            .with_name(WEB_SEARCH_TOOL_NAME));
        }
        let max_results = requested.min(self.config.max_results);

        let egress = self.config.egress();
        Ok(WebSearchFrozenInvocation {
            version: WEB_SEARCH_FROZEN_VERSION,
            endpoint,
            query,
            query_param: self.config.query_param.clone(),
            max_results,
            max_item_bytes: self.config.max_item_bytes,
            max_total_bytes: self.config.max_total_bytes,
            egress_policy: egress.policy().clone(),
            timeout_ms: egress.timeout().as_millis() as u64,
            max_response_bytes: egress.max_response_bytes(),
            max_redirects: egress.max_redirects(),
            user_agent: self.config.user_agent().map(ToOwned::to_owned),
        })
    }

    fn display_invocation(frozen: &WebSearchFrozenInvocation) -> serde_json::Value {
        serde_json::json!({
            "version": frozen.version,
            "endpoint": endpoint_label(&frozen.endpoint),
            "query": frozen.query,
            "query_param": frozen.query_param,
            "max_results": frozen.max_results,
            "max_item_bytes": frozen.max_item_bytes,
            "max_total_bytes": frozen.max_total_bytes,
            "egress_policy": format!("{:?}", frozen.egress_policy),
            "timeout_ms": frozen.timeout_ms,
            "max_response_bytes": frozen.max_response_bytes,
            "max_redirects": frozen.max_redirects,
        })
    }

    async fn execute_frozen(
        &self,
        call: &ToolCall,
        frozen: &WebSearchFrozenInvocation,
    ) -> ToolResult {
        if frozen.version != WEB_SEARCH_FROZEN_VERSION {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "unsupported frozen web search invocation version {} (expected {})",
                    frozen.version, WEB_SEARCH_FROZEN_VERSION
                ),
            )
            .with_name(WEB_SEARCH_TOOL_NAME);
        }

        // 传输层按**冻结**的策略与边界重建（与 `tool.fetch` 同一口径）：
        // DNS 不冻结，连接时仍由受控传输解析并复验。
        let egress = ControlledEgress::new(frozen.egress_policy.clone())
            .with_timeout(Duration::from_millis(frozen.timeout_ms))
            .with_max_response_bytes(frozen.max_response_bytes)
            .with_max_redirects(frozen.max_redirects);

        self.execute_with_egress(call, &egress, frozen).await
    }

    async fn execute_with_egress(
        &self,
        call: &ToolCall,
        egress: &ControlledEgress,
        frozen: &WebSearchFrozenInvocation,
    ) -> ToolResult {
        let request_url =
            match build_request_url(&frozen.endpoint, &frozen.query, &frozen.query_param) {
                Ok(url) => url,
                Err(message) => {
                    return ToolResult::permanent_error(&call.id, message)
                        .with_name(WEB_SEARCH_TOOL_NAME)
                }
            };

        let mut headers: Vec<(&str, &str)> = Vec::with_capacity(2);
        headers.push(("accept", "application/json"));
        if let Some(user_agent) = frozen.user_agent.as_deref() {
            headers.push(("user-agent", user_agent));
        }

        let response = match egress
            .get_with_headers(request_url.as_str(), &headers)
            .await
        {
            Ok(response) => response,
            Err(error) => return map_egress_error(&call.id, error),
        };

        if !(200..300).contains(&response.status) {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "web search endpoint returned HTTP {}; refusing to treat it as results",
                    response.status
                ),
            )
            .with_name(WEB_SEARCH_TOOL_NAME);
        }

        Self::build_result(call, frozen, &response.body)
    }

    /// 解析 + 收敛 + 封套 ⇒ 结果面（**纯函数**：无 IO，可直接注入测）。
    fn build_result(
        call: &ToolCall,
        frozen: &WebSearchFrozenInvocation,
        body: &[u8],
    ) -> ToolResult {
        let parsed: serde_json::Value = match serde_json::from_slice(body) {
            Ok(value) => value,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("web search endpoint returned invalid JSON: {e}"),
                )
                .with_name(WEB_SEARCH_TOOL_NAME)
            }
        };

        let items = match parsed.get("results") {
            Some(serde_json::Value::Array(items)) => items,
            Some(_) => {
                return ToolResult::permanent_error(
                    &call.id,
                    "web search endpoint response has a \"results\" field that is not an array",
                )
                .with_name(WEB_SEARCH_TOOL_NAME)
            }
            None => {
                return ToolResult::permanent_error(
                    &call.id,
                    "web search endpoint response has no \"results\" array; refusing to report an empty result",
                )
                .with_name(WEB_SEARCH_TOOL_NAME)
            }
        };

        // **条数上限**：超限 ⇒ 整单拒绝（宁拒不截）。
        if items.len() > frozen.max_results {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "web search endpoint returned {} results; the cap is {}; refusing to truncate",
                    items.len(),
                    frozen.max_results
                ),
            )
            .with_name(WEB_SEARCH_TOOL_NAME);
        }

        let mut admitted: Vec<CandidateItem> = Vec::with_capacity(items.len());
        let mut dropped = DroppedCounts::default();
        let mut total_bytes = 0usize;

        for item in items {
            let candidate = match classify_item(item) {
                Ok(candidate) => candidate,
                Err(reason) => {
                    dropped.bump(reason);
                    continue;
                }
            };
            if candidate.bytes > frozen.max_item_bytes {
                dropped.bump(DropReason::Oversize);
                continue;
            }
            total_bytes = total_bytes.saturating_add(candidate.bytes);
            admitted.push(candidate);
        }

        // **字节上限**：总量超限 ⇒ 整单拒绝（宁拒不截）。
        if total_bytes > frozen.max_total_bytes {
            return ToolResult::permanent_error(
                &call.id,
                format!(
                    "web search results total {total_bytes} bytes; the cap is {} bytes; refusing to truncate",
                    frozen.max_total_bytes
                ),
            )
            .with_name(WEB_SEARCH_TOOL_NAME);
        }

        let envelope_budget = EnvelopeBudget {
            per_source_chars: frozen.max_item_bytes,
        };

        let mut leaked_kinds: Vec<LeakedCredentialKind> = Vec::new();
        let mut results: Vec<serde_json::Value> = Vec::with_capacity(admitted.len());

        for candidate in admitted {
            // ⚠️ **形状扫描在封套之前**（与 `tool.fetch` 同一顺序）：
            // 封套会做边界伪造中和 ⇒ 先扫原文，扫到的才是原文。
            let title_scan = scan_result_face(&candidate.title);
            leaked_kinds.extend(title_scan.leaked_kinds.iter().copied());
            let snippet_scan = scan_result_face(&candidate.snippet);
            leaked_kinds.extend(snippet_scan.leaked_kinds.iter().copied());

            // 标题与正文都是第三方内容 ⇒ **各自**进不可信封套（来源标签 = 该条 URL）。
            let title_text = UntrustedEnvelope::new(
                &candidate.url,
                &title_scan.text,
                EnvelopeCompleteness::Complete,
            )
            .disclose(envelope_budget, None)
            .text;
            let snippet_text = UntrustedEnvelope::new(
                &candidate.url,
                &snippet_scan.text,
                EnvelopeCompleteness::Complete,
            )
            .disclose(envelope_budget, None)
            .text;

            results.push(serde_json::json!({
                "url": candidate.url,
                "title": title_text,
                "snippet": snippet_text,
            }));
        }

        let count = results.len();
        let mut value = serde_json::json!({
            "query": frozen.query,
            "endpoint": endpoint_label(&frozen.endpoint),
            "count": count,
            "results": results,
            "dropped": dropped.to_json(),
            "bounds": {
                "max_results": frozen.max_results,
                "max_item_bytes": frozen.max_item_bytes,
                "max_total_bytes": frozen.max_total_bytes,
            },
        });

        // 命中侧车（零命中 ⇒ 不挂 ⇒ 干净结果的形状不变）。
        attach_credential_tripwire(&mut value, &leaked_kinds);

        ToolResult::ok(&call.id, value).with_name(WEB_SEARCH_TOOL_NAME)
    }
}

/// 校验一条端点结果 ⇒ 候选（**来源缺失 / 伪来源 ⇒ 丢弃**）。
fn classify_item(item: &serde_json::Value) -> Result<CandidateItem, DropReason> {
    let object = item.as_object().ok_or(DropReason::Malformed)?;

    let raw_url = object
        .get("url")
        .and_then(serde_json::Value::as_str)
        .ok_or(DropReason::MissingSource)?;
    let raw_title = object
        .get("title")
        .and_then(serde_json::Value::as_str)
        .ok_or(DropReason::MissingSource)?;

    let title = raw_title.trim();
    if title.is_empty() {
        return Err(DropReason::MissingSource);
    }

    let url = normalize_source_url(raw_url).ok_or(DropReason::InvalidSource)?;

    let snippet = match object.get("snippet") {
        None | Some(serde_json::Value::Null) => String::new(),
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(_) => return Err(DropReason::Malformed),
    };

    let bytes = url.len() + title.len() + snippet.len();
    Ok(CandidateItem {
        url,
        title: title.to_string(),
        snippet,
        bytes,
    })
}

/// 归一化来源 URL：**绝对 http(s)**、无 userinfo、无 fragment。
///
/// 任何不满足的形态 ⇒ `None` ⇒ 该条按**伪来源**丢弃（不补假来源）。
fn normalize_source_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut url = Url::parse(trimmed).ok()?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return None;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    if url.host_str().is_none() {
        return None;
    }
    url.set_fragment(None);
    Some(url.to_string())
}

/// 把查询挂到**已配置**端点上（端点自带查询串时保留它）。
fn build_request_url(endpoint: &str, query: &str, query_param: &str) -> Result<Url, String> {
    let mut url =
        Url::parse(endpoint).map_err(|e| format!("web search endpoint is not a valid URL: {e}"))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(format!(
            "unsupported web search endpoint scheme {:?}; only http and https are supported",
            url.scheme()
        ));
    }
    if url.host_str().is_none() {
        return Err("web search endpoint must include a host".to_string());
    }
    url.set_fragment(None);
    url.query_pairs_mut().append_pair(query_param, query);
    Ok(url)
}

/// 端点的**安全回显**：去掉查询串（端点可能把账号密钥放在查询串里）与
/// fragment ⇒ 密钥不进模型上下文。
fn endpoint_label(endpoint: &str) -> String {
    match Url::parse(endpoint) {
        Ok(mut url) => {
            url.set_query(None);
            url.set_fragment(None);
            url.to_string()
        }
        Err(_) => "<invalid endpoint>".to_string(),
    }
}

fn map_egress_error(call_id: &str, error: EgressError) -> ToolResult {
    let retryable = matches!(
        &error,
        EgressError::Timeout
            | EgressError::ConnectionFailed(_)
            | EgressError::ResolutionFailed { .. }
    );
    if retryable {
        ToolResult::retryable_error(call_id, format!("web search failed: {error}"))
            .with_name(WEB_SEARCH_TOOL_NAME)
    } else {
        ToolResult::permanent_error(call_id, format!("web search failed: {error}"))
            .with_name(WEB_SEARCH_TOOL_NAME)
    }
}

#[async_trait]
impl ToolCapability for WebSearchTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new(WEB_SEARCH_TOOL_NAME)
            .with_description(
                "Queries the explicitly configured retrieval endpoint through Apeireth's \
                 policy-enforced controlled egress transport and returns bounded, \
                 source-annotated results. Every third-party excerpt is disclosed inside an \
                 untrusted envelope; results without a source are dropped, not invented.",
            )
            .with_parameters(Self::declaration_parameters())
    }

    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        let frozen = self.build_frozen(call)?;
        let payload = serde_json::to_value(&frozen).map_err(|e| {
            ToolResult::permanent_error(
                &call.id,
                format!("failed to serialize frozen web search invocation: {e}"),
            )
            .with_name(WEB_SEARCH_TOOL_NAME)
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

        let parsed: WebSearchFrozenInvocation = match serde_json::from_value(frozen.payload.clone())
        {
            Ok(parsed) => parsed,
            Err(e) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("frozen web search invocation is invalid: {e}"),
                )
                .with_name(WEB_SEARCH_TOOL_NAME)
            }
        };

        self.execute_frozen(call, &parsed).await
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let frozen = match self.build_frozen(call) {
            Ok(frozen) => frozen,
            Err(result) => return result,
        };
        self.execute_frozen(call, &frozen).await
    }
}
