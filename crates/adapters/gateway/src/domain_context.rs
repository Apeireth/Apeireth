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
//!
//! ## V2-c 域面逐面开放 (面板/审批/事件/管理面按域路由补全)
//!
//! 面闸自 W3-①/W3-③ 之后逐面撤除 (演进理由见交付记录):
//!
//! | 面 | 非默认域 | 过滤依据 |
//! |---|---|---|
//! | 会话面板 `/v1/panel/sessions` | 开 | 本域账本直读 (与 `/v1/sessions/*` 同一写闸同一账本) |
//! | 记忆面板 `/v1/panel/memory/episodes`、`/v1/panel/graph` | 开 | 行会话归属 ∈ 本域在册 ([`DomainScope`]) |
//! | 审计面板 `/v1/panel/audit` | 开 | 行归属记号 [`audit_detail_session`] / [`audit_detail_domain`] |
//! | 授权面板 `/v1/panel/grants`、任务面板 `/v1/panel/tasks` | 开 | 同上 (行无归属记号 = 不进域视图, fail-closed) |
//! | 审批面 `/v1/approvals*` | 开 | 归属域校验 (本域在册, 否则 404 同帧) |
//! | 管理面 `/v1/panel/users|domains`、`/v1/admin/users*|domains*` | 开 | **保持全局** (owner 全局视角, 不按域过滤) |
//! | 其余面 (`/v1/models`、traces、tools、记忆写面、授权热撤 …) | 仍拒 | 403 `domain_forbidden`, 帧恒定 |
//!
//! 面板行/审计行的**域归属**与事件面共用同一在册探测 ([`DomainScope::owns`],
//! `SessionManager::contains`): 同一会话的面板行可见 ⇔ 其事件帧投递 (面板订阅
//! 与事件一致)。无归属记号的行不进域视图 (fail-closed, 宁拒不泄)。

use apeireth_core::kernel::SessionId;
use apeireth_core::user_domain::{UserDomain, UserDomainError, DEFAULT_DOMAIN_ID};
use apeireth_runtime::canonical::SessionManager;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;

use crate::client_tokens::ClientIdentity;
use crate::error_frame::{ErrorCode, ErrorFrame};
use crate::panels::GatewayState;

/// 自称域头 (可选): 请求点名自己所在域; 与解析域不一致一律 403 拒
/// (fail-closed, 与 [`crate::client_tokens::CLIENT_NAME_CLAIM_HEADER`] 同族语义)。
pub const DOMAIN_CLAIM_HEADER: &str = "x-apeireth-domain";

/// 按用户域路由的开放面 (前缀匹配): 非默认域只开按域路由的面, 其余面
/// 403 fail-closed (面未按域开放, 逐面开放随星群批; 见交付记录隔离矩阵)。
///
/// - W3-③ 星群正身: 域 chat 面 (`/v1/chat`、`/v1/chat/completions`) 与域事件面
///   (`/v1/apeireth/events`) 开放 —— 走该域自己的伙伴运行时实例, 事件按域过滤;
/// - V2-c 域面逐面开放: 审批面 (`/v1/approvals*`, 归属域校验在处理器) 与
///   users/domains 管理面 (保持全局, owner 全局视角) 开放。
pub const DOMAIN_ROUTED_PATH_PREFIXES: &[&str] = &[
    "/v1/sessions/",
    "/v1/chat",
    "/v1/apeireth/events",
    "/v1/approvals",
    "/v1/panel/users",
    "/v1/panel/domains",
    "/v1/admin/users",
    "/v1/admin/domains",
];

/// 按用户域路由的开放面 (精确匹配): V2-c 面板族逐面开放的**只读**口。
///
/// 精确而非前缀, 是为了不顺带放开同族写口 (`/v1/panel/grants/revoke`、
/// `/v1/panel/tasks/:id/human-judge-retry` 仍按域闸 fail-closed —— 写面随批)。
pub const DOMAIN_ROUTED_EXACT_PATHS: &[&str] = &[
    "/v1/panel/sessions",
    "/v1/panel/memory/episodes",
    "/v1/panel/graph",
    "/v1/panel/audit",
    "/v1/panel/grants",
    "/v1/panel/tasks",
];

/// 该端点是否已按用户域开放 (面闸判定口, 口径唯一)。
pub fn domain_routed_surface(endpoint: &str) -> bool {
    DOMAIN_ROUTED_EXACT_PATHS.contains(&endpoint)
        || DOMAIN_ROUTED_PATH_PREFIXES
            .iter()
            .any(|prefix| endpoint.starts_with(prefix))
}

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

/// 一次请求的域数据范围 (V2-c): 面板过滤与事件过滤**同一在册探测**
/// ([`DomainScope::owns`] = 本域账本 [`SessionManager::contains`]), 面板订阅
/// 与事件面因此对「哪些会话属于本域」永远一致。
#[derive(Clone)]
pub enum DomainScope {
    /// 默认域 / 域面未接线: 不过滤 (兼容路径零变化)。
    Unrestricted,
    /// 非默认域: 只见本域在册会话的行 (在册探测失败按不在册处理, fail-closed)。
    /// `None` = 本域数据面未开 (宁拒不泄, 一切会话行读作不在册)。
    OwnDomain(Option<Arc<SessionManager>>),
}

impl DomainScope {
    /// 是否是非默认域的受限视图 (调用方据此走按域过滤返回)。
    pub fn is_restricted(&self) -> bool {
        matches!(self, Self::OwnDomain(_))
    }

    /// 本域会话写闸 (受限视图的在册探测口; 非受限 = `None`)。
    pub fn manager(&self) -> Option<&Arc<SessionManager>> {
        match self {
            Self::Unrestricted => None,
            Self::OwnDomain(manager) => manager.as_ref(),
        }
    }

    /// 该会话是否归属本域 (在册探测; 失败按不在册处理 —— fail-closed,
    /// 宁拒不泄, 与「他域读作不存在」同一语义)。
    pub async fn owns(&self, session: SessionId) -> bool {
        match self {
            Self::Unrestricted => true,
            Self::OwnDomain(None) => false,
            Self::OwnDomain(Some(manager)) => manager.contains(&session).await.unwrap_or(false),
        }
    }
}

/// 解析一次请求的域数据范围 (面板/审批面过滤口): 域面未接线或默认域 =
/// [`DomainScope::Unrestricted`] (零回归); 非默认域 = 本域在册探测。
///
/// 本域数据面未开 (域解析成功但会话平面缺席) = 403 `domain_refused`
/// (错误即帧, 不给空视图假装「本域无数据」)。
pub fn request_scope(
    state: &GatewayState,
    domain: Option<&DomainContext>,
    endpoint: &str,
) -> Result<DomainScope, Response> {
    let Some(bound) = domain.and_then(|context| context.domain.as_ref()) else {
        return Ok(DomainScope::Unrestricted);
    };
    if bound.is_default() {
        return Ok(DomainScope::Unrestricted);
    }
    let Some(plane) = state.domains.as_ref() else {
        return Ok(DomainScope::Unrestricted);
    };
    let manager = plane
        .sessions(bound)
        .map_err(|error| domain_refused_frame(state, endpoint, &error))?;
    Ok(DomainScope::OwnDomain(Some(Arc::clone(manager))))
}

/// 审计行的会话归属记号 (V2-c 域标注口径): detail 里的 `session=<uuid>` token。
///
/// 网关审计书写约定 (见 [`crate::client_tokens::record_session_client`] 与
/// [`crate::events::RuntimeObservationSink`]): 有会话归属的审计行以
/// `session=<id>` 记号开头或伴随 —— 记号解析失败按无归属处理 (fail-closed)。
pub fn audit_detail_session(detail: &str) -> Option<SessionId> {
    detail
        .split_whitespace()
        .find_map(|token| token.strip_prefix("session="))
        .and_then(|raw| raw.parse::<SessionId>().ok())
}

/// 审计行的显式域标注记号 (V2-c): detail 里的 `domain=<id>` token
/// (管理面动作的目标域标注, 见 [`crate::client_tokens::record_user_action`])。
pub fn audit_detail_domain(detail: &str) -> Option<&str> {
    detail
        .split_whitespace()
        .find_map(|token| token.strip_prefix("domain="))
        .filter(|id| !id.is_empty())
}

/// 授权裁决审计行的**行为域**标注 (V2-c): 行为者所在域 (域引用, 缺省 =
/// user_id 同名域 —— 与域解析链同一缺省语义)。未绑定用户 / 查无记录 /
/// 词面越界 = 不标注 (fail-closed, 未核实值不进审计行)。
pub fn action_domain_label(state: &GatewayState, user_id: Option<&str>) -> Option<String> {
    use apeireth_core::user_domain::DomainUserId;

    let user_id = user_id?;
    let table = state.user_access.table()?;
    let record = table
        .users
        .iter()
        .find(|record| record.user_id == user_id)?;
    let domain = record
        .domain
        .clone()
        .unwrap_or_else(|| record.user_id.clone());
    DomainUserId::new(domain.trim())
        .ok()
        .map(|id| id.as_str().to_string())
}

/// 本域拒开帧 (403 `domain_refused`): 真因入审计, 响应对本域如实。
pub fn domain_refused_frame(
    state: &GatewayState,
    endpoint: &str,
    error: &UserDomainError,
) -> Response {
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

/// 自称域校验 (域级点名): 头 `x-apeireth-domain` 与查询 `?domain=` 同一语义
/// (口径唯一) —— 自称必须与解析域逐字一致, 否则 403 `domain_forbidden`
/// **恒定帧** (目标域存在与否同帧, 不泄露他域存在); 域面未接线时只有默认域
/// 可自称 (无域绑定可校验, 自称即事实)。
pub fn check_domain_claim(
    state: &GatewayState,
    endpoint: &str,
    claimed: &str,
    context: &DomainContext,
) -> Result<(), Response> {
    let claimed_ok = match context.user_id() {
        Some(bound) => claimed == bound,
        // 域面未接线: 只有默认域可自称 (无法开域, 自称即事实)。
        None => claimed == DEFAULT_DOMAIN_ID,
    };
    if claimed_ok {
        return Ok(());
    }
    state.observations.buffer_audit(
        "gateway.domain_refused",
        Some(format!(
            "endpoint={endpoint} reason=domain_claim_mismatch claimed={}",
            crate::canonical_entry::truncate_label(claimed)
        )),
    );
    Err(domain_forbidden_frame(DOMAIN_FORBIDDEN_CLAIM_MESSAGE))
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
        check_domain_claim(state, endpoint, claimed, &context)?;
    }

    // 面闸: 非默认域只开按域路由的面 (V2-c 逐面开放表, 见 [`domain_routed_surface`];
    // 其余面 fail-closed, 不出默认域数据)。
    if context.is_non_default() && !domain_routed_surface(endpoint) {
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

#[cfg(test)]
mod tests {
    use apeireth_runtime::canonical::{InMemorySessionStore, Session};

    use super::*;

    fn manager() -> Arc<SessionManager> {
        Arc::new(SessionManager::new(
            Arc::new(InMemorySessionStore::new()),
            apeireth_core::kernel::system_clock(),
        ))
    }

    /// 面闸逐面开放表: 开的面前缀/精确口放行, 同族写口与其余面仍拒。
    #[test]
    fn the_domain_surface_gate_opens_read_faces_but_keeps_write_faces_closed() {
        for open in [
            "/v1/sessions/abc/settings",
            "/v1/chat",
            "/v1/chat/completions",
            "/v1/apeireth/events",
            "/v1/approvals",
            "/v1/approvals/resolve",
            "/v1/panel/sessions",
            "/v1/panel/memory/episodes",
            "/v1/panel/graph",
            "/v1/panel/audit",
            "/v1/panel/grants",
            "/v1/panel/tasks",
            "/v1/panel/users",
            "/v1/panel/domains",
            "/v1/admin/users",
            "/v1/admin/users/user-a",
            "/v1/admin/domains",
        ] {
            assert!(domain_routed_surface(open), "{open} 应已按域开放");
        }
        for closed in [
            "/v1/models",
            "/v1/panel/traces",
            "/v1/panel/traces/trace-1",
            "/v1/panel/skills",
            "/v1/panel/grants/revoke",
            "/v1/panel/tasks/task-1/human-judge-retry",
            "/v1/memory/append",
            "/v1/apeireth/memory/episodes/ep-1/forget",
            "/v1/admin/config",
            "/v1/admin/shutdown",
            "/v1/tools/list",
        ] {
            assert!(
                !domain_routed_surface(closed),
                "{closed} 不得随读面一起放开"
            );
        }
    }

    /// 审计行归属记号解析 (面板审计面过滤依据): 会话记号/域标注/无归属三分。
    #[test]
    fn audit_row_attribution_reads_the_session_and_domain_tokens() {
        let session = SessionId::new();
        assert_eq!(
            audit_detail_session(&format!("session={session} client=tui action=chat.turn")),
            Some(session)
        );
        assert_eq!(
            audit_detail_session("reason=domain_claim_mismatch claimed=user-bob"),
            None
        );
        assert_eq!(audit_detail_session("session=not-a-uuid"), None);
        assert_eq!(
            audit_detail_domain("user=user-a role=owner domain=user-bob decision=allow"),
            Some("user-bob")
        );
        assert_eq!(audit_detail_domain("domain= decision=allow"), None);
        assert_eq!(audit_detail_domain("user=user-a role=owner"), None);
    }

    /// 在册探测语义: 未接线/默认域放行, 本域在册才可见, 探测失败/未开面 fail-closed。
    #[tokio::test]
    async fn the_shared_scope_probes_domain_ledger_membership() {
        let manager = manager();
        let session = Session::new(
            SessionId::new(),
            apeireth_core::kernel::system_clock().as_ref(),
        );
        manager.save(&session).await.unwrap();
        let foreign = SessionId::new();

        assert!(DomainScope::Unrestricted.owns(foreign).await);
        let scope = DomainScope::OwnDomain(Some(manager));
        assert!(scope.owns(session.id).await);
        assert!(!scope.owns(foreign).await, "他域/不存在会话读作不在册");
        assert!(scope.is_restricted());
        assert!(scope.manager().is_some());

        let empty = DomainScope::OwnDomain(None);
        assert!(
            !empty.owns(session.id).await,
            "本域数据面未开 = 一切会话行读作不在册 (fail-closed)"
        );
        assert!(DomainScope::Unrestricted.manager().is_none());
    }
}
