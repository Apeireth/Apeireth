//! 联系人合同面 (B3a 批) 的 CLI 侧适配。
//!
//! 与网关面 `apeireth_gateway::contact_panel` 同口径的契约七条:
//! 1. **身份由后端铸造** —— 客户端只提交展示字段; `contact_id` / `agent_id` 由宿主铸;
//! 2. **归档是终态** —— 归档是墓碑, 不可复活; 默认列表不含墓碑, 单体读面仍可读;
//! 3. **乐观并发** —— 写面必须带 `expected_revision`, 不符 ⇒ 409 且字段零变化;
//! 4. **读面不含存储位置** —— DTO 里没有根路径、没有内部档位、没有宿主私有字段;
//! 5. **绑定只存 id + 修订号** —— 不存工作区快照 (设计稿裁决 20), 每回合现查现校验;
//! 6. **开不出就是开不出** —— [`CliContactPanel::new_persistent`] 返回 `Err`, 调用方
//!    必须 fail-closed, 不许降级成进程内注册表假装成功;
//! 7. **存储故障不回显宿主路径** —— 领域 reason 里的绝对路径一律剥掉再进响应体。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::agent_contact::{
    Contact, ContactCapabilities, ContactError, ContactIn, ContactPatch, ContactRegistry,
    ContactStatus, ContactWorkspaceBinding, ContractMemoryScope, InMemoryContactRegistry,
};
use apeireth_core::kernel::ids::ContactId;
use apeireth_core::storage_atomic;
use apeireth_core::stored_doc;
use apeireth_gateway::{
    ContactArchiveCommand, ContactBindingChange, ContactBindingCommand, ContactCapabilitiesCommand,
    ContactCapabilitiesDto, ContactCreateCommand, ContactDto, ContactMemoryScopeDto, ContactPanel,
    ContactPanelError, ContactStatusDto, ContactUpdateCommand, ContactWorkspaceBindingDto,
    GatewayServices,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::gateway_panels::now_millis;

/// 联系人绑定档的文档名 (StoredDoc 信封身份, 防串档)。
const CONTACT_BINDING_DOC_NAME: &str = "agent-contact-bindings";
/// 联系人绑定档的格式版本。
const CONTACT_BINDING_DOC_VERSION: u32 = 1;

/// 一条工作区绑定 (只存 id + 修订号, **不存工作区快照** —— 设计稿裁决 20)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredContactBinding {
    /// 绑定的工作区身份。
    workspace_id: String,
    /// 绑定时该工作区的修订号。
    revision: u64,
}

/// 联系人绑定档正文。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ContactBindingOverlay {
    /// 联系人身份 → 绑定。
    #[serde(default)]
    bindings: BTreeMap<String, StoredContactBinding>,
}

/// 联系人面适配 (B3a 批)。
///
/// 真落盘:
/// - 联系人记录走 B1 的 [`FileContactRegistry`] (`<data_dir>/agents/contacts/<id>/contact.json`);
/// - PATCH 将工作区绑定与其它字段写进同一份联系人记录。
/// - 旧记录和现有登记流程的绑定可从宿主侧绑定档读取。
///
/// 绑定档**只存 id + 修订号**, 不存工作区快照 (设计稿裁决 20): 每回合现查现校验。
///
/// 开不出注册表就是开不出来: [`CliContactPanel::new_persistent`] 返回 `Err`, 调用方必须
/// fail-closed, 不许降级成进程内注册表假装成功。
pub struct CliContactPanel {
    /// 联系人注册表 (B1 领域层)。
    registry: Arc<dyn ContactRegistry>,
    /// 绑定档的内存镜像 (写都落盘, 内存只是读缓存)。
    bindings: Mutex<BTreeMap<String, StoredContactBinding>>,
    /// 绑定档路径 (进程内装配时为空 ⇒ 不落盘)。
    binding_path: PathBuf,
}

impl CliContactPanel {
    /// 进程内装配 (只给测试/临时用; 生产组合根走 [`CliContactPanel::new_persistent`]).
    pub fn new() -> Self {
        Self {
            registry: Arc::new(InMemoryContactRegistry::new()),
            bindings: Mutex::new(BTreeMap::new()),
            binding_path: PathBuf::new(),
        }
    }

    /// 落盘装配: 联系人注册表 + 绑定档都在数据根下。
    ///
    /// 开不出 ⇒ `Err` (fail-closed): 调用方必须让启动失败, 不许静默降级。
    pub fn new_persistent(data_dir: impl Into<PathBuf>) -> Result<Self, String> {
        let data_dir = data_dir.into();
        // B3b-1: 注册表按根目录取**进程内单例** —— 回合闸与合同面要读同一份事实 (R8)。
        let registry = crate::binding_registries::contacts(&data_dir)?;
        let binding_path = data_dir.join("agents").join("contact-bindings.json");
        let bindings = load_contact_bindings(&binding_path)?;
        Ok(Self {
            registry,
            bindings: Mutex::new(bindings),
            binding_path,
        })
    }

    /// 读一个联系人的绑定声明。
    fn binding_of(&self, contact_id: &str) -> Option<ContactWorkspaceBindingDto> {
        let guard = self
            .bindings
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        guard
            .get(contact_id)
            .map(|binding| ContactWorkspaceBindingDto {
                workspace_id: binding.workspace_id.clone(),
                revision: binding.revision,
            })
    }

    /// 写绑定档 (锁内重读 → 改 → 落盘), 失败 ⇒ 服务端故障 (500)。
    fn write_binding(
        &self,
        contact_id: &str,
        change: &ContactBindingChange,
    ) -> Result<(), ContactPanelError> {
        if matches!(change, ContactBindingChange::Keep) {
            // 这次改动跟绑定无关: 别白写一档 (也不该动 mtime 让别的进程以为变了)。
            return Ok(());
        }
        if self.binding_path.as_os_str().is_empty() {
            // 进程内装配: 不落盘, 只更新内存镜像。
            let mut guard = self
                .bindings
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            apply_binding_change(&mut guard, contact_id, change);
            return Ok(());
        }
        let path = self.binding_path.clone();
        let contact_id = contact_id.to_string();
        let lock = storage_atomic::lock_path_for(&path);
        storage_atomic::with_file_lock(&lock, || {
            // 锁内重读: 别拿内存里的旧副本盖掉别的进程写进去的内容。
            let mut bindings = load_contact_bindings(&path).map_err(|_| contact_storage_fault())?;
            apply_binding_change(&mut bindings, &contact_id, change);
            save_contact_bindings(&path, &bindings).map_err(|_| contact_storage_fault())?;
            let mut guard = self
                .bindings
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            *guard = bindings;
            Ok(())
        })
        .map_err(|_| contact_storage_fault())?
    }

    /// 把领域记录投影成合同 DTO；显式解绑不会从旧绑定档回读。
    fn project_contact(&self, contact: &Contact) -> ContactDto {
        let out = contact.to_out();
        let contact_id = out.contact_id.to_string();
        ContactDto {
            contact_id: contact_id.clone(),
            agent_id: out.agent_id.to_string(),
            display_name: out.display_name,
            subtitle: out.subtitle,
            avatar_ref: out.avatar_ref,
            status: map_contact_status(out.status),
            persona_revision: out.persona_revision,
            memory_scope: map_memory_scope(out.memory_scope),
            workspace_binding: match contact.workspace_binding() {
                Some(binding) => binding.map(|binding| ContactWorkspaceBindingDto {
                    workspace_id: binding.workspace_id.to_string(),
                    revision: binding.revision,
                }),
                None => self.binding_of(&contact_id),
            },
            capabilities: ContactCapabilitiesDto {
                read: out.capabilities.read,
                write: out.capabilities.write,
                approval_required: out.capabilities.approval_required,
            },
            persona: out.persona,
            revision: out.revision,
            updated_at_ms: out.updated_at_ms,
        }
    }
}

/// 把联系人面装进服务图 (组合根用)。
///
/// 落盘注册表 + 绑定档都开不出 ⇒ `Err` (fail-closed): 调用方让启动失败, 不降级。
///
/// 联系人合同面 (B3a 批: `GET/POST /v1/agents/contacts` + 单体读写 + 归档写面):
/// 真实持有 B1 的联系人注册表, **落盘**在数据根下 ⇒ 重启后仍是同一批联系人;
/// 工作区绑定只存 id + 修订号；PATCH 与联系人修订号一并保存。
/// 开不出来 ⇒ **fail-closed** (启动失败), 不降级成进程内注册表假装成功。
pub fn wire_contacts(
    services: &mut GatewayServices,
    data_dir: impl Into<PathBuf>,
) -> Result<(), String> {
    let panel = Arc::new(CliContactPanel::new_persistent(data_dir)?);
    services.contacts = Some(panel as Arc<dyn ContactPanel>);
    Ok(())
}

/// 改一条绑定 (内存镜像)。
fn apply_binding_change(
    bindings: &mut BTreeMap<String, StoredContactBinding>,
    contact_id: &str,
    change: &ContactBindingChange,
) {
    match change {
        ContactBindingChange::Keep => {}
        ContactBindingChange::Clear => {
            bindings.remove(contact_id);
        }
        ContactBindingChange::Bind(binding) => {
            bindings.insert(
                contact_id.to_string(),
                StoredContactBinding {
                    workspace_id: binding.workspace_id.clone(),
                    revision: binding.revision,
                },
            );
        }
    }
}

/// 存储故障的统一映射: **只回固定文案**, 不带宿主路径 (领域 reason 里有绝对路径)。
fn contact_storage_fault() -> ContactPanelError {
    ContactPanelError::Internal {
        reason: "联系人存储故障".to_string(),
    }
}

/// 读绑定档 (文件不存在 = 还没有绑定; 其它错一律 fail-closed)。
fn load_contact_bindings(path: &Path) -> Result<BTreeMap<String, StoredContactBinding>, String> {
    match stored_doc::open_single::<ContactBindingOverlay>(path, &contact_binding_compat()) {
        Ok(doc) => Ok(doc.body.bindings),
        Err(error) => {
            if fs_err::metadata(path).is_err() {
                // 档还没建过 = 没有绑定 (不是故障)。
                return Ok(BTreeMap::new());
            }
            Err(format!("打开联系人绑定档失败: {error}"))
        }
    }
}

/// 落绑定档 (原子写 + 文档信封)。
fn save_contact_bindings(
    path: &Path,
    bindings: &BTreeMap<String, StoredContactBinding>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs_err::create_dir_all(parent)
            .map_err(|error| format!("准备联系人绑定档目录失败: {error}"))?;
    }
    let body = ContactBindingOverlay {
        bindings: bindings.clone(),
    };
    stored_doc::save_single(
        path,
        &contact_binding_compat(),
        body,
        stored_doc::DEFAULT_DOC_MODE,
    )
    .map(|_| ())
    .map_err(|error| format!("写联系人绑定档失败: {error}"))
}

/// 绑定档的文档信封身份。
fn contact_binding_compat() -> stored_doc::DocCompat {
    stored_doc::DocCompat::exact(CONTACT_BINDING_DOC_NAME, CONTACT_BINDING_DOC_VERSION)
}

/// 领域状态 → 合同状态 (两套枚举显式映射, 不合并 —— 设计稿裁决 4)。
fn map_contact_status(status: ContactStatus) -> ContactStatusDto {
    match status {
        ContactStatus::Ready => ContactStatusDto::Ready,
        ContactStatus::Draft => ContactStatusDto::Draft,
        ContactStatus::Archived => ContactStatusDto::Archived,
        ContactStatus::Unavailable => ContactStatusDto::Unavailable,
    }
}

/// 领域记忆作用域 → 合同记忆作用域 (只在 DTO 层映射 —— 设计稿裁决 5)。
fn map_memory_scope(scope: ContractMemoryScope) -> ContactMemoryScopeDto {
    match scope {
        ContractMemoryScope::Subject => ContactMemoryScopeDto::Subject,
        ContractMemoryScope::Contact => ContactMemoryScopeDto::Contact,
        ContractMemoryScope::Session => ContactMemoryScopeDto::Session,
        ContractMemoryScope::None => ContactMemoryScopeDto::None,
    }
}

/// 合同记忆作用域 → 领域记忆作用域。
fn map_memory_scope_to_domain(scope: ContactMemoryScopeDto) -> ContractMemoryScope {
    match scope {
        ContactMemoryScopeDto::Subject => ContractMemoryScope::Subject,
        ContactMemoryScopeDto::Contact => ContractMemoryScope::Contact,
        ContactMemoryScopeDto::Session => ContractMemoryScope::Session,
        ContactMemoryScopeDto::None => ContractMemoryScope::None,
    }
}

/// 能力展示投影 → 领域能力投影 (只是展示, 不是授权依据 —— 设计稿裁决 6)。
fn map_capabilities(capabilities: ContactCapabilitiesCommand) -> ContactCapabilities {
    ContactCapabilities {
        read: capabilities.read,
        write: capabilities.write,
        approval_required: capabilities.approval_required,
    }
}

/// 解析联系人身份 (格式非法 ⇒ 请求不合法)。
fn parse_contact_id(contact_id: &str) -> Result<ContactId, ContactPanelError> {
    contact_id
        .parse::<ContactId>()
        .map_err(|_| ContactPanelError::InvalidRequest {
            reason: "contact_id 格式非法".to_string(),
        })
}

/// 领域错误 → 合同错误。
///
/// ⛔ 存储故障 (领域 reason 以 "storage: " 开头, **带绝对路径**) ⇒ 映射成服务端故障
/// (500) 且**只回固定文案**; 其余错误按性质映射。
fn map_contact_error(error: ContactError) -> ContactPanelError {
    match error {
        ContactError::NotFound(id) => ContactPanelError::NotFound {
            contact_id: id.to_string(),
        },
        ContactError::RevisionConflict { expected, actual } => ContactPanelError::Conflict {
            contact_id: String::new(),
            expected_revision: expected,
            actual_revision: actual,
        },
        ContactError::Archived => ContactPanelError::Archived {
            contact_id: String::new(),
        },
        ContactError::InvalidDisplayName { reason } => ContactPanelError::InvalidRequest { reason },
        ContactError::InvalidPersona { reason } => ContactPanelError::InvalidRequest { reason },
        ContactError::InvalidRequest { reason } if reason.starts_with("storage: ") => {
            contact_storage_fault()
        }
        ContactError::InvalidRequest { reason } => ContactPanelError::InvalidRequest { reason },
    }
}

#[async_trait]
impl ContactPanel for CliContactPanel {
    async fn list_contacts(&self, include_archived: bool) -> Result<Vec<ContactDto>, String> {
        let rows = if include_archived {
            self.registry.list_all()
        } else {
            self.registry.list()
        };
        Ok(rows
            .iter()
            .map(|contact| self.project_contact(contact))
            .collect())
    }

    async fn get_contact(&self, contact_id: &str) -> Result<ContactDto, ContactPanelError> {
        let id = parse_contact_id(contact_id)?;
        let contact = self
            .registry
            .get(&id)
            .ok_or_else(|| ContactPanelError::NotFound {
                contact_id: contact_id.to_string(),
            })?;
        Ok(self.project_contact(&contact))
    }

    async fn create_contact(
        &self,
        command: ContactCreateCommand,
    ) -> Result<ContactDto, ContactPanelError> {
        let input = ContactIn {
            display_name: command.display_name,
            subtitle: command.subtitle,
            avatar_ref: command.avatar_ref,
            persona: command.persona.map(|persona| persona.as_str().to_string()),
            memory_scope: command.memory_scope.map(map_memory_scope_to_domain),
        };
        let request = input
            .into_request(now_millis().max(0) as u64)
            .map_err(map_contact_error)?;
        let contact = self.registry.create(request).map_err(map_contact_error)?;
        if let Some(binding) = command.workspace_binding.as_ref() {
            self.write_binding(
                &contact.contact_id().to_string(),
                &ContactBindingChange::Bind(binding.clone()),
            )?;
        }
        Ok(self.project_contact(&contact))
    }

    async fn update_contact(
        &self,
        command: ContactUpdateCommand,
    ) -> Result<ContactDto, ContactPanelError> {
        let ContactUpdateCommand {
            contact_id,
            expected_revision,
            persona,
            capabilities,
            workspace_binding,
        } = command;
        let id = parse_contact_id(&contact_id)?;
        let workspace_binding = match workspace_binding {
            ContactBindingChange::Keep => None,
            ContactBindingChange::Clear => Some(None),
            ContactBindingChange::Bind(binding) => Some(Some(ContactWorkspaceBinding {
                workspace_id: binding.workspace_id.parse().map_err(|_| {
                    ContactPanelError::InvalidRequest {
                        reason: "workspace_id 格式非法".into(),
                    }
                })?,
                revision: binding.revision,
            })),
        };
        let patch = ContactPatch {
            persona,
            capabilities: capabilities.map(map_capabilities),
            workspace_binding,
        };
        let contact = self
            .registry
            .update(&id, expected_revision, patch, now_millis().max(0) as u64)
            .map_err(map_contact_error)?;
        Ok(self.project_contact(&contact))
    }

    async fn archive_contact(
        &self,
        command: ContactArchiveCommand,
    ) -> Result<ContactDto, ContactPanelError> {
        let ContactArchiveCommand {
            contact_id,
            expected_revision,
        } = command;
        let id = parse_contact_id(&contact_id)?;
        let contact = self
            .registry
            .archive(&id, expected_revision, now_millis().max(0) as u64)
            .map_err(map_contact_error)?;
        Ok(self.project_contact(&contact))
    }
}
