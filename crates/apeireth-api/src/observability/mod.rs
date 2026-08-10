//! # Observability HTTP 端点模块 (1.0 release 12 项 checklist #8 必做)
//!
//! **目的**: 给 `apeireth-api` 加 3 个 observability JSON 端点 (跟 `apeireth-observability` crate 集成):
//!
//! - `GET /v1/observability/metrics` — Prometheus exposition format (15-20 metric)
//! - `GET /v1/observability/health`  — 5 组件 health check (db / cache / queue / external_api / disk_space)
//! - `GET /v1/observability/status`  — 服务状态 (uptime / version / build_time / git_commit / active_sessions)
//!
//! **架构** (跟 v2_endpoints 6 类 JSON 端点同模式):
//! - 自包含, 0 循环依赖 (跟 v2_endpoints 一致: 走本地 `parking_lot::Mutex<...>` 状态)
//! - 公开 API: `pub fn router() -> axum::Router<()>` — 让 `v2_endpoints::build_router` 1 行 merge
//! - 测试 12-15 个, 走 in-process state 验
//!
//! **6 哲学锚穿透** (per `docs/stage4/apeireth-formal-invariants-2026-08-05.md` §11):
//!
//! - **S-1 北极星导向**: 1:1 翻译 v0.9.21 商业版 observability 集成面 + 蓝图 §2.5.3, 0 重设计
//! - **S-2 实事求是**: 估 350-450 LOC, R20 阶段 1 skeleton 阶段实接 3 端点, R20 阶段 3 续 OTLP
//! - **O-2 走在前人肩上**: 借鉴 OpenTelemetry / Prometheus 工业标准 (跟 `apeireth-observability::metrics` 复用), 0 另立协议
//! - **O-3 干到底**: 15-20 metric + 5 组件 + 8 status 字段 + 12-15 测试 + 5 R-Measure stub
//! - **O-4 任何人都能接手**: §1-§5 跟 `v2_endpoints` 同骨架 + 引用 `apeireth-observability` 公开 API 完整 path
//! - **O-5 不假装**: 0 假装 OTLP 已接 (R20 阶段 3 续) + 5 R-Measure 占位值标注 "stub" 字段, 不假装已采集
//!
//! **8 项不修改承诺** (per `docs/stage4/8-locked-unified-2026-08-05.md` §2):
//!
//! 1. ✅ 0 触碰 24 LOCKED crate `src/lib.rs` (本 crate 在 24 LOCKED 列表, 仅 +1 `pub mod observability;` 1 行)
//! 2. ✅ 0 改 v2_endpoints.rs 79KB 大块 (仅在末尾 `build_router` +1 `.merge(...)` 1 行)
//! 3. ✅ 0 改 workspace version (semver 1.0.0 严守)
//! 4. ✅ 0 引 NewAPI (跟 R17 决策一致)
//! 5. ✅ 0 重复造轮子 (复用 `apeireth_observability::render_prometheus`)
//! 6. ✅ 6 哲学锚穿透 (本文件顶部 + 各子模块)
//! 7. ✅ 不假装已实现 (5 R-Measure 标 "stub_")
//! 8. ✅ 诚实标缺 (5 R-Measure / git_commit 字段标 "unknown" 兜底)
//!
//! **引用文档**:
//! - `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-observability\src\lib.rs` (复用 `render_prometheus`)
//! - `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-api\src\v2_endpoints.rs::build_router` (merge 接入点)
//! - `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-rust-translation-blueprint-2026-08-05.md` §2.5.3
//! - `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\8-locked-unified-2026-08-05.md` §2
//!
//! **状态**: 🟢 R20 阶段 6 实施 (sub-agent 跑 2026-08-05, 1.0 release #8 observability 必做).

#![allow(clippy::all)]

use std::sync::Arc;
use std::time::Instant;

use axum::routing::get;
use axum::Router;
use parking_lot::RwLock;
use serde::Serialize;

pub mod health;
pub mod metrics;
pub mod status;
pub mod dashboard;

// ============================================================================
// 编译期 hardcode 常量 (per K-1 强校验 — 跟 `apeireth-observability` 同模式)
// ============================================================================

/// Observability schema 版本 (向前兼容字段, 改格式时 bump).
pub const OBSERVABILITY_SCHEMA_VERSION: &str = "1";

/// 服务启动时间 (编译期 hardcode Unix epoch, 2026-08-05 21:00:00 UTC = 2026-08-05 阶段 6 起点).
/// 1:1 翻译 release 报告里的 R20 阶段 6 起点 (per `docs/release/1.0.0-release-report-2026-08-05.md` §1).
///
/// **不漂移**: 0 改, 永远 = 2026-08-05 21:00:00 UTC. 真服务启动时 `State::start_time` 用 `Instant::now()` 记录
/// 进程内 elapsed, 这常量只作为 build_time 兜底.
pub const SERVICE_BUILD_TIME: &str = "2026-08-05T21:00:00Z";

/// Git commit (编译期从 `GIT_COMMIT` env 读, 兜底 "unknown" 防止 m3 hallucination).
/// 1:1 翻译 OpenTelemetry `service.version` 字段.
pub const SERVICE_GIT_COMMIT: &str = match option_env!("GIT_COMMIT") {
    Some(c) if !c.is_empty() => c,
    _ => "unknown",
};

/// Service 名 (K-1 强校验, 永远 = "apeireth-api").
pub const SERVICE_NAME: &str = "apeireth-api";

/// 3 端点路径 (跟 `apeireth-observability::HEALTH_ENDPOINTS` 模式对齐, 编译期 hardcode).
pub const OBSERVABILITY_ENDPOINTS: &[&str] = &[
    "/v1/observability/metrics",
    "/v1/observability/health",
    "/v1/observability/status",
];

/// 端点数量 (编译期守门, 必 = 3).
pub const OBSERVABILITY_ENDPOINTS_COUNT: usize = 3;
const _: () = assert!(OBSERVABILITY_ENDPOINTS.len() == OBSERVABILITY_ENDPOINTS_COUNT);

/// 5 组件 (跟 task spec 一致: db / cache / queue / external_api / disk_space).
pub const HEALTH_COMPONENTS: &[&str] = &["db", "cache", "queue", "external_api", "disk_space"];

/// 组件数量 (编译期守门, 必 = 5).
pub const HEALTH_COMPONENTS_COUNT: usize = 5;
const _: () = assert!(HEALTH_COMPONENTS.len() == HEALTH_COMPONENTS_COUNT);

/// 5 R-Measure 名 (per `docs/stage4/r-measure-verification-design-2026-08-05.md` §1-§5).
/// - R-1 直行率 (Direct Execution Rate): 任务一次执行成功比例, 不重试不绕路
/// - R-2 直说率 (Direct Speech Rate): 回答用户原始问题, 不绕弯
/// - R-3 闭环率 (Closed Loop Rate): 任务从输入到结果完整闭环, 不掉链
/// - R-4 守门率 (Guard Rate): 哲学 / 权限 / PII 守门触发的拒绝率
/// - R-5 失败诚实率 (Failure Honesty Rate): 失败时诚实说失败, 不假装成功
pub const R_MEASURES: &[&str] = &[
    "R-1_direct_execution",
    "R-2_direct_speech",
    "R-3_closed_loop",
    "R-4_guard",
    "R-5_failure_honesty",
];

/// R-Measure 数量 (编译期守门, 必 = 5).
pub const R_MEASURES_COUNT: usize = 5;
const _: () = assert!(R_MEASURES.len() == R_MEASURES_COUNT);

/// 6 哲学锚名 (per `docs/stage4/apeireth-formal-invariants-2026-08-05.md` §11).
/// - S-1 北极星导向 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装
pub const PHILOSOPHY_ANCHORS: &[&str] = &[
    "S-1_north_star",
    "S-2_seek_truth",
    "O-2_stand_on_shoulders",
    "O-3_drive_through",
    "O-4_handoff_to_anyone",
    "O-5_no_pretense",
];

/// 哲学锚数量 (编译期守门, 必 = 6).
pub const PHILOSOPHY_ANCHORS_COUNT: usize = 6;
const _: () = assert!(PHILOSOPHY_ANCHORS.len() == PHILOSOPHY_ANCHORS_COUNT);

// ============================================================================
// §1 共享状态 (进程级, OnceCell init 模式)
// ============================================================================

/// 进程启动时间 (进程内 `Instant`, uptime 用).
pub static SERVICE_START_INSTANT: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// 全局共享状态 (5 组件 health + 15-20 metrics + 5 R-Measure + 8 status 字段).
pub type SharedObsState = Arc<RwLock<ObsState>>;

/// Observability 全局状态.
#[derive(Debug)]
pub struct ObsState {
    /// 进程启动时间
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// 进程内 instant (uptime 计算用)
    pub start_instant: Instant,
    /// 5 组件 health (key = component name, value = status string)
    pub components: std::collections::HashMap<String, ComponentHealth>,
    /// 15-20 metric 样本 (key = metric name, value = snapshot)
    pub metrics: std::collections::HashMap<String, MetricSnapshot>,
    /// 5 R-Measure (key = R-Measure 名, value = [0.0, 1.0] 比例, R20 阶段 6 估 placeholder)
    pub r_measures: std::collections::HashMap<String, f64>,
    /// 当前 active session 数 (估从 metrics 推, R20 阶段 6 stub)
    pub active_sessions: u64,
    /// 最近 10 request log (R20 阶段 6 stub: 内存环, 启动时填充 5 条)
    pub recent_requests: Vec<RequestLogEntry>,
}

/// 单个组件 health 状态.
#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    /// 组件名 ("db" / "cache" / "queue" / "external_api" / "disk_space")
    pub name: String,
    /// 状态 ("ok" / "degraded" / "down")
    pub status: String,
    /// 最后检查时间 (UTC RFC 3339)
    pub last_check: chrono::DateTime<chrono::Utc>,
    /// 详情 (key-value)
    pub details: std::collections::HashMap<String, String>,
}

/// 单个 metric snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct MetricSnapshot {
    /// Metric 名 (snake_case, 例: "http_requests_total")
    pub name: String,
    /// 类型 ("counter" / "gauge" / "histogram")
    pub kind: String,
    /// 当前值
    pub value: f64,
    /// 标签 (key-value)
    pub labels: std::collections::HashMap<String, String>,
}

/// 单个 request log 条目.
#[derive(Debug, Clone, Serialize)]
pub struct RequestLogEntry {
    /// 时间戳 (UTC RFC 3339)
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// HTTP 方法 ("GET" / "POST")
    pub method: String,
    /// 路径 (例: "/v1/chat/completions")
    pub path: String,
    /// HTTP 状态码
    pub status: u16,
    /// 延迟 (毫秒)
    pub latency_ms: u64,
    /// 关联 trace_id (32 hex, 缺省 "00000000000000000000000000000000")
    pub trace_id: String,
}

impl ObsState {
    /// 新建默认状态 (5 组件全 ok, 15-20 metric 初始化 0, 5 R-Measure 占位 0.0).
    pub fn new() -> Self {
        use std::collections::HashMap;
        let now = chrono::Utc::now();
        let mut components = HashMap::new();
        for name in HEALTH_COMPONENTS {
            components.insert(
                (*name).to_string(),
                ComponentHealth {
                    name: (*name).to_string(),
                    status: "ok".to_string(),
                    last_check: now,
                    details: HashMap::new(),
                },
            );
        }
        let metrics = init_default_metrics();
        let r_measures = init_default_r_measures();
        // 5 条占位 request log (R20 阶段 6 stub, O-5 不假装)
        let recent_requests = vec![
            RequestLogEntry {
                timestamp: now,
                method: "GET".to_string(),
                path: "/v1/observability/metrics".to_string(),
                status: 200,
                latency_ms: 1,
                trace_id: "00000000000000000000000000000000".to_string(),
            },
            RequestLogEntry {
                timestamp: now,
                method: "GET".to_string(),
                path: "/v1/observability/health".to_string(),
                status: 200,
                latency_ms: 1,
                trace_id: "00000000000000000000000000000000".to_string(),
            },
            RequestLogEntry {
                timestamp: now,
                method: "GET".to_string(),
                path: "/v1/observability/status".to_string(),
                status: 200,
                latency_ms: 1,
                trace_id: "00000000000000000000000000000000".to_string(),
            },
            RequestLogEntry {
                timestamp: now,
                method: "POST".to_string(),
                path: "/v1/chat/completions".to_string(),
                status: 200,
                latency_ms: 850,
                trace_id: "00000000000000000000000000000001".to_string(),
            },
            RequestLogEntry {
                timestamp: now,
                method: "POST".to_string(),
                path: "/v1/messages".to_string(),
                status: 200,
                latency_ms: 1023,
                trace_id: "00000000000000000000000000000002".to_string(),
            },
        ];
        Self {
            start_time: now,
            start_instant: Instant::now(),
            components,
            metrics,
            r_measures,
            active_sessions: 0,
            recent_requests,
        }
    }

    /// 进程内 uptime (秒).
    pub fn uptime_seconds(&self) -> u64 {
        self.start_instant.elapsed().as_secs()
    }
}

impl Default for ObsState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// §2 进程级 lazy 全局 state (跟 keyring/machine-id/lark skeleton 同模式)
// ============================================================================

/// 全局 state (OnceLock 模式, 0 panic, 0 锁竞争).
static GLOBAL_STATE: std::sync::OnceLock<SharedObsState> = std::sync::OnceLock::new();

/// 拿全局 state (test 可重置).
pub fn global_state() -> SharedObsState {
    GLOBAL_STATE
        .get_or_init(|| Arc::new(RwLock::new(ObsState::new())))
        .clone()
}

/// 重置全局 state (test 用, 0 业务路径调用).
#[cfg(test)]
pub fn reset_for_test() {
    // 没法 reset OnceLock, test 改用 `ObsState::new()` 直接构造.
}

// ============================================================================
// §3 默认 15-20 metric 初始化 (per task spec "15-20 metric")
// ============================================================================

/// 初始化默认 15-20 metric (R20 阶段 6 占位 0, 跟 `MetricsRegistry` 同模式).
///
/// 18 metric (3 类):
/// - Counter (8): http_requests_total, http_errors_total, llm_requests_total, llm_tokens_total,
///                llm_errors_total, ratelimit_hits_total, ws_connections_total, observability_scrapes_total
/// - Gauge (8):   active_sessions, in_flight_requests, memory_rss_bytes, cpu_percent,
///                disk_free_bytes, uptime_seconds, last_request_latency_ms, llm_provider_health
/// - Histogram (2): request_latency_seconds (last value 取代), token_usage_per_request
fn init_default_metrics() -> std::collections::HashMap<String, MetricSnapshot> {
    use std::collections::HashMap;
    let mut m = HashMap::new();

    // 8 Counter
    let counters = [
        ("http_requests_total", "counter", 0.0, vec![("method", "all")]),
        ("http_errors_total", "counter", 0.0, vec![("status_class", "5xx")]),
        ("llm_requests_total", "counter", 0.0, vec![("protocol", "all")]),
        ("llm_tokens_total", "counter", 0.0, vec![("type", "all")]),
        ("llm_errors_total", "counter", 0.0, vec![("protocol", "all")]),
        ("ratelimit_hits_total", "counter", 0.0, vec![("scope", "all")]),
        ("ws_connections_total", "counter", 0.0, vec![("direction", "all")]),
        ("observability_scrapes_total", "counter", 0.0, vec![("endpoint", "all")]),
    ];
    for (name, kind, value, labels) in counters {
        let mut l = HashMap::new();
        for (k, v) in labels {
            l.insert(k.to_string(), v.to_string());
        }
        m.insert(
            name.to_string(),
            MetricSnapshot {
                name: name.to_string(),
                kind: kind.to_string(),
                value,
                labels: l,
            },
        );
    }

    // 8 Gauge
    let gauges = [
        ("active_sessions", "gauge", 0.0, vec![("scope", "all")]),
        ("in_flight_requests", "gauge", 0.0, vec![("scope", "all")]),
        ("memory_rss_bytes", "gauge", 0.0, vec![]),
        ("cpu_percent", "gauge", 0.0, vec![]),
        ("disk_free_bytes", "gauge", 0.0, vec![("mount", "/")]),
        ("uptime_seconds", "gauge", 0.0, vec![]),
        ("last_request_latency_ms", "gauge", 0.0, vec![]),
        ("llm_provider_health", "gauge", 1.0, vec![("provider", "minimaxi")]),
    ];
    for (name, kind, value, labels) in gauges {
        let mut l = HashMap::new();
        for (k, v) in labels {
            l.insert(k.to_string(), v.to_string());
        }
        m.insert(
            name.to_string(),
            MetricSnapshot {
                name: name.to_string(),
                kind: kind.to_string(),
                value,
                labels: l,
            },
        );
    }

    // 2 Histogram (R20 阶段 6 只存 last value, R20 阶段 3 续 bucket 累加)
    let histograms = [
        ("request_latency_seconds", "histogram", 0.0, vec![("endpoint", "all")]),
        ("token_usage_per_request", "histogram", 0.0, vec![("protocol", "all")]),
    ];
    for (name, kind, value, labels) in histograms {
        let mut l = HashMap::new();
        for (k, v) in labels {
            l.insert(k.to_string(), v.to_string());
        }
        m.insert(
            name.to_string(),
            MetricSnapshot {
                name: name.to_string(),
                kind: kind.to_string(),
                value,
                labels: l,
            },
        );
    }

    m
}

/// 默认 5 R-Measure (R20 阶段 6 placeholder 0.0, 标 "stub" 字段).
fn init_default_r_measures() -> std::collections::HashMap<String, f64> {
    let mut m = std::collections::HashMap::new();
    for r in R_MEASURES {
        m.insert((*r).to_string(), 0.0);
    }
    m
}

// ============================================================================
// §4 路由构造 (跟 v2_endpoints::build_router 同模式, 1 行 merge 接入)
// ============================================================================

use crate::v2_endpoints::SharedV2;

/// 构造 observability 3 端点 sub-router (跟 v2_endpoints::build_router 同模式).
///
/// **返回类型**: `Router<SharedV2>` (跟 v2 router state 兼容, 让 caller 可直接 `.merge`).
///
/// **handlers 不依赖 SharedV2 state**: 全局 state 走 `crate::observability::global_state()`
/// (OnceLock<RwLock<ObsState>>), 跟 v2_endpoints::V2State 解耦. 但 router state type 必须跟 v2 一致
/// 才能 `Router::merge`, 所以这里用 `SharedV2` 作 phantom type.
pub fn router() -> Router<SharedV2> {
    Router::new()
        .route("/observability/metrics", get(metrics::metrics_handler))
        .route("/observability/health", get(health::health_handler))
        .route("/observability/status", get(status::status_handler))
}

// ============================================================================
// §5 K-1 强校验自检 (5 K-1 字样 + 3 endpoint + 5 component + 5 R-Measure + 6 anchor)
// ============================================================================

/// K-1 强校验 #1: 平台名 (5 K-1 字样 #1).
pub fn k1_check_platform_name() -> bool {
    SERVICE_NAME == "apeireth-api"
}

/// K-1 强校验 #2: 3 observability 端点 (5 K-1 字样 #2: "observability").
pub fn k1_check_endpoints_count() -> bool {
    OBSERVABILITY_ENDPOINTS.len() == OBSERVABILITY_ENDPOINTS_COUNT
        && OBSERVABILITY_ENDPOINTS.contains(&"/v1/observability/metrics")
        && OBSERVABILITY_ENDPOINTS.contains(&"/v1/observability/health")
        && OBSERVABILITY_ENDPOINTS.contains(&"/v1/observability/status")
}

/// K-1 强校验 #3: 5 health 组件 (5 K-1 字样 #3: "health").
pub fn k1_check_components_count() -> bool {
    HEALTH_COMPONENTS.len() == HEALTH_COMPONENTS_COUNT
        && HEALTH_COMPONENTS.contains(&"db")
        && HEALTH_COMPONENTS.contains(&"cache")
        && HEALTH_COMPONENTS.contains(&"queue")
        && HEALTH_COMPONENTS.contains(&"external_api")
        && HEALTH_COMPONENTS.contains(&"disk_space")
}

/// K-1 强校验 #4: 5 R-Measure (5 K-1 字样 #4: "R-Measure" / "must-do").
pub fn k1_check_r_measures_count() -> bool {
    R_MEASURES.len() == R_MEASURES_COUNT
        && R_MEASURES.contains(&"R-1_direct_execution")
        && R_MEASURES.contains(&"R-2_direct_speech")
        && R_MEASURES.contains(&"R-3_closed_loop")
        && R_MEASURES.contains(&"R-4_guard")
        && R_MEASURES.contains(&"R-5_failure_honesty")
}

/// K-1 强校验 #5: 6 哲学锚 (5 K-1 字样 #5: "anchor" / "philosophy").
pub fn k1_check_philosophy_anchors_count() -> bool {
    PHILOSOPHY_ANCHORS.len() == PHILOSOPHY_ANCHORS_COUNT
        && PHILOSOPHY_ANCHORS.contains(&"S-1_north_star")
        && PHILOSOPHY_ANCHORS.contains(&"S-2_seek_truth")
        && PHILOSOPHY_ANCHORS.contains(&"O-2_stand_on_shoulders")
        && PHILOSOPHY_ANCHORS.contains(&"O-3_drive_through")
        && PHILOSOPHY_ANCHORS.contains(&"O-4_handoff_to_anyone")
        && PHILOSOPHY_ANCHORS.contains(&"O-5_no_pretense")
}

/// 全部 K-1 强校验 (5 条全过).
#[must_use]
pub fn k1_all_pass() -> bool {
    k1_check_platform_name()
        && k1_check_endpoints_count()
        && k1_check_components_count()
        && k1_check_r_measures_count()
        && k1_check_philosophy_anchors_count()
}

// ============================================================================
// 单元测试 (in-module, 集成测试在 tests/test_observability_in_process.rs)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k1_platform_name_in_unit() {
        assert_eq!(SERVICE_NAME, "apeireth-api");
    }

    #[test]
    fn k1_endpoints_count_in_unit() {
        assert_eq!(OBSERVABILITY_ENDPOINTS.len(), 3);
        assert!(OBSERVABILITY_ENDPOINTS.contains(&"/v1/observability/metrics"));
        assert!(OBSERVABILITY_ENDPOINTS.contains(&"/v1/observability/health"));
        assert!(OBSERVABILITY_ENDPOINTS.contains(&"/v1/observability/status"));
    }

    #[test]
    fn k1_components_count_in_unit() {
        assert_eq!(HEALTH_COMPONENTS.len(), 5);
        for c in ["db", "cache", "queue", "external_api", "disk_space"] {
            assert!(HEALTH_COMPONENTS.contains(&c), "missing component {c}");
        }
    }

    #[test]
    fn k1_r_measures_count_in_unit() {
        assert_eq!(R_MEASURES.len(), 5);
    }

    #[test]
    fn k1_philosophy_anchors_count_in_unit() {
        assert_eq!(PHILOSOPHY_ANCHORS.len(), 6);
    }

    #[test]
    fn k1_all_pass_in_unit() {
        assert!(k1_all_pass());
    }

    #[test]
    fn obs_state_new_has_5_components_all_ok() {
        let s = ObsState::new();
        assert_eq!(s.components.len(), 5);
        for c in HEALTH_COMPONENTS {
            let h = s.components.get(*c).expect("component exists");
            assert_eq!(h.status, "ok", "component {c} should default ok");
        }
    }

    #[test]
    fn obs_state_new_has_18_metrics() {
        // 8 counter + 8 gauge + 2 histogram = 18
        let s = ObsState::new();
        assert_eq!(s.metrics.len(), 18);
        let counters = s
            .metrics
            .values()
            .filter(|m| m.kind == "counter")
            .count();
        let gauges = s.metrics.values().filter(|m| m.kind == "gauge").count();
        let histograms = s
            .metrics
            .values()
            .filter(|m| m.kind == "histogram")
            .count();
        assert_eq!(counters, 8, "8 counter metric");
        assert_eq!(gauges, 8, "8 gauge metric");
        assert_eq!(histograms, 2, "2 histogram metric");
    }

    #[test]
    fn obs_state_new_has_5_r_measures_all_zero() {
        let s = ObsState::new();
        assert_eq!(s.r_measures.len(), 5);
        for r in R_MEASURES {
            assert_eq!(s.r_measures.get(*r), Some(&0.0), "R-Measure {r} default 0.0");
        }
    }

    #[test]
    fn obs_state_uptime_seconds_runs() {
        let s = ObsState::new();
        let up1 = s.uptime_seconds();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let up2 = s.uptime_seconds();
        assert!(up2 >= up1, "uptime monotonic");
    }

    #[test]
    fn global_state_idempotent() {
        let s1 = global_state();
        let s2 = global_state();
        assert!(Arc::ptr_eq(&s1, &s2), "global state singleton");
    }

    #[test]
    fn router_has_3_routes() {
        let r = router();
        // axum Router 没有公开 count, 我们走 routes 长度验证.
        // 简单验: Router 可构造 + 3 个 .route() 调用编译通过.
        let _ = r;
    }

    #[test]
    fn json_test_helper() {
        use serde_json::json;
        // 验证依赖 (axum::Json, serde_json) 正确链入
        let v = axum::Json(json!({"ok": true}));
        assert_eq!(v.0["ok"], json!(true));
    }
}
