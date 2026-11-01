//! The around stage: timeout and retry wrap the execution itself.
//!
//! The wrap order is fixed: the deadline is armed **around** the whole retry
//! sequence, which wraps the execution — one run, one absolute deadline.
//!
//! The timeout runs through the deadline timer library
//! (`apeireth_core::deadline`): expiry only *notifies*; wrapping up the
//! in-flight work is this stage's own job — the deadline branch cancels the
//! attempt's signal and drops the in-flight future (no late side effect).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use apeireth_core::deadline::{clamp_timeout, Deadline, TimeoutError, TimeoutErrorCode};
use apeireth_core::kernel::SessionId;
use apeireth_plugin::{FrozenInvocation, ToolCancellation, ToolCapability};
use apeireth_protocol::canonical::{ToolCall, ToolResult};

use super::PipelineFailure;

/// Default upper bound accepted by the timeout gate (two minutes).
pub const DEFAULT_MAX_TIMEOUT_MS: u64 = 120_000;

/// The deadline library's stable "the deadline expired" code.
const DEADLINE_EXPIRED: &str = TimeoutErrorCode::DeadlineExpired.as_str();

/// How often a failed attempt is retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Total number of attempts, including the first one. `1` means no retry.
    pub max_attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 1 }
    }
}

impl RetryPolicy {
    /// A policy with `max_attempts` total attempts (clamped to at least one).
    pub const fn new(max_attempts: u32) -> Self {
        Self { max_attempts }
    }

    /// Total attempts actually run: never fewer than one.
    pub const fn attempts(self) -> u32 {
        if self.max_attempts == 0 {
            1
        } else {
            self.max_attempts
        }
    }
}

/// The around-stage policy: timeout and retry around the execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AroundPolicy {
    /// Timeout for one run, in milliseconds. `None` runs without a deadline.
    pub timeout_ms: Option<u64>,
    /// Upper bound the timeout gate accepts.
    pub max_timeout_ms: u64,
    /// Retry policy of the same run.
    pub retry: RetryPolicy,
}

impl Default for AroundPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: None,
            max_timeout_ms: DEFAULT_MAX_TIMEOUT_MS,
            retry: RetryPolicy::default(),
        }
    }
}

impl AroundPolicy {
    /// A policy with no timeout and no retry: execution runs once, unwrapped.
    pub fn new() -> Self {
        Self::default()
    }

    /// Run under a deadline of `timeout_ms` milliseconds, refusing any value
    /// the timeout gate rejects (zero, the unbounded sentinel, above the cap).
    #[must_use]
    pub fn with_timeout(mut self, timeout_ms: u64, max_timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self.max_timeout_ms = max_timeout_ms;
        self
    }

    /// Retry failed attempts per `retry`.
    #[must_use]
    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Gate the configured timeout value before it is armed.
    ///
    /// Returns the timeout to arm, or `None` when the policy runs without one.
    /// Zero, the unbounded sentinel, and values above the cap are refused.
    pub fn validated_timeout_ms(&self) -> Result<Option<u64>, TimeoutError> {
        let Some(requested) = self.timeout_ms else {
            return Ok(None);
        };
        // The default is only read for an absent request; here the request is
        // always present, so it is passed through unchanged.
        let timeout = clamp_timeout(Some(requested), requested, self.max_timeout_ms)?;
        Ok(Some(timeout))
    }
}

/// One timeout-family failure: `code` is the deadline library's stable code,
/// `started` says whether the run had already entered the tool call.
fn timeout_failure(code: &'static str, started: bool) -> PipelineFailure {
    PipelineFailure::Timeout { code, started }
}

/// Execute one call through the around stage.
///
/// Returns the final result together with the number of attempts made. The
/// only error the stage itself produces is the closed timeout family: a failed
/// result is a result, not a stage failure. `session` is the dispatching turn's
/// session scope; `None` for host-side invocations without one.
pub(crate) async fn run_around(
    tool: &dyn ToolCapability,
    call: &ToolCall,
    frozen: Option<&FrozenInvocation>,
    session: Option<SessionId>,
    context: Option<&apeireth_plugin::ToolExecutionContext>,
    policy: &AroundPolicy,
) -> Result<(ToolResult, u32), PipelineFailure> {
    // 取值被拒 ⇒ 定时器从未武装 ⇒ 工具一次都没被进入 ⇒ 未开始 ⇒ 可重试。
    let timeout_ms = policy
        .validated_timeout_ms()
        .map_err(|error| timeout_failure(error.code().as_str(), false))?;
    let mut armed: Option<(Deadline, _)> = match timeout_ms {
        Some(milliseconds) => Some(
            Deadline::after(Duration::from_millis(milliseconds))
                .map_err(|error| timeout_failure(error.code().as_str(), false))?,
        ),
        None => None,
    };

    let mut attempts = 0u32;
    // 运行级粘滞位: 任一次尝试进入了工具 ⇒ 本次运行「已开始」(副作用可能已落地)。
    let started = AtomicBool::new(false);
    loop {
        attempts += 1;
        // 每次尝试一份子取消令牌: 承接上层取消; 超时时由本层主动取消在途调用。
        let parents = context.iter().map(|ctx| ctx.cancellation.clone()).collect();
        let cancellation = ToolCancellation::joined(parents);
        // 把子令牌交给工具 (同 shell/execution.rs 的用法): 已投递的子进程据此中止。
        let attempt_context = context.cloned().map(|mut attempt| {
            attempt.cancellation = cancellation.clone();
            attempt
        });
        let invocation = async {
            // 进入工具 ⇒ 副作用可能已经投递 ⇒ 本次运行「已开始」。
            started.store(true, Ordering::SeqCst);
            match &attempt_context {
                Some(context) => tool.invoke_with_context(call, frozen, context).await,
                None => tool.invoke_in_session(call, frozen, session).await,
            }
        };
        let attempt = match armed.as_mut() {
            Some((_deadline, notice)) => tokio::select! {
                _ = notice.notified() => None,
                result = invocation => Some(result),
            },
            None => Some(invocation.await),
        };
        let Some(result) = attempt else {
            // 超时 ⇒ 主动取消在途调用 (已投递的子进程/调用在此中止); 真正无法取消的
            // 路径 (git_write / MCP 的 in-flight call) 仍会落地 ⇒ 按「已开始」处理。
            cancellation.cancel();
            let was_started = started.load(Ordering::SeqCst);
            return Err(timeout_failure(DEADLINE_EXPIRED, was_started));
        };
        if result.is_ok() || attempts >= policy.retry.attempts() {
            return Ok((result, attempts));
        }
    }
}
