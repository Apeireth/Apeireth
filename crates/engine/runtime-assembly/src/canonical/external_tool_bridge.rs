//! 外部工具（协议桥）生产接线面: 发现 → 登记 → 授权 → 派发。
//!
//! # 本面解决什么
//!
//! 外部工具桥（协议桥, 见 [`apeireth_tools_canonical::mcp_bridge`]）的四个环节
//! —— **发现**（`refresh()` 跑 `tools/list`）、**登记**（动态工具进运行时能力表
//! ⇒ 进模型工具目录）、**授权**（生产权限策略的族登记 + 逐次人批）、**派发**
//! （模型面工具名 `mcp:<server>:<tool>` 真能被回合调用）—— 在此之前生产零调用点。
//! 本文件提供**机制**（登记适配器 / 授权落点 / 启动期接线）; **组合根** (CLI)
//! 负责调用它。
//!
//! # 三件东西
//!
//! 1. [`RuntimeExternalToolRegistry`]: **运行时登记适配器** —— 把桥的
//!    [`McpToolRegistry`] 契约落在一个**已建成**的 [`Runtime`] 上
//!    （`register_dynamic_tool` 进能力表 ⇒ `tool_declarations()` 自动带上;
//!    `capability_registry().unregister` 出能力表 ⇒ 撤销即帧）。
//!    适配器**借 `&Runtime`**（不复制、不 `Arc` 化运行时）: 登记只发生在组合根
//!    启动期, 运行时值就在那儿的栈上, 借用即足; [`Runtime`] 是 `Send + Sync`
//!    ⇒ `&Runtime` 满足 `McpToolRegistry: Send + Sync`。
//!    （周期刷新批若要让适配器长期存活, 组合根改为交出 `Arc<Runtime>` 即可;
//!    本批只做启动期接线。）
//! 2. [`authorize_external_tool_family`]: **授权落点** —— 生产权限策略里把
//!    外部工具**族**（族根 `tool.mcp`, 段边界前缀）登记为「已授权 + 逐次人批」。
//!    族授权是治理层的**族级**能力（`grant_family` / `require_approval_for_family`）:
//!    族内能力**运行时才发现**, 无法逐个静态登记, 族根是它们唯一的共同前缀。
//! 3. [`wire_external_tools`]: **启动期接线** —— 发现（fail-all: 任一服务器失败
//!    ⇒ 整次失败, 不半注册）→ 撤销消失的能力 → 登记新工具 → 授权（生产策略族
//!    登记 + 桥级族放行）。
//!
//! # 授权的两道闸（别混成一道）
//!
//! * **人批闸 = 生产权限策略**: 族内能力在生产策略里 = 族授权 + 族逐次人批标记
//!   ⇒ 派发裁决是 `Decision::RequireApproval` ⇒ 运行时**挂起**该次派发等人决定
//!   （运行时审批恢复链那条路）。这是 owner 拍板的「一律逐次人批」。
//!   会话权限预设（`full` 的改写 / `standard` 的免审记忆）**不得豁免**它 ——
//!   豁免由预设层的族地板（`permission_preset::human_approval_floor_applies`）
//!   兜住, 本面用测试钉死这一条。
//! * **库级闸 = 桥内策略**: 桥自己的流水线在真正出网前再问一次策略
//!   （`McpRiskMappingHook`）。桥的闸是 **deny-only**: 拒绝 ⇒ 拒绝; 放行或
//!   「要人批」⇒ **透传**给运行时治理链（桥内不再重复要人批 —— 重复要批会把
//!   已挂起的调用变成硬失败, 断掉审批恢复路径）。生产装配目前给桥的是**桥内自建
//!   策略**（生产装配面归并行批, 本批不改）, 所以本面在桥级放行整族
//!   （`McpFamilyGrants`）—— 语义 = 「主人授权了整个外部工具族」。
//!
//! # 遗留（不在本面范围）
//!
//! * **周期刷新 / 重连回流**: 本面只做**启动期**一次发现。连接自愈重连会把
//!   重发现的工具列表交回**目录**（并行批已落), 但**运行时能力表**只在下次
//!   刷新时才会跟上 ⇒ 周期刷新另派批。
//! * **桥与生产策略同源**: 理想形态是桥直接持有**生产策略句柄**
//!   （`McpToolBridge::new_with_policy`), 这样「运行时裁决」与「桥内闸」永不
//!   分歧; 该构造点在生产装配面（并行批的地盘), 本批不改 ⇒ 本批用桥级族放行
//!   达到等效效果, 并登记为遗留。
//! * **声明面脱敏**: 外部工具的 `description` / `input_schema` 原文进模型目录,
//!   属高风险项, 另派批处理; 本面不碰。

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::PermissionPolicy;
use apeireth_plugin::ToolCapability;
use apeireth_runtime::canonical::Runtime;

use apeireth_tools_canonical::mcp_bridge::{
    is_capability_in_family, McpToolBridge, McpToolRegistry, MCP_FAMILY_ROOT,
};

use super::tool_modules::McpModule;

/// 外部工具的模块 id（动态能力在运行时能力表里的 owner 标签）。
///
/// 与生产装配的模块袋 id（`module.mcp`）同源: 模块袋是**归属视图**, 运行时
/// 能力表是**派发视图**, 两者必须同标签, 否则能力表里看不出归属。
pub const EXTERNAL_TOOL_MODULE_ID: &str = "module.mcp";

/// 某能力是否属于外部工具族（族根 + 段边界）。
///
/// 族根与判定口径都取自协议桥的**共享真源**
/// ([`MCP_FAMILY_ROOT`] + [`is_capability_in_family`]), 不在本层自带字面量。
pub fn is_external_tool_capability(capability: &CapabilityId) -> bool {
    is_capability_in_family(capability.as_str(), MCP_FAMILY_ROOT)
}

/// 运行时动态工具登记适配器: 把桥的登记契约落进运行时能力表。
///
/// 借 `&Runtime`（不 `Arc` 化运行时）: 登记发生在组合根启动期, 运行时值就在
/// 那儿; 借用让本适配器零分配、零生命周期外溢, 且不改动组合根交出运行时的
/// 形状（`Runtime` 仍按值返回）。
pub struct RuntimeExternalToolRegistry<'a> {
    runtime: &'a Runtime,
}

impl<'a> RuntimeExternalToolRegistry<'a> {
    /// 绑一个已建成的运行时。
    pub fn new(runtime: &'a Runtime) -> Self {
        Self { runtime }
    }

    /// 登记一个动态工具（撞能力 id 或模型面名 ⇒ 拒, fail-closed）。
    pub fn register(&self, tool: Arc<dyn ToolCapability>) -> Result<(), String> {
        self.runtime
            .register_dynamic_tool(EXTERNAL_TOOL_MODULE_ID, tool)
            .map_err(|error| error.to_string())
    }

    /// 从运行时能力表注销一个动态工具。返回是否真的撤掉了。
    pub fn unregister(&self, capability_id: &CapabilityId) -> bool {
        self.runtime.capability_registry().unregister(capability_id)
    }
}

impl McpToolRegistry for RuntimeExternalToolRegistry<'_> {
    fn register_tool(&self, tool: Arc<dyn ToolCapability>) -> Result<(), String> {
        self.register(tool)
    }

    fn unregister_tool(&self, capability_id: &CapabilityId) {
        self.unregister(capability_id);
    }
}

/// 把外部工具**族**登记进**生产权限策略**（逐次人批档）。
///
/// * 族授权（`grant_family`）: 族内能力（`tool.mcp.<server>.<tool>`）在授权面内
///   —— 运行时才发现的能力无法逐个静态登记, 族根是它们唯一的共同前缀。
/// * 族逐次人批（`require_approval_for_family`）: 族内**每次**调用都要人批 ——
///   生产策略对族内能力的裁决是 `Decision::RequireApproval`（**不是** `Allow`,
///   也**不是** `Deny`）⇒ 运行时挂起该次派发等人决定。
/// * 未授权的能力: 生产策略无 grant ⇒ `Decision::Deny`（fail-closed）。
///
/// 族根取自协议桥的共享真源 [`MCP_FAMILY_ROOT`]。
pub fn authorize_external_tool_family(policy: &mut PermissionPolicy) {
    policy.grant_family(MCP_FAMILY_ROOT);
    policy.require_approval_for_family(MCP_FAMILY_ROOT);
}

/// 启动期接线结果（真账: 登记 / 撤销各多少件）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExternalToolWiring {
    /// 新登记进运行时能力表的外部工具数。
    pub registered: usize,
    /// 从运行时能力表撤销的外部工具数。
    pub unregistered: usize,
    /// 本次发现的族内能力总数（登记 + 撤销 + 未变的都在内）。
    pub live: usize,
}

/// 启动期外部工具接线: 发现 → 撤销 → 登记 → 授权。
///
/// # 失败语义
///
/// * **发现 fail-all**: 任一服务器失败 ⇒ 整次返回 `Err`, **不半注册**
///   （半注册 = 能力表与实况漂移, 比不接线更难查）。
/// * **登记 fail-closed**: 撞能力 id 或模型面名 ⇒ 整次 `Err`（已登记的那几件
///   留在能力表里, 但授权尚未落 ⇒ 派发裁决仍是 `Deny`, 不会被派发出去）。
/// * 调用方（组合根）**不因本函数失败而让启动失败**: 外部工具面不可用 ≠ 主机
///   不可用; 失败必须**如实记录**（不静默吞掉）。
///
/// # 授权落点
///
/// * 生产权限策略: 族授权 + 族逐次人批 ⇒ 族内能力的裁决是 `RequireApproval`
///   （人批在运行时挂起该次派发）。
/// * 桥级族放行: 桥内闸是 deny-only, 族放行让族内能力**透传**到运行时治理链
///   （桥内不重复要人批 —— 重复要批会把已挂起的调用变成硬失败）。
pub async fn wire_external_tools(
    runtime: &Runtime,
    bridge: &McpToolBridge,
    module_bag: Option<&McpModule>,
    policy: &Mutex<PermissionPolicy>,
) -> Result<ExternalToolWiring, String> {
    // 1. 发现（fail-all）: 任一服务器失败 ⇒ 整次失败。
    let report = bridge.refresh().await.map_err(|error| error.to_string())?;

    // 2. 撤销: 消失的能力从运行时能力表注销（模型目录随之少了它）。
    let registry = RuntimeExternalToolRegistry::new(runtime);
    let mut unregistered = 0;
    for entry in &report.removed {
        if registry.unregister(&entry.capability_id) {
            unregistered += 1;
        }
        if let Some(bag) = module_bag {
            bag.unregister_tool(&entry.capability_id);
        }
    }

    // 3. 登记: 进运行时能力表 ⇒ `tool_declarations()` 自动带上（模型看得见）。
    let mut registered = 0;
    for tool in &report.registered {
        registry.register(Arc::clone(tool))?;
        registered += 1;
        if let Some(bag) = module_bag {
            bag.register_tool(Arc::clone(tool))
                .map_err(|error| format!("外部工具模块袋拒绝登记: {error}"))?;
        }
    }

    // 4. 授权: 生产策略族登记（逐次人批）+ 桥级族放行。
    {
        let mut policy = policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        authorize_external_tool_family(&mut policy);
    }
    bridge.family_grants().grant_mcp_family();

    Ok(ExternalToolWiring {
        registered,
        unregistered,
        live: report.live.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_governance::{
        Action, Decision, GovernanceHook, GovernanceRequest, PermissionGovernanceHook,
    };

    fn capability(id: &str) -> CapabilityId {
        CapabilityId::new(id).expect("valid capability id")
    }

    /// 一个最小的外部工具形状（只用于登记/撤销断言, 不真调外部服务器）。
    struct ProbeTool {
        id: CapabilityId,
        name: String,
    }

    #[async_trait::async_trait]
    impl ToolCapability for ProbeTool {
        fn id(&self) -> &CapabilityId {
            &self.id
        }

        fn declaration(&self) -> apeireth_protocol::canonical::NormalizedTool {
            apeireth_protocol::canonical::NormalizedTool {
                name: self.name.clone(),
                description: Some("probe".to_string()),
                parameters: apeireth_protocol::canonical::ToolParameters::new(),
                strict: false,
            }
        }

        async fn invoke(
            &self,
            _call: &apeireth_protocol::canonical::ToolCall,
        ) -> apeireth_protocol::canonical::ToolResult {
            apeireth_protocol::canonical::ToolResult::ok("call_1", serde_json::json!({}))
        }
    }

    #[test]
    fn family_membership_uses_the_shared_segment_boundary_vocabulary() {
        assert!(is_external_tool_capability(&capability(
            "tool.mcp.demo.echo"
        )));
        assert!(!is_external_tool_capability(&capability(
            "tool.mcpsomething"
        )));
        assert!(!is_external_tool_capability(&capability("tool.mcp2.demo")));
        assert!(!is_external_tool_capability(&capability("tool.shell")));
    }

    /// **本批核心安全断言**: 默认关 ⇒ 族内能力 `Deny`; 开关开（族登记）⇒
    /// `RequireApproval` —— 既不是 `Allow`, 也不是 `Deny`。
    #[test]
    fn family_decision_is_deny_by_default_and_require_approval_when_authorized() {
        let echo = capability("tool.mcp.demo.echo");

        // 默认关: 生产策略里族未登记 ⇒ 拒绝（fail-closed）。
        let mut policy = PermissionPolicy::new();
        let decision = policy.decision_for_capability(echo.as_str());
        assert!(
            matches!(decision, Decision::Deny { .. }),
            "默认关必须是拒绝: {decision:?}"
        );

        // 开关开: 族登记 + 族逐次人批 ⇒ 挂起等人批。
        authorize_external_tool_family(&mut policy);
        let decision = policy.decision_for_capability(echo.as_str());
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "开关开必须是逐次人批档: {decision:?}"
        );
        assert!(!decision.is_allowed(), "外部工具不得自动放行");
    }

    /// **owner 决策断言**: `full` 会话预设**不得豁免**外部工具的逐次人批。
    #[tokio::test]
    async fn the_full_session_preset_does_not_exempt_the_external_tool_family() {
        use apeireth_core::kernel::{system_clock, SessionId, TraceId};
        use apeireth_runtime::canonical::{InMemorySessionStore, PermissionPreset, Session};

        let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
        authorize_external_tool_family(&mut policy.lock().unwrap());
        let inner: Arc<dyn GovernanceHook> =
            Arc::new(PermissionGovernanceHook::new_shared(Arc::clone(&policy)));
        let store: Arc<dyn apeireth_runtime::canonical::SessionStore> =
            Arc::new(InMemorySessionStore::new());
        let hook = super::super::permission_preset::PermissionPresetGovernanceHook::new(
            inner,
            Arc::clone(&store),
        );

        let session = SessionId::new();
        let mut stored = Session::new(session, system_clock().as_ref());
        stored.settings.permission_preset = PermissionPreset::Full;
        store.save(&stored).await.expect("session saved");

        let echo = capability("tool.mcp.demo.echo");
        let arguments = serde_json::Value::Null;
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &echo,
                arguments: &arguments,
            },
            session,
            TraceId::new(),
            1,
        );
        let verdict = hook.evaluate_verbose(&request).await;
        assert!(
            matches!(verdict.decision, Decision::RequireApproval { .. }),
            "full 预设不得豁免外部工具的逐次人批: {:?}",
            verdict.decision
        );
    }

    /// **登记/撤销断言**: 动态工具进能力表 ⇒ 模型目录立刻带上它; 撤销 ⇒ 立刻没了。
    #[tokio::test]
    async fn registering_and_revoking_a_dynamic_tool_is_immediately_visible() {
        let runtime = Runtime::builder()
            .with_governance(Arc::new(apeireth_governance::AllowAll))
            .build()
            .await
            .expect("runtime builds");
        assert!(runtime.tool_declarations().is_empty());

        let tool = Arc::new(ProbeTool {
            id: capability("tool.mcp.demo.probe"),
            name: "mcp:demo:probe".to_string(),
        });
        let tool_id = tool.id().clone();
        let registry = RuntimeExternalToolRegistry::new(&runtime);
        registry
            .register(tool)
            .expect("dynamic registration is live");

        let names: Vec<String> = runtime
            .tool_declarations()
            .into_iter()
            .map(|declaration| declaration.name)
            .collect();
        assert_eq!(names, vec!["mcp:demo:probe".to_string()]);

        assert!(registry.unregister(&tool_id));
        assert!(
            runtime.tool_declarations().is_empty(),
            "撤销后模型目录必须立刻没有它"
        );
    }
}
