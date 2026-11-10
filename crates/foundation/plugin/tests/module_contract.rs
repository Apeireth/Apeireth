use apeireth_core::kernel::{CapabilityId, PluginId};
use apeireth_plugin::module_contract::*;
use apeireth_plugin::{Plugin, PluginManager, ToolCapability, ToolModule};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

struct Echo(CapabilityId);
#[async_trait]
impl ToolCapability for Echo {
    fn id(&self) -> &CapabilityId {
        &self.0
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new("echo")
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, call.arguments.clone()).with_name(&call.name)
    }
}
fn echo() -> Arc<dyn ToolCapability> {
    Arc::new(Echo(CapabilityId::new("tool.echo").unwrap()))
}
fn pid() -> PluginId {
    PluginId::new("module.contract").unwrap()
}

#[test]
fn existing_manifests_keep_compatibility_and_explicit_contracts_use_the_same_catalog() {
    let old = ToolModule::new(pid(), vec![echo()]).unwrap();
    assert_eq!(
        old.tools()[0].module_contract(),
        ModuleToolContract::default()
    );
    assert!(old.manifest().capabilities[0]
        .metadata
        .get(CONTRACT_METADATA_KEY)
        .is_none());
    let module = ToolModule::new(pid(), vec![echo()])
        .unwrap()
        .with_public_contract(ExecutionMode::Direct)
        .unwrap();
    let descriptor = &module.manifest().capabilities[0];
    let contract = descriptor.module_tool_contract().unwrap().unwrap();
    assert_eq!(contract.surface, ToolSurface::Logical);
    assert_eq!(contract.effect, EffectClass::Protected);
    assert_eq!(module.tools()[0].declaration().name, "echo");
    let mut manager = PluginManager::new();
    manager.register(Arc::new(module)).unwrap();
    let serialized = serde_json::to_value(old.manifest()).unwrap();
    let roundtrip: apeireth_plugin::PluginManifest = serde_json::from_value(serialized).unwrap();
    assert_eq!(&roundtrip, old.manifest());
}

#[test]
fn internal_contract_cannot_be_advertised_and_versions_fail_closed() {
    let mut contract = ModuleToolContract {
        surface: ToolSurface::Internal,
        effect: EffectClass::Protected,
        ..Default::default()
    };
    let tool = ContractTool::wrap(echo(), contract.clone()).unwrap();
    assert!(!tool.model_visible());
    contract.module_visible = true;
    assert!(ContractTool::wrap(echo(), contract.clone()).is_err());
    contract.module_visible = false;
    contract.version = 2;
    assert!(ContractTool::wrap(echo(), contract).is_err());
}

#[test]
fn manifest_cannot_silently_lie_about_the_executable_contract() {
    struct Bad {
        manifest: apeireth_plugin::PluginManifest,
        tool: Arc<dyn ToolCapability>,
    }
    #[async_trait]
    impl Plugin for Bad {
        fn manifest(&self) -> &apeireth_plugin::PluginManifest {
            &self.manifest
        }
        fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
            vec![self.tool.clone()]
        }
        async fn initialize(
            &self,
            _: &apeireth_plugin::PluginContext,
        ) -> apeireth_plugin::PluginResult<()> {
            Ok(())
        }
        async fn shutdown(&self) -> apeireth_plugin::PluginResult<()> {
            Ok(())
        }
    }
    let module = ToolModule::new(pid(), vec![echo()])
        .unwrap()
        .with_public_contract(ExecutionMode::Direct)
        .unwrap();
    let bad = Bad {
        manifest: module.manifest().clone(),
        tool: echo(),
    };
    assert!(PluginManager::new().register(Arc::new(bad)).is_err());
    let mut unknown = serde_json::to_value(ModuleToolContract::default()).unwrap();
    unknown["grant_host_authority"] = json!(true);
    assert!(serde_json::from_value::<ModuleToolContract>(unknown).is_err());
}
