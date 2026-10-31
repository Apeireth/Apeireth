use apeireth_core::kernel::{system_clock, Lifecycle, PluginId, TraceId};
use apeireth_plugin::credentials::NoCredentials;
use apeireth_plugin::task_scope::ModuleTaskScope;
use apeireth_plugin::{Plugin, PluginContext, PluginManager, PluginManifest, PluginResult};
use async_trait::async_trait;
use std::sync::Arc;

#[test]
fn children_are_revoked_independently_and_native_work_remains_owned_by_parent() {
    let parent = ModuleTaskScope::new();
    let first = parent.child();
    let peer = parent.child();
    let signal = apeireth_plugin::ToolCancellation::default();
    let lease = first.lease_with_cancellation(signal.clone()).unwrap();
    first.close();
    assert!(signal.is_cancelled());
    assert!(peer.is_open());
    assert_eq!(parent.active_tasks(), 1);
    assert_eq!(first.active_tasks(), 1);
    assert!(first.lease().is_err());
    drop(lease);
    assert_eq!(parent.active_tasks(), 0);
    let signal = apeireth_plugin::ToolCancellation::default();
    let lease = peer.lease_with_cancellation(signal.clone()).unwrap();
    parent.close();
    assert!(signal.is_cancelled());
    assert!(!peer.is_open());
    assert!(peer.lease().is_err());
    drop(lease);
    futures::executor::block_on(parent.close_and_wait());
}
struct Module {
    manifest: PluginManifest,
}
#[async_trait]
impl Plugin for Module {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }
}
#[tokio::test]
async fn drain_deadline_preserves_stopping_and_can_retry_cleanup_without_losing_owner() {
    let id = PluginId::new("module.drain").unwrap();
    let mut manager = PluginManager::new();
    manager
        .register(Arc::new(Module {
            manifest: PluginManifest::new(id.clone(), "1.0.0", "drain"),
        }))
        .unwrap();
    let context = PluginContext::new(system_clock(), Arc::new(NoCredentials), TraceId::new());
    manager.start_all(&context).await.unwrap();
    let scope = manager.task_scope(&id).unwrap();
    let lease = scope.lease().unwrap();
    assert!(manager
        .stop_until(&id, futures::future::ready(()))
        .await
        .is_err());
    assert_eq!(manager.state(&id).unwrap(), Lifecycle::Stopping);
    assert!(!scope.is_open());
    assert!(!manager.errors(&id).unwrap().is_empty());
    assert!(manager.unregister(&id).is_err());
    drop(lease);
    manager
        .stop_until(&id, futures::future::pending())
        .await
        .unwrap();
    assert_eq!(manager.state(&id).unwrap(), Lifecycle::Stopped);
    manager.stop(&id).await.unwrap();
    manager.unregister(&id).unwrap();
}
