//! 客户端凭证体系 (W2-②): 每客户端令牌表 (name → token) 的配置面与校验面。
//!
//! 两个入口, 一份 fail-closed 契约 (与 `im_channels.rs` 同族语义):
//!
//! * `APEIRETH_GATEWAY_TOKENS` (JSON 数组, 每项 `{name, token}`, 或
//!   `{"clients": [...]}` 对象形) 是主入口: 设了就必须解析并通过校验, 否则
//!   加载失败;
//! * 数据目录的 `client-tokens.json` 是次入口, 走存储文档单文档语义
//!   (身份 + 信封自洽 + 版本契约): 坏文件**拒绝打开**, 0 静默回退默认。
//!
//! 本批口径 = **单令牌族多枚**: 令牌表与 env 单令牌面 (`APEIRETH_GATEWAY_TOKEN`)
//! 同属一个令牌族, 族内任一枚令牌都可过门, 审计带**客户端名** (表内条目用各自
//! `name`, env 面单令牌固定名 [`GatewayTokenGate::ENV_FACE_CLIENT_NAME`])。
//! 为 W3 多用户授权留桩: 本表先落"谁在调用"的可问责身份, 权限分级后批接。
//!
//! 令牌值永不进日志 / 永不进 Debug / 永不进启动横幅 —— 只有客户端名可呈现。

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
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

/// 一枚客户端令牌 (wire 形状: `{name, token}`)。
///
/// `Debug` 手写脱敏: 令牌值只呈现 `token=<redacted>`。
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientTokenSpec {
    /// 客户端名 (审计署名键; 非空、≤64 字节、全表唯一)。
    pub name: String,
    /// 该客户端出示的 Bearer 令牌 (非空白、不得取逃生阀哨兵值、全表唯一)。
    pub token: String,
}

impl ClientTokenSpec {
    /// 结构校验: 名与令牌非空白, 名不超长, 令牌不冒充逃生阀哨兵。
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
