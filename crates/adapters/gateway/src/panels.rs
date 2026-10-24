//! Panel introspection surface (`/v1/panel/*`, `/v1/tools/list`, `/v1/apeireth/capabilities`).
//!
//! The gateway owns HTTP shape and transport only; concrete data access is
//! supplied by the composition root through bounded-context ports in
//! [`GatewayServices`]. When no service is configured (e.g. embedded tests), every panel route answers
//! `501 unsupported` with the canonical error body — the frontend treats that
//! as honest degradation, never as a transport failure.
//!
//! Response shapes follow `docs/gateway-api-contract.md` §4-§9 and mirror the
//! desktop types in the sibling `apeireth-ui/src/lib/types.ts` workspace.
//!
//! 读快照语义 (W2-③ 面板一致性, 多客户端读数一致): 每个 `/v1/panel/*` 响应 =
//! **一次快照读** —— 单个端口调用的完整结果, 绝不拼两次读 (会话列表/内存计数
//! 不会混两个时刻的状态); 响应内各字段同出一刻, 多客户端并发读各自拿到完整
//! 一致的快照。该语义由 `tests/multi_client_session.rs` 面板两例钉住。

use crate::principal_forget::{MemoryGovernanceCommand, PrincipalForgetDto};
use std::sync::{Arc, RwLock};

use apeireth_core::kernel::SessionId;
use apeireth_runtime::canonical::{Runtime, Session};
use async_trait::async_trait;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::domain_context::{
    audit_detail_domain, audit_detail_session, request_scope, DomainContext, DomainScope,
};

/// Shared router state: the runtime, optional panel backends, and the
/// gateway-level SSE event bus.
#[derive(Clone)]
pub struct GatewayState {
    pub runtime: Arc<Runtime>,
    pub services: GatewayServices,
    pub events: crate::events::EventBus,
    /// Keepalive pin on the SSE delivery frame surface (逐连接租约的装配面):
    /// the assembly keeps the delivery stream open, so frame retention spans
    /// connection gaps and every new connection reads a current snapshot.
    pub events_stream: Arc<apeireth_core::resource_lease::ResourcePin>,
    pub observations: Arc<crate::events::RuntimeObservationSink>,
    /// Keepalive pin on the observation flush surface (攒批面租约的装配面):
    /// the flush loop runs for the assembly lifetime and stops with it.
    pub observations_stream: Arc<apeireth_core::resource_lease::ResourcePin>,
    /// Presence synthesizer behind `presence_state` frames (contract §8a); held
    /// here so the service outlives the assembly scope and its heartbeat task.
    pub presence: Arc<crate::presence::PresenceService>,
    /// Keepalive pin on the presence frame surface (资源租约示范接线): the
    /// assembly surface is the first holder, so the frame stream (heartbeat
    /// producer) opens here and closes when the gateway state is gone.
    pub presence_stream: Arc<apeireth_core::resource_lease::ResourcePin>,
    /// Live runtime config, readable by every request path and patchable through
    /// `/v1/admin/config`.
    pub hot_config: Arc<RwLock<crate::admin::GatewayRuntimeConfig>>,
    /// W2-② 全 API 面令牌门 (chat / 事件流 / 面板 / 会话 / 审批 / admin 族,
    /// 唯一豁免 `UNAUTHENTICATED_PROBE_PATHS` 健康探活), 三态口径解析自 env
    /// `APEIRETH_GATEWAY_TOKEN` (see [`crate::canonical_entry::GatewayTokenGate`])。
    pub token_gate: crate::canonical_entry::GatewayTokenGate,
    /// 客户端令牌表 (name → token, 令牌族第二面): 与 env 单令牌并列的通行证,
    /// 过门审计带客户端名。由组合根**显式装载注入** (see
    /// [`crate::client_tokens::ClientTokenTable::load`]) —— 装配函数不隐式读
    /// 配置面, 「坏配置拒开」必须在可失败边界执行; 未注入 = 空表 (仅 env 面)。
    pub client_tokens: crate::client_tokens::ClientTokenTable,
    /// 用户角色面 (W3-②): 用户表 (`users.json`; 未注入 = **角色面未激活**,
    /// 兼容路径零回归) + 会话→用户绑定。由组合根显式装载注入 (see
    /// [`crate::client_tokens::UserAccess::load`]), 坏表/绑定不一致在可失败
    /// 边界拒开; 角色门据此按治理层 role 权限矩阵过闸。
    pub user_access: crate::client_tokens::UserAccess,
    /// 用户域会话平面 (W3-① 多户数据隔离): 域解析 + 按域路由会话写闸 (每域
    /// 恰一把, 默认域与运行时共用同一把)。由组合根显式装载注入 (see
    /// [`apeireth_runtime_assembly::DomainDataPlane`]); 未注入 = 域面未接线
    /// (兼容路径, 一切按默认域原位语义)。
    pub domains: Option<Arc<apeireth_runtime::canonical::DomainSessionPlane>>,
    /// 逐域伙伴运行时实例面 (W3-③ 星群正身): 非默认域的对话跑在该域自己的
    /// 伙伴运行时实例上 (模型/记忆/配额/写闸全按域)。由组合根显式装载注入;
    /// 未注入 = 非默认域 chat 面拒开 (403 `domain_refused` fail-closed,
    /// 绝不拿默认域的运行时代跑 —— 代跑会把默认域数据面泄给该域)。
    pub domain_runtimes: Option<Arc<apeireth_runtime::canonical::DomainRuntimePlane>>,
    /// Graceful-shutdown trigger (W2-① resident form): fired by the token-gated
    /// `/v1/admin/shutdown` endpoint; the serve loop stops, writes the closing
    /// audit fact and flushes the observation archive (see
    /// [`crate::canonical_entry::serve_with_shutdown`]).
    pub shutdown: Arc<crate::shutdown::ShutdownTrigger>,
}

impl GatewayState {
    /// W3-③ 星群装配: 把逐域伙伴运行时实例接上与主运行时**同一套**事件汇
    /// (事件帧同管道, 域订阅按域过滤)。组合根注入 [`Self::domain_runtimes`]
    /// 之后调用; 未注入 = 不动 (兼容路径零变化)。
    pub fn attach_domain_runtime_events(&self) {
        let Some(plane) = self.domain_runtimes.as_ref() else {
            return;
        };
        for runtime in plane.instances().values() {
            runtime.add_event_sink(Arc::new(
                apeireth_runtime::canonical::CompositeRuntimeEventSink::new(vec![
                    Arc::new(self.events.clone()),
                    self.observations.clone(),
                    self.presence.clone(),
                ]),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// DTOs (stable contract — do not rename fields without updating the contract doc)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummaryDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: usize,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpanDto {
    pub span_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    pub kind: String,
    pub actor: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummaryDto {
    pub trace_id: String,
    pub span_count: usize,
    pub started_at: i64,
    pub root_span: TraceSpanDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceDetailDto {
    pub trace_id: String,
    pub spans: Vec<TraceSpanDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDto {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args_schema: Option<serde_json::Value>,
    pub source: String,
    pub permission: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditDto {
    pub ts: i64,
    pub event: String,
    pub service: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EpisodeDto {
    pub id: String,
    /// Epoch milliseconds (adapter converts the core seconds representation).
    pub timestamp: i64,
    pub role: String,
    pub content: String,
    pub session_id: String,
    /// Omitted when the backend does not store the field (rc honesty).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EpisodeMutationDto {
    pub ok: bool,
    /// Compatibility alias retained for older desktop clients.
    pub rev: u64,
    pub id: String,
    pub status: String,
    pub protected: bool,
    pub revision: u64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNodeDto {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdgeDto {
    pub from: String,
    pub to: String,
    pub weight: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryGraphDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
}

pub use apeireth_guard::{GuardDryRunRequest, GuardDryRunResponse, GuardEventDto, GuardStatusDto};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchToolExecutionDto {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchMemoryProvenanceDto {
    pub recalled_count: usize,
    pub governance_filtered: usize,
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchTurnDto {
    pub session_id: String,
    pub goal: String,
    pub agent_status: String,
    pub tools: Vec<WorkbenchToolExecutionDto>,
    pub memory: WorkbenchMemoryProvenanceDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard_verdict: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrantDto {
    /// Stable permission label, e.g. `execute_tool:tool.repo`.
    pub permission: String,
    /// Capability id this grant governs, e.g. `tool.repo`.
    pub capability: String,
    /// Omitted: the canonical policy does not timestamp grants (rc honesty).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GrantMutationDto {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrganDto {
    /// Stable organ code, e.g. `W1`.
    pub id: String,
    pub name: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// PanelData — the composition-root contract
// ---------------------------------------------------------------------------

/// Read/write handles for the introspection panels.
///
/// Implementations live in the composition root (the CLI adapts its session
/// store, tool catalog and trace/audit archives). All list methods return
/// newest-first. Errors are rendered as `500 runtime_error` with the message
/// as-is; implementations must not leak secrets into error strings.
#[async_trait]
pub trait PanelData: Send + Sync {
    /// Session summaries, most recently updated first.
    async fn list_sessions(&self) -> Result<Vec<SessionSummaryDto>, String>;

    /// Tool catalog with permission annotations.
    async fn list_tools(&self) -> Result<Vec<ToolDto>, String>;

    /// Recent trace summaries, newest first.
    async fn list_traces(&self, limit: usize) -> Result<Vec<TraceSummaryDto>, String>;

    /// Full span list for one trace, or `None` when unknown.
    async fn trace_detail(&self, trace_id: &str) -> Result<Option<TraceDetailDto>, String>;

    /// Recent audit events, newest first.
    async fn list_audit(&self, limit: usize) -> Result<Vec<AuditDto>, String>;

    /// Best-effort audit append (chat/approval lifecycle). Never fails the turn.
    async fn append_audit(&self, event: &str, detail: Option<&str>);

    /// Best-effort trace archive append after a completed turn.
    async fn append_trace(&self, trace_id: &str, spans: Vec<TraceSpanDto>);

    /// Whether the memory introspection surface is available. Drives the
    /// capability manifest; defaults to `false` so a minimal embedder stays
    /// honest about what it can serve.
    fn supports_memory(&self) -> bool {
        false
    }

    /// Episodes, newest first, optionally filtered by session and/or a
    /// case-insensitive content query. Tombstoned episodes are omitted.
    async fn list_episodes(
        &self,
        session: Option<&str>,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EpisodeDto>, String>;

    /// Append one episode and return its DTO (rev starts at 0).
    async fn append_episode(
        &self,
        session: &str,
        role: &str,
        content: &str,
    ) -> Result<EpisodeDto, String>;

    /// Gateway-level protect/unprotect/forget flags with optimistic revision
    /// checks. `expected_rev` must equal the current revision, otherwise a
    /// conflict error is returned and nothing changes.
    async fn protect_episode(
        &self,
        id: &str,
        expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String>;
    async fn unprotect_episode(
        &self,
        id: &str,
        expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String>;
    async fn forget_episode(
        &self,
        id: &str,
        expected_rev: u64,
        reason: Option<&str>,
    ) -> Result<EpisodeMutationDto, String>;

    /// **按主体全量遗忘**（被遗忘权）：跨 5 账一个事务清完。
    /// 删用户原来**只改用户表** —— 用户的记忆还留着。默认诚实报未接线。
    async fn forget_principal(
        &self,
        principal_id: &str,
        reason: Option<&str>,
    ) -> Result<PrincipalForgetDto, String> {
        crate::principal_forget::forget_principal_unwired(principal_id, reason)
    }

    /// 纠正内容（用户说「你理解错了」）。
    ///
    /// 语义 = 重写**治理视图**内容（原始 append-only episode 行不动），
    /// 乐观并发 `expected_rev` CAS；检索走 override content ⇒ 后续召回不再出现旧结论。
    /// 默认实现诚实报未接线（未注入治理存储的 embedder 不假装支持）。
    async fn update_episode(
        &self,
        _id: &str,
        _content: &str,
        _updated_by: Option<&str>,
        _expected_rev: u64,
    ) -> Result<EpisodeMutationDto, String> {
        Err("memory.update 未接线（本 embedder 未提供纠正面）".to_string())
    }

    /// Memory graph (v1 semantics: session nodes + episode nodes with
    /// containment edges derived from real stored data).
    async fn memory_graph(&self) -> Result<MemoryGraphDto, String>;

    /// Whether the permissions introspection surface (grants list / hot
    /// revoke) is available. Defaults to `false`.
    fn supports_permissions(&self) -> bool {
        false
    }

    /// Current grants, deterministic order.
    async fn list_grants(&self) -> Result<Vec<GrantDto>, String>;

    /// Revoke the grant governing `capability`. Session-scoped hot change:
    /// process restart restores the default policy.
    async fn revoke_grant(&self, capability: &str) -> Result<GrantMutationDto, String>;

    /// Whether an organ catalog is available. Defaults to `false`.
    fn supports_organs(&self) -> bool {
        false
    }

    /// The organ catalog (production default: organs chain disabled).
    async fn list_organs(&self) -> Result<Vec<OrganDto>, String>;
}

// ---------------------------------------------------------------------------
// Bounded-context gateway services
// ---------------------------------------------------------------------------

/// Session read port exposed to the gateway.
#[async_trait]
pub trait SessionQuery: Send + Sync {
    async fn list_sessions(&self) -> Result<Vec<SessionSummaryDto>, String>;
}

/// Memory read port exposed to the gateway.
#[async_trait]
pub trait MemoryQuery: Send + Sync {
    async fn list_episodes(
        &self,
        session: Option<&str>,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EpisodeDto>, String>;
    async fn memory_graph(&self) -> Result<MemoryGraphDto, String>;
}

/// Memory append port.
#[async_trait]
pub trait MemoryCommand: Send + Sync {
    async fn append_episode(
        &self,
        session: &str,
        role: &str,
        content: &str,
    ) -> Result<EpisodeDto, String>;
}

/// Durable memory governance command port.
#[async_trait]

/// Tool catalog read port.
#[async_trait]
pub trait ToolCatalogQuery: Send + Sync {
    async fn list_tools(&self) -> Result<Vec<ToolDto>, String>;
}

/// Trace query port.
#[async_trait]
pub trait TraceQuery: Send + Sync {
    async fn list_traces(&self, limit: usize) -> Result<Vec<TraceSummaryDto>, String>;
    async fn trace_detail(&self, trace_id: &str) -> Result<Option<TraceDetailDto>, String>;
}

/// Trace archive write port.
#[async_trait]
pub trait TraceCommand: Send + Sync {
    async fn append_trace(&self, trace_id: &str, spans: Vec<TraceSpanDto>);
}

/// Audit query and append ports.
#[async_trait]
pub trait AuditQuery: Send + Sync {
    async fn list_audit(&self, limit: usize) -> Result<Vec<AuditDto>, String>;
}

#[async_trait]
pub trait AuditCommand: Send + Sync {
    async fn append_audit(&self, event: &str, detail: Option<&str>);
}

/// Permission/grant query port.
#[async_trait]
pub trait GrantQuery: Send + Sync {
    async fn list_grants(&self) -> Result<Vec<GrantDto>, String>;
}

/// Permission/grant command port.
#[async_trait]
pub trait GrantCommand: Send + Sync {
    async fn revoke_grant(&self, capability: &str) -> Result<GrantMutationDto, String>;
}

/// Behavior/module projection port.
#[async_trait]
pub trait ModuleQuery: Send + Sync {
    async fn list_modules(&self) -> Result<Vec<OrganDto>, String>;
}

/// Safety guard query port.
#[async_trait]
pub trait SafetyGuardQuery: Send + Sync {
    async fn status(&self) -> Result<GuardStatusDto, String>;
    async fn recent_events(&self, limit: usize) -> Result<Vec<GuardEventDto>, String>;
    async fn dry_run(&self, req: GuardDryRunRequest) -> Result<GuardDryRunResponse, String>;
}

#[async_trait]
impl SafetyGuardQuery for apeireth_guard::BehaviorChainGuardHook {
    async fn status(&self) -> Result<GuardStatusDto, String> {
        Ok(self.status())
    }

    async fn recent_events(&self, limit: usize) -> Result<Vec<GuardEventDto>, String> {
        Ok(self.recent_events(Some(limit)))
    }

    async fn dry_run(&self, req: GuardDryRunRequest) -> Result<GuardDryRunResponse, String> {
        Ok(self.dry_run(&req))
    }
}

/// Workbench turn query port.
#[async_trait]
pub trait WorkbenchQuery: Send + Sync {
    async fn turn_status(
        &self,
        session_id: Option<&str>,
    ) -> Result<Option<WorkbenchTurnDto>, String>;
}

/// Gateway service graph. Presence of a port is the capability fact; there is
/// no second `supports_*` feature-bit interface in the production path.
#[derive(Clone, Default)]
pub struct GatewayServices {
    pub sessions: Option<Arc<dyn SessionQuery>>,
    pub memory: Option<Arc<dyn MemoryQuery>>,
    pub memory_commands: Option<Arc<dyn MemoryCommand>>,
    pub memory_governance: Option<Arc<dyn MemoryGovernanceCommand>>,
    pub tools: Option<Arc<dyn ToolCatalogQuery>>,
    pub traces: Option<Arc<dyn TraceQuery>>,
    pub trace_commands: Option<Arc<dyn TraceCommand>>,
    pub audit: Option<Arc<dyn AuditQuery>>,
    pub audit_commands: Option<Arc<dyn AuditCommand>>,
    pub grants: Option<Arc<dyn GrantQuery>>,
    pub grant_commands: Option<Arc<dyn GrantCommand>>,
    pub modules: Option<Arc<dyn ModuleQuery>>,
    /// Writes hot-reloaded API keys to the process credential backend (keyring).
    pub credentials: Option<Arc<dyn crate::admin::CredentialWriter>>,
    pub safety_guard: Option<Arc<dyn SafetyGuardQuery>>,
    pub workbench: Option<Arc<dyn WorkbenchQuery>>,
    /// 用户管理面 (W3-② HTTP 面): 与 CLI `apeireth user` **同一** UserTable 服务
    /// (双入口同源, 见 [`crate::user_admin::UserAdminService`]); 未注入 = 该面
    /// `501 unsupported` (不假装有数据源)。
    pub user_admin: Option<Arc<crate::user_admin::UserAdminService>>,
    /// 域管理面 (W3-① HTTP 面): [`apeireth_core::user_domain::DomainRegistry`] 的
    /// 开域/列域口 (见 [`crate::domain_admin::DomainAdminService`]); 未注入 = `501`。
    pub domain_admin: Option<Arc<crate::domain_admin::DomainAdminService>>,
    /// 技能库只读面 (`GET /v1/panel/skills`): [`crate::skill_shelf::SkillShelfQuery`]
    /// 的元数据扫描 (StoredDoc 拒开语义, 坏卡列拒不砖库; 不回卡体, 卡体走 L2
    /// 封套面)。适配实现在组合根; 未注入 = `501`。
    pub skills: Option<Arc<dyn crate::skill_shelf::SkillShelfQuery>>,
    /// 任务面板面 (`GET /v1/panel/tasks` + `POST /v1/panel/tasks/:id/human-judge-retry`):
    /// [`crate::task_panel::TaskBoardPanel`] 的只读投影 + 人判重试 (failed 只能
    /// 人判; 增删改派走工具面, 不进本面)。适配实现在组合根 (与 `tool.task_board`
    /// 同队同账); 未注入 = `501`。
    pub tasks: Option<Arc<dyn crate::task_panel::TaskBoardPanel>>,
    /// 关怀面板只读面 (`GET /v1/panel/care`): [`crate::care_panel::CarePanelQuery`]
    /// 的 `care_list` 帧同形投影 + 关怀节真值 (含心跳两列; 与 `tool.care_schedule`
    /// 同表同拒开口径)。适配实现在组合根; 未注入 = `501`。
    pub care: Option<Arc<dyn crate::care_panel::CarePanelQuery>>,
    /// 性格面板只读面 (`GET /v1/panel/traits`): [`crate::trait_panel::TraitPanelQuery`]
    /// 的四滑杆现值/锁定态/最近移动投影 (与 `tool.trait_report` 同账同源)。
    /// 适配实现在组合根; 未注入 = `501`。
    pub traits: Option<Arc<dyn crate::trait_panel::TraitPanelQuery>>,
}

impl GatewayServices {
    /// Compatibility bridge for callers still handing the gateway a legacy
    /// panel object. New production composition should populate ports directly.
    pub fn from_panel(panel: Option<Arc<dyn PanelData>>) -> Self {
        let Some(panel) = panel else {
            return Self::default();
        };
        let adapter = Arc::new(PanelDataAdapter {
            inner: panel.clone(),
        });
        let memory = panel.supports_memory();
        let permissions = panel.supports_permissions();
        let modules = panel.supports_organs();
        Self {
            sessions: Some(adapter.clone()),
            memory: memory.then(|| adapter.clone() as Arc<dyn MemoryQuery>),
            memory_commands: memory.then(|| adapter.clone() as Arc<dyn MemoryCommand>),
            memory_governance: memory.then(|| adapter.clone() as Arc<dyn MemoryGovernanceCommand>),
            tools: Some(adapter.clone()),
            traces: Some(adapter.clone()),
            trace_commands: Some(adapter.clone()),
            audit: Some(adapter.clone()),
            audit_commands: Some(adapter.clone()),
            grants: permissions.then(|| adapter.clone() as Arc<dyn GrantQuery>),
            grant_commands: permissions.then(|| adapter.clone() as Arc<dyn GrantCommand>),
            modules: modules.then(|| adapter as Arc<dyn ModuleQuery>),
            credentials: None,
            safety_guard: None,
            workbench: None,
            // 用户/域/技能库/任务面板/关怀/性格面板面走文件事实服务或持久队列
            // (组合根显式注入), 不走旧 PanelData 适配。
            user_admin: None,
            domain_admin: None,
            skills: None,
            tasks: None,
            care: None,
            traits: None,
        }
    }
}

pub(crate) struct PanelDataAdapter {
    pub(crate) inner: Arc<dyn PanelData>,
}

#[async_trait]
impl SessionQuery for PanelDataAdapter {
    async fn list_sessions(&self) -> Result<Vec<SessionSummaryDto>, String> {
        self.inner.list_sessions().await
    }
}

#[async_trait]
impl MemoryQuery for PanelDataAdapter {
    async fn list_episodes(
        &self,
        session: Option<&str>,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EpisodeDto>, String> {
        self.inner.list_episodes(session, query, limit).await
    }

    async fn memory_graph(&self) -> Result<MemoryGraphDto, String> {
        self.inner.memory_graph().await
    }
}

#[async_trait]
impl MemoryCommand for PanelDataAdapter {
    async fn append_episode(
        &self,
        session: &str,
        role: &str,
        content: &str,
    ) -> Result<EpisodeDto, String> {
        self.inner.append_episode(session, role, content).await
    }
}

#[async_trait]
impl ToolCatalogQuery for PanelDataAdapter {
    async fn list_tools(&self) -> Result<Vec<ToolDto>, String> {
        self.inner.list_tools().await
    }
}

#[async_trait]
impl TraceQuery for PanelDataAdapter {
    async fn list_traces(&self, limit: usize) -> Result<Vec<TraceSummaryDto>, String> {
        self.inner.list_traces(limit).await
    }

    async fn trace_detail(&self, trace_id: &str) -> Result<Option<TraceDetailDto>, String> {
        self.inner.trace_detail(trace_id).await
    }
}

#[async_trait]
impl TraceCommand for PanelDataAdapter {
    async fn append_trace(&self, trace_id: &str, spans: Vec<TraceSpanDto>) {
        self.inner.append_trace(trace_id, spans).await
    }
}

#[async_trait]
impl AuditQuery for PanelDataAdapter {
    async fn list_audit(&self, limit: usize) -> Result<Vec<AuditDto>, String> {
        self.inner.list_audit(limit).await
    }
}

#[async_trait]
impl AuditCommand for PanelDataAdapter {
    async fn append_audit(&self, event: &str, detail: Option<&str>) {
        self.inner.append_audit(event, detail).await
    }
}

#[async_trait]
impl GrantQuery for PanelDataAdapter {
    async fn list_grants(&self) -> Result<Vec<GrantDto>, String> {
        self.inner.list_grants().await
    }
}

#[async_trait]
impl GrantCommand for PanelDataAdapter {
    async fn revoke_grant(&self, capability: &str) -> Result<GrantMutationDto, String> {
        self.inner.revoke_grant(capability).await
    }
}

#[async_trait]
impl ModuleQuery for PanelDataAdapter {
    async fn list_modules(&self) -> Result<Vec<OrganDto>, String> {
        self.inner.list_organs().await
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Panel routes without attached state — merge into the canonical router
/// before its final `.with_state(...)`.
/// 面板族里的**管理面**端点 (W3-② 角色门分类用): 管理面读写同属治理层 role 权限
/// 矩阵 [`apeireth_governance::UserCapability::AdminControl`] 档 (owner 专属) ——
/// 授权热撤 (写)、用户管理列表、域列表 (读) 都是管理面数据, 非 owner 一律 403。
/// 面板路径知识归面板面本体 (canonical_entry 只问不背路径, 架构冻结面不许引
/// 面板路径)。
pub fn panel_admin_face(endpoint: &str) -> bool {
    matches!(
        endpoint,
        "/v1/panel/grants/revoke" | "/v1/panel/users" | "/v1/panel/domains"
    )
}

pub fn panel_routes() -> Router<GatewayState> {
    Router::new()
        .route("/v1/panel/sessions", get(list_sessions))
        .route("/v1/panel/traces", get(list_traces))
        .route("/v1/panel/traces/:trace_id", get(trace_detail))
        .route("/v1/tools/list", get(list_tools))
        .route("/v1/panel/audit", get(list_audit))
        // 技能库只读面 (元数据; 不回卡体, 卡体走 L2 封套面), 见 [`crate::skill_shelf`]。
        .route("/v1/panel/skills", get(crate::skill_shelf::list_skills))
        // 关怀面板只读面 (care_list 帧同形 + 心跳两列), 见 [`crate::care_panel`]:
        // **零写面** (改规则走 `tool.care_schedule`, 停人工审批)。
        .route("/v1/panel/care", get(crate::care_panel::list_care))
        // 性格面板只读面 (四滑杆现值/锁定态/最近移动), 见 [`crate::trait_panel`]:
        // **零写面** (锁定/解锁走 CLI 显式动作)。
        .route("/v1/panel/traits", get(crate::trait_panel::list_traits))
        // 任务面板面 (只读 + 人判重试), 见 [`crate::task_panel`]: **无增删改派
        // 路由** (那些是工具面的写面, 面板越权面为零)。
        .route("/v1/panel/tasks", get(crate::task_panel::list_tasks))
        .route(
            "/v1/panel/tasks/:task_id/human-judge-retry",
            axum::routing::post(crate::task_panel::human_judge_retry),
        )
        .route("/v1/apeireth/capabilities", get(capabilities))
        // V2-b 用户/域管理面 (owner 档, 见 [`panel_admin_face`]): 列表两口在面板族,
        // 增删改口在 canonical_entry 的 admin 族 (同一 UserTable 服务, 双入口同源)。
        .route("/v1/panel/users", get(crate::user_admin::list_users))
        .route("/v1/panel/domains", get(crate::domain_admin::list_domains))
        .route("/v1/panel/memory/episodes", get(list_episodes))
        .route("/v1/memory/append", axum::routing::post(append_episode))
        .route(
            "/v1/apeireth/memory/episodes/:id/forget",
            axum::routing::post(forget_episode),
        )
        .route(
            "/v1/apeireth/memory/episodes/:id/protect",
            axum::routing::post(protect_episode),
        )
        .route(
            "/v1/apeireth/memory/episodes/:id/unprotect",
            axum::routing::post(unprotect_episode),
        )
        // 纠正（用户说「你理解错了」）：重写治理视图内容，原始 append-only 行不动。
        .route(
            "/v1/apeireth/memory/episodes/:id/content",
            axum::routing::put(crate::panel_memory_update::update_episode),
        )
        .route("/v1/panel/graph", get(memory_graph))
        .route("/v1/panel/grants", get(list_grants))
        .route("/v1/panel/grants/revoke", axum::routing::post(revoke_grant))
        .route("/v1/organs", get(list_organs))
        .route("/v1/modules", get(list_modules))
        .route("/v1/safety/guard/status", get(guard_status))
        .route("/v1/panel/safety/guard/status", get(guard_status))
        .route("/v1/safety/guard/events", get(guard_events))
        .route("/v1/panel/safety/guard/events", get(guard_events))
        .route(
            "/v1/safety/guard/evaluate",
            axum::routing::post(guard_evaluate),
        )
        .route("/v1/workbench/turn", get(workbench_turn))
        .route("/v1/panel/workbench/turn", get(workbench_turn))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn unsupported(what: &str) -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": {
                "code": "unsupported",
                "message": format!("{what} 不支持: 当前运行时未实现该内省 API (Apeireth 2.0 canonical gateway)"),
                "solution": "该内省能力当前不可用, 请升级 Companion 后重试"
            }
        })),
    )
        .into_response()
}

/// 管理面/面板族通用错误帧 (`{"error": {code, message, solution}}`)。
pub(crate) fn admin_error(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
    solution: &str,
) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": { "code": code, "message": message.into(), "solution": solution }
        })),
    )
        .into_response()
}

/// 面板族运行期错误帧 (`500 runtime_error`); 技能库等旁路面板模块同用。
pub(crate) fn panel_error(message: String) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": { "code": "runtime_error", "message": message, "solution": "重启 Companion 后重试, 若复现请提交日志" } })),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// V2-c 面板按域过滤 (非默认域 = 按域过滤返回; 默认域/域面未接线 = 零回归)
// ---------------------------------------------------------------------------
//
// 行的**域归属**依据 (口径唯一, 与事件面同一在册探测 [`DomainScope::owns`]):
// 1. 行内会话 id (会话行 `id` / 记忆行 `session_id` / 图节点 `session:<id>` 约定)
//    → 本域账本在册探测;
// 2. 审计行归属记号 (`session=<id>` / `domain=<id>`, 见 [`audit_detail_session`] /
//    [`audit_detail_domain`]);
// 3. 两者皆无 = 无归属记号的行 → **不进域视图** (fail-closed, 宁拒不泄)。
// 默认域 (owner 全局视角) / 域面未接线 = 全量返回 (兼容路径逐字零变化)。

/// 会话摘要投影 (本域账本直读面): 与组合根会话投影同形 (一次快照读)。
fn session_summary(session: &Session) -> SessionSummaryDto {
    SessionSummaryDto {
        id: session.id.to_string(),
        title: session_title(session),
        created_at: session.created_at.epoch_millis(),
        updated_at: session.updated_at.epoch_millis(),
        message_count: session.messages.len(),
        revision: session.revision,
    }
}

/// 会话标题 = 首条用户消息的有界摘录 (无用户消息 = 无标题, 不编造)。
fn session_title(session: &Session) -> Option<String> {
    use apeireth_protocol::canonical::{ContentPart, MessageRole};

    session
        .messages
        .iter()
        .find(|message| message.role == MessageRole::User)
        .map(|message| ContentPart::join_text(&message.content))
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .map(|text| {
            const TITLE_CHARS: usize = 40;
            let mut out: String = text.chars().take(TITLE_CHARS).collect();
            if text.chars().count() > TITLE_CHARS {
                out.push('…');
            }
            out
        })
}

/// 记忆行按域过滤: 行会话归属 ∈ 本域在册才进域视图。
async fn filter_episodes(scope: &DomainScope, episodes: Vec<EpisodeDto>) -> Vec<EpisodeDto> {
    if !scope.is_restricted() {
        return episodes;
    }
    let mut kept = Vec::new();
    for row in episodes {
        let owned = match row.session_id.parse::<SessionId>() {
            Ok(session) => scope.owns(session).await,
            Err(_) => false,
        };
        if owned {
            kept.push(row);
        }
    }
    kept
}

/// 记忆图按域过滤: 会话节点 (`session:<id>` 投影约定) 在册才留, 情节节点随
/// 本域会话的包含边留下; 无归属节点/边不进域视图 (fail-closed)。
async fn filter_memory_graph(scope: &DomainScope, graph: MemoryGraphDto) -> MemoryGraphDto {
    if !scope.is_restricted() {
        return graph;
    }
    let MemoryGraphDto { nodes, edges } = graph;
    let mut kept_ids: Vec<String> = Vec::new();
    for node in &nodes {
        let Some(raw) = node.id.strip_prefix("session:") else {
            continue;
        };
        let owned = match raw.parse::<SessionId>() {
            Ok(session) => scope.owns(session).await,
            Err(_) => false,
        };
        if owned {
            kept_ids.push(node.id.clone());
        }
    }
    let kept_edges: Vec<GraphEdgeDto> = edges
        .into_iter()
        .filter(|edge| {
            if kept_ids.contains(&edge.from) {
                if !kept_ids.contains(&edge.to) {
                    kept_ids.push(edge.to.clone());
                }
                true
            } else {
                false
            }
        })
        .collect();
    let kept_nodes = nodes
        .into_iter()
        .filter(|node| kept_ids.contains(&node.id))
        .collect();
    MemoryGraphDto {
        nodes: kept_nodes,
        edges: kept_edges,
    }
}

/// 审计行按域过滤: 会话记号 → 在册探测; `domain=` 标注 → 与本域比对;
/// 无归属记号 → 不进域视图 (fail-closed)。
async fn filter_audit(
    scope: &DomainScope,
    domain_id: Option<&str>,
    events: Vec<AuditDto>,
) -> Vec<AuditDto> {
    if !scope.is_restricted() {
        return events;
    }
    let mut kept = Vec::new();
    for row in events {
        let detail = row.detail.as_deref().unwrap_or("");
        let visible = if let Some(session) = audit_detail_session(detail) {
            scope.owns(session).await
        } else {
            audit_detail_domain(detail).is_some_and(|claimed| Some(claimed) == domain_id)
        };
        if visible {
            kept.push(row);
        }
    }
    kept
}

/// 授权行按域过滤: 授权行 (permission/capability) 无域归属记号 → 域视图为空
/// (fail-closed; 全局治理口径行的归属标注随后续批, 补齐即按上表生效)。
fn filter_grants(scope: &DomainScope, grants: Vec<GrantDto>) -> Vec<GrantDto> {
    if scope.is_restricted() {
        return Vec::new();
    }
    grants
}

fn limit_of(query: Option<usize>) -> usize {
    query.unwrap_or(50).clamp(1, 500)
}

#[derive(Debug, Default, Deserialize)]
struct LimitQuery {
    limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn list_sessions(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
) -> Response {
    let scope = match request_scope(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/panel/sessions",
    ) {
        Ok(scope) => scope,
        Err(refusal) => return refusal,
    };
    // V2-c 非默认域: 本域账本直读 (与 `/v1/sessions/*` 同一写闸同一账本,
    // 一次快照读) —— 他域行物理不在本域库, 计数天然隔离。
    if let Some(manager) = scope.manager() {
        return match manager.store().list().await {
            Ok(rows) => {
                let sessions = rows.iter().map(session_summary).collect::<Vec<_>>();
                (
                    StatusCode::OK,
                    Json(serde_json::json!({ "sessions": sessions })),
                )
                    .into_response()
            }
            Err(error) => panel_error(error.to_string()),
        };
    }
    let Some(sessions) = &state.services.sessions else {
        return unsupported("sessions.read");
    };
    match sessions.list_sessions().await {
        Ok(sessions) => (
            StatusCode::OK,
            Json(serde_json::json!({ "sessions": sessions })),
        )
            .into_response(),
        Err(e) => panel_error(e),
    }
}

async fn list_tools(State(state): State<GatewayState>) -> Response {
    let Some(tools) = &state.services.tools else {
        return unsupported("tools.list");
    };
    match tools.list_tools().await {
        Ok(tools) => (StatusCode::OK, Json(serde_json::json!({ "tools": tools }))).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn list_traces(State(state): State<GatewayState>, Query(q): Query<LimitQuery>) -> Response {
    let Some(traces) = &state.services.traces else {
        return unsupported("trace.read");
    };
    match traces.list_traces(limit_of(q.limit)).await {
        Ok(traces) => (
            StatusCode::OK,
            Json(serde_json::json!({ "traces": traces })),
        )
            .into_response(),
        Err(e) => panel_error(e),
    }
}

async fn trace_detail(State(state): State<GatewayState>, Path(trace_id): Path<String>) -> Response {
    let Some(traces) = &state.services.traces else {
        return unsupported("trace.read");
    };
    match traces.trace_detail(&trace_id).await {
        Ok(Some(detail)) => (StatusCode::OK, Json(detail)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": { "code": "not_found", "message": format!("trace {trace_id} not found"), "solution": "回到列表重新选择" } })),
        )
            .into_response(),
        Err(e) => panel_error(e),
    }
}

async fn list_audit(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    Query(q): Query<LimitQuery>,
) -> Response {
    let context = domain.as_ref().map(|Extension(context)| context);
    let scope = match request_scope(&state, context, "/v1/panel/audit") {
        Ok(scope) => scope,
        Err(refusal) => return refusal,
    };
    let Some(audit) = &state.services.audit else {
        return unsupported("audit.read");
    };
    match audit.list_audit(limit_of(q.limit)).await {
        Ok(events) => {
            let events = filter_audit(
                &scope,
                context.and_then(|context| context.user_id()),
                events,
            )
            .await;
            (
                StatusCode::OK,
                Json(serde_json::json!({ "events": events })),
            )
                .into_response()
        }
        Err(e) => panel_error(e),
    }
}

async fn guard_status(State(state): State<GatewayState>) -> Response {
    let Some(guard) = &state.services.safety_guard else {
        return unsupported("safety.guard.status.read");
    };
    match guard.status().await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn guard_events(State(state): State<GatewayState>, Query(q): Query<LimitQuery>) -> Response {
    let Some(guard) = &state.services.safety_guard else {
        return unsupported("safety.guard.events.read");
    };
    match guard.recent_events(limit_of(q.limit)).await {
        Ok(events) => (
            StatusCode::OK,
            Json(serde_json::json!({ "events": events })),
        )
            .into_response(),
        Err(e) => panel_error(e),
    }
}

async fn guard_evaluate(
    State(state): State<GatewayState>,
    Json(request): Json<GuardDryRunRequest>,
) -> Response {
    let Some(guard) = &state.services.safety_guard else {
        return unsupported("safety.guard.evaluate");
    };
    match guard.dry_run(request).await {
        Ok(res) => (StatusCode::OK, Json(res)).into_response(),
        Err(e) => panel_error(e),
    }
}

#[derive(Debug, Default, Deserialize)]
struct WorkbenchQueryParam {
    session: Option<String>,
}

async fn workbench_turn(
    State(state): State<GatewayState>,
    Query(q): Query<WorkbenchQueryParam>,
) -> Response {
    let Some(workbench) = &state.services.workbench else {
        return unsupported("workbench.turn.read");
    };
    match workbench.turn_status(q.session.as_deref()).await {
        Ok(turn) => (StatusCode::OK, Json(serde_json::json!({ "turn": turn }))).into_response(),
        Err(e) => panel_error(e),
    }
}

// ---------------------------------------------------------------------------
// Memory introspection
// ---------------------------------------------------------------------------

fn memory_unavailable(state: &GatewayState) -> Option<Response> {
    if state.services.memory.is_none() {
        return Some(unsupported("memory.read"));
    }
    None
}

fn memory_write_unavailable(state: &GatewayState) -> Option<Response> {
    if state.services.memory_commands.is_none() {
        return Some(unsupported("memory.write"));
    }
    None
}

pub(crate) fn memory_governance_unavailable(state: &GatewayState) -> Option<Response> {
    if state.services.memory_governance.is_none() {
        return Some(unsupported("memory.governance"));
    }
    None
}

#[derive(Debug, Default, Deserialize)]
struct EpisodeQuery {
    limit: Option<usize>,
    q: Option<String>,
    session: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EpisodeAppendRequest {
    session: String,
    content: String,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EpisodeMutationRequest {
    expected_rev: u64,
    #[serde(default)]
    reason: Option<String>,
}

async fn list_episodes(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    Query(q): Query<EpisodeQuery>,
) -> Response {
    let scope = match request_scope(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/panel/memory/episodes",
    ) {
        Ok(scope) => scope,
        Err(refusal) => return refusal,
    };
    if let Some(response) = memory_unavailable(&state) {
        return response;
    }
    let memory = state.services.memory.as_ref().expect("checked above");
    match memory
        .list_episodes(q.session.as_deref(), q.q.as_deref(), limit_of(q.limit))
        .await
    {
        Ok(episodes) => {
            let episodes = filter_episodes(&scope, episodes).await;
            (
                StatusCode::OK,
                Json(serde_json::json!({ "episodes": episodes })),
            )
                .into_response()
        }
        Err(e) => panel_error(e),
    }
}

async fn append_episode(
    State(state): State<GatewayState>,
    Json(request): Json<EpisodeAppendRequest>,
) -> Response {
    if let Some(response) = memory_write_unavailable(&state) {
        return response;
    }
    if request.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": { "code": "invalid_request", "message": "content must not be empty", "solution": "检查参数后重试" } })),
        )
            .into_response();
    }
    let memory = state
        .services
        .memory_commands
        .as_ref()
        .expect("checked above");
    let role = request.role.as_deref().unwrap_or("user");
    match memory
        .append_episode(&request.session, role, &request.content)
        .await
    {
        Ok(episode) => (StatusCode::CREATED, Json(episode)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn protect_episode(
    State(state): State<GatewayState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<EpisodeMutationRequest>,
) -> Response {
    if let Some(response) = memory_governance_unavailable(&state) {
        return response;
    }
    let memory = state
        .services
        .memory_governance
        .as_ref()
        .expect("checked above");
    match memory.protect_episode(&id, request.expected_rev).await {
        Ok(mutation) => (StatusCode::OK, Json(mutation)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn unprotect_episode(
    State(state): State<GatewayState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<EpisodeMutationRequest>,
) -> Response {
    if let Some(response) = memory_governance_unavailable(&state) {
        return response;
    }
    let memory = state
        .services
        .memory_governance
        .as_ref()
        .expect("checked above");
    match memory.unprotect_episode(&id, request.expected_rev).await {
        Ok(mutation) => (StatusCode::OK, Json(mutation)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn forget_episode(
    State(state): State<GatewayState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<EpisodeMutationRequest>,
) -> Response {
    if let Some(response) = memory_governance_unavailable(&state) {
        return response;
    }
    let memory = state
        .services
        .memory_governance
        .as_ref()
        .expect("checked above");
    match memory
        .forget_episode(&id, request.expected_rev, request.reason.as_deref())
        .await
    {
        Ok(mutation) => (StatusCode::OK, Json(mutation)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn memory_graph(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
) -> Response {
    let scope = match request_scope(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/panel/graph",
    ) {
        Ok(scope) => scope,
        Err(refusal) => return refusal,
    };
    if let Some(response) = memory_unavailable(&state) {
        return response;
    }
    let memory = state.services.memory.as_ref().expect("checked above");
    match memory.memory_graph().await {
        Ok(graph) => {
            let graph = filter_memory_graph(&scope, graph).await;
            (StatusCode::OK, Json(graph)).into_response()
        }
        Err(e) => panel_error(e),
    }
}

// ---------------------------------------------------------------------------
// Permissions (grants / hot revoke) and organs
// ---------------------------------------------------------------------------

fn permissions_unavailable(state: &GatewayState) -> Option<Response> {
    if state.services.grants.is_none() {
        return Some(unsupported("permissions.grants.read"));
    }
    None
}

#[derive(Debug, Deserialize)]
struct RevokeGrantRequest {
    capability: String,
}

async fn list_grants(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
) -> Response {
    let scope = match request_scope(
        &state,
        domain.as_ref().map(|Extension(context)| context),
        "/v1/panel/grants",
    ) {
        Ok(scope) => scope,
        Err(refusal) => return refusal,
    };
    if let Some(response) = permissions_unavailable(&state) {
        return response;
    }
    let grants = state.services.grants.as_ref().expect("checked above");
    match grants.list_grants().await {
        Ok(grants) => {
            let grants = filter_grants(&scope, grants);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "grants": grants })),
            )
                .into_response()
        }
        Err(e) => panel_error(e),
    }
}

async fn revoke_grant(
    State(state): State<GatewayState>,
    Json(request): Json<RevokeGrantRequest>,
) -> Response {
    if let Some(response) = permissions_unavailable(&state) {
        return response;
    }
    let Some(commands) = &state.services.grant_commands else {
        return unsupported("permissions.revoke");
    };
    match commands.revoke_grant(&request.capability).await {
        Ok(mutation) => (StatusCode::OK, Json(mutation)).into_response(),
        Err(e) => panel_error(e),
    }
}

async fn list_organs(State(state): State<GatewayState>) -> Response {
    let Some(modules) = &state.services.modules else {
        return unsupported("organs.list");
    };
    match modules.list_modules().await {
        Ok(organs) => (
            StatusCode::OK,
            Json(serde_json::json!({ "organs": organs })),
        )
            .into_response(),
        Err(e) => panel_error(e),
    }
}

async fn list_modules(State(state): State<GatewayState>) -> Response {
    let Some(modules) = &state.services.modules else {
        return unsupported("modules.list");
    };
    match modules.list_modules().await {
        Ok(modules) => (
            StatusCode::OK,
            Json(serde_json::json!({ "modules": modules })),
        )
            .into_response(),
        Err(e) => panel_error(e),
    }
}

// ---------------------------------------------------------------------------
// Capability manifest
// ---------------------------------------------------------------------------

fn cap(
    id: &str,
    read: bool,
    write: bool,
    ops: &[&str],
    supported: bool,
    available: bool,
) -> serde_json::Value {
    cap_with_reason(id, read, write, ops, supported, available, None)
}

fn cap_with_reason(
    id: &str,
    read: bool,
    write: bool,
    ops: &[&str],
    supported: bool,
    available: bool,
    reason: Option<&str>,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "id": id,
        "supported": supported,
        "read": read,
        "write": write,
        "version": 1,
        "operations": ops,
        "available": available,
    });
    if !available {
        value["reason"] = serde_json::json!(reason.unwrap_or(if supported {
            "provider_not_configured"
        } else {
            "platform_unsupported"
        }));
    }
    value
}

fn cap_alias(
    id: &str,
    alias_of: &str,
    read: bool,
    write: bool,
    ops: &[&str],
    supported: bool,
    available: bool,
) -> serde_json::Value {
    let mut value = cap(id, read, write, ops, supported, available);
    value["alias_of"] = serde_json::json!(alias_of);
    value
}

async fn capabilities(State(state): State<GatewayState>) -> Response {
    let services = &state.services;
    let sessions_supported = services.sessions.is_some();
    let memory_supported = services.memory.is_some();
    let memory_write_supported = services.memory_commands.is_some();
    let memory_governance_supported = services.memory_governance.is_some();
    let tools_supported = services.tools.is_some();
    let trace_supported = services.traces.is_some();
    let audit_supported = services.audit.is_some();
    let permissions_supported = services.grants.is_some();
    let permissions_write_supported = services.grant_commands.is_some();
    let organs_supported = services.modules.is_some();
    let safety_supported = services.safety_guard.is_some();
    let workbench_supported = services.workbench.is_some();
    let user_admin_supported = services.user_admin.is_some();
    let domain_admin_supported = services.domain_admin.is_some();
    let skills_supported = services.skills.is_some();
    let tasks_supported = services.tasks.is_some();
    let care_supported = services.care.is_some();
    let traits_supported = services.traits.is_some();

    let memory_ids = [
        ("memory.read", true, false, &["list", "read"] as &[&str]),
        ("memory.write", false, true, &["append"] as &[&str]),
        ("memory.update", false, true, &["update"] as &[&str]),
        ("memory.forget", false, true, &["forget"] as &[&str]),
        ("memory.protect", false, true, &["protect"] as &[&str]),
        ("memory.unprotect", false, true, &["unprotect"] as &[&str]),
    ];
    let memory = memory_ids
        .iter()
        .map(|(id, read, write, operations)| {
            let (supported, available) = match *id {
                "memory.read" => (memory_supported, memory_supported),
                "memory.write" => (memory_write_supported, memory_write_supported),
                // 纠正与治理面同源：注入 MemoryGovernanceStore 即可用
                // （存储侧 `update_episode_content` 早已实现，此前只是面板没接电）。
                _ => (memory_governance_supported, memory_governance_supported),
            };
            cap(id, *read, *write, operations, supported, available)
        })
        .chain(std::iter::once(cap(
            "memory.graph.read",
            true,
            false,
            &["graph"],
            memory_supported,
            memory_supported,
        )))
        .collect::<Vec<_>>();

    let manifest = serde_json::json!({
        "schema_version": 1,
        "runtime": { "service": "apeireth-gateway-2.0", "version": env!("CARGO_PKG_VERSION") },
        "capabilities": [
            { "name": "health", "capabilities": [ cap("health", true, false, &["check"], true, true) ] },
            { "name": "models", "capabilities": [ cap("models.list", true, false, &["list"], true, !state.runtime.providers().is_empty()) ] },
            { "name": "providers", "capabilities": [ cap("providers.list", true, false, &["list"], true, !state.runtime.providers().is_empty()) ] },
            { "name": "runtime", "capabilities": [ cap("runtime.snapshot.read", true, false, &["read"], true, true) ] },
            { "name": "chat", "capabilities": [ cap("chat.completions", true, true, &["complete", "stream"], true, !state.runtime.providers().is_empty()) ] },
            { "name": "sessions", "capabilities": [
                cap("sessions.read", true, false, &["list"], sessions_supported, sessions_supported),
                cap("sessions.delete", false, true, &["delete"], true, true),
            ] },
            { "name": "memory", "capabilities": memory },
            { "name": "tools", "capabilities": [ cap("tools.list", true, false, &["list"], tools_supported, tools_supported) ] },
            { "name": "skills", "capabilities": [ cap("skills.read", true, false, &["list"], skills_supported, skills_supported) ] },
            { "name": "care", "capabilities": [ cap("care.read", true, false, &["list"], care_supported, care_supported) ] },
            { "name": "traits", "capabilities": [ cap("traits.read", true, false, &["read"], traits_supported, traits_supported) ] },
            { "name": "tasks", "capabilities": [
                cap("tasks.read", true, false, &["list"], tasks_supported, tasks_supported),
                cap("tasks.human_judge_retry", false, true, &["retry"], tasks_supported, tasks_supported),
            ] },
            { "name": "permissions", "capabilities": [
                cap("permissions.approval.read", true, false, &["list"], true, true),
                cap("permissions.approval.resolve", false, true, &["resolve"], true, true),
                cap_alias("approvals.read", "permissions.approval.read", true, false, &["list"], true, true),
                cap_alias("approvals.resolve", "permissions.approval.resolve", false, true, &["resolve"], true, true),
                cap("permissions.grants.read", true, false, &["list"], permissions_supported, permissions_supported),
                cap("permissions.revoke", false, true, &["revoke"], permissions_write_supported, permissions_write_supported),
            ] },
            { "name": "organs", "capabilities": [
                cap("organs.list", true, false, &["list"], organs_supported, organs_supported),
                cap("modules.list", true, false, &["list"], organs_supported, organs_supported),
            ] },
            { "name": "safety", "capabilities": [
                cap("safety.guard.status.read", true, false, &["read"], safety_supported, safety_supported),
                cap("safety.guard.events.read", true, false, &["list"], safety_supported, safety_supported),
                cap("safety.guard.evaluate", false, true, &["evaluate"], safety_supported, safety_supported),
            ] },
            { "name": "workbench", "capabilities": [
                cap("workbench.turn.read", true, false, &["read"], workbench_supported, workbench_supported),
            ] },
            { "name": "metrics", "capabilities": [
                cap("metrics.read", true, false, &["read"], true, true),
            ] },
            { "name": "users", "capabilities": [
                cap("users.read", true, false, &["list"], user_admin_supported, user_admin_supported),
                cap("users.manage", false, true, &["add", "role", "remove"], user_admin_supported, user_admin_supported),
            ] },
            { "name": "domains", "capabilities": [
                cap("domains.read", true, false, &["list"], domain_admin_supported, domain_admin_supported),
                cap("domains.manage", false, true, &["create"], domain_admin_supported, domain_admin_supported),
            ] },
            { "name": "voice", "capabilities": [
                cap_with_reason("voice.duplex", false, false, &["duplex", "stream"], false, false, Some("not_assembled")),
            ] },
            { "name": "subagents", "capabilities": [
                cap_with_reason("subagents.orchestration", false, false, &["spawn", "coordinate"], false, false, Some("not_assembled")),
            ] },
            { "name": "trace", "capabilities": [ cap("trace.read", true, false, &["list", "detail"], trace_supported, trace_supported) ] },
            { "name": "audit", "capabilities": [ cap("audit.read", true, false, &["list"], audit_supported, audit_supported) ] },
            { "name": "activity", "capabilities": [ cap("activity.sse", true, false, &["subscribe"], true, true) ] },
        ]
    });
    (StatusCode::OK, Json(manifest)).into_response()
}
