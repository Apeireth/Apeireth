//! 用户身份与角色授权面 (W3-②): 冻结角色词表 + 角色→能力矩阵 + 用户表。
//!
//! 三个入口, 一份 fail-closed 契约:
//!
//! * [`UserRole`] —— **冻结词表** `owner` / `member` / `guest`, 未知角色一律拒绝
//!   ([`UserRole::parse`]), 不做"就近映射"、不默认降级;
//! * [`UserRole::grants`] —— 治理层 **role 权限矩阵** (角色→能力映射), 全表显式
//!   枚举, 测试逐档钉住; [`UserCapability`] 是这张矩阵的面 (不是运行时能力授权
//!   `Permission` 的面, 两者不混用);
//! * [`UserTable`] —— 用户表 (`users.json` 存储文档): 身份 + 信封自洽 + 版本契约,
//!   坏文件/未知角色/绑定歧义**拒绝打开**, 0 静默回退默认。
//!
//! [`UserIdentity::from_wide`] 是 W3-① 域模型 (`UserDomain.role: Option<String>`)
//! 的宽接口消费面: 角色**只由域面建立**, 本面只消费并过冻结词表 —— 未配置角色
//! 或未知角色都是 fail-closed 拒, 不存在"没角色就当最低权"的隐式降级。
//!
//! 授权判定一律 fail-closed: 能力不在该角色的显式授权清单里 = 拒。
//! 本面不含任何秘密 (角色/绑定名可呈现; 令牌值在 `client_tokens` 面, 不进本面)。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 角色词表 (**冻结**): `owner` / `member` / `guest`。
///
/// - `owner` —— 全权 (含用户管理与管理面);
/// - `member` —— 域内常规操作 (读写 + 工具 + git 写 L1 档 + 审批 L2 档);
/// - `guest` —— 只读 + 只读工具, **零写面**。
///
/// 词表冻结 = 只有这三个字面值; 任何其他值 (含大小写变体) 都是未知角色,
/// fail-closed 拒。要扩词表 = 改这张枚举 + 改矩阵 + 改测试, 三处同批。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// 全权: 矩阵全能力, 含用户管理与管理面; 跨用户审批仅此角色可为。
    Owner,
    /// 域内常规操作: 读写 + 工具 + git 写 L1 + 审批 L2; 无管理面。
    Member,
    /// 只读 + 只读工具: 零写面 (无写/写类工具/审批/管理)。
    Guest,
}

impl UserRole {
    /// 冻结词表全集 (顺序即文档顺序: owner / member / guest)。
    pub const VOCABULARY: [&'static str; 3] = ["owner", "member", "guest"];

    /// 词表字面值 (持久化与审计署名共用这一形态)。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Member => "member",
            Self::Guest => "guest",
        }
    }

    /// 解析冻结词表: 精确匹配 `owner`/`member`/`guest`, 其余 = 未知角色拒
    /// (fail-closed; 不做大小写归一, 词面即词表)。
    pub fn parse(raw: &str) -> Result<Self, UserAccessError> {
        match raw {
            "owner" => Ok(Self::Owner),
            "member" => Ok(Self::Member),
            "guest" => Ok(Self::Guest),
            other => Err(UserAccessError::UnknownRole {
                role: other.to_string(),
            }),
        }
    }

    /// 治理层 role 权限矩阵 (角色→能力映射, 全表显式)。
    ///
    /// | 能力 | owner | member | guest |
    /// |---|---|---|---|
    /// | `Read` 只读面 | ✓ | ✓ | ✓ |
    /// | `Write` 域内写面 | ✓ | ✓ | ✗ |
    /// | `ToolUse` 只读工具 | ✓ | ✓ | ✓ |
    /// | `ToolUseWrite` 写类工具 | ✓ | ✓ | ✗ |
    /// | `GitWriteL1` git 写·L1 档 | ✓ | ✓ | ✗ |
    /// | `ApprovalL2` 审批·L2 档 | ✓ | ✓ | ✗ |
    /// | `ManageUsers` 用户管理 | ✓ | ✗ | ✗ |
    /// | `AdminControl` 管理面 | ✓ | ✗ | ✗ |
    ///
    /// 判定 fail-closed: 不在该角色显式清单里的能力一律 false。
    pub fn grants(self, capability: UserCapability) -> bool {
        match self {
            Self::Owner => true,
            Self::Member => matches!(
                capability,
                UserCapability::Read
                    | UserCapability::Write
                    | UserCapability::ToolUse
                    | UserCapability::ToolUseWrite
                    | UserCapability::GitWriteL1
                    | UserCapability::ApprovalL2
            ),
            Self::Guest => matches!(capability, UserCapability::Read | UserCapability::ToolUse),
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 治理层角色矩阵的能力词表 (判定面; 与运行时能力授权 `Permission` 分层不混用)。
///
/// - 读写两档是域面基础权力 (`Read` / `Write`);
/// - 工具两档按副作用分级 (`ToolUse` 只读类 / `ToolUseWrite` 写类);
/// - `GitWriteL1` = git 写分级的第一档 (自动放行档), 更高档不入本表;
/// - `ApprovalL2` = 审批分级的第二档 (批准挂起审批), 跨用户归属另闸
///   ([`may_resolve_approval`]);
/// - `ManageUsers` / `AdminControl` = 管理面两族 (用户管理 / 全局开关与停机)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserCapability {
    /// 只读面 (查询 / 快照 / 事件流)。
    Read,
    /// 域内写面 (会话写 / 设置写 / 记忆写)。
    Write,
    /// 工具执行·只读类 (无副作用)。
    ToolUse,
    /// 工具执行·写类 (有副作用)。
    ToolUseWrite,
    /// git 写·第一档 (自动放行档)。
    GitWriteL1,
    /// 审批·第二档 (批准挂起审批)。
    ApprovalL2,
    /// 用户管理 (add / list / role / remove)。
    ManageUsers,
    /// 管理面 (全局开关 / 热配置 / 停机)。
    AdminControl,
}

impl UserCapability {
    /// 稳定标签 (审计行 `action=` 的署名键)。
    pub fn label(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::ToolUse => "tool_use",
            Self::ToolUseWrite => "tool_use_write",
            Self::GitWriteL1 => "git_write_l1",
            Self::ApprovalL2 => "approval_l2",
            Self::ManageUsers => "manage_users",
            Self::AdminControl => "admin_control",
        }
    }
}

/// 一个用户的问责身份: `user_id` ↔ 角色 ↔ 绑定客户端名集合。
///
/// 与客户端令牌表的接合口径 (W2-② `name → token` 升级为
/// `name → token → user_id → role`): 令牌表条目声明 `user_id`, 用户表声明
/// `client_names`, 两侧一致性由装载面校验 (`client_tokens` 面
/// `validate_client_user_bindings`), 不一致拒开。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserIdentity {
    /// 用户 id (审计署名键; 非空、≤64 字节)。
    pub user_id: String,
    /// 冻结词表角色。
    pub role: UserRole,
    /// 该用户绑定的客户端名 (每名非空、≤64 字节、全表唯一归属)。
    pub client_names: Vec<String>,
}

impl UserIdentity {
    /// 构造 + 结构校验 (名与绑定非空白、不超长、用户内绑定不重复)。
    pub fn new(
        user_id: &str,
        role: UserRole,
        client_names: Vec<String>,
    ) -> Result<Self, UserAccessError> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(UserAccessError::InvalidIdentity {
                name: "<unnamed>".to_string(),
                reason: "user id must not be empty".to_string(),
            });
        }
        if user_id.len() > 64 {
            return Err(UserAccessError::InvalidIdentity {
                name: user_id.to_string(),
                reason: "user id must be at most 64 bytes".to_string(),
            });
        }
        let mut seen = BTreeSet::new();
        let mut normalized = Vec::with_capacity(client_names.len());
        for client in client_names {
            let client = client.trim();
            if client.is_empty() {
                return Err(UserAccessError::InvalidIdentity {
                    name: user_id.to_string(),
                    reason: "bound client name must not be empty".to_string(),
                });
            }
            if client.len() > 64 {
                return Err(UserAccessError::InvalidIdentity {
                    name: user_id.to_string(),
                    reason: format!("bound client name {client:?} must be at most 64 bytes"),
                });
            }
            if !seen.insert(client.to_string()) {
                return Err(UserAccessError::InvalidIdentity {
                    name: user_id.to_string(),
                    reason: format!("bound client name {client:?} is listed twice"),
                });
            }
            normalized.push(client.to_string());
        }
        Ok(Self {
            user_id: user_id.to_string(),
            role,
            client_names: normalized,
        })
    }

    /// W3-① 域模型宽接口消费面: `UserDomain.role: Option<String>` → 冻结词表。
    ///
    /// fail-closed 两连: 角色未配置 ([`UserAccessError::RoleUnprovisioned`]) 与
    /// 未知角色 ([`UserAccessError::UnknownRole`]) 都是拒绝 —— 角色只由域面建立,
    /// 本面不代建、不默认降级。
    pub fn from_wide(
        user_id: &str,
        role: Option<&str>,
        client_names: Vec<String>,
    ) -> Result<Self, UserAccessError> {
        let role = match role {
            Some(raw) => UserRole::parse(raw)?,
            None => {
                return Err(UserAccessError::RoleUnprovisioned {
                    user_id: user_id.trim().to_string(),
                })
            }
        };
        Self::new(user_id, role, client_names)
    }

    /// 该身份是否获得某能力 (矩阵判定, fail-closed)。
    pub fn allows(&self, capability: UserCapability) -> bool {
        self.role.grants(capability)
    }
}

/// 跨用户审批归属闸 (G1 对抗面): 谁能批准哪个会话的挂起审批。
///
/// - `owner` = 全权, 可批任意会话的审批;
/// - 其余角色 = 有 [`UserCapability::ApprovalL2`] **且** 会话归属 == 本人
///   (跨用户审批一律拒: 复用他人审批 = 越权);
/// - 会话无归属 (未绑定用户) = 仅 owner (fail-closed, 不猜归属)。
pub fn may_resolve_approval(
    role: UserRole,
    resolver_user: &str,
    subject_user: Option<&str>,
) -> bool {
    if role == UserRole::Owner {
        return true;
    }
    role.grants(UserCapability::ApprovalL2) && subject_user == Some(resolver_user)
}

/// 用户表条目 (持久化形态; `role` 存词面, 打开时过冻结词表)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserRecord {
    /// 用户 id (全表唯一)。
    pub user_id: String,
    /// 角色词面 (`owner`/`member`/`guest`; 未知 = 拒开)。
    pub role: String,
    /// 绑定客户端名 (全表唯一归属; 可为空 = 尚未发令牌, 用户仍可先建)。
    #[serde(default)]
    pub client_names: Vec<String>,
    /// 域引用 (W3-① 域目录的宽接口: 本面只携引用不判域, 域语义归域面)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

impl UserRecord {
    /// 条目 → 问责身份 (角色过冻结词表; 结构校验同 [`UserIdentity::new`])。
    pub fn identity(&self) -> Result<UserIdentity, UserAccessError> {
        let role = UserRole::parse(&self.role)?;
        UserIdentity::new(&self.user_id, role, self.client_names.clone())
    }
}

/// 用户表 (配置主体)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserTable {
    /// 全部用户 (文件顺序)。
    #[serde(default)]
    pub users: Vec<UserRecord>,
}

/// 数据目录里的用户表文件名。
pub const USERS_FILE: &str = "users.json";

/// 用户表文件的存储文档身份。
pub const USERS_DOC_NAME: &str = "users";

/// 用户表文件的格式版本。
pub const USERS_DOC_VERSION: u32 = 1;

/// 数据目录里的用户表路径。
pub fn users_path(data_dir: &Path) -> PathBuf {
    data_dir.join(USERS_FILE)
}

/// 用户表文件的读契约 (仅当前版本可读)。
pub fn doc_compat() -> DocCompat {
    DocCompat::exact(USERS_DOC_NAME, USERS_DOC_VERSION)
}

impl UserTable {
    /// 显式列表构造。
    pub fn new(users: Vec<UserRecord>) -> Self {
        Self { users }
    }

    /// 校验全表 (fail-closed): 条目结构 + 冻结词表 + user_id 唯一 +
    /// 客户端名全表唯一归属 (一个客户端只能问责到一个用户)。
    pub fn validate(&self) -> Result<(), UserAccessError> {
        let mut user_ids = BTreeSet::new();
        let mut client_owner: BTreeSet<String> = BTreeSet::new();
        for record in &self.users {
            let identity = record.identity()?;
            if !user_ids.insert(identity.user_id.clone()) {
                return Err(UserAccessError::DuplicateUserId {
                    user_id: identity.user_id,
                });
            }
            for client in &identity.client_names {
                if !client_owner.insert(client.clone()) {
                    return Err(UserAccessError::DuplicateClientBinding {
                        client: client.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// 装载 (StoredDoc 拒开语义): 文件缺失 = `Ok(None)`; 存在但坏 = 错误。
    pub fn load(data_dir: &Path) -> Result<Option<Self>, UserAccessError> {
        let path = users_path(data_dir);
        if !path.exists() {
            return Ok(None);
        }
        let doc = stored_doc::open_single::<Self>(&path, &doc_compat())?;
        doc.body.validate()?;
        Ok(Some(doc.body))
    }

    /// 持久化 (存储文档信封 + 原子写)。
    pub fn save(&self, data_dir: &Path) -> Result<(), UserAccessError> {
        self.validate()?;
        stored_doc::save_single(
            &users_path(data_dir),
            &doc_compat(),
            self,
            stored_doc::DEFAULT_DOC_MODE,
        )?;
        Ok(())
    }

    /// 按用户 id 取问责身份。
    pub fn identity_for_user(&self, user_id: &str) -> Option<UserIdentity> {
        self.users
            .iter()
            .find(|record| record.user_id.trim() == user_id.trim())
            .and_then(|record| record.identity().ok())
    }

    /// 按绑定客户端名取问责身份 (令牌→用户→角色链的用户步)。
    pub fn identity_for_client(&self, client: &str) -> Option<UserIdentity> {
        let client = client.trim();
        self.users
            .iter()
            .find(|record| {
                record
                    .client_names
                    .iter()
                    .any(|bound| bound.trim() == client)
            })
            .and_then(|record| record.identity().ok())
    }

    /// owner 计数 (「不得清空最后一个 owner」不变量的判定基数)。
    pub fn owner_count(&self) -> usize {
        self.users
            .iter()
            .filter(|record| record.role.trim() == UserRole::Owner.as_str())
            .count()
    }

    /// 加一个用户 (重复 user_id / 重复绑定名 / 未知角色都拒)。
    pub fn add(&mut self, record: UserRecord) -> Result<(), UserAccessError> {
        self.users.push(record);
        if let Err(error) = self.validate() {
            self.users.pop();
            return Err(error);
        }
        Ok(())
    }

    /// 改一个用户的角色 (未知角色拒; **不得把最后一个 owner 降级** ——
    /// 用户表失主 = 管理面永久失守, 默认拒绝)。
    pub fn set_role(&mut self, user_id: &str, role: UserRole) -> Result<(), UserAccessError> {
        let index = self.index_of(user_id)?;
        let was_owner = self.users[index].role.trim() == UserRole::Owner.as_str();
        if was_owner && role != UserRole::Owner && self.owner_count() == 1 {
            return Err(UserAccessError::KeepLastOwner {
                action: format!("demote user {user_id:?} to {}", role.as_str()),
            });
        }
        let previous = std::mem::replace(&mut self.users[index].role, role.as_str().to_string());
        if let Err(error) = self.validate() {
            self.users[index].role = previous;
            return Err(error);
        }
        Ok(())
    }

    /// 删一个用户 (**不得删最后一个 owner**; 其余结构约束随 validate 复查)。
    pub fn remove(&mut self, user_id: &str) -> Result<UserRecord, UserAccessError> {
        let index = self.index_of(user_id)?;
        if self.users[index].role.trim() == UserRole::Owner.as_str() && self.owner_count() == 1 {
            return Err(UserAccessError::KeepLastOwner {
                action: format!("remove user {user_id:?}"),
            });
        }
        let removed = self.users.remove(index);
        Ok(removed)
    }

    fn index_of(&self, user_id: &str) -> Result<usize, UserAccessError> {
        self.users
            .iter()
            .position(|record| record.user_id.trim() == user_id.trim())
            .ok_or_else(|| UserAccessError::UserNotFound {
                user_id: user_id.trim().to_string(),
            })
    }
}

/// 用户身份/授权面缺陷。对装载与授权都是致命的: 坏表拒绝打开, 未知拒绝绝。
#[derive(Debug, Error)]
pub enum UserAccessError {
    /// 未知角色 (冻结词表之外的任何词面)。
    #[error(
        "unknown user role {role:?}; the frozen vocabulary is {}",
        UserRole::VOCABULARY.join("|")
    )]
    UnknownRole {
        /// 被拒的角色词面。
        role: String,
    },
    /// 角色未配置 (宽接口 `Option<String>` 取 `None`)。
    #[error("user {user_id:?} has no role provisioned; authorization fail-closed refuses an unroled identity")]
    RoleUnprovisioned {
        /// 缺角色的用户 id。
        user_id: String,
    },
    /// 身份结构非法 (空名 / 超长 / 用户内绑定重复)。
    #[error("user identity {name:?} is invalid: {reason}")]
    InvalidIdentity {
        /// 用户 id (或 `<unnamed>`)。
        name: String,
        /// 拒绝原因。
        reason: String,
    },
    /// user_id 重复 (问责署名会歧义)。
    #[error("user id {user_id:?} is defined twice")]
    DuplicateUserId {
        /// 重复的用户 id。
        user_id: String,
    },
    /// 客户端名绑给了多个用户 (问责归谁会歧义)。
    #[error("client name {client:?} is bound to more than one user")]
    DuplicateClientBinding {
        /// 重复归属的客户端名。
        client: String,
    },
    /// 用户不在表内。
    #[error("user {user_id:?} is not in the user table")]
    UserNotFound {
        /// 缺席的用户 id。
        user_id: String,
    },
    /// 拒绝清空最后一个 owner (用户表失主 = 管理面永久失守)。
    #[error("refusing to {action}: the user table must keep at least one owner")]
    KeepLastOwner {
        /// 被拒的动作描述。
        action: String,
    },
    /// 令牌表↔用户表绑定不一致 (声明的 user_id 缺席或未回绑客户端名)。
    #[error("client token binding is inconsistent with the user table: {reason}")]
    BindingMismatch {
        /// 拒绝原因。
        reason: String,
    },
    /// 存储文档拒开 (坏 JSON / 身份不符 / 版本不可读)。
    #[error("user table file refused to open: {0}")]
    Stored(#[from] StoredDocError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(user_id: &str, role: &str, clients: &[&str]) -> UserRecord {
        UserRecord {
            user_id: user_id.to_string(),
            role: role.to_string(),
            client_names: clients.iter().map(|c| (*c).to_string()).collect(),
            domain: None,
        }
    }

    fn table(records: Vec<UserRecord>) -> UserTable {
        let table = UserTable::new(records);
        table.validate().expect("valid table");
        table
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("apeireth-user-access-{tag}-{}", std::process::id()));
        let _ = fs_err::remove_dir_all(&dir);
        fs_err::create_dir_all(&dir).expect("temp dir");
        dir
    }

    // ------------------------------------------------------------------
    // 1. 角色词表冻结 + 未知角色 fail-closed
    // ------------------------------------------------------------------

    #[test]
    fn the_role_vocabulary_is_frozen_to_three_terms() {
        assert_eq!(UserRole::VOCABULARY, ["owner", "member", "guest"]);
        for role in [UserRole::Owner, UserRole::Member, UserRole::Guest] {
            assert_eq!(UserRole::parse(role.as_str()).unwrap(), role);
            assert_eq!(role.to_string(), role.as_str());
        }
    }

    #[test]
    fn unknown_roles_are_refused_fail_closed() {
        for raw in [
            "admin",
            "root",
            "Owner",
            "OWNER",
            "owner ",
            "",
            "superuser",
            "0",
        ] {
            assert!(
                matches!(
                    UserRole::parse(raw),
                    Err(UserAccessError::UnknownRole { .. })
                ),
                "{raw:?} 必须是未知角色拒绝"
            );
        }
    }

    // ------------------------------------------------------------------
    // 2. role 权限矩阵逐档 (owner/member/guest × 读/写/工具/管理 + 档位)
    // ------------------------------------------------------------------

    #[test]
    fn the_role_permission_matrix_is_pinned_row_by_row() {
        // (角色, 能力, 期望) —— 全表 24 格逐格钉住。
        let expected = [
            (UserRole::Owner, UserCapability::Read, true),
            (UserRole::Owner, UserCapability::Write, true),
            (UserRole::Owner, UserCapability::ToolUse, true),
            (UserRole::Owner, UserCapability::ToolUseWrite, true),
            (UserRole::Owner, UserCapability::GitWriteL1, true),
            (UserRole::Owner, UserCapability::ApprovalL2, true),
            (UserRole::Owner, UserCapability::ManageUsers, true),
            (UserRole::Owner, UserCapability::AdminControl, true),
            (UserRole::Member, UserCapability::Read, true),
            (UserRole::Member, UserCapability::Write, true),
            (UserRole::Member, UserCapability::ToolUse, true),
            (UserRole::Member, UserCapability::ToolUseWrite, true),
            (UserRole::Member, UserCapability::GitWriteL1, true),
            (UserRole::Member, UserCapability::ApprovalL2, true),
            (UserRole::Member, UserCapability::ManageUsers, false),
            (UserRole::Member, UserCapability::AdminControl, false),
            (UserRole::Guest, UserCapability::Read, true),
            (UserRole::Guest, UserCapability::Write, false),
            (UserRole::Guest, UserCapability::ToolUse, true),
            (UserRole::Guest, UserCapability::ToolUseWrite, false),
            (UserRole::Guest, UserCapability::GitWriteL1, false),
            (UserRole::Guest, UserCapability::ApprovalL2, false),
            (UserRole::Guest, UserCapability::ManageUsers, false),
            (UserRole::Guest, UserCapability::AdminControl, false),
        ];
        for (role, capability, granted) in expected {
            assert_eq!(
                role.grants(capability),
                granted,
                "矩阵格 [{role} × {capability:?}] 应为 {granted}"
            );
        }
    }

    #[test]
    fn guest_has_zero_write_surface_and_owner_has_everything() {
        // guest 零写面: 写/写类工具/git 写/审批/管理全拒, 只读与只读工具放行。
        for capability in [
            UserCapability::Write,
            UserCapability::ToolUseWrite,
            UserCapability::GitWriteL1,
            UserCapability::ApprovalL2,
            UserCapability::ManageUsers,
            UserCapability::AdminControl,
        ] {
            assert!(
                !UserRole::Guest.grants(capability),
                "guest 零写面: {capability:?}"
            );
        }
        // owner 全权: 矩阵内每个能力都放行。
        let all = [
            UserCapability::Read,
            UserCapability::Write,
            UserCapability::ToolUse,
            UserCapability::ToolUseWrite,
            UserCapability::GitWriteL1,
            UserCapability::ApprovalL2,
            UserCapability::ManageUsers,
            UserCapability::AdminControl,
        ];
        for capability in all {
            assert!(
                UserRole::Owner.grants(capability),
                "owner 全权: {capability:?}"
            );
        }
    }

    // ------------------------------------------------------------------
    // 3. 宽接口 (W3-① 接缝) + 令牌→用户→角色链的用户步
    // ------------------------------------------------------------------

    #[test]
    fn the_wide_role_interface_refuses_unprovisioned_and_unknown_roles() {
        // 角色由域面建立: 合法词面成身份。
        let identity =
            UserIdentity::from_wide("u-1", Some("member"), vec!["tui".to_string()]).unwrap();
        assert_eq!(identity.role, UserRole::Member);

        // 未配置角色 = fail-closed 拒 (不默认降级)。
        assert!(matches!(
            UserIdentity::from_wide("u-1", None, vec!["tui".to_string()]),
            Err(UserAccessError::RoleUnprovisioned { .. })
        ));
        // 未知角色 = fail-closed 拒。
        assert!(matches!(
            UserIdentity::from_wide("u-1", Some("admin"), vec!["tui".to_string()]),
            Err(UserAccessError::UnknownRole { .. })
        ));
    }

    #[test]
    fn identity_lookup_walks_user_id_and_bound_client_names() {
        let table = table(vec![
            record("owner-1", "owner", &["desktop-shell", "tui"]),
            record("guest-1", "guest", &["phone-web"]),
        ]);

        // user_id 步: user_id → role。
        let owner = table.identity_for_user("owner-1").unwrap();
        assert_eq!(owner.role, UserRole::Owner);
        // 客户端名步: name → user_id → role (令牌→用户→角色链的用户段)。
        let by_client = table.identity_for_client("tui").unwrap();
        assert_eq!(by_client.user_id, "owner-1");
        assert_eq!(by_client.role, UserRole::Owner);
        let guest = table.identity_for_client("phone-web").unwrap();
        assert_eq!(guest.role, UserRole::Guest);
        // 未绑定客户端 = 无身份 (调用方 fail-closed 处置)。
        assert!(table.identity_for_client("stranger").is_none());
        assert!(table.identity_for_user("ghost").is_none());
    }

    #[test]
    fn identity_structure_is_validated_fail_closed() {
        assert!(matches!(
            UserIdentity::new("", UserRole::Owner, vec![]),
            Err(UserAccessError::InvalidIdentity { .. })
        ));
        assert!(matches!(
            UserIdentity::new("x".repeat(65).as_str(), UserRole::Owner, vec![]),
            Err(UserAccessError::InvalidIdentity { .. })
        ));
        assert!(matches!(
            UserIdentity::new("u-1", UserRole::Owner, vec!["  ".to_string()]),
            Err(UserAccessError::InvalidIdentity { .. })
        ));
        assert!(matches!(
            UserIdentity::new(
                "u-1",
                UserRole::Owner,
                vec!["a".to_string(), "a".to_string()]
            ),
            Err(UserAccessError::InvalidIdentity { .. })
        ));
    }

    // ------------------------------------------------------------------
    // 4. 用户表: 坏表拒开 + 唯一性 + 管理操作 + 失主不变量
    // ------------------------------------------------------------------

    #[test]
    fn stored_user_table_round_trips_and_broken_tables_refuse_to_open() {
        let dir = temp_dir("stored-doc");
        let table = table(vec![record("owner-1", "owner", &["tui"])]);

        // 好文件: 信封合规 → 读到表; save 后可回读 (拒开语义的正向面)。
        table.save(&dir).unwrap();
        let loaded = UserTable::load(&dir).unwrap().expect("table present");
        assert_eq!(loaded, table);

        // 坏 JSON: 拒开, 不回退空表。
        let path = users_path(&dir);
        fs_err::write(&path, b"{ not json").unwrap();
        assert!(matches!(
            UserTable::load(&dir),
            Err(UserAccessError::Stored(_))
        ));

        // 身份不符 (别的文档身份): 同样拒开。
        let wrong_name = stored_doc::StoredDoc::new(
            "some-other-doc",
            USERS_DOC_VERSION,
            vec![USERS_DOC_VERSION],
            UserTable::default(),
        );
        fs_err::write(&path, serde_json::to_vec_pretty(&wrong_name).unwrap()).unwrap();
        assert!(matches!(
            UserTable::load(&dir),
            Err(UserAccessError::Stored(_))
        ));

        // 缺失文件 = 未配置 (不是错误)。
        let absent = temp_dir("absent");
        assert!(UserTable::load(&absent).unwrap().is_none());
    }

    #[test]
    fn unknown_roles_and_ambiguous_bindings_refuse_to_open() {
        // 未知角色的表: 拒开 (词表冻结)。
        let bad_role = UserTable::new(vec![record("u-1", "root", &["tui"])]);
        assert!(matches!(
            bad_role.validate(),
            Err(UserAccessError::UnknownRole { .. })
        ));

        // user_id 重复: 拒开。
        let dup_user = UserTable::new(vec![
            record("u-1", "owner", &["a"]),
            record("u-1", "member", &["b"]),
        ]);
        assert!(matches!(
            dup_user.validate(),
            Err(UserAccessError::DuplicateUserId { .. })
        ));

        // 一个客户端绑给两个用户 (问责歧义): 拒开。
        let dup_client = UserTable::new(vec![
            record("u-1", "owner", &["tui"]),
            record("u-2", "member", &["tui"]),
        ]);
        assert!(matches!(
            dup_client.validate(),
            Err(UserAccessError::DuplicateClientBinding { .. })
        ));
    }

    #[test]
    fn management_operations_keep_the_table_consistent_and_ownered() {
        let mut table = table(vec![record("owner-1", "owner", &["tui"])]);

        // add: 正常入表。
        table
            .add(record("member-1", "member", &["phone-web"]))
            .unwrap();
        assert_eq!(table.users.len(), 2);
        // add: 重复 user_id 拒。
        assert!(matches!(
            table.add(record("member-1", "guest", &["other"])),
            Err(UserAccessError::DuplicateUserId { .. })
        ));
        // add: 未知角色拒 (表不被污染)。
        assert!(matches!(
            table.add(record("member-2", "root", &["other"])),
            Err(UserAccessError::UnknownRole { .. })
        ));
        assert_eq!(table.users.len(), 2, "被拒的 add 不留痕");

        // role: 改角色生效。
        table.set_role("member-1", UserRole::Guest).unwrap();
        assert_eq!(
            table.identity_for_user("member-1").unwrap().role,
            UserRole::Guest
        );
        // role: 未知用户拒。
        assert!(matches!(
            table.set_role("ghost", UserRole::Guest),
            Err(UserAccessError::UserNotFound { .. })
        ));

        // remove: 删掉非 owner 成员。
        let removed = table.remove("member-1").unwrap();
        assert_eq!(removed.user_id, "member-1");
        assert!(matches!(
            table.remove("member-1"),
            Err(UserAccessError::UserNotFound { .. })
        ));
    }

    #[test]
    fn the_last_owner_can_never_be_removed_or_demoted() {
        let mut table = table(vec![
            record("owner-1", "owner", &["tui"]),
            record("member-1", "member", &["phone-web"]),
        ]);

        // 单 owner: 降级/删除都拒 (用户表不得失主)。
        assert!(matches!(
            table.set_role("owner-1", UserRole::Member),
            Err(UserAccessError::KeepLastOwner { .. })
        ));
        assert!(matches!(
            table.remove("owner-1"),
            Err(UserAccessError::KeepLastOwner { .. })
        ));

        // 第二个 owner 出现后, 第一个可退场。
        table.add(record("owner-2", "owner", &["desk-2"])).unwrap();
        table.set_role("owner-1", UserRole::Member).unwrap();
        table.remove("owner-1").unwrap();
        assert_eq!(table.owner_count(), 1);
    }

    // ------------------------------------------------------------------
    // 5. 跨用户审批归属闸
    // ------------------------------------------------------------------

    #[test]
    fn cross_user_approval_resolution_is_refused_for_non_owners() {
        // 本人会话: 有 ApprovalL2 的角色可批。
        assert!(may_resolve_approval(UserRole::Member, "u-a", Some("u-a")));
        // 跨用户: member 拒 (复用他人审批 = 越权)。
        assert!(!may_resolve_approval(UserRole::Member, "u-a", Some("u-b")));
        // 会话无归属: 仅 owner (fail-closed, 不猜归属)。
        assert!(!may_resolve_approval(UserRole::Member, "u-a", None));
        assert!(!may_resolve_approval(UserRole::Guest, "u-a", Some("u-a")));
        // owner 全权: 跨用户/无归属都可批。
        assert!(may_resolve_approval(UserRole::Owner, "u-o", Some("u-b")));
        assert!(may_resolve_approval(UserRole::Owner, "u-o", None));
    }
}
