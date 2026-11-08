//! 外部工具（协议桥）启动期接线面 —— CLI 组合根的薄封装。
//!
//! 接线四步（发现 → 撤销 → 登记 → 授权）的机制在
//! `apeireth_runtime_assembly::canonical::external_tool_bridge`；本模块只做
//! **组合根侧的三件事**：授权落点 + 失败不阻断启动 + 承载周期刷新。
//!
//! # 失败语义
//!
//! * **发现 fail-all**：任一服务器不可达 ⇒ 整次发现失败（不半注册）。
//! * **失败不阻断启动**：外部工具面不可用 ≠ 主机不可用 ⇒ 启动继续，但**如实
//!   记录**到 stderr，不静默吞掉（0 装诚实：主人要看得见「外部工具没接上」）。
//! * **授权先于登记**：族授权 + 族逐次人批落在生产权限策略里，让派发裁决是
//!   `RequireApproval` ⇒ 运行时挂起该次派发等人决定。
//! * **周期刷新失败不误撤**：某服务器不可达 ⇒ 它的工具留在能力表里（不可达
//!   不是「工具没了」的证据）；失败逐条记到 stderr（可观测）。
//!
//! # 周期刷新为什么必须由组合根承载
//!
//! 启动期接线之后，**运行时能力表**只在启动期登记过一次：连接自愈重连会把
//! 重发现的工具列表交回**目录**，但**注册表面**只在 `refresh()` 时产出 ⇒
//! 重连后服务器新增的工具进不了模型工具目录，删除的工具残留在能力表里。
//! 周期刷新把这条缺口补上。
//!
//! 后台任务要活过组合根的栈帧 ⇒ 运行时必须 `Arc` 化 ⇒ **组合根交出
//! `Arc<Runtime>`**（`Runtime` 是 `Send + Sync`，`Arc` 化不改变组合根的公开
//! 面：公开面本来就交出 `Arc<Runtime>`）。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_governance::PermissionPolicy;
use apeireth_runtime::Runtime;
use apeireth_runtime_assembly::canonical::external_tool_bridge::{
    spawn_external_tool_refresh, ExternalToolRefreshObserver,
};
use apeireth_runtime_assembly::canonical::{
    authorize_external_tool_family, wire_external_tools, McpModule,
};
use apeireth_runtime_assembly::ProductionModules;
use apeireth_tools_canonical::mcp_bridge::{
    server_list_path, ClassBDeclaration, McpDefinition, McpToolBridge, CLASS_B_DECLARATION_FILE,
};

/// 装配期取出的外部工具桥句柄。
///
/// 桥与模块袋必须在**装配模块被 move 之前**取出（`register_into` 取走所有
/// 权）⇒ 组合根分两步：先取句柄，建成运行时后再接线。
///
/// 合并说明（2026-11-06）: 桥句柄取**资源工厂定义**（`McpDefinition`）而不是
/// 一只另造的桥 —— 装配根只构造**一个**外部工具桥（资源工厂在运行时初始化时
/// 交出活桥）⇒ 不重复连进程、不重复构造桥对象。模块袋仍是本线的**登记账袋**
/// （自述面晚绑定「实际登记件数」）。
pub struct ExternalToolHandles {
    definition: Option<Arc<McpDefinition>>,
    module_bag: Option<Arc<McpModule>>,
}

impl ExternalToolHandles {
    /// 启动期接线用的**活桥**：资源工厂在运行时初始化后交出（未初始化 = `None`
    /// ⇒ 外部工具面没开 = 无事可接线）。
    fn bridge(&self) -> Option<Arc<McpToolBridge>> {
        self.definition
            .as_ref()
            .and_then(|definition| definition.current_bridge())
    }
}

/// 从装配模块取出外部工具桥句柄（桥未装配时两项皆 `None`）。
pub fn take_handles(modules: &ProductionModules) -> ExternalToolHandles {
    ExternalToolHandles {
        definition: modules.mcp_definition().cloned(),
        module_bag: modules.mcp_module().cloned(),
    }
}

/// **类 B 声明**的落点（与 MCP 服务器清单同一个数据目录）。
///
/// 与 `MCP_SERVERS_FILE` 同目录 ⇒ 操作者只管一个目录 ⇒ 声明与清单不会分家.
fn class_b_declaration_path() -> std::path::PathBuf {
    server_list_path(&crate::default_panel_data_dir()).with_file_name(CLASS_B_DECLARATION_FILE)
}

/// 读**类 B 声明**（**默认空**）。
///
/// * **缺文件 = 空声明** ⇒ 没有工具被声明 ⇒ 结果面行为与接线前逐字节不变;
/// * **文件坏了 = 不猜**（fail-closed）: 声明保持空 + stderr 如实报错
///   （不静默、不假装已开）⇒ 通道保持关.
fn class_b_declaration() -> ClassBDeclaration {
    let path = class_b_declaration_path();
    match ClassBDeclaration::from_json_file(&path) {
        Ok(declaration) => declaration,
        Err(error) => {
            eprintln!(
                "[external-tools] 类 B 声明读取失败（{error}）—— 按 fail-closed 处理: 不声明任何类 B 工具"
            );
            ClassBDeclaration::empty()
        }
    }
}

/// 把**共享会话脱敏域**与**类 B 声明**注入活桥（生产接线面）。
///
/// # 这一步把「两个域」变成「一个域」
///
/// 外部工具结果面（`McpDynamicTool`）与出网闸（`RedactionEgressGate`）必须持
/// **同一个**会话脱敏域 ⇒ 两面才认账：工具面产的占位符，出网闸认得出。
///
/// # 默认关（判据）
///
/// 域注入的判据 = **既有登记名** `APEIRETH_ENABLE_EGRESS_GATE`（**不新增**
/// `APEIRETH_*` 名）⇒ 出网闸没开 ⇒ `shared_session_domain()` 返回 `None`
/// ⇒ 本函数**什么都不做** ⇒ 桥保持「每工具一域 + 无类 B 声明」
/// ⇒ 行为与接线前**逐字节一致**。
///
/// # 注入时机
///
/// 活桥是**资源工厂**在运行时初始化时交出来的 ⇒ 组合根拿到它时它已经存在 ⇒
/// 域只能**事后**注入（`set_session_domain`）。工具是在**发现**时包装的 ⇒
/// 注入必须发生在**第一次发现之前** —— 本函数由 [`wire_at_startup`] 在发现
/// 之前调用。
///
/// 返回值 = 是否真的注入了（`false` = 没有共享域 / 没有活桥 ⇒ 未接线）。
pub fn wire_session_redaction(handles: &ExternalToolHandles) -> bool {
    let Some(bridge) = handles.bridge() else {
        return false;
    };
    let Some(domain) = apeireth_runtime_assembly::egress_gate::shared_session_domain() else {
        // 域注入**未开启** ⇒ 不接线（逐字节不变）。
        return false;
    };
    bridge.set_session_domain(domain);
    let declaration = class_b_declaration();
    if !declaration.is_empty() {
        for line in declaration.audit_lines() {
            eprintln!("[external-tools] {line}");
        }
        bridge.set_class_b_declaration(declaration);
    }
    true
}

/// 启动期接线: 登记族授权 + 逐次人批标记, 再把发现的外部工具登记进能力表。
///
/// **不抛**：外部服务器不可达 ≠ 主机不可用 ⇒ 发现失败只如实记录到 stderr，
/// 启动继续（0 装诚实：主人要看得见「外部工具没接上」）。
pub async fn wire_at_startup(
    runtime: &Runtime,
    handles: &ExternalToolHandles,
    policy: &Mutex<PermissionPolicy>,
) {
    let Some(bridge) = handles.bridge() else {
        return;
    };
    // **共享会话脱敏域 + 类 B 声明**注入（**默认关** ⇒ 未开启时本步是 no-op）。
    //
    // 时机: 必须在**第一次发现之前** —— 工具是在发现时包装的, 域与声明在
    // 包装时读。`wire_external_tools` 下面那一次 `refresh()` 就是第一次发现。
    wire_session_redaction(handles);
    // 族授权 + 族逐次人批: 族内能力运行时才发现, 无法逐个静态登记 ⇒ 族根是
    // 它们唯一的共同前缀。人批权归运行时治理链 (owner 拍板: 一律逐次人批,
    // `full` 会话预设不豁免 —— 由预设层的族地板兜住)。
    authorize_external_tool_family(&mut policy.lock().expect("policy mutex"));
    let module_bag = handles.module_bag.as_ref().map(|bag| bag.as_ref());
    match wire_external_tools(runtime, bridge.as_ref(), module_bag, policy).await {
        Ok(wiring) => {
            if wiring.registered > 0 || wiring.unregistered > 0 {
                eprintln!(
                    "[external-tools] wired: {} registered, {} unregistered, {} live",
                    wiring.registered, wiring.unregistered, wiring.live
                );
            }
        }
        Err(reason) => {
            eprintln!("[external-tools] discovery failed: {reason}");
        }
    }
}

/// 承载周期刷新: 后台任务周期跑一轮刷新, 把增量同步进运行时能力表。
///
/// # 返回
///
/// * `None` = 没有外部工具桥（未装配外部工具面）⇒ 无事可刷新。
/// * `None` = 周期刷新显式关闭（桥的 `refresh_interval_ms = 0`）⇒ 不起任务。
/// * `Some(task)` = 后台任务句柄（组合根负责 join / abort）。
///
/// # 失败语义
///
/// 某服务器不可达 ⇒ 本轮**不撤**它的工具（不可达不是「工具没了」的证据）;
/// 失败逐条记到 stderr（可观测, 不静默）。
///
/// # 组合根接线点（**本批未接线**, 见交付说明）
///
/// 本函数需要 `Arc<Runtime>`（后台任务要活过组合根的栈帧）。组合根里
/// `build_canonical_runtime_with_parts`（**私有** fn）当前按值返回 `Runtime`
/// ⇒ 组合根把那一步 `Arc` 化并在**同一处**调用本函数即可。
///
/// **公开面的影响（如实）**: `build_canonical_runtime_from_env_with_observability`
/// 本来就返回 `Arc<Runtime>`（不变）; 但 `CanonicalRuntimeParts` 这个**类型别名**
/// 的第 0 元是 `Runtime` ⇒ 别名一改, `pub fn
/// build_canonical_runtime_with_sessions_and_voice_emotion_from_env` 的第 0 元
/// 也变成 `Arc<Runtime>`（**一处公开签名变化**）。仓内唯一调用方是 CLI 自己的
/// `tests/care_heartbeat_host.rs`（`Arc::new(runtime)` 一行去掉即可）;
/// gateway / sdk 复用路径**不碰**这个别名（gateway 走
/// `build_canonical_runtime_from_env_with_observability`, sdk 是 HTTP 客户端,
/// 不托管第二个 Runtime）⇒ 不波及那两条复用路径。
pub fn spawn_periodic_refresh(
    runtime: Arc<Runtime>,
    handles: &ExternalToolHandles,
) -> Option<tokio::task::JoinHandle<()>> {
    let bridge = handles.bridge()?;
    let interval_ms = bridge.options().refresh_interval_ms;
    if interval_ms == 0 {
        eprintln!("[external-tools] periodic refresh disabled (interval = 0)");
        return None;
    }
    let observer: ExternalToolRefreshObserver = Arc::new(|outcome| {
        if !outcome.failures.is_empty() {
            eprintln!(
                "[external-tools] periodic refresh degraded: {}",
                outcome.failures.join("; ")
            );
        }
    });
    Some(spawn_external_tool_refresh(
        runtime,
        bridge,
        handles.module_bag.clone(),
        Duration::from_millis(interval_ms),
        observer,
    ))
}
