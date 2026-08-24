//! StaticPlugin — 启动期一次性加载, 不可热替换
//!
//! 与 SyncPlugin 的区别: Static 插件 `load()` 一次后状态固化,
//! `call` 只读访问固化数据.

use crate::error::Result;
use crate::manifest::Manifest;
use crate::traits::{AsyncExtension, ExtensionInput, ExtensionOutput};
use crate::types::PluginKind;
use async_trait::async_trait;
use serde_json::json;
use std::sync::{Arc, OnceLock};

/// 静态插件 (load 一次, 之后只读)
pub struct StaticPlugin {
    manifest: Manifest,
    /// 加载后的固化数据
    loaded: Arc<OnceLock<serde_json::Value>>,
    /// 加载函数
    loader: Arc<dyn Fn() -> serde_json::Value + Send + Sync>,
}

impl StaticPlugin {
    /// 构造
    pub fn new<F>(manifest: Manifest, loader: F) -> Self
    where
        F: Fn() -> serde_json::Value + Send + Sync + 'static,
    {
        Self {
            manifest,
            loaded: Arc::new(OnceLock::new()),
            loader: Arc::new(loader),
        }
    }

    /// 强制 load 一次 (后续 call 不再 load)
    pub fn load(&self) -> &serde_json::Value {
        self.loaded.get_or_init(|| (self.loader)())
    }

    /// 是否已 load
    pub fn is_loaded(&self) -> bool {
        self.loaded.get().is_some()
    }

    /// 示例: 启动期加载常量表
    pub fn example_lookup(name: impl Into<String>) -> Self {
        Self::new(
            Manifest {
                name: name.into(),
                version: "0.1.0".into(),
                kind: PluginKind::Static,
                description: "Example static plugin (lookup table loaded once)".into(),
                entry: "lookup.rs".into(),
                permissions: vec!["read".into()],
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 1000,
            },
            || {
                json!({
                    "alpha": 1,
                    "beta": 2,
                    "gamma": 3,
                })
            },
        )
    }
}

#[async_trait]
impl AsyncExtension for StaticPlugin {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> PluginKind {
        PluginKind::Static
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn call(&self, input: ExtensionInput) -> Result<ExtensionOutput> {
        // 触发 load
        let data = self.load();
        let key = input.args.get("key").and_then(|v| v.as_str()).unwrap_or("");
        let value = data.get(key).cloned().unwrap_or(serde_json::Value::Null);
        Ok(ExtensionOutput::ok(json!({"key": key, "value": value})))
    }
}

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn static_plugin_loads_once() {
        let p = StaticPlugin::example_lookup("lookup-1");
        assert!(!p.is_loaded());
        let _ = p.load();
        assert!(p.is_loaded());
        // call also triggers load
        let p2 = StaticPlugin::example_lookup("lookup-2");
        assert!(!p2.is_loaded());
        let out = p2
            .call(ExtensionInput::new(json!({"key": "alpha"})))
            .await
            .unwrap();
        assert!(p2.is_loaded());
        assert_eq!(out.result["value"], 1);
    }

    #[tokio::test]
    async fn static_plugin_unknown_key() {
        let p = StaticPlugin::example_lookup("lookup-3");
        let out = p
            .call(ExtensionInput::new(json!({"key": "missing"})))
            .await
            .unwrap();
        assert_eq!(out.result["value"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn static_plugin_kind() {
        let p = StaticPlugin::example_lookup("lookup-4");
        assert_eq!(p.kind(), PluginKind::Static);
    }
}
