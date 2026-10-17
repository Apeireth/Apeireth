//! 客户端凭证体系 (W2-②/W3-②): 每客户端令牌表 (name → token) 的配置面与校验面,
//! 外加用户绑定面 (name → token → **user_id** → **role**, W3-②)。
//!
//! 两个入口, 一份 fail-closed 契约 (与 `im_channels.rs` 同族语义):
//!
//! * `APEIRETH_GATEWAY_TOKENS` (JSON 数组, 每项 `{name, token, user_id?}`, 或
//!   `{"clients": [...]}` 对象形) 是主入口: 设了就必须解析并通过校验, 否则
//!   加载失败;
//! * 数据目录的 `client-tokens.json` 是次入口, 走存储文档单文档语义
//!   (身份 + 信封自洽 + 版本契约): 坏文件**拒绝打开**, 0 静默回退默认。
//!
//! 本批口径 = **单令牌族多枚**: 令牌表与 env 单令牌面 (`APEIRETH_GATEWAY_TOKEN`)
//! 同属一个令牌族, 族内任一枚令牌都可过门, 审计带**客户端名** (表内条目用各自
//! `name`, env 面单令牌固定名 [`GatewayTokenGate::ENV_FACE_CLIENT_NAME`])。
//!
//! **W3-② 用户身份与角色**: 令牌表条目可声明 `user_id` (name → token → user_id
//! → role 的链), 用户表 (`users.json`, [`apeireth_governance::UserTable`]) 声明
//! `user_id ↔ role ↔ client_names`; 两侧一致性由
//! [`validate_client_user_bindings`] 装载期校验 (不一致拒开)。授权判定走治理层
//! role 权限矩阵 ([`apeireth_governance::UserRole::grants`]), 本面只做身份解析
//! 与审计落账。
//!
//! 令牌值永不进日志 / 永不进 Debug / 永不进启动横幅 —— 只有客户端名可呈现。

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use apeireth_governance::{UserAccessError, UserCapability, UserIdentity, UserRole, UserTable};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::canonical_entry::GatewayTokenGate;

/// 客户端令牌表的环境变量入口 (JSON 数组或 `{"clients": [...]}` 对象)。
pub const CLIENT_TOKENS_ENV: &str = "APEIRETH_GATEWAY_TOKENS";

/// 数据目录里的客户端令牌表文件名。
pub const CLIENT_TOKENS_FILE: &str = "client-tokens.json";

/// 客户端令牌表文件的存储文档身份。
pub const CLIENT_TOKENS_DOC_NAME: &str = "client-tokens";

/// 客户端令牌表文件的格式版本。
pub const CLIENT_TOKENS_DOC_VERSION: u32 = 1;

/// 一枚客户端令牌 (wire 形状: `{name, token, user_id?}`)。
///
/// `Debug` 手写脱敏: 令牌值只呈现 `token=<redacted>`。
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientTokenSpec {
    /// 客户端名 (审计署名键; 非空、≤64 字节、全表唯一)。
    pub name: String,
    /// 该客户端出示的 Bearer 令牌 (非空白、不得取逃生阀哨兵值、全表唯一)。
    pub token: String,
    /// 用户绑定 (W3-② 链的 name → token → **user_id** 步; 可缺省 = 未声明,
    /// 身份解析回落到用户表 `client_names` 绑定, 见 [`resolve_user_identity`])。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl ClientTokenSpec {
    /// 结构校验: 名与令牌非空白, 名不超长, 令牌不冒充逃生阀哨兵;
    /// 声明了 `user_id` 就必须非空白且不超长 (坏绑定不带病装载)。
    pub fn validate(&self) -> Result<(), ClientTokenError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(ClientTokenError::InvalidSpec {
                name: "<unnamed>".to_string(),
                reason: "client name must not be empty".to_string(),
            });
        }
        if name.len() > 64 {
            return Err(ClientTokenError::InvalidSpec {
                name: name.to_string(),
                reason: "client name must be at most 64 bytes".to_string(),
            });
        }
        let token = self.token.trim();
        if token.is_empty() {
            return Err(ClientTokenError::InvalidSpec {
                name: name.to_string(),
                reason: "client token must not be empty".to_string(),
            });
        }
        if token == GatewayTokenGate::UNPROTECTED_SENTINEL {
            return Err(ClientTokenError::SentinelToken {
                name: name.to_string(),
            });
        }
        if let Some(user_id) = &self.user_id {
            let user_id = user_id.trim();
            if user_id.is_empty() {
                return Err(ClientTokenError::InvalidSpec {
                    name: name.to_string(),
                    reason: "bound user id must not be empty".to_string(),
                });
            }
            if user_id.len() > 64 {
                return Err(ClientTokenError::InvalidSpec {
                    name: name.to_string(),
                    reason: "bound user id must be at most 64 bytes".to_string(),
                });
            }
        }
        Ok(())
    }

    /// 脱敏摘要 (令牌只报 `configured`, 0 明文)。
    pub fn redacted_summary(&self) -> String {
        format!(
            "client \"{}\": token={}",
            self.name.trim(),
            if self.token.trim().is_empty() {
                "absent"
            } else {
                "configured"
            }
        )
    }
}

/// 令牌不进 Debug: 值一律打码 (与 [`GatewayTokenGate`] 同一口径)。
impl std::fmt::Debug for ClientTokenSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ClientTokenSpec {{ name: {:?}, token: <redacted> }}",
            self.name
        )
    }
}

/// 客户端令牌表 (配置主体)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientTokenConfig {
    /// 全部客户端令牌 (配置顺序)。
    #[serde(default)]
    pub clients: Vec<ClientTokenSpec>,
}

impl ClientTokenConfig {
    /// 显式列表构造。
    pub fn new(clients: Vec<ClientTokenSpec>) -> Self {
        Self { clients }
    }

    /// 解析 JSON (env 的数组形或文件的 `{"clients": [...]}` 形), 然后校验。
    /// fail-closed: 任何缺陷都是错误, 0 回退默认。
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, ClientTokenError> {
        let config: Self = match value {
            serde_json::Value::Array(items) => Self {
                clients: serde_json::from_value(serde_json::Value::Array(items)).map_err(|e| {
                    ClientTokenError::InvalidJson {
                        reason: e.to_string(),
                    }
                })?,
            },
            serde_json::Value::Object(_) => {
                serde_json::from_value(value).map_err(|e| ClientTokenError::InvalidJson {
                    reason: e.to_string(),
                })?
            }
            other => {
                return Err(ClientTokenError::InvalidJson {
                    reason: format!("expected a JSON array or object, found {other}"),
                })
            }
        };
        config.validate()?;
        Ok(config)
    }

    /// 解析 `APEIRETH_GATEWAY_TOKENS` 原始值。fail-closed。
    pub fn from_env_value(raw: &str) -> Result<Self, ClientTokenError> {
        let value: serde_json::Value =
            serde_json::from_str(raw).map_err(|e| ClientTokenError::InvalidJson {
                reason: format!("{CLIENT_TOKENS_ENV}: {e}"),
            })?;
        Self::from_json_value(value)
    }

    /// 校验每枚令牌 + 拒绝重复名 / 重复令牌值。
    ///
    /// 重复令牌值是**审计歧义** (同一枚 Bearer 可归两个客户端名) —— 拒开,
    /// 不猜归谁。
    pub fn validate(&self) -> Result<(), ClientTokenError> {
        let mut names = std::collections::BTreeSet::new();
        let mut tokens = std::collections::BTreeSet::new();
        for spec in &self.clients {
            spec.validate()?;
            if !names.insert(spec.name.trim().to_string()) {
                return Err(ClientTokenError::DuplicateName {
                    name: spec.name.trim().to_string(),
                });
            }
            if !tokens.insert(spec.token.trim().to_string()) {
                return Err(ClientTokenError::DuplicateToken {
                    name: spec.name.trim().to_string(),
                });
            }
        }
        Ok(())
    }

    /// 主入口: 未设/空值 = `Ok(None)`; 有值但不合法 = 错误 (0 静默丢弃)。
    pub fn load_from_env() -> Result<Option<Self>, ClientTokenError> {
        match std::env::var(CLIENT_TOKENS_ENV) {
            Ok(raw) if raw.trim().is_empty() => Ok(None),
            Ok(raw) => Self::from_env_value(&raw).map(Some),
            Err(_) => Ok(None),
        }
    }

    /// 次入口: 数据目录文件缺失 = `Ok(None)`; 存在但拒开 = 错误。
    pub fn load_from_data_dir(data_dir: &Path) -> Result<Option<Self>, ClientTokenError> {
        let path = client_tokens_path(data_dir);
        if !path.exists() {
            return Ok(None);
        }
        let doc = stored_doc::open_single::<Self>(&path, &doc_compat())
            .map_err(ClientTokenError::Stored)?;
        doc.body.validate()?;
        Ok(Some(doc.body))
    }

    /// 主入口优先, 次入口兜底, 都没有 = 空表 (inert)。
    pub fn load(data_dir: Option<&Path>) -> Result<Self, ClientTokenError> {
        if let Some(config) = Self::load_from_env()? {
            return Ok(config);
        }
        if let Some(dir) = data_dir {
            if let Some(config) = Self::load_from_data_dir(dir)? {
                return Ok(config);
            }
        }
        Ok(Self::default())
    }

    /// 脱敏启动日志: 每客户端一行 (0 令牌明文)。
    pub fn redacted_startup_log(&self) -> Vec<String> {
        self.clients
            .iter()
            .map(ClientTokenSpec::redacted_summary)
            .collect()
    }
}

/// 数据目录里的客户端令牌表路径。
pub fn client_tokens_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CLIENT_TOKENS_FILE)
}

/// 客户端令牌表文件的读契约 (仅当前版本可读)。
pub fn doc_compat() -> DocCompat {
    DocCompat::exact(CLIENT_TOKENS_DOC_NAME, CLIENT_TOKENS_DOC_VERSION)
}

/// 过门时的具名令牌查找表 (校验后的运行时形态)。
///
/// [`authenticate`](Self::authenticate) 对每枚令牌做恒定时间比较, 命中即回
/// **客户端名** (审计署名键); 0 命中 = `None` (调用方按"伪造 Bearer"处置)。
#[derive(Debug, Clone, Default)]
pub struct ClientTokenTable {
    entries: Vec<ClientTokenSpec>,
}

impl ClientTokenTable {
    /// 从校验过的配置构造 (再校验一遍: 构造面不信任调用方)。
    pub fn from_config(config: ClientTokenConfig) -> Result<Self, ClientTokenError> {
        config.validate()?;
        Ok(Self {
            entries: config.clients,
        })
    }

    /// 从配置面装载 (env 主入口 / 数据目录次入口), fail-closed 拒开。
    pub fn load(data_dir: Option<&Path>) -> Result<Self, ClientTokenError> {
        Self::from_config(ClientTokenConfig::load(data_dir)?)
    }

    /// 表内客户端数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 空表 = 令牌表未配置 (不代表 env 单令牌面也空)。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 全部客户端名 (配置顺序; 只有名字, 无令牌)。
    pub fn names(&self) -> Vec<&str> {
        self.entries.iter().map(|spec| spec.name.trim()).collect()
    }

    /// 恒定时间认证: 命中回客户端名, 未命中回 `None`。
    ///
    /// 逐枚恒定时间比较 (不提前返回命中项的索引以外的时序信号); 表长在配置期
    /// 已知, 不构成秘密。
    pub fn authenticate(&self, bearer: &str) -> Option<&str> {
        let mut matched: Option<&str> = None;
        for spec in &self.entries {
            if constant_time_eq(spec.token.trim().as_bytes(), bearer.as_bytes()) {
                matched = Some(spec.name.trim());
            }
        }
        matched
    }

    /// 按客户端名取令牌条目 (W3-② 用户绑定步: name → token → user_id)。
    pub fn entry_for(&self, name: &str) -> Option<&ClientTokenSpec> {
        self.entries
            .iter()
            .find(|spec| spec.name.trim() == name.trim())
    }

    /// 全部条目 (配置顺序; 绑定校验与脱敏日志用)。
    pub fn entries(&self) -> &[ClientTokenSpec] {
        &self.entries
    }

    /// 脱敏启动日志 (令牌只报 `configured`)。
    pub fn redacted_startup_log(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(ClientTokenSpec::redacted_summary)
            .collect()
    }
}

impl From<ClientTokenTable> for ClientTokenConfig {
    fn from(table: ClientTokenTable) -> Self {
        Self {
            clients: table.entries,
        }
    }
}

/// 恒定时间字节比较 (防时序侧信道; 与令牌门口径同实现语义)。
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// 认证面配置一致性闸 (装配期 fail-closed): 逃生阀与令牌表互斥。
///
/// `APEIRETH_GATEWAY_TOKEN=none` 是"本网关明示不设防"的整面声明; 同时又配置
/// 客户端令牌表是**互相矛盾的配置** —— 表会被静默忽略 (假旋钮) 或被误当成
/// 设防 (谎报)。两者都不可接受: 一律拒开, 让操作者择一。
pub fn validate_gate_table_pair(
    gate: &GatewayTokenGate,
    clients: &ClientTokenTable,
) -> Result<(), ClientTokenError> {
    if *gate == GatewayTokenGate::UnprotectedDeclared && !clients.is_empty() {
        return Err(ClientTokenError::EscapeValveConflict {
            client_count: clients.len(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// W2-③ 会话归属面: 过门身份下行 + name↔token 绑定校验 + 会话关联审计行
// ---------------------------------------------------------------------------

/// `gateway.session_client` 审计行 (W2-③): 过门客户端名与会话事件的关联行 ——
/// "谁 (客户端令牌表 name) 在写/订阅哪个会话", W3 多用户授权的问责地基。
pub const SESSION_CLIENT_AUDIT_EVENT: &str = "gateway.session_client";

/// 客户端自称名请求头 (W2-③ 令牌隔离): 可选; 出现即触发 name↔token 绑定校验
/// ([`client_name_binds`]) —— 自称名不得冒名他人审计署名。
pub const CLIENT_NAME_CLAIM_HEADER: &str = "x-apeireth-client";

/// 过门身份 (W2-③): 令牌绑定的客户端署名键, 由令牌门中间件随请求扩展下行,
/// 处理器据此写 `gateway.session_client` 审计行。
///
/// 署名键**只来自令牌绑定** ([`ClientTokenTable::authenticate`] /
/// env 单令牌面), 请求自称名只做绑定校验、绝不改写署名。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientIdentity {
    /// 审计署名键 (令牌表 `name` / env 面单令牌 `env` / 未设防口径 `anonymous`)。
    pub client: String,
    /// 是否经令牌认证 (未设防口径放行 = false)。
    pub authenticated: bool,
}

impl ClientIdentity {
    /// 未过门调用的署名键 (内部直调, 无客户端身份可署)。
    pub const UNATTRIBUTED: &'static str = "unattributed";

    /// 一枚已认证身份 (令牌绑定名)。
    pub fn authenticated(client: impl Into<String>) -> Self {
        Self {
            client: client.into(),
            authenticated: true,
        }
    }

    /// 未设防口径的放行身份 (无令牌即无绑定, 署名如实为 `anonymous`)。
    pub fn anonymous() -> Self {
        Self {
            client: "anonymous".to_string(),
            authenticated: false,
        }
    }
}

/// name↔token 绑定校验 (W2-③ 令牌隔离): 自称名只有与令牌绑定名一致才可信。
///
/// 无绑定 (`bound = None`, 未设防口径) 时自称名**不可校验** → 一律不通过
/// (fail-closed: 无凭证的自称名就是冒名企图)。
pub fn client_name_binds(claimed: &str, bound: Option<&str>) -> bool {
    matches!(bound, Some(bound) if bound == claimed)
}

/// 请求扩展里的过门署名键 (无身份 = 未过门调用, 署名如实为 `unattributed`)。
pub fn identity_label(identity: Option<&axum::extract::Extension<ClientIdentity>>) -> &str {
    identity
        .map(|axum::extract::Extension(identity)| identity.client.as_str())
        .unwrap_or(ClientIdentity::UNATTRIBUTED)
}

/// 写一条 `gateway.session_client` 审计行 (W2-③ 会话归属面)。
///
/// `admission` 是该次会话写的写闸排队事实 (回合/设置写携带; 读与订阅类动作
/// 无写闸, 传 `None`)。无审计面 (内嵌直调) = 不写, 不伪造。
pub fn record_session_client(
    observations: Option<&crate::events::RuntimeObservationSink>,
    session: apeireth_core::kernel::SessionId,
    client: &str,
    action: &str,
    admission: Option<apeireth_runtime::canonical::SessionWriteAdmission>,
) {
    let Some(observations) = observations else {
        return;
    };
    let mut detail = format!("session={session} client={client} action={action}");
    if let Some(admission) = admission {
        detail.push_str(&format!(
            " queued={} position={} wait_ms={}",
            u8::from(admission.was_queued()),
            admission.queue_position,
            admission.waited.as_millis()
        ));
    }
    observations.buffer_audit(SESSION_CLIENT_AUDIT_EVENT, Some(detail));
}

// ---------------------------------------------------------------------------
// W3-② 用户身份与角色: 令牌→用户→角色链 + 会话归属 + user_action 审计行
// ---------------------------------------------------------------------------

/// `gateway.user_action` 审计行 (W3-② 授权面): 谁 (user) 以什么角色 (role)
/// 做什么 (action) —— 角色门每次裁决落一行, allow 与 deny 都留痕。
pub const USER_ACTION_AUDIT_EVENT: &str = "gateway.user_action";

/// 过门请求的用户上下文 (W3-②): 角色门中间件随请求扩展下行。
///
/// `user = None` = 该客户端未绑定用户 —— 角色面激活时写面/管理面/审批面
/// fail-closed 拒; 角色面未激活 = 兼容路径 (不新增拒绝面)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserContext {
    /// 令牌→用户→角色链解析出的问责身份 (未绑定 = `None`)。
    pub user: Option<UserIdentity>,
}

impl UserContext {
    /// 未绑定用户 (或角色面未激活) 的空上下文。
    pub fn unbound() -> Self {
        Self { user: None }
    }

    /// 解析出的用户 id (未绑定 = `None`)。
    pub fn user_id(&self) -> Option<&str> {
        self.user.as_ref().map(|user| user.user_id.as_str())
    }

    /// 解析出的角色 (未绑定 = `None`)。
    pub fn role(&self) -> Option<UserRole> {
        self.user.as_ref().map(|user| user.role)
    }

    /// role 权限矩阵判定 (未绑定 = fail-closed 拒)。
    pub fn allows(&self, capability: UserCapability) -> bool {
        self.user
            .as_ref()
            .is_some_and(|user| user.allows(capability))
    }
}

/// 令牌→用户→角色的解析链 (W3-②): 客户端名 → 令牌条目 `user_id` → 用户表角色。
///
/// 两步回落 (口径唯一): 令牌条目声明了 `user_id` 就以它为准 (装载期已由
/// [`validate_client_user_bindings`] 校验两侧一致); 未声明 (含 env 单令牌面
/// [`GatewayTokenGate::ENV_FACE_CLIENT_NAME`]) 按用户表 `client_names` 绑定
/// 查找。两步都无命中 = 未绑定 (`None`), 调用方 fail-closed 处置。
pub fn resolve_user_identity(
    client: &str,
    tokens: &ClientTokenTable,
    users: &UserTable,
) -> Option<UserIdentity> {
    if let Some(user_id) = tokens
        .entry_for(client)
        .and_then(|spec| spec.user_id.as_deref())
    {
        return users.identity_for_user(user_id);
    }
    users.identity_for_client(client)
}

/// 令牌表 ↔ 用户表绑定一致性 (装载期 fail-closed 拒开)。
///
/// 令牌条目声明的 `user_id` 必须在用户表内, 且该用户必须回绑这个客户端名
/// (`UserIdentity.client_names` 含该名) —— 两侧对不上 = 问责链断, 拒开不猜。
pub fn validate_client_user_bindings(
    tokens: &ClientTokenTable,
    users: &UserTable,
) -> Result<(), UserAccessError> {
    for spec in tokens.entries() {
        let Some(user_id) = spec.user_id.as_deref() else {
            continue;
        };
        let name = spec.name.trim();
        let identity =
            users
                .identity_for_user(user_id)
                .ok_or_else(|| UserAccessError::BindingMismatch {
                    reason: format!(
                    "client {name:?} declares user id {user_id:?}, which is not in the user table"
                ),
                })?;
        if !identity.client_names.iter().any(|bound| bound == name) {
            return Err(UserAccessError::BindingMismatch {
                reason: format!(
                    "client {name:?} declares user id {user_id:?}, but that user does not \
                     bind {name:?} in client_names"
                ),
            });
        }
    }
    Ok(())
}

/// 用户角色面 (W3-②) 的运行时形态。
///
/// * `users` —— 用户表: `None` = **角色面未激活** (兼容路径, W2-② 行为逐字
///   不变, 不新增拒绝面); `Some` = 激活 (令牌→用户→角色链生效, 写面/管理面/
///   审批面按 role 权限矩阵过闸);
/// * `session_users` —— 会话→用户绑定 (首个具名写者绑定; 跨用户审批拒的判定
///   依据, 见 [`apeireth_governance::may_resolve_approval`])。
#[derive(Debug, Clone, Default)]
pub struct UserAccess {
    users: Option<std::sync::Arc<UserTable>>,
    session_users: std::sync::Arc<
        std::sync::RwLock<std::collections::HashMap<apeireth_core::kernel::SessionId, String>>,
    >,
}

impl UserAccess {
    /// 角色面未激活 (兼容路径)。
    pub fn inactive() -> Self {
        Self::default()
    }

    /// 从用户表构造激活形态。
    pub fn from_table(users: UserTable) -> Self {
        Self {
            users: Some(std::sync::Arc::new(users)),
            session_users: std::sync::Arc::default(),
        }
    }

    /// 装载 (数据目录 `users.json`, 缺失 = 未激活) + 令牌绑定一致性校验。
    /// 坏表 / 绑定不一致 = 拒开 (fail-closed, 在可失败边界执行)。
    pub fn load(data_dir: &Path, tokens: &ClientTokenTable) -> Result<Self, UserAccessError> {
        let Some(users) = UserTable::load(data_dir)? else {
            return Ok(Self::inactive());
        };
        validate_client_user_bindings(tokens, &users)?;
        Ok(Self::from_table(users))
    }

    /// 角色面是否激活。
    pub fn is_active(&self) -> bool {
        self.users.is_some()
    }

    /// 用户表 (未激活 = `None`)。
    pub fn table(&self) -> Option<&UserTable> {
        self.users.as_deref()
    }

    /// 令牌→用户→角色链: 过门客户端名 → 问责身份 (未激活/未绑定 = `None`)。
    pub fn identity(&self, client: &str, tokens: &ClientTokenTable) -> Option<UserIdentity> {
        let users = self.users.as_deref()?;
        resolve_user_identity(client, tokens, users)
    }

    /// 会话→用户绑定 (首个具名写者赢): 返回**生效**的绑定用户 id。
    pub fn bind_session_user(
        &self,
        session: apeireth_core::kernel::SessionId,
        user_id: &str,
    ) -> String {
        let mut bound = self
            .session_users
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        bound
            .entry(session)
            .or_insert_with(|| user_id.to_string())
            .clone()
    }

    /// 会话绑定的用户 (无绑定 = `None`)。
    pub fn session_user(&self, session: apeireth_core::kernel::SessionId) -> Option<String> {
        self.session_users
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&session)
            .cloned()
    }
}

/// 写一条 `gateway.user_action` 审计行 (W3-② 授权面)。
///
/// `allowed` 是角色门裁决 (`allow` / `deny`); `domain` 是该行**所涉用户域**
/// (V2-c 域标注: 管理动作 = 目标域, 授权裁决 = 行为域; `None` = 无域标注)。
/// 标注进 detail 的 `domain=<id>` 记号 (域视图审计面据此归属, 见
/// [`crate::domain_context::audit_detail_domain`]); 取值有界截断 (外部可控
/// 输入不无界进审计行)。无审计面 (内嵌直调) = 不写, 不伪造。
pub fn record_user_action(
    observations: Option<&crate::events::RuntimeObservationSink>,
    user: Option<&str>,
    role: Option<UserRole>,
    action: &str,
    endpoint: &str,
    allowed: bool,
    domain: Option<&str>,
) {
    let Some(observations) = observations else {
        return;
    };
    let mut detail = format!(
        "user={} role={} action={} endpoint={} decision={}",
        user.unwrap_or("unbound"),
        role.map(UserRole::as_str).unwrap_or("unbound"),
        action,
        endpoint,
        if allowed { "allow" } else { "deny" },
    );
    if let Some(domain) = domain {
        detail.push_str(&format!(
            " domain={}",
            crate::canonical_entry::truncate_label(domain)
        ));
    }
    observations.buffer_audit(USER_ACTION_AUDIT_EVENT, Some(detail));
}

/// 客户端令牌配置缺陷。每个变体对加载都是致命的: 坏配置拒绝打开。
#[derive(Debug, Error)]
pub enum ClientTokenError {
    /// JSON 形状/解析不过。
    #[error("client token configuration is not valid JSON: {reason}")]
    InvalidJson {
        /// 解析器拒绝的原因。
        reason: String,
    },
    /// 单枚令牌结构非法。
    #[error("client token {name:?} is invalid: {reason}")]
    InvalidSpec {
        /// 客户端名 (或 `<unnamed>`)。
        name: String,
        /// 拒绝原因。
        reason: String,
    },
    /// 客户端名重复 (审计署名会歧义)。
    #[error("client name {name:?} is configured twice")]
    DuplicateName {
        /// 重复的客户端名。
        name: String,
    },
    /// 令牌值重复 (同一枚 Bearer 可归两个名 = 审计歧义)。
    #[error("client {name:?} repeats a token value already configured for another client")]
    DuplicateToken {
        /// 后出现的客户端名。
        name: String,
    },
    /// 令牌值冒充逃生阀哨兵 (`none`)。
    #[error("client token {name:?} must not use the escape-valve sentinel value")]
    SentinelToken {
        /// 冒充哨兵的客户端名。
        name: String,
    },
    /// 逃生阀 (`APEIRETH_GATEWAY_TOKEN=none`) 与令牌表互斥。
    #[error(
        "the escape valve ({env}=none) conflicts with a configured client token table \
         ({client_count} client token(s)): declare exactly one posture",
        env = GatewayTokenGate::ENV
    )]
    EscapeValveConflict {
        /// 冲突的令牌枚数。
        client_count: usize,
    },
    /// 存储文档拒开 (坏 JSON / 身份不符 / 版本不可读)。
    #[error("client token configuration file refused to open: {0}")]
    Stored(#[from] StoredDocError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(name: &str, token: &str) -> ClientTokenSpec {
        ClientTokenSpec {
            name: name.to_string(),
            token: token.to_string(),
            user_id: None,
        }
    }

    fn config(specs: Vec<ClientTokenSpec>) -> ClientTokenConfig {
        ClientTokenConfig::new(specs)
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-client-tokens-{tag}-{}",
            std::process::id()
        ));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn env_shape_parses_in_both_array_and_object_form() {
        let array = ClientTokenConfig::from_env_value(
            r#"[{"name":"tui","token":"tok-a"},{"name":"desk","token":"tok-b"}]"#,
        )
        .unwrap();
        assert_eq!(array.clients.len(), 2);
        assert_eq!(array.clients[0].name, "tui");

        let object =
            ClientTokenConfig::from_env_value(r#"{"clients":[{"name":"tui","token":"tok-a"}]}"#)
                .unwrap();
        assert_eq!(object.clients.len(), 1);
    }

    #[test]
    fn broken_configurations_fail_closed() {
        // 坏 JSON。
        assert!(ClientTokenConfig::from_env_value("not json").is_err());
        // 名空白 / 名超长。
        assert!(ClientTokenConfig::from_json_value(serde_json::json!([
            {"name": "  ", "token": "tok-a"}
        ]))
        .is_err());
        assert!(ClientTokenConfig::from_json_value(serde_json::json!([
            {"name": "x".repeat(65), "token": "tok-a"}
        ]))
        .is_err());
        // 令牌空白。
        assert!(ClientTokenConfig::from_json_value(serde_json::json!([
            {"name": "tui", "token": "   "}
        ]))
        .is_err());
        // 令牌冒充逃生阀哨兵。
        assert!(ClientTokenConfig::from_json_value(serde_json::json!([
            {"name": "tui", "token": "none"}
        ]))
        .is_err());
        // 重复名 / 重复令牌值。
        let duplicate_name = ClientTokenConfig::from_json_value(serde_json::json!([
            {"name": "tui", "token": "tok-a"},
            {"name": "tui", "token": "tok-b"}
        ]));
        assert!(matches!(
            duplicate_name,
            Err(ClientTokenError::DuplicateName { .. })
        ));
        let duplicate_token = ClientTokenConfig::from_json_value(serde_json::json!([
            {"name": "tui", "token": "tok-a"},
            {"name": "desk", "token": "tok-a"}
        ]));
        assert!(matches!(
            duplicate_token,
            Err(ClientTokenError::DuplicateToken { .. })
        ));
    }

    #[test]
    fn stored_doc_file_is_loaded_and_broken_file_refuses_to_open() {
        let dir = temp_dir("stored-doc");
        let path = client_tokens_path(&dir);

        // 好文件: 信封合规 → 读到配置。
        let good = stored_doc::StoredDoc::new(
            CLIENT_TOKENS_DOC_NAME,
            CLIENT_TOKENS_DOC_VERSION,
            vec![CLIENT_TOKENS_DOC_VERSION],
            config(vec![spec("tui", "tok-a")]),
        );
        fs_err::write(&path, serde_json::to_vec_pretty(&good).unwrap()).unwrap();
        let loaded = ClientTokenConfig::load(Some(&dir)).unwrap();
        assert_eq!(loaded.clients.len(), 1);

        // 坏文件 (非法 JSON): 拒开, 不回退空表。
        fs_err::write(&path, b"{ not json").unwrap();
        assert!(matches!(
            ClientTokenConfig::load(Some(&dir)),
            Err(ClientTokenError::Stored(_))
        ));

        // 身份不符: 同样拒开。
        let wrong_name = stored_doc::StoredDoc::new(
            "some-other-doc",
            CLIENT_TOKENS_DOC_VERSION,
            vec![CLIENT_TOKENS_DOC_VERSION],
            config(vec![spec("tui", "tok-a")]),
        );
        fs_err::write(&path, serde_json::to_vec_pretty(&wrong_name).unwrap()).unwrap();
        assert!(matches!(
            ClientTokenConfig::load(Some(&dir)),
            Err(ClientTokenError::Stored(_))
        ));

        // 缺失文件 = 未配置 (不是错误)。
        let absent = temp_dir("absent");
        assert!(ClientTokenConfig::load(Some(&absent))
            .unwrap()
            .clients
            .is_empty());
    }

    #[test]
    fn table_authenticates_each_named_client_and_redacts_values() {
        let table = ClientTokenTable::from_config(config(vec![
            spec("tui", "tok-a"),
            spec("desktop-shell", "tok-b"),
            spec("im-bridge", "tok-c"),
        ]))
        .unwrap();
        assert_eq!(table.len(), 3);
        assert_eq!(table.authenticate("tok-a"), Some("tui"));
        assert_eq!(table.authenticate("tok-b"), Some("desktop-shell"));
        assert_eq!(table.authenticate("tok-c"), Some("im-bridge"));
        assert_eq!(table.authenticate("tok-zzz"), None);
        assert_eq!(table.authenticate(""), None);

        // 脱敏: 名字可呈现, 令牌 0 明文。
        let log = table.redacted_startup_log().join("\n");
        assert!(log.contains("tui") && log.contains("desktop-shell"));
        for secret in ["tok-a", "tok-b", "tok-c"] {
            assert!(!log.contains(secret), "启动日志不得携带令牌: {log}");
        }
        let rendered = format!("{table:?}");
        for secret in ["tok-a", "tok-b", "tok-c"] {
            assert!(!rendered.contains(secret), "Debug 不得携带令牌: {rendered}");
        }
    }

    #[test]
    fn escape_valve_and_token_table_are_mutually_exclusive() {
        let empty = ClientTokenTable::default();
        let table = ClientTokenTable::from_config(config(vec![spec("tui", "tok-a")])).unwrap();

        // 逃生阀 + 令牌表 = 拒开 (配置自相矛盾)。
        let conflict = validate_gate_table_pair(&GatewayTokenGate::UnprotectedDeclared, &table);
        assert!(matches!(
            conflict,
            Err(ClientTokenError::EscapeValveConflict { client_count: 1 })
        ));

        // 其余组合放行。
        assert!(validate_gate_table_pair(&GatewayTokenGate::UnprotectedDeclared, &empty).is_ok());
        assert!(validate_gate_table_pair(&GatewayTokenGate::UnprotectedDefault, &table).is_ok());
        assert!(
            validate_gate_table_pair(&GatewayTokenGate::Enforced("tok-e".into()), &table).is_ok()
        );
    }

    // ------------------------------------------------------------------
    // W2-③ 令牌隔离: name↔token 绑定校验 (自称名不可冒名审计署名)
    // ------------------------------------------------------------------

    #[test]
    fn a_claimed_name_binds_only_to_its_own_token_identity() {
        // A 的令牌绑定名 = `desktop-a`: 自称 A 通过, 自称 B 拒 (冒名)。
        assert!(client_name_binds("desktop-a", Some("desktop-a")));
        assert!(!client_name_binds("desktop-b", Some("desktop-a")));
        // 无绑定 (未设防口径) = 不可校验 → 一律拒 (fail-closed)。
        assert!(!client_name_binds("desktop-b", None));
    }

    // ------------------------------------------------------------------
    // W3-② 令牌→用户→角色链 + 两侧绑定一致性 (装载期拒开)
    // ------------------------------------------------------------------

    fn bound_spec(name: &str, token: &str, user_id: &str) -> ClientTokenSpec {
        ClientTokenSpec {
            name: name.to_string(),
            token: token.to_string(),
            user_id: Some(user_id.to_string()),
        }
    }

    fn user_record(user_id: &str, role: &str, clients: &[&str]) -> apeireth_governance::UserRecord {
        apeireth_governance::UserRecord {
            user_id: user_id.to_string(),
            role: role.to_string(),
            client_names: clients.iter().map(|c| (*c).to_string()).collect(),
            domain: None,
        }
    }

    fn user_table(records: Vec<apeireth_governance::UserRecord>) -> apeireth_governance::UserTable {
        let table = apeireth_governance::UserTable::new(records);
        table.validate().expect("valid user table");
        table
    }

    #[test]
    fn the_token_chain_resolves_name_to_token_to_user_id_to_role() {
        let tokens = ClientTokenTable::from_config(config(vec![
            bound_spec("tui", "tok-a", "user-owner"),
            // 未声明 user_id: 按用户表 client_names 回落 (env 面单令牌面同路)。
            spec("desk", "tok-b"),
        ]))
        .unwrap();
        let users = user_table(vec![
            user_record("user-owner", "owner", &["tui"]),
            user_record("user-guest", "guest", &["desk", "env"]),
        ]);

        // name → token → user_id → role。
        let owner = resolve_user_identity("tui", &tokens, &users).expect("owner identity");
        assert_eq!(owner.user_id, "user-owner");
        assert_eq!(owner.role, apeireth_governance::UserRole::Owner);
        // 未声明 user_id 的条目按 client_names 回落。
        let guest = resolve_user_identity("desk", &tokens, &users).expect("guest identity");
        assert_eq!(guest.role, apeireth_governance::UserRole::Guest);
        // env 面单令牌面 (`env`) 无表内条目, 同样按 client_names 回落。
        let env_face = resolve_user_identity("env", &tokens, &users).expect("env identity");
        assert_eq!(env_face.user_id, "user-guest");
        // 未绑定客户端 = 无身份 (调用方 fail-closed)。
        assert!(resolve_user_identity("stranger", &tokens, &users).is_none());
    }

    #[test]
    fn token_and_user_bindings_must_agree_or_the_load_refuses() {
        let users = user_table(vec![
            user_record("user-a", "owner", &["tui"]),
            user_record("user-b", "member", &["desk"]),
        ]);

        // 一致: 通过。
        let consistent = ClientTokenTable::from_config(config(vec![
            bound_spec("tui", "tok-a", "user-a"),
            spec("desk", "tok-b"),
        ]))
        .unwrap();
        assert!(validate_client_user_bindings(&consistent, &users).is_ok());

        // 声明的 user_id 不在用户表: 拒。
        let ghost_user =
            ClientTokenTable::from_config(config(vec![bound_spec("tui", "tok-a", "user-ghost")]))
                .unwrap();
        assert!(matches!(
            validate_client_user_bindings(&ghost_user, &users),
            Err(apeireth_governance::UserAccessError::BindingMismatch { .. })
        ));

        // 声明的 user_id 存在但没回绑这个客户端名: 拒 (问责链断)。
        let wrong_pairing =
            ClientTokenTable::from_config(config(vec![bound_spec("tui", "tok-a", "user-b")]))
                .unwrap();
        assert!(matches!(
            validate_client_user_bindings(&wrong_pairing, &users),
            Err(apeireth_governance::UserAccessError::BindingMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn session_client_audit_rows_name_the_bound_client_and_the_session() {
        #[derive(Debug, Default)]
        struct RecordingAudit(std::sync::Mutex<Vec<(String, Option<String>)>>);

        #[async_trait::async_trait]
        impl crate::panels::AuditCommand for RecordingAudit {
            async fn append_audit(&self, event: &str, detail: Option<&str>) {
                self.0
                    .lock()
                    .unwrap()
                    .push((event.to_string(), detail.map(str::to_string)));
            }
        }

        let archive = std::sync::Arc::new(RecordingAudit::default());
        let sink = std::sync::Arc::new(crate::events::RuntimeObservationSink::new(
            None,
            Some(std::sync::Arc::clone(&archive) as std::sync::Arc<dyn crate::panels::AuditCommand>),
        ));
        let session = apeireth_core::kernel::SessionId::new();

        record_session_client(Some(sink.as_ref()), session, "tui", "turn", None);
        // 无审计面 = 不写 (内嵌直调不伪造审计行)。
        record_session_client(None, session, "ghost", "turn", None);

        sink.flush().await;
        let rows = archive.0.lock().unwrap().clone();
        assert_eq!(rows.len(), 1, "只写真实审计面的那一行");
        assert_eq!(rows[0].0, SESSION_CLIENT_AUDIT_EVENT);
        let detail = rows[0].1.clone().expect("detail");
        assert!(
            detail.contains(&format!("session={session}"))
                && detail.contains("client=tui")
                && detail.contains("action=turn"),
            "{detail}"
        );
    }
}
