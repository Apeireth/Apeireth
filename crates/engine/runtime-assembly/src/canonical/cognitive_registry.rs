//! Explicit Desktop Host authorization for its selected cognitive module tools.
use apeireth_governance::{Permission, PermissionPolicy};
use apeireth_runtime::RuntimeBuilder;
use std::sync::{Arc, Mutex};
impl super::ProductionModules {
    pub fn register_with_policy(
        self,
        builder: RuntimeBuilder,
        policy: &Arc<Mutex<PermissionPolicy>>,
    ) -> Result<RuntimeBuilder, String> {
        let capabilities = self
            .modules()
            .iter()
            .map(|module| {
                apeireth_runtime::canonical::behavior_adapter::registered_behavior_capability(
                    &module.manifest().id,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        {
            let mut policy = policy.lock().unwrap_or_else(|e| e.into_inner());
            for capability in capabilities {
                policy.grant(Permission::ExecuteTool(capability.to_string()));
            }
        }
        // **共享 live 策略装进工具面的槽**（2026-11-01 安全审计 P3-6）。
        // 原先错在哪：写工具面各持一份**自建**策略，与治理链那一份不是同一个对象
        // ⇒ 面板上的热撤销对已批准 / 预批的写调用**不生效**。
        // 现在为什么这样：装**同一个 `Arc`** ⇒ 工具面与治理链同源 ⇒ 撤销立刻可见。
        // 放在这里：本函数**本来就持有**那份策略的句柄 ⇒ 装的是同一个实例，
        // 且在**装配期**完成 ⇒ 与注册顺序无关。
        apeireth_tools_canonical::apply_patch_tool::install_shared_live_permission_policy(policy);
        let builder = self.register_context_projection(builder);
        let builder = self.register_into(builder);
        // `rt.module_execution_contract`: 工具模块的**生产契约选举点**。
        // `register_into` 只把模块挂进 builder（模块保持兼容合同）; 这里按装配表
        // （`module_contract_wiring::PRODUCTION_MODULE_CONTRACTS`）把表内模块
        // 物化为既有 `ContractTool` 表面。表内缺席 = 逐字节零变化；已选举 =
        // 跳过（幂等）。只收窄面, 不授予权限。
        super::module_contract_wiring::attach(builder)
    }
}
