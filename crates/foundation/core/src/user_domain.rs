//! 用户域模型 + 域注册表 + 单户兼容采用语义 (W3-① 多户数据隔离·星群数据层地基)。
//!
//! 架构定案 (**星群为骨、共享为翼**): 每户持有独立伙伴实例 (记忆/性格/配额硬隔离)。
//! 本模块是隔离的地基: **一户一域**, 每域自己的数据面。
//!
//! ## 目录形态 (钉死)
//!
//! ```text
//! <数据根>/                              ← 默认用户域的数据面 (原位, 见「采用」)
//! <数据根>/users/<user_id>/domain.json   ← 每域一份域文档 (StoredDoc 信封)
//! <数据根>/users/<user_id>/data/         ← 非默认域的数据面 (sessions.sqlite3 /
//!                                          cognitive.sqlite3 / …, 分库隔离)
//! ```
//!
//! ## 单户兼容「采用」语义 (迁移语义钉死: 标记为 default 域原位, 绝不丢数据)
//!
//! 升级到多户形态时, 既有单户数据面 (数据根原位的 sessions.sqlite3 /
//! cognitive.sqlite3 / 记忆库 / 凭据库)**不移动、不复制、不改名、不删除** ——
//! 首次打开注册表时只写一个「域标记」(`users/default/domain.json`), 把现有数据
//! **原位标记**为默认用户域 (`user_id = "default"`) 的数据面:
//!
//! - 零数据迁移 = 零丢失风险 (对抗断言: 打开前后存量文件逐字节不变);
//! - 升级平滑 (旧路径/env 覆写继续指到同一批文件, 旧版本二进制也可读);
//! - 幂等可重入 (标记已在场则不动; `created_at` 永不被覆写)。
//!
//! 默认域数据面因此是「原位」的: [`DomainLayout::data_dir`] 对默认域返回数据根
//! 自身, 其余域返回 `users/<id>/data/`。
//!
//! ## 坏域配置拒开 (fail-closed)
//!
//! 域文档走 [`crate::stored_doc`] 严格单文档语义: 畸形 JSON / 信封身份不符 /
//! 版本不符 / `user_id` 与目录不符 / 角色不可解析 —— 一律**拒绝打开**并报
//! [`UserDomainError::Invalid`], 不静默降级、不回退默认值。

use std::path::{Path, PathBuf};

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::kernel::Timestamp;
use crate::stored_doc::{self, DocCompat, DEFAULT_DOC_MODE};

/// 默认用户域 id: 单户兼容数据面 (数据根原位) 归属的域。
///
/// 该 id 由「采用」语义保留给既有单户数据, [`DomainRegistry::create`] 拒绝
/// 再建同名域。
pub const DEFAULT_DOMAIN_ID: &str = "default";

/// 默认用户域的显示名 (采用标记写入; 可被域文档改写)。
pub const DEFAULT_DOMAIN_DISPLAY_NAME: &str = "Default";

/// 域文档 (`users/<id>/domain.json`) 的 StoredDoc 身份与格式版本。
///
/// v1 **兼容加字段** (W3-③ 域级运行时覆盖): `persona_text` / `model` 是可选字段
/// (`serde(default)`), 既有域文档逐字可读; `deny_unknown_fields` 不松 ——
/// 未知字段仍然拒开 (坏域配置拒开语义不因加字段而软化)。
const DOMAIN_DOC_NAME: &str = "user-domain";
const DOMAIN_DOC_VERSION: u32 = 1;

/// 域级人设/性格文本的最大字符数 (超出 = 坏域配置拒开)。
pub const PERSONA_TEXT_MAX_CHARS: usize = 16_384;

/// 域级模型覆盖 id 的最大字符数 (超出 = 坏域配置拒开)。
pub const MODEL_OVERRIDE_MAX_CHARS: usize = 128;

fn domain_doc_compat() -> DocCompat {
    DocCompat::exact(DOMAIN_DOC_NAME, DOMAIN_DOC_VERSION)
}

/// 用户域 id: 已校验的目录级安全标识。
///
/// 校验规则 (全部钉死, 目的是让 id 可以安全地拼进文件系统路径):
/// - 1..=64 字节;
/// - 字符集 `[a-z0-9_-]`, 首字符必须是小写字母或数字;
/// - 因此 `.`、`..`、路径分隔符、大小写混淆、控制字符天然被拒。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomainUserId(String);

impl DomainUserId {
    /// 校验并包装一个用户域 id。
    pub fn new(raw: impl Into<String>) -> Result<Self, UserDomainError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(UserDomainError::Invalid(
                "user domain id must not be empty".to_string(),
            ));
        }
        if raw.len() > 64 {
            return Err(UserDomainError::Invalid(
                "user domain id must be at most 64 bytes".to_string(),
            ));
        }
        let mut chars = raw.chars();
        let first_ok = chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
        if !first_ok {
            return Err(UserDomainError::Invalid(format!(
                "user domain id {raw:?} must start with a lowercase letter or digit"
            )));
        }
        if !raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return Err(UserDomainError::Invalid(format!(
                "user domain id {raw:?} contains characters outside [a-z0-9_-]"
            )));
        }
        Ok(Self(raw))
    }

    /// 默认用户域的 id。
    pub fn default_domain() -> Self {
        Self(DEFAULT_DOMAIN_ID.to_string())
    }

    /// id 的字符串形态。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DomainUserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for DomainUserId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for DomainUserId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(|error| D::Error::custom(error.to_string()))
    }
}

/// 用户域角色 (域文档事实字段; 角色不足的域管理操作由 [`UserDomain::authorize`] 拒绝)。
///
/// **词表冻结** (与治理层 `UserRole` 同一词表, 口径唯一): `owner` / `member` /
/// `guest`。域面**建立**角色 (本枚举), 治理层经宽接口
/// (`UserIdentity::from_wide(user_id, role, client_names)`) **消费**角色 ——
/// 角色只由域面建立, 授权面不代建、不默认降级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DomainRole {
    /// 域主: 域内数据面 + 域管理操作。
    Owner,
    /// 域成员: 域内数据面读写; 域管理操作被拒 (403 语义)。
    Member,
    /// 域访客: 域内只读; 域管理操作被拒 (403 语义)。
    Guest,
}

impl DomainRole {
    /// 角色的线格式 (与治理层冻结词表同形: `owner` / `member` / `guest`)。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Member => "member",
            Self::Guest => "guest",
        }
    }
}

/// 域级操作族: 角色门的判定输入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainOp {
    /// 域数据面读写 (会话/记忆/配额)。域内成员即可。
    Data,
    /// 域管理操作 (域元数据/域生命周期)。需要 [`DomainRole::Owner`]。
    Manage,
}

/// 一个用户域 (域文档 body, `users/<id>/domain.json`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserDomain {
    /// 域 id (与所在目录名一致; [`DomainRegistry::load`] 强制校验)。
    pub user_id: DomainUserId,
    /// 显示名 (1..=128 字符, 无控制字符)。
    pub display_name: String,
    /// 域创建时刻 (epoch 毫秒, 非负)。
    pub created_at: i64,
    /// 域角色。
    pub role: DomainRole,
    /// 域级性格/人设文本覆盖 (W3-③ 星群「每户各得其伴」): 缺省 (`None`) 回落
    /// 全局 (不注入人设种子, 与既有行为逐字一致)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona_text: Option<String>,
    /// 域级模型覆盖 (W3-③): 缺省 (`None`) 回落全局默认模型。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl UserDomain {
    /// 构造并校验一个域 (创建/采用入口用)。域级运行时覆盖默认缺省
    /// (回落全局), 需要时用 [`UserDomain::with_runtime_overrides`] 补。
    pub fn new(
        user_id: DomainUserId,
        display_name: impl Into<String>,
        created_at: i64,
        role: DomainRole,
    ) -> Result<Self, UserDomainError> {
        let domain = Self {
            user_id,
            display_name: display_name.into(),
            created_at,
            role,
            persona_text: None,
            model: None,
        };
        domain.validate()?;
        Ok(domain)
    }

    /// 补域级运行时覆盖 (性格/人设文本 + 模型, 缺省回落全局) 并校验。
    pub fn with_runtime_overrides(
        mut self,
        persona_text: Option<String>,
        model: Option<String>,
    ) -> Result<Self, UserDomainError> {
        self.persona_text = persona_text;
        self.model = model;
        self.validate()?;
        Ok(self)
    }

    /// 域文档 body 的字段校验 (显示名长度/控制字符、时间戳非负、
    /// 运行时覆盖越界即拒 —— 坏域配置拒开, 不回退默认)。
    pub fn validate(&self) -> Result<(), UserDomainError> {
        let name_len = self.display_name.chars().count();
        if name_len == 0 || name_len > 128 {
            return Err(UserDomainError::Invalid(format!(
                "user domain {} display_name must be 1..=128 characters",
                self.user_id
            )));
        }
        if self.display_name.chars().any(char::is_control) {
            return Err(UserDomainError::Invalid(format!(
                "user domain {} display_name must not contain control characters",
                self.user_id
            )));
        }
        if self.created_at < 0 {
            return Err(UserDomainError::Invalid(format!(
                "user domain {} created_at must be non-negative",
                self.user_id
            )));
        }
        if let Some(persona_text) = &self.persona_text {
            let persona_len = persona_text.chars().count();
            if persona_len == 0 || persona_len > PERSONA_TEXT_MAX_CHARS {
                return Err(UserDomainError::Invalid(format!(
                    "user domain {} persona_text must be 1..={PERSONA_TEXT_MAX_CHARS} characters",
                    self.user_id
                )));
            }
        }
        if let Some(model) = &self.model {
            let model_len = model.chars().count();
            if model_len == 0 || model_len > MODEL_OVERRIDE_MAX_CHARS {
                return Err(UserDomainError::Invalid(format!(
                    "user domain {} model must be 1..={MODEL_OVERRIDE_MAX_CHARS} characters",
                    self.user_id
                )));
            }
            if model.trim() != model || model.chars().any(char::is_control) {
                return Err(UserDomainError::Invalid(format!(
                    "user domain {} model must be a bare model id (no surrounding whitespace, no control characters)",
                    self.user_id
                )));
            }
        }
        Ok(())
    }

    /// 是否是默认用户域 (单户兼容数据面)。
    pub fn is_default(&self) -> bool {
        self.user_id.as_str() == DEFAULT_DOMAIN_ID
    }

    /// 域级授权 (越权访问域 = 拒绝即帧, 且不泄露他域存在):
    ///
    /// - 目标域 ≠ 本域 → [`UserDomainError::Forbidden`], **无论目标域是否存在**
    ///   (响应帧恒定, 存在性不外泄);
    /// - 目标域 = 本域 + [`DomainOp::Manage`] 但角色不是 [`DomainRole::Owner`]
    ///   → [`UserDomainError::Forbidden`] (只说规则, 不指目标);
    /// - 目标域 = 本域 + [`DomainOp::Data`] → 放行 (域内成员即可)。
    ///
    /// 资源级越权 (他域会话/记忆/配额 id) 不经过这里: 分库隔离下按本域句柄查
    /// 他域资源读作**不存在** ([`UserDomainError::NotFound`] 语义, 与真不存在
    /// 同帧)。
    pub fn authorize(&self, target: &DomainUserId, op: DomainOp) -> Result<(), UserDomainError> {
        if self.user_id != *target {
            return Err(UserDomainError::Forbidden(
                "cross-domain addressing is refused".to_string(),
            ));
        }
        match op {
            DomainOp::Data => Ok(()),
            DomainOp::Manage => {
                if self.role == DomainRole::Owner {
                    Ok(())
                } else {
                    Err(UserDomainError::Forbidden(
                        "domain management requires the owner role".to_string(),
                    ))
                }
            }
        }
    }

    /// 角色词面 (宽接口消费形态: 治理层 `UserIdentity::from_wide` 的 `role` 入参)。
    pub fn role_str(&self) -> &'static str {
        self.role.as_str()
    }
}

/// 用户域面的结构化错误 (语义即帧: HTTP 映射在 [`UserDomainError::http_status`] 钉死)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserDomainError {
    /// 目标不可见 (不存在、或存在但不在本域可见面)。应答 404 uniform `not_found`
    /// —— 「不存在」与「在他域」同帧, 不泄露他域存在。
    NotFound(String),
    /// 越权 (跨域点名 / 角色不足)。应答 403 uniform `forbidden` —— 帧恒定,
    /// 不泄露目标域是否存在。
    Forbidden(String),
    /// 本域不可用 (域配置坏/拒开/未接线)。应答 403 `domain_refused`, 真因入
    /// 审计 (这是对**本域**的如实报因, 不涉及他域存在性)。
    Refused(String),
    /// 配置/校验错误 (坏域配置拒开的上报形态)。
    Invalid(String),
    /// 域已存在 (创建冲突)。
    AlreadyExists(String),
    /// 底层 IO 失败。
    Io(String),
}

impl UserDomainError {
    /// HTTP 状态映射 (越权语义钉死面): NotFound=404, Forbidden/Refused=403,
    /// Invalid=400, AlreadyExists=409, Io=500。
    pub fn http_status(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::Forbidden(_) | Self::Refused(_) => 403,
            Self::Invalid(_) => 400,
            Self::AlreadyExists(_) => 409,
            Self::Io(_) => 500,
        }
    }

    /// 真因文本 (审计/日志面; 响应帧用恒定文案, 见网关域帧构造)。
    pub fn reason(&self) -> &str {
        match self {
            Self::NotFound(reason)
            | Self::Forbidden(reason)
            | Self::Refused(reason)
            | Self::Invalid(reason)
            | Self::AlreadyExists(reason)
            | Self::Io(reason) => reason,
        }
    }
}

impl std::fmt::Display for UserDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Self::NotFound(_) => "not_found",
            Self::Forbidden(_) => "forbidden",
            Self::Refused(_) => "domain_refused",
            Self::Invalid(_) => "invalid_domain_config",
            Self::AlreadyExists(_) => "domain_exists",
            Self::Io(_) => "io",
        };
        write!(f, "{kind}: {}", self.reason())
    }
}

impl std::error::Error for UserDomainError {}

/// 域目录布局 (路径形态的唯一实现)。
///
/// 默认域的数据面是**原位**的 (数据根自身) —— 这就是单户兼容的零迁移语义;
/// 其余域的数据面是 `users/<id>/data/` (分库隔离硬)。
#[derive(Debug, Clone)]
pub struct DomainLayout {
    root: PathBuf,
}

impl DomainLayout {
    /// 以数据根构造布局。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// 数据根。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 域注册区 (`<root>/users/`)。
    pub fn users_dir(&self) -> PathBuf {
        self.root.join("users")
    }

    /// 单域目录 (`<root>/users/<id>/`)。
    pub fn domain_dir(&self, user_id: &DomainUserId) -> PathBuf {
        self.users_dir().join(user_id.as_str())
    }

    /// 域文档 (`<root>/users/<id>/domain.json`)。
    pub fn domain_file(&self, user_id: &DomainUserId) -> PathBuf {
        self.domain_dir(user_id).join("domain.json")
    }

    /// 域数据面: 默认域 = 数据根原位 (零迁移); 其余域 = `users/<id>/data/`。
    pub fn data_dir(&self, user_id: &DomainUserId) -> PathBuf {
        if user_id.as_str() == DEFAULT_DOMAIN_ID {
            self.root.clone()
        } else {
            self.domain_dir(user_id).join("data")
        }
    }
}

/// 域注册表: 域文档的打开/创建/校验, 以及首次启动的「采用」。
#[derive(Debug, Clone)]
pub struct DomainRegistry {
    layout: DomainLayout,
}

impl DomainRegistry {
    /// 打开数据根上的域注册表 (幂等):
    ///
    /// 1. **采用** (首次启动): `users/default/domain.json` 缺席时写入默认域标记,
    ///    把数据根原位的既有数据标记为默认用户域 —— **只写标记, 存量数据
    ///    零移动/零复制/零删除**;
    /// 2. 默认域标记必须可开 (坏域配置拒开: 畸形/身份不符/版本不符 →
    ///    [`UserDomainError::Invalid`], 数据原样不动)。
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, UserDomainError> {
        let registry = Self {
            layout: DomainLayout::new(root),
        };
        registry.adopt_default_domain()?;
        registry.load(&DomainUserId::default_domain())?;
        Ok(registry)
    }

    /// 布局。
    pub fn layout(&self) -> &DomainLayout {
        &self.layout
    }

    /// 采用 (幂等): 标记缺席才写; 标记在场一律不动 (`created_at` 不被覆写)。
    fn adopt_default_domain(&self) -> Result<(), UserDomainError> {
        let default_id = DomainUserId::default_domain();
        let file = self.layout.domain_file(&default_id);
        if file.exists() {
            return Ok(());
        }
        create_dir_all(&self.layout.domain_dir(&default_id))?;
        let now = Timestamp::now().epoch_millis();
        let domain = UserDomain::new(
            default_id,
            DEFAULT_DOMAIN_DISPLAY_NAME,
            now,
            DomainRole::Owner,
        )?;
        write_domain_doc(&file, &domain)
    }

    /// 打开一个域文档 (坏域配置拒开)。
    ///
    /// - 标记缺席 → [`UserDomainError::NotFound`];
    /// - 畸形/身份不符/版本不符/user_id 与目录不符/字段越界 →
    ///   [`UserDomainError::Invalid`] (拒绝打开, 不回退默认)。
    pub fn load(&self, user_id: &DomainUserId) -> Result<UserDomain, UserDomainError> {
        let file = self.layout.domain_file(user_id);
        if !file.exists() {
            return Err(UserDomainError::NotFound(format!(
                "user domain `{user_id}` does not exist"
            )));
        }
        let doc = stored_doc::open_single::<UserDomain>(&file, &domain_doc_compat())
            .map_err(|error| self.refused(user_id, error))?;
        if doc.body.user_id != *user_id {
            return Err(UserDomainError::Invalid(format!(
                "user domain config `{user_id}` declares user_id `{}` (mismatched with its directory)",
                doc.body.user_id
            )));
        }
        doc.body.validate()?;
        Ok(doc.body)
    }

    /// 打开默认用户域 (单户兼容面)。
    pub fn load_default(&self) -> Result<UserDomain, UserDomainError> {
        self.load(&DomainUserId::default_domain())
    }

    /// 注册区内的域 id 全表 (目录名即 id; 目录名不是合法 id = 坏域配置拒开)。
    pub fn domain_ids(&self) -> Result<Vec<DomainUserId>, UserDomainError> {
        let dir = self.layout.users_dir();
        let entries = read_dir(&dir)?;
        let mut ids = Vec::new();
        for entry in entries {
            if !entry_is_dir(&entry) {
                continue;
            }
            let name = entry_file_name(&entry);
            ids.push(DomainUserId::new(name).map_err(|error| {
                UserDomainError::Invalid(format!("users/ holds a non-domain entry: {error}"))
            })?);
        }
        ids.sort();
        Ok(ids)
    }

    /// 创建一个新用户域 (目录 `users/<id>/data/` + 域文档):
    ///
    /// - [`DEFAULT_DOMAIN_ID`] 保留给采用语义, 拒绝创建;
    /// - id 非法 / 显示名非法 → [`UserDomainError::Invalid`];
    /// - 域已存在 → [`UserDomainError::AlreadyExists`] (半成品目录同样拒绝,
    ///   不猜不覆盖)。
    pub fn create(
        &self,
        user_id: &str,
        display_name: &str,
        role: DomainRole,
    ) -> Result<UserDomain, UserDomainError> {
        let id = DomainUserId::new(user_id)?;
        if id.as_str() == DEFAULT_DOMAIN_ID {
            return Err(UserDomainError::Invalid(
                "domain id `default` is reserved for the adopted single-user domain".to_string(),
            ));
        }
        let now = Timestamp::now().epoch_millis();
        let domain = UserDomain::new(id.clone(), display_name, now, role)?;
        let domain_dir = self.layout.domain_dir(&id);
        if domain_dir.exists() {
            return Err(UserDomainError::AlreadyExists(format!(
                "user domain `{id}` already exists"
            )));
        }
        create_dir(&domain_dir)?;
        // 域文档写失败时清掉本次创建的空目录 (只清自己刚建的, 不碰任何存量)。
        if let Err(error) = create_dir_all(&self.layout.data_dir(&id))
            .and_then(|()| write_domain_doc(&self.layout.domain_file(&id), &domain))
        {
            let _ = std::fs::remove_dir_all(&domain_dir);
            return Err(error);
        }
        Ok(domain)
    }

    /// 写域级运行时覆盖 (性格/人设文本 + 模型, 缺省回落全局)。
    ///
    /// - 严格打开域文档 (坏域配置拒开, 语义与 [`DomainRegistry::load`] 一致);
    /// - 只改两个覆盖字段, `created_at` / 角色 / 显示名与 StoredDoc 信封**原样**
    ///   保留 (不重建信封, 不刷新任何时间戳);
    /// - 覆盖值越界 → [`UserDomainError::Invalid`], 文件零改动 (拒开不落盘)。
    pub fn set_runtime_overrides(
        &self,
        user_id: &DomainUserId,
        persona_text: Option<String>,
        model: Option<String>,
    ) -> Result<UserDomain, UserDomainError> {
        let file = self.layout.domain_file(user_id);
        if !file.exists() {
            return Err(UserDomainError::NotFound(format!(
                "user domain `{user_id}` does not exist"
            )));
        }
        let mut doc = stored_doc::open_single::<UserDomain>(&file, &domain_doc_compat())
            .map_err(|error| self.refused(user_id, error))?;
        if doc.body.user_id != *user_id {
            return Err(UserDomainError::Invalid(format!(
                "user domain config `{user_id}` declares user_id `{}` (mismatched with its directory)",
                doc.body.user_id
            )));
        }
        doc.body.persona_text = persona_text;
        doc.body.model = model;
        doc.body.validate()?;
        stored_doc::write_doc(&file, &doc, DEFAULT_DOC_MODE)
            .map_err(|error| UserDomainError::Io(format!("domain doc write failed: {error}")))?;
        Ok(doc.body)
    }

    fn refused(
        &self,
        user_id: &DomainUserId,
        error: stored_doc::StoredDocError,
    ) -> UserDomainError {
        UserDomainError::Invalid(format!("user domain `{user_id}` config refused: {error}"))
    }
}

fn write_domain_doc(path: &Path, domain: &UserDomain) -> Result<(), UserDomainError> {
    stored_doc::save_single(path, &domain_doc_compat(), domain, DEFAULT_DOC_MODE)
        .map(|_| ())
        .map_err(|error| UserDomainError::Io(format!("domain doc write failed: {error}")))
}

fn create_dir(path: &Path) -> Result<(), UserDomainError> {
    std::fs::create_dir(path)
        .map_err(|error| UserDomainError::Io(format!("create dir {}: {error}", path.display())))
}

fn create_dir_all(path: &Path) -> Result<(), UserDomainError> {
    std::fs::create_dir_all(path)
        .map_err(|error| UserDomainError::Io(format!("create dir {}: {error}", path.display())))
}

fn read_dir(path: &Path) -> Result<Vec<std::fs::DirEntry>, UserDomainError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(path)
        .map_err(|error| UserDomainError::Io(format!("read dir {}: {error}", path.display())))?;
    entries
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| UserDomainError::Io(format!("read dir {}: {error}", path.display())))
}

fn entry_is_dir(entry: &std::fs::DirEntry) -> bool {
    entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
}

fn entry_file_name(entry: &std::fs::DirEntry) -> String {
    entry.file_name().to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn test_root(tag: &str) -> PathBuf {
        let n = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "apeireth-user-domain-{tag}-{}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("创建测试数据根");
        dir
    }

    /// 递归快照一个目录树: 相对路径 → 文件字节。
    fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in std::fs::read_dir(dir).expect("快照读目录") {
                let entry = entry.expect("快照条目");
                let path = entry.path();
                if path.is_dir() {
                    walk(base, &path, out);
                } else {
                    let rel = path.strip_prefix(base).expect("相对路径").to_path_buf();
                    out.insert(rel, std::fs::read(&path).expect("快照读文件"));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    /// 造一个「既有单户数据面」: 数据根上若干存量文件 (会话库/记忆库/凭据库等)。
    fn legacy_data_root(tag: &str) -> (PathBuf, BTreeMap<PathBuf, Vec<u8>>) {
        let root = test_root(tag);
        std::fs::write(root.join("sessions.sqlite3"), b"legacy-sessions-bytes").unwrap();
        std::fs::write(root.join("cognitive.sqlite3"), b"legacy-memory-bytes").unwrap();
        std::fs::write(root.join("creds.json"), b"legacy-secret-bytes").unwrap();
        std::fs::create_dir_all(root.join("reflexion")).unwrap();
        std::fs::write(root.join("reflexion/reflexions.json"), b"legacy-lessons").unwrap();
        let before = snapshot_tree(&root);
        (root, before)
    }

    // ------------------------------------------------------------------
    // 域创建
    // ------------------------------------------------------------------

    #[test]
    fn creating_a_domain_provisions_its_own_data_plane_and_marker() {
        let root = test_root("create");
        let registry = DomainRegistry::open(&root).unwrap();
        let domain = registry
            .create("alice", "Alice", DomainRole::Owner)
            .unwrap();
        assert_eq!(domain.user_id.as_str(), "alice");
        assert_eq!(domain.role, DomainRole::Owner);
        assert!(!domain.is_default());

        let id = DomainUserId::new("alice").unwrap();
        assert!(registry.layout().data_dir(&id).is_dir());
        assert!(registry.layout().domain_file(&id).is_file());
        let loaded = registry.load(&id).unwrap();
        assert_eq!(loaded, domain);
        assert!(registry.domain_ids().unwrap().contains(&id));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_default_domain_id_is_reserved_and_bad_ids_are_refused() {
        let root = test_root("reserved");
        let registry = DomainRegistry::open(&root).unwrap();
        assert!(matches!(
            registry.create(DEFAULT_DOMAIN_ID, "x", DomainRole::Owner),
            Err(UserDomainError::Invalid(_))
        ));
        // 路径穿越 / 大写 / 空 / 超长 / 分隔符 —— 全部拒绝, 永不落盘。
        for bad in ["", "..", "../evil", "a/b", "a\\b", "ALICE", "爱丽丝", "-x"] {
            assert!(DomainUserId::new(bad).is_err(), "id {bad:?} 必须被拒绝");
            assert!(registry.create(bad, "x", DomainRole::Member).is_err());
        }
        let long = "a".repeat(65);
        assert!(DomainUserId::new(long).is_err());
        assert!(!root.join("users/evil").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    // ------------------------------------------------------------------
    // 兼容迁移 (采用): 现有数据入 default 域零丢失 (对抗断言)
    // ------------------------------------------------------------------

    #[test]
    fn first_launch_adopts_the_legacy_data_root_in_place_with_zero_data_loss() {
        let (root, before) = legacy_data_root("adopt");
        let registry = DomainRegistry::open(&root).unwrap();

        // 对抗断言 ①: 存量文件逐字节不变、无一被移走/删除 (只有新增的 users/ 标记)。
        let after = snapshot_tree(&root);
        for (path, bytes) in &before {
            assert_eq!(
                after.get(path),
                Some(bytes),
                "存量文件必须原样保留: {}",
                path.display()
            );
        }

        // 对抗断言 ②: 采用结果 = 默认域标记在场, 数据面仍指数据根原位。
        let default = registry.load_default().unwrap();
        assert_eq!(default.user_id.as_str(), DEFAULT_DOMAIN_ID);
        assert_eq!(default.role, DomainRole::Owner);
        assert_eq!(
            registry.layout().data_dir(&default.user_id),
            root,
            "默认域数据面 = 原位 (零迁移)"
        );

        // 对抗断言 ③: 新增文件只有域标记本身 (users/default/domain.json)。
        let added: Vec<PathBuf> = after
            .keys()
            .filter(|path| !before.contains_key(*path))
            .cloned()
            .collect();
        assert_eq!(
            added,
            vec![PathBuf::from("users/default/domain.json")],
            "采用只写标记, 不搬数据"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn adoption_is_idempotent_and_never_rewrites_the_marker() {
        let (root, _) = legacy_data_root("adopt-idempotent");
        let first = DomainRegistry::open(&root).unwrap();
        let marker = root.join("users/default/domain.json");
        let marker_bytes = std::fs::read(&marker).unwrap();
        let adopted = first.load_default().unwrap();

        let second = DomainRegistry::open(&root).unwrap();
        assert_eq!(
            std::fs::read(&marker).unwrap(),
            marker_bytes,
            "二次打开不得覆写标记 (created_at 不被刷新)"
        );
        assert_eq!(second.load_default().unwrap(), adopted);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn adoption_still_marks_the_default_domain_on_a_fresh_root() {
        let root = test_root("adopt-fresh");
        let registry = DomainRegistry::open(&root).unwrap();
        assert_eq!(
            registry.load_default().unwrap().user_id.as_str(),
            DEFAULT_DOMAIN_ID
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    // ------------------------------------------------------------------
    // 坏域配置拒开
    // ------------------------------------------------------------------

    #[test]
    fn a_corrupt_default_marker_refuses_to_open_and_leaves_data_untouched() {
        let (root, before) = legacy_data_root("bad-default");
        DomainRegistry::open(&root).unwrap();
        let marker = root.join("users/default/domain.json");
        std::fs::write(&marker, b"{ this is not json").unwrap();
        let corrupt = snapshot_tree(&root);

        let err = DomainRegistry::open(&root).unwrap_err();
        assert!(
            matches!(err, UserDomainError::Invalid(_)),
            "坏域标记必须拒开: {err}"
        );
        assert_eq!(snapshot_tree(&root), corrupt, "拒开路径不得改动任何数据");
        assert_eq!(
            snapshot_tree(&root).get(&PathBuf::from("sessions.sqlite3")),
            before.get(&PathBuf::from("sessions.sqlite3"))
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_domain_marker_that_mismatches_its_directory_is_refused() {
        let root = test_root("mismatch");
        let registry = DomainRegistry::open(&root).unwrap();
        registry
            .create("alice", "Alice", DomainRole::Member)
            .unwrap();
        let file = root.join("users/alice/domain.json");
        let doc = stored_doc::open_single::<UserDomain>(&file, &domain_doc_compat()).unwrap();
        let mut swapped = doc.clone();
        swapped.body.user_id = DomainUserId::new("bob").unwrap();
        stored_doc::write_doc(&file, &swapped, DEFAULT_DOC_MODE).unwrap();

        let id = DomainUserId::new("alice").unwrap();
        let err = registry.load(&id).unwrap_err();
        assert!(
            matches!(err, UserDomainError::Invalid(_)),
            "user_id 与目录不符必须拒开: {err}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn domain_configs_with_bad_fields_are_refused_without_default_fallback() {
        let root = test_root("bad-fields");
        let registry = DomainRegistry::open(&root).unwrap();
        registry
            .create("alice", "Alice", DomainRole::Member)
            .unwrap();
        let file = root.join("users/alice/domain.json");
        let id = DomainUserId::new("alice").unwrap();

        // 未知字段 (deny_unknown_fields) → 拒开。
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        value["body"]["surprise"] = serde_json::json!("x");
        std::fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            registry.load(&id),
            Err(UserDomainError::Invalid(_))
        ));

        // 角色不可解析 → 拒开。
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        value["body"].as_object_mut().unwrap().remove("surprise");
        value["body"]["role"] = serde_json::json!("wizard");
        std::fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            registry.load(&id),
            Err(UserDomainError::Invalid(_))
        ));

        // 负 created_at → 拒开 (不回退默认), 拒开后文件原样 (零改动)。
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        value["body"]["role"] = serde_json::json!("member");
        value["body"]["created_at"] = serde_json::json!(-1);
        std::fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let before = std::fs::read(&file).unwrap();
        assert!(matches!(
            registry.load(&id),
            Err(UserDomainError::Invalid(_))
        ));
        assert_eq!(std::fs::read(&file).unwrap(), before, "拒开不得改动文件");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_users_entry_that_is_not_a_domain_id_refuses_the_listing() {
        let root = test_root("junk-entry");
        let registry = DomainRegistry::open(&root).unwrap();
        std::fs::create_dir(root.join("users/坏名字")).unwrap();
        let err = registry.domain_ids().unwrap_err();
        assert!(matches!(err, UserDomainError::Invalid(_)), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    // ------------------------------------------------------------------
    // 越权不泄露 (404/403 语义)
    // ------------------------------------------------------------------

    #[test]
    fn cross_domain_addressing_is_forbidden_regardless_of_target_existence() {
        let root = test_root("gate");
        let registry = DomainRegistry::open(&root).unwrap();
        let alice = registry
            .create("alice", "Alice", DomainRole::Owner)
            .unwrap();

        let existing = DomainUserId::new("bob").unwrap();
        let missing = DomainUserId::new("nobody").unwrap();
        // bob 不存在; 另一个域创建后再点名 —— 两种情况的错误类型必须一致。
        let forbidden_missing = alice.authorize(&existing, DomainOp::Data);
        let created = registry.create("bob", "Bob", DomainRole::Member).unwrap();
        let forbidden_existing = alice.authorize(&existing, DomainOp::Data);
        assert!(matches!(
            forbidden_missing,
            Err(UserDomainError::Forbidden(_))
        ));
        assert!(matches!(
            forbidden_existing,
            Err(UserDomainError::Forbidden(_))
        ));
        assert_eq!(
            std::mem::discriminant(&forbidden_missing.unwrap_err()),
            std::mem::discriminant(&forbidden_existing.unwrap_err()),
            "目标存在与否不得改变拒绝形态 (存在性不泄露)"
        );
        assert!(matches!(
            alice.authorize(&missing, DomainOp::Data),
            Err(UserDomainError::Forbidden(_))
        ));

        // 域内: Data 放行; Manage 按角色判定。
        assert!(alice.authorize(&alice.user_id, DomainOp::Data).is_ok());
        assert!(alice.authorize(&alice.user_id, DomainOp::Manage).is_ok());
        assert!(created.authorize(&created.user_id, DomainOp::Data).is_ok());
        assert!(matches!(
            created.authorize(&created.user_id, DomainOp::Manage),
            Err(UserDomainError::Forbidden(_))
        ));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_error_frame_semantics_are_pinned_to_404_and_403() {
        assert_eq!(UserDomainError::NotFound("x".into()).http_status(), 404);
        assert_eq!(UserDomainError::Forbidden("x".into()).http_status(), 403);
        assert_eq!(UserDomainError::Refused("x".into()).http_status(), 403);
        assert_eq!(UserDomainError::Invalid("x".into()).http_status(), 400);
        assert_eq!(
            UserDomainError::AlreadyExists("x".into()).http_status(),
            409
        );
    }

    #[test]
    fn loading_a_missing_domain_reads_as_not_found() {
        let root = test_root("missing-domain");
        let registry = DomainRegistry::open(&root).unwrap();
        let err = registry
            .load(&DomainUserId::new("nobody").unwrap())
            .unwrap_err();
        assert!(matches!(err, UserDomainError::NotFound(_)), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_role_vocabulary_is_frozen_to_the_three_shared_terms() {
        // 与治理层冻结词表同一词面 (owner/member/guest): 域面建立的角色经宽接口
        // 消费时必须逐字可解析, 其余词面在域文档处就被拒。
        for role in [DomainRole::Owner, DomainRole::Member, DomainRole::Guest] {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, format!("\"{}\"", role.as_str()));
            assert!(serde_json::from_str::<DomainRole>(&json).is_ok());
        }
        assert_eq!(DomainRole::Owner.as_str(), "owner");
        assert_eq!(DomainRole::Member.as_str(), "member");
        assert_eq!(DomainRole::Guest.as_str(), "guest");
        for bad in ["\"admin\"", "\"Owner\"", "\"\"", "\"wizard\""] {
            assert!(
                serde_json::from_str::<DomainRole>(bad).is_err(),
                "{bad} 必须被拒 (冻结词表)"
            );
        }
        let root = test_root("role-vocab");
        let registry = DomainRegistry::open(&root).unwrap();
        let guest = registry
            .create("guesty", "Guesty", DomainRole::Guest)
            .unwrap();
        assert_eq!(guest.role_str(), "guest");
        let _ = std::fs::remove_dir_all(&root);
    }

    // ------------------------------------------------------------------
    // 域级运行时覆盖 (W3-③ 星群: persona/model 按域可配, 缺省回落全局)
    // ------------------------------------------------------------------

    #[test]
    fn domain_documents_accept_optional_runtime_overrides_and_keep_old_docs_readable() {
        let root = test_root("overrides-compat");
        let registry = DomainRegistry::open(&root).unwrap();
        let alice = registry
            .create("alice", "Alice", DomainRole::Owner)
            .unwrap();
        let id = alice.user_id.clone();

        // 兼容加字段: 无覆盖字段的既有域文档照常打开, 覆盖读作缺省 (回落全局)。
        let loaded = registry.load(&id).unwrap();
        assert_eq!(loaded.persona_text, None);
        assert_eq!(loaded.model, None);

        // 带覆盖的域文档往返: 字段如实读回, 其余字段零改动。
        let updated = registry
            .set_runtime_overrides(
                &id,
                Some("你是 Alice 的专属伙伴, 语气温和。".to_string()),
                Some("model-alice".to_string()),
            )
            .unwrap();
        assert_eq!(
            updated.persona_text.as_deref(),
            Some("你是 Alice 的专属伙伴, 语气温和。")
        );
        assert_eq!(updated.model.as_deref(), Some("model-alice"));
        let reloaded = registry.load(&id).unwrap();
        assert_eq!(reloaded, updated);
        assert_eq!(reloaded.created_at, alice.created_at);
        assert_eq!(reloaded.role, DomainRole::Owner);

        // 信封原样 (身份/版本不被重建): StoredDoc 信封可按原兼容面打开。
        let doc = stored_doc::open_single::<UserDomain>(
            &root.join("users/alice/domain.json"),
            &domain_doc_compat(),
        )
        .unwrap();
        assert_eq!(doc.body, updated);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn bad_runtime_override_values_refuse_to_open_without_touching_the_file() {
        let root = test_root("overrides-bad");
        let registry = DomainRegistry::open(&root).unwrap();
        let alice = registry
            .create("alice", "Alice", DomainRole::Owner)
            .unwrap();
        let id = alice.user_id.clone();
        let file = root.join("users/alice/domain.json");

        // 空模型 / 空人设 / 超长人设 / 模型带空白或控制字符 —— 一律拒开且文件零改动。
        let long_persona = "x".repeat(PERSONA_TEXT_MAX_CHARS + 1);
        for (persona, model) in [
            (None, Some(String::new())),
            (None, Some(" padded ".to_string())),
            (None, Some("bad\nmodel".to_string())),
            (None, Some("m".repeat(MODEL_OVERRIDE_MAX_CHARS + 1))),
            (Some(String::new()), None),
            (Some(long_persona.clone()), None),
        ] {
            let before = std::fs::read(&file).unwrap();
            let err = registry
                .set_runtime_overrides(&id, persona.clone(), model.clone())
                .unwrap_err();
            assert!(
                matches!(err, UserDomainError::Invalid(_)),
                "越界覆盖必须拒开: {err}"
            );
            assert_eq!(std::fs::read(&file).unwrap(), before, "拒开不得落盘");
        }

        // 未知字段仍然拒开 (deny_unknown_fields 不因兼容加字段而松)。
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        value["body"]["surprise"] = serde_json::json!("x");
        std::fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            registry.load(&id),
            Err(UserDomainError::Invalid(_))
        ));

        // 覆盖字段类型错 (model 是数字) —— 同样拒开。
        value["body"].as_object_mut().unwrap().remove("surprise");
        value["body"]["model"] = serde_json::json!(5);
        std::fs::write(&file, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            registry.load(&id),
            Err(UserDomainError::Invalid(_))
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn runtime_overrides_clear_back_to_global_fallback_without_rewriting_identity() {
        let root = test_root("overrides-clear");
        let registry = DomainRegistry::open(&root).unwrap();
        let alice = registry
            .create("alice", "Alice", DomainRole::Member)
            .unwrap();
        let id = alice.user_id.clone();
        registry
            .set_runtime_overrides(
                &id,
                Some("persona".to_string()),
                Some("model-alice".to_string()),
            )
            .unwrap();

        // 覆盖清空 = 回落全局; 身份字段 (created_at/role) 与信封零改动。
        let cleared = registry.set_runtime_overrides(&id, None, None).unwrap();
        assert_eq!(cleared.persona_text, None);
        assert_eq!(cleared.model, None);
        assert_eq!(cleared.created_at, alice.created_at);
        assert_eq!(cleared.role, DomainRole::Member);
        assert_eq!(registry.load(&id).unwrap(), cleared);

        // 不存在的域 = NotFound (不误建、不误写)。
        let missing = DomainUserId::new("nobody").unwrap();
        assert!(matches!(
            registry.set_runtime_overrides(&missing, None, Some("m".into())),
            Err(UserDomainError::NotFound(_))
        ));
        let _ = std::fs::remove_dir_all(&root);
    }
}
