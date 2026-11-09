//! `tool.object_grant`（C-D1）：记忆**对象级授权**的工具面 + 治理档钩子 + 宿主服务面。
//!
//! # 这个面是什么
//!
//! 把**一条**记忆（episode / fact）以**只读**授予某主体；被授方可读不可写；
//! 撤销**即时生效**；授予/撤销**逐次人批 + 留审计**。
//!
//! # 语义（lead 拍板，**不得改**）：**叠加 ≠ 替代**
//!
//! ```text
//! 读可见 = 域可见 ∨ (存在有效对象 grant)
//! ```
//!
//! * **域可见** = 既有 [`MemoryScope`] 域级可见性（**一个字都不改**）；
//! * **对象 grant** 是**叠加层**：只在「域可见为 false」时补一条授权；
//! * **域级隔离不被突破**：对象 grant **不扩大**任何域的可见范围
//!   （只对**那一个对象 id** 生效），也**不绕过**域级闸；
//! * **跨分区披露 = R4 硬门**：对象所在分区 ≠ 主体所在分区 ⇒ 授权请求先落
//!   `pending`，**逐次人批**通过后才 `active`；授予 / 撤销 / 人批 / 拒绝
//!   **全部留审计行**（审计账在 `apeireth_memory::object_grant`）。
//!
//! # 治理档（R0/R2）
//!
//! * **读面**（`list` / `check`）= R0：**零审批**（读取被授对象不需任何审批）；
//! * **写面**（`grant` / `revoke`）= R2：**逐次人批** —— 由
//!   [`ObjectGrantApprovalHook`] 唯一承载（与本地审批面板 / IM 审批卡同一条链），
//!   无自动放行档；未知动作 fail-closed 停审批。
//!
//! ⚠️ 本工具面**不自己实现**审批链：档位在治理链的钩子上（同
//! `TaskBoardApprovalHook` / `PlayApprovalHook` 的承载方式）。
//!
//! # 依赖边界
//!
//! 账本与判定原语在 `apeireth_memory::object_grant`（记忆 crate **不依赖**
//! `apeireth-governance`）；本文件只做**工具面投影**、**治理档映射**与
//! **宿主服务面**（人批落章口）。

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use apeireth_core::kernel::{system_clock, CapabilityId, Clock};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_memory::object_grant::{
    GrantAuditKind, GrantAuditRow, GrantEffect, GrantError, GrantLedgerEntry, GrantStatus,
    MemoryObjectResolver, ObjectGrantLedger, ObjectGrantRegistry,
};
use apeireth_memory::{MemoryCoordinator, MemoryScope};
use apeireth_plugin::{
    FrozenInvocation, ToolCaller, ToolCapability, ToolExecutionContext, ToolExecutionPolicy,
    ToolPermissionPolicy,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use apeireth_runtime::canonical::ObjectGrantActivation;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::canonical::memory_module::namespace_key;

/// 稳定能力标识（注册表 / 治理授权 / 审批档共用）。
pub const OBJECT_GRANT_CAPABILITY_ID: &str = "tool.object_grant";
/// 模型侧声明名。
pub const OBJECT_GRANT_TOOL_NAME: &str = "object_grant";

/// 动作：登记一条**只读**授权请求（**未生效**：等人批）。
pub const ACTION_GRANT: &str = "grant";
/// 动作：撤销授权（**即时生效**）。
pub const ACTION_REVOKE: &str = "revoke";
/// 动作：列授权与审计（只读）。
pub const ACTION_LIST: &str = "list";
/// 动作：查某主体对某对象的读可见性（只读）。
pub const ACTION_CHECK: &str = "check";

/// 闭合动作词表（登记 / 撤销 / 列 / 查；**没有**"直接生效的授予"动作）。
pub const OBJECT_GRANT_ACTIONS: &[&str] = &[ACTION_GRANT, ACTION_REVOKE, ACTION_LIST, ACTION_CHECK];

/// 审批卡载荷同文（按动作如实标注写面 / 只读面）。
pub fn object_grant_approval_reason(action: Option<&str>) -> String {
    let face = match action.map(str::as_ref) {
        Some(ACTION_GRANT) => "grant = 登记只读授权请求（跨分区披露 = R4 硬门 ⇒ 逐次人批后才生效）",
        Some(ACTION_REVOKE) => "revoke = 撤销只读授权（即时生效）",
        Some(ACTION_LIST) => "list = 只读列授权与审计（无外部内容注入面）, 零审批",
        Some(ACTION_CHECK) => "check = 只读查可见性分解（无外部内容注入面）, 零审批",
        _ => "动作未识别, fail-closed 停人工审批",
    };
    format!(
        "object_grant ({face}) —— 写面 RequireApproval 级: 每次写面都停人工审批, 无自动放行档; \
         只读面零审批; 对象授权**只读** ⇒ 被授方永远拿不到写权"
    )
}

/// 治理档位钩子：`tool.object_grant` 的**写面**（登记 / 撤销）每次调用停
/// [`Decision::RequireApproval`]（逐次人批，无自动放行档）；只读面
/// （`list` / `check`）零审批；未知动作 fail-closed 停审批。
///
/// 与任务簿 / 关怀规则同一承载方式：档位在治理层的钩子上，与本地审批面板 /
/// IM 审批卡同一条审批链。其它能力一律放行（本钩子只管自己的面）。
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectGrantApprovalHook;

impl ObjectGrantApprovalHook {
    /// 空构造（无状态钩子）。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GovernanceHook for ObjectGrantApprovalHook {
    fn name(&self) -> &str {
        "object_grant_approval_tier"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != OBJECT_GRANT_CAPABILITY_ID {
            return Decision::Allow;
        }
        let action = arguments.get("action").and_then(Value::as_str);
        match action {
            // 只读面: 与 `tool.self_status` 同类只读面, 零审批。
            Some(ACTION_LIST) | Some(ACTION_CHECK) => Decision::Allow,
            // 写面: 逐次人批（跨分区披露是 R4 硬门 ⇒ 绝不自动放行）。
            _ => Decision::require_approval(object_grant_approval_reason(action)),
        }
    }
}

/// 记忆对象解析面（生产接线）：以**记忆协调器**为真源回答
/// 「对象存在吗 / 在哪个分区 / 主体在哪个分区 / 域级是否已可见」。
///
/// 域级可见口径与既有记忆读路径**同源**：主体的可见域 = 宿主注入的额外可见域
/// ∪ 主体自己的私有会话域（`MemoryScope::Session { session_id: 主体键 }`）。
///
/// # 为什么只持**弱**引用（不是强 `Arc`）
///
/// 生产接线里有一条**天然回路**：协调器持有授权登记处（读路径并联）⇒ 登记处
/// 持有解析面 ⇒ 解析面若**强**持有协调器 ⇒ `Arc` 环 ⇒ 协调器**永不析构**
/// （泄漏）。
/// ⇒ 解析面只持 `Weak` ⇒ 环断开；协调器没了 ⇒ 解析面**查不出** ⇒ 一律
/// **fail-closed**（对象不存在 / 分区查不到 / 域不可见）⇒ 授权永不生效。
pub struct MemoryGrantResolver {
    /// 协调器句柄（**弱**引用槽；`None` = 还没绑定 ⇒ 查不出 ⇒ fail-closed）。
    coordinator: Arc<Mutex<Option<Weak<MemoryCoordinator>>>>,
    /// 宿主注入的**额外**域可见范围（主体 → 可见域）。
    extra_scopes: Mutex<BTreeMap<String, Vec<MemoryScope>>>,
}

impl std::fmt::Debug for MemoryGrantResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryGrantResolver").finish()
    }
}

impl MemoryGrantResolver {
    /// 造一个解析面（记忆协调器为真源）。
    ///
    /// ⚠️ 协调器只被**弱**持有 ⇒ 调用方若把协调器丢了，解析面即**查不出**
    /// （fail-closed：授权永不生效）。
    #[must_use]
    pub fn new(coordinator: Arc<MemoryCoordinator>) -> Self {
        Self {
            coordinator: Arc::new(Mutex::new(Some(Arc::downgrade(&coordinator)))),
            extra_scopes: Mutex::new(BTreeMap::new()),
        }
    }

    /// 造一个**晚绑定**的解析面（装配根用：协调器还没造出来 ⇒ 槽先空）。
    ///
    /// 装配根的构造顺序是「先造登记处（要解析面）→ 再造协调器（要登记处）」
    /// ⇒ 解析面必须能**后补**协调器 ⇒ [`Self::bind`]。
    #[must_use]
    pub fn deferred() -> Self {
        Self {
            coordinator: Arc::new(Mutex::new(None)),
            extra_scopes: Mutex::new(BTreeMap::new()),
        }
    }

    /// 绑定协调器（**弱**引用 ⇒ 不构成 `Arc` 环）。
    ///
    /// 幂等：后绑覆盖前绑（装配根只绑一次）。
    pub fn bind(&self, coordinator: &Arc<MemoryCoordinator>) {
        let mut slot = self
            .coordinator
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = Some(Arc::downgrade(coordinator));
    }

    /// 协调器句柄（`None` = 没绑 / 协调器已析构 ⇒ **查不出** ⇒ fail-closed）。
    fn coordinator(&self) -> Option<Arc<MemoryCoordinator>> {
        let slot = self
            .coordinator
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        slot.as_ref().and_then(Weak::upgrade)
    }

    /// 追加某主体的域可见范围（宿主装配面注入）。
    #[must_use]
    pub fn with_subject_scopes(
        mut self,
        subject: impl Into<String>,
        scopes: Vec<MemoryScope>,
    ) -> Self {
        self.extra_scopes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(subject.into(), scopes);
        self
    }

    /// 某主体的域可见范围（默认 = 主体自己的私有会话域）。
    #[must_use]
    pub fn scopes_for(&self, subject: &str) -> Vec<MemoryScope> {
        // ⚠️ **统一 trim 口径**: 授权判定侧 (`read_grant_allows`) 会 trim 主体键
        // ⇒ 这里不 trim ⇒ 同一主体的两种写法会分家（授权静默落不到人）。
        let subject = subject.trim();
        let mut scopes = self
            .extra_scopes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(subject)
            .cloned()
            .unwrap_or_default();
        let private = MemoryScope::Session {
            session_id: subject.to_string(),
        };
        if !scopes.contains(&private) {
            scopes.push(private);
        }
        scopes
    }
}

impl MemoryObjectResolver for MemoryGrantResolver {
    fn object_exists(&self, object_id: &str) -> bool {
        // ⚠️ 协调器缺席（没绑 / 已析构）⇒ **查不出** ⇒ fail-closed（不是「存在」）。
        let Some(coordinator) = self.coordinator() else {
            return false;
        };
        coordinator
            .backend()
            .get_episode(object_id)
            .ok()
            .flatten()
            .is_some()
    }

    fn object_partition(&self, object_id: &str) -> Option<String> {
        let coordinator = self.coordinator()?;
        let episode = coordinator
            .backend()
            .get_episode(object_id)
            .ok()
            .flatten()?;
        Some(coordinator.episode_scope_metadata(&episode).0.to_string())
    }

    fn subject_partition(&self, subject: &str) -> Option<String> {
        let subject = subject.trim();
        if subject.is_empty() {
            return None;
        }
        Some(
            MemoryScope::Session {
                session_id: subject.to_string(),
            }
            .to_string(),
        )
    }

    fn domain_visible(&self, subject: &str, object_id: &str) -> bool {
        let Some(coordinator) = self.coordinator() else {
            return false;
        };
        let Some(episode) = coordinator.backend().get_episode(object_id).ok().flatten() else {
            return false;
        };
        let (scope, _) = coordinator.episode_scope_metadata(&episode);
        scope.is_visible_in(&self.scopes_for(subject))
    }
}

/// 读可见性分解（**两个分量都报出来** ⇒ 叠加语义可核对）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadVisibility {
    /// 主体。
    pub subject: String,
    /// 对象 id。
    pub object_id: String,
    /// 域级可见（既有 `MemoryScope` 语义）。
    pub domain_visible: bool,
    /// 对象授权可见（存在有效 grant）。
    pub object_grant: bool,
    /// 合计读可见 = 域可见 ∨ 对象授权（**叠加**）。
    pub visible: bool,
    /// 写是否放行（**恒 false**：只读授予）。
    pub write_allowed: bool,
}

/// 授权账快照（账本行 + 审计行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectGrantSnapshot {
    /// 账本行（append-only，追加顺序）。
    pub entries: Vec<GrantLedgerEntry>,
    /// 审计行（追加顺序）。
    pub audits: Vec<GrantAuditRow>,
}

/// 对象授权服务：工具面与宿主面共用的**唯一入口**。
///
/// * **模型面**（`tool.object_grant`）：只能登记**请求**（`pending`）与撤销；
///   **不能**自己让授权生效；
/// * **宿主面**（人批落章口）：[`ObjectGrantService::approve_grant`] /
///   [`ObjectGrantService::deny_grant`] / [`ObjectGrantService::revoke_grant`]
///   —— 逐次人批的落点；
/// * **读面**：[`ObjectGrantService::check_read`] —— 零审批。
pub struct ObjectGrantService {
    registry: Arc<ObjectGrantRegistry>,
    clock: Arc<dyn Clock>,
}

impl std::fmt::Debug for ObjectGrantService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectGrantService").finish()
    }
}

impl ObjectGrantService {
    /// 造服务（注入登记处与时钟）。
    #[must_use]
    pub fn new(registry: Arc<ObjectGrantRegistry>, clock: Arc<dyn Clock>) -> Self {
        Self { registry, clock }
    }

    /// 造服务（用宿主系统时钟）。
    #[must_use]
    pub fn with_default_clock(registry: Arc<ObjectGrantRegistry>) -> Self {
        Self {
            registry,
            clock: system_clock(),
        }
    }

    /// 登记处（读路径接线用：判定原语在这里）。
    #[must_use]
    pub fn registry(&self) -> &Arc<ObjectGrantRegistry> {
        &self.registry
    }

    fn now_ms(&self) -> i64 {
        self.clock.now().timestamp_millis()
    }

    /// 登记一条**只读**授权请求（**不生效**：等人批 ⇒ 域级隔离此刻未被突破）。
    pub fn request_grant(
        &self,
        grantor: &str,
        object_id: &str,
        subject: &str,
        reason: Option<&str>,
    ) -> Result<GrantLedgerEntry, GrantError> {
        let audits_before = self.registry.audit_rows().len();
        let entry =
            self.registry
                .request_grant(grantor, object_id, subject, reason, self.now_ms())?;
        self.ensure_audit_written(audits_before, "授权请求登记")?;
        Ok(entry)
    }

    /// 人批通过 ⇒ 授权**生效**（只读）。
    pub fn approve_grant(
        &self,
        approver: &str,
        grant_id: &str,
    ) -> Result<GrantLedgerEntry, GrantError> {
        let audits_before = self.registry.audit_rows().len();
        let entry = self
            .registry
            .approve_grant(approver, grant_id, self.now_ms())?;
        self.ensure_audit_written(audits_before, "授权人批通过")?;
        Ok(entry)
    }

    /// 人批否决。
    ///
    /// ⚠️ **审计守卫与 approve / revoke 同款**（2026-11-09 补）：否决路径原先
    /// **没有** `ensure_audit_written` ⇒ 「否决」这条审计行写失败会被**静默吞**
    /// ⇒ ⇒ 「全部留审计行」的承诺在否决路径上是破的 ⇒ 现在审计未落 ⇒ **Err**
    /// （不静默放行：否决已发生但审计丢了 = 结果不可信）。
    pub fn deny_grant(
        &self,
        approver: &str,
        grant_id: &str,
        reason: Option<&str>,
    ) -> Result<GrantLedgerEntry, GrantError> {
        let audits_before = self.registry.audit_rows().len();
        let entry = self
            .registry
            .deny_grant(approver, grant_id, reason, self.now_ms())?;
        self.ensure_audit_written(audits_before, "授权人批否决")?;
        Ok(entry)
    }

    /// 撤销（**即时生效**：落账后下一次判定立刻读不到）。
    pub fn revoke_grant(
        &self,
        approver: &str,
        grant_id: &str,
        reason: Option<&str>,
    ) -> Result<GrantLedgerEntry, GrantError> {
        let audits_before = self.registry.audit_rows().len();
        let entry = self
            .registry
            .revoke_grant(approver, grant_id, reason, self.now_ms())?;
        self.ensure_audit_written(audits_before, "授权撤销")?;
        Ok(entry)
    }

    /// 撤销某主体对某对象的**全部**授权（返回被撤销的单数）。
    pub fn revoke_pair(
        &self,
        approver: &str,
        object_id: &str,
        subject: &str,
        reason: Option<&str>,
    ) -> Result<usize, GrantError> {
        let audits_before = self.registry.audit_rows().len();
        let count =
            self.registry
                .revoke_pair(approver, object_id, subject, reason, self.now_ms())?;
        self.ensure_audit_written(audits_before, "授权撤销")?;
        Ok(count)
    }

    /// 审计**守卫**: 写操作**必须**留下审计行 ⇒ 没留下 ⇒ 审计写失败被静默吞了
    /// ⇒ **向上报错**（不静默放行: 授权已生效但审计丢了 = 「全部留审计行」的
    /// 承诺被破 ⇒ 结果不可信）。
    ///
    /// ⚠️ 兜底深度 (0 装诚实): 审计写失败发生在**账本 crate 内部**（三处
    /// `append_audit` 的失败被吞）⇒ 本守卫只能**事后发现并报错**，不能替它
    /// 补写补偿行（登记处未开放审计追加口）⇒ 真正的修复在账本 crate 侧。
    fn ensure_audit_written(&self, before: usize, what: &str) -> Result<(), GrantError> {
        if self.registry.audit_rows().len() > before {
            return Ok(());
        }
        Err(GrantError::LedgerWrite(format!(
            "{what}: 审计行未落账 (审计写失败被吞) ⇒ 本次结果不可信, 拒绝静默放行"
        )))
    }

    /// 某条授权的**授予人**（按授权单 id 查最近一行; 查不到 = `None`）。
    #[must_use]
    pub fn grantor_of(&self, grant_id: &str) -> Option<String> {
        self.registry
            .entries()
            .into_iter()
            .rev()
            .find(|entry| entry.grant_id == grant_id)
            .map(|entry| entry.grantor.trim().to_string())
    }

    /// 某 (主体, 对象) 对上**最近一行**授权的授予人。
    #[must_use]
    pub fn grantor_of_pair(&self, subject: &str, object_id: &str) -> Option<String> {
        let subject = subject.trim();
        let object_id = object_id.trim();
        self.registry
            .entries()
            .into_iter()
            .rev()
            .find(|entry| entry.subject.trim() == subject && entry.object_id.trim() == object_id)
            .map(|entry| entry.grantor.trim().to_string())
    }

    /// 读可见性分解（**零审批**）：域可见 / 对象授权 / 合计 / 写是否放行。
    #[must_use]
    pub fn check_read(&self, subject: &str, object_id: &str) -> ReadVisibility {
        let domain_visible = self.registry.domain_visible(subject, object_id);
        let object_grant = self.registry.read_grant_allows(subject, object_id);
        ReadVisibility {
            subject: subject.to_string(),
            object_id: object_id.to_string(),
            domain_visible,
            object_grant,
            // 叠加：域可见 ∨ 有效对象授权。
            visible: domain_visible || object_grant,
            // 只读授予 ⇒ 写恒不放行。
            write_allowed: self.registry.allows_write(subject, object_id),
        }
    }

    /// 账本 + 审计快照（只读；**宿主视角**：全量）。
    #[must_use]
    pub fn snapshot(&self) -> ObjectGrantSnapshot {
        ObjectGrantSnapshot {
            entries: self.registry.entries(),
            audits: self.registry.audit_rows(),
        }
    }

    /// 账本 + 审计快照（只读；**按主体过滤**：只给与该主体有关的行）。
    ///
    /// 模型面 `list` 走这里 ⇒ **不跨域泄露**别的分区的授权元数据
    /// （R4 硬门：跨分区披露只经人批，不经只读面）。
    #[must_use]
    pub fn snapshot_for(&self, subject: &str) -> ObjectGrantSnapshot {
        let full = self.snapshot();
        ObjectGrantSnapshot {
            entries: full
                .entries
                .into_iter()
                .filter(|entry| entry.subject == subject || entry.grantor == subject)
                .collect(),
            audits: full
                .audits
                .into_iter()
                .filter(|row| row.subject == subject || row.actor == subject)
                .collect(),
        }
    }
}

/// **宿主激活口**（C-D1）：把「人批落章」这一步实现成引擎层的端口。
///
/// ⚠️ **身份口径**：`approver` 由**宿主**给（网关的审批路由），**绝不**来自模型
/// 入参 ⇒ 模型面**没有** approve/deny 动作 ⇒ 模型无法自批。
impl ObjectGrantActivation for ObjectGrantService {
    /// 人批通过 ⇒ 授权生效（只读）。
    ///
    /// ⚠️ 参数顺序**刻意**与端口一致（`grant_id` 在前、`approver` 在后）⇒ 与
    /// 服务内的 `approve_grant(approver, grant_id)` 相反 ⇒ 转换处显式换位。
    fn approve(&self, grant_id: &str, approver: &str) -> Result<(), String> {
        self.approve_grant(approver, grant_id)
            .map(|_entry| ())
            .map_err(|error| error.to_string())
    }

    /// 人批否决 ⇒ 授权永不生效（审计未落 ⇒ Err，不静默放行）。
    fn deny(&self, grant_id: &str, approver: &str, reason: Option<&str>) -> Result<(), String> {
        self.deny_grant(approver, grant_id, reason)
            .map(|_entry| ())
            .map_err(|error| error.to_string())
    }
}

/// `tool.object_grant`：记忆对象级授权的模型面工具。
pub struct ObjectGrantTool {
    id: CapabilityId,
    service: Arc<ObjectGrantService>,
}

impl ObjectGrantTool {
    /// 绑定服务（登记处 + 宿主服务面）。
    pub fn new(service: Arc<ObjectGrantService>) -> Self {
        Self {
            id: CapabilityId::new(OBJECT_GRANT_CAPABILITY_ID).expect("constant capability"),
            service,
        }
    }

    /// 服务句柄（宿主面用）。
    #[must_use]
    pub fn service(&self) -> &Arc<ObjectGrantService> {
        &self.service
    }
}

fn permanent(call: &ToolCall, message: impl Into<String>) -> ToolResult {
    ToolResult::permanent_error(&call.id, message).with_name(OBJECT_GRANT_TOOL_NAME)
}

fn entry_json(entry: &GrantLedgerEntry) -> Value {
    json!({
        "grant_id": entry.grant_id,
        "object_id": entry.object_id,
        "subject": entry.subject,
        "grantor": entry.grantor,
        "granted_at_ms": entry.granted_at_ms,
        "version": entry.version,
        "effect": match entry.effect {
            GrantEffect::ReadOnly => "read_only",
        },
        "status": match entry.status {
            GrantStatus::Pending => "pending",
            GrantStatus::Active => "active",
            GrantStatus::Denied => "denied",
            GrantStatus::Revoked => "revoked",
        },
        // ⚠️ **不回显 `partition`**: 域不可见对象的分区名 = 存在性 oracle
        // （「这个 id 落在哪个域」⇒ 可枚举）。账本内部照留该列（判定要用），
        // 工具面只报 `cross_partition` 这个**布尔**（跨分区与否）。
        "cross_partition": entry.cross_partition,
        "decided_by": entry.decided_by,
        "decided_at_ms": entry.decided_at_ms,
        "reason": entry.reason,
    })
}

fn audit_json(row: &GrantAuditRow) -> Value {
    json!({
        "seq": row.seq,
        "at_ms": row.at_ms,
        "kind": match row.kind {
            GrantAuditKind::GrantRequested => "grant_requested",
            GrantAuditKind::GrantApproved => "grant_approved",
            GrantAuditKind::GrantDenied => "grant_denied",
            GrantAuditKind::GrantRevoked => "grant_revoked",
            GrantAuditKind::GrantRefused => "grant_refused",
        },
        "grant_id": row.grant_id,
        "object_id": row.object_id,
        "subject": row.subject,
        "actor": row.actor,
        "detail": row.detail,
    })
}

fn visibility_json(view: &ReadVisibility) -> Value {
    json!({
        "subject": view.subject,
        "object_id": view.object_id,
        "domain_visible": view.domain_visible,
        "object_grant": view.object_grant,
        "visible": view.visible,
        "write_allowed": view.write_allowed,
    })
}

#[async_trait]
impl ToolCapability for ObjectGrantTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut params = ToolParameters::new();
        params.extend(
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": OBJECT_GRANT_ACTIONS,
                        "description": "grant (register a read-only grant request) / revoke / list / check (closed vocabulary)."
                    },
                    "object_id": {
                        "type": "string",
                        "maxLength": 256,
                        "description": "Memory object id (episode id). Required by grant and check."
                    },
                    "subject": {
                        "type": "string",
                        "maxLength": 256,
                        "description": "Grantee subject key (namespace key). Required by grant and check."
                    },
                    "grant_id": {
                        "type": "string",
                        "maxLength": 128,
                        "description": "Grant ticket id returned by grant. Optional for revoke (revokes that ticket)."
                    },
                    "reason": {
                        "type": "string",
                        "maxLength": 256,
                        "description": "Free-text reason recorded in the audit trail."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(OBJECT_GRANT_TOOL_NAME)
            .with_description(
                "Object-level memory grant (read-only, per memory object id). grant registers a \
                 request that stays pending until a human approves it; revoke takes effect \
                 immediately; list and check are read-only and need no approval. Grants are an \
                 overlay on domain visibility (a memory stays readable when its domain is visible \
                 OR when a valid object grant exists) and never open the write path.",
            )
            .with_parameters(params)
    }

    fn execution_policy(&self) -> ToolExecutionPolicy {
        // 逐次人批由治理链的 [`ObjectGrantApprovalHook`] 按动作承载
        // （同 `TaskBoardTool` 的承载方式）：写面停审批，只读面零审批。
        ToolExecutionPolicy {
            timeout: Some(Duration::from_secs(5)),
            permission: ToolPermissionPolicy::Governed,
            ..Default::default()
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        permanent(
            call,
            "authenticated runtime context required (object grant needs the caller identity)",
        )
    }

    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        _frozen: Option<&FrozenInvocation>,
        ctx: &ToolExecutionContext,
    ) -> ToolResult {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        if !arguments.is_object() {
            return permanent(call, "object_grant arguments must be a JSON object");
        }
        let Some(action) = arguments.get("action").and_then(Value::as_str) else {
            return permanent(call, "object_grant requires an \"action\"");
        };
        if !OBJECT_GRANT_ACTIONS.contains(&action) {
            return permanent(
                call,
                format!(
                    "unknown object_grant action {action:?}; the closed vocabulary is {}",
                    OBJECT_GRANT_ACTIONS.join(" / ")
                ),
            );
        }
        // 写面要授予人身份 ⇒ 必须有已认证主体（宿主命名空间键）。
        let actor = ctx
            .principal
            .as_ref()
            .map(namespace_key)
            .unwrap_or_default();
        let reason = arguments.get("reason").and_then(Value::as_str);
        match action {
            ACTION_GRANT => {
                if actor.is_empty() {
                    return permanent(call, "Host principal is required to register a grant");
                }
                let object_id = arguments
                    .get("object_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();
                // 主体键归一 (trim + 形状): 同一主体的两种写法不再分家。
                let subject = match normalize_subject_key(
                    arguments
                        .get("subject")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                ) {
                    Ok(subject) => subject,
                    Err(reason) => return permanent(call, reason),
                };
                match self.service.request_grant(&actor, object_id, &subject, reason) {
                    Ok(entry) => ToolResult::ok(
                        &call.id,
                        json!({
                            "action": ACTION_GRANT,
                            "ok": true,
                            "status": "pending",
                            "effect": "read_only",
                            "cross_partition": entry.cross_partition,
                            "requires_human_approval": true,
                            "grant": entry_json(&entry),
                            "note": "授权请求已登记但**未生效**；逐次人批通过后才可读（跨分区披露 = R4 硬门）",
                        }),
                    )
                    .with_name(OBJECT_GRANT_TOOL_NAME),
                    // ⚠️ **存在性错误统一文案**: 「对象不存在」与「对象不可解析」
                    // /「域不可见」**同一句话** ⇒ 不给「这个 id 存不存在」当
                    // oracle (回显 partition 更不行)。
                    Err(GrantError::UnknownObject(_)) => permanent(
                        call,
                        "object grant refused: 目标对象不可用（不存在 / 不可解析 / 域不可见，同一文案）",
                    ),
                    Err(error) => permanent(call, format!("object grant refused: {error}")),
                }
            }
            ACTION_REVOKE => {
                if actor.is_empty() {
                    return permanent(call, "Host principal is required to revoke a grant");
                }
                let grant_id = arguments.get("grant_id").and_then(Value::as_str);
                let object_id = arguments.get("object_id").and_then(Value::as_str);
                let subject = arguments.get("subject").and_then(Value::as_str);
                // ⚠️ **模型面必须校验归属**: 撤销**别人**的授权 = DoS（对方突然
                // 读不到自己的记忆）⇒ 只许「该授权的授予人」或「被授主体本人」
                // 撤。宿主面（审批面板的落章口）不受此限（宿主才是 owner）。
                if matches!(ctx.caller, ToolCaller::Model) {
                    let grantor = match grant_id {
                        Some(grant_id) => self.service.grantor_of(grant_id),
                        None => match (object_id, subject) {
                            (Some(object_id), Some(subject)) => {
                                self.service.grantor_of_pair(subject, object_id)
                            }
                            _ => None,
                        },
                    };
                    let subject_key = subject.unwrap_or("").trim();
                    if !caller_may_act_for(
                        &ctx.caller,
                        &actor,
                        subject_key,
                        grantor.as_deref().unwrap_or(""),
                    ) {
                        return permanent(
                            call,
                            "object_grant revoke refused: 调用者既不是该授权的授予人, 也不是被授主体本人",
                        );
                    }
                }
                let outcome = match (grant_id, object_id, subject) {
                    (Some(grant_id), _, _) => self
                        .service
                        .revoke_grant(&actor, grant_id, reason)
                        .map(|entry| json!({"revoked": 1usize, "grant": entry_json(&entry)})),
                    (None, Some(object_id), Some(subject)) => self
                        .service
                        .revoke_pair(&actor, object_id, subject, reason)
                        .map(|count| json!({"revoked": count, "grant": Value::Null})),
                    _ => Err(GrantError::UnknownGrant(
                        "revoke requires \"grant_id\" or (\"object_id\" + \"subject\")".to_string(),
                    )),
                };
                match outcome {
                    Ok(payload) => ToolResult::ok(
                        &call.id,
                        json!({
                            "action": ACTION_REVOKE,
                            "ok": true,
                            "immediate": true,
                            "note": "撤销已落账 ⇒ 下一次读判定立刻读不到（即时生效）",
                            "result": payload,
                        }),
                    )
                    .with_name(OBJECT_GRANT_TOOL_NAME),
                    Err(error) => permanent(call, format!("object grant revoke refused: {error}")),
                }
            }
            ACTION_LIST => {
                if actor.is_empty() {
                    return permanent(call, "Host principal is required to list grants");
                }
                // 只列**与调用主体有关**的行 ⇒ 不跨域泄露别的分区的授权元数据。
                let snapshot = self.service.snapshot_for(&actor);
                ToolResult::ok(
                    &call.id,
                    json!({
                        "action": ACTION_LIST,
                        "ok": true,
                        "entries": snapshot.entries.iter().map(entry_json).collect::<Vec<_>>(),
                        "audits": snapshot.audits.iter().map(audit_json).collect::<Vec<_>>(),
                    }),
                )
                .with_name(OBJECT_GRANT_TOOL_NAME)
            }
            ACTION_CHECK => {
                if actor.is_empty() {
                    return permanent(call, "Host principal is required to check visibility");
                }
                // 主体键归一 (trim + 形状)。
                let subject = match normalize_subject_key(
                    arguments
                        .get("subject")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                ) {
                    Ok(subject) => subject,
                    Err(reason) => return permanent(call, reason),
                };
                let object_id = arguments
                    .get("object_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();
                // ⚠️ **调用者必须校验**（可见性矩阵 oracle 的封口）: 本动作**零
                // 审批** ⇒ 若放任任意主体探测「任意主体 × 任意对象是否可见」⇒
                // 等于把可见性矩阵白送。⇒ 只许「主体本人」或「该 (主体, 对象)
                // 授权的授予人」问。宿主面（面板的落章口）不受此限。
                if matches!(ctx.caller, ToolCaller::Model) {
                    let grantor = self.service.grantor_of_pair(&subject, object_id);
                    if !caller_may_act_for(
                        &ctx.caller,
                        &actor,
                        &subject,
                        grantor.as_deref().unwrap_or(""),
                    ) {
                        return permanent(
                            call,
                            "object_grant check refused: 调用者既不是该主体本人, 也不是该授权的授予人",
                        );
                    }
                }
                let view = self.service.check_read(&subject, object_id);
                ToolResult::ok(
                    &call.id,
                    json!({
                        "action": ACTION_CHECK,
                        "ok": true,
                        "visibility": visibility_json(&view),
                        "note": "读可见 = 域可见 ∨ 有效对象授权（叠加 ≠ 替代）；写恒不放行",
                    }),
                )
                .with_name(OBJECT_GRANT_TOOL_NAME)
            }
            _ => permanent(call, "unknown object_grant action"),
        }
    }
}

/// 主体键形状被拒的**统一文案**（值自由报错: 不回显被拒的值）。
pub const SUBJECT_SHAPE_REFUSED: &str =
    "subject shape refused: expected profile:<uuid>/namespace:<uuid> (or a short domain key)";

/// 主体键**归一**: trim + 形状校验。
///
/// # 为什么必须归一（静默失效的真因）
///
/// 同一主体两种写法（带不带空白 / 段序颠倒 / 少了半段）⇒ 授权会静默落到
/// **另一个**主体名下 ⇒ 被授方永远读不到 ⇒ 「授了却读不到」。
///
/// # 形状口径
///
/// * 命名空间键形态 `profile:<uuid>/namespace:<uuid>` ⇒ 严格校验（两段都在、
///   段内非空、段序固定、uuid 形态合法）;
/// * **短键**形态（不含 `/`，宿主自定的会话域键）⇒ 接受（兼容既有宿主口径），
///   但**必 trim** ⇒ 同一短键的两种写法不再分家。
pub fn normalize_subject_key(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("subject must not be empty".to_string());
    }
    if !trimmed.contains('/') {
        return Ok(trimmed.to_string());
    }
    let Some((profile, namespace)) = trimmed.split_once('/') else {
        return Err(SUBJECT_SHAPE_REFUSED.to_string());
    };
    if namespace.contains('/') {
        return Err(SUBJECT_SHAPE_REFUSED.to_string());
    }
    let Some(profile_id) = profile.strip_prefix("profile:") else {
        return Err(SUBJECT_SHAPE_REFUSED.to_string());
    };
    let Some(namespace_id) = namespace.strip_prefix("namespace:") else {
        return Err(SUBJECT_SHAPE_REFUSED.to_string());
    };
    if !is_uuid_shape(profile_id) || !is_uuid_shape(namespace_id) {
        return Err(SUBJECT_SHAPE_REFUSED.to_string());
    }
    Ok(trimmed.to_string())
}

/// uuid 形态判定（8-4-4-4-12 十六进制; 不引 uuid 依赖 —— 形态够用）。
fn is_uuid_shape(raw: &str) -> bool {
    let parts: Vec<&str> = raw.split('-').collect();
    parts.len() == 5
        && [8usize, 4, 4, 4, 12]
            .iter()
            .zip(parts.iter())
            .all(|(len, part)| part.len() == *len && part.chars().all(|c| c.is_ascii_hexdigit()))
}

/// 调用者的**信任分档**: 谁能替主体/授予人行事。
///
/// * **模型面** ([`ToolCaller::Model`]): 不受信 ⇒ 只许「主体本人 / 该授权的
///   授予人」⇒ 否则拒绝（否则任何人可零审批探测「任意主体 × 任意对象是否
///   可见」= 可见性矩阵 **oracle**, 或撤掉别人的授权 = **DoS**）。
/// * **宿主面**（[`ToolCaller::Host`] / 模块面）: 宿主自己的机器（审批面板的
///   落章口）⇒ 放行（宿主才是真正的 owner）。
fn caller_may_act_for(caller: &ToolCaller, actor: &str, subject: &str, grantor: &str) -> bool {
    if !matches!(caller, ToolCaller::Model) {
        return true;
    }
    let actor = actor.trim();
    if actor.is_empty() {
        return false;
    }
    actor == subject.trim() || (!grantor.trim().is_empty() && actor == grantor.trim())
}

/// 造一个**装好账本**的登记处（账本文件路径 + 解析面）。
///
/// 账本文件 = 账本行 + 审计行的 JSONL（append-only）；坏行按不存在处理。
pub fn object_grant_registry(
    ledger_path: Option<&std::path::Path>,
    resolver: Arc<dyn MemoryObjectResolver>,
) -> Result<Arc<ObjectGrantRegistry>, GrantError> {
    let ledger = match ledger_path {
        Some(path) => ObjectGrantLedger::open(path)?,
        None => ObjectGrantLedger::in_memory(),
    };
    Ok(Arc::new(ObjectGrantRegistry::with_resolver(
        ledger, resolver,
    )))
}
