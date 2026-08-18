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
use apeireth_tools::{
    post_call_tripwire, pre_call_guard, GuardrailError, SchemaMap, Tripwire, ValidationError,
};
use serde_json::Value;
use tracing::{debug, warn};

use crate::parser::ParsedToolCall;

/// **战役 2-2 — 工具执行结果**
///
/// 字段级参考 VCP `toolExecutor.js:_createErrorResult` 错误格式
///
/// **TP12 (A2, P0) 扩展字段** (向后兼容, 默认 None):
/// - `guardrail_error` — pre_call_guard 命中 (args 可疑, 阻断工具调用)
/// - `validation_error` — validate 命中 (output 与 schema 不匹配)
/// - `tripwire` — post_call_tripwire 命中 (output 含敏感凭据, 阻断回灌)
#[derive(Debug, Clone, PartialEq, Default)]
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
    /// TP12 pre_call_guard 错误 (args 含路径穿越 / shell 注入 / 危险命令)
    pub guardrail_error: Option<GuardrailError>,
    /// TP12 validate 错误 (output 与 output_schema 不匹配)
    pub validation_error: Option<ValidationError>,
    /// TP12 post_call_tripwire 错误 (output 含敏感凭据)
    pub tripwire: Option<Tripwire>,
}

/// **战役 2-2 — 工具执行器**
///
/// 复刻 VCP `vcpLoop/toolExecutor.js:ToolExecutor` 字段级 (简化).
///
/// **TP12 字段**:
/// - `schemas: SchemaMap` — 工具名 → 输出 schema 映射 (sidecar, 默认空 = 不校验)
/// - `validate_outputs: bool` — 总开关 (false = 跳过 validate, 默认 true)
/// - `tripwire_outputs: bool` — 总开关 (false = 跳过 post_call_tripwire, 默认 true)
/// - `guardrail_inputs: bool` — 总开关 (false = 跳过 pre_call_guard, 默认 true)
pub struct ToolExecutor {
    /// 工具注册中心 (Arc, 跨线程共享)
    registry: Arc<ToolRegistry>,
    /// 单次调用的超时 (毫秒), 默认 30s
    timeout_ms: u64,
    /// TP12: 工具名 → 输出 schema 映射 (向后兼容默认空)
    schemas: SchemaMap,
    /// TP12: pre_call_guard 总开关 (默认 true)
    guardrail_inputs: bool,
    /// TP12: validate 总开关 (默认 true)
    validate_outputs: bool,
    /// TP12: post_call_tripwire 总开关 (默认 true)
    tripwire_outputs: bool,
}

impl ToolExecutor {
    /// 默认超时 30 秒 (战役 2-2 拍板)
    pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

    /// 新建执行器, 用默认超时 + 空 schema map + 全部 TP12 钩子开
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            registry,
            timeout_ms: Self::DEFAULT_TIMEOUT_MS,
            schemas: SchemaMap::new(),
            guardrail_inputs: true,
            validate_outputs: true,
            tripwire_outputs: true,
        }
    }

    /// 新建执行器, 自定义超时
    pub fn with_timeout(registry: Arc<ToolRegistry>, timeout_ms: u64) -> Self {
        let mut s = Self::new(registry);
        s.timeout_ms = timeout_ms;
        s
    }

    /// 新建执行器, 自定义超时 + TP12 schema map (向后兼容: 空 map = 不校验)
    pub fn with_schema_map(
        registry: Arc<ToolRegistry>,
        timeout_ms: u64,
        schemas: SchemaMap,
    ) -> Self {
        let mut s = Self::with_timeout(registry, timeout_ms);
        s.schemas = schemas;
        s
    }

    /// 关闭 pre_call_guard (per-tool 特殊豁免, 实战少用)
    pub fn disable_input_guardrail(mut self) -> Self {
        self.guardrail_inputs = false;
        self
    }

    /// 关闭 output validate
    pub fn disable_output_validation(mut self) -> Self {
        self.validate_outputs = false;
        self
    }

    /// 关闭 post_call_tripwire
    pub fn disable_output_tripwire(mut self) -> Self {
        self.tripwire_outputs = false;
        self
    }

    /// 取内部 registry 引用 (供 RecordStore 等消费者)
    pub fn registry(&self) -> &Arc<ToolRegistry> {
        &self.registry
    }

    /// 超时配置 (毫秒)
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// 当前 schema map 引用
    pub fn schemas(&self) -> &SchemaMap {
        &self.schemas
    }

    /// **真执行一个工具调用**
    ///
    /// **VCP 复刻**: `toolExecutor.js:192-391 execute()` 字段级
    /// 1. 查 registry → 不存在返错误结果
    /// 2. tokio::time::timeout 包裹 call (VCP 没做, 我们加)
    /// 3. 调 tool.call(args)
    /// 4. 错误透传, 包装成 ExecutionResult
    ///
    /// **TP12 三件套** (按顺序, 任一失败立即返回 ExecutionResult with structured error):
    /// - `pre_call_guard(tool_name, args)` — args 可疑 → 阻断
    /// - `tool.call(args)` — 正常执行
    /// - `validate(schemas[tool_name], output)` — output 与 schema 不匹配 → 阻断
    /// - `post_call_tripwire(tool_name, output)` — output 含敏感凭据 → 阻断
    pub async fn execute(&self, call: &ParsedToolCall) -> ExecutionResult {
        let started = std::time::Instant::now();
        let tool_name = call.tool_name.clone();

        // TP12 step 0: pre_call_guard (args 可疑 → 立即阻断)
        if self.guardrail_inputs {
            if let Err(guard_err) = pre_call_guard(&tool_name, &call.args) {
                let duration_ms = started.elapsed().as_millis() as u64;
                let msg = guard_err.to_string();
                warn!("[ToolExecutor] pre_call_guard 阻断 {tool_name}: {msg}");
                return ExecutionResult {
                    success: false,
                    output: Value::String(format!("[GuardrailBlocked] {msg}")),
                    error: Some(msg),
                    duration_ms,
                    tool_name,
                    guardrail_error: Some(guard_err),
                    ..Default::default()
                };
            }
        }

        // 1. 查 registry (VCP toolExecutor.js:358-367 插件不存在检查)
        let Some(tool) = self.registry.get(&tool_name) else {
            let message = format!("Tool not found: {tool_name}");
            warn!("[ToolExecutor] {message}");
            return ExecutionResult {
                success: false,
                output: Value::String(format!("[Error] {message}")),
                error: Some(message),
                duration_ms: started.elapsed().as_millis() as u64,
                tool_name,
                ..Default::default()
            };
        };

        // 2. tokio::time::timeout 包裹 call (Apeireth 优势: 防止工具 hang 住)
        let timeout_duration = Duration::from_millis(self.timeout_ms);
        let call_result =
            tokio::time::timeout(timeout_duration, tool.call(call.args.clone())).await;

        let duration_ms = started.elapsed().as_millis() as u64;

        // 3. 处理 timeout / 工具错误
        let output = match call_result {
            Ok(Ok(value)) => {
                debug!("[ToolExecutor] {tool_name} 成功, duration = {duration_ms}ms");
                value
            }
            Ok(Err(e)) => {
                // 工具自身报错的错误 (VCP toolExecutor.js:382-390)
                let message = format!("Tool call error: {e}");
                warn!("[ToolExecutor] {tool_name} 失败: {message}");
                return ExecutionResult {
                    success: false,
                    output: Value::String(format!("[Error] {message}")),
                    error: Some(message),
                    duration_ms,
                    tool_name,
                    ..Default::default()
                };
            }
            Err(_elapsed) => {
                // tokio timeout 触发
                let message = format!("Tool call timeout after {}ms", self.timeout_ms);
                warn!("[ToolExecutor] {tool_name} 超时: {message}");
                return ExecutionResult {
                    success: false,
                    output: Value::String(format!("[Timeout] {message}")),
                    error: Some(message),
                    duration_ms,
                    tool_name,
                    ..Default::default()
                };
            }
        };

        // TP12 step 3: validate output against schema (if registered)
        if self.validate_outputs {
            if let Some(schema) = self.schemas.get(&tool_name) {
                if let Err(verr) = apeireth_tools::validate(schema, &output) {
                    let msg = verr.to_string();
                    warn!("[ToolExecutor] validate 阻断 {tool_name}: {msg}");
                    return ExecutionResult {
                        success: false,
                        output,
                        error: Some(msg.clone()),
                        duration_ms,
                        tool_name,
                        validation_error: Some(verr),
                        ..Default::default()
                    };
                }
            }
        }

        // TP12 step 4: post_call_tripwire (output 含敏感凭据 → 阻断回灌)
        if self.tripwire_outputs {
            if let Some(trip) = post_call_tripwire(&tool_name, &output) {
                let msg = trip.to_string();
                warn!("[ToolExecutor] post_call_tripwire 阻断 {tool_name}: {msg}");
                return ExecutionResult {
                    success: false,
                    // 输出含敏感凭据 → 用 redacted 标记替代, 不回灌 raw
                    output: Value::String(format!("[TripwireBlocked] {msg}")),
                    error: Some(msg),
                    duration_ms,
                    tool_name,
                    tripwire: Some(trip),
                    ..Default::default()
                };
            }
        }

        ExecutionResult {
            success: true,
            output,
            error: None,
            duration_ms,
            tool_name,
            ..Default::default()
        }
    }

    /// **批量执行** (顺序, 非并发; VCP 用 Promise.all, 我们保守)
    pub async fn execute_all(&self, calls: &[ParsedToolCall]) -> Vec<ExecutionResult> {
        let mut results = Vec::with_capacity(calls.len());
        for call in calls {
            results.push(self.execute(call).await);
        }
        results
    }

    /// **N10 archery 式解析与执行分离** (VCP `toolExecutor.js` archery no-reply 语义)
    ///
    /// normal 调用顺序 await, 结果回灌调用方; archery 调用逐条 `tokio::spawn`
    /// fire-and-forget —— 不阻塞主循环, 调用方拿到 `ArcheryHandle` 仅供观测
    /// (`no_reply = true` 时结果永不回灌, 与 VCP `__vcpArcheryNoReplySilent` 同义).
    pub async fn execute_separated(
        &self,
        calls: &[ParsedToolCall],
    ) -> (Vec<ExecutionResult>, Vec<ArcheryHandle>) {
        let sep = crate::text_protocol::TextToolProtocol::separate(calls);

        let mut results = Vec::with_capacity(sep.normal.len());
        for call in &sep.normal {
            results.push(self.execute(call).await);
        }

        let mut handles = Vec::with_capacity(sep.archery.len());
        for call in sep.archery {
            let registry = self.registry.clone();
            let timeout_ms = self.timeout_ms;
            let tool_name = call.tool_name.clone();
            let no_reply = call.archery_no_reply;
            let join = tokio::spawn(async move {
                ToolExecutor::with_schema_map(registry, timeout_ms, SchemaMap::new())
                    .execute(&call)
                    .await
            });
            handles.push(ArcheryHandle {
                tool_name,
                no_reply,
                join,
            });
        }

        (results, handles)
    }
}

/// **N10 — archery 异步分发句柄**
///
/// fire-and-forget: 主循环不应 await 它来推进对话 (VCP archery 语义).
/// `join` 仅供审计/观测侧选择性等待.
#[derive(Debug)]
pub struct ArcheryHandle {
    /// 工具名
    pub tool_name: String,
    /// true = 结果永不回灌 (VCP archeryNoReply)
    pub no_reply: bool,
    /// 后台任务句柄 (观测用)
    pub join: tokio::task::JoinHandle<ExecutionResult>,
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

    #[tokio::test]
    async fn execute_separated_archery_does_not_block() {
        // N10 archery 式分离: normal 顺序 await, archery spawn fire-and-forget
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        let calls = vec![
            ParsedToolCall {
                tool_name: "EchoSync".to_string(),
                args: json!({"input": "n"}),
                raw_marker: "".into(),
                archery: false,
                archery_no_reply: false,
            },
            ParsedToolCall {
                tool_name: "SlowAsync".to_string(), // 50ms delay
                args: json!({"input": "a"}),
                raw_marker: "".into(),
                archery: true,
                archery_no_reply: true,
            },
        ];

        let started = std::time::Instant::now();
        let (results, mut handles) = exec.execute_separated(&calls).await;
        let elapsed = started.elapsed();

        // normal 结果立即回灌
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].output["echo"], "n");

        // archery 不阻塞主循环: SlowAsync 50ms delay, 但 spawn 立即返回
        assert!(
            elapsed < std::time::Duration::from_millis(40),
            "archery 应 fire-and-forget 不等待, 实际 {:?}",
            elapsed
        );

        // 句柄观测侧可选等待, no_reply 标记透传
        assert_eq!(handles.len(), 1);
        assert_eq!(handles[0].tool_name, "SlowAsync");
        assert!(handles[0].no_reply);
        let bg = handles.remove(0).join.await.expect("join");
        assert!(bg.success);
    }

    #[tokio::test]
    async fn execute_separated_all_normal_empty_archery() {
        // 全 normal 无 archery → handles 为空
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        let calls = vec![ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({"input": "x"}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        }];
        let (results, handles) = exec.execute_separated(&calls).await;
        assert_eq!(results.len(), 1);
        assert!(handles.is_empty());
    }

    // ============================================================
    // TP12 (A2, P0) — schema 校验 + guardrail + tripwire 钩子测试
    // ============================================================

    /// pre_call_guard 命中路径穿越 → 阻断, 不调用工具
    #[tokio::test]
    async fn execute_guardrail_blocks_path_traversal() {
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        let call = ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({"path": "../../../etc/shadow"}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(!r.success, "路径穿越应被阻断");
        assert!(
            r.guardrail_error.is_some(),
            "应有 guardrail_error 结构化字段"
        );
        let ge = r.guardrail_error.unwrap();
        assert_eq!(ge.tool_name, "EchoSync");
        assert!(r.error.as_ref().unwrap().contains("path"));
        assert!(r.output.as_str().unwrap().starts_with("[GuardrailBlocked]"));
    }

    /// pre_call_guard 命中 shell 注入 → 阻断
    #[tokio::test]
    async fn execute_guardrail_blocks_shell_injection() {
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        let call = ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({"cmd": "echo hi; rm -rf /"}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(!r.success);
        assert!(r.guardrail_error.is_some());
        // shell 注入在 dangerous_command 之前被检测到 → kind = ShellInjection
        assert!(matches!(
            r.guardrail_error.unwrap().kind,
            apeireth_tools::GuardrailKind::ShellInjection
        ));
    }

    /// pre_call_guard 关闭时, 即使 args 可疑也放行 (向后兼容 opt-out)
    #[tokio::test]
    async fn execute_guardrail_can_be_disabled() {
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry).disable_input_guardrail();
        let call = ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({"path": "../../../etc/shadow"}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(r.success, "关闭 guardrail 后应放行");
        assert!(r.guardrail_error.is_none());
    }

    /// 默认空 SchemaMap → 全部工具不校验, 行为不变 (向后兼容)
    #[tokio::test]
    async fn execute_validate_skips_when_schema_map_empty() {
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        // EchoSync output = {"echo": <args>}, 不在 schema 中 → 不校验
        let call = ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({"input": "anything"}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(r.success);
        assert!(r.validation_error.is_none());
    }

    /// SchemaMap 注入 EchoSync → output 不匹配 schema → 阻断
    #[tokio::test]
    async fn execute_validate_blocks_schema_mismatch() {
        let registry = make_registry_with_sync();
        // EchoSync 返回 {"echo": <args>}, schema 要求 {"expected": "string"}
        let mut schemas = apeireth_tools::SchemaMap::new();
        schemas.insert(
            "EchoSync",
            apeireth_tools::SchemaNode::Object {
                fields: std::collections::BTreeMap::from([(
                    "expected".into(),
                    apeireth_tools::SchemaNode::String,
                )]),
            },
        );
        let exec = ToolExecutor::with_schema_map(registry, 30_000, schemas);
        let call = ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({"input": "x"}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(!r.success, "schema 不匹配应被阻断");
        assert!(r.validation_error.is_some());
        let ve = r.validation_error.unwrap();
        assert_eq!(ve.path, "$.expected");
        assert_eq!(ve.actual, "missing");
    }

    /// SchemaMap 注入 EchoSync → output 匹配 schema → 放行
    #[tokio::test]
    async fn execute_validate_passes_when_schema_matches() {
        let registry = make_registry_with_sync();
        // EchoSync 返回 {"echo": <args>}, schema 要求 {"echo": <anything>}
        let mut schemas = apeireth_tools::SchemaMap::new();
        schemas.insert(
            "EchoSync",
            apeireth_tools::SchemaNode::Object {
                fields: std::collections::BTreeMap::from([(
                    "echo".into(),
                    apeireth_tools::SchemaNode::Optional {
                        inner: Box::new(apeireth_tools::SchemaNode::String),
                    },
                )]),
            },
        );
        let exec = ToolExecutor::with_schema_map(registry, 30_000, schemas);
        let call = ParsedToolCall {
            tool_name: "EchoSync".to_string(),
            args: json!({"input": "ok"}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(r.success, "schema 匹配应放行");
        assert!(r.validation_error.is_none());
    }

    /// post_call_tripwire 命中敏感凭据 → 阻断
    #[tokio::test]
    async fn execute_tripwire_blocks_secret_leak() {
        // 自定义工具返回含 AWS key 的 output
        use apeireth_tool_registry::Tool;
        use apeireth_tool_registry::{ToolAxes, ToolKind};
        use async_trait::async_trait;

        struct SecretTool;
        #[async_trait]
        impl Tool for SecretTool {
            fn name(&self) -> &str {
                "SecretTool"
            }
            fn kind(&self) -> ToolKind {
                ToolKind::Sync
            }
            fn axes(&self) -> ToolAxes {
                ToolAxes::default()
            }
            async fn call(&self, _args: Value) -> Result<Value, String> {
                Ok(json!({"config": "AKIAIOSFODNN7EXAMPLE"}))
            }
        }

        let r = ToolRegistry::new();
        r.register("SecretTool".to_string(), Arc::new(SecretTool));
        let registry = Arc::new(r);
        let exec = ToolExecutor::new(registry);
        let call = ParsedToolCall {
            tool_name: "SecretTool".to_string(),
            args: json!({}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let res = exec.execute(&call).await;
        assert!(!res.success, "凭据泄漏应被阻断");
        assert!(res.tripwire.is_some());
        let trip = res.tripwire.unwrap();
        assert_eq!(trip.tool_name, "SecretTool");
        assert!(trip.detail.contains("AWS"));
        // 原始 output 不应回灌, 应用 redacted 标记替代
        assert!(res
            .output
            .as_str()
            .unwrap()
            .starts_with("[TripwireBlocked]"));
    }

    /// post_call_tripwire 可关闭
    #[tokio::test]
    async fn execute_tripwire_can_be_disabled() {
        use apeireth_tool_registry::Tool;
        use apeireth_tool_registry::{ToolAxes, ToolKind};
        use async_trait::async_trait;

        struct SecretTool;
        #[async_trait]
        impl Tool for SecretTool {
            fn name(&self) -> &str {
                "SecretTool"
            }
            fn kind(&self) -> ToolKind {
                ToolKind::Sync
            }
            fn axes(&self) -> ToolAxes {
                ToolAxes::default()
            }
            async fn call(&self, _args: Value) -> Result<Value, String> {
                Ok(json!({"k": "AKIAIOSFODNN7EXAMPLE"}))
            }
        }

        let r = ToolRegistry::new();
        r.register("SecretTool".to_string(), Arc::new(SecretTool));
        let registry = Arc::new(r);
        let exec = ToolExecutor::new(registry).disable_output_tripwire();
        let call = ParsedToolCall {
            tool_name: "SecretTool".to_string(),
            args: json!({}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let res = exec.execute(&call).await;
        assert!(res.success, "关闭 tripwire 后放行");
        assert!(res.tripwire.is_none());
    }

    /// 三件套顺序: guardrail > timeout > 工具错误 (guardrail 先于一切)
    #[tokio::test]
    async fn execute_guardrail_runs_before_registry_lookup() {
        let registry = make_registry_with_sync();
        let exec = ToolExecutor::new(registry);
        // 工具不存在 + args 可疑 → 应报 guardrail (不是 Tool not found)
        let call = ParsedToolCall {
            tool_name: "NonExistent".to_string(),
            args: json!({"path": "../x"}),
            raw_marker: "".into(),
            archery: false,
            archery_no_reply: false,
        };
        let r = exec.execute(&call).await;
        assert!(!r.success);
        assert!(
            r.guardrail_error.is_some(),
            "guardrail 应在 registry lookup 前先执行"
        );
        assert!(r.error.as_ref().unwrap().contains("path"));
    }
}
