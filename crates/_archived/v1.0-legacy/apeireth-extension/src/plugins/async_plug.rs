//! AsyncPlugin — 异步执行 (返回 Future)

use crate::error::{ExtensionError, Result};
use crate::manifest::Manifest;
use crate::traits::{AsyncExtension, ExtensionInput, ExtensionOutput};
use crate::types::PluginKind;
use async_trait::async_trait;
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// 异步函数签名
pub type AsyncFn = Arc<
    dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

/// 异步插件
pub struct AsyncPlugin {
    manifest: Manifest,
    func: AsyncFn,
}

impl AsyncPlugin {
    /// 构造
    pub fn new<F, Fut>(manifest: Manifest, func: F) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<serde_json::Value>> + Send + 'static,
    {
        let wrapped: AsyncFn = Arc::new(move |v| Box::pin(func(v)));
        Self {
            manifest,
            func: wrapped,
        }
    }

    /// 示例: 模拟 IO 延迟 10ms 后返回
    pub fn example_io(name: impl Into<String>) -> Self {
        Self::new(
            Manifest {
                name: name.into(),
                version: "0.1.0".into(),
                kind: PluginKind::Async,
                description: "Example async plugin (simulates IO delay)".into(),
                entry: "io.rs".into(),
                permissions: vec!["invoke".into(), "read".into()],
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 5000,
            },
            |args| async move {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                Ok(json!({"query": query, "result": "async-ok"}))
            },
        )
    }
}

#[async_trait]
impl AsyncExtension for AsyncPlugin {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> PluginKind {
        PluginKind::Async
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn call(&self, input: ExtensionInput) -> Result<ExtensionOutput> {
        match (self.func)(input.args).await {
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
    async fn async_plugin_returns() {
        let p = AsyncPlugin::example_io("io-1");
        let out = p
            .call(ExtensionInput::new(json!({"query": "ping"})))
            .await
            .unwrap();
        assert!(out.success);
        assert_eq!(out.result["result"], "async-ok");
        assert_eq!(out.result["query"], "ping");
    }

    #[tokio::test]
    async fn async_plugin_concurrent_safe() {
        let p = std::sync::Arc::new(AsyncPlugin::example_io("io-2"));
        let mut handles = vec![];
        for i in 0..5 {
            let pp = p.clone();
            handles.push(tokio::spawn(async move {
                pp.call(ExtensionInput::new(json!({"query": format!("q{i}")})))
                    .await
            }));
        }
        for h in handles {
            let out = h.await.unwrap().unwrap();
            assert!(out.success);
        }
    }

    #[tokio::test]
    async fn async_plugin_kind() {
        let p = AsyncPlugin::example_io("io-3");
        assert_eq!(p.kind(), PluginKind::Async);
    }
}
