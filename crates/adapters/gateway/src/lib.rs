//! Canonical HTTP gateway adapter.
//!
//! The gateway decodes transport requests, invokes the canonical runtime, and
//! encodes responses. It does not own provider routing, tool dispatch,
//! sessions, governance, or a second orchestration engine.

#![deny(unsafe_code)]

/// Unified JSON error frame contract (`{"error": {"message", "code", "solution"}}`).
pub mod error_frame;

/// Wire-level error code string catalog and frame constructor helpers.
pub mod error_codes;

/// Runtime configuration hot reload (`/v1/admin/config`).
pub mod admin;

/// Native and OpenAI-compatible HTTP chat entry points.
pub mod canonical_entry;

/// Panel introspection surface (`/v1/panel/*`, tools, capabilities, audit, traces).
pub mod panels;

/// G-P1 指标端点 (`GET /v1/panel/metrics`): 指标注册表的 HTTP 数据面。
pub mod panel_metrics;

/// Gateway-level SSE event bus (`/v1/apeireth/events`).
pub mod events;

/// Full-duplex real-time voice barge-in and client interrupt controller.
pub mod barge_in;

/// 8-frame full-duplex protocol and streaming sentence divider.
pub mod duplex_gateway;

/// Transparent file fetcher for distributed node file fetching.
pub mod file_fetcher;

/// Ember HUD 4.0s breath and peripheral vignette glow driver.
pub mod ember_hud_driver;

/// Presence state synthesizer (`presence_state` event, contract §10 / §8a).
pub mod presence;

/// Session-scoped model + permission settings (`/v1/sessions/{id}/settings`).
pub mod session_settings;

/// Resident-slot (daemon) bookkeeping: PID file claim / release / liveness.
pub mod resident;

/// Graceful-shutdown trigger shared by signals and the admin shutdown endpoint.
pub mod shutdown;

/// IM quick access: channel configuration surface (env + data directory).
pub mod im_channels;

/// IM quick access: bidirectional message bridge (session mapping + turn relay).
pub mod im_bridge;

/// IM quick access: approval cards and four-state closure (paired audit writes).
pub mod im_approval;

/// W2-② client credential system: per-client token table (name → token, env /
/// json config faces with stored-doc refuse-to-open semantics).
pub mod client_tokens;

/// W3-① 域上下文面: 请求 → ClientIdentity → user_id 域解析 → 按域路由,
/// 越域访问拒绝即帧 (404/403 语义, 不泄露他域存在)。
pub mod domain_context;

/// W3-② 用户管理面 (HTTP + CLI 双入口同源): 共享 UserTable 服务与
/// `/v1/panel/users`、`/v1/admin/users*` 端点。
pub mod user_admin;

/// W3-① 域管理面 (V2-b): `DomainRegistry` 开域/列域口与
/// `/v1/panel/domains`、`/v1/admin/domains` 端点。
pub mod domain_admin;

/// 技能库只读面 (`GET /v1/panel/skills`): `SkillLibrary::scan` 的元数据投影端口
/// (StoredDoc 拒开语义, 坏卡列拒不砖库; 不回卡体, 卡体走 L2 封套面)。
pub mod skill_shelf;

/// W2-② remote access face: listen knob, non-loopback fail-closed bind gate,
/// and the explicitly unwired TLS placeholder surface.
pub mod remote_access;

pub use barge_in::{
    format_sse_interrupt_event, BargeInController, InterruptNotice, InterruptReason,
    InterruptStreamSubscription, StreamHandle,
};
pub use duplex_gateway::{DuplexFrame, DuplexSessionController, SentenceDivider};
pub use ember_hud_driver::{EmberCognitiveStance, EmberHudDriver, EmberShaderUniforms};
pub use file_fetcher::{
    FetchedFile, FileFetchError, InternalFileRequest, InternalFileResponse, TransparentFileFetcher,
};
pub use presence::{
    presence_snapshot_handler, spawn_presence_heartbeat, InitiativeBudget, PresenceBreath,
    PresencePad, PresenceService, PresenceSignificance, PresenceSnapshotBody, PresenceSource,
    PresenceState, PresenceSubscription, PresenceSynthesizer, HEARTBEAT_INTERVAL_SECS,
    INITIATIVE_DAILY_CAP,
};

pub use canonical_entry::{
    build_gateway_state, build_gateway_state_with_services, canonical_router,
    canonical_router_with_panels, canonical_router_with_services, canonical_router_with_state,
    execute_chat, execute_chat_attributed, resolve_approval, resolve_approval_attributed,
    serve_canonical, serve_canonical_with_services, serve_state_with_shutdown, serve_with_shutdown,
    CanonicalApprovalRequest, CanonicalChatOutcome, CanonicalChatRequest, CanonicalChatResponse,
    CanonicalEntryError, CanonicalExecutionEvent, CanonicalPendingApproval, GateVerdict,
    GatewayTokenGate, SessionAttribution, UNAUTHENTICATED_PROBE_PATHS,
};

pub use client_tokens::{
    client_name_binds, identity_label, record_session_client, record_user_action,
    resolve_user_identity, validate_client_user_bindings, validate_gate_table_pair, ClientIdentity,
    ClientTokenConfig, ClientTokenError, ClientTokenSpec, ClientTokenTable, UserAccess,
    UserContext, CLIENT_NAME_CLAIM_HEADER, CLIENT_TOKENS_DOC_NAME, CLIENT_TOKENS_DOC_VERSION,
    CLIENT_TOKENS_ENV, CLIENT_TOKENS_FILE, SESSION_CLIENT_AUDIT_EVENT, USER_ACTION_AUDIT_EVENT,
};

pub use domain_admin::{DomainAdminDto, DomainAdminService, DomainCreateBody};
pub use skill_shelf::{
    SkillMetaDto, SkillRefusalDto, SkillShelfQuery, SkillShelfScanDto, SkillSourceDto,
};
pub use user_admin::{
    UserAdminDto, UserAdminError, UserAdminService, UserCreateBody, UserRolePatchBody,
};

pub use remote_access::{
    bind_scope, enforce_remote_bind, resolve_listen_bind, resolve_listen_bind_from,
    tls_surface_status, tls_surface_status_from, BindScope, RemoteBindClearance, RemoteBindError,
    TlsSurface, DEFAULT_BIND, LISTEN_ENV, TLS_CERT_ENV, TLS_KEY_ENV,
};

pub use admin::{
    credential_name, normalize_provider, CapabilitiesConfig, ConfigPatch, ConfigView,
    CredentialWriter, GatewayRuntimeConfig,
};

pub use error_frame::{ErrorCode, ErrorEnvelope, ErrorFrame};
pub use im_approval::{
    approval_risk_level, FileApprovalAudit, ImApprovalAuditCommit, ImApprovalCloser,
    ImApprovalClosure, ImApprovalClosureResult, ImApprovalNotice, ImApprovalRequest,
    ImApprovalResolution, ImApprovalResolver, MemoryApprovalAudit,
};
pub use im_bridge::{
    im_router, im_sessions_path, sessions_doc_compat, CanonicalChainHandler, ImBridge,
    ImBridgeError, ImInboundResult, ImSessionMapStore, ImSessionMapping, ImSessionTable,
    ImTurnHandler, ImTurnOutcome, IM_SESSIONS_DOC_NAME, IM_SESSIONS_DOC_VERSION, IM_SESSIONS_FILE,
    IM_SIGNATURE_HEADER,
};
pub use im_channels::{
    assemble_im_channels, doc_compat, im_channels_path, ImChannelAssembly, ImChannelConfig,
    ImChannelSpec, ImConfigError, IM_CHANNELS_DOC_NAME, IM_CHANNELS_DOC_VERSION, IM_CHANNELS_ENV,
    IM_CHANNELS_FILE,
};

pub use events::{
    events_handler, is_session_event, EventBus, EventsQuery, EventsSubscription, FlushReport,
    GatewayEvent, ObservationFlushSubscription, RuntimeObservationSink, SessionEventsSubscription,
    SessionScope, FLUSH_PASS_INTERVAL_SECS,
};

pub use resident::{
    claim_slot, read_slot, ResidentError, ResidentLease, ResidentParseError, ResidentRecord,
};
pub use shutdown::ShutdownTrigger;

pub use panels::{
    AuditCommand, AuditDto, AuditQuery, EpisodeDto, EpisodeMutationDto, GatewayServices,
    GatewayState, GrantCommand, GrantDto, GrantMutationDto, GrantQuery, GraphEdgeDto, GraphNodeDto,
    GuardDryRunRequest, GuardDryRunResponse, GuardEventDto, GuardStatusDto, MemoryCommand,
    MemoryGovernanceCommand, MemoryGraphDto, MemoryQuery, ModuleQuery, OrganDto, PanelData,
    SafetyGuardQuery, SessionQuery, SessionSummaryDto, ToolCatalogQuery, ToolDto, TraceCommand,
    TraceDetailDto, TraceQuery, TraceSpanDto, TraceSummaryDto, WorkbenchMemoryProvenanceDto,
    WorkbenchQuery, WorkbenchToolExecutionDto, WorkbenchTurnDto,
};
