use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;


#[derive(Default)]
pub struct RuntimeMetrics {
    pub chat_turns_total: AtomicU64,
    pub tool_executions_total: AtomicU64,
    pub total_latency_ms: AtomicU64,
    pub token_usage_total: AtomicU64,
}

pub struct Telemetry {
    metrics: Arc<RuntimeMetrics>,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self::new()
    }
}

impl Telemetry {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RuntimeMetrics::default()),
        }
    }

    pub fn record_chat_turn(&self, latency_ms: u64, tokens: u32) {
        self.metrics.chat_turns_total.fetch_add(1, Ordering::Relaxed);
        self.metrics.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
        self.metrics.token_usage_total.fetch_add(tokens as u64, Ordering::Relaxed);
        tracing::info!(
            latency_ms = latency_ms,
            tokens = tokens,
            "Runtime telemetry: Chat turn completed"
        );
    }

    pub fn record_tool_execution(&self, tool_name: &str, success: bool, latency_ms: u64) {
        self.metrics.tool_executions_total.fetch_add(1, Ordering::Relaxed);
        tracing::info!(
            tool = tool_name,
            success = success,
            latency_ms = latency_ms,
            "Runtime telemetry: Tool executed"
        );
    }

    pub fn record_latency(ms: u64) {
        tracing::debug!(latency_ms = ms, "Latency sample recorded");
    }

    pub fn record_span(name: &str) {
        tracing::trace!(span = name, "Span recorded");
    }

    pub fn metrics_snapshot(&self) -> (u64, u64, u64, u64) {
        (
            self.metrics.chat_turns_total.load(Ordering::Relaxed),
            self.metrics.tool_executions_total.load(Ordering::Relaxed),
            self.metrics.total_latency_ms.load(Ordering::Relaxed),
            self.metrics.token_usage_total.load(Ordering::Relaxed),
        )
    }
}
