//! ServicePlugin — 长驻 service, 启动后持续运行
//!
//! `start` 启动 service, `stop` 停止, `call` 与 service 交互.

use crate::error::{ExtensionError, Result};
use crate::manifest::Manifest;
use crate::traits::{AsyncExtension, ExtensionInput, ExtensionOutput};
use crate::types::PluginKind;
use async_trait::async_trait;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Service 状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceState {
    /// 未启动
    Stopped,
    /// 运行中
    Running,
    /// 已停止 (启动过)
    Terminated,
}

/// 长驻 service 插件
pub struct ServicePlugin {
    manifest: Manifest,
    state: Arc<AtomicBool>, // false=stopped/termininated, true=running
    state_kind: Arc<std::sync::Mutex<ServiceState>>,
    counter: Arc<AtomicU64>,
}

impl ServicePlugin {
    /// 构造
    pub fn new(manifest: Manifest) -> Self {
        Self {
            manifest,
            state: Arc::new(AtomicBool::new(false)),
            state_kind: Arc::new(std::sync::Mutex::new(ServiceState::Stopped)),
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 启动 service
    pub fn start(&self) -> Result<()> {
        let mut s = self.state_kind.lock().unwrap();
        if *s == ServiceState::Terminated {
            return Err(ExtensionError::Execution(
                "service already terminated, cannot restart".into(),
            ));
        }
        self.state.store(true, Ordering::SeqCst);
        *s = ServiceState::Running;
        Ok(())
    }

    /// 停止 service
    pub fn stop(&self) {
        self.state.store(false, Ordering::SeqCst);
        let mut s = self.state_kind.lock().unwrap();
        *s = ServiceState::Terminated;
    }

    /// 当前状态
    pub fn state(&self) -> ServiceState {
        *self.state_kind.lock().unwrap()
    }

    /// 计数器
    pub fn counter(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }

    /// 示例: 计数器 service
    pub fn example_counter(name: impl Into<String>) -> Self {
        Self::new(Manifest {
            name: name.into(),
            version: "0.1.0".into(),
            kind: PluginKind::Service,
            description: "Example service plugin (counter)".into(),
            entry: "counter.rs".into(),
            permissions: vec!["system".into(), "invoke".into()],
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
        })
    }
}

#[async_trait]
impl AsyncExtension for ServicePlugin {
    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn kind(&self) -> PluginKind {
        PluginKind::Service
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    async fn call(&self, input: ExtensionInput) -> Result<ExtensionOutput> {
        if !self.state.load(Ordering::SeqCst) {
            return Err(ExtensionError::Execution("service not running".into()));
        }
        let op = input
            .args
            .get("op")
            .and_then(|v| v.as_str())
            .unwrap_or("inc");
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        let result = match op {
            "inc" => json!({"op": "inc", "counter": n + 1}),
            "get" => json!({"op": "get", "counter": n}),
            _ => json!({"op": op, "counter": n, "error": "unknown op"}),
        };
        Ok(ExtensionOutput::ok(result))
    }
}

impl Drop for ServicePlugin {
    fn drop(&mut self) {
        if self.state.load(Ordering::SeqCst) {
            self.stop();
        }
    }
}

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn service_lifecycle() {
        let p = ServicePlugin::example_counter("counter-1");
        assert_eq!(p.state(), ServiceState::Stopped);
        // call before start fails
        let res = p.call(ExtensionInput::new(json!({"op": "inc"}))).await;
        assert!(matches!(res, Err(ExtensionError::Execution(_))));
        // start
        p.start().unwrap();
        assert_eq!(p.state(), ServiceState::Running);
        // call works
        let out = p
            .call(ExtensionInput::new(json!({"op": "inc"})))
            .await
            .unwrap();
        assert_eq!(out.result["counter"], 1);
        let out = p
            .call(ExtensionInput::new(json!({"op": "inc"})))
            .await
            .unwrap();
        assert_eq!(out.result["counter"], 2);
        // stop
        p.stop();
        assert_eq!(p.state(), ServiceState::Terminated);
        // cannot call after stop
        let res = p.call(ExtensionInput::new(json!({"op": "inc"}))).await;
        assert!(matches!(res, Err(ExtensionError::Execution(_))));
    }

    #[tokio::test]
    async fn service_get_op() {
        let p = ServicePlugin::example_counter("counter-2");
        p.start().unwrap();
        let out = p
            .call(ExtensionInput::new(json!({"op": "get"})))
            .await
            .unwrap();
        assert_eq!(out.result["counter"], 0);
    }

    #[tokio::test]
    async fn service_cannot_restart_after_terminate() {
        let p = ServicePlugin::example_counter("counter-3");
        p.start().unwrap();
        p.stop();
        let res = p.start();
        assert!(matches!(res, Err(ExtensionError::Execution(_))));
    }

    #[tokio::test]
    async fn service_kind() {
        let p = ServicePlugin::example_counter("counter-4");
        assert_eq!(p.kind(), PluginKind::Service);
    }
}
