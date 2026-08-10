//! HybridPlugin — 同步入口 + 异步后端 (混合)
//!
//! 内部使用 tokio::sync::mpsc: call() 立刻入队返回, 后台异步处理.
//! 适合"立即受理" + "后端批量"场景.

use crate::error::{ExtensionError, Result};
use crate::manifest::Manifest;
use crate::traits::{AsyncExtension, ExtensionInput, ExtensionOutput};
use crate::types::PluginKind;
use async_trait::async_trait;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// 混合插件 (同步入口 + 异步后端)
pub struct HybridPlugin {
    manifest: Manifest,
    tx: Sender<serde_json::Value>,
    queue_size: Arc<AtomicUsize>,
}

impl HybridPlugin {
    /// 构造 (同步入口向 tx 发送, 后台 worker 异步处理)
    pub fn new<F, Fut>(manifest: Manifest, backend: F) -> Self
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value>> + Send + 'static,
    {
        let (tx, mut rx): (Sender<_>, Receiver<_>) = channel(64);
        let queue_size = Arc::new(AtomicUsize::new(0));
        let queue_for_worker = queue_size.clone();
        // Spawn backend worker (async recv, no blocking)
        tokio::spawn(async move {
            while let Some(args) = rx.recv().await {
                let result = backend(args).await;
                queue_for_worker.fetch_sub(1, Ordering::SeqCst);
                if result.is_err() {
                    break;
                }
            }
        });
        Self {
            manifest,
            tx,
            queue_size,
        }
    }

    /// 当前队列长度
    pub fn queue_len(&self) -> usize {
        self.queue_size.load(Ordering::SeqCst)
    }

    /// 示例: 异步 echo
    pub fn example_echo(name: impl Into<String>) -> Self {
        Self::new(
            Manifest {
                name: name.into(),
                version: "0.1.0".into(),
                kind: PluginKind::Hybrid,
                description: "Example hybrid plugin (sync enqueue, async process)".into(),
                entry: "echo.rs".into(),
                permissions: vec!["invoke".into(), "read".into()],
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 5000,
            },
            |args| async move {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                Ok(json!({"echo": args, "processed": true}))
            },
        )
    }
}

#[async_trait]
impl AsyncExtension for HybridPlugin {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> PluginKind {
        PluginKind::Hybrid
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn call(&self, input: ExtensionInput) -> Result<ExtensionOutput> {
        // sync enqueue
        self.tx
            .send(input.args.clone())
            .await
            .map_err(|e| ExtensionError::Execution(format!("enqueue failed: {e}")))?;
        self.queue_size.fetch_add(1, Ordering::SeqCst);
        // wait briefly for worker to process at least one item
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(ExtensionOutput::ok(json!({
            "enqueued": true,
            "queue_size": self.queue_len(),
        })))
    }
}

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn hybrid_plugin_enqueue() {
        let p = HybridPlugin::example_echo("echo-1");
        let out = p.call(ExtensionInput::new(json!({"x": 1}))).await.unwrap();
        assert!(out.success);
        assert_eq!(out.result["enqueued"], true);
    }

    #[tokio::test]
    async fn hybrid_plugin_queue_drains() {
        let p = std::sync::Arc::new(HybridPlugin::example_echo("echo-2"));
        for i in 0..3 {
            p.call(ExtensionInput::new(json!({"i": i}))).await.unwrap();
        }
        // wait for worker to drain (3 items × 20ms + slack)
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        // queue should be 0 after drain
        assert_eq!(p.queue_len(), 0, "queue should be drained");
    }

    #[tokio::test]
    async fn hybrid_plugin_kind() {
        let p = HybridPlugin::example_echo("echo-3");
        assert_eq!(p.kind(), PluginKind::Hybrid);
    }
}
