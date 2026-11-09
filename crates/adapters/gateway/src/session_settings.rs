//! Session-scoped settings surface.
//!
//! `GET /v1/sessions/{session_id}/settings` reads the durable session settings;
//! `PATCH /v1/sessions/{session_id}/settings` applies a partial update;
//! `DELETE /v1/sessions/{session_id}` removes the stored session record.
//!
//! These endpoints go through the runtime's own [`SessionManager`], so they act
//! on exactly the same durable session the agent loop reads at the start of a
//! turn. A `model` of `null` in a PATCH resets the session back to the global
//! default model.
//!
//! W2-③ 并发写语义: PATCH / DELETE 是会话账本写者, 与在途回合走**同一把会话
//! 写闸** (先到先服务排队) —— 读-改-写全程持证, 与回合写零交叠 (设置补丁不会
//! 覆写回合刚追加的消息, 回合也不会吃掉设置改动); 每次写落
//! `gateway.session_client` 审计行 (谁改了哪个会话)。
//!
//! # 会话隐私档 (W3 · 单元 2: **本地声明载体**)
//!
//! `privacy_level` 是**可选**的会话级隐私声明 (值域 `ordinary|restricted|
//! confidential`, 与 `apeireth_core::routing_policy::PrivacyLevel` 同名同域):
//!
//! - **本地持久**: 落在数据目录的会话隐私档表 (`session-privacy.json`) ⇒ 跨进程
//!   可见 (网关写、CLI 读) ⇒ 重启**不**丢声明 (丢了 = 隐私闸静默失效);
//! - **绝不外发**: 本表只在本机磁盘; ⚠️ **绝不**进规范请求的 `metadata`
//!   (那一整包会被转发到厂商线 ⇒ 声明本身变成泄漏面);
//! - **不认识的值 ⇒ 明确报错** (400), **不**降级为普通档 (降级 = 静默放宽);
//! - 缺省 ⇒ 未声明 ⇒ 上游信号 `Defaulted` ⇒ 隐私闸不过滤 ⇒ **逐字节零变化**。
//!
//! ⚠️ 运行时的 [`SessionSettings`] 结构体在 **engine 层** (本单元禁改) ⇒ 隐私档
//! 另存一张**本地表**, 不塞进会话账本本体 (会话账本只带 model / 权限档 / 审批记忆)。

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path as StdPath, PathBuf};
use std::str::FromStr;
use std::sync::{OnceLock, RwLock};

use apeireth_core::kernel::SessionId;
use apeireth_core::routing_policy::{privacy_signal_from_str, PrivacySignal};
use apeireth_core::storage_atomic;
use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use apeireth_core::user_domain::UserDomainError;
use apeireth_runtime::canonical::{PermissionPreset, SessionManager, SessionSettings};
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::client_tokens::{identity_label, record_session_client, ClientIdentity};
use crate::domain_context::DomainContext;
use crate::panels::GatewayState;

/// Full session-settings projection returned by both endpoints.
#[derive(Debug, Serialize)]
pub struct SessionSettingsBody {
    pub session_id: String,
    pub model: Option<String>,
    pub permission_preset: PermissionPreset,
    pub approval_remember: bool,
    /// 会话声明的隐私档 (**可选**); `null` = 未声明 ⇒ 上游按缺省兜底处理。
    ///
    /// ⚠️ 这是**声明值本身** (会进 HTTP 响应 = 本地控制面) ⇒ 它**不**进任何
    /// 出网载荷 (模型请求 / 遥测 / 上报一律不带)。
    pub privacy_level: Option<String>,
}

impl SessionSettingsBody {
    fn from_session(
        session_id: SessionId,
        settings: &SessionSettings,
        privacy_level: Option<&str>,
    ) -> Self {
        Self {
            session_id: session_id.to_string(),
            model: settings.model.clone(),
            permission_preset: settings.permission_preset,
            approval_remember: settings.approval_remember,
            privacy_level: privacy_level.map(str::to_string),
        }
    }
}

/// Partial PATCH body. `model: null` resets the session to the global default;
/// a missing field leaves it unchanged.
///
/// `privacy_level` 用**双层** `Option` 区分三态: 字段缺省 = 不改; `null` =
/// **清除声明** (回到缺省兜底); 字符串 = 声明 (值不认识 ⇒ 报错, 不降级)。
#[derive(Debug, Deserialize)]
pub struct PatchSessionSettingsRequest {
    #[serde(default, deserialize_with = "double_option")]
    pub model: Option<Option<String>>,
    #[serde(default)]
    pub permission_preset: Option<PermissionPreset>,
    #[serde(default)]
    pub approval_remember: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    pub privacy_level: Option<Option<String>>,
}

/// Deserialize a present-but-null field as `Some(None)` so `Option<Option<T>>`
/// can distinguish "field missing" from "field explicitly reset to null".
fn double_option<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(Some)
}

/// Unified error contract for this surface.
#[derive(Debug, Serialize)]
pub(crate) struct ErrorDetail {
    pub(crate) message: String,
    pub(crate) code: String,
    pub(crate) solution: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ErrorBody {
    pub(crate) error: ErrorDetail,
}

type SettingsError = (StatusCode, Json<ErrorBody>);

// ---------------------------------------------------------------------------
// 会话隐私档表 (本地声明载体 · 本地持久 · 绝不外发)
// ---------------------------------------------------------------------------

/// 数据目录里的会话隐私档表文件名。
pub const SESSION_PRIVACY_FILE: &str = "session-privacy.json";

/// 会话隐私档表的存储文档身份名。
pub const SESSION_PRIVACY_DOC_NAME: &str = "session-privacy";

/// 会话隐私档表的存储文档版本。
pub const SESSION_PRIVACY_DOC_VERSION: u32 = 1;

/// 会话隐私档表: 会话 id → 隐私档名。
///
/// **本地持久** (数据目录 `session-privacy.json`) ⇒ 跨进程可见 (网关写、CLI 读)
/// ⇒ 重启不丢声明 (丢了 = 隐私闸静默失效 ⇒ 宁可报错也不静默放宽)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPrivacyTable {
    /// 全部会话的隐私档声明 (会话 id → 档位名)。
    #[serde(default)]
    pub entries: BTreeMap<String, String>,
}

impl SessionPrivacyTable {
    /// 该会话声明的隐私档 (原始值); `None` = 未声明。
    pub fn get(&self, session_id: &SessionId) -> Option<&str> {
        self.entries
            .get(&session_id.to_string())
            .map(String::as_str)
    }

    /// 声明该会话的隐私档 (`Some`) 或**清除**声明 (`None` ⇒ 回到缺省兜底)。
    pub fn set(&mut self, session_id: &SessionId, level: Option<&str>) {
        match level {
            Some(level) => {
                self.entries
                    .insert(session_id.to_string(), level.to_string());
            }
            None => {
                self.entries.remove(&session_id.to_string());
            }
        }
    }

    /// 校验: 全部档位名都在 core 词表内。
    ///
    /// **fail-closed**: 存了不认识的档位 ⇒ 整表拒开 (错误帧**不回显**值) ——
    /// 把它当普通档 = 静默放宽。
    pub fn validate(&self) -> Result<(), SessionPrivacyError> {
        for (session_id, level) in &self.entries {
            if privacy_signal_from_str(Some(level)).is_err() {
                return Err(SessionPrivacyError::InvalidLevel {
                    session_id: session_id.clone(),
                    reason: "unknown privacy level (accepted: ordinary, restricted, confidential)"
                        .to_string(),
                });
            }
        }
        Ok(())
    }

    /// 从数据目录读 (文件缺省 = `Ok(None)`; 存在但拒开 = 错误)。
    pub fn load_from_data_dir(data_dir: &StdPath) -> Result<Option<Self>, SessionPrivacyError> {
        let path = session_privacy_path(data_dir);
        if !path.exists() {
            return Ok(None);
        }
        let doc = stored_doc::open_single::<Self>(&path, &session_privacy_doc_compat())
            .map_err(SessionPrivacyError::Stored)?;
        doc.body.validate()?;
        Ok(Some(doc.body))
    }

    /// 写回数据目录 (持久档原子写)。
    pub fn save_to_data_dir(&self, data_dir: &StdPath) -> Result<(), SessionPrivacyError> {
        stored_doc::save_single(
            &session_privacy_path(data_dir),
            &session_privacy_doc_compat(),
            self.clone(),
            storage_atomic::DEFAULT_FILE_MODE,
        )
        .map_err(SessionPrivacyError::Stored)?;
        Ok(())
    }
}

/// 数据目录里的会话隐私档表路径。
pub fn session_privacy_path(data_dir: &StdPath) -> PathBuf {
    data_dir.join(SESSION_PRIVACY_FILE)
}

/// 会话隐私档表文件的读契约 (仅当前版本可读)。
pub fn session_privacy_doc_compat() -> DocCompat {
    DocCompat::exact(SESSION_PRIVACY_DOC_NAME, SESSION_PRIVACY_DOC_VERSION)
}

/// 会话隐私档表的缺陷。**值自由报错**: 只报会话 id 与期望词表, 从不回显档位值。
#[derive(Debug)]
pub enum SessionPrivacyError {
    /// 存储文档拒开 (IO / 身份 / 版本 / 形状)。
    Stored(StoredDocError),
    /// 存了不认识的档位名 ⇒ 整表拒开 (fail-closed)。
    InvalidLevel {
        /// 出错的会话 id (键名是配置面, 不是值)。
        session_id: String,
        /// 期望词表说明 (不含值)。
        reason: String,
    },
    /// 进程内表锁被毒化 (panic 波及) ⇒ 拒开 (不静默当成「未声明」)。
    LockPoisoned,
}

impl fmt::Display for SessionPrivacyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stored(error) => write!(f, "session privacy table refused to open: {error}"),
            Self::InvalidLevel { session_id, reason } => write!(
                f,
                "session privacy declaration for session {session_id:?} is invalid: {reason}"
            ),
            Self::LockPoisoned => {
                write!(f, "session privacy table lock is poisoned")
            }
        }
    }
}

impl std::error::Error for SessionPrivacyError {}

impl From<StoredDocError> for SessionPrivacyError {
    fn from(error: StoredDocError) -> Self {
        Self::Stored(error)
    }
}

/// 进程内**写互斥** (读-改-写全程持证 ⇒ 同进程内两个并发 PATCH 不互相吃掉)。
///
/// ⚠️ **不缓存表内容**: 每次读写都现读磁盘 ⇒ 跨进程可见 (网关写、CLI 读) ⇒
/// 声明即刻生效; 缓存会变成「另一个进程改了但我看不见」的静默陈旧。
fn privacy_write_lock() -> &'static RwLock<()> {
    static LOCK: OnceLock<RwLock<()>> = OnceLock::new();
    LOCK.get_or_init(|| RwLock::new(()))
}

/// 读某个会话声明的隐私档 (原始值); `None` = 未声明。
///
/// **现读磁盘** (跨进程: 网关写、CLI 读) ⇒ 声明即刻生效; 坏表 ⇒ **报错**
/// (不静默当成「未声明」—— 那等于隐私闸静默失效)。
pub fn session_privacy_level(
    session_id: &SessionId,
) -> Result<Option<String>, SessionPrivacyError> {
    let table = SessionPrivacyTable::load_from_data_dir(&privacy_data_dir())?.unwrap_or_default();
    Ok(table.get(session_id).map(str::to_string))
}

/// 会话隐私档表所在的数据目录 (口径: `APEIRETH_DATA_DIR` → 家目录 `.apeireth`)。
fn privacy_data_dir() -> PathBuf {
    apeireth_core::paths::default_data_root()
}

/// 隐私档的 PATCH 三态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrivacyPatch {
    /// 字段缺省 ⇒ 不改。
    Unchanged,
    /// `null` ⇒ 清除声明 (回到缺省兜底)。
    Clear,
    /// 声明一个档位 (值已校验过)。
    Declare(&'static str),
}

/// 校验 PATCH 里的隐私档值: 认识的词 ⇒ 声明; 不认识 ⇒ **报错** (不降级)。
fn parse_privacy_patch(raw: &str) -> Result<PrivacyPatch, SettingsError> {
    match privacy_signal_from_str(Some(raw)) {
        Ok(PrivacySignal::Declared(level)) => Ok(PrivacyPatch::Declare(level.as_str())),
        // 空白 = 声明了但没内容 ⇒ 不认识 ⇒ 报错 (不是「未声明」; 未声明是**字段缺省**)。
        Ok(PrivacySignal::Defaulted) => Err(invalid_privacy_level()),
        Err(_) => Err(invalid_privacy_level()),
    }
}

/// 隐私档值不合法的错误帧 (**值自由**: 不回显被声明的值)。
fn invalid_privacy_level() -> SettingsError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorBody {
            error: ErrorDetail {
                message: format!(
                    "{} must be one of ordinary, restricted, confidential",
                    apeireth_core::routing_policy::PRIVACY_SIGNAL_KEY
                ),
                code: "invalid_privacy_level".into(),
                solution: "send a known privacy level, or null to clear the declaration".into(),
            },
        }),
    )
}

/// 本地隐私档表不可用 (坏表) 的错误帧 (**值自由**)。
fn privacy_table_unavailable(error: &SessionPrivacyError) -> SettingsError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorBody {
            error: ErrorDetail {
                message: format!("session privacy declaration table is unavailable: {error}"),
                code: "session_privacy_unavailable".into(),
                solution: "fix or remove the session privacy declaration table and retry".into(),
            },
        }),
    )
}

fn session_not_found(session_id: &str) -> SettingsError {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody {
            error: ErrorDetail {
                message: format!("session {session_id} does not exist"),
                code: "session_not_found".into(),
                solution: "create the session first or check the session id".into(),
            },
        }),
    )
}

fn invalid_request(message: String) -> SettingsError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorBody {
            error: ErrorDetail {
                message,
                code: "invalid_request".into(),
                solution: "check the request body or session id and retry".into(),
            },
        }),
    )
}

fn delete_failed(message: String) -> SettingsError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorBody {
            error: ErrorDetail {
                message,
                code: "session_delete_failed".into(),
                solution: "retry; if it persists check the gateway storage log".into(),
            },
        }),
    )
}

/// W3-①: 本域拒开帧 (403 `domain_refused`) —— 真因如实 (本域信息, 不涉他域)。
fn domain_refused(error: &UserDomainError) -> SettingsError {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorBody {
            error: ErrorDetail {
                message: format!("the request's user domain is not open: {}", error.reason()),
                code: "domain_refused".into(),
                solution: "check users/<id>/domain.json (bad domain config refuses to open)".into(),
            },
        }),
    )
}

/// W3-①: 按请求域上下文取会话写闸 (**每域恰一把**)。
///
/// - 域面未接线 / 无域上下文 = 兼容路径: 运行时那把 (默认域原位语义);
/// - 已接线 = 域平面按域路由; 默认域与运行时**同一把** (W2-③ 同会话写串行
///   承诺不拆);
/// - 资源级跨域不需要专门拒帧: 本域写闸查不到他域会话, 一律走
///   [`session_not_found`] —— 与真不存在**同帧** (存在性不泄露)。
fn session_manager<'s>(
    state: &'s GatewayState,
    domain: Option<&Extension<DomainContext>>,
) -> Result<&'s SessionManager, SettingsError> {
    let Some(Extension(context)) = domain else {
        return Ok(state.runtime.sessions());
    };
    let Some(plane) = state.domains.as_ref() else {
        return Ok(state.runtime.sessions());
    };
    match &context.domain {
        None => Ok(state.runtime.sessions()),
        Some(domain) => plane
            .sessions(domain)
            .map(|manager| manager.as_ref())
            .map_err(|error| domain_refused(&error)),
    }
}

pub(crate) async fn get_session_settings(
    State(state): State<GatewayState>,
    domain: Option<Extension<DomainContext>>,
    Path(session_id): Path<String>,
) -> Result<Response, SettingsError> {
    let session_id = SessionId::from_str(&session_id)
        .map_err(|_| invalid_request(format!("invalid session id {session_id:?}")))?;
    let loaded = session_manager(&state, domain.as_ref())?
        .load(&session_id)
        .await
        .map_err(|error| invalid_request(error.to_string()))?;
    let Some(session) = loaded else {
        return Err(session_not_found(&session_id.to_string()));
    };
    // 会话隐私档: 现读本地表 (未声明 = `null`); 坏表 ⇒ 报错 (不静默当成未声明)。
    let privacy_level = read_privacy_level(&session_id)?;

    Ok(Json(SessionSettingsBody::from_session(
        session_id,
        &session.settings,
        privacy_level.as_deref(),
    ))
    .into_response())
}

/// 现读本地会话隐私档表里的该会话声明 (坏表 ⇒ 错误帧, 不静默当成「未声明」)。
fn read_privacy_level(session_id: &SessionId) -> Result<Option<String>, SettingsError> {
    session_privacy_level(session_id).map_err(|error| privacy_table_unavailable(&error))
}

/// 把隐私档声明写进本地表并落盘 (三态: 不改 / 清除 / 声明)。
///
/// 读-改-写**全程持写互斥**且**现读磁盘** ⇒ 同进程并发 PATCH 不互相吃掉;
/// 跨进程可见 (CLI 现读即得) ⇒ 声明即刻生效。
fn apply_privacy_patch(
    session_id: &SessionId,
    patch: PrivacyPatch,
) -> Result<Option<String>, SettingsError> {
    if patch == PrivacyPatch::Unchanged {
        return read_privacy_level(session_id);
    }
    let _guard = privacy_write_lock()
        .write()
        .map_err(|_| privacy_table_unavailable(&SessionPrivacyError::LockPoisoned))?;
    let mut table = SessionPrivacyTable::load_from_data_dir(&privacy_data_dir())
        .map_err(|error| privacy_table_unavailable(&error))?
        .unwrap_or_default();
    match patch {
        PrivacyPatch::Unchanged => {}
        PrivacyPatch::Clear => table.set(session_id, None),
        PrivacyPatch::Declare(level) => table.set(session_id, Some(level)),
    }
    table
        .save_to_data_dir(&privacy_data_dir())
        .map_err(|error| privacy_table_unavailable(&error))?;
    Ok(table.get(session_id).map(str::to_string))
}

pub(crate) async fn patch_session_settings(
    State(state): State<GatewayState>,
    identity: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
    Path(session_id): Path<String>,
    Json(request): Json<PatchSessionSettingsRequest>,
) -> Result<Response, SettingsError> {
    let session_id = SessionId::from_str(&session_id)
        .map_err(|_| invalid_request(format!("invalid session id {session_id:?}")))?;
    // 隐私档先校验 (不认识 ⇒ 400, **不**降级为普通档; 降级 = 静默放宽)。
    let privacy_patch = match request.privacy_level.as_ref() {
        None => PrivacyPatch::Unchanged,
        Some(None) => PrivacyPatch::Clear,
        Some(Some(raw)) => parse_privacy_patch(raw)?,
    };
    // W2-③: 读-改-写全程持会话写闸 (与在途回合零交叠, 不丢任何一方的写);
    // W3-①: 写闸按请求域路由 (每域恰一把)。
    let updated = session_manager(&state, domain.as_ref())?
        .update(session_id, |session| {
            if let Some(model) = request.model {
                session.settings.model = model;
            }
            if let Some(preset) = request.permission_preset {
                session.settings.permission_preset = preset;
            }
            if let Some(remember) = request.approval_remember {
                session.settings.approval_remember = remember;
            }
            SessionSettingsBody::from_session(session_id, &session.settings, None)
        })
        .await
        .map_err(|error| invalid_request(error.to_string()))?;
    let Some((mut body, admission)) = updated else {
        return Err(session_not_found(&session_id.to_string()));
    };
    // 隐私档另存本地表 (会话账本在 engine 层, 本单元禁改) ⇒ 声明**不**混进
    // 会话账本本体, 但同样本地持久。
    body.privacy_level = apply_privacy_patch(&session_id, privacy_patch)?;
    record_session_client(
        Some(state.observations.as_ref()),
        session_id,
        identity_label(identity.as_ref()),
        "settings.patch",
        Some(admission),
    );

    Ok(Json(body).into_response())
}

pub(crate) async fn delete_session(
    State(state): State<GatewayState>,
    identity: Option<Extension<ClientIdentity>>,
    domain: Option<Extension<DomainContext>>,
    Path(session_id): Path<String>,
) -> Result<Response, SettingsError> {
    let session_id = SessionId::from_str(&session_id)
        .map_err(|_| invalid_request(format!("invalid session id {session_id:?}")))?;
    // W2-③: 删除同为会话账本写 —— 过同一把写闸, 不在回合半途抽走账本;
    // W3-①: 写闸按请求域路由 (每域恰一把)。
    let manager = session_manager(&state, domain.as_ref())?;
    let permit = manager.write_permit(session_id).await;
    let deleted = manager
        .delete(&session_id)
        .await
        .map_err(|error| delete_failed(error.to_string()))?;
    if !deleted {
        return Err(session_not_found(&session_id.to_string()));
    }
    record_session_client(
        Some(state.observations.as_ref()),
        session_id,
        identity_label(identity.as_ref()),
        "session.delete",
        Some(permit.admission()),
    );
    drop(permit);

    Ok(Json(serde_json::json!({ "deleted": true })).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::system_clock;
    use apeireth_runtime::canonical::{InMemorySessionStore, Runtime, Session, SessionStore};
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::{delete, get, patch};
    use axum::Router;
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn test_router(store: Arc<dyn SessionStore>) -> Router {
        let runtime = Runtime::builder()
            .with_session_store(store)
            .build()
            .await
            .unwrap();
        let presence = crate::presence::PresenceService::new(crate::events::EventBus::default());
        let presence_stream = Arc::new(
            presence
                .keepalive()
                .expect("fresh presence surface cannot be shut down"),
        );
        let events = crate::events::EventBus::default();
        let events_stream = Arc::new(
            events
                .keepalive()
                .expect("fresh events surface cannot be shut down"),
        );
        let observations = Arc::new(crate::events::RuntimeObservationSink::new(None, None));
        let observations_stream = Arc::new(
            observations
                .keepalive()
                .expect("fresh flush surface cannot be shut down"),
        );
        let state = GatewayState {
            runtime: Arc::new(runtime),
            services: crate::panels::GatewayServices::default(),
            events,
            events_stream,
            observations,
            observations_stream,
            presence,
            presence_stream,
            hot_config: Arc::new(std::sync::RwLock::new(
                crate::admin::GatewayRuntimeConfig::from_env(),
            )),
            token_gate: crate::canonical_entry::GatewayTokenGate::from_env(),
            client_tokens: crate::client_tokens::ClientTokenTable::default(),
            user_access: crate::client_tokens::UserAccess::default(),
            domains: None,
            domain_runtimes: None,
            shutdown: Arc::new(crate::shutdown::ShutdownTrigger::default()),
        };
        Router::<GatewayState>::new()
            .route(
                "/v1/sessions/:session_id/settings",
                get(get_session_settings).patch(patch_session_settings),
            )
            .route("/v1/sessions/:session_id", delete(delete_session))
            .with_state(state)
    }

    async fn save_session(store: &Arc<dyn SessionStore>, id: SessionId) {
        let clock = system_clock();
        store.save(&Session::new(id, clock.as_ref())).await.unwrap();
    }

    async fn json_body(response: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn get_returns_default_settings_for_an_existing_session() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["session_id"], sid.to_string());
        assert_eq!(body["model"], serde_json::Value::Null);
        assert_eq!(body["permission_preset"], "standard");
        assert_eq!(body["approval_remember"], false);
    }

    #[tokio::test]
    async fn patch_updates_settings_and_returns_full_projection() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"anthropic/claude-3-5","permission_preset":"full","approval_remember":true}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["model"], "anthropic/claude-3-5");
        assert_eq!(body["permission_preset"], "full");
        assert_eq!(body["approval_remember"], true);
    }

    #[tokio::test]
    async fn patch_approval_remember_is_partial_and_preserves_other_fields() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store).await;

        // Set model + preset first.
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"model":"some/model","permission_preset":"full"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then toggle only approval_remember; model and preset must survive.
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"approval_remember":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["approval_remember"], true);
        assert_eq!(body["model"], "some/model");
        assert_eq!(body["permission_preset"], "full");
    }

    #[tokio::test]
    async fn patch_model_null_resets_to_global_default() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store.clone()).await;

        // Set a model first.
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":"some/model"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then reset it with an explicit null.
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"model":null}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["model"], serde_json::Value::Null);
        assert_eq!(body["permission_preset"], "standard");
        assert_eq!(body["approval_remember"], false);
    }

    #[tokio::test]
    async fn missing_session_returns_session_not_found_error_frame() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let app = test_router(store).await;
        let sid = SessionId::new();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], "session_not_found");
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&sid.to_string()));
        assert!(body["error"]["solution"].is_string());
    }

    #[tokio::test]
    async fn malformed_session_id_returns_invalid_request_error_frame() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let app = test_router(store).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/sessions/not-a-uuid/settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], "invalid_request");
    }

    #[tokio::test]
    async fn delete_removes_an_existing_session_and_reports_deletion() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store.clone()).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/v1/sessions/{sid}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["deleted"], true);
        assert!(
            store.load(&sid).await.unwrap().is_none(),
            "the store must no longer hold the deleted session"
        );
    }

    #[tokio::test]
    async fn delete_unknown_session_returns_session_not_found_error_frame() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let app = test_router(store).await;
        let sid = SessionId::new();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/v1/sessions/{sid}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], "session_not_found");
    }

    #[tokio::test]
    async fn delete_malformed_session_id_returns_invalid_request_error_frame() {
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let app = test_router(store).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/v1/sessions/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], "invalid_request");
    }

    /// One settings PATCH against the HTTP surface, then the read-back GET.
    async fn patch_then_get(
        app: &Router,
        sid: SessionId,
        patch: &str,
    ) -> (serde_json::Value, serde_json::Value) {
        let patched = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .header("content-type", "application/json")
                    .body(Body::from(patch.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(patched.status(), StatusCode::OK);
        let read_back = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(read_back.status(), StatusCode::OK);
        (json_body(patched).await, json_body(read_back).await)
    }

    #[tokio::test]
    async fn patch_permission_preset_roundtrips_each_tier_through_read_back() {
        // Select a tier, persist it, read the same tier back — for every tier and
        // through every transition (read_only → standard → full), so a changed
        // tier must never round-trip distorted.
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;
        let app = test_router(store).await;

        for tier in ["read_only", "standard", "full"] {
            let body = format!(r#"{{"permission_preset":"{tier}"}}"#);
            let (patched, read_back) = patch_then_get(&app, sid, &body).await;
            assert_eq!(patched["permission_preset"], tier);
            assert_eq!(
                read_back["permission_preset"], tier,
                "tier {tier} must read back unchanged after being persisted"
            );
            assert_eq!(read_back["session_id"], sid.to_string());
        }
    }

    #[tokio::test]
    async fn permission_preset_survives_router_rebuild_over_the_same_store() {
        // A gateway restart rebuilds every HTTP surface from scratch while the
        // durable store keeps the rows. The stored tier must come back as itself,
        // not fall back to the default tier.
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let sid = SessionId::new();
        save_session(&store, sid).await;

        let (patched, _) = patch_then_get(
            &test_router(store.clone()).await,
            sid,
            r#"{"permission_preset":"full","approval_remember":true}"#,
        )
        .await;
        assert_eq!(patched["permission_preset"], "full");

        let restarted = test_router(store).await;
        let response = restarted
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/sessions/{sid}/settings"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["permission_preset"], "full");
        assert_eq!(body["approval_remember"], true);
    }

    // -----------------------------------------------------------------------
    // 会话隐私档 (W3 · 单元 2): 本地声明载体
    // -----------------------------------------------------------------------

    /// 测试用数据目录 (进程级 env ⇒ 本模块内**串行**执行)。
    ///
    /// ⚠️ 只在本模块内改 `APEIRETH_DATA_DIR` (网关源码内**无人**读它 ⇒ 不波及其它
    /// 用例); 串行锁防本模块自己的用例互踩。
    static DATA_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 把 `APEIRETH_DATA_DIR` 指向一个临时目录, 跑完复原。
    async fn with_temp_data_dir<F, Fut>(body: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let _serial = DATA_DIR_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let original = std::env::var("APEIRETH_DATA_DIR").ok();
        let dir =
            std::env::temp_dir().join(format!("apeireth-session-privacy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("临时数据目录");
        std::env::set_var("APEIRETH_DATA_DIR", &dir);
        body().await;
        match original {
            Some(value) => std::env::set_var("APEIRETH_DATA_DIR", value),
            None => std::env::remove_var("APEIRETH_DATA_DIR"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **零变化钉桩**: 缺省 ⇒ 未声明 ⇒ 投影里 `privacy_level = null`。
    #[tokio::test]
    async fn the_settings_projection_carries_no_privacy_declaration_by_default() {
        with_temp_data_dir(|| async {
            let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
            let sid = SessionId::new();
            save_session(&store, sid).await;
            let app = test_router(store).await;
            let response = app
                .oneshot(
                    Request::builder()
                        .uri(format!("/v1/sessions/{sid}/settings"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body = json_body(response).await;
            assert_eq!(
                body["privacy_level"],
                serde_json::Value::Null,
                "缺省 ⇒ 未声明 ⇒ 隐私闸不过滤 (逐字节零变化)"
            );
        })
        .await;
    }

    /// 声明 ⇒ 投影回显 + **本地持久** (现读磁盘 ⇒ 重启不丢 ⇒ 闸不静默失效)。
    #[tokio::test]
    async fn patching_a_privacy_level_declares_it_and_it_reads_back() {
        with_temp_data_dir(|| async {
            let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
            let sid = SessionId::new();
            save_session(&store, sid).await;
            let app = test_router(store).await;

            let (patched, read_back) =
                patch_then_get(&app, sid, r#"{"privacy_level":"restricted"}"#).await;
            assert_eq!(patched["privacy_level"], "restricted");
            assert_eq!(read_back["privacy_level"], "restricted");

            // 本地持久: 另一个进程 (现读磁盘) 也读得到。
            assert_eq!(
                session_privacy_level(&sid).expect("本地表可读"),
                Some("restricted".to_string()),
                "声明落在本地表 ⇒ 跨进程可见 (CLI 现读即得)"
            );
        })
        .await;
    }

    /// **不认识的值 ⇒ 400 拒绝** (不降级为普通档: 降级 = 静默放宽) + **不回显值**。
    #[tokio::test]
    async fn an_unknown_privacy_level_is_refused_and_the_value_is_not_echoed() {
        with_temp_data_dir(|| async {
            let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
            let sid = SessionId::new();
            save_session(&store, sid).await;
            let app = test_router(store).await;

            let response = app
                .oneshot(
                    Request::builder()
                        .method("PATCH")
                        .uri(format!("/v1/sessions/{sid}/settings"))
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"privacy_level":"secret"}"#))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            let body = json_body(response).await;
            assert_eq!(body["error"]["code"], "invalid_privacy_level");
            assert!(
                !body.to_string().contains("secret"),
                "被拒绝的值绝不能回显: {body}"
            );
            // 未声明 (不静默降级成普通档, 也不落盘)。
            assert_eq!(session_privacy_level(&sid).expect("本地表可读"), None);
        })
        .await;
    }

    /// **空白 ≠ 缺位**: 字段存在但空白 ⇒ 400 (不静默当成「没人声明」)。
    #[tokio::test]
    async fn a_blank_privacy_level_is_refused_instead_of_being_treated_as_absent() {
        with_temp_data_dir(|| async {
            let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
            let sid = SessionId::new();
            save_session(&store, sid).await;
            let app = test_router(store).await;

            let response = app
                .oneshot(
                    Request::builder()
                        .method("PATCH")
                        .uri(format!("/v1/sessions/{sid}/settings"))
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"privacy_level":"   "}"#))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "空白 ⇒ 明确拒绝 (不静默放宽成普通档)"
            );
            assert_eq!(session_privacy_level(&sid).expect("本地表可读"), None);
        })
        .await;
    }

    /// `null` ⇒ **清除声明** (回到缺省兜底 ⇒ 闸不过滤 ⇒ 零变化)。
    #[tokio::test]
    async fn patching_null_clears_the_declaration() {
        with_temp_data_dir(|| async {
            let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
            let sid = SessionId::new();
            save_session(&store, sid).await;
            let app = test_router(store).await;

            patch_then_get(&app, sid, r#"{"privacy_level":"confidential"}"#).await;
            assert_eq!(
                session_privacy_level(&sid).expect("本地表可读"),
                Some("confidential".to_string())
            );

            let (_, read_back) = patch_then_get(&app, sid, r#"{"privacy_level":null}"#).await;
            assert_eq!(read_back["privacy_level"], serde_json::Value::Null);
            assert_eq!(session_privacy_level(&sid).expect("本地表可读"), None);
        })
        .await;
    }

    /// 声明**不进会话账本本体** (账本在 engine 层, 只带 model/权限/审批记忆) ⇒
    /// 隐私档不混进回合 transcript / 事件流。
    #[tokio::test]
    async fn the_declaration_is_not_part_of_the_session_ledger() {
        with_temp_data_dir(|| async {
            let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
            let sid = SessionId::new();
            save_session(&store, sid).await;
            let app = test_router(store.clone()).await;
            patch_then_get(&app, sid, r#"{"privacy_level":"restricted"}"#).await;

            let loaded = store.load(&sid).await.unwrap().expect("会话行在场");
            let json = serde_json::to_string(&loaded).expect("会话行可序列化");
            assert!(
                !json.contains("restricted"),
                "隐私档声明**不进会话账本** (只进本地隐私档表): {json}"
            );
        })
        .await;
    }
}
