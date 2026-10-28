use apeireth_core::kernel::{system_clock, CapabilityId, Lifecycle, PluginId, TraceId};
use apeireth_plugin::credentials::NoCredentials;
use apeireth_plugin::resource_factory::{
    module_from_definition, ModuleDefinition, ModuleResources, OwnedModuleResource,
};
use apeireth_plugin::{
    CapabilityKind, Plugin, PluginContext, PluginError, PluginManager, PluginManifest,
    PluginResult, ToolCapability, ToolModule,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
struct Resource {
    revoked: AtomicBool,
    closes: AtomicUsize,
    fail_close: AtomicBool,
}
#[async_trait]
impl OwnedModuleResource for Resource {
    fn revoke(&self) {
        self.revoked.store(true, Ordering::SeqCst);
    }
    async fn close(&self) -> Result<(), String> {
        self.closes.fetch_add(1, Ordering::SeqCst);
        if self.fail_close.swap(false, Ordering::SeqCst) {
            Err("controlled close failure".into())
        } else {
            Ok(())
        }
    }
}
struct Tool {
    id: CapabilityId,
    resource: Arc<Resource>,
}
#[async_trait]
impl ToolCapability for Tool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }
    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "factory_probe".into(),
            description: Some("Factory test".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        assert!(
            !self.resource.revoked.load(Ordering::SeqCst),
            "revoked resource executed"
        );
        ToolResult::ok(&call.id, serde_json::json!("fresh resource"))
    }
}
struct Definition {
    manifest: PluginManifest,
    resources: Mutex<Vec<Arc<Resource>>>,
    fail: AtomicBool,
}
impl Definition {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            manifest: PluginManifest::new(
                PluginId::new("module.factory_probe").unwrap(),
                "1.0.0",
                "Factory test",
            )
            .declare_capability(
                CapabilityId::new("tool.factory_probe").unwrap(),
                CapabilityKind::Tool,
                "Factory test",
            )
            .unwrap(),
            resources: Mutex::new(vec![]),
            fail: AtomicBool::new(false),
        })
    }
}
#[async_trait]
impl ModuleDefinition for Definition {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    async fn instantiate(
        &self,
        _: &PluginContext,
        resources: &ModuleResources,
    ) -> PluginResult<Arc<dyn Plugin>> {
        let resource = Arc::new(Resource::default());
        resources.own("connection", resource.clone()).unwrap();
        self.resources.lock().unwrap().push(resource.clone());
        if self.fail.load(Ordering::SeqCst) {
            return Err(PluginError::PluginFailed {
                plugin: self.manifest.id.clone(),
                phase: "factory",
                reason: "controlled creation failure".into(),
            });
        }
        Ok(Arc::new(ToolModule::new(
            self.manifest.id.clone(),
            vec![Arc::new(Tool {
                id: self.manifest.capabilities[0].id.clone(),
                resource,
            })],
        )?))
    }
}
// The factory and returned plugin must agree on the exact declaration, including description.
fn definition() -> Arc<Definition> {
    let mut d = Definition::new();
    Arc::get_mut(&mut d).unwrap().manifest = ToolModule::new(
        PluginId::new("module.factory_probe").unwrap(),
        vec![Arc::new(Tool {
            id: CapabilityId::new("tool.factory_probe").unwrap(),
            resource: Arc::new(Resource::default()),
        })],
    )
    .unwrap()
    .manifest()
    .clone();
    d
}
fn context() -> PluginContext {
    PluginContext::new(system_clock(), Arc::new(NoCredentials), TraceId::new())
}
fn call() -> ToolCall {
    ToolCall {
        id: "factory-call".into(),
        name: "factory_probe".into(),
        arguments: serde_json::json!({}),
    }
}

#[tokio::test]
async fn one_definition_creates_independent_instances_and_old_handles_never_revive() {
    let d = definition();
    let id = &d.manifest.id;
    let mut a = PluginManager::new();
    let mut b = PluginManager::new();
    let old = module_from_definition(d.clone());
    a.register(old.clone()).unwrap();
    b.register(module_from_definition(d.clone())).unwrap();
    a.initialize(id, &context()).await.unwrap();
    let handle = old.tools().pop().unwrap();
    assert!(
        !handle.invoke(&call()).await.is_ok(),
        "initialized is not running"
    );
    a.start(id, &context()).await.unwrap();
    b.start_all(&context()).await.unwrap();
    assert_ne!(a.instance_id(id).unwrap(), b.instance_id(id).unwrap());
    assert!(handle.invoke(&call()).await.is_ok());
    a.stop(id).await.unwrap();
    a.stop(id).await.unwrap();
    assert!(!handle.invoke(&call()).await.is_ok());
    assert_eq!(
        d.resources.lock().unwrap()[0].closes.load(Ordering::SeqCst),
        1
    );
    assert!(!d.resources.lock().unwrap()[1]
        .revoked
        .load(Ordering::SeqCst));
    a.unregister(id).unwrap();
    a.register(module_from_definition(d.clone())).unwrap();
    a.start_all(&context()).await.unwrap();
    assert!(
        !handle.invoke(&call()).await.is_ok(),
        "new registration must not revive old Arc"
    );
    assert_eq!(d.resources.lock().unwrap().len(), 3);
    a.shutdown_all().await;
    b.shutdown_all().await;
}
#[tokio::test]
async fn creation_failure_rolls_back_owned_resources_and_leaves_no_callable_tool() {
    let d = definition();
    d.fail.store(true, Ordering::SeqCst);
    let module = module_from_definition(d.clone());
    let mut manager = PluginManager::new();
    manager.register(module.clone()).unwrap();
    assert!(manager.start_all_or_rollback(&context()).await.is_err());
    assert_eq!(manager.state(&d.manifest.id).unwrap(), Lifecycle::Failed);
    assert!(module.tools().is_empty());
    let resources = d.resources.lock().unwrap();
    assert!(resources[0].revoked.load(Ordering::SeqCst));
    assert_eq!(resources[0].closes.load(Ordering::SeqCst), 1);
    manager.unregister(&d.manifest.id).unwrap();
}
#[tokio::test]
async fn failed_close_is_observable_retryable_and_successful_close_is_idempotent() {
    let resources = ModuleResources::default();
    let resource = Arc::new(Resource::default());
    resource.fail_close.store(true, Ordering::SeqCst);
    resources.own("connection", resource.clone()).unwrap();
    assert!(resources.close().await.is_err());
    assert!(!resources.statuses()[0].closed);
    assert!(resources.statuses()[0].error.is_some());
    assert!(resource.revoked.load(Ordering::SeqCst));
    resources.close().await.unwrap();
    resources.close().await.unwrap();
    assert!(resources.statuses()[0].closed);
    assert!(resources.statuses()[0].error.is_none());
    assert_eq!(resource.closes.load(Ordering::SeqCst), 2);
    assert!(resources
        .own("late", Arc::new(Resource::default()))
        .is_err());
}
