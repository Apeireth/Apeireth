//! W3-① 域上下文面: 请求 → ClientIdentity → user_id 域解析 → 按域路由 / 越域拒绝即帧。
//!
//! 挂在令牌门 (认人) 与角色门 (判权)**之内**的第三道门: **数据域门** (判域)。
//!
//! ```text
//! 请求 → 令牌门 (ClientIdentity) → 角色门 (UserContext) → 域门 (本模块)
//!      → user_id 域解析 → DomainContext 随请求扩展下行
//!      → 会话面按域路由 / 越域访问拒绝即帧
//! ```
//!
//! ## 域解析链 (口径唯一)
//!
//! 1. **user_id**: 令牌条目的 `user_id` 绑定 (W3-② name→token→user_id 链第一步)
//!    → 用户表 `client_names` 回落 (角色面激活时); 无用户 = 默认域 (单户兼容);
//! 2. **域引用**: 用户表 `UserRecord.domain` (W3-② 只携引用, 本面**判域**) →
//!    缺省 = 与 user_id 同名的域 (`users/<user_id>/`);
//! 3. **域解析** ([`DomainSessionPlane::resolve`]): 未开域的绑定用户留默认域
//!    (兼容延伸); 域配置坏 → 拒 ([`UserDomainError::Refused`], 坏域配置拒开);
//! 4. 域面未接线 (`GatewayState::domains = None`) = 兼容路径: 一切照旧按默认域。
//!
//! ## 越权语义 (404/403 钉死, 不泄露他域存在)
//!
//! | 形态 | 判定 | 帧 |
//! |---|---|---|
//! | 本域资源不存在 / 在他域 (资源级: 会话/记忆/配额 id) | 读作不存在 | 404 `session_not_found` 族, 与真不存在同帧 |
//! | 自称域头 ≠ 本域 (域级点名) | 拒 | 403 `domain_forbidden`, **帧恒定** (目标存在与否同帧) |
//! | 面未按用户域开放 (非默认域) | 拒 | 403 `domain_forbidden`, 帧恒定 |
//! | 本域配置坏/拒开 | 拒 | 403 `domain_refused`, 真因入审计 (本域信息, 不涉他域) |
//!
//! 拒绝帧全部走 [`ErrorFrame`] 正式通道; 真因只进审计 (`gateway.domain_refused`),
//! 不进跨域可观察的响应。

use apeireth_core::user_domain::{UserDomain, UserDomainError, DEFAULT_DOMAIN_ID};
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;

use crate::client_tokens::ClientIdentity;
use crate::error_frame::{ErrorCode, ErrorFrame};
use crate::panels::GatewayState;

/// 自称域头 (可选): 请求点名自己所在域; 与解析域不一致一律 403 拒
/// (fail-closed, 与 [`crate::client_tokens::CLIENT_NAME_CLAIM_HEADER`] 同族语义)。
pub const DOMAIN_CLAIM_HEADER: &str = "x-apeireth-domain";

/// 按用户域路由的开放面 (前缀): 非默认域只开这些面, 其余面本批对非默认域
/// 403 fail-closed (面未按域开放, 逐面开放随星群后续批; 见交付记录隔离矩阵)。
pub const DOMAIN_ROUTED_PATH_PREFIXES: &[&str] = &["/v1/sessions/"];

/// 域级越权帧的恒定文案 (不因目标域存在与否而变 —— 存在性不泄露)。
pub const DOMAIN_FORBIDDEN_CLAIM_MESSAGE: &str =
    "the claimed user domain does not match this credential's domain";

/// 无域绑定时自称域的恒定拒帧文案。
pub const DOMAIN_FORBIDDEN_UNVERIFIABLE_MESSAGE: &str =
    "a claimed user domain requires a domain binding to verify against";

/// 面未按域开放的恒定拒帧文案。
pub const DOMAIN_FORBIDDEN_SURFACE_MESSAGE: &str =
    "this endpoint is not exposed to non-default user domains yet";

/// 请求的用户域上下文 (随请求扩展下行; 会话面据此路由)。
///
/// `domain = None` = 域面未接线 (兼容路径, 数据面仍按默认域原位语义)。
#[derive(Debug, Clone, Default)]
pub struct DomainContext {
    /// 当前请求的用户域。
    pub domain: Option<UserDomain>,
}

impl DomainContext {
    /// 是否是已开域的非默认域 (跨域隔离面对它生效)。
    pub fn is_non_default(&self) -> bool {
        self.domain
            .as_ref()
            .is_some_and(|domain| !domain.is_default())
    }

    /// 当前域 id (域面未接线 = `None`)。
    pub fn user_id(&self) -> Option<&str> {
        self.domain.as_ref().map(|domain| domain.user_id.as_str())
    }
}

/// 域级越权帧 (403 `domain_forbidden`, 恒定文案)。
pub fn domain_forbidden_frame(message: &'static str) -> Response {
    ErrorFrame::response(StatusCode::FORBIDDEN, ErrorCode::DomainForbidden, message)
}

/// 本域拒开帧 (403 `domain_refused`): 真因入审计, 响应对本域如实。
fn domain_refused_frame(state: &GatewayState, endpoint: &str, error: &UserDomainError) -> Response {
    state.observations.buffer_audit(
        "gateway.domain_refused",
        Some(format!(
            "endpoint={endpoint} reason={}",
            crate::canonical_entry::truncate_label(error.reason())
        )),
    );
    ErrorFrame::response(
        StatusCode::FORBIDDEN,
        ErrorCode::DomainRefused,
        "the request's user domain is not open: its domain config refused to load",
    )
}

/// 解析请求的用户域上下文 (链见模块文档)。
///
/// 域面未接线 = [`DomainContext::default`] (兼容路径); 域配置坏/非法绑定 =
/// [`UserDomainError::Refused`] (拒开, 不猜不回落)。
pub fn resolve_domain_context(
    state: &GatewayState,
    client: &str,
) -> Result<DomainContext, UserDomainError> {
    let Some(plane) = state.domains.as_ref() else {
        return Ok(DomainContext::default());
    };
    // user_id: 令牌绑定 (不需要用户表) → 用户表 client_names 回落。
    let bound: Option<String> = state
        .client_tokens
        .entry_for(client)
        .and_then(|spec| spec.user_id.clone())
        .or_else(|| {
            state
                .user_access
                .identity(client, &state.client_tokens)
                .map(|user| user.user_id.clone())
        });
    // 域引用: 用户表 `domain` 字段 (只携引用, 本面判域) → 缺省 = user_id 同名域。
    let domain_ref: Option<String> = bound
        .as_ref()
        .and_then(|user_id| {
            state.user_access.table().and_then(|table| {
                table
                    .users
                    .iter()
                    .find(|record| record.user_id == *user_id)
                    .and_then(|record| record.domain.clone())
            })
        })
        .or(bound);
    let domain = plane.resolve(domain_ref.as_deref())?;
    Ok(DomainContext {
        domain: Some(domain),
    })
}

/// 域门裁决 (整面): 解析域上下文 + 自称域头校验 + 面闸, 过闸后把
/// [`DomainContext`] 插入请求扩展。
///
/// 返回 `Err(帧)` = 请求不得继续; 帧形态见模块文档的越权语义表。
pub fn admit_request_domain(
    state: &GatewayState,
    request: &mut axum::extract::Request,
    client: &str,
    endpoint: &str,
) -> Result<DomainContext, Response> {
    let context = match resolve_domain_context(state, client) {
        Ok(context) => context,
        Err(error) => return Err(domain_refused_frame(state, endpoint, &error)),
    };

    // 自称域头校验 (域级点名): 必须与解析域逐字一致, 否则 403 恒定帧。
    if let Some(claimed) = request
        .headers()
        .get(DOMAIN_CLAIM_HEADER)
        .and_then(|value| value.to_str().ok())
    {
        let claimed_ok = match context.user_id() {
            Some(bound) => claimed == bound,
            // 域面未接线: 只有默认域可自称 (无法开域, 自称即事实)。
            None => claimed == DEFAULT_DOMAIN_ID,
        };
        if !claimed_ok {
            state.observations.buffer_audit(
                "gateway.domain_refused",
                Some(format!(
                    "endpoint={endpoint} reason=domain_claim_mismatch claimed={}",
                    crate::canonical_entry::truncate_label(claimed)
                )),
            );
            return Err(domain_forbidden_frame(DOMAIN_FORBIDDEN_CLAIM_MESSAGE));
        }
    }

    // 面闸: 非默认域只开按域路由的面 (其余面本批 fail-closed, 不出默认域数据)。
    if context.is_non_default()
        && !DOMAIN_ROUTED_PATH_PREFIXES
            .iter()
            .any(|prefix| endpoint.starts_with(prefix))
    {
        state.observations.buffer_audit(
            "gateway.domain_refused",
            Some(format!(
                "endpoint={endpoint} reason=surface_not_domain_routed domain={}",
                context.user_id().unwrap_or("unknown")
            )),
        );
        return Err(domain_forbidden_frame(DOMAIN_FORBIDDEN_SURFACE_MESSAGE));
    }

    request.extensions_mut().insert(context.clone());
    Ok(context)
}

/// 域门中间件 (W3-①, 挂在令牌门与角色门**之内**): 域解析 + 自称域校验 + 面闸,
/// 过闸后 [`DomainContext`] 随请求扩展下行给处理器 (会话面据此按域路由)。
pub async fn require_domain_context(
    State(state): State<GatewayState>,
    mut request: axum::extract::Request,
    next: Next,
) -> Response {
    let endpoint = request.uri().path().to_string();
    let client = request
        .extensions()
        .get::<ClientIdentity>()
        .map(|identity| identity.client.clone())
        .unwrap_or_else(|| ClientIdentity::UNATTRIBUTED.to_string());
    match admit_request_domain(&state, &mut request, &client, &endpoint) {
        Ok(_) => next.run(request).await,
        Err(response) => response,
    }
}
