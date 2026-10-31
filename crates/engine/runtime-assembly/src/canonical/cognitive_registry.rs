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
        let builder = self.register_context_projection(builder);
        Ok(self.register_into(builder))
    }
}
