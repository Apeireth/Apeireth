//! Explicit desktop Host grants and the default managed main composition.
use apeireth_plugin::agents::{AgentBudget, AgentConfig};
// `Plugin` 提供 `manifest()` / `tools()`（mesh 模块的两张授权表都从它派生）。
use apeireth_plugin::Plugin;
use apeireth_runtime::{AgentHostPolicy, ModuleAgentGrant, Runtime, RuntimeBuilder};

impl super::ProductionModules {
    /// Select the existing desktop catalog for module tasks. Governance and the
    /// parent session scope still apply; registering a module does not call this.
    pub fn desktop_agent_policy(&self) -> Result<AgentHostPolicy, String> {
        let mut tools = self
            .capabilities()
            .iter()
            .map(|tool| tool.id().clone())
            .collect::<std::collections::BTreeSet<_>>();
        for module in self.modules() {
            tools.insert(
                apeireth_runtime::canonical::behavior_adapter::registered_behavior_capability(
                    &module.manifest().id,
                )?,
            );
        }
        let mut policy = AgentHostPolicy::default();
        for module in self.modules() {
            let definition =
                apeireth_runtime::canonical::behavior_adapter::registered_behavior_owner(
                    &module.manifest().id,
                )?;
            policy.modules.insert(
                definition,
                ModuleAgentGrant {
                    tools: tools.clone(),
                    max_instances: policy.max_instances,
                    max_depth: apeireth_runtime::canonical::module::DEFAULT_MAX_INVOCATION_DEPTH,
                    budget: policy.max_budget.clone(),
                    ..Default::default()
                },
            );
        }
        // `rt.module_messages`: 桌面 Host **显式武装**受限模块消息管道（2026-11-07 接线）。
        //
        // 缺口（补前）：本组合根从不设置 `message_policy` ⇒ 模块间消息端口恒为
        // `Disabled` ⇒ ⇒ 「owner 认证 + 有界」的那条管道在桌面产品里**根本不存在**
        // （只有测试在装它）。**没动** `production.rs` / CLI 组合根 ⇒ 装配面零尺寸增长。
        //
        // ⭐ 零行为变化（这是本条能默认开的前提）：`ModuleMessagePolicy::default()`
        // 只有**有界限额**（in_flight 16 / payload 64 KiB / hops 16 / 幂等 128 / 5s），
        // **零授权**（`grants` 空）⇒ ⇒ 没有任何模块被授权发/收 ⇒ **零流量、零副作用**。
        // 授权仍须 Host 显式加 `ModuleMessageGrant`（投递不授予权限）。
        //
        // ⭐ 认证语义仍是「owner 认证」：`AgentHostPolicy.modules` 的 grant 与本管道
        // grant 是**两张表**；管道只按 `message_policy.grants` 放行。
        policy.message_policy =
            Some(apeireth_runtime::canonical::module_messages::ModuleMessagePolicy::default());
        // Desktop already has explicit per-session quota settings. Preserve its
        // lifetime semantics rather than imposing a small portable task budget.
        policy.max_budget = desktop_main_budget();
        Ok(policy)
    }
}
fn desktop_main_budget() -> AgentBudget {
    AgentBudget {
        max_requests: u64::MAX,
        max_tokens: u64::MAX,
        max_tool_calls: u64::MAX,
        max_cost_micros: None,
        turn_timeout_ms: u64::MAX,
    }
}

// ============================================================================
// `rt.mesh_local_worker` 生产接线（2026-11-07）
// ============================================================================
//
// 缺口（补前）：`MeshConfig` / `MeshModule` 在**生产装配面零引用**（只有测试在装它）
// ⇒ ⇒ 生产里没有任何地方能提供「固定本机 peer + 私有载荷」的 Host 配置
// ⇒ ⇒ 那条「治理链门禁下的本机 UDP worker」在产品里**根本不存在**。
//
// 为什么落在这里：`desktop_host` 是本组合根唯一的装配面（CLI `lib.rs:1759` 调用），
// 且 `rt.module_messages` 已在此武装消息管道 ⇒ ⇒ mesh 的 Internal exchange 正好
// 走同一张 `message_policy`。**没动** `production.rs` / `cli/src/lib.rs` /
// `cognitive_registry.rs`（三处都是尺寸/热区约束面）。

/// Host 侧 mesh 配置文件（数据根下）。
pub const MESH_CONFIG_FILE: &str = "mesh.json";
/// 文档身份（防串档：别的 JSON 不会被当成 mesh 配置打开）。
const MESH_CONFIG_DOC_NAME: &str = "apeireth.mesh.host_config";
/// 本构建唯一的格式版本。
const MESH_CONFIG_DOC_VERSION: u32 = 1;

/// `mesh.json` 主体：Host 显式给出的**固定**本机 peer 配置。
///
/// ⚠️ 这份配置**永远不来自模型载荷、也不来自环境变量**：只有运维/用户手写文件
/// 才能开启网络面。`deny_unknown_fields` ⇒ ⇒ 拼错字段一律**拒开**，不猜默认值。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshHostConfigBody {
    /// 固定 peer 名（≤64，字母数字 / `_` / `-`）。
    pub peer: String,
    /// 固定**本机回环**端点 `ip:port`（端口必须显式非 0）。
    pub endpoint: String,
    /// 私有载荷（明文；≤512 字节）。它是 mesh 包体的内容，不是密钥。
    pub private_payload: String,
}

fn mesh_config_compat() -> apeireth_core::stored_doc::DocCompat {
    apeireth_core::stored_doc::DocCompat::exact(MESH_CONFIG_DOC_NAME, MESH_CONFIG_DOC_VERSION)
}

/// 解析数据根下的 `mesh.json` 并构造 mesh 模块（**拒开**语义，不猜默认值）。
///
/// # 三种结果（失败闭合，逐条可复核）
///
/// * **文件缺失** ⇒ `Ok(None)`：不注册 mesh ⇒ ⇒ **逐字节零行为变化**（这是默认态）；
/// * **文件存在但坏**（畸形 / 串档 / 未来版本 / 字段缺失 / 端点非法 / 载荷越界）
///   ⇒ `Err(真因)`：**不注册也不回落默认值**（坏配置不许被当成「没配置」）；
/// * **文件存在且合法但没绑定监督者** ⇒ `Err`：mesh 的 Internal exchange 是
///   `Protected` 面 ⇒ 运行时分发器在没有监督者时**一律拒执行**
///   （`SystemOneUnavailable`）⇒ ⇒ 静默不注册会让运维误以为网络面开着
///   ⇒ ⇒ 这里**明确报错**（`fail-closed`，不猜）。
pub fn local_mesh_module_at(
    data_root: &std::path::Path,
    system_one_bound: bool,
) -> Result<Option<std::sync::Arc<crate::mesh::MeshModule>>, String> {
    let path = data_root.join(MESH_CONFIG_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let doc =
        apeireth_core::stored_doc::open_single::<MeshHostConfigBody>(&path, &mesh_config_compat())
            .map_err(|error| format!("{} 拒开: {error}", path.display()))?;
    let body = doc.body;
    let endpoint = body
        .endpoint
        .trim()
        .parse::<std::net::SocketAddr>()
        .map_err(|error| format!("{} 的 endpoint 不是合法 ip:port（{error}）", path.display()))?;
    if !system_one_bound {
        return Err(format!(
            "{} 要求 System One 监督（mesh 的 Internal exchange 是 Protected 面，\
             未绑定监督者时运行时会拒执行）⇒ ⇒ 请设 APEIRETH_ENABLE_SYSTEM_ONE=1，\
             或删除 {} 以关闭 mesh",
            path.display(),
            path.display()
        ));
    }
    let config =
        crate::mesh::MeshConfig::loopback(body.peer, endpoint, body.private_payload.into_bytes());
    let module = crate::mesh::MeshModule::new(config)
        .map_err(|error| format!("{} 的 mesh 配置不合法: {error}", path.display()))?;
    Ok(Some(std::sync::Arc::new(module)))
}

/// 把 mesh 模块挂进装配：插件本体 + **两张授权表**（缺一不可）。
///
/// * `AgentHostPolicy.modules` —— 模块自己的代理授权，工具集 = **mesh 自己的**工具
///   （最小集，不是桌面全域工具）；
/// * `message_policy.grants` —— **Internal exchange 的投递授权**：只放行 mesh 自己的
///   `Internal` 面工具 + `allow_background`（生命周期 worker 是后台任务）
///   ⇒ ⇒ 一个出站目标都不授予（`targets` 空 ⇒ 不能给别的模块发消息）。
fn attach_local_mesh(
    builder: RuntimeBuilder,
    mut policy: AgentHostPolicy,
    module: std::sync::Arc<crate::mesh::MeshModule>,
) -> (RuntimeBuilder, AgentHostPolicy) {
    use apeireth_plugin::module_contract::ToolSurface;
    use apeireth_runtime::canonical::module_messages::{ModuleMessageGrant, ModuleMessagePolicy};
    use std::collections::BTreeSet;

    let module_id = module.manifest().id.clone();
    let tools: BTreeSet<_> = module
        .tools()
        .iter()
        .map(|tool| tool.id().clone())
        .collect();
    let internal: BTreeSet<_> = module
        .tools()
        .iter()
        .filter(|tool| tool.module_contract().surface == ToolSurface::Internal)
        .map(|tool| tool.id().clone())
        .collect();
    policy.modules.insert(
        module_id.clone(),
        ModuleAgentGrant {
            tools,
            max_instances: policy.max_instances,
            max_depth: apeireth_runtime::canonical::module::DEFAULT_MAX_INVOCATION_DEPTH,
            budget: policy.max_budget.clone(),
            ..Default::default()
        },
    );
    let message_policy: &mut ModuleMessagePolicy = policy
        .message_policy
        .get_or_insert_with(ModuleMessagePolicy::default);
    message_policy.grants.insert(
        module_id,
        ModuleMessageGrant {
            // 授权引用必须非空且可追溯：来源就是 Host 手写的那份配置。
            authorization_ref: format!("host:{MESH_CONFIG_FILE}:internal_exchange"),
            allow_background: true,
            internal_tools: internal,
            ..Default::default()
        },
    );
    (builder.with_plugin(module), policy)
}

/// Same registry, providers, sessions and loop; all rich entries delegate to main.
/// Private Profile mappings remain an explicit, independent Host choice.
pub async fn build_desktop_main_runtime(
    builder: RuntimeBuilder,
    policy: AgentHostPolicy,
) -> Result<Runtime, apeireth_runtime::RuntimeError> {
    build_desktop_main_runtime_at(builder, policy, &crate::paths::default_data_root()).await
}

/// [`build_desktop_main_runtime`] 的**数据根可注入**版本。
///
/// 生产入口只差一个数据根（`APEIRETH_DATA_DIR` / `~/.apeireth` 的单一真源）；
/// 注入版本让测试能用**同一段真代码**跑「有/无 `mesh.json`」两态，而不必碰环境变量。
pub async fn build_desktop_main_runtime_at(
    builder: RuntimeBuilder,
    policy: AgentHostPolicy,
    data_root: &std::path::Path,
) -> Result<Runtime, apeireth_runtime::RuntimeError> {
    // 监督者是否已绑定必须在装配前定案：mesh 的 Internal 面没有监督者就永远起不来。
    let mesh = local_mesh_module_at(data_root, builder.system_one_configured())
        .map_err(apeireth_runtime::RuntimeError::misconfigured)?;
    let (builder, policy) = match mesh {
        Some(module) => attach_local_mesh(builder, policy, module),
        // 无配置文件 ⇒ 原样返回（builder / policy 都没被碰过）。
        None => (builder, policy),
    };
    let budget = policy.max_budget.clone();
    let host = builder.build_agent_runtime(policy).await?;
    let tools = {
        let kernel = host.kernel().await;
        kernel
            .capability_registry()
            .capabilities()
            .iter()
            .map(|tool| tool.id().clone())
            .chain(
                kernel
                    .capability_registry()
                    .controls()
                    .iter()
                    .map(|tool| tool.id().clone()),
            )
            .collect()
    };
    host.into_main_runtime_with_session_quotas(AgentConfig {
        tools,
        budget,
        ..Default::default()
    })
    .await
}
