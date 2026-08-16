//! R112: MCP handler call metrics (atomic-based, 0 改现有 handler)
//!
//! **目标**: 每个 MCP handler call 都自动 record 一次 metrics (counter + duration),
//! 给 TUI dashboard / health check / debug 用.
//!
//! **Apeireth 真接 (本 module)**:
//! - `McpMetrics` struct — 持有 4 类 atomic counter (per dispatch kind + per method)
//! - `record_dispatch(method, kind, success, duration_ms)` — 每次 dispatch 自动调
//! - `snapshot() -> McpMetricsSnapshot` — 拿当前 snapshot (给 reporting 用)
//! - `McpMetrics::disabled()` / `default()` — 0 cost 默认
//! - `McpMetricsSnapshot` — 序列化 friendly, 含 per_method / per_kind 统计
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `lib.rs` `McpServer::dispatch` / `McpClient::request` 已有调用链 (本 module 是旁路)
//! - 0 改 `tools.rs` / `resources.rs` / `prompts.rs` / `subscriptions.rs` / `tool_subscriptions.rs` 已有 handler
//! - 0 引入 apeireth-telemetry 重 deps (atomic 即可; 留 migration path)
//!
//! **借鉴锚 (S-10)**:
//! - Prometheus counter pattern (monotonic, label-based)
//! - OpenTelemetry `Meter` (per-operation histogram + counter)
//! - VCP vcptoolbox call stats (per-tool latency / success / fail)

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

// ============================================================
// DispatchKind
// ============================================================

/// **Dispatch kind** (per MCP method, 5 类)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DispatchKind {
    /// initialize (per lib.rs handle_initialize)
    Initialize,
    /// tools/list 或 tools/call
    Tools,
    /// resources/list 或 resources/read
    Resources,
    /// prompts/list 或 prompts/get
    Prompts,
    /// 其他 / 未知
    Other,
}

impl DispatchKind {
    /// **从 method 字符串推 dispatch kind**
    pub fn from_method(method: &str) -> Self {
        if method == "initialize" {
            Self::Initialize
        } else if method.starts_with("tools/") {
            Self::Tools
        } else if method.starts_with("resources/") {
            Self::Resources
        } else if method.starts_with("prompts/") {
            Self::Prompts
        } else {
            Self::Other
        }
    }

    /// **kind_str** (per kind)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Tools => "tools",
            Self::Resources => "resources",
            Self::Prompts => "prompts",
            Self::Other => "other",
        }
    }
}

// ============================================================
// PerKindCounters
// ============================================================

/// **Per-kind counters** (4 atomic)
#[derive(Debug, Default)]
struct PerKindCounters {
    total: AtomicU64,
    success: AtomicU64,
    error: AtomicU64,
    /// 累计 duration (ms)
    total_duration_ms: AtomicU64,
}

impl PerKindCounters {
    fn record(&self, success: bool, duration: Duration) {
        self.total.fetch_add(1, Ordering::Relaxed);
        if success {
            self.success.fetch_add(1, Ordering::Relaxed);
        } else {
            self.error.fetch_add(1, Ordering::Relaxed);
        }
        self.total_duration_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }
}

/// **Per-kind snapshot** (4 字段 read)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerKindSnapshot {
    pub kind: String,
    pub total: u64,
    pub success: u64,
    pub error: u64,
    pub total_duration_ms: u64,
    /// 平均 latency (ms) — 0 if total=0
    pub avg_duration_ms: u64,
}

impl PerKindSnapshot {
    fn from_atomic(kind: DispatchKind, c: &PerKindCounters) -> Self {
        let total = c.total.load(Ordering::Relaxed);
        let total_duration_ms = c.total_duration_ms.load(Ordering::Relaxed);
        let avg_duration_ms = if total > 0 {
            total_duration_ms / total
        } else {
            0
        };
        Self {
            kind: kind.as_str().to_string(),
            total,
            success: c.success.load(Ordering::Relaxed),
            error: c.error.load(Ordering::Relaxed),
            total_duration_ms,
            avg_duration_ms,
        }
    }
}

// ============================================================
// McpMetrics
// ============================================================

/// **MCP handler call metrics** (atomic-based, lock-free)
///
/// 用法 (in McpServer::dispatch):
/// ```ignore
/// let start = std::time::Instant::now();
/// let kind = DispatchKind::from_method(&req.method);
/// let resp = /* ... existing dispatch ... */;
/// let success = resp.error.is_none();
/// let metrics = McpMetrics::global();
/// metrics.record_dispatch(&req.method, kind, success, start.elapsed());
/// ```
#[derive(Debug, Default)]
pub struct McpMetrics {
    enabled: std::sync::atomic::AtomicBool,
    /// Per-kind aggregate
    by_kind: [PerKindCounters; 5], // index by DispatchKind as u8
    /// Per-method aggregate (BTreeMap for deterministic snapshot order)
    by_method: std::sync::Mutex<BTreeMap<String, PerKindCounters>>,
    /// Created at (for uptime calc)
    created_at_ms: AtomicU64,
}

impl McpMetrics {
    /// **构造默认 enabled** (但 0 cost until record is called)
    pub fn new() -> Self {
        Self {
            enabled: std::sync::atomic::AtomicBool::new(true),
            by_kind: Default::default(),
            by_method: std::sync::Mutex::new(BTreeMap::new()),
            created_at_ms: AtomicU64::new(now_ms()),
        }
    }

    /// **disabled 模式** — 所有 record 都是 no-op
    pub fn disabled() -> Self {
        let m = Self::new();
        m.enabled.store(false, Ordering::Relaxed);
        m
    }

    /// **是否启用**
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// **enable / disable** (runtime toggle)
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// **record 一次 dispatch** (per method call)
    pub fn record_dispatch(
        &self,
        method: &str,
        kind: DispatchKind,
        success: bool,
        duration: Duration,
    ) {
        if !self.is_enabled() {
            return;
        }
        let kind_idx = kind as u8 as usize;
        if kind_idx < self.by_kind.len() {
            self.by_kind[kind_idx].record(success, duration);
        }
        // Per-method (BTreeMap guarded by std Mutex, BTreeMap 写慢但 OK for low-frequency)
        let mut map = self
            .by_method
            .lock()
            .expect("apeireth-mcp telemetry by_method mutex poisoned");
        map.entry(method.to_string())
            .or_insert_with(PerKindCounters::default)
            .record(success, duration);
    }

    /// **拿 snapshot** (per-kind + per-method)
    pub fn snapshot(&self) -> McpMetricsSnapshot {
        let kinds = vec![
            PerKindSnapshot::from_atomic(
                DispatchKind::Initialize,
                &self.by_kind[DispatchKind::Initialize as u8 as usize],
            ),
            PerKindSnapshot::from_atomic(
                DispatchKind::Tools,
                &self.by_kind[DispatchKind::Tools as u8 as usize],
            ),
            PerKindSnapshot::from_atomic(
                DispatchKind::Resources,
                &self.by_kind[DispatchKind::Resources as u8 as usize],
            ),
            PerKindSnapshot::from_atomic(
                DispatchKind::Prompts,
                &self.by_kind[DispatchKind::Prompts as u8 as usize],
            ),
            PerKindSnapshot::from_atomic(
                DispatchKind::Other,
                &self.by_kind[DispatchKind::Other as u8 as usize],
            ),
        ];

        let methods_map = self
            .by_method
            .lock()
            .expect("apeireth-mcp telemetry by_method mutex poisoned");
        let mut methods: Vec<PerMethodSnapshot> = methods_map
            .iter()
            .map(|(name, c)| {
                let total = c.total.load(Ordering::Relaxed);
                let total_duration_ms = c.total_duration_ms.load(Ordering::Relaxed);
                let avg_duration_ms = if total > 0 {
                    total_duration_ms / total
                } else {
                    0
                };
                PerMethodSnapshot {
                    method: name.clone(),
                    total,
                    success: c.success.load(Ordering::Relaxed),
                    error: c.error.load(Ordering::Relaxed),
                    total_duration_ms,
                    avg_duration_ms,
                }
            })
            .collect();
        methods.sort_by(|a, b| a.method.cmp(&b.method));

        let total_dispatches: u64 = kinds.iter().map(|k| k.total).sum();
        let total_success: u64 = kinds.iter().map(|k| k.success).sum();
        let total_error: u64 = kinds.iter().map(|k| k.error).sum();
        let total_duration_ms: u64 = kinds.iter().map(|k| k.total_duration_ms).sum();

        McpMetricsSnapshot {
            enabled: self.is_enabled(),
            created_at_ms: self.created_at_ms.load(Ordering::Relaxed),
            total_dispatches,
            total_success,
            total_error,
            total_duration_ms,
            avg_duration_ms: if total_dispatches > 0 {
                total_duration_ms / total_dispatches
            } else {
                0
            },
            by_kind: kinds,
            by_method: methods,
        }
    }

    /// **重置所有 counters** (测试用, 0 改 created_at)
    pub fn reset(&self) {
        for c in &self.by_kind {
            c.total.store(0, Ordering::Relaxed);
            c.success.store(0, Ordering::Relaxed);
            c.error.store(0, Ordering::Relaxed);
            c.total_duration_ms.store(0, Ordering::Relaxed);
        }
        self.by_method
            .lock()
            .expect("apeireth-mcp telemetry by_method mutex poisoned")
            .clear();
    }
}

// ============================================================
// Snapshot (serializable)
// ============================================================

/// **Per-method snapshot** (含 method 名)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerMethodSnapshot {
    pub method: String,
    pub total: u64,
    pub success: u64,
    pub error: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: u64,
}

/// **MCP metrics 全量 snapshot** (per kind + per method)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpMetricsSnapshot {
    pub enabled: bool,
    pub created_at_ms: u64,
    pub total_dispatches: u64,
    pub total_success: u64,
    pub total_error: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: u64,
    pub by_kind: Vec<PerKindSnapshot>,
    pub by_method: Vec<PerMethodSnapshot>,
}

impl McpMetricsSnapshot {
    /// **snapshot 转 markdown 报告** (per V2-R strategy 报告格式)
    pub fn to_markdown(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        let _ = writeln!(s, "# MCP Metrics Snapshot");
        let _ = writeln!(s, "");
        let _ = writeln!(s, "- enabled: {}", self.enabled);
        let _ = writeln!(s, "- total_dispatches: {}", self.total_dispatches);
        let _ = writeln!(s, "- total_success: {}", self.total_success);
        let _ = writeln!(s, "- total_error: {}", self.total_error);
        let _ = writeln!(s, "- total_duration_ms: {}", self.total_duration_ms);
        let _ = writeln!(s, "- avg_duration_ms: {}", self.avg_duration_ms);
        let _ = writeln!(s, "");
        let _ = writeln!(s, "## Per kind");
        let _ = writeln!(s, "");
        let _ = writeln!(s, "| Kind | Total | Success | Error | Total ms | Avg ms |");
        let _ = writeln!(s, "|------|-------|---------|-------|----------|--------|");
        for k in &self.by_kind {
            let _ = writeln!(
                s,
                "| {} | {} | {} | {} | {} | {} |",
                k.kind, k.total, k.success, k.error, k.total_duration_ms, k.avg_duration_ms
            );
        }
        let _ = writeln!(s, "");
        let _ = writeln!(s, "## Per method");
        let _ = writeln!(s, "");
        let _ = writeln!(
            s,
            "| Method | Total | Success | Error | Total ms | Avg ms |"
        );
        let _ = writeln!(
            s,
            "|--------|-------|---------|-------|----------|--------|"
        );
        for m in &self.by_method {
            let _ = writeln!(
                s,
                "| {} | {} | {} | {} | {} | {} |",
                m.method, m.total, m.success, m.error, m.total_duration_ms, m.avg_duration_ms
            );
        }
        s
    }
}

// ============================================================
// 工具
// ============================================================

/// **now epoch millis** (避 chrono 依赖)
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_kind_from_method() {
        assert_eq!(
            DispatchKind::from_method("initialize"),
            DispatchKind::Initialize
        );
        assert_eq!(DispatchKind::from_method("tools/list"), DispatchKind::Tools);
        assert_eq!(DispatchKind::from_method("tools/call"), DispatchKind::Tools);
        assert_eq!(
            DispatchKind::from_method("resources/list"),
            DispatchKind::Resources
        );
        assert_eq!(
            DispatchKind::from_method("resources/read"),
            DispatchKind::Resources
        );
        assert_eq!(
            DispatchKind::from_method("prompts/list"),
            DispatchKind::Prompts
        );
        assert_eq!(
            DispatchKind::from_method("prompts/get"),
            DispatchKind::Prompts
        );
        assert_eq!(DispatchKind::from_method("foo/bar"), DispatchKind::Other);
    }

    #[test]
    fn dispatch_kind_as_str() {
        assert_eq!(DispatchKind::Initialize.as_str(), "initialize");
        assert_eq!(DispatchKind::Tools.as_str(), "tools");
        assert_eq!(DispatchKind::Resources.as_str(), "resources");
        assert_eq!(DispatchKind::Prompts.as_str(), "prompts");
        assert_eq!(DispatchKind::Other.as_str(), "other");
    }

    #[test]
    fn metrics_new_is_enabled() {
        let m = McpMetrics::new();
        assert!(m.is_enabled());
    }

    #[test]
    fn metrics_disabled_mode() {
        let m = McpMetrics::disabled();
        assert!(!m.is_enabled());
        m.record_dispatch(
            "tools/list",
            DispatchKind::Tools,
            true,
            Duration::from_millis(10),
        );
        let snap = m.snapshot();
        assert_eq!(snap.total_dispatches, 0);
    }

    #[test]
    fn record_dispatch_increments_counters() {
        let m = McpMetrics::new();
        m.record_dispatch(
            "tools/list",
            DispatchKind::Tools,
            true,
            Duration::from_millis(10),
        );
        m.record_dispatch(
            "tools/call",
            DispatchKind::Tools,
            false,
            Duration::from_millis(20),
        );
        let snap = m.snapshot();
        assert_eq!(snap.total_dispatches, 2);
        assert_eq!(snap.total_success, 1);
        assert_eq!(snap.total_error, 1);
        assert_eq!(snap.total_duration_ms, 30);
    }

    #[test]
    fn record_per_kind_aggregation() {
        let m = McpMetrics::new();
        m.record_dispatch(
            "initialize",
            DispatchKind::Initialize,
            true,
            Duration::from_millis(5),
        );
        m.record_dispatch(
            "tools/list",
            DispatchKind::Tools,
            true,
            Duration::from_millis(10),
        );
        m.record_dispatch(
            "resources/list",
            DispatchKind::Resources,
            true,
            Duration::from_millis(15),
        );
        m.record_dispatch(
            "prompts/list",
            DispatchKind::Prompts,
            true,
            Duration::from_millis(20),
        );
        let snap = m.snapshot();
        let initialize = snap
            .by_kind
            .iter()
            .find(|k| k.kind == "initialize")
            .unwrap();
        let tools = snap.by_kind.iter().find(|k| k.kind == "tools").unwrap();
        let resources = snap.by_kind.iter().find(|k| k.kind == "resources").unwrap();
        let prompts = snap.by_kind.iter().find(|k| k.kind == "prompts").unwrap();
        assert_eq!(initialize.total, 1);
        assert_eq!(tools.total, 1);
        assert_eq!(resources.total, 1);
        assert_eq!(prompts.total, 1);
    }

    #[test]
    fn record_per_method_aggregation() {
        let m = McpMetrics::new();
        m.record_dispatch(
            "tools/list",
            DispatchKind::Tools,
            true,
            Duration::from_millis(10),
        );
        m.record_dispatch(
            "tools/list",
            DispatchKind::Tools,
            true,
            Duration::from_millis(20),
        );
        m.record_dispatch(
            "tools/call",
            DispatchKind::Tools,
            false,
            Duration::from_millis(30),
        );
        let snap = m.snapshot();
        let tools_list = snap
            .by_method
            .iter()
            .find(|m| m.method == "tools/list")
            .unwrap();
        let tools_call = snap
            .by_method
            .iter()
            .find(|m| m.method == "tools/call")
            .unwrap();
        assert_eq!(tools_list.total, 2);
        assert_eq!(tools_list.success, 2);
        assert_eq!(tools_call.total, 1);
        assert_eq!(tools_call.error, 1);
    }

    #[test]
    fn record_avg_duration_calculation() {
        let m = McpMetrics::new();
        m.record_dispatch("x", DispatchKind::Other, true, Duration::from_millis(10));
        m.record_dispatch("x", DispatchKind::Other, true, Duration::from_millis(20));
        m.record_dispatch("x", DispatchKind::Other, true, Duration::from_millis(30));
        let snap = m.snapshot();
        let x = snap.by_method.iter().find(|m| m.method == "x").unwrap();
        assert_eq!(x.total_duration_ms, 60);
        assert_eq!(x.avg_duration_ms, 20); // 60/3
        assert_eq!(snap.avg_duration_ms, 20);
    }

    #[test]
    fn record_set_enabled_runtime_toggle() {
        let m = McpMetrics::new();
        m.set_enabled(false);
        m.record_dispatch("x", DispatchKind::Other, true, Duration::from_millis(10));
        let snap = m.snapshot();
        assert_eq!(snap.total_dispatches, 0);
        m.set_enabled(true);
        m.record_dispatch("x", DispatchKind::Other, true, Duration::from_millis(10));
        let snap = m.snapshot();
        assert_eq!(snap.total_dispatches, 1);
    }

    #[test]
    fn snapshot_empty() {
        let m = McpMetrics::new();
        let snap = m.snapshot();
        assert_eq!(snap.total_dispatches, 0);
        assert_eq!(snap.total_success, 0);
        assert_eq!(snap.total_error, 0);
        assert_eq!(snap.avg_duration_ms, 0);
        assert_eq!(snap.by_kind.len(), 5);
        assert!(snap.by_method.is_empty());
    }

    #[test]
    fn reset_clears_counters() {
        let m = McpMetrics::new();
        m.record_dispatch("x", DispatchKind::Other, true, Duration::from_millis(10));
        m.reset();
        let snap = m.snapshot();
        assert_eq!(snap.total_dispatches, 0);
        assert!(snap.by_method.is_empty());
    }

    #[test]
    fn snapshot_to_markdown_basic() {
        let m = McpMetrics::new();
        m.record_dispatch(
            "tools/list",
            DispatchKind::Tools,
            true,
            Duration::from_millis(10),
        );
        let snap = m.snapshot();
        let md = snap.to_markdown();
        assert!(md.contains("# MCP Metrics Snapshot"));
        assert!(md.contains("total_dispatches: 1"));
        assert!(md.contains("Per kind"));
        assert!(md.contains("Per method"));
        assert!(md.contains("tools/list"));
    }

    #[test]
    fn snapshot_to_markdown_empty() {
        let m = McpMetrics::new();
        let md = m.snapshot().to_markdown();
        assert!(md.contains("total_dispatches: 0"));
        // 5 kind rows even with 0
        assert!(md.contains("| initialize |"));
        assert!(md.contains("| tools |"));
        assert!(md.contains("| resources |"));
        assert!(md.contains("| prompts |"));
        assert!(md.contains("| other |"));
    }
}
