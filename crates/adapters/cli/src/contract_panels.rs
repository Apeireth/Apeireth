//! 工作区合同面适配 (组合根): 受管根 + 作用域上限 + 落盘工作区注册表。
//!
//! 本模块自 `crate::gateway_panels` 拆出 (架构尺寸棘轮压回 · 行为等价搬迁):
//! 端点路径、错误文案、作用域截断口径、落盘位置与 fail-closed 语义一律不变; 公开
//! 路径经 `crate::gateway_panels` 再导出保持不变。
//!
//! 刻意不做什么: 不改宿主作用域上限 (那是 [`HOST_SCOPE_CEILING`], 宿主策略常量);
//! 不把工作区写面搬进面板 (写面仍走 `/v1/workspaces*` 合同面)。

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::ids::WorkspaceId;
use apeireth_gateway::{
    GatewayServices, WorkspaceCreateCommand, WorkspaceDto, WorkspacePanel, WorkspacePanelError,
};
use apeireth_runtime::canonical::workspace::{
    InMemoryWorkspaceRegistry, Workspace, WorkspaceCreateRequest as DomainWorkspaceCreateRequest,
    WorkspaceError, WorkspacePermissionScope, WorkspaceRegistry, WorkspaceRootKind,
};
use async_trait::async_trait;

use crate::gateway_panels::{errstr, now_millis};
/// 宿主策略: 本宿主可授予的作用域上限 (请求值高于上限时被截断到上限)。
///
/// 这是**宿主策略常量**, 不是可插拔的构建器: 上限由宿主定, 客户端提交的
/// requested scope 永远只是请求, 生效值 = min(requested, HOST_SCOPE_CEILING)。
/// 写成常量而不是留一个没人调的 builder: 留 builder 等于把「谁定上限」这件事
/// 悬空 (孤儿 API 棘轮会如实报出来), 而上限本就是宿主的一条策略。
pub const HOST_SCOPE_CEILING: WorkspacePermissionScope = WorkspacePermissionScope::Write;

/// 工作区合同面适配 (组合根, A2 批): 真实持有 `InMemoryWorkspaceRegistry` +
/// 宿主受管根。
///
/// 宿主口径 (A2 合同面):
/// - **受管根由宿主决定** —— 客户端只提交身份字段, 根路径永远来自本适配器;
/// - **作用域宿主确认** —— 客户端提交的是 requested scope, 生效的是宿主确认值
///   (被宿主策略上限 [`HOST_SCOPE_CEILING`] 截断);
/// - **读面不含绝对根路径** —— 出口只给 `root_display` (叶子名), 根路径不出适配器;
/// - **只装配受管根** —— temporary / preset 的根装配属后续批, 本适配如实拒开。
///
/// 存储口径 (A3 批): 注册表可以是**进程内**的 ([`CliWorkspacePanel::new`],
/// 供测试与一次性进程使用), 也可以是**落盘**的
/// ([`CliWorkspacePanel::new_persistent`], 生产组合根用) —— 后者让工作区的
/// 登记事实跨重启存活。
pub struct CliWorkspacePanel {
    registry: Arc<dyn WorkspaceRegistry>,
    managed_root: PathBuf,
    scope_ceiling: WorkspacePermissionScope,
    source: String,
}

impl CliWorkspacePanel {
    /// 以宿主受管根装配一个工作区面 (并绑定一个受管工作区)。
    ///
    /// 绑定的那个工作区就是宿主此刻的受管根 —— 读面看到的是身份 + 叶子名,
    /// 不是路径。
    ///
    /// 本构造用**进程内**注册表: 事实只活到进程结束。生产组合根用
    /// [`CliWorkspacePanel::new_persistent`]。
    pub fn new(managed_root: impl Into<PathBuf>) -> Self {
        Self {
            registry: Arc::new(InMemoryWorkspaceRegistry::new()),
            managed_root: managed_root.into(),
            scope_ceiling: HOST_SCOPE_CEILING,
            source: "cli".to_string(),
        }
    }

    /// 以宿主受管根 + **数据根**装配一个落盘的工作区面。
    ///
    /// 数据根下写出 `<data_dir>/workspaces/<workspace_id>/workspace.json`:
    /// 工作区的登记事实 (身份 / 展示名 / 根类型 / 受管根 / 作用域 / revision /
    /// 状态 / 过期时间 / 来源 / 更新时间) 落盘; 受管根**只落盘不外泄**。
    ///
    /// 开不出注册表就是**开不出来**: 本构造返回 `Err`, 调用方必须 fail-closed,
    /// 不许降级成进程内注册表假装成功。
    pub fn new_persistent(
        managed_root: impl Into<PathBuf>,
        data_dir: impl Into<PathBuf>,
    ) -> Result<Self, String> {
        // B3b-1: 注册表按根目录取**进程内单例** —— 回合闸与合同面要读同一份事实 (R8)。
        let data_dir = data_dir.into();
        let registry = crate::binding_registries::workspaces(&data_dir)?;
        Ok(Self {
            registry,
            managed_root: managed_root.into(),
            scope_ceiling: HOST_SCOPE_CEILING,
            source: "cli".to_string(),
        })
    }

    /// 换掉工作区来源标签 (写进 `source` 字段)。
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// 宿主当前生效的受管根 (仅供组合根/测试核对, 不进任何响应体)。
    pub fn managed_root(&self) -> &std::path::Path {
        &self.managed_root
    }

    /// 宿主确认作用域: 请求值被上限截断。
    fn confirm_scope(&self, requested: WorkspacePermissionScope) -> WorkspacePermissionScope {
        if scope_rank(requested) <= scope_rank(self.scope_ceiling) {
            requested
        } else {
            self.scope_ceiling
        }
    }
}

/// 作用域的强弱次序 (read < write < approval_write), 用来截断请求值。
fn scope_rank(scope: WorkspacePermissionScope) -> u8 {
    match scope {
        WorkspacePermissionScope::Read => 0,
        WorkspacePermissionScope::Write => 1,
        WorkspacePermissionScope::ApprovalWrite => 2,
    }
}

/// 领域错误 → 合同面错误的归因 (不泄露根路径, 不改写语义)。
fn map_workspace_error(error: WorkspaceError) -> WorkspacePanelError {
    match error {
        WorkspaceError::NotFound(id) => WorkspacePanelError::NotFound {
            workspace_id: id.to_string(),
        },
        WorkspaceError::RevisionConflict { expected, actual } => WorkspacePanelError::Conflict {
            workspace_id: String::new(),
            expected_revision: expected,
            actual_revision: actual,
        },
        WorkspaceError::Revoked => WorkspacePanelError::Revoked {
            workspace_id: String::new(),
        },
        WorkspaceError::InvalidDisplayName { reason } => {
            WorkspacePanelError::InvalidRequest { reason }
        }
        WorkspaceError::InvalidRequest { reason } => WorkspacePanelError::InvalidRequest { reason },
        // 存储故障是**服务端**故障, 不是客户端请求的问题 ⇒ 500 而不是 400。
        // 读面只给恒定文案: 领域层的 reason 里带绝对路径, 那是本机诊断用的,
        // 不进任何响应体。
        WorkspaceError::Storage { .. } => WorkspacePanelError::Internal {
            reason: "工作区存储故障".to_string(),
        },
    }
}

/// 领域工作区 → 合同 DTO 的投影 (只给身份 + 安全展示名)。
fn project_workspace(workspace: &Workspace) -> WorkspaceDto {
    WorkspaceDto {
        workspace_id: workspace.workspace_id.to_string(),
        display_name: workspace.display_name.clone(),
        root_kind: workspace.root_kind,
        root_display: workspace.root_display.clone(),
        permission_scope: workspace.permission_scope,
        revision: workspace.revision,
        status: workspace.status,
        can_select: workspace.can_select,
        can_create_temporary: workspace.can_create_temporary,
        can_write: workspace.can_write,
        expires_at: workspace.expires_at.map(|value| value as i64),
        source: workspace.source.clone(),
        updated_at_ms: workspace.updated_at_ms as i64,
    }
}

#[async_trait]
impl WorkspacePanel for CliWorkspacePanel {
    async fn list_workspaces(&self) -> Result<Vec<WorkspaceDto>, String> {
        Ok(self.registry.list().iter().map(project_workspace).collect())
    }

    async fn get_workspace(&self, workspace_id: &str) -> Result<WorkspaceDto, WorkspacePanelError> {
        let id = workspace_id.parse::<WorkspaceId>().map_err(|_| {
            WorkspacePanelError::InvalidRequest {
                reason: "workspace_id 格式非法".to_string(),
            }
        })?;
        let workspace = self
            .registry
            .get(&id)
            .ok_or_else(|| WorkspacePanelError::NotFound {
                workspace_id: workspace_id.to_string(),
            })?;
        Ok(project_workspace(&workspace))
    }

    async fn create_workspace(
        &self,
        command: WorkspaceCreateCommand,
    ) -> Result<WorkspaceDto, WorkspacePanelError> {
        // 宿主只装配受管根: 其它根类型如实拒开, 不假装支持。
        if command.root_kind != WorkspaceRootKind::Managed {
            return Err(WorkspacePanelError::InvalidRequest {
                reason: format!(
                    "宿主只装配受管根, 暂不支持 {} 根",
                    command.root_kind.source_label()
                ),
            });
        }
        if command.expires_at.is_some() {
            return Err(WorkspacePanelError::InvalidRequest {
                reason: "受管工作区不接受 expires_at (过期时间只属于临时工作区)".to_string(),
            });
        }
        let request = DomainWorkspaceCreateRequest {
            root_kind: WorkspaceRootKind::Managed,
            root: self.managed_root.clone(),
            display_name: command.display_name,
            permission_scope: self.confirm_scope(command.permission_scope),
            expires_at: None,
            source: Some(self.source.clone()),
            now_ms: now_millis().max(0) as u64,
        };
        match self.registry.create(request) {
            Ok(workspace) => Ok(project_workspace(&workspace)),
            Err(error) => Err(map_workspace_error(error)),
        }
    }

    async fn select_workspace(
        &self,
        workspace_id: &str,
        expected_revision: u64,
    ) -> Result<WorkspaceDto, WorkspacePanelError> {
        let id = workspace_id.parse::<WorkspaceId>().map_err(|_| {
            WorkspacePanelError::InvalidRequest {
                reason: "workspace_id 格式非法".to_string(),
            }
        })?;
        match self.registry.select(&id, expected_revision) {
            Ok(workspace) => Ok(project_workspace(&workspace)),
            Err(error) => Err(map_workspace_error(error)),
        }
    }

    async fn close_workspace(
        &self,
        workspace_id: &str,
        expected_revision: u64,
    ) -> Result<WorkspaceDto, WorkspacePanelError> {
        let id = workspace_id.parse::<WorkspaceId>().map_err(|_| {
            WorkspacePanelError::InvalidRequest {
                reason: "workspace_id 格式非法".to_string(),
            }
        })?;
        let now_ms = now_millis().max(0) as u64;
        match self.registry.close(&id, expected_revision, now_ms) {
            Ok(workspace) => Ok(project_workspace(&workspace)),
            Err(error) => Err(map_workspace_error(error)),
        }
    }
}

/// 绑定宿主受管根的受管工作区 (组合根装配时调用)。
///
/// 受管根是宿主事实: 这里给它一个身份 + 叶子名展示, 路径不出宿主。
///
/// **幂等 (A3)**: 受管根已经登记过 ⇒ 复用那一个身份。否则每次重启都会新铸一个
/// 身份, 列表越滚越长, 「重启后列表逐字段一致」也就无从谈起。
/// 命中一个**已吊销**的受管根时不复活它: 吊销是终态, 墓碑留着, 另立新身份
/// ([`WorkspaceRegistry::find_by_root`] 不返回已吊销的工作区)。
///
/// 匹配按**受管根** (宿主事实) 走, 不按展示名走: 展示名不是身份 (同名工作区
/// 仍按 UUID 区分)。命中若有多个, 取注册表迭代序的第一个 —— 迭代序由
/// `WorkspaceId` 决定, 因此重启前后同一个集合必然选中同一个。
pub fn bind_managed_workspace(
    panel: &CliWorkspacePanel,
    display_name: &str,
) -> Result<WorkspaceDto, String> {
    if let Some(existing) = panel.registry.find_by_root(panel.managed_root()) {
        return Ok(project_workspace(&existing));
    }
    let scope = panel.scope_ceiling;
    let request = DomainWorkspaceCreateRequest {
        root_kind: WorkspaceRootKind::Managed,
        root: panel.managed_root.clone(),
        display_name: display_name.to_string(),
        permission_scope: scope,
        expires_at: None,
        source: Some(panel.source.clone()),
        now_ms: now_millis().max(0) as u64,
    };
    match panel.registry.create(request) {
        Ok(workspace) => Ok(project_workspace(&workspace)),
        Err(error) => Err(errstr(error)),
    }
}

/// 把工作区合同面接进网关服务容器 (组合根接线)。
pub(crate) fn wire_workspace_panel(services: &mut GatewayServices) -> Result<(), String> {
    // 工作区合同面 (A2 批: `GET /v1/panel/workspaces` + `/v1/workspaces*` 写面):
    // 真实持有工作区注册表 + **宿主受管根** (进程唯一根 = 当前工作目录, 与
    // 组合根既有的唯一根口径同源)。受管根由宿主决定: 客户端只提交身份字段,
    // 提交根路径一律 400。作用域宿主确认 (上限 Write, 请求值被截断)。
    // A3 批: 注册表**落盘**在数据根 (`APEIRETH_DATA_DIR` / `~/.apeireth`) 下 ⇒
    // 重启后 `GET /v1/panel/workspaces` 仍列出同一批工作区, revision 单调。
    // 落盘开不出来 ⇒ **fail-closed** (启动失败), 不降级成进程内注册表假装成功。
    let workspace_panel = Arc::new(CliWorkspacePanel::new_persistent(
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        crate::default_panel_data_dir(),
    )?);
    // 宿主把受管根绑成一个工作区 (身份 + 叶子名展示; 路径不出宿主)。
    // 幂等: 受管根已登记过就复用那一个身份, 不新铸。
    bind_managed_workspace(&workspace_panel, "默认工作区")?;
    services.workspaces = Some(workspace_panel as Arc<dyn WorkspacePanel>);
    Ok(())
}
