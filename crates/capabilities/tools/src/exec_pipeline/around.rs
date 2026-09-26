//! The around stage: timeout and retry wrap the execution itself.
//!
//! The wrap order is fixed and cannot be rearranged: the deadline is armed
//! **around** the whole retry sequence, which wraps the execution. One run
//! therefore has one absolute deadline, however many attempts it makes.
//!
//! The timeout runs through the deadline timer library
//! (`apeireth_core::deadline`): expiry only *notifies*, and wrapping up the
//! in-flight work is this stage's own job — the in-flight future is dropped
//! here, and the armed timer is dropped (and cleaned up) with the stage.

use std::time::Duration;

use apeireth_core::deadline::{clamp_timeout, Deadline, TimeoutError};
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{ToolCall, ToolResult};

use super::PipelineFailure;

/// Default upper bound accepted by the timeout gate (two minutes).
pub const DEFAULT_MAX_TIMEOUT_MS: u64 = 120_000;

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
    /// Zero, the unbounded sentinel, and values above the cap are refused with
    /// the deadline library's own error code.
    pub fn validated_timeout_ms(&self) -> Result<Option<u64>, TimeoutError> {
        match self.timeout_ms {
            None => Ok(None),
            // The default is only read for an absent request; here the request
            // is always present, so it is passed through unchanged.
            Some(requested) => Ok(Some(clamp_timeout(
                Some(requested),
                requested,
                self.max_timeout_ms,
            )?)),
        }
    }
}

/// Execute one call through the around stage.
///
/// Returns the final result together with the number of attempts made. The
/// only error the stage itself produces is the closed timeout family: a failed
/// result is a result, not a stage failure, and travels on to the next stage.
pub(crate) async fn run_around(
    tool: &dyn ToolCapability,
    call: &ToolCall,
    frozen: Option<&FrozenInvocation>,
    policy: &AroundPolicy,
) -> Result<(ToolResult, u32), PipelineFailure> {
    let timeout_ms = policy
        .validated_timeout_ms()
        .map_err(|error| PipelineFailure::Timeout {
            code: error.code().as_str(),
        })?;
    let mut armed: Option<(Deadline, _)> = match timeout_ms {
        Some(milliseconds) => Some(
            Deadline::after(Duration::from_millis(milliseconds)).map_err(|error| {
                PipelineFailure::Timeout {
                    code: error.code().as_str(),
                }
            })?,
        ),
        None => None,
    };

    let mut attempts = 0u32;
    loop {
        attempts += 1;
        let attempt = match armed.as_mut() {
            Some((_deadline, notice)) => tokio::select! {
                _ = notice.notified() => None,
                result = tool.invoke_frozen(call, frozen) => Some(result),
            },
            None => Some(tool.invoke_frozen(call, frozen).await),
        };
        let Some(result) = attempt else {
            return Err(PipelineFailure::Timeout {
                code: apeireth_core::deadline::TimeoutErrorCode::DeadlineExpired.as_str(),
            });
        };
        if result.is_ok() || attempts >= policy.retry.attempts() {
            return Ok((result, attempts));
        }
    }
}
