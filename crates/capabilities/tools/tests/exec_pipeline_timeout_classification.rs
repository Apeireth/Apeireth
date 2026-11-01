//! Timeout classification in the five-stage pipeline.
//!
//! P3-5: the around stage used to report *every* timeout as retryable, even
//! when the call had already been entered — so a caller that retried could
//! repeat a side effect that landed after the deadline (git_write's
//! `spawn_blocking`, an MCP `tools/call`).
//!
//! Covered here:
//! * a timeout that fired **before** the tool was entered stays retryable
//!   (nothing happened ⇒ a retry is safe);
//! * a timeout that fired **after** the tool was entered is permanent
//!   (the side effect may have landed ⇒ a retry would repeat it);
//! * the timeout branch actively cancels the in-flight call through the
//!   per-call cancellation signal, so an already-delivered child process or
//!   call can be aborted instead of landing after the timeout.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use apeireth_core::kernel::{AgentId, CapabilityId, PluginId};
use apeireth_plugin::task_scope::ModuleTaskScope;
use apeireth_plugin::{
    FrozenInvocation, ToolAuthorization, ToolCaller, ToolCancellation, ToolCapability,
    ToolExecutionContext,
};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use apeireth_tools_canonical::{
    AroundPolicy, PipelineFailure, PipelinedCapability, ToolExecutionPipeline,
};

fn call() -> ToolCall {
    ToolCall {
        id: "call_timeout".into(),
        name: "demo".into(),
        arguments: serde_json::json!({}),
    }
}

/// A tool that answers only after `delay`.
struct SlowTool {
    id: CapabilityId,
    delay: Duration,
    entered: Arc<AtomicBool>,
}

impl SlowTool {
    fn new(delay: Duration) -> Self {
        Self {
            id: CapabilityId::new("tool.slow").expect("valid capability id"),
            delay,
            entered: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait::async_trait]
impl ToolCapability for SlowTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "slow".into(),
            description: Some("timeout classification stub".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.entered.store(true, Ordering::SeqCst);
        tokio::time::sleep(self.delay).await;
        ToolResult::ok(call.id.clone(), serde_json::json!({ "done": true }))
    }
}

/// A tool that never finishes on its own and watches the cancellation signal
/// it was handed.
struct HangingTool {
    id: CapabilityId,
    entered: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

impl HangingTool {
    fn new() -> Self {
        Self {
            id: CapabilityId::new("tool.hanging").expect("valid capability id"),
            entered: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait::async_trait]
impl ToolCapability for HangingTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool {
            name: "hanging".into(),
            description: Some("timeout cancellation stub".into()),
            parameters: ToolParameters::new(),
            strict: false,
        }
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.entered.store(true, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_secs(30)).await;
        ToolResult::ok(call.id.clone(), serde_json::json!({}))
    }

    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        _frozen: Option<&FrozenInvocation>,
        context: &ToolExecutionContext,
    ) -> ToolResult {
        self.entered.store(true, Ordering::SeqCst);
        let cancellation = context.cancellation.clone();
        let flag = Arc::clone(&self.cancelled);
        // 观察者: 在途 future 被丢弃后仍要能观测到取消 —— 这正是「超时分支主动
        // 取消在途调用」的可证事实。
        tokio::spawn(async move {
            loop {
                if cancellation.is_cancelled() {
                    flag.store(true, Ordering::SeqCst);
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
        tokio::time::sleep(Duration::from_secs(30)).await;
        ToolResult::ok(call.id.clone(), serde_json::json!({}))
    }
}

/// The dispatching turn's scope, as the dispatcher builds it.
fn execution_context(call_id: &str) -> ToolExecutionContext {
    ToolExecutionContext {
        agent: AgentId::default(),
        principal: None,
        session: None,
        turn: None,
        call_id: call_id.to_string(),
        caller: ToolCaller::Host,
        module: PluginId::new("test.pipeline").expect("valid plugin id"),
        module_instance: None,
        internal_review: None,
        message_support: false,
        message_route: None,
        delegated_tools: None,
        delegation: None,
        tool_scope: None,
        authorization: ToolAuthorization::default(),
        cancellation: ToolCancellation::default(),
        deadline: Instant::now() + Duration::from_secs(30),
        trace: None,
        module_tasks: ModuleTaskScope::default(),
        resource_lease: None,
    }
}

#[tokio::test]
async fn timeout_before_the_call_started_stays_retryable() {
    // 取值被拒 ⇒ 定时器根本没武装 ⇒ 工具一次都没被进入 ⇒ 确实什么都没发生。
    let tool = SlowTool::new(Duration::from_millis(500));
    let pipeline =
        ToolExecutionPipeline::new().with_around(AroundPolicy::new().with_timeout(0, 5_000));
    let executed = pipeline.run(&tool, &call()).await;

    assert_eq!(
        executed.failure,
        Some(PipelineFailure::Timeout {
            code: "timeout.invalid_timeout",
            started: false,
        }),
        "未开始的超时应带 `started: false`"
    );
    assert!(
        executed.result.outcome.is_retryable(),
        "未开始 ⇒ 确实什么都没发生 ⇒ 可重试: {}",
        executed.result.render()
    );
    assert!(
        !tool.entered.load(Ordering::SeqCst),
        "未开始的超时不得真的调用工具"
    );
}

#[tokio::test]
async fn timeout_after_the_call_started_is_permanent() {
    let tool = SlowTool::new(Duration::from_millis(500));
    let pipeline =
        ToolExecutionPipeline::new().with_around(AroundPolicy::new().with_timeout(40, 5_000));
    let executed = pipeline.run(&tool, &call()).await;

    assert_eq!(
        executed.failure,
        Some(PipelineFailure::Timeout {
            code: "timeout.deadline_expired",
            started: true,
        }),
        "已开始的超时应带 `started: true`"
    );
    assert!(
        tool.entered.load(Ordering::SeqCst),
        "已开始的超时 ⇒ 工具确实被进入过"
    );
    assert!(
        !executed.result.outcome.is_retryable(),
        "已开始 ⇒ 副作用可能已落地 ⇒ 重试会重复 ⇒ 永久错误: {}",
        executed.result.render()
    );
}

#[tokio::test]
async fn timeout_cancels_the_in_flight_call() {
    let tool = HangingTool::new();
    let entered = Arc::clone(&tool.entered);
    let cancelled = Arc::clone(&tool.cancelled);
    let pipeline =
        ToolExecutionPipeline::new().with_around(AroundPolicy::new().with_timeout(40, 5_000));
    let wrapped = PipelinedCapability::new(Arc::new(tool), Arc::new(pipeline));
    let context = execution_context("call_timeout");

    let result = wrapped.invoke_with_context(&call(), None, &context).await;

    assert!(!result.is_ok(), "超时必须出帧: {}", result.render());
    assert!(
        !result.outcome.is_retryable(),
        "已开始的超时 ⇒ 永久错误: {}",
        result.render()
    );

    // 观察者任务在在途 future 被丢弃后仍要看到取消信号。
    for _ in 0..200 {
        if cancelled.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(entered.load(Ordering::SeqCst), "工具确实被进入过");
    assert!(
        cancelled.load(Ordering::SeqCst),
        "超时分支必须主动取消在途调用 (取消信号要真的被取消)"
    );
}

#[test]
fn timeout_frames_close_into_the_right_retryability() {
    // 未开始 ⇒ 可重试; 已开始 ⇒ 永久。口径同 `canonical/execute.rs` 的
    // `NotStarted` / `WindDownExpired`。
    let not_started = PipelineFailure::Timeout {
        code: "timeout.deadline_expired",
        started: false,
    };
    let started = PipelineFailure::Timeout {
        code: "timeout.deadline_expired",
        started: true,
    };

    assert!(not_started
        .emit("call_x", Some("demo"))
        .outcome
        .is_retryable());
    assert!(!started.emit("call_x", Some("demo")).outcome.is_retryable());
    assert_eq!(not_started.code(), started.code());
}
