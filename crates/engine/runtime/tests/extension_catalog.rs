//! R2-3b：BehaviorRegistry 的 ExtensionCatalog 读取面验收。

use std::sync::Arc;

use apeireth_plugin::{ExtensionCatalog, ExtensionKind};
use apeireth_runtime::canonical::{BehaviorModule, BehaviorRegistry, ModuleManifest};

struct TestModule {
    manifest: ModuleManifest,
}

impl BehaviorModule for TestModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
}

#[test]
fn behavior_registry_exposes_extension_catalog() {
    let mut registry = BehaviorRegistry::new();
    registry
        .register(Arc::new(TestModule {
            manifest: ModuleManifest::new("m.b", "B"),
        }))
        .unwrap();

    assert_eq!(ExtensionCatalog::extension_ids(&registry), vec!["m.b"]);
    assert!(ExtensionCatalog::contains(&registry, "m.b"));
    assert!(!ExtensionCatalog::contains(&registry, "m.none"));

    let descriptor = ExtensionCatalog::describe(&registry, "m.b").unwrap();
    assert_eq!(descriptor.kind, ExtensionKind::BehaviorModule);
    assert_eq!(descriptor.owner_crate, "apeireth-runtime");
    assert_eq!(descriptor.id, "m.b");
}
