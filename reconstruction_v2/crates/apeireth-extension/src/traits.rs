//! traits — 异步扩展 trait + 输入/输出包装
//!
//! 全部 6 类插件 (sync/async/static/service/messagePreprocessor/hybrid)
//! 都实现 `AsyncExtension`, 即 `async fn call` 接口.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 扩展输入
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionInput {
    /// 调用方提供的 JSON 参数
    pub args: Value,
    /// 上下文 (e.g. trace_id, caller_id)
    pub context: Value,
}

impl ExtensionInput {
    /// 简单构造
    pub fn new(args: Value) -> Self {
        Self {
            args,
            context: Value::Null,
        }
    }

    /// 完整构造
    pub fn with_context(args: Value, context: Value) -> Self {
        Self { args, context }
    }

    /// 估算输入字节数
    pub fn byte_size(&self) -> usize {
        serde_json::to_string(self).map(|s| s.len()).unwrap_or(0)
    }
}

/// 扩展输出
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionOutput {
    /// 输出 JSON
    pub result: Value,
    /// 是否成功
    pub success: bool,
    /// 错误信息 (成功时 None)
    pub error: Option<String>,
}

impl ExtensionOutput {
    /// 成功
    pub fn ok(result: Value) -> Self {
        Self {
            result,
            success: true,
            error: None,
        }
    }

    /// 失败
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            result: Value::Null,
            success: false,
            error: Some(msg.into()),
        }
    }

    /// 估算输出字节数
    pub fn byte_size(&self) -> usize {
        serde_json::to_string(self).map(|s| s.len()).unwrap_or(0)
    }
}

/// 异步扩展 trait
///
/// 6 类插件 (sync/async/static/service/messagePreprocessor/hybrid) 全部实现此 trait.
/// 实现方保证 `call` 是 `async`, 可在多线程 runtime 中安全调用 (Send + Sync).
#[async_trait]
pub trait AsyncExtension: Send + Sync {
    /// 插件名
    fn name(&self) -> &str;

    /// 插件 kind
    fn kind(&self) -> crate::types::PluginKind;

    /// 关联 manifest
    fn manifest(&self) -> &crate::manifest::Manifest;

    /// 异步执行 (沙盒已检查通过后)
    async fn call(&self, input: ExtensionInput) -> crate::error::Result<ExtensionOutput>;
}

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_byte_size_non_zero() {
        let i = ExtensionInput::new(serde_json::json!({"a": 1}));
        assert!(i.byte_size() > 0);
    }

    #[test]
    fn output_ok_size() {
        let o = ExtensionOutput::ok(serde_json::json!({"x": "y"}));
        assert!(o.byte_size() > 0);
        assert!(o.success);
        assert!(o.error.is_none());
    }

    #[test]
    fn output_err_size() {
        let o = ExtensionOutput::err("boom");
        assert!(!o.success);
        assert_eq!(o.error.as_deref(), Some("boom"));
    }

    #[test]
    fn input_with_context() {
        let i = ExtensionInput::with_context(
            serde_json::json!({"a": 1}),
            serde_json::json!({"trace_id": "abc"}),
        );
        assert_eq!(i.context["trace_id"], "abc");
    }
}
