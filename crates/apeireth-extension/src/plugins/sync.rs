//! SyncPlugin — 同步执行
//!
//! `call` 内部立即返回结果, 不挂起.

use crate::error::{ExtensionError, Result};
use crate::manifest::Manifest;
use crate::traits::{AsyncExtension, ExtensionInput, ExtensionOutput};
use crate::types::PluginKind;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// 同步插件 (签名 `(args: Value) -> Result<Value>`)
pub type SyncFn = Arc<dyn Fn(serde_json::Value) -> Result<serde_json::Value> + Send + Sync>;

/// 同步插件
pub struct SyncPlugin {
    manifest: Manifest,
    func: SyncFn,
}

impl SyncPlugin {
    /// 构造
    pub fn new<F>(manifest: Manifest, func: F) -> Self
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value> + Send + Sync + 'static,
    {
        Self {
            manifest,
            func: Arc::new(func),
        }
    }

    /// 简单加法插件 (测试用)
    pub fn example_add(name: impl Into<String>) -> Self {
        let m = Manifest {
            name: name.into(),
            version: "0.1.0".into(),
            kind: PluginKind::Sync,
            description: "Example sync plugin (adds two numbers)".into(),
            entry: "add.rs".into(),
            permissions: vec!["invoke".into()],
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
        };
        Self::new(m, |args| {
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(json!({"sum": a + b}))
        })
    }
}

#[async_trait]
impl AsyncExtension for SyncPlugin {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> PluginKind {
        PluginKind::Sync
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn call(&self, input: ExtensionInput) -> Result<ExtensionOutput> {
        match (self.func)(input.args) {
            Ok(v) => Ok(ExtensionOutput::ok(v)),
            Err(e) => Err(ExtensionError::Execution(e.to_string())),
        }
    }
}

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn sync_plugin_basic() {
        let p = SyncPlugin::example_add("add-1");
        let out = p
            .call(ExtensionInput::new(json!({"a": 2.0, "b": 3.0})))
            .await
            .unwrap();
        assert!(out.success);
        assert_eq!(out.result["sum"], 5.0);
    }

    #[tokio::test]
    async fn sync_plugin_kind() {
        let p = SyncPlugin::example_add("add-2");
        assert_eq!(p.kind(), PluginKind::Sync);
        assert_eq!(p.name(), "add-2");
    }

    #[tokio::test]
    async fn sync_plugin_propagates_error() {
        let p = SyncPlugin::new(
            Manifest {
                name: "fail".into(),
                version: "0.1.0".into(),
                kind: PluginKind::Sync,
                description: "always fails".into(),
                entry: "fail.rs".into(),
                permissions: vec!["invoke".into()],
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 1000,
            },
            |_args| Err(ExtensionError::Execution("nope".into())),
        );
        let res = p.call(ExtensionInput::new(json!({}))).await;
        assert!(matches!(res, Err(ExtensionError::Execution(_))));
    }
}
