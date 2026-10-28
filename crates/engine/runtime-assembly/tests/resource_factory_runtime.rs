#[path = "support/convergence.rs"]
mod support;
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_plugin::resource_factory::{
    module_from_definition, ModuleDefinition, ModuleResources, OwnedModuleResource,
};
use apeireth_plugin::{
    Plugin, PluginContext, PluginManifest, PluginResult, ToolCapability, ToolModule,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use apeireth_runtime::TurnRequest;
use async_trait::async_trait;
use serde_json::json;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use support::*;
#[derive(Default)]
struct Resource {
    closed: AtomicBool,
    calls: AtomicUsize,
}
#[async_trait]
impl OwnedModuleResource for Resource {
    fn revoke(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
    async fn close(&self) -> Result<(), String> {
        self.revoke();
        Ok(())
    }
}
struct Echo {
    id: CapabilityId,
    resource: Arc<Resource>,
}
#[async_trait]
impl ToolCapability for Echo {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "factory_echo".into(),
            description: Some("Independent instance".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        assert!(!self.resource.closed.load(Ordering::SeqCst));
        self.resource.calls.fetch_add(1, Ordering::SeqCst);
        ToolResult::ok(&call.id, json!("factory echo"))
    }
}
fn module(resource: Arc<Resource>) -> ToolModule {
    ToolModule::new(
        pid("module.factory_echo"),
        vec![Arc::new(Echo {
            id: CapabilityId::new("tool.factory_echo").unwrap(),
            resource,
        })],
    )
    .unwrap()
}
struct Definition {
    manifest: PluginManifest,
    resources: Mutex<Vec<Arc<Resource>>>,
}
#[async_trait]
impl ModuleDefinition for Definition {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    async fn instantiate(
        &self,
        ctx: &PluginContext,
        resources: &ModuleResources,
    ) -> PluginResult<Arc<dyn Plugin>> {
        assert!(ctx.module_instance().is_some());
        let resource = Arc::new(Resource::default());
        resources.own("echo", resource.clone()).unwrap();
        self.resources.lock().unwrap().push(resource.clone());
        Ok(Arc::new(module(resource)))
    }
}
#[tokio::test]
async fn real_runtime_dispatches_factory_tool_then_revokes_old_arc_across_replacement() {
    let definition = Arc::new(Definition {
        manifest: module(Arc::new(Resource::default())).manifest().clone(),
        resources: Mutex::new(vec![]),
    });
    let provider = Provider::new(vec![vec![call("factory_echo", json!({}))]]);
    let plugin = module_from_definition(definition.clone());
    let mut runtime = builder(provider.clone())
        .with_plugin(plugin.clone())
        .build()
        .await
        .unwrap();
    let session = SessionId::new();
    runtime
        .execute(TurnRequest::new(session, "echo"))
        .await
        .unwrap();
    assert_eq!(
        definition.resources.lock().unwrap()[0]
            .calls
            .load(Ordering::SeqCst),
        1
    );
    assert!(facts(&runtime, session)
        .await
        .iter()
        .any(|fact| fact.call().name == "factory_echo"));
    let raw_handle = plugin.tools().pop().unwrap();
    let old_instance = runtime
        .plugins()
        .instance_id(&definition.manifest.id)
        .unwrap();
    runtime.stop_module(&definition.manifest.id).await.unwrap();
    runtime.unregister_module(&definition.manifest.id).unwrap();
    assert!(plugin
        .resource_statuses()
        .iter()
        .all(|status| status.closed));
    let ctx = PluginContext::new(
        runtime.clock().clone(),
        Arc::new(apeireth_plugin::credentials::NoCredentials),
        TraceId::new(),
    );
    runtime
        .register_module(module_from_definition(definition.clone()), &ctx)
        .await
        .unwrap();
    assert_ne!(
        old_instance,
        runtime
            .plugins()
            .instance_id(&definition.manifest.id)
            .unwrap()
    );
    assert!(!raw_handle
        .invoke(&call("factory_echo", json!({})))
        .await
        .is_ok());
    provider.enqueue_tools(vec![call("factory_echo", json!({}))]);
    runtime
        .execute(TurnRequest::new(SessionId::new(), "new echo"))
        .await
        .unwrap();
    let resources = definition.resources.lock().unwrap();
    assert_eq!(resources[0].calls.load(Ordering::SeqCst), 1);
    assert_eq!(resources[1].calls.load(Ordering::SeqCst), 1);
    drop(resources);
    assert!(runtime.shutdown().await.is_empty());
}
