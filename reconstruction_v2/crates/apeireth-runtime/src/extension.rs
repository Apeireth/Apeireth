//! Extension - 扩展点机制 (从 v1.0 apeireth-extension 3,173 LOC 收敛)
//!
//! 0 装 PASS: 重构版 extension 用 v2 现有 apeireth_tools::Tool trait + CapabilityRegistry
//! 提供统一扩展点, 不再独立管理 plugin lifecycle.
//!
//! 设计 (per right 图 "Unified Runtime Host" 扩展点):
//! - ExtensionPoint: 命名扩展点 (e.g. "tool-search" / "memory-persist")
//! - ExtensionHandler: 注册到某 extension point 的回调
//! - ExtensionRegistry: 全局注册表

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// 扩展点 (per 阶段 1.4 CapabilityRegistry 协同)
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionPoint(pub String);

impl ExtensionPoint {
    /// 0 装 PASS: 预定义常用扩展点 (pub fn 而非 const, 因为 String::from 不能在 const 调用)
    pub fn tool_search() -> Self { Self("tool-search".into()) }
    pub fn memory_persist() -> Self { Self("memory-persist".into()) }
    pub fn lifecycle_hook() -> Self { Self("lifecycle-hook".into()) }
    pub fn governance_override() -> Self { Self("governance-override".into()) }
}

/// 扩展点处理 trait (异步, 任意输入输出)
#[async_trait::async_trait]
pub trait ExtensionHandler: Send + Sync {
    fn name(&self) -> &'static str;
    /// 0 装 PASS: 处理 + 返回 (Result 包装, 失败不影响其他 handler)
    async fn handle(&self, input: serde_json::Value) -> Result<serde_json::Value, String>;
}

/// 全局注册表
pub struct ExtensionRegistry {
    handlers: RwLock<HashMap<ExtensionPoint, Vec<Arc<dyn ExtensionHandler>>>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self { Self { handlers: RwLock::new(HashMap::new()) } }

    pub async fn register(&self, point: ExtensionPoint, handler: Arc<dyn ExtensionHandler>) {
        let mut h = self.handlers.write().await;
        h.entry(point).or_insert_with(Vec::new).push(handler);
    }

    /// 0 装 PASS: 调用该 point 的所有 handlers, 失败返 Err 但不 panic
    pub async fn invoke(&self, point: &ExtensionPoint, input: serde_json::Value) -> Vec<Result<serde_json::Value, String>> {
        let h = self.handlers.read().await;
        match h.get(point) {
            Some(handlers) => {
                let mut results = Vec::new();
                for h in handlers {
                    results.push(h.handle(input.clone()).await);
                }
                results
            }
            None => Vec::new(),
        }
    }

    pub async fn count(&self, point: &ExtensionPoint) -> usize {
        self.handlers.read().await.get(point).map_or(0, |v| v.len())
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct EchoHandler;
    #[async_trait]
    impl ExtensionHandler for EchoHandler {
        fn name(&self) -> &'static str { "echo" }
        async fn handle(&self, input: serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(input)
        }
    }

    struct FailHandler;
    #[async_trait]
    impl ExtensionHandler for FailHandler {
        fn name(&self) -> &'static str { "fail" }
        async fn handle(&self, _input: serde_json::Value) -> Result<serde_json::Value, String> {
            Err("simulated failure".into())
        }
    }

    #[test]
    fn test_extension_point_constants() {
        assert_eq!(ExtensionPoint::tool_search().0, "tool-search");
        assert_eq!(ExtensionPoint::memory_persist().0, "memory-persist");
    }

    #[tokio::test]
    async fn test_registry_invoke_chain() {
        let reg = ExtensionRegistry::new();
        reg.register(ExtensionPoint::tool_search(), Arc::new(EchoHandler)).await;
        reg.register(ExtensionPoint::tool_search(), Arc::new(FailHandler)).await;
        let results = reg.invoke(&ExtensionPoint::tool_search(), serde_json::json!({"x": 1})).await;
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
    }

    #[tokio::test]
    async fn test_registry_count() {
        let reg = ExtensionRegistry::new();
        assert_eq!(reg.count(&ExtensionPoint::tool_search()).await, 0);
        reg.register(ExtensionPoint::tool_search(), Arc::new(EchoHandler)).await;
        assert_eq!(reg.count(&ExtensionPoint::tool_search()).await, 1);
    }
}
