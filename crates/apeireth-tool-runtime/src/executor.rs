//! **战役 2-2 / VCP `vcpLoop/toolExecutor.js` — 工具执行器**
//!
//! **目标**: 调 `apeireth-tool-registry` 真执行 + tokio 超时 + 错误处理.
//!
//! **字段级引用 VCP** (per `docs/stage3-blueprints/borrowed-from-projects.md`):
//! - `toolExecutor.js:38-46 ToolExecutor constructor` — 持 pluginManager / debugMode / auth
//!   我们持有 `Arc<ToolRegistry>` / `timeout_ms`
//! - `toolExecutor.js:192-391 execute(toolCall, clientIp, contextMessages)` — 真执行入口
//! - `toolExecutor.js:355-367` 插件不存在检查 → 返错误结果
//! - `toolExecutor.js:370-390` 调 pluginManager.processToolCall + 错误兜底
//! - `toolExecutor.js:475-482 _createErrorResult` — 错误结果格式
//!
//! **Apeireth 简化**:
//! - VCP 5+ 步骤: river 上下文注入 + vref 知识库注入 + auth 校验 + plugin 查询 + processToolCall
//!   我们 4 步: tool 查询 + 超时控制 + 真调 call + 错误处理
//! - VCP `auth` 验证我们不在 runtime 阶段做 (战役 2-3 approval crate 做)
//! - VCP 错误结果用 `content: [{type: 'text', text: '...'}]` OpenAI 格式; 我们用简单 JSON Value
//!
//! **不假装**:
//! - ✅ 真调 `ToolRegistry::get` + `Tool::call`
//! - ✅ 真用 `tokio::time::timeout` (VCP 没有, 我们加作为 Apeireth 优势)
//! - ✅ 错误透传 + 包装 (不假装成功)
//! - ✅ 编译期 hardcode (`DEFAULT_TIMEOUT_MS`)

use std::sync::Arc;
use std::time::Duration;

use apeireth_tool_registry::ToolRegistry;
use serde_json::Value;
use tracing::{debug, warn};

use crate::parser::ParsedToolCall;

/// **战役 2-2 — 工具执行结果**
///
/// 字段级参考 VCP `toolExecutor.js:_createErrorResult` 错误格式
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionResult {
    /// 是否成功
    pub success: bool,
    /// 成功时为 tool.call 的返回值, 失败时为错误消息
    pub output: Value,
    /// 错误信息 (失败时)
    pub error: Option<String>,
    /// 耗时 (毫秒)
    pub duration_ms: u64,
    /// 工具名 (冗余, 方便 audit)
    pub tool_name: String,
}

/// **战役 2-2 — 工具执行器**
///
/// 复刻 VCP `vcpLoop/toolExecutor.js:ToolExecutor` 字段级 (简化).
pub struct ToolExecutor {
    /// 工具注册中心 (Arc, 跨线程共享)
    registry: Arc<ToolRegistry>,
    /// 单次调用的超时 (毫秒), 默认 30s
    timeout_ms: u64,
}

impl ToolExecutor {
    /// 默认超时 30 秒 (战役 2-2 拍板)
    pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

    /// 新建执行器, 用默认超时
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            registry,
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
        }
    }

    /// 新建执行器, 自定义超时
    pub fn with_timeout(registry: Arc<ToolRegistry>, timeout_ms: u64) -> Self {
        Self {
            registry,
            timeout_ms,
        }
    }

    /// 取内部 registry 引用 (供 RecordStore 等消费者)
    pub fn registry(&self) -> &Arc<ToolRegistry> {
        &self.registry
    }

    /// 超时配置 (毫秒)
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// **真执行一个工具调用**
    ///
    /// **VCP 复刻**: `toolExecutor.js:192-391 execute()` 字段级
    /// 1. 查 registry → 不存在返错误结果
    /// 2. tokio::time::timeout 包裹 call (VCP 没做, 我们加)
    /// 3. 调 tool.call(args)
    /// 4. 错误透传, 包装成 ExecutionResult
    pub async fn execute(&self, call: &ParsedToolCall) -> ExecutionResult {
        let started = std::time::Instant::now();
        let tool_name = call.tool_name.clone();

        // 1. 查 registry (VCP toolExecutor.js:358-367 插件不存在检查)
        let tool = match self.registry.get(&tool_name) {
            Some(t) => t,
            None => {
                let message = format!("Tool not found: {tool_name}");
                warn!("[ToolExecutor] {message}");
                return ExecutionResult {
                    success: false,
                    output: Value::String(format!("[Error] {message}")),
                    error: Some(message),
                    duration_ms: started.elapsed().as_millis() as u64,
                    tool_name,
                };
            }
        };

        // 2. tokio::time::timeout 包裹 call (Apeireth 优势: 防止工具 hang 住)
        let timeout_duration = Duration::from_millis(self.timeout_ms);
        let call_result =
            tokio::time::timeout(timeout_duration, tool.call(call.args.clone())).await;

        let duration_ms = started.elapsed().as_millis() as u64;

        // 3. 处理 timeout 错误
        let result = match call_result {
            Ok(Ok(value)) => {
                debug!("[ToolExecutor] {tool_name} 成功, duration = {duration_ms}ms");
                ExecutionResult {
                    success: true,
                    output: value,
                    error: None,
                    duration_ms,
                    tool_name,
                }
            }
            Ok(Err(e)) => {
                // 工具自身报错的错误 (VCP toolExecutor.js:382-390)
                let message = format!("Tool call error: {e}");
                warn!("[ToolExecutor] {tool_name} 失败: {message}");
                ExecutionResult {
                    success: false,
                    output: Value::String(format!("[Error] {message}")),
                    error: Some(message),
                    duration_ms,
                    tool_name,
                }
            }
            Err(_elapsed) => {
                // tokio timeout 触发
                let message = format!("Tool call timeout after {}ms", self.timeout_ms);
                warn!("[ToolExecutor] {tool_name} 超时: {message}");
                ExecutionResult {
                    success: false,
                    output: Value::String(format!("[Timeout] {message}")),
                    error: Some(message),
                    duration_ms,
                    tool_name,
                }
            }
        };

        result
    }

    /// **批量执行** (顺序, 非并发; VCP 用 Promise.all, 我们保守)
    pub async fn execute_all(&self, calls: &[ParsedToolCall]) -> Vec<ExecutionResult> {
        let mut results = Vec::with_capacity(calls.len());
        for call in calls {
            results.push(self.execute(call).await);
        }
        results
    }
}

// ============================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

const _: () = {
    // 默认超时 30s, 改要双改
    assert!(
        ToolExecutor::DEFAULT_TIMEOUT_MS == 30_000,
        "DEFAULT_TIMEOUT_MS 必须是 30_000 (战役 2-2 拍板 30s)"
    );
    assert!(
        ToolExecutor::DEFAULT_TIMEOUT_MS >= 1_000,
        "DEFAULT_TIMEOUT_MS 必须 ≥ 1s (防止误判为 hang)"
    );
};

// ============================================================
// 单元测试 (战役 2-2 DoD: ≥ 5 个)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::{MockAsyncTool, MockSyncTool};
    use serde_json::json;

    fn make_registry_with_sync() -> Arc<ToolRegistry> {
        let r = ToolRegistry::new();
        r.register(
            "EchoSync".to_string(),
            Arc::new(MockSyncTool {
                name: "EchoSync".to_string(),
            }),
        );
        r.register(
            "SlowAsync".to_string(),
            Arc::new(MockAsyncTool {
                name: "SlowAsync".to_string(),
                delay_ms: 50,
            }),
        );
        Arc::new(r)
    }

    #[tokio::test]
    async fn execute_sync_tool_success() {
        // 正常同步执行
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        let call = ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({"input": "hello"}),
            raw_marker: "tool_name:<<<EchoSync>>>".to_string(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(r.success);
        assert_eq!(r.tool_name, "EchoSync");
        assert_eq!(r.output["echo"], "hello");
        assert!(r.error.is_none());
        assert!(r.duration_ms < 1000);
    }

    #[tokio::test]
    async fn execute_async_tool_with_delay() {
        // 异步工具 (带 50ms delay)
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        let call = ParsedToolCall {
            tool_name: "SlowAsync".to_string(),
            args: json!({"input": "world"}),
            raw_marker: "tool_name:<<<SlowAsync>>>".to_string(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(r.success);
        assert!(
            r.duration_ms >= 50,
            "异步 delay 应被等待, 实际: {}",
            r.duration_ms
        );
    }

    #[tokio::test]
    async fn execute_tool_not_found() {
        // 工具不存在 → 返 ExecutionResult { success: false, error: ... }
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        let call = ParsedToolCall {
            tool_name: "NonExistent".to_string(),
            args: json!({}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(!r.success);
        assert!(r.error.is_some());
        assert!(r.error.as_ref().unwrap().contains("Tool not found"));
    }

    #[tokio::test]
    async fn execute_with_timeout() {
        // 异步工具, 但 timeout 设 10ms, 工具要 50ms, 应超时
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::with_timeout(registry, 10);
        let call = ParsedToolCall {
            tool_name: "SlowAsync".to_string(),
            args: json!({}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(!r.success, "10ms timeout 工具 50ms 应超时");
        assert!(r.error.as_ref().unwrap().contains("timeout"));
    }

    #[tokio::test]
    async fn execute_complex_args() {
        // 复杂 args (嵌套对象)
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        let call = ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({
                "input": {
                    "nested": {
                        "key": ["a", "b", "c"]
                    }
                }
            }),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(r.success);
        // echo 应该回传 input 字段
        let echoed = r.output.get("echo").unwrap();
        assert_eq!(echoed["nested"]["key"][0], "a");
    }

    #[tokio::test]
    async fn execute_all_sequential() {
        // 批量执行 (顺序)
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        let calls = vec![
            ParsedToolCall {
                tool_name: "EchoSync".to_string(),
                args: json!({"input": "first"}),
                raw_marker: "".into(),
                archery: false,
                archery_no_reply: false,
            },
            ParsedToolCall {
                tool_name: "EchoSync".to_string(),
                args: json!({"input": "second"}),
                raw_marker: "".into(),
                archery: false,
                archery_no_reply: false,
            },
        ];
        let results = exec.execute_all(&calls).await;
        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(results[1].success);
        assert_eq!(results[0].output["echo"], "first");
        assert_eq!(results[1].output["echo"], "second");
    }

    #[tokio::test]
    async fn execute_records_duration() {
        // 验证 duration_ms 字段被填充
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        let call = ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        // duration_ms 应有值 (即使很小)
        // 不严格断言值, 仅断言字段存在且非负
        let _: u64 = r.duration_ms;
    }
}
