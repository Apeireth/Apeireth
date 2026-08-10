//! # Health check 3 端点
//!
//! 1:1 翻译 v0.9.21 商业版 `out/main` observability 集成 (per blueprint §2.5.3).
//! 3 端点: `/health` (liveness) / `/ready` (readiness) / `/metrics` (Prometheus scrape).
//!
//! ## 端点语义
//!
//! - `/health`: 进程是否存活 (永远 Healthy 除非 panic, 适合 Kubernetes liveness probe)
//! - `/ready`: 依赖是否就绪 (DB / Keyring / OTLP collector, 适合 Kubernetes readiness probe)
//! - `/metrics`: Prometheus exposition format (text/plain, scrape 用)

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use tokio::time::timeout;
use tracing::{info, warn};

use super::{
    render_prometheus, HealthResponse, HealthStatus, MetricSample, ObservabilityError,
    ObservabilityResult, HEALTH_CHECK_TIMEOUT_MS, HEALTH_ENDPOINTS, PLATFORM_NAME,
    SUPPORTED_METRICS,
};

/// Health 端点枚举 (3 端点, 编译期 hardcode 跟 `HEALTH_ENDPOINTS` 对齐).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HealthEndpoint {
    /// `/health` (liveness)
    Health,
    /// `/ready` (readiness)
    Ready,
    /// `/metrics` (Prometheus scrape)
    Metrics,
}

impl HealthEndpoint {
    /// 从路径字符串解析 (例: "/health" → `HealthEndpoint::Health`).
    pub fn from_path(path: &str) -> ObservabilityResult<Self> {
        match path {
            "/health" => Ok(Self::Health),
            "/ready" => Ok(Self::Ready),
            "/metrics" => Ok(Self::Metrics),
            other => Err(ObservabilityError::HealthEndpointUnknown(other.to_string())),
        }
    }

    /// 路径字符串.
    #[must_use]
    pub fn as_path(&self) -> &'static str {
        match self {
            Self::Health => "/health",
            Self::Ready => "/ready",
            Self::Metrics => "/metrics",
        }
    }
}

impl fmt::Display for HealthEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_path())
    }
}

impl From<HealthEndpoint> for &'static str {
    fn from(ep: HealthEndpoint) -> Self {
        ep.as_path()
    }
}

/// Health check 处理器 (带超时, async).
pub async fn health_check(
    endpoint: HealthEndpoint,
    current_status: HealthStatus,
    metrics: &[MetricSample],
) -> HealthResponse {
    // 编译期守门: 端点必须在 HEALTH_ENDPOINTS 中
    if !HEALTH_ENDPOINTS.contains(&endpoint.as_path()) {
        warn!(
            endpoint = %endpoint,
            "health: endpoint not in HEALTH_ENDPOINTS (should never happen)"
        );
    }

    // 异步超时 (用 HEALTH_CHECK_TIMEOUT_MS, 不阻塞)
    let fut = async {
        match endpoint {
            HealthEndpoint::Health => {
                // liveness: 永远 Healthy (除非 panic)
                HealthResponse::new(endpoint.as_path(), HealthStatus::Healthy)
                    .with_detail("check_type", "liveness")
                    .with_detail("uptime_check", "process_alive")
            }
            HealthEndpoint::Ready => {
                // readiness: 走 current_status
                HealthResponse::new(endpoint.as_path(), current_status)
                    .with_detail("check_type", "readiness")
                    .with_detail("platform", PLATFORM_NAME)
            }
            HealthEndpoint::Metrics => {
                // metrics: 永远 Healthy (Prometheus scrape 端点), 把 metric 数量塞 detail
                let count = metrics.len();
                let kinds: Vec<String> = SUPPORTED_METRICS.iter().map(|k| k.to_string()).collect();
                HealthResponse::new(endpoint.as_path(), HealthStatus::Healthy)
                    .with_detail("check_type", "metrics_scrape")
                    .with_detail("metric_count", count.to_string())
                    .with_detail("supported_kinds", kinds.join(","))
            }
        }
    };

    match timeout(Duration::from_millis(HEALTH_CHECK_TIMEOUT_MS), fut).await {
        Ok(resp) => {
            info!(
                endpoint = %endpoint,
                status = %resp.status,
                "health: check ok"
            );
            resp
        }
        Err(_) => {
            warn!(
                endpoint = %endpoint,
                timeout_ms = HEALTH_CHECK_TIMEOUT_MS,
                "health: check timeout"
            );
            HealthResponse::new(endpoint.as_path(), HealthStatus::Unhealthy)
                .with_detail("error", "health_check_timeout")
                .with_detail(
                    "timeout_ms",
                    HEALTH_CHECK_TIMEOUT_MS.to_string(),
                )
        }
    }
}

/// 渲染 health response 为 JSON 字符串 (HTTP handler 用).
pub fn render_health_response(resp: &HealthResponse) -> ObservabilityResult<String> {
    serde_json::to_string(resp).map_err(ObservabilityError::JsonLog)
}

/// 渲染 Prometheus exposition format (HTTP `/metrics` handler 用).
#[must_use]
pub fn render_metrics_response(samples: &[MetricSample]) -> String {
    render_prometheus(samples)
}

/// Health check 配置 (3 端点共享).
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// 端点列表 (3 端点, 编译期对齐 `HEALTH_ENDPOINTS`)
    pub endpoints: Vec<HealthEndpoint>,
    /// 超时 (毫秒)
    pub timeout_ms: u64,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            endpoints: vec![
                HealthEndpoint::Health,
                HealthEndpoint::Ready,
                HealthEndpoint::Metrics,
            ],
            timeout_ms: HEALTH_CHECK_TIMEOUT_MS,
        }
    }
}

impl HealthConfig {
    /// 编译期守门: endpoints 长度 == 3 (per K-1 强校验).
    #[must_use]
    pub fn validate(&self) -> bool {
        self.endpoints.len() == HEALTH_ENDPOINTS.len()
    }

    /// 把所有端点转 path 字符串 (`{"/health", "/ready", "/metrics"}`).
    #[must_use]
    pub fn endpoint_paths(&self) -> HashMap<String, HealthEndpoint> {
        self.endpoints
            .iter()
            .map(|e| (e.as_path().to_string(), *e))
            .collect()
    }
}

// ============================================================================
// 单元测试 (in-module)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_endpoint_from_path_3_valid() {
        assert_eq!(
            HealthEndpoint::from_path("/health").unwrap(),
            HealthEndpoint::Health
        );
        assert_eq!(
            HealthEndpoint::from_path("/ready").unwrap(),
            HealthEndpoint::Ready
        );
        assert_eq!(
            HealthEndpoint::from_path("/metrics").unwrap(),
            HealthEndpoint::Metrics
        );
    }

    #[test]
    fn health_endpoint_from_path_unknown() {
        let err = HealthEndpoint::from_path("/unknown").unwrap_err();
        assert!(matches!(err, ObservabilityError::HealthEndpointUnknown(_)));
    }

    #[test]
    fn health_endpoint_as_path_roundtrip() {
        for ep in [
            HealthEndpoint::Health,
            HealthEndpoint::Ready,
            HealthEndpoint::Metrics,
        ] {
            assert_eq!(HealthEndpoint::from_path(ep.as_path()).unwrap(), ep);
        }
    }

    #[test]
    fn health_endpoint_display() {
        assert_eq!(HealthEndpoint::Health.to_string(), "/health");
        assert_eq!(HealthEndpoint::Ready.to_string(), "/ready");
        assert_eq!(HealthEndpoint::Metrics.to_string(), "/metrics");
    }

    #[tokio::test]
    async fn health_check_health_endpoint_always_healthy() {
        let resp = health_check(HealthEndpoint::Health, HealthStatus::Unhealthy, &[]).await;
        assert_eq!(resp.status, HealthStatus::Healthy);
        assert_eq!(resp.endpoint, "/health");
    }

    #[tokio::test]
    async fn health_check_ready_endpoint_uses_current_status() {
        let resp = health_check(HealthEndpoint::Ready, HealthStatus::Degraded, &[]).await;
        assert_eq!(resp.status, HealthStatus::Degraded);
        assert_eq!(resp.endpoint, "/ready");
    }

    #[tokio::test]
    async fn health_check_metrics_endpoint_returns_count() {
        let samples = vec![
            MetricSample::new("requests_total", super::super::MetricKind::Counter, 1.0),
            MetricSample::new("requests_total", super::super::MetricKind::Counter, 2.0),
        ];
        let resp = health_check(HealthEndpoint::Metrics, HealthStatus::Healthy, &samples).await;
        assert_eq!(resp.status, HealthStatus::Healthy);
        assert_eq!(
            resp.details.get("metric_count").map(|s| s.as_str()),
            Some("2")
        );
    }

    #[test]
    fn health_config_default_has_3_endpoints() {
        let cfg = HealthConfig::default();
        assert!(cfg.validate());
        assert_eq!(cfg.endpoints.len(), 3);
    }

    #[test]
    fn health_status_http_code() {
        assert_eq!(HealthStatus::Healthy.http_status_code(), 200);
        assert_eq!(HealthStatus::Degraded.http_status_code(), 200);
        assert_eq!(HealthStatus::Unhealthy.http_status_code(), 503);
    }
}
